#!/usr/bin/env python3
"""Exercise the shipped Compose installation against a local image on Linux.

Requires a Docker daemon, Compose v2+, and free localhost ports 9841-9844.
Uses only disposable data and never writes an operator installation.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time
import urllib.error
import urllib.request


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("image")
    args = parser.parse_args()
    compose = Path(__file__).resolve().parents[1] / "deploy/container/compose.yaml"
    with tempfile.TemporaryDirectory(prefix="collector-compose-") as temporary:
        env = dict(os.environ, COLLECTOR_IMAGE=args.image, COLLECTOR_DATA=temporary,
                   COLLECTOR_HOST="localhost", COLLECTOR_PRIVATE_IP="127.0.0.1")
        command = ["docker", "compose", "-p", f"collector-smoke-{os.getpid()}",
                   "-f", str(compose)]

        def run(*arguments, **kwargs):
            return subprocess.run(command + list(arguments), env=env, check=True,
                                  timeout=90, **kwargs)

        def read_file(path):
            return run("run", "--rm", "--entrypoint", "cat", "cli", path,
                       capture_output=True).stdout

        def ready():
            deadline = time.monotonic() + 30
            while time.monotonic() < deadline:
                try:
                    with urllib.request.urlopen("http://localhost:9842/", timeout=1) as response:
                        assert b"/app.js" in response.read()
                        return
                except (OSError, urllib.error.URLError):
                    time.sleep(0.1)
            raise AssertionError("Compose service did not become ready")

        def login():
            request = urllib.request.Request(
                "http://localhost:9842/api/v1/auth/login",
                data=json.dumps(dict(username="owner", password="container smoke password",
                                     device_name="Compose smoke")).encode(),
                headers={"Content-Type": "application/json", "Origin": "http://localhost:9842"})
            with urllib.request.urlopen(request, timeout=5) as response:
                result = json.load(response)
                assert response.headers.get("Set-Cookie"), "login did not issue a session"
                return result

        def stop():
            run("stop", "-t", "10", "collector")
            identifier = run("ps", "-aq", "collector", capture_output=True, text=True).stdout.strip()
            state = subprocess.check_output(
                ["docker", "inspect", "--format", "{{json .State}}", identifier], text=True)
            assert json.loads(state)["ExitCode"] == 0, state

        try:
            run("run", "--rm", "initialize")
            initial = read_file("/data/config/collector.toml")
            run("run", "--rm", "initialize")
            assert initial == read_file("/data/config/collector.toml"), "initialize replaced config"
            run("run", "--rm", "-T", "cli", "setup", input=b"container smoke password\n")
            run("run", "--rm", "cli", "doctor")
            certificate = hashlib.sha256(read_file("/data/state/tls/cert.pem")).hexdigest()
            for tool in ("flac", "opusenc"):
                run("run", "--rm", "--entrypoint", tool, "cli", "--version")
            run("up", "-d", "collector")
            ready()
            login()
            stop()
            run("rm", "-f", "collector")
            run("up", "-d", "collector")
            ready()
            login()
            stop()
            assert certificate == hashlib.sha256(read_file("/data/state/tls/cert.pem")).hexdigest()
            run("run", "--rm", "cli", "backup", "--destination", "/data/backups/smoke")
            run("run", "--rm", "cli", "validate-backup", "--backup", "/data/backups/smoke")
            # Exercise the documented rollback layout: fresh host roots using the
            # same in-container paths, with the original service kept stopped.
            env["COLLECTOR_DATA"] = str(Path(temporary) / "recovered")
            run("run", "--rm", "--entrypoint", "install", "initialize",
                "-d", "-m", "0700", "-o", "10001", "-g", "10001",
                "/data/config", "/data/archive", "/data/state", "/data/backups")
            run("run", "--rm", "-v", f"{temporary}/backups/smoke:/backup:ro", "cli",
                "restore", "--backup", "/backup", "--archive", "/data/archive", "--state", "/data/state")
            run("run", "--rm", "cli", "doctor")
            run("up", "-d", "collector")
            ready()
            login()
            stop()
            assert certificate == hashlib.sha256(read_file("/data/state/tls/cert.pem")).hexdigest()
            print("Compose initialize/setup/login/recreation/SIGTERM/tools/backup/restore checks passed")
        finally:
            env["COLLECTOR_DATA"] = temporary
            subprocess.run(command + ["logs", "--tail", "60", "collector"], env=env, timeout=20)
            subprocess.run(command + ["down", "-t", "5"], env=env, timeout=20)
            # Return only this generated test tree to the invoking user for cleanup.
            subprocess.run(command + ["run", "--rm", "--entrypoint", "chown",
                                       "initialize", "-R", f"{os.getuid()}:{os.getgid()}", "/data"],
                           env=env, check=True, timeout=30)


if __name__ == "__main__":
    main()
