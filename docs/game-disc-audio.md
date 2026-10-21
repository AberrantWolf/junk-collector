# Game-disc CD audio

Included in Collector beta.1, following source, owned-disc/browser and isolated
native streaming/offline acceptance. The matching Trash Amp integration and native
recovery improvements are committed on `codex/game-disc-audio`; this Docker release
does not publish a new Trash Amp application package. The authoritative assignment
is [HL-09](remaining-work.md#game-disc-audio-in-music-listening-and-trash-amp).

The game keeps its collection identity, originals and artwork. Standard BINARY
CUE/BIN packages can expose their real CD-DA tracks through the existing audio
preparation, player, song search, playlists and Trash Amp cache. Proprietary game
sound files, CHD and mixed-mode raw redumper timelines remain outside this delivery.
A source-specific manual title/printed numbering survives metadata refresh.

Only AUDIO tracks become playable. Stored INDEX 00 and explicit synthetic gaps
follow the existing shared CUE playback policy: audio-to-audio following gaps belong
to the preceding audio track; data-track pregaps never do. First-track hidden audio
is not queued implicitly. Explicit synthetic gaps produce declared silence; missing
or truncated bytes are errors. No de-emphasis or bit-perfect output claim is made.

Import/explicit inspection records bounded geometry, with source fingerprints and
logical plus physical frame coordinates. Catalog reads/admission do not hash raw
masters. Background preparation checks consumed sources and frozen geometry, then
uses the existing cancellable encoder, immutable recipe and durable publication.
Extraction/integrity checks are separate from catalog or whole-disc audio verification.

The user authorized read-only use of useful owned material under `~/dumps`; original
dumps stay unchanged and copyrighted payloads are not committed as test fixtures.
Synthetic independent vectors and private owned-disc runs supply separate evidence.
Results are recorded in [dated acceptance](validation/game-disc-audio-2026-10-21.md);
beta.1 advertises only these accepted formats and bounded claims.
