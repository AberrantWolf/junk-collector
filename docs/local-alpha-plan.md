# Local alpha: import, enrich, browse, listen

Accepted direction 2026-09-16, after D06.3 commit `3a6143b`. **LA1 is reviewed and corrected
locally; acceptance passed. LA2 has not started.** This remains the
active execution plan. It supersedes the
serial Wave 4/P05.1 assignment and the earlier music-first scheduling proposal.
It does not mark unfinished work implemented or erase the broader release target.

## User outcome and scope

On a local server, select raw dump packages, add them to a typed collection,
automatically gather available metadata/artwork, browse the results, and play
supported music CDs. Multiple trusted local users use the existing accounts and
collection roles. Deliver the existing web UI first; native apps follow. Sources
are copied and retained, never moved or edited. Inventory without files remains valid.

Use explicit inbox selection and resumable browser uploads. Provide a preview of
package boundaries before acceptance. A package may be a cartridge file, a disc
image with companions, or a redumper/BIN+CUE set; directory and filename order are
not sufficient to infer a disc, album, physical copy or edition. Start with the
existing registered capabilities and extend only a demonstrated missing format
needed by the acceptance corpus. Unsupported packages can still be preserved.
Do not claim redumper playback merely because redumper analysis exists.

Automatic enrichment is the default for this new guided import action, with an
opt-out and visible provider prerequisites. Previously saved imports do not gain
new network side effects. CD lookup uses supported disc identity, then useful
provisional candidates; ambiguous candidates remain visibly tentative. Games use
configured DAT-backed identity/names. General online game descriptions/cover art
are not currently provided: uploaded/local supporting art and placeholders remain
available, and online game enrichment is an explicit follow-up, not a completed
alpha capability. No new external provider is required to begin LA1.

FLAC preparation remains optional: the user requests it during import or through
Prepare album. It is never triggered by browsing or a media GET. LA1 exposes only
implemented choices; LA2 adds the import-time preparation option. Playback must
not depend on MusicBrainz, artwork or AccurateRip success. Structural decode
eligibility and independent lossless validation still apply.

## Delivery ledger

| Delivery | State | Complete user outcome | Existing work to reuse/close |
|---|---|---|---|
| LA1 — Guided ingestion and automatic enrichment | Reviewed and corrected; acceptance passed | Select packages, import, leave the page, return to preserved entries with metadata/artwork or actionable partial outcomes | P03/P04, U05 readiness on this journey, D04–D06, P02 jobs; import history and safe correction/discard |
| LA2 — Album preparation and usable library | Ready for implementation; not started | Request whole albums, queue independent work, browse/play completed tracks while other work runs | D07.4–D07.5, existing browser player, remaining browse/music readiness defects |
| LA3 — Local installation qualification | Planned; after LA2 review | Actual owned dumps and two accounts work on the intended host, including recovery and backup restore | Focused U05/P04 journey review, local deployment/oracles, bounded backup drill |

These are integrated deliveries, not additional layers above all the old child
checkpoints. Implement application, adapter, API and UI changes together. One
boundary review at the beginning of a novel durable contract and one integrated
review at delivery completion are sufficient; routine internal steps need no
handoff. The LA1 design below is the starting contract, so do not stop after merely
rewriting it. Escalate a substantive unresolved boundary/product contradiction;
otherwise refine names against the checkout and continue through the full journey.

Preserve P04.4–P04.6, U05 and D07.4–D07.5 acceptance as the source of detailed
requirements. Track which criteria a delivery actually satisfies; do not mark an
entire old ticket complete based on partial alpha coverage. Full cross-interface
web completion remains distinct from acceptance of these alpha journeys. Do not
restart completed D06/D07 implementation or add a parallel importer/player.

## Ownership and contracts

The [component diagram](diagrams/local-alpha-components.puml) and
[execution sequence](diagrams/local-alpha-ingest.puml) describe the implemented
LA1 ownership and flow. PlantUML rendering is optional; typed contracts and tests
remain authoritative.

