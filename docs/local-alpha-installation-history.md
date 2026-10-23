# Historical local-alpha installation validation

These dated observations describe the installations and tooling exercised at the
time. They are evidence, not current setup instructions or an active work queue.
For installation use the [container guide](../deploy/container/README.md) or
[current operator runbook](local-alpha-installation.md).

## Supplied corpus inventory — 2026-09-17

The user supplied `/Users/scott/dumps` for LA3. Read-only enumeration found 12
package directories: two under `bad`, ten under `likely-valid`. Those names are
user assessments, not verification evidence. No source files were changed, copied,
or hashed during this inventory, and no import/playback qualification is claimed.

- `bad/SCES-00344`: redumper companions and scrambled data, no CUE or BIN found.
- `bad/GUST-9001`: one AUDIO track declared by `image.cue`, with BIN and redumper companions.
- Four dated `likely-valid/2026-*` directories: `dump.toml`, historical evidence,
  and nested `raw/` packages. Two CUEs declare data-only discs; two declare data
  plus audio tracks. Preserve historical files without presenting them as newly
  verified evidence or claiming semantic Retro Junk migration.
- Four identifier/barcode-named directories: flat AUDIO CUE packages declaring
  2, 3, 4 and 18 tracks, with metadata and redumper companions.
- `octopath_0_ost_1`: flat 18-track AUDIO CUE package.
- `octopath_0_ost_2`: flat 16-track AUDIO CUE package with existing FLAC sidecars.
  Existing FLACs must not silently become validated Collector derivatives.

These are name/CUE inventory observations only; structural correctness, audio
correctness, catalog matches and package completeness remain untested.

Current discovery lists direct child directories, while package inventory recurses
within a selected directory. Therefore do not configure `dumps` itself as an inbox:
that offers `bad` and `likely-valid` as two giant packages. For immediate LA3 work,
copy selected whole package directories into isolated inboxes, retaining nested
paths and companions. Mixed collection types still need their respective game/CD
collection destinations. Recursive discovery should eventually distinguish grouping
folders from packages, preview boundaries and ask about ambiguous groupings; it
must not infer package identity merely from directory depth or combine independent
discs. Module-owned format recognition and application-owned intake policy remain
the architectural boundaries for that improvement.

## Owned-corpus execution record — 2026-09-17 (partial qualification)

LA3 was exercised on the selected Mac and through an actual stopped-server restore,
but remains **partially qualified**. Live MusicBrainz/Cover Art Archive access and
automatic cover/text browsing now pass. A configured game DAT was not supplied and
the intended second physical LAN browser was not available. Consequently DAT-backed
game identity (if required for adoption) and second-device encrypted playback and
revocation are not accepted by this record.

### Host, tools and selected roots

| Input | Executed value |
|---|---|
| Host | macOS 26.6.2 build 25G83, arm64, Darwin 25.6.0; LAN `192.168.1.241`; Tailscale `100.81.55.61` |
| Rust / Node | rustc and cargo 1.95.0; Node 24.3.0; npm 11.4.2 |
| Browser | Codex in-app browser, Codex 26.623.81905 with bundled Chromium framework 149.0.7827.197 |
| FLAC | `/opt/homebrew/bin/flac`, FLAC 1.5.0 |
| Provider identity | `JunkCollector/0.1.0 (https://github.com/AberrantWolf/junk-collector)`; project URL used instead of a personal address |
| Source configuration | `/private/tmp/junk-collector-la3-owned.YFRhnq/collector.toml` |
| Source archive / state | `/private/tmp/junk-collector-la3-owned.YFRhnq/archive`; `/private/tmp/junk-collector-la3-owned.YFRhnq/state` |
| Test inbox | `/private/tmp/junk-collector-la3-owned.YFRhnq/inbox` |
| Backup | `/private/tmp/junk-collector-la3-owned.YFRhnq/backup` |
| Restore configuration | `/private/tmp/junk-collector-la3-owned.YFRhnq/restore-parent/collector.toml` |
| Restore archive / state | `/private/tmp/junk-collector-la3-owned.YFRhnq/restore-parent/archive`; `/private/tmp/junk-collector-la3-owned.YFRhnq/restore-parent/state` |
| Listeners | public `127.0.0.1:21841`; private management `127.0.0.1:21842`; LAN TLS `0.0.0.0:21843` |

