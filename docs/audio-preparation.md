# Audio support and qualification

Updated 2026-11-07. Implemented support and qualification are separate:
an accepted result covers the named input, tool and client, not every recording,
device or layout. The only remaining-work queue is [remaining work](remaining-work.md).

| Operation | Implemented / qualified scope | Limits and evidence |
| --- | --- | --- |
| FLAC/MP3 original purchases | Direct import, tags/artwork and playback; HL-02 named rates, timing, seek and web/native-client acceptance | [Digital-purchase evidence](validation/digital-purchases-2026-10-19.md); no broader gapless/fidelity claim |
| Supported CD range → FLAC | Explicit background preparation; real NAS libFLAC encoding and independent full integer PCM equality | Non-silent 3-second stereo CUE/BIN fixture, flac 1.4.2; [HL-06A evidence](validation/audio-oracles-2026-10-20.md) |
| Supported CD range → Opus 96/128/192 kbps | Configured explicit preparation, independent decode and Chrome download/play/seek, repeated after full restore | opus-tools 0.2 / libopus 1.3.1, known stereo signal, exact decoded frame count and bounded channel comparison; [HL-06A evidence](validation/audio-oracles-2026-10-20.md). Requested presets do not imply exact encoded average bitrate or lossless PCM |
| FLAC, MP3 and PCM WAV originals → Opus 96/128/192 kbps | Album preparation and optional import preparation of analyzed mono/stereo files | Original files stay in the archive. Digital audio is decoded with the shared readers, adapted to 44.1 kHz stereo PCM, and encoded with the existing Opus adapter. |
| Preparation cancellation/failure/restart | NAS real-process cancellation, failure after output, retry and crash recovery | [HL-05C evidence](validation/nas-preparation-recovery-2026-10-20.md); fixture controls do not qualify power/storage failure |
| Opus original-file import | Not delivered as a supported digital-purchase path | Prepared Opus acceptance does not extend the FLAC/MP3 original-file scope |
| Prepared Opus on Trash Amp or other browsers/devices | Externally unqualified by HL-06A | HL-06A exercises Chrome on the Mac; device acceptance stays in HL-04/HL-08 |
| Whole-disc audio catalog verification | Live owned-disc AccurateRip qualification remains open | Encoding, PCM equality, structural checks and metadata identity do not establish a catalog match |

## Prepare one album or a batch

Configure the [external encoders](configuration.md#audio-encoders). In **Prepare
audio**, choose **FLAC** or **Opus** and, for Opus, choose 96, 128 or 192 kbps
VBR. **Albums needing Opus** uses the selected bitrate. An album with a ready
96 kbps copy still appears when you ask for 128 kbps.

Open an album to choose its sources and prepare missing tracks, or select
albums and choose **Start preparation**. **Select all matching results**
includes albums on later pages, up to 10,000 per batch. Uncheck an album to
exclude it. If a disc has alternate captures, open the album and choose the
original to use before starting. The batch saves the selected albums and
source choices immediately. Track counts appear as albums are planned in the
background. You can leave the page and follow, cancel or retry preparation in
**Activity**. Retrying a batch returns failed albums to the background queue.
Opening an album from a batch keeps that batch's preparation profile. Albums
that need a source choice or whose inventory was edited after submission are
flagged for review. The list updates as imports and jobs
change. While the server is rebuilding summaries, results
filtered by readiness may be incomplete. An album with incomplete recorded
track coverage stays in the filter, and its row counts only known tracks.

## Prepare Opus after import

In **Administration → Library settings**, select the music collection. Under
**Opus on import**, choose a scope and bitrate and save the defaults. **CD and
digital album imports** includes digital-only FLAC, MP3 and supported PCM WAV
albums. Review **Audio preparation after import** before submitting an import
to keep the displayed collection default, select another preset, or skip
preparation. The submitted review keeps its displayed default even if settings
change before admission. Imports save the exact originals first and show the preparation outcome
separately. A missing encoder, unsupported source or failed conversion can leave
the originals saved and the preparation step needing attention.

Prepared audio can be downloaded and used by the normal player according to
client codec support. Opus uses lossy encoding. Multi-track boundaries, gapless
playback and bit-perfect device output require their own qualification.
