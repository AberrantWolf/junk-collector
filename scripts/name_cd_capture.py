#!/usr/bin/env python3
"""Optional descriptive folder naming after a successful raw capture.

Only the worker contacts MusicBrainz. Its process group has one wall-clock budget,
including identity extraction, DNS, TLS, headers and slow response bodies.
"""
import ctypes
import errno
import json
import os
from pathlib import Path
import re
import signal
import socket
import subprocess
import sys
import urllib.parse
import urllib.error
import urllib.request
import uuid


LOOKUP_SECONDS = 15
MAX_RESPONSE = 1024 * 1024
USER_AGENT = "JunkCollector-CD-Naming/1.0 (https://github.com/AberrantWolf/junk-collector)"


class NamingFailure(Exception):
    """An actionable reason to retain the generated capture name."""


def diagnostic(value):
    # Keep tool/network messages readable without terminal control characters.
    return " ".join("".join(c if c.isprintable() else " " for c in str(value)).split())[:300]


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


def catalog_folder(data, disc_id, track_count):
    """Require one exact release, one matching medium and one usable number."""
    if not isinstance(data, dict) or data.get("id") != disc_id:
        raise NamingFailure("MusicBrainz response does not match the requested Disc ID")
    releases = data.get("releases")
    if not isinstance(releases, list):
        raise NamingFailure("MusicBrainz response has no valid release list")
    if not releases:
        raise NamingFailure("MusicBrainz has no release registered for this Disc ID")
    if len(releases) != 1:
        raise NamingFailure(f"MusicBrainz returned {len(releases)} releases; edition is ambiguous")
    if any(type(data.get(key, 1)) is not int or data.get(key, 1) != 1
           for key in ("release-count", "count")):
        raise NamingFailure("MusicBrainz release count is inconsistent or ambiguous")
    release = releases[0]
    if not isinstance(release, dict):
        raise NamingFailure("MusicBrainz returned an invalid release")
    try:
        uuid.UUID(release["id"])
    except (KeyError, ValueError, TypeError, AttributeError):
        raise NamingFailure("MusicBrainz release has no valid release identifier")
    media = release.get("media")
    if not isinstance(media, list) or not media or not all(isinstance(m, dict) for m in media):
        raise NamingFailure("MusicBrainz release has missing or invalid disc information")
    matching = []
    for medium in media:
        discs = medium.get("discs", [])
        if not isinstance(discs, list) or not all(isinstance(d, dict) for d in discs):
            raise NamingFailure("MusicBrainz medium has invalid Disc ID information")
        if any(d.get("id") == disc_id for d in discs):
            matching.append(medium)
    if len(matching) != 1:
        raise NamingFailure(f"MusicBrainz release has {len(matching)} matching media; expected exactly one")
    medium = matching[0]
    position = medium.get("position")
    if type(position) is not int or not 1 <= position <= 99:
        raise NamingFailure("MusicBrainz medium has an invalid disc position")
    if type(medium.get("track-count")) is not int:
        raise NamingFailure("MusicBrainz medium has no valid track count")
    if medium["track-count"] != track_count:
        raise NamingFailure(f"MusicBrainz medium lists {medium['track-count']} tracks; captured disc has {track_count}")
    count = release.get("medium-count", len(media))
    if type(count) is not int or not 1 <= position <= count <= 99:
        raise NamingFailure("MusicBrainz release has inconsistent disc positions/counts")
    labels = release.get("label-info")
    if not isinstance(labels, list) or not all(isinstance(label, dict) for label in labels):
        raise NamingFailure("MusicBrainz release has no valid label/catalog information")
    numbers = set()
    for label in labels:
        number = label.get("catalog-number")
        if number is None:
            continue
        if not isinstance(number, str):
            raise NamingFailure("MusicBrainz returned an invalid catalog number")
        number = number.strip()
        if number:
            numbers.add(number)
    if not numbers:
        raise NamingFailure("MusicBrainz release has no catalog number")
    if len(numbers) != 1:
        raise NamingFailure(f"MusicBrainz release has {len(numbers)} different catalog numbers; naming is ambiguous")
    number = numbers.pop()
    # Reject unsafe names instead of silently changing a catalog identifier.
    if (not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9 ._()~+\-]{0,95}", number)
            or number.endswith(".") or number.casefold() in {"none", "unknown", "n/a"}
            or number.split(".")[0].upper() in {
                "CON", "PRN", "AUX", "NUL", *[f"COM{i}" for i in range(1, 10)],
                *[f"LPT{i}" for i in range(1, 10)]}):
        raise NamingFailure("MusicBrainz catalog number is a placeholder or unsafe as a folder name")
    return f"{number} - Disc {position}" if count > 1 else number