The run began with about 195 GiB free and ended with about 171 GiB free. The source
corpus remained outside the inbox. Only the following whole package directories
were copied. `bad` and `likely-valid` were never offered as packages.

| Test package | Source directory | Files / bytes | Initial relative-tree SHA-256 | Result |
|---|---|---:|---|---|
| `game-mixed-nested` | `/Users/scott/dumps/likely-valid/2026-07-23-019f8ef8` | 17 / 1,090,422,259 | `d33043af3b7beb0f08bf962d68a5be8d3d99ada2757def44b30a7c432e452c5b` | Nested `raw/` mixed-mode PS1 CUE plus retained `dump.toml` and historical evidence; preservation and 17-file raw hash observation succeeded; no DAT match attempted. |
| `cd-flat-two-track` | `/Users/scott/dumps/likely-valid/_QyyxioVRyBqTsJQKgyelNOp7rE-_4938068001041` | 13 / 334,449,135 | `470cc5bb377a07ef61d33ef6e30cf8214d4ba25091ce6b8d80b9b5371f5ab689` | Explicit Redumper TOC selection produced two tracks and MusicBrainz disc ID `_QyyxioVRyBqTsJQKgyelNOp7rE-`; FLAC preparation and playback succeeded. This is structure/provider identity, not catalog or audio verification. |
| `cd-flac-sidecars` | `/Users/scott/dumps/likely-valid/octopath_0_ost_2` | 40 / 2,455,327,621 | `8dc1312346b0210a7c8f7614bd7b4e95d6ec8af0be70ec144f8be9a541cb3872` | Redumper analysis produced 16 tracks and disc ID `RjOjvqcI8e7qgLeHfx4EtnidRps-`. All 16 supplied FLACs remain members of the 40-file `preservation_master`; none was relabelled a Collector derivative. |
| `damaged-audio` | `/Users/scott/dumps/bad/GUST-9001` | 8 / 510,365,466 | `e8d5db0e27cca5e3024ce7041c285081bc1f09e92512a83b3aca6258ab8911bb` | The user label did not substitute for analysis: the retained package produced a supported one-track Redumper layout and disc ID `2MbcqB6O3GeZQ4HZqr4O4mTi2Zw-`. |
| `damaged-no-anchor` | `/Users/scott/dumps/bad/SCES-00344` | 6 / 950,108,972 | `115f2371f565016da5f9921491c427fdbe2daa18f7145d1083130c35ae8b179b` | Originals published; analysis reported exactly `redumper mixed-mode or four-channel CD audio` and made no supported-layout or verification claim. |

Initial and final `diff -rq` comparisons of every source directory against its
inbox copy returned no differences. The inbox picker enumerated each direct child
as one package and recursively retained the nested game's 17 relative paths. It
does not recursively discover package roots beneath grouping directories; that is
recorded as a usability limitation, not expanded into an importer redesign.

### Real journey and corrections

The browser preview initially treated a package containing both a CUE and a
Redumper TOC as ambiguous with no way to choose. Planning now exposes the validated
CUE/TOC anchors, persists the selected index in the signed plan and durable intent,
and reuses the retained selection for preparation. This fixed two observed failures:
the initial inability to proceed and a first preparation attempt that re-detected
the published package and lost the selection. Focused regressions cover the
Redumper choice and its durable guided-ingest use.

The accepted corrected CD Import was
`b37f4a9e-77a1-5c4e-a161-5edc54831ebf`, Capture
`14c00a90-7ffe-58c5-8cc7-7cbbd7484dd0`, preservation Representation
`5fbe5615-c9e5-5e9b-8c25-505e6e730144` and analysis observation
`14fa2b58-0e50-5dd0-93a7-c9d87d2ee98c`. The two published FLACs were:

- `track-01.flac`, Representation `0ec304f5-9313-5f4a-a138-1768bd79ad1e`,
  33,799,412 bytes, SHA-256
  `eda50537d2d8a3fa81479bc8a3ff20c8dbda0994602f5ce30128c1129f2107d4`.
