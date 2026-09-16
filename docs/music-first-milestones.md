# Proposed music-first delivery order

Planning assessment, 2026-09-14. This is a proposed scheduling change, not evidence
of implemented playback or authorization to skip acceptance gates. D05.3 was reviewed on 2026-09-15. D07 now precedes D06 in the active Wave 4
ledger; D07.1 and D07.2 are reviewed and corrected locally, D07.3 is reviewed and corrected locally, and its bounded browser-player child is reviewed and corrected locally. The later native-client acceleration remains proposed;
settle its bounded children before changing those assignments.

## Finding

The backlog's dependency graph allows usable music substantially before the end
of Wave 4. Its serial ledger places D06 enrichment and P05 backup ahead of native
connections, and Wave 5 places the Trash Amp adapter alongside a new desktop app.
Those are delivery choices, not playback prerequisites. C03 does not depend on
C02 or S04; C07 already depends only on D03, C01 and D07. D07.1 explicitly depends
on D05.1 and W4.2, not metadata providers or catalog verification success.

D05.3 has closed the local CD workflow checkpoint and exposes supported
layouts. AccurateRip no-data, outage or mismatch must never be a permission gate
for otherwise structurally supported audio playback. Unsupported PCM/layout
semantics remain a valid reason to refuse conversion.

## Recommended milestones and checkpoints

Additional user requirement (2026-09-15):
[D07.4–D07.5](background-work-plan.md) adds whole-album FLAC preparation and
independent enqueueing. U05 audits the same nonblocking rule across the application:
background work may limit execution concurrency, but must not prevent unrelated
eligible submissions. Both are mandatory for web completion, not prerequisites
for the already-reviewed early playback checkpoint. Scheduling remains unassigned;
the current P05.1 handoff is unchanged.

Additional user requirement (2026-09-15): [U05.1–U05.3](interface-readiness-plan.md)
is a second mandatory web completion gate. Establish smart defaults and typed
readiness, audit/correct every implemented screen and action, and validate full
journeys with missing/ambiguous inputs and unavailable capabilities. Apply the
same standard as future native/CLI interfaces land. This gate has no assigned
implementation date and does not change the current P05.1 handoff.

User requirement (2026-09-15): early playback is not web UI completion. The
[P04.4–P04.6 import-management follow-up](import-management-plan.md) is mandatory
before declaring web curation complete: identifiable imports and attempt history,
discard/reclamation of erroneous staged packages, and history cleanup. Schedule
this as a web completion checkpoint after the early browser-player milestone;
it can follow current feature work and does not replace the D06.1 handoff or block
independent native-client development. Its implementation date remains unassigned.

| Order | Scope | User-verifiable result | Required review/backup checkpoint |
|---|---|---|---|
| 1 | Finish D05.3 | Import a copied supported BIN/CUE or raw-redumper package, inspect structure and separate AccurateRip results | D05 root review; checkpoint D05.2–D05.3 together |
| 2 | D07.1–D07.3, then a bounded web playback child | Prepare validated FLAC tracks, browse with existing title/artwork or placeholders, play/pause/seek and advance through a disc in the WebUI | Review recipe/track ownership at D07.1; publication/recovery at D07.2; integrated HTTP and actual browser playback at milestone end; checkpoint |
| 3 | C01.1–C01.3 | Pair once, restart, browse and read media through a manually configured encrypted LAN endpoint | Credential/DTO/trust review, real ranged-reader tests and macOS Keychain qualification; checkpoint |
| 4 | Bring C07 forward after a bounded provider/playback contract review | Browse Collector CDs with art/text and play them in Trash Amp on macOS | Sibling and shared-reader review; actual device playback, seek, next-track and revocation checks; checkpoint |
| 5 | D06.1–D06.3, P05.1–P05.3 | Better automatic names/artwork and explicit portable export/full backup/recovery | Preserve metadata authority and recovery root reviews |
| 6 | S04 then C02; C03/C04, C05/C06 and remaining release work | Rotation, automatic routing/failover, full Collector desktop app, offline library and activity sync | Retain their security, recovery, audio and release gates |

This changes ordering, not first-release requirements. P06 scale, R01 legacy
import, R02 deployment/oracles and R03 release audit remain required. P05's current
post-S04 backup/recovery rerun also remains required. Manual configured LAN access
can work before discovery; unexpected trust changes must fail closed and require
explicit re-enrollment until authenticated rotation exists.

