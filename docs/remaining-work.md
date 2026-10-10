# Remaining work — home-lab priorities

Reassessed 2026-10-09 at the user's request. This is the **single authoritative
queue** for remaining work, including qualification and deferred ideas. It
supersedes assignment/order sections in earlier plans, the old serial waves and
older “required before first release” gap lists. Detailed plans remain design
hypotheses and evidence references; old proposed contracts require reassessment
and do not create separate queues. Obsolete backlogs and assignment prompts
have been removed; completed work is described by status and dated validation.
[Implementation status](implementation-status.md) records capabilities;
[release requirements](release-requirements.md) records the revised delivery gate.

## Prioritization and execution

Optimize for one home server, a few trusted users, valuable originals, ordinary
LAN listening and optional authenticated Internet access. Deliver complete useful
workflows across layers. Preservation, recoverability and Internet-facing access
controls are essential even for one user. Keep working security controls; simplify
future scope rather than weakening authorization or TLS.

The numbered groups below are priority order, not a serial dependency chain.
Phono Junk is no longer a target. Begin with the contract/retention reassessment,
then finish available Group 1 work. Unavailable hardware does not prevent
independent backup or deployment work. Group 2's public-access checks are required
before describing an Internet-facing installation as qualified. Group 3 is useful
when actual usage calls for it. Group 4 remains documented without blocking a
home-lab release. A concrete vulnerability or low-effort hardening fix moves ahead
of feature work regardless of group. No implementation or deployment is authorized
by this documentation consolidation alone.

Before implementing an entry, read AGENTS.md, architecture, current status and
its linked design/evidence; inspect the current checkout and affected sibling instructions.
Preserve credentials, grants, configuration, unresolved command journals, raw
masters, meaningful collection evidence, manual authority and unknown module data.
Apply [the revised scope/retention policy](decisions/home-lab-scope.md): old proposed
contracts are not mandatory, and repeated lookups are not collection provenance.
Do not delete existing records merely because their future retention is reassessed.
Use temporary installations for tests.
Complete the named outcome and its necessary dependencies; do not restart completed
roots or silently absorb optional features. Update this queue, the owning child
ledger, status and dated validation together. No commits or agents without an
explicit assignment. Apply repository gates at implementation boundaries; prose-only
changes need architecture/link/coverage checks.

## Completed requested delivery — automatic standard games, 2026-10-10