- `track-02.flac`, Representation `b720f936-eb81-5a94-831a-d4bf1363ac2e`,
  27,623,823 bytes, SHA-256
  `dfb5df12630e407cb75ec88d6664832874c8f9cd2fb0d1c80aad4873d6bdb6f9`.

The actual player advanced the first track to five seconds, paused, and later
advanced under the Viewer account to 31 seconds. After restore it played from the
restored roots to ten seconds and paused. The operator-supplied manual title
`LA3 Manual Album Title` remained marked manual across restart and restore. The
initial provider-outage exercise reported `MusicBrainz is not configured; originals
remain preserved` and fabricated no cover.

After configuring the project URL as the live provider contact, automatic
enrichment reused the preserved Import rather than re-importing it. MusicBrainz
identified release `99c5a9f0-2070-4738-b4db-05e60395f627` as `HOME` by `B’z`;
metadata snapshot/Job `317f6345-5f1c-57f0-a8e1-0476faf1ac61` completed on its
first attempt. The manual title remained the displayed title while provider text
supplied the subtitle.

The live Cover Art Archive listing exposed and verified three journey repairs:
the archive redirect target `dn721500.ca.archive.org` is now admitted only by a
narrow HTTPS hostname pattern, numeric image IDs are normalized to canonical decimal
strings, and retried catalog Jobs expose their nested completion receipt to guided
reconciliation. Listing Job/snapshot `3d46548a-0dc4-530a-8519-783b47370e87`
retained its two failed attempts and completed on attempt 3. Image Job/snapshot
`079c4db4-d8d5-5414-8d7f-ecb50e89a6fd` published JPEG cover Asset
`d23de098-6923-5811-a842-22dcb7ce02db`, 560,318 bytes, SHA-256
`72cbbd8777f858e99de2892e5dced32c0fa983ec6918f5e048228d065f6eab6b`.
The restored browser visibly rendered that cover beside `LA3 Manual Album Title`
and `B’z`, loaded both prepared FLAC tracks, changed to `Now playing`, advanced the
first track past three seconds and paused it at 7.45 seconds. This qualifies live
provider identity/artwork and browsing; it is not catalog or audio verification.

The game inbox Import was interrupted while staging. The durable Job
`009048ea-49bd-58d6-b14c-b7f6e31ff7c5` retained 961,553,973 of
1,090,422,259 bytes at process termination, resumed as the same Job and attempt,
completed without a duplicate Capture, and continued to publication. An anchor
field accidentally added to the shared operation envelope then caused the strict
game decoder to reject that first analysis. The owning application layer now emits
the music-only extension only when selected; a focused guided-game regression
passes. A fresh real browser Import
`37dafd9b-f1b5-5592-b591-e632ace8f366` completed staging, 17-file raw hashing and
publication with observation `28fcd52a-9df7-5fb4-b5f2-00e56f5a1984`. Its exact
limitation is `No active DAT snapshot was selected; exact hashes were retained for
later matching.` The browser labels this as preservation/hash evidence and
unidentified inventory, not catalog verification. The live panel also exposed a
stale pre-publication association selector; it now follows polled server state until
the curator actually edits the selector, and the repaired browser showed the new
game Item selected.

### Accounts, trust and access

Local accounts `owner` (administrator/Owner) and `viewer` (Viewer on LA3 Music)
were used. The Viewer saw only the granted collection, browsed the manual title and
played the prepared FLAC. A direct attempt to promote its own collection role
returned HTTP 403. The same credentials, grant and active sessions worked after
restore. The grant was then revoked: collection listing returned `[]` and a media
request returned HTTP 403. Revoking the actual browser session returned that browser
to the login screen; a separately revoked API session returned HTTP 401.

The LAN TLS endpoint was checked from this Mac with certificate validation against
the copied certificate; validation was not disabled. The private listener remained
loopback-only and public curation/administration remained false. This does not
substitute for the missing second physical device, its browser trust store or its
remote playback.

### Stopped copy and restore results

The source server was stopped before copying and remained stopped while the restored
identity ran. `pairing` exclusively opened each stopped root successfully. Source
and restored profiles both reported ServerId
`01a0adaf-1621-7152-9357-3a6387f04992` and certificate SHA-256
`eb73cbb34df45936e5593643ae42713f4fcdd9f679b996bce6f509f10d980111`.
The TLS key digest was
`319e43415c237427f88c66a9bae3380324566c063e752e572f676dd12f0898c0`.
The two identities were never run concurrently.

