# Album import review — 2026-11-11

Source work for preserving alternate encodings in one digital album, reviewed
playback membership and manual/discovered disc numbering. No release or deployment
is implied. Earlier dated format-folder validation retains its historical behavior.

## Scope and acceptance

Digital top-level ownership, exact originals and companion visibility, preferred
FLAC/WAV/MP3 playback, named-section and count-mismatch review, file/folder
checkboxes and disc dropdowns, durable selections, catalog reindex, top-level inbox
file staging and safe opt-in cleanup are covered by focused fixtures. A 1,001-file
browser review checks bounded child rendering and pagination without importing
its intentionally unanalyzed content.

## Rust, contracts and tooling

Three discovery regressions failed against the earlier implementation because it
split encodings and sections. The current fixtures preserve distinct top-level
albums and raw-disc ownership while grouping digital variants. The former nested
separation expectation is superseded. The existing ambiguity test exercises
same-format copies, which can still require a decision after tag analysis.

A standalone collector-store test build exposed the existing missing Tokio macro
feature without workspace feature unification. The workspace run used
`COLLECTOR_TEST_FLAC=1`. Its archive suite had 316 passes and three ignored cases.
One fixture replaced the entire override payload during a track-label edit and
omitted the reviewed album membership. It passed the focused rerun after retaining
those explicit choices. Real WAV-to-FLAC preparation, source-bound playback,
private alternate authorization, durable review/reindex and interrupted top-level
inbox-file cleanup all passed. The module and PCM suites passed a focused rerun. The remaining email-purchase,
job-dispatch, storage-guard, Wave 2 regression and workspace doctest commands passed.

Strict all-target workspace Clippy, formatting, OpenAPI/TypeScript contract matching,
web type checks/build and queue/context/label models passed. Architecture, link and
fixture checks passed. Selected tooling, container-policy and mail checks passed
with one existing container-validation skip.

## Browser acceptance

The real-browser digital matching, intake review, short smoke and local playback
journeys passed before the final browser-only draft restoration and folder-control
adjustments. Development runs exposed a duplicate React runtime in the new bundle,
a pagination assertion that assumed filesystem enumeration order, and loss of
file handles and uncommitted browser review state across immediate reload. The
final UI shares the app runtime, checks disjoint pages, acknowledges committed
review state, and restores selected files without replacing the saved review.

The final debug server passed the purchase and short smoke journeys in Chrome
156.0.8078.74. Purchase acceptance covered all three originals on one album,
FLAC playback selection, named-layout review, folder/file controls, manual disc
assignment, saved-review reload and original-file reselection, two separate
reviewed disc packages with playback ordered by disc, and 1,001-file review with
50 disjoint rendered entries per page. The 390 px review capture was inspected
and had no horizontal overflow. Artifacts are under
`<temp-dir>/collector-purchases-9hqjtb`, including `format-discovery-mobile.png`.
Final debug server SHA-256: `b1545f940c9568a134b2c751ab9e786f53b927f35c2ac46b58a838f030498ad4`.

The Opus browser journey was not run because `opusenc` was unavailable. No release
or deployment qualification was attempted.

## Follow-up review

A failed browser preview could replace the file handles for the previous review.
Discovery now installs the new inventory and its handles together after success.
The browser test rejects a replacement preview, then imports the previously
reviewed originals. It also prepares a grouped two-disc import with WAV and MP3
originals and verifies both reviewed MP3 tracks play in disc order and neither
WAV was converted. Guided preparation now admits work for its own source and
reports review failures before scheduling preparation.

Eight focused archive cases passed with real FLAC enabled. They include guided
preparation and cancellation, restricted preferred formats, and durable playback
review. The disc-folder precedence test, web type check and models, strict
all-target Clippy for the affected Rust crates, and documentation checks passed.
Chrome 156.0.8078.74 passed the purchase and smoke journeys. Browser artifacts are
under `<temp-dir>/collector-purchases-Dt0l3s`. The tested debug server had SHA-256
`13b2a3141b2091073f73559f0a780102e198043d78392fc12d426a230da738a3`.
The full workspace suite was outside this focused rerun. Opus browser acceptance
requires the unavailable `opusenc` executable.

## Collection destination and disc controls follow-up

