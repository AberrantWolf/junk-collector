#!/usr/bin/env python3
"""Qualify the shipped media container against disposable data, never a real archive.

Requires Docker Compose v2. Does not open a port, run ACME, or change host networking.
The absent core authority deliberately yields no successful authenticated playback;
real core/playback acceptance is exercised by Rust and browser integration tests.
"""
import argparse
import json
import os
from pathlib import Path
import secrets
import socket
import ssl
import urllib.error
import urllib.request
import subprocess
import tempfile
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("image")
    parser.add_argument("--proxy", action="store_true", help="Also exercise the shipped Caddy routes with a disposable trusted certificate")
    args = parser.parse_args()
    project = Path(__file__).resolve().parents[1]
    name = "collector-media-check-" + secrets.token_hex(6)
    with tempfile.TemporaryDirectory(prefix=name + "-", dir="/tmp") as directory:
        fixture = Path(directory)
        env = dict(os.environ, COLLECTOR_IMAGE=args.image, COLLECTOR_DATA=directory,
                   COLLECTOR_CONTROL_ORIGIN="https://control.example", COLLECTOR_MEDIA_HOST="media.example",
                   COLLECTOR_MEDIA_BIND_IP="127.0.0.1")
        compose = ["docker", "compose", "-p", name, "-f", str(project / "deploy/container/compose.yaml"),
                   "-f", str(project / "deploy/container/compose.media.yaml")]

        def run(command, *, check=True, **kwargs):
            return subprocess.run(command, env=env, check=check, capture_output=True, text=True,
                                  timeout=60, **kwargs)

        def tool(shell, mounts, user="10001:10001", check=True):
            return run(["docker", "run", "--rm", "--network", "none", "--read-only", "--cap-drop", "ALL",
                        "--security-opt", "no-new-privileges:true", "--user", user,
                        *mounts, "--entrypoint", "/bin/sh", args.image, "-ec", shell], check=check)

        # This root helper touches only the newly created disposable fixture.
        # Keep owner write permissions so the later denial proves the mount boundary.
        run(["docker", "run", "--rm", "--network", "none", "--read-only", "--user", "0:0",
             "-v", directory + ":/fixture", "--entrypoint", "/bin/sh", args.image, "-ec",
             "mkdir -p /fixture/archive/nested /fixture/media-authority /fixture/media-http /fixture/child; "
             "printf 'valuable original' > /fixture/archive/original; "
             "printf 'nested original' > /fixture/archive/nested/original; "
             "printf '%064d' 0 > /fixture/media-authority/service-key; "
             "chown -R 10001:10001 /fixture/archive /fixture/media-authority /fixture/media-http /fixture/child; chmod 750 /fixture/media-authority /fixture/media-http; "
             "chmod 640 /fixture/media-authority/service-key"])
        run(compose + ["run", "--rm", "--no-deps", "initialize-media"])
        run(compose + ["run", "--rm", "--no-deps", "--entrypoint", "/bin/sh", "initialize-media", "-ec",
                       "printf '%064d' 0 > /data/media-authority/service-key; chown 10001:10001 /data/media-authority/service-key; chmod 640 /data/media-authority/service-key"])
        archive = ["-v", directory + "/archive:/media"]
        tool("touch /media/writable-control; rm /media/writable-control", archive)
        config = json.loads(run(compose + ["config", "--format", "json"]).stdout)
        media = config["services"]["media"]
        assert media["network_mode"] == "none" and media["read_only"]
        assert media["user"] == "10001:10001" and media["cap_drop"] == ["ALL"]
        try:
            run(compose + ["run", "--no-deps", "-d", "--name", name, "media"])
            for _ in range(100):
                if run(["docker", "exec", name, "test", "-S", "/run/collector-media/http.sock"], check=False).returncode == 0:
                    break
                time.sleep(.1)
            else:
                raise AssertionError("media did not start: " + run(["docker", "logs", name], check=False).stderr)
            inspected = json.loads(run(["docker", "inspect", name]).stdout)[0]
            assert inspected["Config"]["Image"] == media["image"]
            assert inspected["HostConfig"]["NetworkMode"] == "none"
            assert inspected["HostConfig"]["ReadonlyRootfs"]
            mounts = {m["Destination"]: m for m in inspected["Mounts"]}
            required = {"/media", "/run/collector-authority", "/run/collector-media"}
            assert required <= set(mounts) <= required | {"/tmp"}, set(mounts)
            assert "/tmp" in inspected["HostConfig"]["Tmpfs"]
            assert not mounts["/media"]["RW"] and not mounts["/run/collector-authority"]["RW"]
            before = tool("sha256sum /media/original /media/nested/original", archive).stdout
            for mutation in ["touch /media/new", "printf damaged > /media/original",
                             "mv /media/original /media/renamed", "rm /media/original",
                             "printf damaged > /media/nested/original", "touch /etc/media-check",
                             "printf changed > /run/collector-authority/service-key"]:
                result = run(["docker", "exec", name, "/bin/sh", "-ec", mutation], check=False)
                assert result.returncode != 0, "mutation unexpectedly succeeded: " + mutation
            assert tool("sha256sum /media/original /media/nested/original", archive).stdout == before
            assert len(run(["docker", "exec", name, "cat", "/proc/net/route"]).stdout.splitlines()) == 1
            interfaces = run(["docker", "exec", name, "cat", "/proc/net/dev"]).stdout.splitlines()[2:]
            for line in interfaces:
                if not line.strip(): continue
                interface = line.split(":")[0].strip()
                flags = int(run(["docker", "exec", name, "cat", f"/sys/class/net/{interface}/flags"]).stdout.strip(), 16)
                assert interface == "lo" or not flags & 1, "non-loopback interface is up: " + interface
            connection = run(["docker", "exec", name, "/bin/bash", "-c", "echo probe > /dev/tcp/192.0.2.1/443"], check=False)
            assert connection.returncode != 0 and "Network is unreachable" in connection.stderr
            assert run(["docker", "exec", name, "test", "!", "-e", "/data/state"], check=False).returncode == 0
            # The production guard must reject a writable parent AND writable nested mount.
            base = ["docker", "run", "--rm", "--network", "none", "--read-only", "--cap-drop", "ALL",
                    "--user", "10001:10001", "--entrypoint", "/usr/local/bin/collector-media"]
            for extra in [archive, ["-v", directory + "/archive:/media:ro", "-v", directory + "/child:/media/nested"]]:
                result = run(base + extra + [args.image, "--require-read-only-archive", "--archive", "/media",
                             "--authority-socket", "/missing/socket"], check=False)
                assert result.returncode != 0 and "writable" in result.stderr, result.stderr
            if args.proxy:
                proxy_fixture(run, project, fixture, name, mounts["/run/collector-media"]["Name"], config["services"]["media-https"]["image"])
            print("Media container passed: actual shipped Compose profile, no external routes or active interfaces, "
                  "read-only archive/root/authority, create/overwrite/rename/unlink denied, original digests retained, "
                  "writable parent and nested mounts rejected. Disposable Linux fixture only; NAS and public TLS unqualified.")
        finally:
            run(["docker", "rm", "-f", name], check=False)
            run(compose + ["down", "--remove-orphans", "--volumes"], check=False)
            run(["docker", "run", "--rm", "--network", "none", "--read-only", "--user", "0:0",
                 "-v", directory + ":/fixture", "--entrypoint", "/bin/sh", args.image,
                 "-ec", "chmod -R a+rwX /fixture"], check=False)


