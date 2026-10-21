# Game-disc CD audio acceptance — 2026-10-21

Post-alpha.3 source work; alpha.3 images and production are unchanged. Standard
BINARY CUE/BIN game packages reuse the audio preparation, availability, media,
playlist and native cache paths. [Delivery contract](../game-disc-audio.md).

## Shared mechanisms and owned references

Shared extraction is published at junk-libs `3490c96985030ffaac1b116ac3207ec428d34d12`
with signed API/acceptance record `993afc932c6b2acb5cf02c95d3cb8a387d9a70a5`
on `codex/game-disc-audio`. Collector pins all seven affected workspace dependencies
to the signed record revision; the only subsequent change is its playback documentation. No local Cargo patch is part of the delivery. The preceding
shared revision also updates an unrelated PDF parser dependency; Collector's disc
and audio dependency graphs do not use that parser.

The affected disc/audio crates passed 117 tests, with six existing opt-in tests
ignored, plus strict all-target Clippy and formatting. Independent generated
vectors check signed PCM channels, mixed data/audio boundaries, stored and
synthetic gaps, smallest sectors, changed lengths, truncation, bounded reads,
range concatenation and forward/backward seeks. CUE size and file/track counts
are bounded before parsing/allocation. Unknown playback directives and duplicate
relative aliases fail closed. Small fixed-array iteration repairs satisfy Rust
1.99's stricter Clippy without suppressing checks.

The operator explicitly authorized read-only owned references under `~/dumps`.
A private independent oracle compared every decoded frame against direct original
BIN ranges, using each AUDIO file after its declared two-second INDEX 00 and the
following AUDIO file's stored pregap. It never decoded the data file:

| Reference | Audio tracks | Exact stereo frames |
| --- | --- | --- |
| PS1 test, 2026-07-23 package | 2, 3, 4 | 12,705,504 |
| Mixed-mode disk7, 2026-08-02 package | 2 | 10,838,016 |

End-of-track and backward/forward seeks passed too. Originals were unchanged;
no copyrighted payload is committed. This qualifies extraction for these layouts,
not catalog identity, exhaustive format support or whole-disc audio verification.

## Collector and actual browser workflow

The integration regression imports a mixed-mode game, records only AUDIO tracks
2/3, keeps the Games identity, creates separate inventory and audio lists, retains
source-specific title/printed-number edits, prepares audio through durable jobs,
reindexes and resolves playback/search against the same original. Catalog reads
and batch admission are also exercised with the original payload temporarily
offline; background workers remain responsible for actual-byte validation.

Actual Chrome 155 against an isolated generated-media installation passed setup,
Games collection creation, original import, immediate track editing, real FLAC
preparation, title persistence after reload, playback and seeking, audio-only
playlist expansion, source-bound search and mobile overflow checks. The new browser
journey is `web/tests/game-audio.mjs`; setting `COLLECTOR_GAME_CUE` selects a private
owned CUE/BIN reference instead. Large owned imports use a longer bounded deadline.
The first owned attempt exceeded the small-fixture 60-second deadline during
preservation; it did not report success. The subsequent owned PS1 run passed the same full browser journey, including
all three audio tracks, real FLAC preparation, playback/seek, title edit/reload,
playlist expansion and source-bound search. The dump originals remained unchanged.

## Native client

An isolated copy of Trash Amp preserves the operator's staged native release work.
Its four adapter unit tests and three actual-server tests pass (one existing
physical-output callback test ignored), including actual TLS
refresh, streaming/seek, explicit download, server shutdown, credential removal
and offline reopen. A mixed-mode Game fixture exposes track 2 only; its title,
artwork and audio survive offline reopen. The existing real audio-output callback
acceptance remains opt-in and was not rerun for this adapter change. Publishing
and final dependency-pin validation are recorded below when complete.

## Review and limits

Self-review covers recorded source fingerprints, source-specific search cursors,
recipe/geometry consistency, collection role/exposure checks, manual authority,
missing-module historical playback, background cancellation and unchanged raw
masters. Review repaired a hidden game track editor, bound saved list-creation
receipts to their inventory/audio kind, and rejected caller-selected music geometry
for a Games preparation request. Invalid source geometry cannot publish audio.

CHD, proprietary game sound formats and mixed-mode redumper timelines remain
unsupported here. Multiple alternate originals still require an explicit source
choice. This is not a claim about a newly tested physical audio device or public
production deployment. Existing FLAC/Opus process failure/recovery, authorization,
Safari and NAS release evidence applies to the reused mechanisms.

## Final Collector source gates

All 458 workspace tests passed, with four existing opt-in tests ignored. Strict
all-target Clippy, formatting, architecture/document/fixture checks, generated API
contract checks and web type/build checks passed. The embedded-web server is
rebuilt after the asset changes. Subsequent focused checks cover the final game
catalog visibility guard and the complete import/preparation/reindex regression.

The offline-master regression admits work from recorded geometry, observes both
workers fail without publishing derivatives while source bytes are absent, restores
the same immutable original and retries the same batch successfully. Prepared list
entries remain playable with the module missing; unprepared entries retain the
Module unavailable diagnosis. Both this case and existing grouping/undo coverage
pass. Duplicate/conflicting audio result envelopes fail validation. Ordinary
non-CD game entries do not acquire empty music controls or preparation work.

The focused new browser journey, owned PS1 run and final complete Chrome 155 smoke
passed against served embedded assets. The smoke also covers setup/WebAuthn,
provider concurrency/cancellation, bulk library recovery, music playback/revocation
and playlist receipt/conflict recovery. No production container or data was changed. The native
consumer's published-pin commit is a separate dependency follow-up to this source
commit, with its evidence above retained rather than claimed prematurely.