A read-only inspection of an installed beta.25 in Firefox showed a mixed
soundtrack inbox review under a Games collection. That review displayed
original-file privacy checkboxes and a file-order editor. Playback proposals
and disc assignments were available only when the same source was reviewed in
a Music collection. The source change adds a collection switch, separates the
playback and sharing controls, and sorts mixed directory entries by format
without reordering the preserved inventory. It replaces the file-order buttons
with a package-level disc count and per-file and folder assignment.

The real-server purchase journey reproduced the Games destination, switched
to Music, and confirmed that two FLAC tracks were checked by default and two MP3
tracks were outside the playback selection. It exercised both format shortcuts,
disc-count reduction, draft reload, and publication. The saved album played FLAC
from discs 1 and 2 and retained all seven originals. Desktop and 390 px captures
were inspected and the browser checked for horizontal overflow. An initial run
timed out at a screenshot locator; the corrected journey passed in Chrome
156.0.8078.74. Captures are under `<temp-dir>/collector-purchases-9tdK0S`.

Web type checks, bundle build, web models, documentation checks and the short
browser smoke passed. The inbox review journey also passed in Chromium
153.0.8010.12, covering anchor selection, privacy, draft recovery, cleanup and
31-album activity. Tested debug server SHA-256:
`abfbb165e7b1107df0ec477265eec2ad25d796cecf33bcffa8c3e5b523bf823f`.
This follow-up did not qualify a release or change the live installation.

## Assign discs from a track onward

Chrome 156.0.8078.74 passed the purchase journey with two successive disc splits
in the 1,001-file review. The first split crossed a page boundary and the second
retained the disc assignments before its starting track. The mixed inbox case
verified that unchecked MP3s stayed outside playback, the reviewed assignments
survived reload, and the imported album played the two FLACs on separate discs.
Desktop and 390 px captures were inspected under
`<temp-dir>/collector-purchases-v64lET`. The initial fixture timed out while
trying to apply the already displayed disc count. The passing fixture creates
each new disc from its starting track.

The short smoke, type check, bundle build, web models and documentation checks
passed. Tested debug server SHA-256
`cf7d611fdf31c080db6a9c28f209b55cecc1d25e77871b8288c676cf7a5aee09`.

## Aligned file review and editable track numbers

Chrome 156.0.8078.74 passed the purchase journey on the final debug server.
The mixed inbox fixture used Enter and Tab to move a numbered track, used both
arrow directions, and kept keyboard focus on the edited track after Enter.
The reordered review survived reload. Inbox staging preserved the reviewed
file order, and the imported playback snapshot showed discs 1 and 2 with track
numbers 1 and 7. Unchecked MP3 files had no disc or track number controls and
were retained as originals. The 390 px and desktop captures were inspected under
`<temp-dir>/collector-purchases-97cKpk`.

Earlier runs exposed a tie in the keyboard test fixture, overflow outside the
mobile table scroll area, and an inbox staging comparison that treated sorted
directory order as part of file identity. The passing fixture uses distinct
numbers, and the final UI and staging checks cover both fixes.

The short browser smoke, module layout tests, 14 inbox archive tests, and the
reviewed-number reindex test passed. The inbox review journey passed in Chromium
153.0.8010.12, including anchor selection, privacy, cleanup, and 31-album
activity. The standalone archive target needed Tokio
macros enabled through its provider dependency. Strict Clippy for the affected
crates, web type checks, bundle build, web models, API contract matching,
formatting and documentation checks passed. Tested debug server SHA-256
`3076dceba282c642470d5dded1cd68091c83a9d638b90d2a5a02e18d5d654429`.

## Selected-track numbering — 2026-11-12

The Chrome purchase journey reproduced a numbering gap on beta.26. After
switching a mixed package to MP3, the first checked MP3 showed track 2. The
corrected journey showed track 1, verified that a new disc starts at 1, and
imported a batch with independent untagged albums. Playback returned 1 and 2
for one album and 1 for the selected track on disc 2 of another album. The
existing manual-number and embedded-tag checks also passed.

Chrome 156.0.8078.74 passed the purchase journey. Digital album matching,
the short browser smoke,
collector-app tests and strict Clippy, web type checks, bundle build, and web
models passed. The debug server SHA-256 was
`0e2ed762d31ddbe7a3fea97f1a22ac438e41b2a17353c2d011bb6158af5e04dd`.
This source change has no release or deployment acceptance.
