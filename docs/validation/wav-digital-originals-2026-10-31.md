# WAV digital originals — 2026-10-31

User-requested source delivery. No release, push or deployment is performed.

## Contract and delivery

Music file analysis recognizes `.wav` case-insensitively and uses strict shared
RIFF parsing in the reusable `junk-libs-pcm` workspace crate. Supported inputs:
16/24-bit integer PCM, mono/stereo, 8–192 kHz, RIFF or checked extensible PCM;
4,096 chunks and 24 hours per file. File/payload limits remain intake policy.
Reject inconsistent RIFF/chunk/format/data lengths, duplicate format/data chunks,
partial PCM frames, unsupported formats and truncation. Metadata chunks are skipped
without allocating their advertised payloads. Float/compressed/multichannel WAV,
RF64 and WAV tag/fingerprint interpretation remain unimplemented.

Exact originals, purchase facts, per-file visibility, manual metadata and sidecar
artwork use existing digital ownership and publication. WAVs get no physical copy,
CD identity or catalog verification claim. Explicit intake/later FLAC preparation
uses existing durable track/album Jobs and native-format sealed recipes; mixed
MP3/FLAC tracks remain direct originals. The existing process lifecycle, cancellation,
bounded streams, source revalidation and journal publication apply to WAV. Independent
Claxon decoding compares every generated FLAC integer sample against original PCM.
Availability and admission use recorded observations without full-payload hashing.
Prepared listening uses FLAC and original downloads still return exact WAV bytes.
Previously saved unanalyzed WAVs require fresh analysis, such as reviewed reimport.

The Python standard-library `wave` fixture `03.wav` is 24-bit mono 96-kHz PCM,
one second long, with independently specified signed extrema (-8388608, -1, 0,
1, 8388607) followed by a 440-Hz sine. Existing independent CD/FLAC/MP3 vectors
remain in use. The shared reader tests also cover the smallest supported mono
24-bit/stereo 16-bit files, extensible PCM, every truncation boundary, inconsistent
format fields, duplicate data, partial frames, bounded chunks and interrupted parsing.

## Validation

Focused shared-reader tests passed (5 tests), including refusal of a second
waveform layout hidden in a LIST chunk. Real configured FLAC integration
passed mixed WAV/FLAC/MP3 admission, no physical copies, exact original downloads,
24-bit/96-kHz mono/stereo preparation at distinct file positions, independent
sample equality, command replay,
no MP3/FLAC conversions, album playback and reindex. No-encoder/private-viewer and
malformed-source preservation tests passed. Cancelled preparation and changed
source bytes cannot call the encoder or publish a derivative (3 integration tests
total). Prepared files inherit the WAV source file’s visibility; a direct media
URL cannot bypass private-source access.

An isolated archive of Trash Amp `2204ec83305457a92704bafcb4a823995266831b`
was tested with local Collector source patches, without modifying the sibling
checkout: 9 unit and 4 real-server tests passed. The HTTPS helper is invoked by
its parent journey; the host audio-device callback test remains ignored/unrun.
Existing direct FLAC/MP3 streaming, seek, downloads and offline reopening pass.

The change selector was previewed against staged, unstaged and relevant untracked
paths and selected infrastructure/workspace coverage. The selected source checks
passed: architecture/document links/fixture manifests, formatting, web type check
and build, testing/container/release/mail tooling, workspace tests (580 passed,
5 existing ignores), strict workspace/all-target Clippy and web model tests.
The latest strict-reader cases (5 tests) and expanded mixed-file/failure integration
cases (3 tests) also passed after their final additions. The workspace ignores are
the owned-disc oracle, the separately invoked native-reader helper, two generated
scale measurements and a manual encoder concurrency benchmark; they are not newly
qualified here.

All 30 selected browser journeys passed, including actual Chrome 156.0.8078.74
WAV intake with **Prepare FLAC after import**, later preparation through the Music
editor, native-format availability, real playback, reload and mobile overflow.
Existing FLAC/MP3 direct playback, playlists, retained metadata/artwork, source
revocation, public/private routing, recovery and other feature journeys passed.
WAV screenshot evidence is in the disposable fixture directory
`collector-purchases-mw6aQQ` under the host temporary directory.

Commands and corrections:

- `COLLECTOR_TEST_FLAC=1 python3 scripts/check_changes.py --classification infra --report /tmp/wav-final-checks.json`
  passed every selected source check and the first 25 browser journeys. The
  game-audio fixture then refused `1` as an executable path: Rust checks use this
  variable as an enable flag, while that browser fixture expects the tool path.
  The selector receipt honestly retains its failed overall status.
- `COLLECTOR_TEST_FLAC=/opt/homebrew/bin/flac node web/tests/run-suite.mjs game-audio documents backup-reminder remote-media storage`
  passed all five remaining journeys. This completes the originally selected
  browser set; logs remain in `/tmp/wav-browser-recovery.log`.
- The real-FLAC HTTP restart test initially exposed an existing fixture setup bug:
  its restarted router omitted the original fixture's direct-media listener flag.
  Restoring that flag preserves the meaningful historical-read assertion; both
  the focused rerun and the full workspace run passed. Production listener policy
  is unchanged.
- `python3 scripts/api_contract.py --write` regenerated unchanged API contracts;
  the neutral internal audio selection and frozen visibility fields add no API
  schema drift. Final architecture/link, format and whitespace checks passed.

These are local working-tree results, not a committed release qualification receipt.
Native Trash Amp GUI/physical-device qualification, broader codec forms, generated
scale measurements and deployment were not performed. Existing dated acceptance
records remain unchanged.
