# Declared WAV segments with trailing bytes — 2026-11-02

User-approved policy following investigation of Steam's Sea of Thieves 2024 and
2026 soundtracks. Source implementation is locally qualified. No release,
deployment, automatic production retry or original-file repair is included.

## Investigation

Steam integrity checks passed for both editions (user reported). All five affected
same-title tracks shared across editions are byte-for-byte identical, independently
checked with SHA-256 and direct comparison. Seven affected 2024 files and five
2026 files have complete declared RIFF segments followed by additional bytes.
All twelve declared WAV frame counts match their corresponding decoded Steam MP3
frame counts. The two detailed vectors are Maiden Voyage (48 kHz, 24-bit stereo,
7,611,000 frames) and A Star To Sail By (44.1 kHz, 16-bit stereo, 8,224,085 frames).
FFmpeg 8.1.2 normal decoding uses exactly those frames. Ignoring length adds
0.842958 and 1.486599 seconds respectively, reaches approximately full scale after
near-silence, and reports two incomplete final bytes. Six-second ending correlation
against the MP3s exceeds 0.99996. This supports declared-boundary playback;
it does not establish the meaning of trailing bytes or verify a music catalog.
Local diagnostics: `/tmp/sea-of-thieves-playback/decode-report.json` and
`/tmp/sea-of-thieves-playback/all-malformed-mp3-durations.json`. Purchased audio
is not committed as a fixture.

## Implemented contract

Shared `junk-libs-pcm` accepts one complete internally checked RIFF segment within
the actual file. It counts bytes outside the segment without reading them. Chunk
headers/payloads/pads, format, PCM alignment, duration/resource limits and cooperative
cancellation stay checked. A suffix cannot complete malformed declared chunks.
All audio consumers share that bounded reader: identification, fingerprints,
preparation and independent FLAC sample comparison. Original storage/download and
whole-file source validation retain every byte, including the suffix.

Music records a per-track suffix count and emits an aggregate warning through
existing Import analysis limitations. Ordinary and retained-source processing
publish the warning; review/history uses the existing presentation. Existing
manifests without counts remain readable; no archived records are converted.
No MP3 fallback, sample inference, header rewriting or provider-specific exception
is introduced. This supersedes the pending source-repair policy noted in the
historical beta.17 record for these twelve files.

## Validation

The combined selected full source/browser check set passes on macOS: architecture
and documentation links, formatting, 610 Rust tests (five existing ignores), strict
workspace/all-target Clippy, web types/build/models, tooling/release-policy checks
and all 30 selected browser journeys. Real FLAC tests were enabled with
`COLLECTOR_TEST_FLAC`; Rust concurrency was bounded to four test threads.

The final source run passed every source check and the short browser smoke;
its first integration attempt found a synchronous ordering assertion racing the
independently loaded track list. The test now waits for the expected ordered rows
without changing the product. The remaining browser run passes integration and
the purchase/intake journeys. Its game-audio fixture initially received `1` where
it requires a FLAC executable path; the final five journeys pass with
`COLLECTOR_TEST_FLAC=/opt/homebrew/bin/flac`. No passed journeys were needlessly
repeated. Original failures remain retained rather than rewritten as passes.

Receipts/logs: `/tmp/wav-trailing-final-checks.*` (source checks and initial
browser result), `/tmp/wav-trailing-browser-checks.*` (purchase/integration and
remaining journeys up to the fixture setup failure), and
`/tmp/wav-trailing-browser-remainder.*` (final five journeys).
`/tmp/wav-trailing-combined-validation.json` maps their combined selected coverage;
it is local source evidence, not release-image qualification.

The updated real Chrome 156.0.8078.74 purchase journey verifies the warning after
reload, unchanged complete original-download SHA-256, real prepared-FLAC playback
and a 390-pixel mobile review without horizontal overflow. The mobile warning
screenshot was visually inspected:
`/var/folders/5w/77h8k79d3z7dftcv8mt3mgzm0000gp/T/collector-purchases-adEiyE/wav-trailing-warning-mobile.png`.
Only validation/status prose was finalized afterward; documentation links,
formatting and the entire diff were reviewed again.

The updated shared reader inspected all 65 mounted WAVs, reporting exactly 12
suffix warnings. Its entire decoded PCM output for both detailed tracks matches
normal FFmpeg output byte for byte; receipt:
`/tmp/sea-of-thieves-playback/collector-wav-audit.json`.
Initial focused WAV workflow tests pass (seven),
including real native FLAC preparation, unchanged original downloads, retained
analysis warnings/reindex and manual metadata authority. An initial test compile
failed on an incorrect IngestSummary field access; the test was corrected to
read the actual Import view and the focused rerun passed.

The first full selection stopped at an unrelated AccurateRip timeout/retry fixture
(two observed requests versus three); its isolated rerun passes. The continuation
with four Rust test threads passed that fixture but exposed a legacy real-FLAC
HTTP assertion expecting whole-payload integrity checking on GET. The existing
requested-byte contract explicitly removed that behavior. The test now checks
that same-length changed bytes are served, while retaining the missing-file
failure assertion. No media runtime behavior is changed for that correction.
Original failed reports/logs are retained in `/tmp/wav-trailing-full-checks.*`
and `/tmp/wav-trailing-continuation.*`; they are not counted as full passes.

The co-located PCM crate is not a dependency of the sibling Trash Amp checkout;
its external junk-libs pins and native decoder are unchanged. No sibling source
extraction or dependency pin change is required for this delivery.

## Review follow-up — 2026-11-02

The complete staged/unstaged/untracked candidate was reviewed for RIFF/chunk/pad
boundaries, bounded PCM/metadata traversal, preservation of the complete original,
source fingerprint validation, warning publication, retained retry and reindex.
No remaining actionable runtime findings were identified. Review strengthened
existing regressions rather than changing production behavior:

- A physically present required PCM padding byte outside the declared RIFF segment
  cannot repair that segment.
- A previously failed/preserved WAV with a suffix retries the same Job after
  restart, retains its warning through reindex, keeps manual metadata and complete
  original bytes, and prepares FLAC matching the clean declared PCM sample for sample.
- Changing only an uninterpreted suffix still fails background source validation;
  it cannot publish successful analysis or a derivative.

All 15 PCM tests and the two focused retained-retry tests (including both FLAC
and suffix-only corruption cases) pass with the real FLAC encoder. Affected
PCM/Store all-target strict Clippy, formatting and architecture/documentation checks
pass. Logs: `/tmp/wav-trailing-review-pcm-tests.log`,
`/tmp/wav-trailing-review-retry-tests.log` and
`/tmp/wav-trailing-review-clippy.log`. Production Rust and web code are unchanged
from the recorded complete source/browser qualification above, so its unchanged
consumer/browser evidence is reused. No release-image or production deployment
qualification is inferred from these local checks.
