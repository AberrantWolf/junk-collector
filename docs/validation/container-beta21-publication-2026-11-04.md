# Beta.21 qualification, publication and application host update — 2026-11-04 JST

> Deployment identifiers are anonymized. Dates, source and artifact identities, outcomes, and qualification limits are retained.

The user authorized review, fixes, pushing, a new release and installation on the application host.
GitHub failures must be diagnosed and repaired before another attempt unless the
cause is demonstrably external. This record distinguishes completed outcomes from
pending work; earlier source-only acceptance remains dated history.

## Candidate and review

Beta.21 includes normal authenticated website audio by default, a revision-fenced
persistent admin route preference and the unreleased cancelled-publication cleanup
from `71eaf64`. The [audio record](audio-connection-policy-2026-11-04.md) owns the
cross-layer contract, prior local coverage and initial beta.20 configuration change.

Review found a fresh-volume deployment failure in the initial optional-authority
preflight: this remote-archive profile always starts media, whose first startup
requires its service key. The repaired guard requires the provisioned authority
while permitting normal audio. Core HTTP readiness follows authority startup.
Offline coverage checks normal mode, missing authority and incorrect sockets.
The application host regained its retained endpoint configuration during the stopped update,
with normal audio still selected, enabling admin-only switching.
No remaining actionable authorization, revision/receipt, byte-read or client-route
finding was identified. Five existing Rust ignores and the local rootless-skopeo
skip remain unqualified rather than silently counted as passes.

The official Rust stable manifest was rechecked on 2026-11-04. It remains dated
2026-10-15 and selects Rust 1.99.1 (`58cf8f9c3`, 2026-10-13); repository, CI and
Docker pins remain unchanged. No external dependency or toolchain upgrade is part
of this release; workspace package versions alone advance to beta.21.

## Qualification and installation status

Qualification, publication and the application host update completed successfully at signed source
`280b04f6cfe76f6f52122e63ea00e1238e37d1e6`. The application host is healthy on beta.21 with normal
audio selected and its separate endpoint retained for the admin toggle. The
stopped-writer recovery pair, immutable image identity and live acceptance are
recorded below. Earlier failed attempts remain explicit historical evidence.

## Previous main-branch GitHub failure and correction

