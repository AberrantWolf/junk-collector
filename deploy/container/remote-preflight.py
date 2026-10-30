#!/usr/bin/env python3
"""Read-only Fi admission. Never mount, mkdir, chown, repair or hash media."""
import os
from pathlib import Path
import re
import sys
import uuid
import tomllib


def unescape(value):
    return re.sub(r"\\([0-7]{3})", lambda match: chr(int(match[1], 8)), value)


def mounts(table):
    result = []
    for line in table.splitlines():
        fields = line.split()
        split = fields.index("-")
        if split < 6 or len(fields) < split + 4:
            raise ValueError("malformed mountinfo")
        result.append(dict(target=Path(unescape(fields[4])), source=unescape(fields[split + 2]),
                           kind=fields[split + 1], mount_options=set(fields[5].split(",")), options=set(fields[5].split(",")) | set(fields[split + 3].split(","))))
    return result


def filesystem(path, entries):
    candidates = [entry for entry in entries if path == entry["target"] or entry["target"] in path.parents]
    if not candidates:
        raise ValueError(f"no filesystem for {path}")
    return max(candidates, key=lambda entry: len(entry["target"].parts))


def check(env, table):
    entries = mounts(table)
    paths = {}
    for name in ("CONFIG", "STATE", "BACKUPS", "ARCHIVE", "INBOX"):
        raw = env[f"COLLECTOR_{name}"]
        if not Path(raw).is_absolute():
            raise ValueError(f"{name} must be absolute")
        paths[name] = Path(raw).resolve(strict=True)
        if not paths[name].is_dir():
            raise ValueError(f"{name} must be an existing directory")
    for name in ("ARCHIVE", "INBOX"):
        path = paths[name]
        entry = filesystem(path, entries)
        if entry["target"] != path or entry["kind"] not in ("nfs", "nfs4") or entry["source"] != env[f"COLLECTOR_{name}_SOURCE"]:
            raise ValueError(f"{name}: intended NFS source is not mounted at the exact root")
        if not entry["options"].intersection({"vers=4.1", "vers=4.2"}):
            raise ValueError(f"{name}: NFSv4.1 or NFSv4.2 required")
        if "hard" not in entry["options"] or "soft" in entry["options"] or "softerr" in entry["options"]:
            raise ValueError(f"{name}: hard NFS mount required")
        if name == "ARCHIVE" and "rw" not in entry["mount_options"]:
            raise ValueError("archive writer needs a read/write mount")
        if any(other["target"] != path and path in other["target"].parents for other in entries):
            raise ValueError(f"{name}: nested mounts are unsupported")
    data_root = Path(env["COLLECTOR_DATA"])
    if not data_root.is_absolute():
        raise ValueError("COLLECTOR_DATA must be absolute")
    for private_path in (data_root, data_root / "media-certificates", data_root / "media-proxy-config"):
        private_path = private_path.resolve(strict=True)
        if not private_path.is_dir() or filesystem(private_path, entries)["kind"] not in ("ext4", "btrfs", "xfs"):
            raise ValueError("Caddy data/config must be existing directories on local SSD storage")
    for name in ("CONFIG", "STATE", "BACKUPS"):
        # Supported local SSD filesystems for this deployment profile.
        if filesystem(paths[name], entries)["kind"] not in ("ext4", "btrfs", "xfs"):
            raise ValueError(f"{name}: requires a qualified local SSD filesystem")
    for name, path in paths.items():
        for other, other_path in paths.items():
            if name != other and (path == other_path or path in other_path.parents):
                raise ValueError(f"overlapping roots: {name}/{other}")
    config = tomllib.loads((paths["CONFIG"] / "collector.toml").read_text())
    if config.get("archive") != "/data/archive" or config.get("state") != "/data/state" or config.get("archive_mount", {}).get("nfs_source") != env["COLLECTOR_ARCHIVE_SOURCE"]:
        raise ValueError("container storage paths and archive_mount.nfs_source must match the deployment")
    if config.get("remote_media_allowed", False) or config.get("remote_media", {}).get("socket") != "/run/collector-authority/socket":
        raise ValueError("isolated media requires its dedicated authority socket and disabled control-origin fallback")
    expected = str(uuid.UUID(env["COLLECTOR_ARCHIVE_ID"])) + "\n"
    for path in (paths["ARCHIVE"] / ".archive-identity", paths["STATE"] / "archive-binding"):
        if path.is_symlink() or path.stat().st_size != 37 or path.read_text() != expected:
            raise ValueError(f"archive identity mismatch: {path}")


def main():
    try:
        check(os.environ, Path("/proc/self/mountinfo").read_text())
    except (KeyError, OSError, ValueError) as error:
        print(f"Collector remote-storage preflight refused: {error}", file=sys.stderr)
        return 1
    print("Intended NFS archive/inbox and local state/config admitted; NFS durability still requires qualification.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
