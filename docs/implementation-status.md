# Implementation status

Current source reference: `71eaf64`, reviewed 2026-11-03. Source implementation,
published release and installation acceptance are separate states. This document
makes no claim about any reader's installed version. The [ADRs](decisions/README.md)
define architectural rules rather than implementation or qualification status. See [documentation authority](index.md).

The latest completed publication evidence in this checkout is
[beta.20](validation/container-beta20-publication-2026-11-03.md), which retains
failed qualification attempts, the inspected recovery incident and final acceptance.
The later [cancelled-publication cleanup repair](validation/cancelled-publication-2026-11-03.md)
is implemented and qualified in source; it is not included in that published image.
Publication and installation must not be inferred for this newer source change.

| Capability | Implemented behavior | Limits / evidence |
| --- | --- | --- |
| Preservation and inventory | Exact originals, physical/digital inventory, manual authority, unknown-data retention, removal/restoration and reindex | No generic history pruning; [retention policy](retention-audit.md) |
| Intake | Browser resumable uploads, configured inboxes, grouped package review, durable progress/recovery and explicit source cleanup | [Getting started](getting-started.md), [cleanup](inbox-cleanup.md) |
| Games | Manual DAT upload/review/activation and automatic identification | Finite [system/format matrix](game-identification.md); no universal container support |
| Digital music | FLAC/MP3 originals, integer PCM WAV analysis and FLAC preparation, tags/artwork and retained-audio analysis | [Supported inputs](digital-purchases.md), [audio matrix](audio-preparation.md) |
| CD audio | Supported raw/CUE layouts, explicit FLAC/Opus preparation, durable AccurateRip workflows and defaults | Named algorithms/coverage; [audio qualification](audio-preparation.md) |
| Digital album matching | Source-bound fingerprints, AcoustID/MusicBrainz discovery, incremental candidates, expansion and reviewed full mapping | Bounded identity suggestions; [latest source delivery](validation/digital-matching-and-review-2026-11-03.md) |
| Metadata/artwork | Automatic/manual authority, independent digital source and artwork choices, retained candidate history | Source/revision fences; [latest review](validation/github-metadata-review-2026-11-03.md) |
| Library/player | Progressive counts/pages, lazy tracks, artwork, search, queues and collection lists | [Read contract](layered-library-refresh.md), [player contract](album-metadata-consumers.md) |
| Media transport | Private media selection and isolated direct HTTPS service; requested-byte reads | Browser/network qualification is bounded; [private playback](automatic-private-playback.md), [direct media](../deploy/container/media.md) |
| Accounts/access | Passwords, passkeys, browser enrollment, collection roles, recovery/revocation and exposure controls | [Security contract](decisions/security.md); no implicit administrator collection grant |
| Documents | Authorized PDF/text originals and bounded browser previews | [Dated acceptance](validation/documents-2026-10-21.md); no server PDF renderer |
| Purchase inbox | Optional Proton Bridge, explicit receipt review, folder editing, credential repair and cursor recovery | [Email guide](email-purchases.md); no Gmail adapter |
| Backup/export | Stopped-server full backup/restore, portable full export/import, optional backup-age reminder | [Operations](server-operations.md); no scheduled backup or database-only recovery mode |
| Packaging | Native AMD64/ARM64 images, exact-byte validation/promotion, pinned install bundles | [Release pipeline](container-release-pipeline.md); candidate success is not publication |
| Withdrawn publication cleanup | Cancellation, lost-claim and typed-contention exits settle their own uncommitted V4 journal/staging while holding the writer; non-missing observation lookup errors propagate | [Repair/recovery evidence](validation/cancelled-publication-2026-11-03.md); committed revisions and uncertain-failure recovery remain protected |
| Contention/error handling | Typed contention yields/retries and differentiated mutation/refresh outcomes | [Contention review](validation/github-background-contention-2026-11-03.md), [error review](validation/github-error-presentation-2026-11-03.md) |

Trash Amp has separate source, packaging and device acceptance. Collector changes
cannot establish which native app is installed. Reuse its dated transport/offline
[evidence](validation/native-failure-recovery-2026-10-21.md) only where applicable.

Synchronized listening, broader codecs/layouts, generic retention, delta export,
legacy adoption, event delivery and advanced trust/routing remain conditional or
incomplete. [Remaining work](remaining-work.md) owns their disposition. The retired
Phono client is not a target; existing data and reusable mechanisms remain protected.

The [earlier status in Git](https://github.com/AberrantWolf/junk-collector/blob/7f4a56c8d6b38724a371549d09b978bbcb8e5cb1/docs/implementation-status.md) preserves
checkpoint outcomes. Its earlier “pending,” branch and deployment statements are
historical, not additional current status.
