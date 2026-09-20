#!/usr/bin/env python3
"""Cache the bounded game DAT set used by local Collector qualification.

This is intentionally not a general-purpose crawler.  It knows only the catalog
systems whose byte domains are currently implemented by Collector, performs one
request at a time, and preserves the provider ZIP and extracted DAT unchanged.
"""

from __future__ import annotations

import argparse
import fcntl
import hashlib
import http.cookiejar
import json
import random
import re
import shutil
import sys
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request
import xml.etree.ElementTree as ET
import zipfile
from dataclasses import dataclass
from datetime import datetime, timezone
from html.parser import HTMLParser
from pathlib import Path
from typing import BinaryIO


USER_AGENT = "junk-collector-local-alpha-dat-cache/1.0 (manual, rate-limited)"
DATOMATIC_URL = "https://datomatic.no-intro.org/index.php?page=download&op=dat&s={system_id}"
MAX_HTML_BYTES = 4 * 1024 * 1024
MAX_ARCHIVE_BYTES = 64 * 1024 * 1024
MAX_DAT_BYTES = 128 * 1024 * 1024
# Paused after provider error 1260; restoring access alone does not qualify the client.
DATOMATIC_DOWNLOADS_PAUSED = True
QUEUED_MARKERS = (
    "temporarily not available",
    "add this job to the queue",
)
REFUSAL_MARKERS = (
    "the ban won't be lifted",
    "unknown filter",
    "error id:",
)
REDUMP_PRERELEASE_TAG = re.compile(
    r"\((?:alpha|beta|demo|kiosk|preview|proto(?:type)?|sample|taikenban)(?:\)|[^)]*\))",
    re.IGNORECASE,
)


@dataclass(frozen=True)
class Catalog:
    key: str
    provider: str
    provider_system: str
    display_name: str
    byte_domain: str
    byte_domain_version: int = 1
    redump_slug: str | None = None
    datomatic_system_id: int | None = None
    datomatic_format: int | None = None


# Do not extend this list merely because a provider exposes another system. Each
# entry must correspond to a byte domain accepted by collector-modules::games.
CATALOGS = {
    catalog.key: catalog
    for catalog in (
        Catalog(
            key="redump-psx",
            provider="redump",
            provider_system="PSX",
            display_name="Sony - PlayStation",
            byte_domain="org.junk.game.cd.raw-2352-track",
            redump_slug="PSX",
        ),
        Catalog(
            key="no-intro-nes",
            provider="no-intro",
            provider_system="45",
            display_name="Nintendo - Nintendo Entertainment System",
            byte_domain="org.junk.game.nes.ines-payload",
            datomatic_system_id=45,
            datomatic_format=1,  # Headerless payload, not the default iNES file.
        ),
        Catalog(
            key="no-intro-n64",
            provider="no-intro",
            provider_system="24",
            display_name="Nintendo - Nintendo 64",
            byte_domain="org.junk.game.n64.z64-big-endian",
            datomatic_system_id=24,
        ),
        Catalog(
            key="no-intro-snes",
            provider="no-intro",
            provider_system="49",
            display_name="Nintendo - Super Nintendo Entertainment System",
            byte_domain="org.junk.game.snes.headerless-payload",
            datomatic_system_id=49,
            datomatic_format=1,
        ),
        Catalog(
            key="no-intro-pce",
            provider="no-intro",
            provider_system="12",
            display_name="NEC - PC Engine - TurboGrafx-16",
            byte_domain="org.junk.game.pce.hucard-payload",
            datomatic_system_id=12,
            datomatic_format=1,
        ),
    )
}


class MainFormParser(HTMLParser):
    def __init__(self) -> None:
        super().__init__()
        self.active = False
        self.fields: list[tuple[str, str]] = []
        self.names: set[str] = set()
        self.values: dict[str, set[str]] = {}
        self.prepare: tuple[str, str] | None = None
        self.download: tuple[str, str] | None = None

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        values = dict(attrs)
        if tag.lower() == "form":
            self.active = values.get("name") == "main_form"
            return
        if tag.lower() != "input":
            return
        name = values.get("name")
        if name is None:
            return
        value = values.get("value") or ""
        input_type = (values.get("type") or "text").lower()
        if self.active:
            self.names.add(name)
            self.values.setdefault(name, set()).add(value)
        if self.active and (input_type == "hidden" or "checked" in values):
            self.fields.append((name, value))
        if self.active and input_type == "submit" and value == "Prepare":
            self.prepare = (name, value)
        if (
            input_type == "submit"
            and value == "Download!!"
            and "display:none" not in (values.get("style") or "").replace(" ", "")
        ):
            self.download = (name, value)

    def handle_endtag(self, tag: str) -> None:
        if tag.lower() == "form" and self.active:
            self.active = False


