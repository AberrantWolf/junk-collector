# Audio support and preparation

## Supported audio

| Input or operation | Support | Limits |
| --- | --- | --- |
| FLAC and MP3 originals | Import, tags and artwork, playback | No gapless or fidelity claims beyond the tested rates |
| Integer PCM WAV originals | 16/24-bit mono or stereo, 8–192 kHz; tags, artwork, fingerprinting, FLAC preparation | Compressed, multichannel and RF64 WAV are not supported |
| 32-bit float WAV originals | Mono or stereo, 8–192 kHz, kept exactly; live FLAC/Opus or saved Opus | No saved FLAC |
| WAV with trailing bytes after a complete data chunk | Original kept exactly; only the declared PCM is used, with a warning | Truncated files and invalid internal boundaries fail |
| Supported CD ranges → FLAC | Explicit background preparation; output PCM is checked against the source | |
| CD ranges and FLAC/MP3/WAV originals → Opus 96/128/192 kbps | Album preparation or optional preparation on import | Lossy; digital sources are converted to 44.1 kHz stereo first. Presets are VBR targets, not exact bitrates |
| Opus originals | Not supported as an import format | |
| Whole-disc catalog verification | AccurateRip lookups where coverage exists | Encoding checks, structure checks and MusicBrainz identity do not prove a disc is correct |

Preparation can be cancelled and retried, and resumes after a restart.

## Prepare one album or a batch

Configure the [external encoders](configuration.md#audio-encoders). In **Prepare
audio**, choose **FLAC** or **Opus**; for Opus choose 96, 128 or 192 kbps VBR.
**Albums needing Opus** uses the selected bitrate, so an album with a ready
96 kbps copy still appears when you ask for 128 kbps.

Open an album to choose its sources and prepare missing tracks, or select albums
and choose **Start preparation**. **Select all matching results** includes albums
on later pages, up to 10,000 per batch. If a disc has alternate captures, open the
album and choose the original before starting.

The batch is saved immediately and albums are planned in the background. Follow,
cancel or retry it in **Activity**; retrying returns failed albums to the queue.
Albums that need a source choice, or whose inventory changed after submission, are
flagged for review. While the server is rebuilding summaries, readiness filters may
be incomplete. An album with incomplete track coverage stays in the filter and
counts only its known tracks.

## Prepare Opus after import

In **Administration → Library settings**, select the music collection. Under
**Opus on import**, choose a scope and bitrate and save. **CD and digital album
imports** includes digital-only FLAC, MP3 and supported WAV albums. Before
submitting an import, **Audio preparation after import** lets you keep that
default, choose another preset or skip preparation.

Imports save the originals first and report preparation separately. A missing
encoder, unsupported source or failed conversion leaves the originals saved and
the preparation step marked as needing attention.

Prepared audio can be downloaded and played by any client that supports its codec.
