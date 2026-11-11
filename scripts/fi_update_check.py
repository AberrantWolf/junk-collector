#!/usr/bin/env python3
"""Check a routine split-host beta update against a published image and a stopped recovery point."""

import argparse
from contextlib import closing, contextmanager, nullcontext
from datetime import datetime
from getpass import getpass
from hashlib import sha256
import json
import os
from pathlib import Path
import re
import shutil
import sqlite3
import ssl
import subprocess
import sys
import tempfile
import time
from urllib.error import HTTPError, URLError
from urllib.parse import urlsplit
from urllib.request import HTTPRedirectHandler, HTTPSHandler, ProxyHandler, Request, build_opener
from uuid import UUID

from bridge_status import SYNC_INTERRUPTED, query as bridge_status


UPDATED = {"collector", "media"}
REQUIRED = UPDATED | {"media-https", "proton-bridge"}
DIGEST = re.compile(r"sha256:[a-f0-9]{64}")
BETA = re.compile(r"(\d+)\.(\d+)\.(\d+)-beta\.(\d+)")
# The scanner updates these fields without changing the connection settings.
RECEIPT_SCAN_FIELDS = {"cursor", "last_checked", "problem", "failure", "progress"}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def docker_json(*args):
    return json.loads(subprocess.check_output(["docker", *args], timeout=15, stderr=subprocess.DEVNULL))


def beta_version(value):
    match = BETA.fullmatch(value)
    require(match, "Routine updates require published beta versions")
    return tuple(map(int, match.groups()))


def target(path):
    release = json.loads(path.read_text())
    ref, digest, version, revision = (release[key] for key in ("reference", "digest", "version", "revision"))
    require(
        release["anonymous_docker_pulls"] is True and release["channel"] == "preview"
        and DIGEST.fullmatch(digest) and ref == release["image"] + "@" + digest
        and re.fullmatch(r"[a-f0-9]{40}", revision),
        "Use the verified release-image.json from a published beta",
    )
    beta_version(version)
    image, = docker_json("image", "inspect", ref)
    labels = image["Config"]["Labels"]
    require(
        ref in image["RepoDigests"] and image["Os"] == "linux" and image["Architecture"] == "amd64"
        and labels.get("org.opencontainers.image.version") == version
        and labels.get("org.opencontainers.image.revision") == revision
        and labels.get("org.opencontainers.image.source") == release["source"],
        "Pulled application image does not match the published release",
    )
    return {"reference": ref, "id": image["Id"], "version": version, "revision": revision}


def stack(project, companions):
    output = subprocess.check_output(
        ["docker", "ps", "-aq", "--filter", f"label=com.docker.compose.project={project}",
         "--filter", "label=com.docker.compose.oneoff=False"],
        text=True, timeout=15, stderr=subprocess.DEVNULL,
    )
    ids = output.split()
    require(ids, "No containers found for the specified Compose project")
    services = {}
    for container in docker_json("inspect", *ids):
        name = container["Config"]["Labels"]["com.docker.compose.service"]
        require(name not in services, f"Expected one container for {name}")
        services[name] = container
    require(REQUIRED <= services.keys(), "Compose project is missing a required service")
    if companions:
        for container in docker_json("inspect", *companions):
            services["external:" + container["Name"]] = container
    return services


def container_facts(services):
    return {
        name: {
            "id": value["Id"],
            "image": value["Image"],
            "mounts": sorted(
                [mount["Destination"], mount["Type"], mount["Source"], mount["RW"]]
                for mount in value["Mounts"]
            ),
            "reference": value["Config"]["Image"],
        }
        for name, value in services.items()
    }


def ready(services, after):
    for name, service in services.items():
        state = service["State"]
        require(state["Running"] and not state["Restarting"], f"{name} is not running steadily")
        if name == "collector":
            require(state.get("Health", {}).get("Status") == "healthy", "Collector is not healthy")
        if after and name in UPDATED:
            require(service["RestartCount"] == 0, f"{name} has restarted")


def compare_stack(baseline, current, image):
    require(baseline.keys() == current.keys(), "Compose service set changed")
    for name, old in baseline.items():
        new = current[name]
        require(new["mounts"] == old["mounts"], f"{name} mount bindings changed")
        if name in UPDATED:
            require(new["image"] == image["id"] and new["reference"] == image["reference"],
                    f"{name} is not running the verified image by digest")
        else:
            require(new["id"] == old["id"] and new["image"] == old["image"],
                    f"{name} was recreated or changed image")