- Modules declare eligible package anchors, required companions and operations,
  using shared parsers. They return typed candidates/blockers; they do not schedule.
- Application services plan ingestion, authorize it, choose enrichment policy,
  preserve authority and coordinate durable progress. Keep this cohesive use case
  in a focused application module rather than growing the main lib.rs indefinitely.
- Store adapters persist private intent/step records and existing jobs, stage
  files, and publish through existing archive journals. Use existing worker leases,
  admission limits and recovery. No generic DAG engine or second queue.
- HTTP exposes generated typed commands/read views; React submits intent, renders
  progress and submits user decisions. No browser-owned publication/provider chain.
- Existing domain/archive data remains portable. User/actor bindings, scheduler
  state and attempts stay private. Preserve unknown module data and originals.

Start inspection with collector-app's provisional Import methods, processing.rs
and metadata.rs; collector-store's provisional_imports.rs, jobs.rs and processing.rs;
collector-server's workers/routes; web's import-workspace.tsx, metadata-review.tsx,
processing-workspace.tsx and music-tracks.tsx. Reuse their actual effects and tests.
Do not create speculative crates or production placeholder endpoints.

Settle the immediate cross-layer shape together. The following is semantic
pseudocode, **not compile-ready Rust or a mandated new public API**:

```rust
plan_ingest(actor, collection_id, package_selection) -> IngestPlan
// Per-package bounded inventory, eligible anchors/operations, default new-entry
// intent, supported existing-entry associations, optional provider availability,
// blocker/choice enums, plan revision/fingerprint. No arbitrary host paths.

start_ingest(actor, plan_token, decisions, enrichment, command_context) -> Accepted
// Accepted per-package ImportIds and typed rejected members; honest partial
// admission. Browser uploads become runnable only after all declared bytes arrive.

list_ingests(actor, collection_filter, cursor) -> Page<IngestSummary>
ingest_detail(actor, import_id) -> IngestDetail
// Package name, ImportId, target Item/Capture, phase, per-step status, attempts,
// current permissions, actionable blockers. No raw requests, credentials or paths.

resolve_ingest(actor, import_id, expected_revision, decision, context) -> IngestDetail
retry_ingest(actor, import_id, selected_failed_steps, context) -> Accepted
cancel_ingest(actor, import_id, context) -> IngestDetail
// Discard uses the P04 lifecycle: fence work, reclaim only uncommitted staging,
// report reclaimed/pending bytes, and preserve committed archive/history.
```

Reuse ImportId as the durable workflow handle. Extend the existing private Import
record or a narrow associated record when necessary; avoid another independent
lifecycle for the same package. Step records distinguish pending, active,
needs-input, succeeded, failed, skipped and cancelled with typed reasons. Track
preservation, identification, artwork and preparation independently: preservation
can succeed while enrichment fails. A summary must not label that full success
without exposing the partial outcome. Use bounded lists and streaming inventories.

A submitted plan is a proposal: authorize and revalidate source inventory,
collection owner, chosen association and revisions at acceptance. Preallocate new
Item/Capture/Representation identities before effects. Default to a new catalog
entry per selected package, without inventing physical-specimen ownership. Never
merge entries solely because content hashes match. Repeated acceptance of the same
command replays; deliberate duplicate specimens remain possible. Ambiguous disc
associations or package boundaries require a choice, while ambiguous descriptive
metadata can remain provisional without blocking preservation or playback.

## Durable coordinator pseudocode and failure rules