def proxy_fixture(run, project, fixture, name, http_volume, proxy_image):
    """No ACME or WAN use: trust only a generated certificate in this Python context."""
    tls = fixture / "test-tls";tls.mkdir()
    run(["openssl", "req", "-x509", "-newkey", "rsa:2048", "-nodes", "-days", "1", "-subj", "/CN=localhost",
         "-addext", "subjectAltName=DNS:localhost", "-keyout", str(tls / "key.pem"), "-out", str(tls / "cert.pem")])
    tls.chmod(0o755)
    (tls / "key.pem").chmod(0o644)
    (tls / "cert.pem").chmod(0o644)
    caddyfile = fixture / "Caddyfile"
    # Keep the shipped routes/security policy; substitute only certificate acquisition.
    source = (project / "deploy/container/Caddyfile.media").read_text()
    source = source.replace("    tls {\n        issuer acme {\n            disable_http_challenge\n        }\n    }", "    tls /test-tls/cert.pem /test-tls/key.pem")
    caddyfile.write_text(source)
    caddyfile.chmod(0o644)
    for part in ["proxy-data", "proxy-config"]:
        (fixture / part).mkdir();(fixture / part).chmod(0o777)
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0));port = sock.getsockname()[1]
    proxy = name + "-proxy"
    try:
        run(["docker", "run", "-d", "--name", proxy, "--user", "10001:10001", "--read-only",
             "--cap-drop", "ALL", "--cap-add", "NET_BIND_SERVICE", "--security-opt", "no-new-privileges:true", "--memory", "128m", "--pids-limit", "64",
             "--tmpfs", "/tmp:rw,nosuid,nodev,noexec,size=16m", "-e", "COLLECTOR_MEDIA_HOST=localhost",
             "-p", f"127.0.0.1:{port}:8443", "-v", str(caddyfile) + ":/etc/caddy/Caddyfile:ro",
             "-v", str(tls) + ":/test-tls:ro", "-v", http_volume + ":/run/collector-media:ro",
             "-v", str(fixture / "proxy-data") + ":/data", "-v", str(fixture / "proxy-config") + ":/config", proxy_image])
        context = ssl.create_default_context(cafile=str(tls / "cert.pem"))
        def status(path, host=None):
            request = urllib.request.Request(f"https://localhost:{port}" + path, headers={"Host": host} if host else {})
            try:
                with urllib.request.urlopen(request, context=context, timeout=3) as response:return response.status
            except urllib.error.HTTPError as error:return error.code
        for _ in range(100):
            try:
                if status("/api/v1/identity") == 503:break  # real media reached; authority deliberately absent
            except (OSError, urllib.error.URLError):pass
            time.sleep(.1)
        else:
            logs = run(["docker", "logs", proxy], check=False)
            raise AssertionError("TLS proxy did not reach media: " + logs.stdout + logs.stderr)
        assert status("/api/v1/auth/login") == 404
        assert status("/api/v1/identity", "unrelated.example") == 421
        assert status("/read", "127.0.0.1") == 421
        # Force a reverse-proxy error and ensure its runtime log filter removes URL capabilities.
        run(["docker", "stop", "-t", "2", name])
        secret = secrets.token_hex(32)
        assert status("/read?grant=" + secret) == 502
        time.sleep(.2)
        logs = run(["docker", "logs", proxy])
        assert secret not in logs.stdout + logs.stderr, "proxy logged a capability URL"
        print("Caddy passed: fixture-trusted TLS, real Unix upstream, unrelated routes and wrong/IP Host denied, capability absent from forced upstream-error logs. ACME/WAN not tested.")
    finally:
        run(["docker", "rm", "-f", proxy], check=False)


if __name__ == "__main__":
    main()
