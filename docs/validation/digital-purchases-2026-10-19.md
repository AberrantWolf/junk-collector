# HL-02 qualification — 2026-10-19

Implementation: [supported matrix and ownership](../digital-purchases.md).
Queue: [HL-02](../remaining-work.md#hl-02--complete-flac-and-mp3-digital-purchase-support).
All installations, accounts, certificates, audio and output profiles below are
temporary test fixtures. No live archive, server, grant, credential or listening
profile was changed. Pre-existing release work in Trash Amp was retained.

## Evidence

- `junk-libs-audio/tests/purchases.rs`: independent FLAC PCM vectors cover
  16/24-bit × mono/stereo × 44.1/48/88.2/96/176.4/192 kHz, exact decoded lengths
  and sought samples against the independent PCM formula. Independently encoded
  MPEG-1/2/2.5 mono/stereo CBR/VBR/no-Xing files cover actual decode/seek, short streams,
  known encoder trims and Unicode tags. The Collector/native stereo MP3 fixture
  additionally exercises the actual clients. Hand-built ID3v1/v2.2/v2.3/v2.4 envelopes exercise
  text encoding, whole-tag and frame unsynchronization, PIC/APIC and invalid inner
  lengths/counts; FLAC picture/comment allocations are bounded before decoding.
  Truncated frames and cooperative cancellation cannot report inspection success.
  Compressed/encrypted ID3 frames fail explicitly. No MP3 gapless-output or
  exhaustive variant qualification is inferred from decode success.
- Collector archive/service tests exercise tagged mixed FLAC/MP3 import with no
  encoder/provider, private purchases and companions, exact original bytes,
  native-rate availability, Library title, song search, repeated album playlist
  occurrences, manual track titles/disc/order edits, immutable saved source
  targets, receipt replay, reindex and restart. Full backup and archive-only
  recovery retain purchase facts, file visibility, originals, playlists and
  authority; archive-only access is explicitly recovered for a new account.
- Additional purchase tests cover MP3 singles, multi-disc native ordering,
  disjoint nested-folder discovery, conflicting alternate encodings and explicit
  album selection, embedded cover Asset publication, unsupported companions,
  receipt-photo defaults, reviewed booklet sharing and private audio tag/art
  non-disclosure. Malformed or misleading audio is preserved with failed analysis
  visible and no ready audio claim. Existing lifecycle, cancellation/shared-owner,
  publication-fault, replacement, unknown-envelope, cross-collection and backup
  regression suites exercise the reused policies. Historical media reads retain
  file-level authorization instead of requiring the original to remain current.
- `npm --prefix web run test:purchases`: Chrome **153.0.8010.37**, real isolated
  HTTPS Collector. Actual setup/collection creation, purchase/price and companion
  review, interrupted file transfer/reload/reselection/resume, configured inbox,
  publication, immediate album/artwork, FLAC **and MP3** media playback, seek,
  queue/playlist and paused reload, manual track/purchase edits, and 390px mobile
  layout and failed-source guidance without CD preparation prompts passed. The final acceptance artifacts are under the temporary directory
  printed by the test (`collector-purchases-OefMO9` for the final failed-source UI regression).
- Full real-Chrome smoke passed: existing CD playback start/pause/resume/seek,
  next/previous/natural advance, grant revocation, interrupted folder intake,
  grouping, preparation, collection lists/repeats/conflicts/receipt recovery,
  concurrent provider work, role denial, WebAuthn and mobile workflows.
- Trash Amp `real_server` tests use actual TLS/authenticated Collector servers,
  native-rate FLAC/MP3 decode and seek, separate files/download identities in one
  Representation, retained artwork/text, credential removal/server shutdown and
  offline adapter reopen/decode/seek. Existing two-server/queue/list/download and
  Slint shell regressions remain in the native workspace suite.
- The separately invoked ignored test
  `purchased_originals_reach_the_real_audio_output_callback` passed on this Mac's
  actual default CoreAudio output, through Trash Amp's normal decoder/resampler/
  ring buffer/device callback. Each format opened a fresh Player, preventing the
  previous track's counters from satisfying the next assertion. Measured FLAC
  callback peak **0.088371746**, MP3 **0.08895567**, both beyond **0.0213125 s**.
  This proves device callback consumption; it is not a subjective listening or
  sample-accurate gapless certification.

## Gates

- Shared libraries: `cargo test --workspace --all-features --locked --offline`,
  strict all-target/all-feature Clippy and formatting passed: **274 passed,
  7 existing qualification tests ignored**. The MPEG matrix was then expanded
  to both channels for each version/mode and the full gates passed again.
- Collector: architecture/document/fixture checks, workspace tests, strict
  all-target Clippy, formatting, TypeScript check/build, generated API contract,
  server rebuild and real Chrome smoke/purchase journeys passed for this delivery.
  Final workspace run: **427 passed, 4 existing qualification tests ignored**.
  After the final same-codec alternative and error-guidance changes, focused
  alternative/malformed-source/client tests and strict Clippy passed again;
  TypeScript/build and the embedded Rust server were rebuilt. Generated contracts
  matched. The purchase browser journey was rerun for those final UI changes.
- Trash Amp: workspace tests, strict all-target Clippy and formatting passed;
  the explicit real-device test passed separately. Final native workspace run:
  **53 passed, 2 ignored** (the real-device check was explicitly invoked separately).
  Final strict Clippy and formatting passed after coordinated consumer changes.

## Actual Trash Amp GUI acceptance

Passed on this Mac, 2026-10-19, using the actual Slint/Skia application, a fresh
isolated profile and real TLS Collector fixture. The `amp-collector` `ui_fixture`
example accepts an optional media directory. This run used independently generated
30-second 96-kHz stereo 24-bit FLAC and 44.1-kHz stereo VBR MP3 sine fixtures plus
a 256×256 sidecar cover. Native enrollment used the fixture's explicit public
trust bundle and temporary test owner approval; no browser warning was bypassed.

- Paired and immediately browsed the purchased album, titles, artist and cover.
  Both originals appeared as 30-second Streamable tracks. Played FLAC, sought to
  12 seconds through the scrubber, advanced with Next to MP3 and sought to 15 seconds.
- Added both tracks to Up next, verified readable title/artist labels, and explicitly
  downloaded the album. Both original tracks became Downloaded.
- Removed the temporary connection through Settings. The UI immediately reported
  “Connection removed; completed downloads retained” and remained responsive.
- Quit the app, stopped the fixture server and confirmed its socket was closed.
  Reopened the same profile with no server/trust command-line arguments: the
  source was offline, playback was paused with Nothing playing, and cached album
  text/cover and both Downloaded tracks remained available.
- Played FLAC offline, sought to 12 seconds, advanced to MP3 and sought to 15 seconds.
  Offline search for MP3 original returned the retained title, album, artist and
  30-second duration. [Actual offline playback screenshot](hl02-native-offline-2026-10-19.png)
  records the disconnected source, artwork, both retained originals and MP3 playback.
  Closed the temporary app after acceptance; the temporary server stayed stopped.

The initial GUI attempts were blocked by the locked Mac; the user unlocked it.
The first completed run exposed opaque source IDs in artist/queue captions and a
main-thread Keychain removal stall after replacing the enrolled executable. Captions
now show descriptive artist text, and credential removal runs on a worker with a
visible pending state, keeping the UI responsive even when the OS credential store
blocks. The final run above used an unchanged executable identity throughout
enrollment/removal/reopen. Full native workspace tests (**53 passed, 2 ignored**),
strict all-target Clippy, formatting and the actual app build passed after these
repairs. Earlier locked-GUI and removal failures are resolved by this repeat.

## Delivery and limits

HL-02 is delivered for the documented FLAC/MP3 matrix. This actual GUI evidence
supplements the independent vectors and real device callback check; it does not
claim subjective fidelity, sample-accurate gaplessness, second-device LAN,
output-device loss or broader HL-04 qualification.

The initial qualification used local sibling Cargo patches. The user subsequently
authorized coordinated commits/pushes and exact revision pins. The publication
follow-up below records those checks; live installations remain unchanged. No new codec beyond initial FLAC/MP3, DRM support, image splitting,
live storefront acquisition or MusicBrainz/CAA network qualification is claimed.


## Pinned-revision publication — 2026-10-19

The user authorized publishing HL-02 across all three repositories. The shared
reader was committed and pushed first; Collector then replaced every junk-libs
path patch with the same exact Git revision, `2c8eaf8fd5a7f3e5f4873d3463b8008039e9af98`.
Trash Amp pins that shared revision and the pushed Collector revision. Dedicated
`codex/hl02-digital-purchases` branches preserve unrelated local release/research
work. Tags and existing remote branches are not rewritten.

Publication checks are rerun against the Git-backed dependency graph. The final
results and clean-source checks are recorded after completion below.