```text
accept(request, receipt):
  authorize; validate submitted shape and bind the receipt to those exact inputs
  replay matching success before resolving mutable configuration/source plans
  freeze intent, stable output identities and bounded admission results
  commit private intent + acceptance receipt atomically
  return accepted IDs immediately

resume(import):                         # existing bounded job infrastructure
  if bytes incomplete: expose resumable staging; do not start processing
  recheck current actor/account/grant eligibility for each new effect
  derive module-owned structural readiness from the exact staged inventory
  if association/package choice required: persist needs-input; return
  ensure stable target entry using the existing archive effect/recovery path
  preserve package through the existing Import/Capture publication job
  reconcile its committed result before scheduling downstream effects
  schedule eligible identification and requested preparation independently
  on identification result: apply provisional metadata via existing resolver
  on eligible artwork result: publish Asset and update automatic artwork authority
  persist each outcome; retain failed optional steps for selective retry
```

Each `ensure`/schedule operation uses stable Import/step/source identities. Persist
intent before dispatch; replay/reconcile existing effects after a crash between
publication and bookkeeping. A coordinator does not wrap independently committed
child commands in one pretend atomic receipt. Do not fabricate an HTTP Principal
for background work; use the existing submitted-actor authorization/job pattern.
Current grant loss blocks new writes; recovery still completes already committed
publication bookkeeping without destroying bytes. Test both cases.

Use exact source fingerprints and expected metadata revisions. A stale automatic
result must re-resolve against current manual authority or become an actionable
conflict; it cannot overwrite a newer edit. Filename/default text is explicitly
provisional and must not become a manual override. Existing user-entered titles
and artwork remain authoritative. Automatic CAA attachment needs an application
operation that binds the published Asset to the appropriate immutable candidate
or automatic choice; do not simulate the user's manual artwork selection. Reuse
the existing publication/resolver effects and retain provenance. Specify this
contract in the implementation notes before wiring it into the coordinator.

Cancellation stops future owned work and fences in-flight publication using the
existing commit boundary. After commit, report preserved content and cancel only
remaining optional work. Retry reuses valid results; it does not duplicate entries,
Captures, provider observations or derivatives. An optional provider outage never
rolls back preservation. Bound retries, CPU, network, file reads and queue admission;
do not busy-loop missing providers or serialize independent albums in the browser.

## LA1 acceptance: one real workflow

Implement this full delivery, including API/UI, before stopping for review:

1. Select several explicit inbox packages or upload a package from the collection
   page without first creating an empty Item. Preview boundaries, eligible inputs,
   destination and automatic-enrichment policy. Unique valid anchors are selected;
   missing companions and genuine ambiguity are explained before doomed jobs run.
2. Start importing; show identifiable entries and server-owned progress. Close the
   browser, reopen from a server-side import list, and resume incomplete uploads.
   Completed uploads and inbox work continue without an open browser.
3. For music fixtures, obtain provider candidates and original/thumbnail artwork
   automatically. For game fixtures, obtain configured DAT names/identity with
   appropriately scoped evidence. Useful provisional metadata appears without a
   mandatory review click. Browsing/search reflect results immediately.
4. A provider outage or no match leaves originals and an identifiable entry intact.
   Manual title/artwork changes made during lookup survive completion and retry.
   A generated filename title does not prevent later automatic identification.
5. History identifies package, Item/Capture and each retained attempt. Retry one
   failed step. Discard an accidental unpublished import with actual staging-byte
   reclamation; correct a mistaken published association without deleting originals.
   Reuse the P04 design for cleanup/restart races; no ad hoc file deletion.
6. Exercise receipt replay, changed input, restart around intent/job/publication
   boundaries, cancellation before/after commit, stale plans, path traversal,
   ambiguous packages, missing tools/providers and collection/account revocation.
   Assert stable IDs, one intended effect and no cross-user/collection access.

Keep extended bulk history management and the broader U05 audit mapped explicitly
as remaining acceptance if not implemented here. This is not a claim of complete
P04/U05 or all-format support. Use independent vectors for new parsing mechanisms;
use integration/contract tests for orchestration and actual browser workflows for UI.

### LA1 implementation result — 2026-09-16

