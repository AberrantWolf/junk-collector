# Inbox audio compatibility — 2026-11-01

> Deployment identifiers are anonymized; dates, source identities, outcomes and qualification limits are unchanged. Example addresses are not deployment instructions.

User-requested source delivery, published in beta.17 and installed on application-host.
All selected local and exact-source release checks pass. See
[publication and deployment](container-beta17-publication-2026-11-01.md).

## Delivered behavior

- Ordinary integer PCM WAV accepts a 40-byte `fmt` chunk only when the empty
  extension and padding are entirely zero. Existing format/boundary checks remain.
- MP3 decoding incorporates the narrow Symphonia 0.5.5 empty-channel correction
  from [upstream PR #528](https://github.com/pdeljanov/Symphonia/pull/528).
- Bounded APEv2 suffix parsing supports a footer, optional matching header and
  optional final ID3v1, text metadata and JPEG/PNG front artwork. ID3v2 retains
  priority. Decoder reads, length and seeks exclude suffix metadata bytes.
- Failed identification can retry after preservation/restart through the existing
  retained-audio processing implementation. Ordered frozen fingerprints bind the
  Import to its archived Representation; normal byte validation and claim,
  lifecycle, grant and collection-binding fences apply. The same Job is retried,
  the failed attempt remains visible, and no second preservation Capture is made.
- Retry clears only an unstarted preparation error and delays downstream admission
  until identification commits. Originally requested automatic FLAC preparation
  then resumes. An admitted preparation batch retains its own error/retry state.

Mechanisms remain in shared libraries; Collector applies workflow and metadata
policy. The metadata-only port reads checked prefix/suffix tags with cancellation
without decoding the payload or claiming structural validity. The archived masters
and manual metadata authority are retained; no destructive conversion or automatic
production retries occur. APEv1, header-only prefix APE, float/multichannel WAV and
additional codecs are outside this delivery.

## Direct mounted-file qualification

No inbox audio was added to either repository or copied into a fixture directory.
Permanent regressions use synthetic headers/tags and existing small repository
vectors. Standalone diagnostics read mounted files into bounded process memory;
disposable ingest staging/archive copies exercise the real preservation workflow.

- All **547 MP3s** pass complete inspection with the final shared build:
  `/tmp/audio-inbox-payload-final-mp3-audit.json`.
- **736 of 748 WAVs** pass structural/metadata inspection:
  `/tmp/audio-inbox-wav-audit.json`. Twelve have samples beyond both declared RIFF
  and `data` boundaries and remain explicit failures: Soundtrack2024 tracks 02,
  05, 08, 14, 16, 17, 25; Soundtrack2026 tracks 02, 06, 10, 17, 29. These require a
  separate policy for contradictory source lengths. Originals remain unchanged.
- The mounted inventory also contains 758 FLACs; that codec path is unchanged and
  a complete new FLAC corpus inspection is not claimed.

Independent strict FFmpeg decode reports no errors for the three formerly failing
MP3 samples. Chained Echoes track 01 yields 7,496,064 frames in both readers; Elder
Scrolls Online track 01 yields 14,397,696. Talos Principle 2 track 01 yields
4,763,062 versus FFmpeg's 4,763,567. That 505-frame gapless-boundary difference is
recorded, not called sample-accurate agreement. Existing MP3 gapless qualification
limits remain applicable. Evidence: `/tmp/audio-inbox-independent-frames.json`.

## Failure-to-retry browser acceptance

The old executable reproduced actual failures for mounted AthenaAudioBook closing
credits, Chained Echoes 01, Elder Scrolls Online 01 and Talos Principle 2 01.
Chrome reopened the same disposable archive using the fixed executable and
**Review import → Retry this step** completed all four. Original download hashes
match the mounted inputs, preservation Capture counts stay unchanged, and real
playback succeeds. The WAV also prepared native-format FLAC with independent PCM
comparison. Initial four-file evidence: `/tmp/collector-audio-retry-jDwyI6`.

A fresh final-build run repeated persisted failures and retries for the WAV with
its original **Prepare FLAC after import** request and Talos with its APE cover.
Automatic preparation, exact original download hashes, playback, front artwork,
settled intents and reloaded 390-pixel mobile views without overflow all pass.
Both before/after commands exit successfully. Final evidence:
`/tmp/collector-audio-retry-GTAWAh` and `/tmp/collector-audio-final-state.json`.

## Automated and sibling validation

Final full Collector selection passes, recorded in
`/tmp/audio-compat-release-ready-checks.json` and its log: architecture, formatting,
600 workspace tests (five existing ignores), strict Clippy, web checks/build,
model tests, tooling/policy checks and all 30 browser journeys. Dependency/storage
scope uses full coverage; no narrower exemption is used. The receipt reports
`source_unchanged=false` because this delivery document and architecture prose
were finalized during the suite; executable/test sources remained unchanged.
Architecture/document links, formatting and the complete diff were checked again
after final documentation updates. This is source validation, not release-image
or deployed-system qualification.

Shared audio/music tests pass **44 cases**, plus the vendored empty-channel/
missing-bits regression. All-target/all-feature and WAV-only strict Clippy pass;
WAV-only tests pass. Synthetic coverage includes malformed APE boundaries,
cancellation, bounded metadata reads and actual MPEG frames embedded in binary
cover data. The latter previously inflated 10,368 source frames to 41,472; the
bounded adapter preserves frame count and late exact-seek PCM with/without an APE
header. The vendor was compared with registry 0.5.5 after identical rustfmt: only
the upstream comparison and synthetic regression differ semantically. MPL notices
are retained. APE coordinates were cross-checked against the
[original specification retained by TagLib](https://raw.githubusercontent.com/taglib/taglib/master/taglib/ape/ape-tag-format.txt)
and [FFmpeg's reader](https://www.ffmpeg.org/doxygen/trunk/apetag_8c_source.html).

An isolated Trash Amp checkout passes **42 amp-audio tests** and compiles
`amp-collector --lib` with aligned shared core/disc/audio. Installed native pins
and the user's sibling working tree are unchanged. No new GUI/device or broad
native transport qualification is claimed.

Earlier failed/interrupted checks are retained, not counted as passes. These
include the unbuffered network diagnostic deadline; browser setup navigation
races; a repeat browser run whose assertions passed but shutdown exceeded five
seconds and was reaped; mixed native crate identities; and an interrupted broader
native fixture build. Test corrections use Shared artwork visibility, distinguish
playable FLAC from preserved WAV, and drive periodic ingest reconciliation after
child jobs. A previous full run passed 600 Rust tests but stopped at an unrelated
false-premise progress assertion expecting accepted album identities from an
intentionally ambiguous two-release fixture. That proposal assertion was removed;
its meaningful 31 distinct reviewed entries and progress assertions remain, with
accepted grouping covered by dedicated journeys and Rust tests.

## Publication boundary

Collector pins shared audio/music commit
`438f195449622da2a2b9082c36cb06f66098813c`. It is retained on local junk-libs branch
`codex/audio-compatibility` and in Cargo's local Git cache. Other existing shared
pins and unrelated dependency versions are unchanged. The initial publication was
blocked by automatic review; the user then explicitly authorized merge, GitHub
publication, a beta update and application-host deployment. The shared branch is fast-forward
merged to main and both refs are published at that exact commit. No production
Imports are automatically retried.

## Clean review for beta.17

The fresh review traces parser bounds/flags/length arithmetic, optional APE header
agreement, tag precedence, MPEG read/seek limits and the narrow upstream backport.
Workflow review covers ordered source rebinding, preservation identities, claim/
authority/lifecycle fences, downstream admission, restart, and retry replay.
Coverage was extended to put a private companion before the audio file, retain its
exact download, and preserve a manual album title through retry and reindex.
All three strengthened restart/retry regressions pass (including real FLAC
preparation); receipts are `/tmp/audio-review-retry-tests.log` and
`/tmp/audio-review-package-retry.log`. A second review of the complete final
diff finds no remaining actionable correctness or preservation gaps. The named
format/timing limitations above remain explicit, rather than successful-looking
fallbacks. Exact-source release checks and image acceptance subsequently passed;
see the publication record above.

Release scope includes the previously local selector/popup commit `167d14b` since
beta.16. Its real intake/mobile/keyboard acceptance is included in full source
qualification. Container validation builds each architecture once and publication
must consume those exact validated bytes. No local debug executable is deployed.