While stopped, the source archive manifest contained 88 files and its manifest
digest was `c49c3a0bf87ba8cc805e37c32fc33a5f4c4c7e73cd1594797ff8b3a03f1edf79`.
The source state manifest contained 102 files and its manifest digest was
`15f41813f286e0bcf45986c90dbec0edf9f71c7eeaa12ee26009ed5ee81caeb4`.
Both backup manifests and both pre-open restore manifests matched exactly. The
source configuration SHA-256 was
`c64351f7281e1a68561ca41acc77d88c91b50452b33e523b35180dcfcb159e98`;
at the pre-open restore comparison, the restored config differed only in `archive`
and `state`. The later live-provider continuation added only the recorded
`musicbrainz_user_agent` to that restored test configuration.

Representative source/restore checks also matched exactly:

- original Track 1 BIN:
  `e1fedd7da01bda897e7f84b24f5bdb2c062fa70174f877472c01461515f98eec`;
- structure observation record:
  `931643ad3889e65181ee1b2ca083154606707e337ea4c2028cf7667daf84338f`;
- manual metadata-authority record:
  `579eff44f126f391520453eab8d1cc2e9cecf392a7f64cfdd006f7ae32cfc377`.

Before restored startup, both users and the Viewer grant were present. Four
unfinished downstream Jobs were retained: three CD analyses and the first game
publication. Restored `doctor` and `reindex` passed; the same Jobs resumed and
published the sidecar, damaged and game Captures. After the additional corrected
game journey and access revocations, all Jobs were terminal. A clean stop and
exclusive `pairing` open passed. At that pre-provider checkpoint, the archive
manifest contained 218 files and had digest
`d2e01ccfd72b5289f001397aee7589addba2d587820c55639b4b2a5a02035c91`;
the state manifest contained 120 files and had digest
`00350bbd8b175222616ae11c759b075279996e30e899922cadc1e6c6f1dd6f01`.
Those expected changes are the resumed publications, the corrected second game
journey, access/session revocations and rebuilt projection state—not a failed
pre-open copy comparison.

After the live-provider jobs and browser playback, all Jobs were again terminal.
The server stopped cleanly, all three listeners were closed, and an exclusive
`pairing` open returned the same ServerId and certificate digest. The final restored
archive contains 235 files with manifest digest
`cf07586ba170f7c5a441f136e136cdb156544e29eeb52cea5db2f49cc3429216`;
the final restored state contains 120 files with manifest digest
`f31f3d0645b581dcf7554abc787357d7864672b34a5ed86099ab35ddf368b125`.

To repeat the drill, stop the server, prove exclusive access with `pairing`, create
sorted SHA-256 manifests for the complete archive and state roots, copy those roots
and the config into a new empty destination, compare manifests before opening,
change only the two root paths, run `doctor` and `reindex`, and start only the
restored identity. Preserve the backup as credential-bearing private data. This
record makes no portable-export or change-bundle claim.

## Redump catalog acquired — 2026-09-19

The user supplied https://redump.info/downloads as the catalog source. The standard
PlayStation download at https://redump.info/datfile/PSX returned HTTP 200 and was
saved as `/tmp/junk-collector-redump-psx.dat-download` (ZIP, 3,300,805 bytes).
The extracted XML DAT is `/tmp/junk-collector-redump-psx.dat` (12,834,369 bytes),
header version `2026-09-18 19-52-35`, named `Sony - PlayStation`, with 10976
in the download's catalog label. ZIP SHA-256:
`7b2883f4398f9d0e3018649a0ef6e9701b281b30d9f11fedf7eab0a15301781e`.
DAT SHA-256:
`c698671d4fb304ce636352c245323b1f787d216c8f629f9aac1b328537241bb2`.

