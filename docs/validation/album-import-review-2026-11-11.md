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