The implementation uses `ImportId` plus a narrow private guided-intent record and
the existing staging, Job, processing, publication, metadata and Asset effects.
Typed plan/start/list/detail/retry/cancel/discard/association commands are exposed
through generated API contracts. The collection page accepts configured direct
inbox packages or resumable browser files, previews anchors/blockers/provider
availability, defaults enrichment on, and reopens active or terminal server-side
history. Paginated startup and periodic reconciliation close durable intent-to-job crash
windows without adding a scheduler.

Music identification can schedule MusicBrainz then Cover Art Archive and publish
automatic candidate artwork; configured DAT observations feed the existing game
metadata resolver. Generated package titles establish an inventory baseline but no
manual field override, and automatic artwork never sets manual artwork authority.
Optional-provider failure leaves the preservation master and entry intact. Published
association correction writes a new Capture revision; unpublished discard fences
work and reports actual reclaimed or pending staging bytes.

LA1 covers the focused P04/U05 journey, including bounded personal history,
attempts, reopen, one-step retry, cancellation, discard and association correction.
P04 bulk selection, search/filter, dismiss/archive/restore and the application-wide
U05 audit remain open. LA1 adds no preparation option, online game provider, native
client or all-format qualification. Deterministic local provider fixtures passed;
live MusicBrainz/CAA and user-owned dump qualification remain LA3 work. Review the
implementation and evidence in [local-alpha review](local-alpha-review.md) and
[validation](validation.md).

## LA2 acceptance: library and listening

Implement D07.4–D07.5 as one integrated delivery. Prepare album selects the supported
tracks across unambiguous associated discs, with explicit alternatives when needed.
Use durable bounded batch admission over existing track jobs, stable recipe/output
identities, accurate partial outcomes and cancellation ownership for shared jobs.
Add the optional import-time Prepare music checkbox using this same service.

During a deliberately slow encode, enqueue another track and album; browse, edit
unrelated records and play completed tracks. Close/reload/restart and recover all
accepted work. Retry unfinished members without re-encoding valid outputs. Test
multi-disc scope, overlapping requests, unavailable encoder, partial failure and
revocation. Show cover/text rows, ordered usable track labels, visible progress,
play/pause/seek/next/previous, and useful missing/unsupported states. Verify actual
FLAC playback and lossless validation; no fake player or assumed browser support.
Metadata/artwork/verification outages must not prevent supported audio preparation.

## LA3 acceptance: a usable installation

Use representative user-owned game dumps and music CDs (including the actual
redumper/BIN+CUE variants intended for use), copied into a test inbox. Ask for
paths/formats when unavailable; do not substitute synthetic fixtures for that claim.
Record host, browser, tools, formats and limitations. Exercise real MusicBrainz/CAA
configuration and responses while retaining deterministic outage tests.

Configure two local accounts with explicit collection roles; test second-device
LAN access, denial, session expiry/revocation and a viewer playing completed media.
Use existing encrypted/private ingress and browser trust guidance. Local-only is a
deployment choice, not anonymous access, weakened transport, or source-IP auth.
No public tunnel setup, discovery or new native-client trust work is required.

Provide and execute a stopped-server backup/restore runbook covering archive,
private state, config/TLS and pending-work scratch, with quiescence/locks verified.
Restore into separate empty locations, verify original/metadata/evidence digests,
accounts and resumed work; never run old and restored identities concurrently.
This bounded drill is sufficient for alpha; it does not claim P05 portable export,
deltas or complete backup tooling. Identify all roots before copying; never copy
an active SQLite database file alone. A failed drill blocks archive adoption.

Walk import → automatic metadata → browse → album preparation → playback using
those dumps and two users, including an interrupted import and an optional-provider
failure. Fix journey blockers in their owning layers. Record residual limits without
claiming all web surfaces or the full release qualified. Checkpoint the usable result.

## Deferred from alpha, preserved in the release backlog

P05 portable export/change bundles and full backup UI; native Slint/SwiftUI and
Trash Amp integration; offline caches; automatic route discovery/failover and
certificate rotation; public deployment and passkey-platform qualification;
additional online game metadata providers; full Retro Junk archive/evidence migration;
scale qualification, runtime plugins, video and emulation. Importing raw files from
an old archive does not claim migration of its IDs, history or manual metadata.