User clarification (2026-09-14): the parallel FLAC library is optional. Initial
preparation is explicit; import, inventory and verification do not depend on it.
FLAC is the default requested derivative format, not a mandatory second copy of
every disc. Future automatic maintenance is opt-in and uses the same durable jobs.

Keep D07's existing three children and FLAC/Opus acceptance intact initially.
FLAC is the first user path, but splitting every codec into more checkpoints would
add handoff overhead. If Opus becomes the actual blocker, explicitly move its
remaining acceptance into a named follow-up before claiming a FLAC-only milestone;
never mark all of D07 complete with its lossy path absent.

## Small web playback child after D07.3

This is an early usable surface, not C04 native-playback acceptance. It needs no
new backend player, session queue protocol or streaming/transcoding endpoint.

- Application services now expose authorized ordered track descriptors bound to
  immutable published representation/file IDs, exact frame counts, media type,
  availability and safe display fields. D07.3 settled this consumer contract.
  C01 subsequently consumes the same read DTOs;
  do not invent a parallel browser-only model or expose archive paths.
- Use the existing authenticated same-origin media route, with D07's corrected
  Range/If-Range, ETag and authorization behavior. Never place credentials in URLs.
  Conversion stays an explicit durable job, not work triggered inside a media GET.
- Add Play/pause/seek, next/previous and an in-memory ordered disc queue beside the
  existing art/text display. Show preparing, unavailable, expired-session and
  playback failures honestly. No persistent favorites, offline cache or cross-device
  queue is needed for this milestone.
- Browser time is a UI projection. Domain track ranges remain integer frames.
  Do not claim sample-accurate seeking, gapless output or DSP parity from an HTML
  media element. Verify FLAC decoding on the actual target browser; if unavailable,
  show download/open guidance rather than declaring browser playback accepted.
- Test with actual generated derivatives and a viewer account, including seeking,
  track transitions, refresh, failed conversion and revoked access. A click that
  merely sets an audio URL is not playback qualification. Keep artwork beside text
  rows and meaningful placeholders when no cover exists.

Existing manual inventory labels and uploaded artwork are sufficient initially;
track-number fallback labels must remain useful and ordered. MusicBrainz/CAA and
metadata review are enhancements, not a second source of playback identities.

## Native acceleration through Trash Amp

C07 is likely the smaller native route because Trash Amp already has a Slint
browser, queue, decoding, CPAL output and shared audio primitives. It is not a
trivial URL substitution: its current MediaAccess values and playback preparation
are largely filesystem-oriented, and its browsing provider needs inspection for
paged remote results. Do not claim the existing Managed variant is a complete
remote reader implementation.

Before assigning C07 to an implementing model, split it into bounded children:

1. Review and implement the provider/read boundary: opaque connection + durable
   media identity, paged album/track/artwork queries, and bounded authorized
   seekable reads through collector-client. Keep credentials/trust in C01, byte
   transport in its adapter, decode/queue/DSP in shared audio services, and UI
   orchestration outside them. Extract genuinely shared mechanisms separately and
   check both consumers. Do not route every remote track through a whole-file
   temporary download or teach the audio callback to perform network I/O.
2. Compose enrollment and provider browsing in Trash Amp without changing local
   library behavior. Collector metadata is authoritative for its provider; do not
   run Trash Amp's local-library metadata scraper over remote albums. Preserve
   distinct server/connection/item identities and collection authorization.
3. Connect the bounded reader to existing playback services and controls. Test
   first playback, seeks, queue transitions, interruption, expiry/revocation and
   restart. Qualify actual macOS output and encrypted LAN use with Internet
   disconnected. Automatic alternative-route failover and persistent offline
   copies remain C02/C05, with explicit errors until available.

Any transport-to-decoder bridge needed by both C07 and future C04 belongs in the
shared client/audio service boundary, not in Trash Amp's UI. This ensures the early
player is an enduring consumer of the same server and services.

## Review cadence

Stop for review at changes to durable publication, source/track semantics, secret
storage/trust and the first remote-reader/audio boundary. After those contracts
are reviewed, an agent can complete routine wiring and UI work within its assigned
bounded milestone, running its checks without an approval after every edit.
Continue to review root acceptance before advancing, and checkpoint working user
milestones for backup rather than adding schema versions for each checkpoint.

Do not make real-dump, hardware or browser qualification a paper exercise. Start
with copied test packages and retain originals. Bulk archive adoption still needs
P05 and the dedicated R01 importer where applicable; early playback is not that
acceptance gate.
