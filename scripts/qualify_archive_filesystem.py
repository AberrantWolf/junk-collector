#!/usr/bin/env python3
"""Small isolated NFS mechanism probe; never an archive or power-loss verifier."""
import argparse
import fcntl
import importlib.util
import os
from pathlib import Path
import subprocess
import sys
import tempfile

HELPER = Path(__file__).resolve().parents[1] / "deploy/container/remote-preflight.py"
spec = importlib.util.spec_from_file_location("collector_remote_preflight", HELPER)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
SENTINEL = ".collector-nfs-qualification-only"


def sync_dir(path):
    fd = os.open(path, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(fd)
    finally:
        os.close(fd)


def qualify(root, source):
    if sys.platform != "linux":
        raise ValueError("Linux client required")
    if root.is_symlink():
        raise ValueError("qualification root cannot be an alias")
    root = root.resolve(strict=True)
    if {entry.name for entry in root.iterdir()} != {SENTINEL}:
        raise ValueError("refusing nonempty or unmarked qualification root")
    marker = root / SENTINEL
    if marker.is_symlink() or marker.stat().st_size != len(b"DISPOSABLE COLLECTOR NFS QUALIFICATION ONLY\n") or marker.read_bytes() != b"DISPOSABLE COLLECTOR NFS QUALIFICATION ONLY\n":
        raise ValueError("invalid disposable qualification marker")
    entry = module.filesystem(root, module.mounts(Path("/proc/self/mountinfo").read_text()))
    if entry["kind"] not in ("nfs", "nfs4") or entry["source"] != source or "hard" not in entry["options"] or "rw" not in entry["mount_options"] or not entry["options"].intersection({"vers=4.1", "vers=4.2"}):
        raise ValueError("intended hard read/write NFSv4.1/4.2 source is required")
    with tempfile.TemporaryDirectory(prefix="owned-probe-", dir=root) as temporary:
        probe = Path(temporary)
        staged = probe / "staging"
        staged.mkdir()
        payload = bytes(range(256)) * 4096
        with (staged / "payload").open("xb") as output:
            output.write(payload)
            output.flush()
            os.fsync(output.fileno())
        try:
            with (staged / "payload").open("xb"):
                pass
        except FileExistsError:
            pass
        else:
            raise ValueError("exclusive create replaced an existing file")
        # Identity publication may use a no-replace hard-link fallback on NFS.
        linked = staged / "identity-link"
        os.link(staged / "payload", linked)
        try:
            os.link(staged / "payload", linked)
        except FileExistsError:
            pass
        else:
            raise ValueError("no-replace link overwrote an existing identity")
        if linked.read_bytes() != payload:
            raise ValueError("hard-link identity publication readback failed")
        linked.unlink()
        sync_dir(staged)
        sync_dir(probe)
        published = probe / "published"
        os.rename(staged, published)
        sync_dir(probe)
        if staged.exists() or (published / "payload").read_bytes() != payload:
            raise ValueError("directory rename/readback failed")
        with (probe / "writer.lock").open("xb") as lock:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
            child = subprocess.run([sys.executable, "-c", "import fcntl,sys; f=open(sys.argv[1],'rb');\ntry: fcntl.flock(f,fcntl.LOCK_EX|fcntl.LOCK_NB)\nexcept BlockingIOError: sys.exit(42)", str(probe / "writer.lock")], timeout=10, check=False)
            if child.returncode != 42:
                raise ValueError("same-host competing writer was not refused")
        sync_dir(probe)
    sync_dir(root)
    print("PASS: 1 MiB exclusive-create, file/directory sync, same-filesystem directory rename/readback and same-host lock contention")
    print("UNQUALIFIED: server power-loss durability, disconnect/restart recovery, cross-host locking and application/container lifecycle")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", required=True, type=Path)
    parser.add_argument("--source", required=True)
    args = parser.parse_args()
    try:
        qualify(args.root, args.source)
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        parser.exit(1, f"Qualification refused/failed: {error}\n")


if __name__ == "__main__":
    main()