class RateLimitedClient:
    def __init__(
        self,
        *,
        minimum_delay: float,
        maximum_delay: float,
        timeout: float,
        retries: int,
        rate_state: Path,
    ) -> None:
        if minimum_delay < 5:
            raise ValueError("minimum delay must be at least five seconds")
        if maximum_delay < minimum_delay:
            raise ValueError("maximum delay must not be less than minimum delay")
        self.minimum_delay = minimum_delay
        self.maximum_delay = maximum_delay
        self.timeout = timeout
        self.retries = retries
        self.rate_state = rate_state
        self.last_request_at: float | None = None
        self.cookies = http.cookiejar.CookieJar()
        self.opener = urllib.request.build_opener(
            urllib.request.HTTPCookieProcessor(self.cookies)
        )

    def _wait(self) -> None:
        target = random.uniform(self.minimum_delay, self.maximum_delay)
        waits = []
        if self.last_request_at is not None:
            waits.append(target - (time.monotonic() - self.last_request_at))
        try:
            previous = float(self.rate_state.read_text(encoding="ascii").strip())
            waits.append(target - (time.time() - previous))
        except (FileNotFoundError, OSError, ValueError):
            pass
        delay = max(waits, default=0.0)
        if delay > 0:
            time.sleep(delay)

    def open(self, request: urllib.request.Request) -> BinaryIO:
        for attempt in range(self.retries + 1):
            self._wait()
            # Record the request start before network I/O so an interrupted run
            # still spaces the next invocation conservatively.
            self.rate_state.write_text(f"{time.time()}\n", encoding="ascii")
            self.last_request_at = time.monotonic()
            request.add_header("User-Agent", USER_AGENT)
            try:
                response = self.opener.open(request, timeout=self.timeout)
                return response
            except urllib.error.HTTPError as error:
                # A provider can return its refusal page with an error status too.
                read_html(error)
                if error.code not in (429, 503) or attempt == self.retries:
                    raise
                retry_after = error.headers.get("Retry-After")
                try:
                    delay = float(retry_after) if retry_after else 60.0 * (attempt + 1)
                except ValueError:
                    delay = 60.0 * (attempt + 1)
                time.sleep(max(30.0, min(delay, 15 * 60.0)))
        raise AssertionError("retry loop exited unexpectedly")


class ProviderRefusal(RuntimeError):
    pass


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def read_html(response: BinaryIO) -> str:
    data = response.read(MAX_HTML_BYTES + 1)
    if len(data) > MAX_HTML_BYTES:
        raise RuntimeError("provider HTML exceeded the four-MiB safety bound")
    html = data.decode("utf-8", errors="replace")
    lowered = html.lower()
    if any(marker in lowered for marker in REFUSAL_MARKERS):
        raise ProviderRefusal("provider refused the request; stop and inspect the form contract")
    return html


def save_response(response: BinaryIO, destination: Path) -> None:
    if "text/html" in response.headers.get("Content-Type", "").lower():
        read_html(response)
        raise ProviderRefusal("provider returned HTML instead of a catalog archive; stop the batch")
    length = response.headers.get("Content-Length")
    if length is not None and int(length) > MAX_ARCHIVE_BYTES:
        raise RuntimeError("provider archive exceeded the 64-MiB safety bound")
    temporary = destination.with_suffix(destination.suffix + ".part")
    with temporary.open("wb") as output:
        written = 0
        while chunk := response.read(1024 * 1024):
            written += len(chunk)
            if written > MAX_ARCHIVE_BYTES:
                temporary.unlink(missing_ok=True)
                raise RuntimeError("provider archive exceeded the 64-MiB safety bound")
            output.write(chunk)
    temporary.replace(destination)


