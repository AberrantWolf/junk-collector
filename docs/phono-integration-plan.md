# Phono integration delivery — PJ1

Accepted 2026-10-04. This is a new native Phono delivery, independent of Trash Amp
and the remaining general native-client backlog. Preserve local catalog data,
manual authority, raw masters and existing provider workflows.

## Boundaries and delivery ledger

| Child | Delivery | Current evidence |
|---|---|---|
| PJ1.0 | Extract bounded compressed decoding and sequence conversion into junk-libs-audio; adapt Trash Amp | Implemented; independent FLAC vector, exact seek, truncation, cancellation and continuous/mixed-rate PCM tests pass; shared workspace tests/lint and Trash Amp test/check/lint plus Retro Junk tests pass |
| PJ1.1 | Keychain enrollment, multiple connections, shared transport DTOs, literal song search, artwork and prepared playback | Implemented; native TLS enrollment/range reader and playlist command test pass; native enrollment, restart, tabs/artwork and audible output observed; remaining GUI matrix open |
| PJ1.2 | Continuous Kira stream, personal queue, buffering, seeking, shuffle and repeat | Implemented; shared deterministic PCM capture passes; native pause/resume, seek, repeat-one, navigation and revocation passed; focused audible boundaries and remaining queue matrix open |
| PJ1.3 | Revision-fenced playlist editing and durable uncertain commands | Implemented; restart/exact-input journal tests and native TLS create/replay/rename/conflict pass; native create/repeated append/rename/conflict/reapply passed; native restart lost-response journey remains open |
| PJ1.U | Account/connection usability repair | Three software increments implemented; [child ledger](account-usability-delivery.md) tracks preservation, checks and remaining native GUI/system-root HTTPS qualification |
| PJ1.4 | Complete acceptance and second physical Mac | Open; do not infer audible gaplessness or LA3 acceptance from PCM/unit tests |

## DRY and best practices

Collector API DTOs live in transport-only collector-api-types. Authenticated
requests, pinned trust and seekable range reads live in collector-client. Phono's
CollectorService and joined worker own use cases; GUI code consumes typed results.
Local LibrarySession remains independent. Media bytes are decoded and normalized
by junk-libs-audio; Phono retains Kira. Queue transitions use junk-libs-playback.
Search indexes metadata and recorded structures, never original audio bytes.

Unknown/unprepared songs remain visible. Album expansion reuses Collector's
existing list target policy. Playlist playback freezes complete revision-consistent
occurrences. Client journals bind server/account/collection, revision, key and exact
inputs before contacting the server. Conflicts require explicit reapplication.
The journal is separate from the local catalog; credentials reside in Keychain.

Network chunks, JSON, artwork and PCM buffers are bounded. Generation cancellation
fences reads and stale UI results; admitted mutations retain durable ownership.
Workers and media readers are stopped and joined. Authorization is checked by the
server and refreshed by the client. Listener capabilities reflect actual exposure.

Repository gates passed locally on 2026-10-07. Local evidence is recorded in [the dated validation record](validation/phono-integration.md).

## Usability repair direction — 2026-10-07

The user requested ordinary web sign-in to lead directly to native access. Use
system-trusted HTTPS by default; owner-provided certificate/fingerprint entry is
an exceptional trust path. Collector now binds account-less enrollment to the enabled account that explicitly
approves it after recent browser authentication. See [the review](account-connection-usability-review.md) and
[dated validation](validation/phono-integration.md) for remaining recovery/account
work and exact evidence. Do not claim actual native system-root HTTPS deployment
from a localhost browser or pinned-certificate fixture.

## Remaining acceptance

Finish the remaining matrix in the validation record: multiple servers, denied
trust, pagination and keyboard navigation, native delete/move and lost-response
restart recovery, shuffle/repeat-all and unavailable queue entries. Exercise malformed/range-change,
network stall and revocation while media is being read. Verify healthy transitions
through a real output device. Finally use a second physical Mac against a trusted
LAN endpoint for playback, Curator writes, denied Viewer writes and grant/device
revocation. Keep hardware acceptance open when unavailable.

Excluded: preparation, persistent media cache, offline downloads, synchronized
listening, discovery and a private-playlist model. No startup autoplay.
