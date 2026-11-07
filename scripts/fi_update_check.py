#!/usr/bin/env python3
"""Check a routine Fi beta update against a published image and a stopped recovery point."""

import argparse
from contextlib import closing, contextmanager, nullcontext
from getpass import getpass
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

from bridge_status import query as bridge_status


UPDATED = {"collector", "media"}
REQUIRED = UPDATED | {"media-https", "proton-bridge"}
DIGEST = re.compile(r"sha256:[a-f0-9]{64}")
BETA = re.compile(r"(\d+)\.(\d+)\.(\d+)-beta\.(\d+)")


def require(condition, message):
    if not condition:
        raise ValueError(message)


def docker_json(*args):
    return json.loads(subprocess.check_output(["docker", *args], timeout=15, stderr=subprocess.DEVNULL))


def beta_version(value):
    match = BETA.fullmatch(value)
    require(match, "Routine Fi updates require published beta versions")
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
        "Pulled Fi image does not match the published release",
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
    require(REQUIRED <= services.keys(), "Compose project is missing a required Fi service")
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
    require(baseline.keys() == current.keys(), "Fi service set changed")
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
    require(len(set(identities.values())) == 1, "Fi HTTPS origins disagree on ServerId")
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
            "mail_connections": db.execute("SELECT id, owner, record FROM mail_connections ORDER BY id").fetchall(),
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


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("phase", choices=("before", "after"))
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
    args = parser.parse_args()
    try:
        require(sys.stdin.isatty(), "Run interactively to enter an existing browser session securely")
        require(args.pair_dir.is_dir() and args.pair_dir.stat().st_mode & 0o077 == 0,
                "Use a private recovery pair directory")
        origins = {name: getattr(args, name) for name in ("public", "lan", "media")}
        require(len(set(origins.values())) == 3, "Provide three distinct Fi HTTPS origins")
        record = args.pair_dir / "before.json"
        image = target(args.release)
        request = {"project": args.project, "target": image, "origins": origins, "item": args.item}
        after = args.phase == "after"
        if after:
            before = json.loads(record.read_text())
            require(before["request"] == request, "Release, origins or sample item changed")
        require(subprocess.run(
            ["systemctl", "is-active", "--quiet", "junk-collector-remote.service"],
            timeout=10, stderr=subprocess.DEVNULL,
        ).returncode == 0, "Fi's guarded systemd service is not active")
        services = running_stack(args.project, args.companion, after)
        current = container_facts(services)
        state = bound_path(services, "/data/state", True)
        reporter = bound_path(services, "/run/collector-bridge-status", False) / "status.json"
        bridge, status = bridge_status(reporter, time.time(), require_synced=True)
        require(status == 0, bridge)
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
            with os.fdopen(os.open(record, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600), "w") as output:
                json.dump({"request": request, "services": current, "sample": sample}, output, indent=2)
                output.write("\n")
            print(f"Pre-update checks passed. Private baseline saved to {record}. {bridge}")
        else:
            require(before["sample"] == sample, "ServerId, signed-in account or retained item changed")
            compare_stack(before["services"], current, image)
            snapshot = (args.pair_dir / "fi-data").resolve(strict=True)
            require(snapshot == args.pair_dir.resolve() / "fi-data" and subprocess.check_output(
                ["btrfs", "property", "get", "-ts", str(snapshot), "ro"],
                text=True, timeout=15, stderr=subprocess.DEVNULL,
            ).strip() == "ro=true", "Use the pair's read-only Fi snapshot")
            cutoff = (snapshot / args.state_in_snapshot).resolve(strict=True)
            require(cutoff.is_relative_to(snapshot) and not cutoff.is_relative_to(state.resolve()),
                    "Use state inside the stopped Fi snapshot")
            old = state_facts(cutoff, stopped=True)
            new = state_facts(state)
            changed = [name for name in old if old[name] != new[name]]
            require(not changed, "Retained state differs from the cutoff: " + ", ".join(changed))
            config = bound_path(services, "/data/config", False) / "collector.toml"
            require(config.read_bytes() == (snapshot / "config" / "collector.toml").read_bytes(),
                    "Collector configuration differs from the cutoff")
            print(f"Fi {image['version']} passed image, runtime, HTTPS, authenticated read and cutoff checks. {bridge}")
    except (ValueError, OSError, KeyError, TypeError, sqlite3.Error, subprocess.SubprocessError) as error:
        # Keep response bodies, Docker output and credentials out of operational logs.
        if type(error) is ValueError:
            parser.exit(1, f"Fi update check failed: {error}\n")
        parser.exit(1, f"Fi update check could not read required evidence ({type(error).__name__})\n")


if __name__ == "__main__":
    main()
