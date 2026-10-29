# Digital album matching delivery — 2026-10-29

## Scope and implementation

Shared `junk-libs-music` provides exact-revision Chromaprint Test2 over bounded
streaming FLAC/MP3 decode, community MusicBrainz entity decoding with our own query
encoding, bounded AcoustID interpretation, maximum-cardinality one-to-one mapping
and ambiguity/coverage/duration reporting. HTTP execution, credentials, lifecycle,
rate limits, cache and metadata authority stay in each application.

Collector integrates the shared mechanisms with existing durable processing,
provider acquisition and reviewed metadata resolution. It preserves source
fingerprints, computation coordinates and chosen raw provider evidence. Repeat
searches can reuse equivalent source-bound computation; cancellation and stale
inventory/source/metadata authority prevent acceptance. No original-file writes
or browse-time audio reads are introduced.

Trash Amp uses the same mechanisms for local folders, with cancellable background
work, digest-bound cached fingerprints, explicit review, source validation before
application and retained accepted metadata for offline refresh. Existing names
retain authority. Collector-managed albums open their server's metadata workspace.
The work was isolated in a feature worktree before integration with the stable root.

## Validation

- The dependency [probe](../digital-album-matching-probe.md) compares corrected
  chromaprint-next with native 1.6.2 across rates/channels/algorithms/chunk boundaries.
- The shared crate has independent native 1.6.2 Test2 vectors for the actual decoded
  generated FLAC/MP3 fixtures, as well as cancellation, truncation, schema, encoding,
  one-to-one assignment and ambiguity tests.
- Collector's real-storage integration exercises audio import, durable reopen,
  idempotent admission, reviewed application, unchanged masters, fingerprint reuse,
  stale metadata rejection, cancellation, reindex and a second store reopen.
- The real browser journey runs generated audio through server jobs and offline
  HTTP provider fixtures, reopens pending work, inspects mobile layout/artwork and
  mapping, then verifies accepted track names in the album view.
- Trash Amp tests exercise real local-file decode, reviewed application, source
  changes, duplicate source track numbers, source/manual name authority, offline
  refresh and cancellation. Slint controls cover reopen, cancel and eligibility.

The fixture sources contain only synthetic audio; no user music or purchase data
was used. Final integration/qualification completed on 2026-10-30 (Japan time).

The full change selector chose `infra`: architecture/links, formatting, web
checking/build, testing/container/release-policy checks, workspace tests/Clippy,
web model tests, rebuilt server/media executables and all 28 browser journeys.
All selected check categories pass after fixes. The final main Rust workspace
rerun passes 538 tests; five existing opt-in/scale tests remain ignored. The new completion-budget regression and real-storage matching test
also pass after the final safeguards, as does strict all-target workspace Clippy.
Type checking/build and the queue/context/label model tests pass. All 28 real
Chrome journeys pass, including the existing broad integration; the matching
journey was rerun against the final rebuilt server and served assets.

Trash Amp main passes 236 workspace tests, with three existing opt-in tests
ignored, strict all-target Clippy and formatting. All 28 release-tooling tests
pass. Runtime notice generation succeeds and includes the fingerprint dependency's
LGPL text. Actual Skia mobile settings/error renders were inspected, and Slint
controls check mobile/desktop action bounds and 1.5× text. Interactive desktop
inspection was unavailable while the Mac was locked. Two real server fixtures
verify the Collector link's originating server, collection and absence of native
credentials. The integrated main diff and all new files were checked against the
validated worktree before rerunning the native checks.

Self-review fixed queued selection changes, duplicate local source numbers,
inconsistent completed assignments, stale/unverified-source restoration, native
error visibility, mobile action overflow and the whole completion-record budget
(including escaped source names). The shared crate passes 13 tests and strict
Clippy, including independent native fingerprint vectors.

Initial broad checks encountered disk exhaustion; only disposable task caches
were removed and final builds disabled incremental/debug artifacts. AVFoundation
tests initially failed in seatbelt and passed outside it. Browser fixtures were
corrected to use complete public MusicBrainz entities and ISO 639-3 language
values; the production parser was not weakened to accept those incomplete
fixtures. Clippy findings were corrected and rerun. These failures are resolved;
no omitted platform/live-service acceptance is counted as passed.

Shared source is pinned to
[`3a0042936cd29fd794518a47ca121ff172f34046`](https://github.com/AberrantWolf/junk-libs/commit/3a0042936cd29fd794518a47ca121ff172f34046).
Trash Amp integration is
[`c2b1e74c86a3389574a62b2d3ce6ea1e74ec46da`](https://github.com/AberrantWolf/trash-amp/commit/c2b1e74c86a3389574a62b2d3ce6ea1e74ec46da).
Both are on their repositories' main branches; consumer dependencies use a
portable exact Git revision, with no local workspace paths.

## Limits

No real AcoustID key was available, so network identification quality against its
live catalog is untested. Provider responses in acceptance are deterministic
fixtures, not a successful-looking recognition mock in product code. All real
recognition depends on configured providers. Tests do not claim live catalog
coverage, physical-device/mobile audio acceptance, every codec, an exhaustive
release search or catalog verification. No deployment is part of this change.