def extract_single_dat(archive: Path, destination: Path) -> str:
    with zipfile.ZipFile(archive) as source:
        members = [item for item in source.infolist() if not item.is_dir()]
        if len(members) != 1 or not members[0].filename.lower().endswith(".dat"):
            raise RuntimeError(
                f"expected one DAT in {archive}, found {[item.filename for item in members]}"
            )
        if members[0].file_size > MAX_DAT_BYTES:
            raise RuntimeError("expanded DAT exceeds the 128-MiB safety bound")
        temporary = destination.with_suffix(destination.suffix + ".part")
        try:
            with source.open(members[0]) as input_file, temporary.open("wb") as output:
                written = 0
                while chunk := input_file.read(1024 * 1024):
                    written += len(chunk)
                    if written > MAX_DAT_BYTES:
                        raise RuntimeError("expanded DAT exceeds the 128-MiB safety bound")
                    output.write(chunk)
            temporary.replace(destination)
        finally:
            temporary.unlink(missing_ok=True)
        return members[0].filename


def filter_redump_prerelease(source: Path, destination: Path) -> tuple[int, int]:
    tree = ET.parse(source)
    root = tree.getroot()
    entries = [child for child in root if child.tag.rsplit("}", 1)[-1] in ("game", "machine")]
    removed = 0
    for entry in entries:
        if REDUMP_PRERELEASE_TAG.search(entry.attrib.get("name", "")):
            root.remove(entry)
            removed += 1
    tree.write(destination, encoding="utf-8", xml_declaration=True)
    return removed, len(entries)


def datomatic_form(catalog: Catalog, selector: str) -> dict[str, str]:
    # The site has many optional flag/region checkboxes. Selecting its explicit
    # "all" controls avoids replaying hundreds of dynamic field names.
    parser = MainFormParser()
    parser.feed(selector)
    if parser.prepare is None:
        raise RuntimeError(f"DAT-o-MATIC Prepare control changed for {catalog.key}")
    fields = dict(parser.fields)
    fields[parser.prepare[0]] = parser.prepare[1]
    # These checked defaults are intentionally removed for the bounded cache.
    for excluded in ("release_2", "license_2", "license_0", "lifespan_2"):
        fields.pop(excluded, None)
    # Override only controls that this exact system form advertised. Sending a
    # plausible field from a different system is not forward compatible and can
    # cause DAT-o-MATIC to refuse the request as an unknown filter.
    for name, value in {
        "header_plugin": "0",
        "naming": "0",
        "inc_bios": "0",
        "inc_nodump": "0",
    }.items():
        if name in parser.names:
            if value not in parser.values[name]:
                raise RuntimeError(f"DAT-o-MATIC no longer offers {name}={value}")
            fields[name] = value
    if catalog.datomatic_format is not None:
        if "format" not in parser.names:
            raise RuntimeError(f"DAT-o-MATIC has no required format control for {catalog.key}")
        fields["format"] = str(catalog.datomatic_format)
    if any(value not in parser.values.get(name, set()) for name, value in fields.items()):
        raise RuntimeError("DAT-o-MATIC selection contains an unadvertised value")
    return fields


def find_download_control(html: str) -> tuple[str, str] | None:
    parser = MainFormParser()
    parser.feed(html)
    return parser.download


def fetch_redump(client: RateLimitedClient, catalog: Catalog, archive: Path) -> str:
    url = f"https://redump.info/datfile/{catalog.redump_slug}"
    with client.open(urllib.request.Request(url)) as response:
        save_response(response, archive)
        return response.geturl()


