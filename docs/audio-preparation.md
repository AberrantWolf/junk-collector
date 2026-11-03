# Audio support and qualification

Reviewed against source `7f4a56c`, 2026-11-03. Implemented support and qualification are separate:
an accepted result covers the named input, tool and client, not every recording,
device or layout. The only remaining-work queue is [remaining work](remaining-work.md).

| Operation | Implemented / qualified scope | Limits and evidence |
| --- | --- | --- |
| FLAC/MP3 original purchases | Direct import, tags/artwork and playback; HL-02 named rates, timing, seek and web/native-client acceptance | [Digital-purchase evidence](validation/digital-purchases-2026-10-19.md); no broader gapless/fidelity claim |
| Integer PCM WAV originals | 16/24-bit mono/stereo, 8–192 kHz; metadata/artwork and direct PCM fingerprinting; explicit native-format FLAC preparation for listening | [WAV completion](validation/wav-digital-completion-2026-11-01.md); float/compressed, multichannel and RF64 unsupported |
| Complete declared WAV segment with trailing bytes | Preserves exact originals; consumes only the internally valid declared PCM and records a suffix warning | [Trailing-byte evidence](validation/wav-trailing-bytes-2026-11-02.md); truncation and invalid internal boundaries still fail |
| Supported CD range → FLAC | Explicit background preparation; real NAS libFLAC encoding and independent full integer PCM equality | Non-silent 3-second stereo CUE/BIN fixture, flac 1.4.2; [HL-06A evidence](validation/audio-oracles-2026-10-20.md) |
| Supported CD range → Opus 96/128/192 kbps | Configured explicit preparation, independent decode and Chrome download/play/seek, repeated after full restore | opus-tools 0.2 / libopus 1.3.1, known stereo signal, exact decoded frame count and bounded channel comparison; [HL-06A evidence](validation/audio-oracles-2026-10-20.md). Requested presets do not imply exact encoded average bitrate or lossless PCM |
| Preparation cancellation/failure/restart | NAS real-process cancellation, failure after output, retry and crash recovery | [HL-05C evidence](validation/nas-preparation-recovery-2026-10-20.md); fixture controls do not qualify power/storage failure |
| Opus original-file import | Not delivered as a supported digital-purchase path | Prepared Opus acceptance does not extend the FLAC/MP3 original-file scope |
| Prepared Opus on Trash Amp or other browsers/devices | Externally unqualified by HL-06A | HL-06A exercises Chrome on the Mac; device acceptance stays in HL-04/HL-08 |
| Whole-disc audio catalog verification | Live owned-disc AccurateRip qualification remains open | Encoding, PCM equality, structural checks and metadata identity do not establish a catalog match |

Configure the [external encoders](configuration.md#audio-encoders), then use the album's
music preparation controls. Keep the raw masters. Prepared derivatives can be
downloaded and used by the normal player according to client codec support.
Multi-track boundaries, gapless playback, bit-perfect device output and additional
layouts are not established by the single-track synthetic fixture.
