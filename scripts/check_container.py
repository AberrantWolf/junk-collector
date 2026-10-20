#!/usr/bin/env python3
"""Qualify shipped Compose operations with disposable data on a Linux Docker host.

Requires Python 3, Docker/Compose v2 and four free loopback ports. Never points at
an operator installation. Optional browser handoff exposes only generated fixture
credentials in a private temporary file; no production credential is read.
"""
import argparse
from contextlib import ExitStack
import hashlib
import ipaddress
import json
import os
import re
from pathlib import Path
import secrets
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request
import uuid


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("image")
    parser.add_argument("--update-from", help="Existing image used before candidate update")
    parser.add_argument("--data-parent", type=Path, help="Existing writable parent on the intended filesystem")
    parser.add_argument("--port-base", type=int, default=9841)
    parser.add_argument("--proxy", action="store_true", help="Exercise the shipped proxy configuration transport")
    parser.add_argument("--inbox", action="store_true", help="Exercise the shipped supplemental-group inbox override")
    parser.add_argument("--provider-user-agent-file", type=Path, help="Optional real monitored provider contact; never printed")
    parser.add_argument("--browser-host", help="Private DNS name used by an external qualification browser")
    parser.add_argument("--private-ip", help="Explicit private IPv4 bind for the isolated browser fixture")
    parser.add_argument("--browser-handoff", type=Path, help="Write private fixture details and wait for FILE.done before restore")
    parser.add_argument("--snapshot-tool-image", help="Disposable Btrfs/GNU-tar tools image; requires a Btrfs data parent")
    parser.add_argument("--snapshot-handoff", type=Path, help="Private independent-storage roundtrip handoff (snapshot mode)")
    parser.add_argument("--portainer-image", help="Run service deploy/update through a disposable Portainer instance")
    parser.add_argument("--portainer-keep-companion", action="store_true", help="Check service-targeted updates preserve an unchanged companion container")
    parser.add_argument("--preparation-faults", action="store_true", help="Qualify actual encoder cancellation/failure/restart through private browser handoffs")
    parser.add_argument("--audio-oracles", action="store_true", help="Qualify real FLAC and Opus profiles against independent tool/PCM oracles and browser journeys")
    parser.add_argument("--storage-permissions", action="store_true", help="Qualify generated archive write-permission failure and browser retry/recovery")
    parser.add_argument("--storage-full", action="store_true", help="Qualify ENOSPC in a dedicated bounded 96 MiB tmpfs archive, with real restart/retry/recovery")
    parser.add_argument("--authentication-limits", action="store_true", help="Qualify actual Linux TCP account/peer/global login limits and restart reset")
    args = parser.parse_args()
    if bool(args.snapshot_tool_image) != bool(args.snapshot_handoff):
        parser.error("Snapshot qualification requires both tools image and independent-storage handoff")
    if args.snapshot_tool_image and not args.update_from:
        parser.error("Snapshot qualification requires two distinct update images")
    if not 1024 <= args.port_base <= 65532:
        parser.error("Choose four consecutive non-privileged ports")
    if args.portainer_image and (args.port_base > 65531 or args.snapshot_tool_image):
        parser.error("Portainer requires a fifth port and uses the full-backup recovery journey")
    if args.portainer_keep_companion and not args.portainer_image:
        parser.error("Companion retention qualification requires Portainer")
    if bool(args.browser_host) != bool(args.private_ip):
        parser.error("Use --browser-host and --private-ip together")
    if args.browser_handoff and not args.browser_host:
        parser.error("Browser handoff requires an explicit private HTTPS host/bind")
    if args.private_ip and not ipaddress.IPv4Address(args.private_ip).is_private:
        parser.error("Browser qualification binds only an explicit private IPv4 address")
    if sum((args.preparation_faults, args.audio_oracles, args.storage_permissions, args.storage_full, args.authentication_limits)) > 1:
        parser.error("Choose one bounded qualification journey")
    if (args.preparation_faults or args.audio_oracles or args.storage_permissions or args.storage_full or args.authentication_limits) and (not args.inbox or not args.browser_handoff or args.update_from or args.snapshot_tool_image or args.portainer_image):
        parser.error("Fault/audio qualification requires a fresh Compose inbox/browser installation")
    project_root = Path(__file__).resolve().parents[1]
    compose = project_root / "deploy/container/compose.yaml"
    password = secrets.token_hex(24)
    origin = f"http://localhost:{args.port_base + 1}"
    user_agent = args.provider_user_agent_file.read_text().strip() if args.provider_user_agent_file else None
    with tempfile.TemporaryDirectory(prefix="collector-compose-", dir=args.data_parent) as temporary, ExitStack() as resources:
        snapshots = None
        original_root = Path(temporary)
        if args.snapshot_tool_image:
            from snapshot_qualification import SnapshotQualification
            snapshots = SnapshotQualification(original_root, args.snapshot_tool_image, args.snapshot_handoff)
            original_root = snapshots.create()
        env = dict(os.environ, COLLECTOR_IMAGE=args.update_from or args.image,
                   COLLECTOR_DATA=str(original_root), COLLECTOR_HOST="localhost",
                   COLLECTOR_PRIVATE_IP="127.0.0.1")
        command = ["docker", "compose", "-p", f"collector-smoke-{os.getpid()}-{secrets.token_hex(4)}", "-f", str(compose)]
        cue = b'FILE "disc.bin" BINARY\n TRACK 01 AUDIO\n INDEX 01 00:00:00\n'
        pcm = b'\0' * (2352 * 225)
        if args.audio_oracles:
            from audio_oracle_qualification import AudioOracleQualification
            pcm = AudioOracleQualification.source_pcm(588 * 225)
        originals = {"disc.cue": cue, "disc.bin": pcm, ".unknown": b"Keep this hidden collection note"}
        inbox = original_root / "inbox-fixture"
        if args.inbox:
            for name, content in originals.items():
                path = inbox / "Reference disc" / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(content)
            env.update(COLLECTOR_INBOX=str(inbox), COLLECTOR_INBOX_GID="65538")
            command.extend(["-f", str(project_root / "deploy/container/compose.inbox.yaml")])

        def run(*arguments, **kwargs):
            return subprocess.run(command + list(arguments), env=env, check=True,
                                  timeout=120, **kwargs)

        portainer = None
        faults = None
        audio = AudioOracleQualification(run) if args.audio_oracles else None

        def up(recreate=False):
            if portainer:
                portainer.deploy()
            else:
                run("up", "-d", *(("--force-recreate",) if recreate else ()), "collector", capture_output=True)

        def root_tool(tool, *arguments):
            return run("run", "--rm", "--entrypoint", tool, "initialize", *arguments,
                       capture_output=True)

        def read_file(path):
            return run("run", "--rm", "--entrypoint", "cat", "cli", path,
                       capture_output=True).stdout

        def request(path, body=None, cookie=None, method=None, headers=None, raw=False):
            extra = {"Origin": origin, **(headers or {})}
            if cookie:
                extra["Cookie"] = cookie
            if isinstance(body, dict):
                body = json.dumps(body).encode()
                extra["Content-Type"] = "application/json"
            if body is not None:
                extra.setdefault("Content-Type", "application/octet-stream")
                extra.setdefault("Idempotency-Key", str(uuid.uuid4()))
            req = urllib.request.Request(origin + "/api/v1" + path, data=body,
                                         headers=extra, method=method)
            with urllib.request.urlopen(req, timeout=15) as response:
                content = response.read()
                return (response.status, {key.lower(): value for key, value in response.headers.items()}, content) if raw else json.loads(content)

        def ready():
            deadline = time.monotonic() + 45
            while time.monotonic() < deadline:
                try:
                    with urllib.request.urlopen(origin + "/", timeout=1) as response:
                        if b"/app.js" in response.read():
                            return
                except (OSError, urllib.error.URLError):
                    pass
                time.sleep(0.2)
            raise AssertionError("Compose service did not become ready")

        def login():
            status, headers, _ = request("/auth/login", dict(username="owner", password=password,
                                                            device_name="Compose qualification"), raw=True)
            assert status == 200
            # Explicit fixture cookie on loopback HTTP; browser trust is qualified separately.
            return headers["set-cookie"].split(";", 1)[0]

        def stop():
            started = time.monotonic()
            if portainer:
                portainer.stop()
            else:
                run("stop", "-t", "10", "collector", capture_output=True)
            identifier = run("ps", "-aq", "collector", capture_output=True, text=True).stdout.strip()
            state = subprocess.check_output(["docker", "inspect", "--format", "{{json .State}}", identifier], text=True)
            assert json.loads(state)["ExitCode"] == 0, state
            print(f"Clean SIGTERM stop: {time.monotonic()-started:.2f}s", flush=True)

        def wait_import(cookie, import_id):
            deadline = time.monotonic() + 180
            while time.monotonic() < deadline:
                detail = request(f"/ingests/{import_id}", cookie=cookie)
                if detail["import"]["phase"] == "published":
                    return detail
                failed = [step for step in detail["steps"] if step["state"] == "failed" and step["step"] == "preservation"]
                if failed:
                    raise AssertionError(failed)
                time.sleep(0.5)
            raise AssertionError("Preservation did not settle")

        def plan(cookie, collection, source, files):
            return request("/ingests/plan", dict(collection_id=collection, packages=[dict(source=source, files=files)]), cookie)

        def start(cookie, planned, prepare=False, enrichment="none"):
            return request("/ingests", dict(plan=planned, enrichment=enrichment, prepare_music=prepare), cookie)["imports"][0]["import_id"]

        def verify_originals(cookie, item_id, expected):
            item = request(f"/items/{item_id}", cookie=cookie)
            assert len(item["captures"]) == 1, "Restart/update duplicated the import capture"
            seen = set()
            for capture in item["captures"]:
                for representation in capture["representations"]:
                    if representation["role"] != "preservation_master":
                        continue
                    for file in representation["files"]:
                        status, _, content = request(f"/items/{item_id}/representations/{representation['id']}/{file['index']}", cookie=cookie, raw=True)
                        assert status == 200 and content == expected[file["name"]], file["name"]
                        seen.add(file["name"])
            assert seen == set(expected), "A saved original is missing"
            return item

        handoff_files = []
        storage = None
        if args.storage_permissions:
            from storage_permission_qualification import StoragePermissionQualification
            storage = StoragePermissionQualification(root_tool)
        capacity = None
        def restart():
            stop()
            up()
            ready()
        if args.storage_full:
            from storage_capacity_qualification import StorageCapacityQualification
            capacity = StorageCapacityQualification(command, env, args.image, root_tool, restart)
            resources.callback(capacity.cleanup)
            storage = capacity
        authentication = None
        if args.authentication_limits:
            from authentication_rate_qualification import AuthenticationRateQualification
            authentication = AuthenticationRateQualification(origin, f"http://localhost:{args.port_base}", password)

        def browser_handoff(album_id, restored=False, scenario=None, **extra):
            if not args.browser_handoff:
                return
            target = args.browser_handoff
            if scenario:
                target = target.with_name(target.stem + "-" + scenario + target.suffix)
            if restored:
                target = target.with_name(target.stem + "-restored" + target.suffix)
            # Publish a complete closed inode without replacing an existing path.
            # A fast external browser driver must never read a partly written secret.
            if Path(str(target) + ".done").exists():
                raise ValueError("Browser handoff has a stale completion marker")
            descriptor, pending = tempfile.mkstemp(prefix=target.name + ".pending-", dir=target.parent)
            try:
                with os.fdopen(descriptor, "w") as output:
                    json.dump(dict(scenario=scenario, **extra, origin=browser_origin, private_ip=args.private_ip, username="owner", password=password, collection_id=collection, item_id=album_id, certificate_pem=read_file("/data/state/tls/cert.pem").decode(), server_id=server_id, originals={name:dict(bytes=len(content), sha256=hashlib.sha256(content).hexdigest()) for name,content in originals.items()}), output)
                    output.flush()
                    os.fsync(output.fileno())
                os.link(pending, target)
                handoff_files.append(target)
            finally:
                Path(pending).unlink()
            print(f"Temporary browser fixture ready: {target}; waiting for its .done marker", flush=True)
            deadline = time.monotonic() + 600
            marker = Path(str(target) + ".done")
            while not marker.exists():
                if time.monotonic() > deadline:
                    raise AssertionError("Browser handoff timed out")
                time.sleep(1)
            assert marker.read_text().strip() == "passed", "External browser qualification failed"

        def image_id(reference):
            return subprocess.check_output(["docker", "image", "inspect", "--format", "{{.Id}}", reference], text=True).strip()

        def container_id():
            return run("ps", "-aq", "collector", capture_output=True, text=True).stdout.strip()

        def current_image():
            container = container_id()
            return subprocess.check_output(["docker", "inspect", "--format", "{{.Image}}", container], text=True).strip()

        initial_image = env["COLLECTOR_IMAGE"]
        try:
            if args.portainer_image:
                from portainer_qualification import PortainerQualification
                portainer = PortainerQualification(args.portainer_image, command, env, args.port_base + 4,
                                                   args.portainer_keep_companion, args.image)
                portainer.start()
                if args.image.startswith("ghcr.io/"):
                    portainer.pull(args.image)
            candidate_image_id = image_id(args.image)
            if args.update_from:
                assert image_id(initial_image) != candidate_image_id, "Update qualification needs two distinct images"
            if capacity:
                capacity.new_archive()
            run("run", "--rm", "initialize", capture_output=True)
            initial = read_file("/data/config/collector.toml")
            run("run", "--rm", "initialize", capture_output=True)
            assert initial == read_file("/data/config/collector.toml"), "initialize replaced config"
            config = (project_root / "deploy/container/collector.proxy.toml.example").read_text() if args.proxy else initial.decode()
            for offset in range(4):
                config = config.replace(f":{9841+offset}", f":{args.port_base+offset}")
            if args.browser_host:
                browser_origin = f"https://{args.browser_host}:{args.port_base+3}"
                config = re.sub(r'^private_tls_bind = .*\n', '', config, flags=re.MULTILINE)
                config = f'private_tls_bind = "{args.private_ip}:{args.port_base+3}"\n' + config
                config = re.sub(r'^tls_names = .*$', 'tls_names = ' + json.dumps(["localhost", "127.0.0.1", args.browser_host, args.private_ip]), config, flags=re.MULTILINE)
                config = config.replace('allowed_origins = [', 'allowed_origins = [' + json.dumps(browser_origin) + ', ')
                config = re.sub(r'^authentication_origin = .*$', 'authentication_origin = ' + json.dumps(browser_origin), config, flags=re.MULTILINE)
            else:
                browser_origin = origin
            if args.inbox:
                config += '\n[inboxes]\n"Qualification shelf" = "/inbox"\n'
                root_tool("chown", "-R", f"{os.getuid()}:65538", "/data/inbox-fixture")
                root_tool("chmod", "-R", "g+rX,o-rwx", "/data/inbox-fixture")
            if user_agent:
                config += "\n[providers]\nmusicbrainz_user_agent = " + json.dumps(user_agent) + "\n"
            if args.preparation_faults:
                from preparation_failure_qualification import PreparationFailureQualification
                faults = PreparationFailureQualification(run)
                config = faults.configure(config)
            # Fresh config transport comes from the candidate, before the old-image account exists.
            env["COLLECTOR_IMAGE"] = args.image
            run("run", "--rm", "-T", "--entrypoint", "collector-install-proxy-config", "cli", input=config.encode(), capture_output=True)
            copied = read_file("/data/config/collector.toml")
            assert copied == config.encode(), "proxy config transport changed bytes"
            empty = subprocess.run(command + ["run", "--rm", "-T", "--entrypoint", "collector-install-proxy-config", "cli"], env=env, input=b"", capture_output=True, timeout=120)
            assert empty.returncode != 0 and read_file("/data/config/collector.toml") == copied
            root_tool("touch", "/data/archive/config-guard-fixture")
            guarded = subprocess.run(command + ["run", "--rm", "-T", "--entrypoint", "collector-install-proxy-config", "cli"], env=env, input=b"invalid replacement", capture_output=True, timeout=120)
            assert guarded.returncode != 0 and read_file("/data/config/collector.toml") == copied
            assert read_file("/data/archive/config-guard-fixture") == b""
            root_tool("rm", "/data/archive/config-guard-fixture")
            root_tool("touch", "/data/state/unknown-private-fixture")
            guarded = subprocess.run(command + ["run", "--rm", "-T", "--entrypoint", "collector-install-proxy-config", "cli"], env=env, input=b"invalid replacement", capture_output=True, timeout=120)
            assert guarded.returncode != 0 and read_file("/data/config/collector.toml") == copied
            assert read_file("/data/state/unknown-private-fixture") == b""
            root_tool("rm", "/data/state/unknown-private-fixture")
            if faults:
                faults.install()
            env["COLLECTOR_IMAGE"] = initial_image
            run("run", "--rm", "-T", "cli", "setup", input=(password + "\n").encode(), capture_output=True)
            run("run", "--rm", "cli", "doctor", capture_output=True)
            saved_config = read_file("/data/config/collector.toml")
            certificate = hashlib.sha256(read_file("/data/state/tls/cert.pem")).hexdigest()
            for tool in ("flac", "opusenc"):
                run("run", "--rm", "--entrypoint", tool, "cli", "--version", capture_output=True)
            up()
            ready()
            cookie = login()
            original_cookie = cookie
            if args.browser_host == args.private_ip and args.browser_host:
                features = request("/features", cookie=cookie)
                assert features["passkeys"] is False
                assert features["passkeys_unavailable_reason"] == "dns_name_required"
                print("Numeric-IP authentication startup/password login and typed DNS-required passkey guidance passed", flush=True)
            server_id = request("/identity", cookie=cookie)["server_id"]
            assert current_image() == image_id(initial_image)
            initial_container = container_id()
            owner = request("/me", cookie=cookie)["user_id"]
            if snapshots:
                snapshots.seed_unknown()
            collection = request("/admin/collections", dict(name="NAS qualification", module="org.junk.music"), cookie)["id"]
            # Preserve committed originals as well as unfinished work across the image change.
            saved_plan = plan(cookie, collection, dict(kind="browser", label=None, package_name="Saved originals"),
                              [dict(name=name, bytes=len(content)) for name, content in originals.items()])
            saved_import = start(cookie, saved_plan)
            for index, file in enumerate(saved_plan["packages"][0]["files"]):
                request(f"/imports/{saved_import}/files/{index}?offset=0", originals[file["name"]], cookie, "PUT")
            saved_item = wait_import(cookie, saved_import)["summary"]["target_item_id"]
            verify_originals(cookie, saved_item, originals)
            source = dict(kind="browser", label=None, package_name="Pending upload")
            planned = plan(cookie, collection, source, [dict(name=name, bytes=len(content)) for name, content in originals.items()])
            pending = start(cookie, planned)
            first = planned["packages"][0]["files"][0]
            data = originals[first["name"]]
            partial = max(1, len(data)//2)
            request(f"/imports/{pending}/files/0?offset=0", data[:partial], cookie, "PUT")
            stop()
            if snapshots:
                snapshots.capture("before-update")
            else:
                run("run", "--rm", "cli", "backup", "--destination", "/data/backups/before-update", capture_output=True)
                run("run", "--rm", "cli", "validate-backup", "--backup", "/data/backups/before-update", capture_output=True)
            env["COLLECTOR_IMAGE"] = args.image
            up(recreate=True)
            ready()
            assert current_image() == candidate_image_id
            assert container_id() != initial_container, "Update/recreation retained the old container"
            if snapshots:
                # Export the immutable recovery point after service availability returns.
                snapshots.run("sh", "-c", "printf 'after!' > /fixture/live/state/.qualification-cutoff")
                snapshots.export("before-update")
                run("kill", "-s", "SIGKILL", "collector", capture_output=True)
                up()
                ready()
                assert request(f"/ingests/{pending}", cookie=original_cookie)["import"]["files"][0]["received_bytes"] == partial
                stop()
                env["COLLECTOR_DATA"] = str(snapshots.rollback())
                env["COLLECTOR_IMAGE"] = initial_image
                up(recreate=True)
                ready()
                assert current_image() == image_id(initial_image)
                assert request("/identity", cookie=original_cookie)["server_id"] == server_id
                assert request("/me", cookie=original_cookie)["user_id"] == owner
                assert request(f"/ingests/{pending}", cookie=original_cookie)["import"]["files"][0]["received_bytes"] == partial
                stop()
                env["COLLECTOR_DATA"] = str(original_root)
                env["COLLECTOR_IMAGE"] = args.image
                up(recreate=True)
                ready()
            assert request("/me", cookie=original_cookie)["user_id"] == owner
            cookie = login()
            assert request("/me", cookie=cookie)["user_id"] == owner
            assert request("/identity", cookie=cookie)["server_id"] == server_id
            assert read_file("/data/config/collector.toml") == saved_config
            assert certificate == hashlib.sha256(read_file("/data/state/tls/cert.pem")).hexdigest()
            verify_originals(cookie, saved_item, originals)
            detail = request(f"/ingests/{pending}", cookie=cookie)
            assert detail["import"]["files"][0]["received_bytes"] == partial
            for index, file in enumerate(detail["import"]["files"]):
                offset = file["received_bytes"]
                content = originals[file["name"]]
                if offset < len(content):
                    request(f"/imports/{pending}/files/{index}?offset={offset}", content[offset:], cookie, "PUT")
            detail = wait_import(cookie, pending)
            item_id = detail["summary"]["target_item_id"]
            verify_originals(cookie, item_id, originals)
            if args.inbox:
                # Candidate uses the real share-group override and existing inbox planning/staging.
                planned = plan(cookie, collection, dict(kind="inbox", label="Qualification shelf", package_name="Reference disc"), [])
                assert planned["packages"][0]["metadata_available"] == bool(user_agent)
                assert planned["packages"][0]["artwork_available"] == bool(user_agent)
                imported = start(cookie, planned, prepare=True, enrichment="automatic")
                detail = wait_import(cookie, imported)
                album_id = detail["summary"]["target_item_id"]
                verify_originals(cookie, album_id, originals)
                deadline = time.monotonic() + 180
                while time.monotonic() < deadline:
                    rows = request(f"/items/{album_id}/music-tracks", cookie=cookie)
                    available = [row for row in rows if row["state"]["status"] == "available"]
                    if available:
                        break
                    time.sleep(0.5)
                assert available, "Explicit FLAC preparation did not finish"
                deadline = time.monotonic() + 120
                while time.monotonic() < deadline:
                    optional = request(f"/ingests/{imported}", cookie=cookie)
                    metadata = next(step for step in optional["steps"] if step["step"] == "metadata")
                    if metadata["state"] in ("succeeded", "failed"):
                        break
                    time.sleep(0.5)
                assert metadata["state"] in ("succeeded", "failed"), "Optional metadata did not settle"
                if not user_agent:
                    assert metadata["state"] == "failed" and metadata["message"], "Unconfigured metadata must stay visibly unavailable"
                else:
                    assert metadata["job_id"], "Configured metadata never acquired its provider request"
                print(f"Optional metadata outcome: {metadata['state']} (originals and explicit FLAC remain usable)", flush=True)
                row = available[0]
                status, headers, content = request(f"/items/{album_id}/representations/{row['state']['representation_id']}/{row['state']['file_index']}", cookie=cookie, raw=True, headers={"Range": "bytes=4-15"})
                assert status == 206 and len(content) == 12 and headers["accept-ranges"] == "bytes"
            else:
                album_id = item_id
            if faults or audio:
                def seed(label):
                    planned = plan(cookie, collection, dict(kind="browser", label=None, package_name=label),
                                   [dict(name=name, bytes=len(content)) for name, content in originals.items()])
                    imported = start(cookie, planned)
                    for index, file in enumerate(planned["packages"][0]["files"]):
                        request(f"/imports/{imported}/files/{index}?offset=0", originals[file["name"]], cookie, "PUT")
                    return wait_import(cookie, imported)["summary"]["target_item_id"]
                verify = lambda session, item: verify_originals(session, item, originals)
                if faults:
                    faults.exercise(cookie, seed, request, verify, stop, up, ready, browser_handoff)
                if audio:
                    audio.exercise(cookie, seed, request, verify, browser_handoff, pcm, album_id)
            if storage:
                storage.exercise(cookie, album_id, request, lambda session, item: verify_originals(session, item, originals), browser_handoff)
            if authentication:
                authentication.exercise(cookie, request, restart)
            browser_handoff(album_id)
            stop()
            if snapshots:
                snapshots.capture("after-update")
                # The live instance can serve while backup exports/transport run.
                up()
                ready()
                snapshots.export("after-update", "before-update")
                snapshots.independent_roundtrip(candidate_image_id, server_id)
                stop()
                env["COLLECTOR_DATA"] = str(snapshots.restore())
            else:
                run("run", "--rm", "cli", "backup", "--destination", "/data/backups/qualified", capture_output=True)
                run("run", "--rm", "cli", "validate-backup", "--backup", "/data/backups/qualified", capture_output=True)
                # The candidate cannot replace config after accounts exist, including after update.
                protected = subprocess.run(command + ["run", "--rm", "-T", "--entrypoint", "collector-install-proxy-config", "cli"], env=env, input=b"invalid replacement", capture_output=True, timeout=120)
                assert protected.returncode != 0 and read_file("/data/config/collector.toml") == saved_config
                root_tool("install", "-d", "-m", "0700", "-o", "10001", "-g", "10001", "/data/recovered")
                env["COLLECTOR_DATA"] = str(original_root / "recovered")
                if capacity:
                    capacity.new_archive()
                root_tool("install", "-d", "-m", "0700", "-o", "10001", "-g", "10001", "/data/config", "/data/archive", "/data/state", "/data/backups")
                run("run", "--rm", "-v", f"{original_root}/backups/qualified:/backup:ro", "cli", "restore", "--backup", "/backup", "--archive", "/data/archive", "--state", "/data/state", capture_output=True)
            run("run", "--rm", "cli", "doctor", capture_output=True)
            up(recreate=True)
            ready()
            assert current_image() == candidate_image_id
            assert request("/me", cookie=original_cookie)["user_id"] == owner
            cookie = login()
            assert request("/me", cookie=cookie)["user_id"] == owner
            assert request("/identity", cookie=cookie)["server_id"] == server_id
            verify_originals(cookie, item_id, originals)
            verify_originals(cookie, album_id, originals)
            verify_originals(cookie, saved_item, originals)
            assert certificate == hashlib.sha256(read_file("/data/state/tls/cert.pem")).hexdigest()
            if args.inbox:
                restored_rows = request(f"/items/{album_id}/music-tracks", cookie=cookie)
                restored_audio = next(row for row in restored_rows if row["state"]["status"] == "available")
                status, _, content = request(f"/items/{album_id}/representations/{restored_audio['state']['representation_id']}/{restored_audio['state']['file_index']}", cookie=cookie, raw=True)
                assert status == 200 and hashlib.sha256(content).hexdigest() == restored_audio["state"]["digest"]["hex"]
            if faults:
                faults.verify_restored(cookie, request, lambda session, item: verify_originals(session, item, originals))
            if audio:
                audio.verify_restored(cookie, request, lambda session, item: verify_originals(session, item, originals), pcm, browser_handoff)
            if storage:
                storage.verify_restored(cookie, request, lambda session, item: verify_originals(session, item, originals))
            browser_handoff(album_id, restored=True)
            stop()
            transition = "distinct-image update" if args.update_from else "container recreation"
            lifecycle = "Portainer" if portainer else "Compose"
            print(f"{lifecycle} initialize/config/group access/setup/partial-upload {transition}/SIGTERM/FLAC/backup/restore passed", flush=True)
        except subprocess.CalledProcessError as error:
            # Commands operate on generated fixtures. Retain bounded diagnostics
            # before teardown removes their transient failed state.
            if error.stderr:
                diagnostic = error.stderr.decode(errors="replace") if isinstance(error.stderr, bytes) else error.stderr
                print(diagnostic[-8192:], file=sys.stderr, flush=True)
            raise
        finally:
            try:
                if portainer:
                    portainer.cleanup()
            finally:
                env["COLLECTOR_DATA"] = str(original_root)
                subprocess.run(command + ["down", "-t", "5"], env=env, timeout=30, capture_output=True)
                # Only this generated fixture tree, never an operator installation.
                subprocess.run(command + ["run", "--rm", "--entrypoint", "chown", "initialize", "-R", f"{os.getuid()}:{os.getgid()}", "/data"], env=env, check=True, timeout=60, capture_output=True)
                if snapshots:
                    snapshots.cleanup()
                for target in handoff_files:
                    target.unlink(missing_ok=True)
                    Path(str(target)+".done").unlink(missing_ok=True)


if __name__ == "__main__":
    main()