def fetch_datomatic(
    client: RateLimitedClient,
    catalog: Catalog,
    archive: Path,
    *,
    queue_attempts: int,
    queue_delay: float,
) -> tuple[str, dict[str, str]]:
    if DATOMATIC_DOWNLOADS_PAUSED:
        raise ProviderRefusal(
            "DAT-o-MATIC network downloads are paused after error 1260; "
            "use existing catalogs or manually supplied official DATs until reviewed"
        )
    page_url = DATOMATIC_URL.format(system_id=catalog.datomatic_system_id)
    # Establish the session and confirm the system-specific selector still exists.
    selector = ""
    for attempt in range(queue_attempts + 1):
        with client.open(urllib.request.Request(page_url)) as response:
            selector = read_html(response)
        parser = MainFormParser()
        parser.feed(selector)
        if parser.prepare is not None:
            break
        lowered = selector.lower()
        if not any(marker in lowered for marker in QUEUED_MARKERS):
            raise RuntimeError(f"DAT-o-MATIC selector changed for {catalog.key}")
        if attempt == queue_attempts:
            raise RuntimeError(
                f"DAT-o-MATIC still has {catalog.key} queued; rerun later to resume"
            )
        time.sleep(queue_delay * (attempt + 1))
    if f'name="system_selection"' not in selector or catalog.display_name not in selector:
        raise RuntimeError(f"DAT-o-MATIC selector changed for {catalog.key}")

    submitted_controls = datomatic_form(catalog, selector)
    encoded = urllib.parse.urlencode(submitted_controls).encode("ascii")
    for attempt in range(queue_attempts + 1):
        request = urllib.request.Request(page_url, data=encoded, method="POST")
        request.add_header("Content-Type", "application/x-www-form-urlencoded")
        with client.open(request) as response:
            generated_url = response.geturl()
            html = read_html(response)
        download = find_download_control(html)
        if download is not None:
            download_request = urllib.request.Request(
                generated_url,
                data=urllib.parse.urlencode([download]).encode("ascii"),
                method="POST",
            )
            download_request.add_header("Content-Type", "application/x-www-form-urlencoded")
            with client.open(download_request) as response:
                save_response(response, archive)
                return response.geturl(), submitted_controls
        lowered = html.lower()
        if not any(marker in lowered for marker in QUEUED_MARKERS):
            raise RuntimeError(f"DAT-o-MATIC did not expose one generated ZIP for {catalog.key}")
        if attempt == queue_attempts:
            raise RuntimeError(
                f"DAT-o-MATIC still has {catalog.key} queued; rerun later to resume"
            )
        time.sleep(queue_delay * (attempt + 1))
    raise AssertionError("queue retry loop exited unexpectedly")


def cache_catalog(
    client: RateLimitedClient,
    root: Path,
    catalog: Catalog,
    *,
    queue_attempts: int,
    queue_delay: float,
) -> None:
    directory = root / catalog.provider / catalog.key
    archive = directory / "source.zip"
    source_dat = directory / "source.dat"
    dat = directory / "catalog.dat"
    manifest = directory / "manifest.json"
    if directory.exists():
        try:
            record = json.loads(manifest.read_text())
            if record["catalog_key"] != catalog.key:
                raise ValueError("catalog key differs")
            for key, path in [("archive", archive), ("source_dat", source_dat), ("dat", dat)]:
                if (record[key]["path"] != path.name
                    or record[key]["size"] != path.stat().st_size
                    or record[key]["sha256"] != sha256(path)):
                    raise ValueError(f"{key} digest or size differs")
        except (OSError, KeyError, ValueError, TypeError) as error:
            raise RuntimeError("cache is incomplete or changed; preserve it and use a new cache root") from error
        print(f"cached {catalog.key}: {directory}")
        return

    if catalog.provider == "no-intro" and DATOMATIC_DOWNLOADS_PAUSED:
        raise ProviderRefusal("DAT-o-MATIC network downloads remain paused after error 1260")
    directory.parent.mkdir(parents=True, exist_ok=True)
    # Publish a complete new cache directory only after every artifact is validated.
    with tempfile.TemporaryDirectory(prefix=".dat-stage-", dir=directory.parent) as staging:
        staged = Path(staging) / "entry"
        _cache_new_catalog(client, staged, catalog, queue_attempts, queue_delay)
        staged.rename(directory)
    print(f"cached {catalog.key}: {directory}")


