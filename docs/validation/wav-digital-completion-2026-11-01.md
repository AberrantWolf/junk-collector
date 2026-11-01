# WAV digital-release completion — 2026-11-01

The four support-review remedies are now published in beta.15 and installed on Fi;
see [release/deployment evidence](container-beta15-publication-2026-11-01.md).
The implementation validation below records its initial source-only scope.
The [initial delivery](wav-digital-originals-2026-10-31.md)
retains its dated validation; [current workflow](../digital-purchases.md) describes
supported intake and controls.

## Delivered contracts

- Shared `junk-libs-pcm` reads RIFF INFO and ID3v2.2/2.3/2.4 text and embedded
  covers into the existing file metadata vocabulary. Trailing INFO/CSET, common
  code pages, ID3 unsynchronization and legacy PIC are supported. Metadata is
  bounded to 32 MiB, 256 entries/frames, 4 KiB normalized text and bounded pictures;
  malformed/unsupported structural input fails closed. Masters remain exact.
- WAV PCM feeds the existing Chromaprint envelope directly, without a FLAC encoder
  or a prepared derivative. Album exclusion is respected; existing discovery,
  provider retry, reviewed mapping and manual-conflict handling remain shared.
  Fingerprint candidates are identity evidence, never catalog verification.
- **Analyze retained digital audio** opens ordinary processing, with the explicit
  **Analyze retained audio** action. A durable Job commits the frozen source-bound
  observation before enrichment. Replay retries enrichment, with claim/generation
  fencing and the exact recorded manifest. Existing Captures, physical inventory,
  originals and manual metadata survive. Accepted artwork is reused; deleted
  matching artwork is not implicitly restored.
- Prepared Music FLAC downloads carry current accepted title/album/artist,
  album artist, disc/track coordinates, effective release ID and available cover
  preview, with descriptive UTF-8 filenames. Shared bounded metadata repacking
  preserves STREAMINFO/audio frames. Source digest and artwork length/digest are
  checked, access and Item state are checked again, and output has its own digest.
  Archived originals, listening media, recipes and cache identities are unchanged.
  Temporary scratch is removed with its reader. The server admits two simultaneous
  exports, holding permits through blocking work and response lifetime; cancellation
  checkpoints and an 8 GiB output bound prevent unbounded work. Normal media access
  policy applies; output is private/no-store. MP3 is never converted.

## Acceptance and validation

Focused checks passed before the broad regression run:

- Shared reader/metadata/fingerprint/export tests: 12 tests, including exact integer
  vectors, trailing code pages, legacy PIC/unsync, malformed metadata, cancellation,
  and independent FLAC/WAV fingerprint equality using the installed FLAC tool.
- Three `collector-store` `wav_completion` workflows: tagged intake/artwork,
  independently decoded FLAC export with accepted manual Unicode metadata,
  preserve-only legacy analysis/replay/manual survival/reindex/artwork reuse, and
  full WAV matching/provider failure/retry/restart/review/apply conflict workflow.
- Strict workspace Clippy and web type checking/build passed during implementation.
- Real Chrome digital-purchases and digital-album-matching journeys passed. They
  exercise the real import, preparation, export download, external `metaflac`
  inspection, retained-source Job and refreshed metadata controls. Matching runs
  without server encoders. Mobile screenshot inspected: title/artwork/track/actions
  wrap within the viewport. Artifact: temporary `collector-purchases-rxHC0y` fixture,
  `wav-metadata-mobile.png`.

Earlier focused browser attempts exposed incorrect fixture expectations for image
alt text and a transient generic action. The fixture expectation was corrected and
digital analysis received a stable explicit action; both journeys then passed.
A later concurrency-test compile error (`unwrap` on a required representation ID)
was corrected before rerunning acceptance. No failed launch/assertion counts as a pass.

The broad change-based run passed 591 workspace tests (5 explicitly ignored),
strict workspace Clippy, architecture/link/fixture checks, formatting, web types/
build/models and the maintained tooling tests. Its browser integration detected a
saved FLAC export URL failing after the owning module became unavailable. The
export now falls back to the authorized historical media bytes when its module or
schema cannot execute, honoring ordinary transfer integrity checks. A focused
store assertion covers this fallback; the existing browser integration retains its
restart/missing-module download and playback assertions. Follow-up: all three WAV completion archive workflows, the HTTP preparation/range/
revocation/restart/export-concurrency test, and strict workspace Clippy passed.
The server/media executables were rebuilt, and all 30 named browser journeys then
passed, including integration's missing-module historical download/playback,
WAV matching without encoders and WAV purchase/export/retained-analysis acceptance.
Architecture/link/fixture, formatting and diff whitespace checks passed again.
The complete workspace test set was not repeated after this narrow historical-read
fix; the relevant follow-up checks above validate that final change.

The isolated Trash Amp `amp-collector` consumer check passed 9 unit and 4 real-server
tests against these workspace dependencies, with 2 explicit ignores (one nested
fixture and physical audio output). Its existing sibling warnings are unchanged.
Temporary source/build copies were removed. OpenAPI and generated TypeScript
contracts match the annotated routes and DTOs.

## Boundaries

Supported audio remains integer PCM RIFF/checked extensible WAV, mono/stereo,
16/24-bit, 8–192 kHz, at most 24 hours/file and 4,096 chunks. Float/compressed WAV,
RF64, multichannel and continuous-image/CUE splitting remain unimplemented codec/
layout support; they are not inherent WAV limitations. They are outside these four
remedies. No live provider-service, physical audio-output or new native GUI
qualification is claimed. Provider fixtures are offline; native consumer checks
exercise the existing protocol rather than a new native curation interface.

Local evidence: `/tmp/wav-completion-checks.json` records the initial broad run's
browser failure, rather than falsely recording it as green. Logs are retained at
`/tmp/wav-completion-checks.log`, `/tmp/wav-completion-followup.log`,
`/tmp/wav-completion-http-final.log`, `/tmp/wav-completion-browser-full.log`,
`/tmp/wav-completion-native.log` and `/tmp/wav-completion-api.log`.
The final purchase screenshots are in the temporary
`collector-purchases-Vllk2l` artifact directory. No release qualification, deployment,
physical audio-device or live provider-service acceptance is inferred from these
local fixtures.
