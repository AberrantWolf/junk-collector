#!/usr/bin/env python3
"""Bind a completed GNU tar format-2 working snapshot file to the filesystem snapshot time.

Run on a private .snar.part after tar succeeds, before publishing the backup pair.
The cutoff must precede filesystem snapshot creation, with Collector stopped.
"""
import argparse
import os
import stat
from pathlib import Path
import tempfile


def freeze(path, captured_ns):
    if not path.name.endswith(".part"):
        raise ValueError("Use a private .snar.part working copy")
    try:
        descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    except OSError as error:
        raise ValueError("Use a regular working snapshot file without aliases") from error
    with os.fdopen(descriptor, "rb") as source:
        original = os.fstat(source.fileno())
        if not stat.S_ISREG(original.st_mode) or original.st_nlink != 1:
            raise ValueError("Use a regular working snapshot file without aliases")
        limit = 64*1024*1024
        if not 0 < original.st_size <= limit:
            raise ValueError("Use a bounded private .snar.part working copy")
        raw = source.read(limit + 1)
        if len(raw) > limit:
            raise ValueError("Working snapshot grew beyond its limit")
    header, separator, body = raw.partition(b"\n")
    if not separator or not header.startswith(b"GNU tar-") or not header.endswith(b"-2"):
        raise ValueError("Unsupported GNU tar snapshot format; keep the previous backup")
    fields = body.split(b"\0", 2)
    if len(fields) != 3 or not all(field.isdigit() for field in fields[:2]):
        raise ValueError("Malformed GNU tar snapshot cutoff")
    previous = int(fields[0])*1_000_000_000 + int(fields[1])
    if not 0 <= int(fields[1]) < 1_000_000_000 or not 0 < captured_ns <= previous:
        raise ValueError("Snapshot time must not follow tar start; investigate clock changes")
    seconds, nanos = divmod(captured_ns, 1_000_000_000)
    corrected = header + b"\n" + str(seconds).encode() + b"\0" + str(nanos).encode() + b"\0" + fields[2]
    descriptor, temporary = tempfile.mkstemp(prefix=".tar-cutoff-", dir=path.parent)
    try:
        with os.fdopen(descriptor, "wb") as output:
            output.write(corrected)
            output.flush()
            os.fsync(output.fileno())
        current = path.lstat()
        if (current.st_dev, current.st_ino, current.st_size, current.st_mtime_ns) != (original.st_dev, original.st_ino, original.st_size, original.st_mtime_ns):
            raise ValueError("Working snapshot changed during cutoff correction")
        os.replace(temporary, path)
    finally:
        Path(temporary).unlink(missing_ok=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("working_snapshot", type=Path)
    parser.add_argument("captured_ns", type=int)
    args = parser.parse_args()
    freeze(args.working_snapshot, args.captured_ns)