def lookup(output, identity_binary):
    try:
        result = subprocess.run(
            [identity_binary, str(output / "disc.toc")], check=True,
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=LOOKUP_SECONDS,
        )
    except subprocess.CalledProcessError as error:
        detail = diagnostic((error.stderr or b"").decode("utf-8", errors="replace"))
        raise NamingFailure(f"Disc identity failed (exit {error.returncode}): {detail or 'no diagnostic output'}") from error
    except subprocess.TimeoutExpired as error:
        raise NamingFailure(f"Disc identity extraction timed out after {LOOKUP_SECONDS} seconds") from error
    except OSError as error:
        raise NamingFailure(f"Cannot run identity utility: {diagnostic(error)}") from error
    try:
        identity = json.loads(result.stdout)
        disc_id = identity["disc_id"]
        tracks = identity["track_count"]
    except (ValueError, KeyError, TypeError) as error:
        raise NamingFailure("Identity utility returned invalid JSON or missing identity fields") from error
    if (not isinstance(disc_id, str) or not re.fullmatch(r"[A-Za-z0-9._]{27}-", disc_id)
            or type(tracks) is not int or not 1 <= tracks <= 99):
        raise NamingFailure("Identity utility returned an invalid Disc ID or track count")
    request = urllib.request.Request(
        f"https://musicbrainz.org/ws/2/discid/{disc_id}?"
        # /discid already includes releases, media and disc IDs. Only request
        # labels explicitly; release-resource subquery rules do not apply here.
        + urllib.parse.urlencode({"fmt": "json", "inc": "labels", "cdstubs": "no"}),
        headers={"User-Agent": USER_AGENT, "Accept": "application/json"},
    )
    opener = urllib.request.build_opener(NoRedirect)
    # A socket timeout alone is insufficient for slow/trickling bodies; the
    # parent also kills this whole process group after LOOKUP_SECONDS.
    try:
        with opener.open(request, timeout=4) as response:
            body = response.read(MAX_RESPONSE + 1)
            if len(body) > MAX_RESPONSE:
                raise NamingFailure("MusicBrainz response exceeded the 1 MiB naming limit")
            try:
                data = json.loads(body)
            except ValueError as error:
                raise NamingFailure("MusicBrainz returned malformed JSON") from error
    except urllib.error.HTTPError as error:
        reasons = {404: "no release registered for this Disc ID", 429: "rate limit reached",
                   503: "service unavailable"}
        reason = reasons.get(error.code, "request rejected")
        if 300 <= error.code < 400:
            reason = "redirect refused"
        # MusicBrainz explains invalid API requests in a small JSON error field.
        # Preserve that useful cause, without printing HTML or an unbounded body.
        try:
            error_body = error.read(4097)
            if len(error_body) <= 4096:
                error_data = json.loads(error_body)
                if isinstance(error_data, dict) and isinstance(error_data.get("error"), str):
                    reason += f" ({diagnostic(error_data['error'])})"
        except (OSError, ValueError, AttributeError):
            pass
        raise NamingFailure(f"MusicBrainz HTTP {error.code}: {reason}") from error
    except urllib.error.URLError as error:
        raise NamingFailure(f"MusicBrainz connection failed: {diagnostic(error.reason)}") from error
    except (socket.timeout, TimeoutError) as error:
        raise NamingFailure("MusicBrainz request timed out (4-second socket timeout)") from error
    except OSError as error:
        raise NamingFailure(f"MusicBrainz request failed: {diagnostic(error)}") from error
    return catalog_folder(data, disc_id, tracks)


def lookup_reply(output, identity_binary):
    """Carry failure details across the supervised process boundary."""
    try:
        return {"folder": lookup(output, identity_binary)}
    except Exception as error:
        detail = str(error) if isinstance(error, NamingFailure) else f"{type(error).__name__}: {error}"
        return {"error": diagnostic(detail)}


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
    try:
        worker = subprocess.Popen(
            [sys.executable, "-B", str(Path(__file__).resolve()), "--lookup",
             str(output), identity_binary],
            stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, start_new_session=True,
        )
    except OSError as error:
        raise NamingFailure(f"Cannot start naming worker: {diagnostic(error)}") from error
    try:
        stdout, _ = worker.communicate(timeout=LOOKUP_SECONDS)
    except BaseException:
        # Also reap identity extraction if interruption/timeout arrives while
        # that child is running. Never leave a background naming continuation.
        try:
            os.killpg(worker.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        worker.communicate()
        raise
    if worker.returncode != 0:
        raise NamingFailure(f"Naming worker exited with status {worker.returncode}")
    try:
        reply = json.loads(stdout)
        if "error" in reply:
            raise NamingFailure(diagnostic(reply["error"]))
        folder = reply["folder"]
        if not isinstance(folder, str) or not folder:
            raise ValueError("missing folder name")
        return folder
    except (ValueError, KeyError, TypeError) as error:
        raise NamingFailure("Naming worker returned an invalid result") from error


def main():
    if len(sys.argv) == 4 and sys.argv[1] == "--lookup":
        print(json.dumps(lookup_reply(Path(sys.argv[2]), sys.argv[3])))
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
        # Only the worker's validated safe component may become a path.
        destination = output.with_name(folder)
        rename_without_replacement(output, destination)
        output = destination
        print(f"Catalog folder: {folder} (descriptive lookup only).", file=sys.stderr)
    except subprocess.TimeoutExpired:
        print(f"Automatic naming exceeded the {LOOKUP_SECONDS}-second deadline; keeping generated folder.", file=sys.stderr)
    except NamingFailure as error:
        print(f"{error}; keeping generated folder.", file=sys.stderr)
    except OSError as error:
        if error.errno in (errno.EEXIST, errno.ENOTEMPTY):
            reason = f"Catalog destination already exists: {destination}"
        else:
            reason = f"Cannot rename capture: {diagnostic(error)}"
        print(f"{reason}; keeping generated folder.", file=sys.stderr)
    except AttributeError:
        print("Atomic exclusive rename is unavailable on this platform; keeping generated folder.", file=sys.stderr)
    print(output)


if __name__ == "__main__":
    main()
