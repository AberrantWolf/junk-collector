#!/usr/bin/env python3
"""Optional descriptive folder naming after a successful raw capture.

Only the worker contacts MusicBrainz. Its process group has one wall-clock budget,
including identity extraction, DNS, TLS, headers and slow response bodies.
"""
import ctypes
import json
import os
from pathlib import Path
import re
import signal
import subprocess
import sys
import urllib.parse
import urllib.request
import uuid


LOOKUP_SECONDS = 15
MAX_RESPONSE = 1024 * 1024
USER_AGENT = "JunkCollector-CD-Naming/1.0 (https://github.com/AberrantWolf/junk-collector)"


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


def catalog_folder(data, disc_id, track_count):
    """Require one exact release, one matching medium and one usable number."""
    if not isinstance(data, dict) or data.get("id") != disc_id:
        return None
    releases = data.get("releases")
    if not isinstance(releases, list) or len(releases) != 1:
        return None
    if any(type(data.get(key, 1)) is not int or data.get(key, 1) != 1
           for key in ("release-count", "count")):
        return None
    release = releases[0]
    if not isinstance(release, dict):
        return None
    try:
        uuid.UUID(release["id"])
    except (KeyError, ValueError, TypeError, AttributeError):
        return None
    media = release.get("media")
    if not isinstance(media, list) or not media or not all(isinstance(m, dict) for m in media):
        return None
    matching = []
    for medium in media:
        discs = medium.get("discs", [])
        if not isinstance(discs, list) or not all(isinstance(d, dict) for d in discs):
            return None
        if any(d.get("id") == disc_id for d in discs):
            matching.append(medium)
    if len(matching) != 1:
        return None
    medium = matching[0]
    position = medium.get("position")
    if (type(position) is not int or not 1 <= position <= 99
            or type(medium.get("track-count")) is not int
            or medium.get("track-count") != track_count):
        return None
    count = release.get("medium-count", len(media))
    if type(count) is not int or not 1 <= position <= count <= 99:
        return None
    labels = release.get("label-info")
    if not isinstance(labels, list) or not all(isinstance(label, dict) for label in labels):
        return None
    numbers = set()
    for label in labels:
        number = label.get("catalog-number")
        if number is None:
            continue
        if not isinstance(number, str):
            return None
        number = number.strip()
        if number:
            numbers.add(number)
    if len(numbers) != 1:
        return None
    number = numbers.pop()
    # Reject unsafe names instead of silently changing a catalog identifier.
    if (not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9 ._()~+\-]{0,95}", number)
            or number.endswith(".") or number.casefold() in {"none", "unknown", "n/a"}
            or number.split(".")[0].upper() in {
                "CON", "PRN", "AUX", "NUL", *[f"COM{i}" for i in range(1, 10)],
                *[f"LPT{i}" for i in range(1, 10)]}):
        return None
    return f"{number} - Disc {position}" if count > 1 else number


def lookup(output, identity_binary):
    result = subprocess.run(
        [identity_binary, str(output / "disc.toc")], check=True,
        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=LOOKUP_SECONDS,
    )
    identity = json.loads(result.stdout)
    disc_id = identity["disc_id"]
    tracks = identity["track_count"]
    if (not isinstance(disc_id, str) or not re.fullmatch(r"[A-Za-z0-9._]{27}-", disc_id)
            or type(tracks) is not int or not 1 <= tracks <= 99):
        raise ValueError("invalid disc identity")
    request = urllib.request.Request(
        f"https://musicbrainz.org/ws/2/discid/{disc_id}?"
        + urllib.parse.urlencode({"fmt": "json", "inc": "releases+labels+discids", "cdstubs": "no"}),
        headers={"User-Agent": USER_AGENT, "Accept": "application/json"},
    )
    opener = urllib.request.build_opener(NoRedirect)
    # A socket timeout alone is insufficient for slow/trickling bodies; the
    # parent also kills this whole process group after LOOKUP_SECONDS.
    with opener.open(request, timeout=4) as response:
        body = response.read(MAX_RESPONSE + 1)
        if len(body) > MAX_RESPONSE:
            raise ValueError("response exceeds naming limit")
        data = json.loads(body)
    return catalog_folder(data, disc_id, tracks)


def rename_without_replacement(source, destination):
    """Atomic directory rename; never move into/replace an existing directory."""
    libc = ctypes.CDLL(None, use_errno=True)
    if sys.platform == "darwin":
        rename = libc.renamex_np
        rename.argtypes = [ctypes.c_char_p, ctypes.c_char_p, ctypes.c_uint]
        args = (os.fsencode(source), os.fsencode(destination), 0x4)  # RENAME_EXCL
    elif sys.platform.startswith("linux"):
        rename = libc.renameat2
        rename.argtypes = [ctypes.c_int, ctypes.c_char_p, ctypes.c_int,
                           ctypes.c_char_p, ctypes.c_uint]
        args = (-100, os.fsencode(source), -100, os.fsencode(destination), 1)  # NOREPLACE
    else:
        raise OSError("atomic exclusive rename unavailable")
    rename.restype = ctypes.c_int
    if rename(*args) != 0:
        code = ctypes.get_errno()
        raise OSError(code, os.strerror(code))


def bounded_lookup(output, identity_binary):
    worker = subprocess.Popen(
        [sys.executable, "-B", str(Path(__file__).resolve()), "--lookup",
         str(output), identity_binary],
        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, start_new_session=True,
    )
    try:
        stdout, _ = worker.communicate(timeout=LOOKUP_SECONDS)
        if worker.returncode != 0:
            raise ValueError("lookup unavailable")
        return json.loads(stdout)
    except BaseException:
        # Also reap identity extraction if interruption/timeout arrives while
        # that child is running. Never leave a background naming continuation.
        try:
            os.killpg(worker.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        worker.communicate()
        raise


def main():
    if len(sys.argv) == 4 and sys.argv[1] == "--lookup":
        print(json.dumps(lookup(Path(sys.argv[2]), sys.argv[3])))
        return
    if len(sys.argv) != 3:
        raise SystemExit("Usage: name_cd_capture.py CAPTURE_DIRECTORY IDENTITY_EXECUTABLE")
    # Default SIGTERM/HUP would bypass Python cleanup and orphan the worker's
    # separate session. Turn them into exceptions before starting that worker.
    def interrupted(signum, frame):
        raise SystemExit(128 + signum)

    signal.signal(signal.SIGTERM, interrupted)
    signal.signal(signal.SIGHUP, interrupted)
    output = Path(sys.argv[1])
    try:
        folder = bounded_lookup(output, sys.argv[2])
        if folder is None:
            print("No unambiguous catalog number; keeping generated folder.", file=sys.stderr)
        else:
            # Only the worker's validated safe component may become a path.
            destination = output.with_name(folder)
            rename_without_replacement(output, destination)
            output = destination
            print(f"Catalog folder: {folder} (descriptive lookup only).", file=sys.stderr)
    except (OSError, ValueError, AttributeError, subprocess.SubprocessError):
        print("Naming unavailable, timed out, or destination occupied; keeping generated folder.",
              file=sys.stderr)
    print(output)


if __name__ == "__main__":
    main()
