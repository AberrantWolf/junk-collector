# Implementation status

What the server currently does. Planned work is in [remaining work](remaining-work.md).

| Area | Implemented | Limits |
| --- | --- | --- |
| Preservation and inventory | Exact originals, physical and digital inventory, manual metadata authority, unknown-data retention, removal/restoration, reindex | No generic history pruning |
| Intake | Resumable browser uploads, configured inboxes, grouped package review, durable progress and recovery, explicit [inbox cleanup](inbox-cleanup.md) | |
| Games | DAT upload and activation, automatic identification | Finite [system/format list](game-identification.md) |
| Digital music | FLAC/MP3 originals, integer and float PCM WAV, tags and artwork, FLAC preparation from integer WAV, file ordering and disc/track assignment | [Supported inputs](digital-purchases.md) |
| CD audio | Supported raw/CUE layouts, FLAC/Opus preparation, AccurateRip checks | [Audio support](audio-preparation.md) |
| Album matching | AcoustID/MusicBrainz suggestions with side-by-side review; optional AcoustID contributions | Suggestions only; contributions need a personal AcoustID key |
| Metadata and artwork | Automatic and manual authority, independent source and artwork choices, candidate history | |
| Library and player | Progressive counts and pages, lazy tracks, artwork, search, queues, collection lists; light/dark appearance | [Read contract](layered-library-refresh.md), [player API](album-metadata-consumers.md) |
| Playback | Original, High Bandwidth (saved audio or live FLAC) and Low Bandwidth (Opus, default 128 kbps) modes; per-account format preference; live conversion without touching the archive | [Settings](configuration.md) |
| Media transport | Normal authenticated audio by default; optional [private](automatic-private-playback.md) or isolated [direct HTTPS](../deploy/container/media.md) media service | |
| Accounts and access | Passwords, passkeys, browser enrollment, collection roles, recovery/revocation, listener exposure controls | Administrators get no implicit collection access |
| Documents | PDF/text originals with browser previews | No server-side PDF rendering |
| Purchase inbox | Optional Proton Bridge [email intake](email-purchases.md) with receipt review | No Gmail |
| Backup and export | Stopped-server backup/restore, [portable export/import](archive-export.md), optional backup-age reminder | No scheduled backups |
| Packaging | AMD64/ARM64 container images, pinned install bundles | |

Trash Amp is a separate project with its own releases.

Not implemented: synchronized listening, AAC/ALAC/Opus originals, generic history
pruning, delta export, migration from other products, event push to clients, and
relay/advanced routing. See [remaining work](remaining-work.md).