The explicit user assignment for automatic standard-game identification is delivered
across shared mechanisms, Collector services/storage/API and the real web workflow.
See [supported matrix](game-identification.md), [status](implementation-status.md#automatic-standard-game-identification--2026-10-10)
and [dated validation](validation/game-identification-2026-10-10.md). This supersedes
older descriptions requiring manual system/profile selection in the ordinary game
catalog workflow. No second remaining-work queue is created.

New format/normalization algorithms, arbitrary custom systems/conventions, game
artwork providers and provenance compaction remain deferred under existing scope
and HL-01/HL-05/HL-08. The finite matrix does not claim all conceivable standards.
Collector pins pushed shared-library revision
`83177af57ca36a5b284298020fea9d5e8fb0c415`; local Cargo patches are removed.

## 1. Finish the usable installation and protect the collection

### HL-01 — Simplify contracts and retain meaningful provenance

**State:** the 2026-10-10 provider acquisition/response delivery is implemented; the broader retention audit remains.
See [delivery validation](validation/concurrent-processing.md) and [stopped-server conversion](snapshot-duplicate-conversion.md).
Two processing slots and one provider slot, active request sharing, equivalent response reuse and explicit duplicate-payload conversion address provider redundancy. Other retention policies remain unimplemented; no live archive cleanup was performed.
**Maps:** A04/P01/D06/FX-02 and old history requirements. **Owner:** application
policy, module evidence semantics and archive/private-state adapters.

Review proposed contracts and current mechanisms against one actual user outcome
before implementation or refactoring. Remove speculative abstractions, frozen
checkpoint compatibility, redundant state and mandatory handoffs with no real
consumer/recovery purpose. Preserve clean ownership and real data guarantees.

Define and implement a bounded retention policy. Keep originals, user inventory/
manual choices, accepted metadata/artwork with enough source information to explain
it, and substantive source-bound verification results. Do not treat “looked up album
art on X, Y and Z days” as collection provenance. Repeated equivalent metadata or
artwork fetches need at most bounded diagnostic/cache state; redundant candidate
versions, raw responses and check occurrences need a demonstrated user/evidence
purpose rather than automatic permanent retention. Meaningful changed results or
user-requested rechecks can have useful history without retaining every access.

Separate durable command receipts needed for uncertain retries and unfinished-work
recovery from expired operational logs. Audit archive growth from revisions,
provider observations, snapshots, tombstones and derived assets; define reference
reachability and retention before implementing compaction. Do not invent a generic
retention engine if a few owned policies suffice.

**Done when:** the policy distinguishes must-keep collection data, replaceable/
bounded diagnostics and removable redundancy; dry-run cleanup shows exact effects,
backup/conversion and interrupted-compaction recovery are proven, current reads/
reindex/unknown data/manual authority and real outstanding retries survive, and
unreferenced records are not still required by meaningful evidence. Retire invalid
old contracts explicitly. No deletion of originals as routine history cleanup.
See [scope and retention](decisions/home-lab-scope.md). This is a priority design
and simplification delivery, not approval to prune the user's archive now.

### HL-02 — Complete web and Trash Amp LAN qualification

**State:** web and Trash Amp workflows implemented; external acceptance incomplete.
**Maps:** remaining relevant LA3 and C01/C04/C07 checks. **Owner:** application,
transport and Trash Amp presentation; fixes belong in their owning layers.

Use [the installation runbook](local-alpha-installation.md), Collector's
[Trash Amp status](implementation-status.md#trash-amp-client-integration-2026-10-07)
and sibling `docs/collector-connection.md`. Finish:

- Trash Amp system-trusted HTTPS enrollment/browser approval and persistence;
  multiple servers/accounts, cancelled or interrupted enrollment and switching
  during playback or uncertain edits. Reassess old native matrix cases against
  the actual Trash Amp implementation before adding new requirements.
- Real output/track transitions, seek/queue controls and device-loss behavior;
  stalled/malformed/changed ranges, denied trust and grant/device revocation
  during actual reads/playback. Native playlist command loss/restart recovery
  should demonstrate the real durable-command path, not reproduce Phono tests.
- An actual second LAN device with browser-trusted HTTPS: Viewer browse/artwork
  and FLAC playback, permitted Curator writes, denied Viewer writes and grant/
  active-session revocation. Use Trash Amp on a second Mac only for claims about
  that native platform; a web-device gate need not require a Mac. Verify supported
  native downloads and Internet-disconnected LAN use on their actual client.

**Done when:** the remaining relevant cases have reproducible reports naming host,
browser/client, trust mode and output device. LA3 stays partially qualified without
its required physical-device evidence. Reuse passing corpus/provider/restore work
unless affected code changes. Phono's PJ1/account GUI and second-Mac acceptance
are retired, not passed, and do not block anything. Shared Collector/account/TLS
mechanisms remain useful; do not remove working services just to retire a client.

### HL-03 — Make stopped-server backup and restore dependable to operate

**State:** an actual stopped-copy/restore drill passed; dedicated tooling is missing.
**Maps:** practical P05.2–P05.3 subset. **Owner:** application preflight/recovery,
archive/private-state adapters and CLI; web offers instructions/status.
Reassess the old P05 design for the smallest
coherent backup/restore workflow; its individual proposed commands are not binding.

Start from [the working runbook](local-alpha-installation.md). Build the smallest
coherent stopped-server workflow; no online-backup system is required. Provide explicit stopped-server backup, validation and restore into empty
roots. Lock both storage roots, settle journals, include archive, private state,
TLS/config and pending-work scratch coherently, and write a digest inventory plus
completion marker last. Validate before opening/migrating restored data. Report
relocated inbox/tool paths and restrictive private-state permissions.

**Done when:** temporary-root tests/drills cover live-lock refusal, interrupted or
corrupt backups, exact originals/unknown data/manual metadata/loans/evidence,
accounts/grants and resumed pending work. Full restore preserves identity with
only one active instance; archive-only recovery/independent clones use fresh
private identity and explicit grants. Portable export/deltas are HL-09; they are
not prerequisites for a safe full backup. An actionable CLI and runbook suffice;
a backup dashboard and unattended scheduling are optional later work.

## 2. Secure and qualify the actual home-server deployment

### HL-04 — Linux, NAS and optional public-Internet deployment

**State:** Linux ARM64 Docker Desktop runtime/persistence passed locally; actual
home-server, NAS and public ingress qualification remains. **Maps:** R02/R03
practical deployment, S01–S03 external qualification. **Owner:** release/server
adapters and focused security review.

Qualify the intended Linux binary or non-root container on the actual host, with
explicit archive/state/scratch mounts, local SQLite and optional NAS payloads.
Exercise restart, interrupted publication/upload/encoding, missing optional tools,
storage permissions and backup restore there. Record the filesystem and failure
model; local process-kill tests are not power-loss/NAS certification.

For Internet access, execute one intended trusted-HTTPS proxy/tunnel configuration
end to end. Public ingress must reach only the public listener; private management
uses explicit private LAN/tailnet ingress. Curation/admin stay publicly disabled by
default. Test all supported exposure combinations for both pages/assets and APIs,
direct role denials, spoofed forwarded headers, exact Origin/CSRF, cookie/session
expiry, recovery, credential revocation and redaction. Verify stable authentication
origin and real supported browser/password-manager/passkey behavior; test a hardware
key when that authenticator is part of the supported setup. Keep platform claims
limited to the devices actually exercised.

Exercise account/peer/global rate buckets, process-restart reset behavior,
unauthenticated enrollment/challenge floods, slow/oversized uploads and ranges,
provider/parser CPU/memory bounds and Job cancellation under realistic home-server
load. Keep forwarded headers untrusted. Fix demonstrable abuse gaps; distributed
limiting is unnecessary for a single process. Document operator recovery and
certificate renewal/re-enrollment for the chosen ingress. Obtain focused review
of exposed authentication, upload/media and listener boundaries; broader protocol
certification is HL-13.

**Done when:** the recorded installation can browse/listen, recover, deny unwanted
access and withstand the bounded failure/abuse matrix. Public qualification is a
gate for supported public use, not for a LAN-only release. A commercial deployment
matrix, automatic route switching and every browser/platform are unnecessary.

### HL-05 — Qualify advertised preservation, verification and audio support

**State:** implementation and independent synthetic vectors exist; coverage gaps
remain. **Maps:** D02/D04/D05/D07 and R03 real oracles. **Owner:** shared libraries
for mechanisms, modules for semantics, application/adapters for work and evidence.

Maintain a per-operation supported/unsupported/externally-unqualified matrix.
Use owned or legally supplied independent references for the formats actually
advertised: cartridge normalization, ordered game tracks, CD structure and
AccurateRip v1/v2/offset/coverage, and FLAC PCM equality. Live MusicBrainz/CAA and
one owned Redump PlayStation complete match already passed; these do not qualify
all formats or audio checks. Live AccurateRip/whole-disc owned audio verification
remains open. Preserve partial/inconclusive results and exact computation coverage.

Qualify real `opusenc` and configured lossy profiles only if advertising that path;
FLAC-first releases may label Opus unqualified without blocking FLAC. CHD runtime,
trainer-bearing NES and other unsupported/ambiguous layouts remain unavailable
until their named independent vectors/oracles and bounded readers pass. Additional
owned NES/N64/SNES/PC Engine matches require the actual dumps and matching catalogs;
they are not prerequisites for a PlayStation/music home-lab delivery. Use manually
supplied DATs, never retry blocked DAT-o-MATIC downloads or restore scraping.

**Done when:** each advertised claim has independent evidence and explicit limits,
including truncation, aliases, smallest inputs, interrupted work and changed
snapshots. Metadata identity, integrity and structure never masquerade as catalog
or whole-disc audio verification. Expand formats only in response to actual use.

### HL-06 — Release and maintenance acceptance

**State:** source alpha exists; no supported home-lab release baseline declared.
**Maps:** bounded R03, release-toolchain maintenance and independent UI review.
**Dependencies:** HL-02–HL-05 only for the platforms/capabilities advertised; safe
backup/recovery remains mandatory. **Owner:** integrated release review.

Publish a capability/evidence checklist for the chosen web/Trash Amp scope,
installation/upgrade/backup instructions and explicit unsupported behavior. Run
repository gates, rebuild embedded assets, exercise actual setup/import/metadata/
prepare/listen/list editing and targeted keyboard/mobile/accessibility review.
Check current stable Rust with [the maintenance procedure](release-toolchain.md)
before each release; the October upgrade is done, not a recurring feature ticket.
Track affected sibling lint findings only when touching or qualifying those crates;
do not silently modify sibling projects to clear unrelated warnings.

**Done when:** every claimed capability links to tests or actual manual evidence,
residual defects have scoped entries, and any supported API/storage/upgrade promise
is explicitly declared under [pre-release evolution](decisions/pre-release-evolution.md).
An experimental source tag is not full acceptance. Prebuilt binary/image publishing
is useful packaging, not proof of runtime qualification. Phono Junk, a separate
Collector Slint app, 100k-item benchmark, TLS rotation and legacy migration are not universal gates.

## 3. Useful follow-ups, selected by actual home-lab need

### HL-07 — Finish Trash Amp reliability and offline acceptance

**State:** adapter, native Slint browsing/list editing, FLAC streaming and explicit
downloads are implemented; acceptance remains partial. **Maps:** C01/C03–C05/C07.
**Owner:** Trash Amp adapter/presentation over shared client/audio services.

Inspect sibling `docs/collector-connection.md` before assigning work. Qualify
output capture/fidelity, device loss, stalled/changed ranges and revocation,
rapid server/account switching, two-account live authorization isolation and
comprehensive download failure/restart UI journeys. Check durable snapshots,
artwork/text offline browse/search, pinning/eviction, budget behavior and immutable
cache identity against C05; record exact missing behavior before implementing it.
The existing 10 GiB download budget and 128 MiB artwork cache are not proof that
all older C05 requirements pass. Preserve standalone local playback and retained
offline files; revocation cannot recall disconnected copies. No need to rebuild
C07, duplicate a player or require C02 failover before single-route downloads.

### HL-08 — Collection conveniences, game-disc listening, provider contributions and performance gaps

**State:** optional scoped work; most earlier web gaps are completed. **Maps:** U04,
D05/D07 listening extensions, D06 follow-ups, FX-04 history bound, P06/R02 practical scale.

#### Collection convenience and performance follow-ups

- Repair Library “Back to results” after nested Work & history navigation: an
  older `work` URL entry can reopen the history modal instead of returning to the
  unselected list. Observed in the 2026-10-09 AccurateRip shortcut smoke journey;
  Library navigation avoids that history-stack defect. Preserve intentional
  browser Back restoration of a work detail while correcting the browse return.
- Add document attachments when manuals/receipts are needed, using existing Asset
  visibility/publication with bounded validation and honest download/preview types.
  Image Covers/Photos/Scans already work. Do not imply arbitrary documents work now.
- Add a game metadata/artwork provider only for an identified collection need,
  with provider terms/access established, bounded acquisition, meaningful source
  attribution and manual authority preserved under HL-01 retention policy. Manual
  artwork and uploaded DATs work today.
- Page old preparation batches if Work & history is insufficient; the item view's
  latest-50 bound is disclosed. Page metadata responses before increasing the
  current 4,096-version per-item ceiling. Improve prefix-comparison upload resume
  bandwidth or inbox traversal only after measuring an actual bottleneck.
- Measure startup/browse/search/import/preparation on representative collection
  sizes and the actual low-powered host/NAS. P06 and generated bulk measurements
  are complete. Fix observed slow/uncancellable/unbounded work; do not reopen
  incremental indexing wholesale or require enterprise-sized load as a release gate.

Each bullet is independently assignable with real UI/recovery acceptance. Smart
lists, private/personal lists, post-edit splitting and richer DSP/codecs remain
optional ideas needing product/contract design, not established missing features.

#### Game-disc audio in music listening and Trash Amp

**State:** requested 2026-10-10; game-disc listening is not implemented.
**Owner:** Games owns game identity and descriptive metadata; shared `junk-libs`
readers own disc layout and PCM extraction; application services own preparation,
authorized listening projections and list expansion; web and Trash Amp consume
the shared listening contracts.

Expose audio tracks included on CD-based game discs through normal music
browse/search, queues and playlists, plus Trash Amp streaming and explicit offline
downloads. Keep game artwork and source attribution beside track text. Start with
actual CD audio tracks, including mixed data/audio discs; proprietary in-game audio
files and soundtrack decoding are separate future scope. Preserve the original
game package and its collection identity without requiring a duplicate music import.

Before implementation, settle cross-module listening capability ownership and
source-bound track metadata, including manual titles and useful unnamed-track
defaults. Current preparation requires a music collection, and mixed-mode audio
preparation is unsupported. Extend the existing preparation/availability/media,
music discovery and list pipeline rather than introducing a second importer,
player or workflow. Complete bounded, cancellable audio-range extraction with
explicit data-track exclusion and gap/boundary policy; reuse exact source
coordinates, immutable recipes, lossless validation and durable publication.
Browse and admission must use recorded observations without hashing masters.
Automatic game metadata/artwork providers are not prerequisites.

**Done when:** an owned mixed-mode game demonstrates track discovery and explicit
FLAC preparation, ordinary web music browsing/search and listening, playlist
expansion, Trash Amp streaming and offline artwork/text browsing and playback.
Independent vectors cover audio/data boundaries, stored/synthetic gaps and malformed
or truncated inputs. Cancellation/restart, reindex, manual metadata authority and
collection authorization/revocation preserve the existing guarantees. Data tracks
never become playable, unsupported layouts remain explicit, and successful
extraction does not imply catalog or whole-disc audio verification. HL-05 owns
qualification of the advertised layouts; HL-02/HL-07 provide client acceptance
patterns without turning this new feature into a prerequisite for existing LAN
listening. Deliver the complete workflow across layers as one assignment.

#### AccurateRip contribution investigation

**State:** requested 2026-10-10; submission support is not implemented.
**Owner:** application/provider policy; adapters own any supported submission mechanism.

Investigate how to contribute eligible AccurateRip results, especially for discs
whose lookup returns no data. Motivating case: **SQ Relax - Krystelle's Chillwave
Vol. 2**, by SQUARE ENIX MUSIC & Krystelle. An independent original-TOC calculation
and fresh HTTP/HTTPS requests confirmed no data for its exact disc layout; see
[lookup validation](validation.md#sq-relax-vol-2-live-accuraterip-lookup--2026-10-10).

Establish the supported submission route, provider access requirements, eligibility
and required drive/rip information. Assess a Collector workflow or guidance through
a supported external tool. Keep missing provider data distinct from a verification
failure; submitting results must not itself confer verified status. Require explicit
user authorization before submitting results externally.

**Done when:** the supported route and prerequisites are documented, and the
investigation recommends a feasible integration, an external-tool workflow, or
records a concrete blocker. Implementation is a separate scoped assignment after
that assessment; this investigation does not authorize external submission.

#### Completed presentation follow-ups

Completed presentation follow-up, 2026-10-09: album FLAC/Opus availability now uses
compact secondary labels and on-demand unavailable reasons, two-line track rows,
overflow actions, and collapsed inspection information. The desktop entry
panel also meets the right viewport edge and dock at every scroll position; see
[status](implementation-status.md#compact-album-format-availability--2026-10-09) and
[validation](validation.md#compact-album-format-availability--2026-10-09).

Completed presentation follow-up, 2026-10-09: album rows show recorded AccurateRip
results instead of the independent Identity flag. Curators can open audio checks
from the column, album menu, or disc inspection; this retains honest original-level
coverage and adds no audio reads to browsing. See
[status](implementation-status.md#accuraterip-browsing-shortcut--2026-10-09) and
[validation](validation.md#accuraterip-browsing-shortcut--2026-10-09).
HL-05 live owned-disc qualification remains outstanding.

### HL-09 — Portable export and one-way legacy adoption

**State:** P05.1 was paused; no implementation is claimed. **Maps:** P05.1, R01.
**Owner:** application/archive export and legacy-import adapters.

First deliver a coherent frozen full archive export retaining IDs, originals,
assets, lists, recipes, meaningful collection history/evidence and exact unknown
records under the HL-01 policy, with digests/completion marker and no
credentials/grants. Test
partial copy, concurrent later writes, fresh import/reindex and private-data absence.
Standalone full export can precede optional delta bundles. Deltas require explicit base digest and dependency
closure; they do not implement replication.

Only implement Retro Junk migration when a real legacy archive must be adopted.
Inspect actual versions, dry-run mapping/unmappable fields, retain original sources
and manual choices/history, and make restart/idempotence explicit. Importing raw
files today does not migrate legacy IDs/history. Use export/backup contracts and
independent fixtures; R01 is conditional, not a mandatory rewrite for every owner.

### HL-10 — Personal activity, events and additional native surfaces

**State:** basic activity service and polling exist; offline reconciliation/events
and a standalone Collector Slint app are incomplete. **Maps:** C06, residual C03/C04.

Specify favorites/playback-event identity, conflicts and reconnect deduplication
before adding cross-device personal activity. Add HTTP events with bounded authorized
cursors and polling recovery only if polling latency/load warrants it. Neither
requires synchronized playback or archive replication. A dedicated Collector Slint
inventory/browser app is optional now that web and Trash Amp cover practical
use; reuse typed services, readers and artwork rather than creating parallel policy.
Native/offline gaps apply to the actual supported client, Trash Amp.

## 4. Deferred advanced capabilities and retained long-term requirements

### HL-11 — Automatic certificate rotation, discovery and route failover

**State:** S04/C02 unimplemented; manual trusted endpoints work. The design below is
a reference to reassess, not a fixed contract.

Automatic authenticated local-identity rotation with generations, old/new grace
listeners, offline transitions, atomic trust storage and restart recovery is substantial
work. Ordinary HTTPS renewal and explicit re-enrollment guidance belong to HL-04.
Prioritize this machinery only if expiry/recovery burden or a concrete threat warrants
it. Never accept unexpected keys or bypass certificate validation in the meantime.

mDNS, health probing, LAN preference/hysteresis and verified ranged failover are
conveniences for multi-route use, not prerequisites for manually configured encrypted
LAN or public access. Inspect C01 gaps in the actual consumers first; verified trust
must precede credentials, and resumed bytes must bind to immutable content. Re-run
backup qualification if rotation adds durable state. Actual LAN/tailnet tests remain
required before claiming these capabilities.

Potential rotation design, if needed: authenticate new trust through an already
trusted route; bind old/new certificate fingerprints and increasing generation to
ServerId; persist trust atomically before use. Stage key/certificate pairs together,
recover interrupted activation, and explicitly choose a bounded old-trust grace
route or re-enrollment. Never send credentials to an unexpected identity. Test
replays, wrong identity, expiry, offline devices and restart/restore with real TLS.
Grace listeners must not expose management implicitly. These are design constraints,
not a mandate for a multi-listener generation framework.

Potential routing design, if needed: treat mDNS/DNS-SD as untrusted hints, keep
public/tailnet routes explicit, bound uncredentialed probes and verify identity
before attaching credentials. Prefer healthy LAN with measured hysteresis; resume
only against the same immutable representation, length/digest and validator.
Persist only necessary download progress. Test forged/expired hints, changed keys,
route loss/recovery and actual LAN/tailnet transfers. `.local` is local-link
naming, not tailnet discovery. Exact probe counts/timing are implementation choices
based on a measured need, not inherited stability contracts.

### HL-12 — Synchronized listening and broader product expansion

**State:** ideas retained, not implemented. Shared listening was requested 2026-10-03:
join ongoing playback across rooms/devices without restart, support same-user and
other authorized listeners, define close LAN synchronization/drift correction, and
allow WAN shared listening without tight timing. Design session ownership, control/
join permission and coordination first; each participant retains media authorization.
This is distinct from HL-10 personal activity sync.

Also retained: SwiftUI iOS/UniFFI, PriceCharting exchange after API/permission research,
runtime plugin isolation, archive replication, cross-collection deduplication, video,
emulators, ripping hardware, broader codecs/DSP, smart/personal lists and general
post-edit splitting. No hidden dependencies on these for the home-lab server.

### HL-13 — Advanced hardening and large-deployment qualification

**State:** deferred retained scope, not certification. **Maps:** S01/R02/R03 advanced
qualification. Preserve the original [security contract](decisions/security.md),
[broader target requirements](release-requirements.md#broader-product-target-reference)
and dated implementation evidence. The advanced scope is retained here;
there is no separate historical work queue.

Retain persistent/distributed rate limiting, exhaustive password-manager/platform/
hardware matrices, broad independent protocol/security/accessibility review,
100,000 Items/one million file-evidence records/five-user load on a recorded
four-core/16-GB/local-SSD host (warm browse/search <500 ms p95), full deployment
matrix and stronger power-loss/storage qualification. They are not automatic
home-lab blockers. Distributed queues/multi-server active writers are outside
current scope, not solutions to invent for a single server.

Promote low-effort, concrete hardening and any finding affecting exposed access,
credential safety, untrusted parsing, data preservation or recovery immediately.
A focused Internet-facing security review remains HL-04; deferring exhaustive
enterprise-scale assurance does not defer actual authentication/access/resource
safety. If a deployment needs restart-resistant limiting or stronger durability,
record that requirement and implement/qualify it for that deployment.

## Reassessment and source coverage

| Earlier work/source | Disposition in this queue |
| --- | --- |
| A01–A04, U01–U04 image/inventory/access, S01–S03, P01–P04, D01–D07 local implementation | Keep completed history; only concrete residual qualification/features above remain |
| LA1/LA2, FX-01–FX-07, work-context hardening, U05/P04 local interface audit | Completed locally; do not schedule a second implementation/audit wholesale; HL-02/04/06 retain external checks |
| BL1–BL3, CL1, P06, navigation/wording/preparation help, account software repairs, October game/DAT/toolchain TODOs | Delivered; evidence limitations mapped to HL-02/04/06/07/08 |
| LA3 installation and shared account/connection software | HL-02/04; retain relevant web/Trash Amp checks |
| PJ1 and Phono-specific account/GUI/physical-Mac acceptance | Retired target; preserve dated evidence, do not complete its remaining matrix |
| Previous P05 proposals | HL-03 full backup first; HL-09 export and optional deltas later |
| Earlier native-client proposals | Reassess against implemented Trash Amp; HL-02/07/10 residual work, no wholesale native rewrite |
| S04/C02 | HL-11; no artificial dependency on them for single-route playback/offline downloads |
| R01 | HL-09 only for actual legacy adoption |
| R02/R03 and release requirements | HL-04/05/06 practical release; HL-08 representative performance; HL-13 extreme matrix |
| U04 documents, D06 extra providers, FX-04 scale bounds, C06 | HL-08/10; optional measured demand |
| Shared listening and follow-on scope in old plans/status | HL-12; HL-08/10 where appropriate |
| Obsolete wave queues and music-first proposal | Removed; relevant unfinished scope and advanced design are consolidated here |
| Reviews, foundation/reuse/security decisions, UX review, validation ledgers and release notes | Technical/evidence references; HL-01 reassesses validity, complexity and retention; no independent assignment order |

Dropped as unnecessary scheduling requirements: restarting completed roots; serial
wave handoffs; checkpoint-only compatibility wrappers/schema reservations;
mandatory new Collector desktop app or legacy migration for every installation;
mandatory rotation/discovery before single-route use; a huge synthetic benchmark
before ordinary home-lab adoption. Data conversion/recovery for actual archives
and independent evidence for advertised claims remain required. Old proposed contracts
are subject to redesign; append-only history is an implemented mechanism, not an
unlimited future-retention requirement.

The disabled DAT-o-MATIC downloader stays disabled. Manual download/upload is
implemented; provider-access restoration or automated scraping is not an open task.

## Ready-to-send next assignment

> Reassess and simplify **HL-01** from `docs/remaining-work.md`. Read AGENTS.md,
> architecture, current status, release requirements and the home-lab scope policy.
> Inspect the actual consumers/storage; identify old proposed contracts without a
> useful user outcome and excessive metadata/artwork/provider/operational retention.
> Produce concrete minimal contracts and retention rules, with an explicit mapping
> of retained, bounded and obsolete mechanisms. Implement only a bounded, justified
> simplification with safe backup/conversion/recovery and relevant acceptance; do
> not invent a generic retention framework. Preserve originals, manual authority,
> meaningful evidence, unknown data and real outstanding retries. Do not prune the
> user's archive as an experiment or start Phono Junk work. Update the queue, owning
> ledgers, status and dated validation. Do not commit or spawn agents without an
> explicit user request. HL-02 LAN qualification and HL-03 backup tooling remain
> independent follow-ups when their required inputs are available.

Completed bug repair, 2026-10-09: AccurateRip check panels no longer default a
single-anchor disc to an unrelated collection response. Current recorded layout
supplies the full query for admission and response selection, including matches on
older pages and correction of stale browser preferences. Missing responses prompt
acquisition. The worker retains its strict selected-layout check; real owned-disc
qualification remains HL-05. See [status](implementation-status.md#accuraterip-response-selection-repair--2026-10-09)
and [validation](validation.md#accuraterip-response-selection-repair--2026-10-09).

Follow-up to that repair, 2026-10-09: missing-reference readiness keeps eligible
source files and has no empty-UUID error; imported layouts expose an actionable
fetch through server-owned structural eligibility. No recorded layout offers the
existing structure check. Acquisition keeps source/module/role and ambiguity guards.
See the same status and validation record above; HL-05 remains open.

Completed progress follow-up, 2026-10-09: active file checks report typed
indeterminate progress and actual source/computation/publication phases instead
of misleading zero completion. Development builds optimize shared audio loops.
Algorithm/coverage and live-owned-disc qualification are unchanged; see
[status](implementation-status.md#file-check-progress-and-development-audio-performance--2026-10-09)
and [validation](validation.md#file-check-progress--2026-10-09).