The provider ZIP and XML are now cached unchanged under
`/private/tmp/junk-collector-la3-owned.YFRhnq/catalogs/redump/redump-psx/` as
`source.zip` and `source.dat`. The same directory contains a manifest and an
explicit full-release `catalog.dat`: 975 rows tagged Alpha, Beta, Demo, Kiosk,
Preview, Proto/Prototype, Sample or Taikenban were excluded, leaving 10,001 rows.
The derived DAT is 11,993,736 bytes with SHA-256
`f3d24d5d394c6f248c6c479b892c67b382a315bdf39220ad4fdb14f3d8befabc`;
the manifest retains the raw provider digests above and the exact exclusion count.

Collector archived and activated the derived bytes as snapshot
`7d61cc8d-eb58-53e7-ac00-f4f17cde7c92` for system `sony-playstation` and byte domain
`org.junk.game.cd.raw-2352-track` version 1. The first owned four-track comparison
correctly remained partial because the provider row also contained a CUE checksum.
The local-DAT adapter was then corrected to retain that CUE in explicit
`excluded_files` while comparing only `.bin` files whose lengths are exact
multiples of 2352. Reindex reparsed the same immutable snapshot bytes. Browser
observation `1667ceb8-11ac-55a7-96a2-bae41a97ce2b` then produced one complete
four-SHA-1 match, `Brave Fencer Musashi (USA)`, provider record 920. Historical
partial observation `a8f93fb8-968d-5118-8b44-d828399f37b6` remains retained. The
exact result supplied an automatic title candidate; applying it did not replace or
upgrade verification evidence. This is catalog verification for the analyzed
track set, not proof of subchannel, CUE-text or whole-package preservation identity.

## No-Intro reference acquisition — 2026-09-19

The user supplied DAT-o-MATIC's selector:
https://datomatic.no-intro.org/index.php?page=download&s=64&op=dat
System 64 is Nintendo 3DS; the general download page also lists other systems.
A session-cookie-preserving GET, POST of the main form's selected defaults and
Prepare button, followed by the visible Download button on the returned manager
page, successfully downloaded ZIP catalogs without login. Button names and the
manager URL are generated; do not hard-code the observed tokens or assume a stable
API. No shoutbox or login forms were submitted.

Two reference catalogs were downloaded and their XML parsed for inspection before
the supported-platform cache was narrowed:

| System | Default catalog variant | Version | XML game entries | Local extracted DAT |
|---|---|---|---|---|
| 64 | Nintendo 3DS (Encrypted) | 20260914-103505 | 2165 | `/tmp/junk-collector-no-intro-64.dat` |
| 45 | Nintendo Entertainment System (Headered) | 20260918-104030 | 4553 | `/tmp/junk-collector-no-intro-45.dat` |

Original ZIPs are at the corresponding `/tmp/junk-collector-no-intro-<id>.zip`
paths. These are temporary acquisition fixtures, not bundled or configured product
catalogs. System 64 (Nintendo 3DS) is outside the implemented byte-domain matrix and
was deliberately not copied into the qualification cache. No owned-ROM match was
attempted for either temporary reference.
The NES sample includes per-ROM SHA-256, header bytes and a provider verification
status; the 3DS sample includes CRC/MD5/SHA-1 and serial data. Retain raw XML so
additional provider attributes are not lost. Provider status is not proof that a
user's dump matches.

Any future downloader belongs behind provider acquisition ports and existing
snapshot publication. Record system, selected filters/variant, source, acquisition
time, catalog version and digest. Headered NES hashes must not be compared to
header-stripped payload hashes; encrypted/decrypted variants likewise need explicit
byte semantics. Preserve the existing local-file path as a fallback, bound download
and ZIP expansion, and fail visibly on HTML/error responses or changed forms.
This observation does not implement automatic refresh or expand LA3 scope.

- `junk-collector-no-intro-64.zip` SHA-256: `c0364a66461d255d3663d77492ad6f28b794785c5d9323c99e9a000f1d608698`.

- `junk-collector-no-intro-64.dat` SHA-256: `f2ba7d17a86a246369514a92f02a790605d3d3e5c0ef36333c22ca5829d5c09d`.

- `junk-collector-no-intro-45.zip` SHA-256: `a9cc496a13a6c6681997d04af00931315b63243faa4cc08f9b62929764602560`.

- `junk-collector-no-intro-45.dat` SHA-256: `5b5dda635389f975e1151b0fab1530ef588473681bb85c0af9322cc0777df392`.

