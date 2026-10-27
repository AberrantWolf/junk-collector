# Automatic AccurateRip and operational defaults — 2026-10-27

## Delivered scope

One application workflow handles each accepted CD request: current recorded source
fingerprints and source membership, missing structure inspection, shared bounded
provider acquisition and the existing qualified audio operation. The browser
submits one command. Evidence keeps exact source/reference/algorithm/coverage
coordinates; only a Verified result confirms audio. Manual metadata and originals
retain their existing authority and bytes. Technical processing history remains
available independently of the one-click action.

The album disc view and advanced check view expose the action and show durable
progress, cancellation and results after reopen. Per-music-collection settings in
Administration enable automatic checking during future ingests and configure the
maximum offset. The existing library batch engine runs unconfirmed originals with
bounded rolling admission, cancellation, recovery and individual review outcomes.
Confirmed originals and digital purchases are skipped; unsupported originals do
not gain a successful-looking result. Settings are private operational state;
collection ownership controls edits and curator access controls checks. Their
backing APIs use the curation router's exposure policy. Worker settings require
server administration and use the administration router.

Worker processing/copy limits apply to new claims immediately. Running jobs finish
when limits shrink. Configuration provides defaults until the UI saves a durable
override. Ingest snapshots its CD settings at admission; existing unfinished
intents have null settings and retain their previous behavior. No automatic
conversion of old workflows or pruning of collection evidence occurs.

## Validation

Selection was previewed with `python3 scripts/check_changes.py --dry-run` over
staged, unstaged and untracked changes. Storage/contracts select workspace tests
and Clippy; focused browser acceptance covers the actual affected workflows.
The working tree already contained separate artwork/import-progress/capture work;
those changes were retained. This is a delivery record, not a release
qualification receipt.

- Focused backend checks cover settings authorization/revisions/receipts, live
  claim limits, restart/reindex persistence, ingest-frozen defaults, matching and
  no-data results, overlapping-request refusal, cancellation, provider failure
  and retry, missing structure, multiple eligible layout sidecars with a recorded
  import choice, missing bytes and batch review outcomes.
- One-click browser fixture passes independent disc submission, server-driven
  provider waiting, reopen polling without another request, confirmed result,
  cancellation and mobile width. Inspected its rendered 390px screenshot.
- Actual local-server Chrome acceptance passes saved worker/CD settings, reopening,
  immediate backend effects, mobile wrapping, and the library batch from an older
  preservation-only original through structure/acquisition/audio verification to
  a refreshed Verified library row. The new `cd-library` journey retains this test.
- The broad integration journey passes actual audio/seek, two concurrent encodes,
  shared provider acquisition, separate-request cancellation, reopen, metadata,
  playlists and recovery with the new one-click controls.
- Web type checking/build and queue/context/presentation model checks pass.
- Workspace Clippy with all targets and warnings denied passes, as do architecture,
  format, documentation-link and generated API contract checks. Testing/packaging
  policy checks pass (their credential-dependent case remains skipped).
- All change-selected browser journeys pass in Chrome 156.0.8078.48 across the
  recorded suite batches, including the complete integration, CD-library,
  purchases, inbox/recovery, authorization and media journeys. Final mobile
  Administration navigation and CD checkbox alignment were inspected at 390px.
- `cargo test --workspace --locked` exits successfully. The archive suite passes
  283 tests with zero failures. Explicitly ignored workspace cases remain the
  configured user-owned disc oracle, two opt-in scale runs, a manual performance
  benchmark and a subprocess reader helper (exercised by its isolation test).
  Optional physical-disc and scale qualification was not performed.

## Review repairs

The independent constant-PCM fixture deliberately uses a zero-offset bound for a
unique matching result. Widening that bound can match multiple offsets and must
remain unconfirmed. Four older preservation/metadata/encoder fixtures now explicitly
disable automatic AccurateRip: their tiny synthetic discs have no AR provider and
the tests exercise separate optional workflows. Their meaningful assertions remain.
The new complete CD fixture separately checks the enabled ingest default.
The broader digital-purchase browser regression caught file-audio packages without
purchase facts incorrectly inheriting CD defaults; admission now requires a
module-declared selected CD anchor, independently of acquisition facts.

Multi-layout packages reuse the imported/inspected anchor and freeze its archived
layout observation, so a later inspection cannot silently change a queued query.
The older acquisition command shares the same strict application observation mapper.
The inbox-cleanup browser regression now compares its import/work state with the
state before cleanup, preserving independent AccurateRip review instead of requiring
an unconditional success label. Its AccurateRip responses are explicitly offline
404 fixtures, separate from its metadata provider responses.

The initial `fx06` browser assertions passed but Chrome shutdown exceeded the
harness's five-second bound; that run remains an infrastructure failure. A separate
retry passed the complete journey and teardown. No harness limit was weakened.

## Limits

Source-only delivery. No release/deployment or batch against the live NAS library
was performed. Provider/network/physical-disc qualification remains separate from
the offline fixtures and actual local browser workflows. Running an explicit
batch does not schedule indefinite retries of absent provider data. Only music CD
settings are implemented; Games and future video modules retain their own scope.