class NoRedirect(HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


def origin(value):
    url = urlsplit(value)
    require(url.scheme == "https" and url.hostname and not url.username and not url.password
            and url.path in ("", "/") and not url.query and not url.fragment,
            "Use a bare HTTPS origin")
    return value.rstrip("/")


def get(base, path, session=None, ca=None):
    headers = {"Cookie": "collector_session=" + session} if session else {}
    tls = HTTPSHandler(context=ssl.create_default_context(cafile=ca))
    try:
        with build_opener(ProxyHandler({}), NoRedirect(), tls).open(
            Request(base + path, headers=headers), timeout=5
        ) as response:
            require(response.status == 200, f"{base}{path} did not return HTTP 200")
            body = response.read(1024 * 1024 + 1)
            require(len(body) <= 1024 * 1024, f"{base}{path} exceeded the response limit")
            return json.loads(body) if path.startswith("/api/") else None
    except HTTPError as error:
        status = error.code
        error.close()
        raise ValueError(f"{base}{path} returned HTTP {status}") from None
    except (URLError, TimeoutError):
        raise ValueError(f"{base}{path} could not be reached over trusted HTTPS") from None


def probe(origins, item, session, lan_ca):
    identities = {}
    for role, base in origins.items():
        ca = lan_ca if role == "lan" else None
        if role != "media":
            get(base, "/healthz", ca=ca)
        identities[role] = str(UUID(get(base, "/api/v1/identity", ca=ca)["server_id"]))
    require(len(set(identities.values())) == 1, "HTTPS origins disagree on ServerId")
    public = origins["public"]
    principal = get(public, "/api/v1/me", session)
    sample = get(public, "/api/v1/items/" + item, session)
    require(sample["id"] == item, "Authenticated read returned a different item")
    return {
        "server_id": identities["public"], "user_id": str(UUID(principal["user_id"])),
        "item_id": item, "collection_id": str(UUID(sample["collection_id"])),
        "revision": sample["revision"],
    }


@contextmanager
def database(path, stopped):
    require(path.is_file(), f"{path.name} is missing")
    wal = path.with_name(path.name + "-wal")
    needs_copy = stopped and wal.exists() and wal.stat().st_size > 0
    scratch_dir = tempfile.TemporaryDirectory(prefix="fi-update-db-") if needs_copy else nullcontext()
    with scratch_dir as scratch:
        if needs_copy:
            copy = Path(scratch) / path.name
            shutil.copyfile(path, copy)
            shutil.copyfile(wal, copy.with_name(wal.name))
            path = copy
        options = "?mode=ro&immutable=1" if stopped and not needs_copy else "?mode=ro"
        with closing(sqlite3.connect(path.resolve().as_uri() + options, uri=True, timeout=3)) as db:
            yield db


def mail_connections(db):
    rows = db.execute("SELECT id, owner, record FROM mail_connections ORDER BY id")
    return [(id_, owner, json.loads(record)) for id_, owner, record in rows]


def state_facts(state, stopped=False):
    with database(state / "private.sqlite", stopped) as db:
        if stopped:
            require(db.execute("PRAGMA quick_check").fetchall() == [("ok",)], "Stopped private database failed quick_check")
        db.execute("BEGIN")
        require(db.execute("SELECT 1 FROM jobs WHERE state IN ('queued','running','cancel_requested') LIMIT 1").fetchone() is None,
                "Unfinished Jobs require the extended update procedure")
        require(db.execute("SELECT 1 FROM imports WHERE phase IN ('analyzing','publishing') LIMIT 1").fetchone() is None,
                "Active imports require the extended update procedure")
        facts = {
            "jobs": db.execute("SELECT state, count(*) FROM jobs GROUP BY state ORDER BY state").fetchall(),
            "imports": db.execute("SELECT phase, count(*) FROM imports GROUP BY phase ORDER BY phase").fetchall(),
            "staged_imports": db.execute("SELECT id, revision, phase FROM imports WHERE phase IN ('staging','awaiting_review') ORDER BY id").fetchall(),
            "accounts": db.execute("SELECT id, administrator, enabled FROM users ORDER BY id").fetchall(),
            "grants": db.execute("SELECT collection_id, user_id, role FROM grants ORDER BY collection_id, user_id").fetchall(),
            "mail_connection_settings": [
                (id_, owner, {key: value for key, value in record.items() if key not in RECEIPT_SCAN_FIELDS})
                for id_, owner, record in mail_connections(db)
            ],
        }
    with database(state / "catalog.sqlite", stopped) as db:
        if stopped:
            require(db.execute("PRAGMA quick_check").fetchall() == [("ok",)], "Stopped catalog database failed quick_check")
        facts["items"] = db.execute("SELECT count(*) FROM items").fetchone()[0]
    facts["archive_binding"] = (state / "archive-binding").read_bytes()
    facts["tls_certificate"] = (state / "tls" / "cert.pem").read_bytes()
    return facts


def bound_path(services, destination, writable):
    matches = [mount["Source"] for mount in services["collector"]["Mounts"]
               if mount["Destination"] == destination and mount["Type"] == "bind" and mount["RW"] == writable]
    require(len(matches) == 1, f"Unexpected Collector binding at {destination}")
    return Path(matches[0])


def running_stack(project, companions, after):
    until = time.monotonic() + 120
    while True:
        try:
            services = stack(project, companions)
            ready(services, after)
            return services
        except ValueError:
            if not after or time.monotonic() >= until:
                raise
            time.sleep(2)


def wait_for_bridge(reporter, timeout):
    until = time.monotonic() + timeout
    while True:
        message, status = bridge_status(reporter, time.time(), require_synced=True)
        if status == 0:
            return message
        require(status != SYNC_INTERRUPTED, message)
        remaining = until - time.monotonic()
        if remaining <= 0:
            raise ValueError(f"Timed out waiting for a fresh completed Bridge sync. {message}" if timeout else message)
        time.sleep(min(2, remaining))


def wait_for_receipt_scanner(state, started_at, timeout=180):
    started = datetime.fromisoformat(started_at.replace("Z", "+00:00"))
    until = time.monotonic() + timeout
    while True:
        with database(state / "private.sqlite", stopped=False) as db:
            enabled = [record for _, _, record in mail_connections(db) if record["enabled"]]
        if not enabled:
            return "Receipt scanner: no enabled connections."
        problems = sum(bool(record.get("problem") or record.get("failure")) for record in enabled)
        pending = sum(
            not record.get("last_checked")
            or datetime.fromisoformat(record["last_checked"].replace("Z", "+00:00")) <= started
            for record in enabled
        )
        if not problems and not pending:
            return f"Receipt scanner: successful post-start checks for {len(enabled)} enabled connections."
        remaining = until - time.monotonic()
        if remaining <= 0:
            raise ValueError(
                f"Receipt scanner verification timed out ({problems} reported a problem, "
                f"{pending} waiting for a successful post-start check)."
            )
        time.sleep(min(2, remaining))


def snapshot_identity(pair):
    path = pair.resolve() / "fi-data"
    require(path.resolve(strict=True) == path, "Use the pair's read-only application-state snapshot")
    command_options = dict(text=True, timeout=15, stderr=subprocess.DEVNULL, env=dict(os.environ, LC_ALL="C"))
    properties = subprocess.check_output(["btrfs", "property", "get", "-ts", str(path), "ro"], **command_options)
    require(properties.strip() == "ro=true", "Use the pair's read-only application-state snapshot")
    metadata = subprocess.check_output(["btrfs", "subvolume", "show", str(path)], **command_options)
    match = re.search(r"^\s*UUID:\s*([a-f0-9-]+)\s*$", metadata, re.MULTILINE)
    require(match, "Application-state snapshot UUID is missing")
    return path, str(UUID(match[1]))


def fingerprints(facts, config):
    values = {name: value if isinstance(value, bytes) else json.dumps(value, sort_keys=True).encode()
              for name, value in facts.items()}
    values["collector_config"] = config.read_bytes()
    return {name: sha256(value).hexdigest() for name, value in values.items()}


def write_record(path, value):
    with tempfile.NamedTemporaryFile(mode="w", prefix=".fi-check-", dir=path.parent) as output:
        json.dump(value, output, indent=2)
        output.write("\n")
        output.flush()
        os.fsync(output.fileno())
        os.link(output.name, path)


def seal_cutoff(args, request, before):
    require(args.nas_snapshot_uuid, "Supply the NAS snapshot UUID from the sealed pair record")
    status = subprocess.run(
        ["systemctl", "is-active", "junk-collector-remote.service"],
        capture_output=True, text=True, timeout=10,
    )
    require(status.stdout.strip() == "inactive", "Stop the guarded systemd service before checking the cutoff")
    services = stack(args.project, [])
    expected = {name: facts for name, facts in before["services"].items() if not name.startswith("external:")}
    require(container_facts(services) == expected, "Compose services changed before the cutoff")
    require(all(not service["State"]["Running"] and not service["State"]["Restarting"] for service in services.values()),
            "Stop all Compose services before checking the cutoff")
    snapshot, snapshot_uuid = snapshot_identity(args.pair_dir)
    cutoff = (snapshot / args.state_in_snapshot).resolve(strict=True)
    config = (snapshot / "config" / "collector.toml").resolve(strict=True)
    state = bound_path(services, "/data/state", True).resolve()
    require(cutoff.is_relative_to(snapshot) and not cutoff.is_relative_to(state) and config.is_relative_to(snapshot),
            "Use state and config inside the stopped application-state snapshot")
    facts = fingerprints(state_facts(cutoff, stopped=True), config)
    write_record(args.pair_dir / "cutoff.json", {
        "request": request, "fi_snapshot_uuid": snapshot_uuid,
        "nas_snapshot_uuid": args.nas_snapshot_uuid, "fingerprints": facts,
    })


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("phase", choices=("before", "cutoff", "after"))
    parser.add_argument("--project", required=True)
    parser.add_argument("--release", type=Path, required=True)
    parser.add_argument("--pair-dir", type=Path, required=True)
    parser.add_argument("--public", type=origin, required=True)
    parser.add_argument("--lan", type=origin, required=True)
    parser.add_argument("--lan-ca", type=Path, help="Retained LAN certificate or CA when it is outside system trust")
    parser.add_argument("--media", type=origin, required=True)
    parser.add_argument("--item", type=lambda value: str(UUID(value)), required=True)
    parser.add_argument("--companion", action="append", default=[], help="Retained container outside this Compose project")
    parser.add_argument("--state-in-snapshot", type=Path, default=Path("state"))
    parser.add_argument("--nas-snapshot-uuid", type=lambda value: str(UUID(value)),
                        help="NAS snapshot attested by the operator during cutoff")
    args = parser.parse_args()
    try:
        require(args.phase == "cutoff" or sys.stdin.isatty(),
                "Run interactively to enter an existing browser session securely")
        require(args.pair_dir.is_dir() and args.pair_dir.stat().st_mode & 0o077 == 0,
                "Use a private recovery pair directory")
        origins = {name: getattr(args, name) for name in ("public", "lan", "media")}
        require(len(set(origins.values())) == 3, "Provide three distinct HTTPS origins")
        record = args.pair_dir / "before.json"
        image = target(args.release)
        request = {"project": args.project, "target": image, "origins": origins, "item": args.item}
        after = args.phase == "after"
        if args.phase != "before":
            before = json.loads(record.read_text())
            require(before["request"] == request, "Release, origins or sample item changed")
        if args.phase == "cutoff":
            seal_cutoff(args, request, before)
            print("Stopped-stack cutoff passed. The sealed pair is ready for the image switch.")
            return
        if after:
            cutoff = json.loads((args.pair_dir / "cutoff.json").read_text())
            require(cutoff["request"] == request and cutoff["nas_snapshot_uuid"],
                    "Use the cutoff receipt from this update's sealed pair")
            _, snapshot_uuid = snapshot_identity(args.pair_dir)
            require(cutoff["fi_snapshot_uuid"] == snapshot_uuid, "Application-state snapshot changed after cutoff validation")
        require(subprocess.run(
            ["systemctl", "is-active", "--quiet", "junk-collector-remote.service"],
            timeout=10, stderr=subprocess.DEVNULL,
        ).returncode == 0, "the guarded systemd service is not active")
        services = running_stack(args.project, args.companion, after)
        current = container_facts(services)
        state = bound_path(services, "/data/state", True)
        reporter = bound_path(services, "/run/collector-bridge-status", False) / "status.json"
        bridge = wait_for_bridge(reporter, timeout=120 if after else 0)
        session = getpass("Existing collector_session cookie value: ")
        require(re.fullmatch(r"[a-f0-9]{64}", session), "Invalid session cookie value")
        sample = probe(origins, args.item, session, args.lan_ca)
        if not after:
            for image_id in {services[name]["Image"] for name in UPDATED}:
                old_image, = docker_json("image", "inspect", image_id)
                old_version = beta_version(old_image["Config"]["Labels"]["org.opencontainers.image.version"])
                require((0, 1, 0, 22) <= old_version < beta_version(image["version"]),
                        "Routine updates require an installed beta.22 or newer and a later target beta")
            state_facts(state)
            write_record(record, {"request": request, "services": current, "sample": sample})
            print(f"Pre-update checks passed. Private baseline saved to {record}. {bridge}")
        else:
            require(before["sample"] == sample, "ServerId, signed-in account or retained item changed")
            compare_stack(before["services"], current, image)
            receipts = wait_for_receipt_scanner(state, services["collector"]["State"]["StartedAt"])
            config = bound_path(services, "/data/config", False) / "collector.toml"
            old = cutoff["fingerprints"]
            new = fingerprints(state_facts(state), config)
            require(old.keys() == new.keys(), "Cutoff receipt has incomplete state evidence")
            changed = [name for name in old if old[name] != new[name]]
            require(not changed, "Retained state differs from the cutoff: " + ", ".join(changed))
            print(f"Collector {image['version']} passed image, runtime, HTTPS, authenticated read and cutoff checks. {bridge}")
            print(receipts)
    except (ValueError, OSError, KeyError, TypeError, sqlite3.Error, subprocess.SubprocessError) as error:
        # Keep response bodies, Docker output and credentials out of operational logs.
        if type(error) is ValueError:
            parser.exit(1, f"Update check failed: {error}\n")
        parser.exit(1, f"Update check could not read required evidence ({type(error).__name__})\n")


if __name__ == "__main__":
    main()
