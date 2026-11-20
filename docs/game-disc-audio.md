# Game-disc CD audio

Standard BINARY CUE/BIN game packages can expose their CD-DA tracks through the
normal audio preparation, player, song search, playlists and Trash Amp cache. The
game keeps its collection identity, originals and artwork. Proprietary game sound
files, CHD and mixed-mode raw redumper timelines are not supported.

- Only AUDIO tracks are playable.
- Pregaps follow the shared CUE playback policy: a gap between two audio tracks
  belongs to the preceding track; data-track pregaps never do. Hidden audio before
  track 1 is not queued automatically.
- Explicit synthetic gaps produce silence; missing or truncated bytes are errors.
- No de-emphasis is applied and no bit-perfect output is claimed.
- A manual title or track numbering for a source survives metadata refresh.

Import records the disc geometry with source fingerprints and frame coordinates.
Catalog reads do not hash raw masters. Background preparation re-checks the sources
against the recorded geometry before encoding. Extraction checks are separate from
catalog verification.