def _cache_new_catalog(
    client: RateLimitedClient,
    directory: Path,
    catalog: Catalog,
    queue_attempts: int,
    queue_delay: float,
) -> None:
    directory.mkdir()
    archive = directory / "source.zip"
    source_dat = directory / "source.dat"
    dat = directory / "catalog.dat"
    manifest = directory / "manifest.json"

    submitted_controls: dict[str, str] = {}
    if catalog.provider == "redump":
        final_url = fetch_redump(client, catalog, archive)
    else:
        final_url, submitted_controls = fetch_datomatic(
            client,
            catalog,
            archive,
            queue_attempts=queue_attempts,
            queue_delay=queue_delay,
        )
    member = extract_single_dat(archive, source_dat)
    document = ET.parse(source_dat).getroot()
    document_name = document.findtext("header/name", "")
    document_version = document.findtext("header/version", "")
    if document.tag != "datafile" or not document_version or not (
        document_name == catalog.display_name or document_name.startswith(catalog.display_name + " (")
    ):
        raise RuntimeError("downloaded DAT has an unexpected system or missing version")
    if catalog.key == "no-intro-nes" and "(Headerless)" not in document_name:
        raise RuntimeError("NES payload matching requires a Headerless catalog")
    filter_record: dict[str, object]
    if catalog.provider == "redump":
        removed, total = filter_redump_prerelease(source_dat, dat)
        filter_record = {
            "kind": "redump-name-tag-prerelease-exclusion-v1",
            "input_entries": total,
            "removed_entries": removed,
            "retained_entries": total - removed,
            "tags": ["Alpha", "Beta", "Demo", "Kiosk", "Preview", "Proto/Prototype", "Sample", "Taikenban"],
        }
    else:
        shutil.copyfile(source_dat, dat)
        filter_record = {
            "kind": "provider-selection",
            "submitted_controls": submitted_controls,
        }
    record = {
        "schema": 1,
        "catalog_key": catalog.key,
        "provider": catalog.provider,
        "provider_system": catalog.provider_system,
        "display_name": catalog.display_name,
        "document_name": document_name,
        "document_version": document_version,
        "source_url": final_url,
        "retrieved_at": datetime.now(timezone.utc).isoformat(),
        "selection": (
            {
                "system_id": catalog.datomatic_system_id,
                "submitted_controls": submitted_controls,
            }
            if catalog.provider == "no-intro"
            else {"slug": catalog.redump_slug}
        ),
        "excluded_system_categories": ["source-code-systems", "unofficial-systems"],
        "byte_domain": catalog.byte_domain,
        "byte_domain_version": catalog.byte_domain_version,
        "archive": {
            "path": archive.name,
            "size": archive.stat().st_size,
            "sha256": sha256(archive),
        },
        "source_dat": {
            "path": source_dat.name,
            "archive_member": member,
            "size": source_dat.stat().st_size,
            "sha256": sha256(source_dat),
        },
        "dat": {
            "path": dat.name,
            "size": dat.stat().st_size,
            "sha256": sha256(dat),
        },
        "filter": filter_record,
    }
    manifest.write_text(json.dumps(record, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path, help="cache root outside the repository")
    parser.add_argument(
        "--catalog",
        action="append",
        choices=sorted(CATALOGS),
        required=True,
        help="bounded catalog to fetch (repeatable; explicit selection required)",
    )
    parser.add_argument("--minimum-delay", type=float, default=10.0)
    parser.add_argument("--maximum-delay", type=float, default=18.0)
    parser.add_argument("--timeout", type=float, default=120.0)
    parser.add_argument("--retries", type=int, default=3)
    parser.add_argument("--queue-attempts", type=int, default=2)
    parser.add_argument("--queue-delay", type=float, default=180.0)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    output = args.output.resolve()
    repository = Path(__file__).resolve().parent.parent
    if output == repository or repository in output.parents:
        raise SystemExit("refusing to cache provider DATs inside the repository")
    output.mkdir(parents=True, exist_ok=True)
    lock = (output / ".scrape.lock").open("a+", encoding="utf-8")
    try:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError:
        raise SystemExit("another DAT cache process is already using this output root")
    selected = args.catalog
    client = RateLimitedClient(
        minimum_delay=args.minimum_delay,
        maximum_delay=args.maximum_delay,
        timeout=args.timeout,
        retries=args.retries,
        rate_state=output / ".last-request",
    )
    failures = 0
    for key in selected:
        try:
            cache_catalog(
                client,
                output,
                CATALOGS[key],
                queue_attempts=args.queue_attempts,
                queue_delay=args.queue_delay,
            )
        except ProviderRefusal as error:
            print(f"aborted {key}: {error}", file=sys.stderr)
            return 1
        except (OSError, RuntimeError, urllib.error.URLError, zipfile.BadZipFile) as error:
            failures += 1
            print(f"failed {key}: {error}", file=sys.stderr)
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