Do not broaden alpha by completing these in order. Do not remove existing working
security or typed boundaries to accelerate it. Keep the original full-release
requirements as the destination; alpha introduces a smaller useful delivery gate.

## Agent handoff and review method

One focused packet per delivery: this plan, relevant contracts/code and its explicit
acceptance cases. Keep diagrams small and update them when ownership changes.
Typed production interfaces and executable tests supersede illustrative pseudocode;
do not maintain a second detailed implementation in documentation. Measure total
implementation plus review/repair effort and successful user journeys, not number
of checkpoints. No promise of cheaper-model savings without measurements.

Use targeted tests while editing, then the required repository checks once at the
integrated delivery boundary. Repeat affected checks for fixes; do not rerun every
expensive suite after prose changes. Record exact results in validation.md. Update
this ledger, implementation status, mapped old tickets and the next prompt together.
No automatic commits; recommend backup checkpoints after working deliveries.

## Ready-to-send LA1 review prompt

Completed review assignment; use the LA2 implementation prompt below next.

```text
Review the integrated LA1 implementation against docs/local-alpha-plan.md and
docs/local-alpha-review.md. Read AGENTS.md, architecture.md,
implementation-status.md, release-requirements.md and the mapped P04/U05 criteria.
Do not begin LA2, P05, native clients, broad P04/U05 completion or a new provider.

Inspect the diff and trace configured-inbox and resumable-browser intake through
intent acceptance, stable identities, existing Jobs, publication, metadata and
Asset authority. Challenge restart/retry/cancellation, receipt binding, stale plans,
grant loss, optional-provider failure, manual title/artwork conflicts, cleanup races,
history projection and published association correction. Verify automatic metadata
is never described as verification and preservation remains independent.

Run focused regressions and the repository gates, rebuild embedded assets, and run
the real temporary-profile browser journey. Record findings and corrections in
docs/local-alpha-review.md and docs/validation.md. Keep live-provider and owned-dump
qualification explicitly unrun unless those inputs are actually supplied. Stop
after LA1 is reviewed and corrected; update the next prompt to LA2 only if review
acceptance passes. Do not commit or spawn agents.
```

## Ready-to-send LA2 implementation prompt

```text
Implement LA2 from docs/local-alpha-plan.md as one complete delivery: optional album
preparation and a usable browser library/player. Read AGENTS.md, architecture.md,
implementation-status.md, release-requirements.md, local-alpha-review.md and
la1-implementation-notes.md first, then D07.4–D07.5 in music-first-milestones.md.
Begin only once the LA1 integrated review is accepted in the delivery ledger.

Reuse the existing guided import, durable Jobs, FLAC recipe/validation, availability
queries and browser player. Application services own album scope, stable batch/member
identities, bounded admission, partial failure, retry and cancellation of owned work.
Expose the same preparation service from Prepare album and the optional import-time
checkbox. Keep UI selection/progress below that policy; do not build a second queue,
provider pipeline or encoder. Preserve masters and manual metadata authority.

Complete the backend, generated API, UI and end-to-end acceptance together. While an
encode runs, another album/track can be queued and users can browse, edit unrelated
items and play completed tracks. Recover accepted work after close/reload/restart;
reuse valid outputs. Cover multidisc ambiguity, overlap/shared jobs, missing encoders,
partial failure, cancellation and revocation. Keep cover art beside text rows.

Use focused regression tests while implementing, then all repository gates and real
temporary-profile browser playback. Record exact evidence and limitations; update the
alpha ledger, status, mapped D07 tickets and review handoff together. Keep LA3 owned
dumps/live-provider/deployment/backup qualification open. Do not implement P05, native
clients, public deployment or broader P04/U05 work. Continue through the whole LA2
delivery without routine checkpoint handoffs. Do not commit or spawn agents.
```