### Bounded supported-platform DAT cache

`scripts/cache_game_dats.py` is a manual, standard-library downloader, not an
automatic server refresh path or a general crawler. It has an explicit allowlist
for Redump PlayStation and No-Intro NES, N64, SNES and PC Engine catalogs matching
the byte domains currently accepted by the game module. Source Code and Unofficial
systems are not addressable. No-Intro requests select full releases, licensed and
machine-life-span titles when those controls exist; the resulting manifest records
all selected controls. The Redump source is filtered only into a separate derived
DAT while the raw ZIP/XML remain unchanged.

The script makes one request at a time, defaults to 10–18 seconds of jitter between
ordinary requests, honors bounded `Retry-After`/429/503 backoff, waits 180 seconds
or more for DAT-o-MATIC generation, caps HTML at 4 MiB and ZIPs at 64 MiB, and uses
an output-root lock plus persistent request timestamp across invocations. Completed
entries are digest-checked and reused; refresh into a new cache root to retain
existing snapshots. Invoke individual catalogs, for
example:

~~~shell
python3 scripts/cache_game_dats.py /selected/catalog-cache --catalog redump-psx
python3 scripts/cache_game_dats.py /selected/catalog-cache --catalog no-intro-nes
~~~

The isolated cache currently contains:

| Key | Provider result | Catalog digest | Status |
|---|---|---|---|
| `redump-psx` | 10,001-entry explicit full-release view; raw 10,976-entry XML retained | `f3d24d5d394c6f248c6c479b892c67b382a315bdf39220ad4fdb14f3d8befabc` | Parsed, indexed and matched against the owned four-track package |
| `no-intro-nes` | Headerless `20260918-104030`, 2,661 entries, 1,107,207 bytes | `4b7961f4737120e10cdd925b63a9ff1a84b73b078cd4088627787f8674a260fd` | Cached and indexed as snapshot `9e3645f9-e600-5ccc-8e30-030932e023d7`; ZIP digest `0b5861775b54940916affd77bd9566192bf8886fb869980df575b3fd1d0bbe78`; no owned NES dump supplied or match claimed |

DAT-o-MATIC generated the NES file only after its queue delay. A later N64 request
was refused with provider error 1260, `Unknown filter`; the site then reported that
the network ban requires contacting its operator. No evasion or further request was
attempted. The downloader was corrected to submit only controls present in the exact system
form and abort the whole batch on provider-refusal markers. The 2026-09-20 review
also disabled new DAT-o-MATIC network downloads in code pending a reviewed restart;
existing verified cache entries remain readable. N64, SNES and PC
Engine remain uncached until the provider restores access or the operator supplies
their ZIPs manually. Do not treat the refusal as a frequency-limit result, and do
not retry automatically.

## Independent LA3 review — 2026-09-20

DAT-o-MATIC downloads remain paused after the operator contacted the site. The
script refuses new No-Intro acquisitions before any request, even if access has
been restored; no live requests were made during this review. Redump remains
separately selectable. `--catalog` is now required, so an omitted selection cannot
start a multi-provider batch. Existing cache entries have their recorded sizes and
SHA-256 digests checked on reuse. Refreshes require a new cache root rather than
replacing an existing snapshot. New entries are staged privately and published
only when the ZIP, extracted DAT, derived catalog and manifest are complete.
Expanded DATs are limited to 128 MiB in addition to the 64-MiB download bound.
Form overrides validate advertised values as well as field names; live form
compatibility is still unqualified. Run the offline regressions with
`PYTHONDONTWRITEBYTECODE=1 python3 -B -m unittest discover -s scripts -p 'test_cache_game_dats.py'`.

Raw-2352 exclusion is now narrow module-owned policy: CUE rows remain retained
outside track coverage, but malformed BINs or unknown payload types fail the
catalog instead of being silently omitted beside good tracks. The matcher also
checks that included/excluded fields agree with that policy. These changes do not
reclassify historical observations or claim whole-package verification.

Correction to the earlier refusal summary: the old batch continued to SNES/PCE
and made a diagnostic fetch after its N64 failure, before the refusal was recognized.
It stopped after recognition, not immediately after the first failing response.
The exact field that caused provider error 1260 has not been established.
