"""Disposable Btrfs/GNU-tar qualification support; never an operator backup API."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time


class SnapshotQualification:
    def __init__(self, parent, image, handoff):
        self.parent = parent
        self.handoff = handoff
        self.subvolumes = []
        self.capture_times = {}
        if not parent.name.startswith("collector-compose-") or parent.is_symlink():
            raise ValueError("Snapshot qualification requires its generated temporary root")
        self.command = ["docker", "run", "--rm", "--network", "none", "--read-only",
                        "--cap-add", "SYS_ADMIN", "--user", "0:0", "-e", "TAR_OPTIONS=",
                        "-v", str(parent) + ":/fixture", "--entrypoint"]
        self.image = image

    def run(self, tool, *arguments):
        return subprocess.check_output(self.command + [tool, self.image, *arguments],
                                       stderr=subprocess.PIPE, timeout=180)

    def create(self):
        self.run("btrfs", "subvolume", "create", "/fixture/live")
        self.subvolumes.append("live")
        self.run("chown", f"{os.getuid()}:{os.getgid()}", "/fixture/live")
        return self.parent / "live"

    def seed_unknown(self):
        self.run("mkdir", "/fixture/live/archive/.qualification-unknown")
        self.run("dd", "if=/dev/urandom",
                 "of=/fixture/live/archive/.qualification-unknown/opaque-master.bin",
                 "bs=1048576", "count=32", "status=none")
        self.run("sh", "-c", "printf before > /fixture/live/state/.qualification-cutoff")
        self.unknown_digest = self.run("sha256sum", "/fixture/live/archive/.qualification-unknown/opaque-master.bin").split()[0]

    def capture(self, name):
        started = time.monotonic()
        self.capture_times[name] = time.time_ns()
        self.run("btrfs", "subvolume", "snapshot", "-r", "/fixture/live", "/fixture/" + name)
        self.subvolumes.append(name)
        print(f"Atomic stopped-server snapshot {name}: {time.monotonic()-started:.2f}s (no payload hashing/copy)", flush=True)

    def export(self, name, previous=None):
        if previous:
            assert self.capture_times[name] >= self.capture_times[previous], "Clock moved backwards; require a fresh full baseline"
            self.run("cp", "/fixture/" + previous + ".snar", "/fixture/" + name + ".snar.part")
        self.run("tar", "--create", "--file=/fixture/" + name + ".tar.part",
                 "--listed-incremental=/fixture/" + name + ".snar.part",
                 "--no-check-device", "--numeric-owner", "--acls", "--xattrs",
                 "--directory=/fixture/" + name, "config", "archive", "state")
        # tar starts after the frozen filesystem point: its own clock would miss
        # live writes made between snapshot capture and later archive export.
        self.run("chown", f"{os.getuid()}:{os.getgid()}", "/fixture/" + name + ".snar.part")
        import importlib.util
        helper = Path(__file__).resolve().parents[1] / "deploy/container/freeze-tar-time.py"
        spec = importlib.util.spec_from_file_location("freeze_tar_time", helper)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        module.freeze(self.parent / (name + ".snar.part"), self.capture_times[name])
        self.run("mv", "/fixture/" + name + ".tar.part", "/fixture/" + name + ".tar")
        self.run("mv", "/fixture/" + name + ".snar.part", "/fixture/" + name + ".snar")
        for suffix in (".tar", ".snar"):
            self.run("chown", f"{os.getuid()}:{os.getgid()}", "/fixture/" + name + suffix)
            self.run("chmod", "0600", "/fixture/" + name + suffix)
        members = self.run("tar", "--list", "--file=/fixture/" + name + ".tar").decode().splitlines()
        unknown = "archive/.qualification-unknown/opaque-master.bin"
        assert (unknown in members) == (previous is None), "Unchanged opaque payload was recopied or omitted from the base"
        if previous:
            assert (self.parent / (name + ".tar")).stat().st_size < (self.parent / (previous + ".tar")).stat().st_size / 2, "Incremental backup unexpectedly recopied the collection fixture"
        print(f"{name} incremental archive: {(self.parent / (name+'.tar')).stat().st_size} bytes", flush=True)

    def rollback(self):
        self.run("btrfs", "subvolume", "snapshot", "/fixture/before-update", "/fixture/rollback")
        self.subvolumes.append("rollback")
        return self.parent / "rollback"

    def independent_roundtrip(self, image_id, server_id):
        files = {}
        for name in ("before-update.tar", "before-update.snar", "after-update.tar", "after-update.snar"):
            path = self.parent / name
            digest = hashlib.sha256()
            with path.open("rb") as source:
                for chunk in iter(lambda: source.read(1024*1024), b""):
                    digest.update(chunk)
            files[name] = dict(path=str(path), bytes=path.stat().st_size, sha256=digest.hexdigest())
        (self.parent / "independent").mkdir(mode=0o700)
        descriptor = os.open(self.handoff, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        with os.fdopen(descriptor, "w") as target:
            json.dump(dict(image_id=image_id, server_id=server_id, files=files,
                           return_directory=str(self.parent / "independent")), target)
        print(f"Independent-backup fixture ready: {self.handoff}; copy to separate storage, return verified files, then write FILE.done", flush=True)
        marker = Path(str(self.handoff) + ".done")
        deadline = time.monotonic() + 600
        while not marker.exists():
            if time.monotonic() > deadline:
                raise AssertionError("Independent-storage roundtrip timed out")
            time.sleep(1)
        assert marker.read_text().strip() == "passed"
        for name, expected in files.items():
            path = self.parent / "independent" / name
            assert not path.is_symlink() and path.stat().st_size == expected["bytes"]
            digest = hashlib.sha256()
            with path.open("rb") as source:
                for chunk in iter(lambda: source.read(1024*1024), b""):
                    digest.update(chunk)
            assert digest.hexdigest() == expected["sha256"], "Independent backup bytes changed"
        # The restore must use returned files; remove the source exports first.
        for name in files:
            (self.parent / name).unlink()

    def restore(self):
        self.run("mkdir", "-m", "0700", "/fixture/recovered")
        for name in ("before-update", "after-update"):
            self.run("tar", "--extract", "--listed-incremental=/dev/null",
                     "--file=/fixture/independent/" + name + ".tar", "--numeric-owner",
                     "--same-owner", "--acls", "--xattrs", "--directory=/fixture/recovered")
        self.run("install", "-d", "-m", "0700", "-o", "10001", "-g", "10001",
                 "/fixture/recovered", "/fixture/recovered/backups")
        # This is restore acceptance, where full byte validation is appropriate.
        # Compare the complete file set too, including private spool deletions.
        manifest = 'cd "$1" && find config archive state -type f -exec sha256sum {} +'
        expected = self.run("sh", "-c", manifest, "sh", "/fixture/after-update")
        actual = self.run("sh", "-c", manifest, "sh", "/fixture/recovered")
        assert sorted(expected.splitlines()) == sorted(actual.splitlines()), "Restored roots differ from the complete cold snapshot"
        metadata = 'cd "$1" && find config archive state -printf "%p|%y|%m|%U|%G|%l\\n"'
        expected = self.run("sh", "-c", metadata, "sh", "/fixture/after-update")
        actual = self.run("sh", "-c", metadata, "sh", "/fixture/recovered")
        assert sorted(expected.splitlines()) == sorted(actual.splitlines()), "Restored permissions/ownership or path types changed"
        assert self.run("cat", "/fixture/recovered/state/.qualification-cutoff") == b"after!", "A write between snapshot and export was lost"
        restored = self.run("sha256sum", "/fixture/recovered/archive/.qualification-unknown/opaque-master.bin").split()[0]
        assert restored == self.unknown_digest, "Unknown data changed during incremental restore"
        return self.parent / "recovered"

    def cleanup(self):
        for name in reversed(self.subvolumes):
            self.run("btrfs", "subvolume", "delete", "/fixture/" + name)
        # Generated temporary data only; make restore/export cleanup possible for the caller.
        self.run("chown", "-R", f"{os.getuid()}:{os.getgid()}", "/fixture")
        self.handoff.unlink(missing_ok=True)
        Path(str(self.handoff) + ".done").unlink(missing_ok=True)