Checks [41623865565](https://github.com/AberrantWolf/junk-collector/actions/runs/41623865565)
at `71eaf64` failed in integration at the immediate metadata-search row count.
The assertion slept 350 ms and sampled cleared results before asynchronous query
completion. It now waits for the exact successful Library search response and its
single matching rendered row. A one-use 600 ms delay forwards the actual request
rather than inventing successful results, defeating the old fixed-delay shortcut.
The adjacent manual-title lookup relies on its existing actionable-row wait.
This was diagnosed and repaired before pushing another candidate, not retried as
an unexplained hosted-runner failure. Final qualification remains pending.

## Reviewed local checks before push

The repaired integration journey passes in Chrome with its deliberately delayed
metadata search (151.66 seconds for the complete journey); the remote-media
workflow also passes, including default signed-in playback and immediate admin
route switching. Focused store settings, native/private TLS and isolated-media
Rust tests pass. Preflight/container-tools, web type checking/build, generated
contracts, architecture/document links, format and diff checks pass. Broader prior
local coverage is retained in the dated audio record; clean exact-source release
coverage must still come from the candidate workflow.

Application host preflight reports 123 Items, one user, two grants and 6,811 Jobs, with no unfinished
Jobs, paused mail connection revision 2 and a fresh synced Bridge. Local Btrfs has
about 458 GB free, and the archive remains the intended hard NFSv4.1 mount. These
read-only observations do not constitute deployment or a recovery point.

## First beta.21 qualification failure and repair

Candidate `f4a46b7` source Checks
[41784619475](https://github.com/AberrantWolf/junk-collector/actions/runs/41784619475)
passed 636 Rust tests (five existing ignores), strict Clippy, tooling/web/model
checks and the preceding browser journeys, including the repaired metadata search.
It failed in `digital-purchases.mjs` during `setInputFiles` on the folder picker,
waiting 30 seconds for a directory input event before discovery began. The hosted
browser was Chrome 157.0.8103.38; local Chrome was 156.0.8078.74.

Inspection shows the Imports picker is disabled during asynchronous IndexedDB
draft restoration, while Playwright protocol file injection does not wait for
enabled state. A local minimal case confirmed that direct injection can act on a
disabled control, so these tests bypassed real user readiness. Folder selection now
uses a click, which waits for enabled state, followed by the actual file chooser.
The initial folder, resumed-folder and legacy guided-folder journeys share this
mechanism; package discovery/publication and all original assertions remain.
This repairs the unsafe test interaction without increasing its timeout or
substituting file lists for actual directory selection. Final hosted verification
is required; a specific Chrome defect is not asserted from this evidence alone.

Both native image jobs and AMD64 release-image browser acceptance passed in
[41784619526](https://github.com/AberrantWolf/junk-collector/actions/runs/41784619526),
but that qualification run was cancelled after the source failure, before its
fallback could execute the same source checks again. These exports are not sealed
release evidence and will not be published. No workflow was blindly rerun. The next
candidate's source Checks must pass before container qualification is dispatched.
The application host remains on the healthy beta.20 image; no deployment or recovery snapshot has
been performed for beta.21 yet.

The readiness drill now selects a real directory through a picker initially
blocked for 500 ms; the chooser waits for readiness and returns the actual file
(local measured 867 ms). The repaired complete purchase journey passes locally
(67.11 seconds), including real Steam-style nested folder discovery, reviewed
sharing, publication and playback. The cancelled qualification's fallback source
step never executed. The complete integration/import/resume journey also passes (142.01 seconds),
including real initial, resumed and guided directory selection, before the
repaired candidate is pushed.

## Hosted directory investigation after the readiness repair

The readiness repair at `2dbf773` passed locally but source Checks
[41787962138](https://github.com/AberrantWolf/junk-collector/actions/runs/41787962138)
failed at the same directory assignment through the real chooser. The click was
successful; waiting for enabled state did not resolve the hosted failure. No
container qualification was dispatched for that failed source candidate.

An isolated diagnostic branch uses a read-only manual workflow, with no publication
or release steps. Its first run
[41791762293](https://github.com/AberrantWolf/junk-collector/actions/runs/41791762293)
failed before the browser while restoring the Rust cache: runner annotations report
`No space left on device`, including failure to write its own worker log. The cache
was about 5.7 GB compressed. Before another diagnostic, unused Android, .NET, GHC
and CodeQL installations were removed on the disposable runner, with filesystem
headroom recorded before/after restoration. This correction is also prepared for
the main source-check workflow; it does not remove required build or browser tools.

The corrected diagnostic
[41793373137](https://github.com/AberrantWolf/junk-collector/actions/runs/41793373137)
restored a 29 GB target directory with 77 GB free. All minimal real directory
selections passed in Chrome 157.0.8103.38 and Chromium 153.0.8010.12, including
nested folders and relative paths. The complete purchase journey still failed:
the chooser control stayed enabled and connected, but received neither files nor
input/change events. Thus neither disk pressure nor a general Chrome directory
failure explains that remaining result. Further session/context diagnostics are
in progress; this diagnostic failure is not release acceptance. No failed release
workflow has been blindly rerun, and the application host is unchanged.

The expanded diagnostic
[41794430747](https://github.com/AberrantWolf/junk-collector/actions/runs/41794430747)
localizes the remaining failure to the existing long-lived page. After the failure,
a plain directory input substituted into that page still times out. A new page in
the **same signed-in browser context** immediately selects the exact same five
files and relative paths. New contexts over HTTP/HTTPS and a new browser also work.
The source directory exists, has normal ownership/permissions and all expected
entries; deleting the earlier download does not repair the old page. This is not
an Imports handler failure or a missing test fixture. The independent Steam-format
import scenario now opens a fresh page in that same authenticated context. Every
folder-discovery, sharing, publication and immediate Library refresh assertion is
retained, along with the preceding playback/export/analysis assertions. The precise
long-lived-page/CDP sequence remains an unresolved browser-automation limitation;
these diagnostics do not establish that an ordinary native file picker fails.
Final source qualification of the isolated journey is still required.

## Final qualification and publication

- Source [Checks 41795544460](https://github.com/AberrantWolf/junk-collector/actions/runs/41795544460)
  passed at `280b04f`: full coverage from beta.20, 636 Rust tests, five existing
  ignores, strict workspace Clippy, format, architecture/document links, tooling,
  generated contracts, web types/build/models and all 30 browser journeys. The
  receipt records `dirty=false`, `status=passed` and `source_unchanged=true`.
  The purchase journey passes in hosted Chrome 157.0.8103.38 with real folder
  selection, disjoint packages, publication and immediate Library refresh.
- [Qualification 41798535769](https://github.com/AberrantWolf/junk-collector/actions/runs/41798535769)
  reused that exact receipt; both native AMD64/ARM64 release exports passed their
  actual runtime gates, and AMD64 release-image browser acceptance passed. Rootless
  OCI transport was exercised on Linux after installing skopeo; the earlier local
  skip is not counted as a local pass. The accepted exports were assembled,
  reverified and sealed as artifact `13306866217`, attempt 1, artifact digest
  `sha256:2363138841b8137cc4b98aa3f7c5ad1134bed47a6a5690ce8631f32fb78df1c8`.
- The signed tag `v0.1.0-beta.21` targets that exact source. Its redundant tag-push
  Checks run `41799051589` was cancelled because the same source was already fully
  qualified; it is not additional passing evidence. No failed run was blindly
  retried. The diagnostic-only branch is not release evidence.
- [Publication 41799054107](https://github.com/AberrantWolf/junk-collector/actions/runs/41799054107)
  restored those exports, reverified their labels/digests, published without another
  build and passed anonymous verification. The release package and `release-image.json`
  match `SHA256SUMS`; the downloaded sealed OCI archive matches its recorded digest.
  [Beta.21](https://github.com/AberrantWolf/junk-collector/releases/tag/v0.1.0-beta.21)
  is a public prerelease. `preview` matches its immutable index and `latest` remains
  absent, unchanged from before publication.

| Published object | SHA-256 digest |
| --- | --- |
| Multi-platform index | `897ed9af50f3f3e500cd83f3177d95389a4d7339cbf2209b9fac07310b5dbc74` |
| Linux AMD64 manifest | `0904c0cc1d5fd24bec18d3c31b77bdbb458b9add6a304cd381fb95b958b58b79` |
| Linux ARM64 manifest | `a719ae22db7b8115e03c9273c53c3cc0ae8ac7accca95132febd96ff7652fdc7` |
| Sealed OCI archive | `0bb3ca7269c5106fd8380d7ee672c7da34b1d1bc5eebfbe0aa7f7e1a335eeac5` |

Install reference:
`ghcr.io/aberrantwolf/junk-collector@sha256:897ed9af50f3f3e500cd83f3177d95389a4d7339cbf2209b9fac07310b5dbc74`.
The application host source archive checksum is
`6643da05bfb38dc6ce2ef92a15415d9e6b3aea79080e2fdff6d111a209f2a38c`.

## Application host activation and live acceptance — 17:27 JST

The application host anonymously pulled the immutable index using an isolated empty Docker auth
configuration, and its platform/version/source labels matched qualification.
Before activation, all five stack containers were stopped through the owning
`junk-collector-remote.service`; no Jobs were cancelled. Both databases passed
`quick_check` against the stopped full-state snapshot. The complete recovery pair
is sealed in private metadata:

- The read-only full application-state snapshot includes config, private/catalog
  state, profiles, certificates and Caddy data. The media authority service key
  and exact deployment/env/unit files are retained alongside it with private
  permissions.
- The private record identifies the read-only whole archive-parent snapshot and
  its source. The operator checked for nested subvolumes and NAS Collector writers
  before snapshotting and found neither. This is a matched stopped-writer pair,
  not a partial database rollback or an independent/off-site backup claim.

The exact source's preflight and HTTP readiness overlay were installed. The
preflight uses systemd's existing EnvironmentFile, verified read-only before the
update; it is not run with an accidentally empty helper environment. Only the
saved remote-media endpoint was restored to active config. Normal audio stays
allowed and automatic private playback stays disabled. Other parsed config values
and TLS material are unchanged. Core/media image references alone advance to the
accepted image; activation uses the whole systemd-owned Compose stack.

All five containers run, core is healthy, CLI reports `0.1.0-beta.21`, core/media
report the exact source and both have restart count zero. Bridge is fresh/synced
with its unchanged independently checked certificate pin. Bridge, Caddy and tunnel
container IDs/images, home, profiles, topology, permissions and mounts are retained;
seven unrelated running containers retain their IDs and start times. The before/
after facts match: 123 Items, 6,811 Jobs, one user, two grants, two profile files,
one paused mail connection at revision 2, unchanged cursor and no unfinished Jobs.
Receipt suggestions and accepted purchase facts are compared, not recreated.

Trusted public and LAN HTTPS health/identity return 200 with unchanged server identity. All eight checked assets, including the new
administration bundle, match frozen release source. Anonymous playback discovery,
audio bytes, FLAC export and admin audio-policy requests return 401 on both origins.
The provisioned separate HTTPS identity also returns the same ServerId. An early
live probe during stack startup returned 502; it was not counted as acceptance.
After the explicit readiness check succeeded, all live checks above passed.

Authenticated production playback, iPhone/Safari and cellular behavior were not
retested during this update. Real signed-in playback, ranges, seeking, route
switching and permission denial are qualified by the isolated source/image tests;
those are not a claim of a live listening session on the application host. Five existing Rust ignores
and the precise old-page directory automation limitation remain documented.
Private deployment evidence stays on the application host; session logs, public receipts and registry
checks are retained under `/tmp/collector-beta21` for this working session.
