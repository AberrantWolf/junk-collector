# Beta.21 qualification, publication and Fi update — 2026-11-04 JST

The user authorized review, fixes, pushing, a new release and Fi installation.
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
Fi will regain its retained endpoint configuration during the stopped update,
with normal audio still selected, enabling future admin-only switching.
No remaining actionable authorization, revision/receipt, byte-read or client-route
finding was identified. Five existing Rust ignores and the local rootless-skopeo
skip remain unqualified rather than silently counted as passes.

The official Rust stable manifest was rechecked on 2026-11-04. It remains dated
2026-10-15 and selects Rust 1.99.1 (`58cf8f9c3`, 2026-10-13); repository, CI and
Docker pins remain unchanged. No external dependency or toolchain upgrade is part
of this release; workspace package versions alone advance to beta.21.

## Qualification and installation status

Exact-source GitHub source checks, both native image/runtime gates, AMD64
release-image browser acceptance and sealed-artifact publication are pending.
Fi remains healthy on beta.20 with normal audio enabled. Before installation,
verify pending work and retain a matched stopped-writer Fi/NAS recovery pair.
Only qualified bytes may be installed. Preserve identity, originals, private state,
manual authority, meaningful evidence, credentials, Bridge/Caddy/tunnel topology
and unrelated services. No production Jobs will be cancelled to make the update
convenient. Final outcomes, digests and any failed attempt will be recorded here.

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

Fi preflight reports 123 Items, one user, two grants and 6,811 Jobs, with no unfinished
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
Fi remains on the healthy beta.20 image; no deployment or recovery snapshot has
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
workflow has been blindly rerun, and Fi is unchanged.

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
