# Module-owned collection types: implementation handoff

**Current direction (2026-10-09):** [remaining work](remaining-work.md)
is the only remaining-work queue. Older assignment/order and gap statements below
are historical. Proposed contracts require reassessment under
[the home-lab scope/retention policy](decisions/home-lab-scope.md); their presence
does not require implementing speculative machinery or permanent lookup history.
Phono Junk is no longer a target. Implemented contracts describe current behavior,
not a claim that cleanup or redesign has already happened.

Status: **CT.1–CT.4 reviewed and corrected; resume Wave 4 at D04.1**. This is a prerequisite
inserted after reviewed W4.2, not a redo of W4.0–W4.2. Target executor: Sol at medium
reasoning. No runtime plugins, new analyzers or sibling extractions in this task.

Read AGENTS.md, architecture, implementation status, release requirements,
[pre-release evolution](decisions/pre-release-evolution.md), and the relevant
Wave 4 contracts first. Inspect the actual dirty tree: W4.0–W4.2 and review fixes
are currently uncommitted. Preserve them and all user data. This document specifies
new target behavior; it does not claim that behavior exists today.

## Outcome and fixed decisions

A collection has exactly one owning collection module. Initially use the existing
`org.junk.games` and `org.junk.music` module identities, not a new Games/Music enum
in core. One module supplies one collection type for now; do not build a separate
plugin/type registry or prematurely model multiple types per plugin.

The owner defines domain fields, validation, identification, operation results,
metadata authority and presentation contributions. The shared foundation retains
identity, inventory relationships, ownership/possession, files, captures, assets,
revisions, grants and preservation evidence envelopes. A collection's type is
immutable after creation; this task offers no ordinary change-type or move-item API.

A game item can contain a manual, soundtrack carrier, scan or bonus video. Its
collection and item domain remain games. Media format is not collection type.
Do not gate import by extension, MIME type, carrier kind, identified title or file
presence. Unidentified captures, inventory without files, wishlist entries and
multiple physical copies remain valid. Cross-type aggregate browse is a read view,
not a mixed collection. Preserve existing aggregate browsing; no new aggregate UI
is required if the current UI has none.

Unavailable owner modules remain portable: show safe generic read-only records,
artwork and existing authorized downloads. Disable collection-content mutations
and new domain jobs, including shared inventory edits, while unavailable. Access
administration and personal activity retain their independent authorization rules.
Reindex/export and post-commit recovery must not require an installed module.
Do not expose raw private payloads in a generic viewer or execute plugin code.

## Current seams to inspect

- Core `src/lib.rs`: Collection has no module; Item has `domain: ModuleData`;
  Work/Edition and CollectionGraph carry shared identity relationships.
- Core `processing.rs`: generic preservation contracts are mixed with
  GameProcessingResult, GameFileDigest, AudioVerificationResult, CdProcessingResult.
- Core `metadata.rs`: MetadataField hard-codes Artist and Platform.
- Modules `lib.rs`, `games.rs`, `music.rs`: registry/host contracts and typed
  GameEdition/MusicEdition already exist. They currently validate item payloads;
  naming must not imply these are canonical Edition records if they are not.
- App `lib.rs`, `processing.rs`: create_collection, PresentedCollection,
  SaveItemCommand, Work/Edition/Capture mutations, imports, operation enqueue/run,
  metadata/recipe ports and module presentation. Read adapters before changing them.
- Store `lib.rs`, `archive_v3.rs`, `processing.rs`, `provisional_imports.rs`,
  `jobs.rs`, `collection_recovery.rs` if present: actual manifest readers,
  creation recovery, projections, import publication and frozen jobs.
- Server `lib.rs`, `main.rs`; web `administration.tsx`, `inventory-editor.tsx`,
  `curation.tsx`, `import-workspace.tsx`, `app.tsx`; generated API and smoke fixtures.

Use rg to resolve names/locations rather than inventing files or obsolete methods.

## Ownership contract

| Meaning | Owner after change |
|---|---|
| Collection owner identity, envelopes and graph identity invariants | core |
| Generic fingerprints, observations, snapshots and source rebinding | existing shared contracts |
| Game normalization/match policy and game result payloads | modules::games |
| AccurateRip outcome/confidence/checksum policy, music result payloads | modules::music |
| PCM/CD coordinates, track layout and PCM/conversion settings genuinely shared by media operations | focused core media contracts, without collection policy |
| Generic immutable conversion/source binding | shared processing; do not force all audio into music collections |
| Selected metadata candidate, common title/subtitle authority, artwork authority | common metadata contract |
| Artist/platform and future domain overrides | owner's typed metadata payload |
| Authorization, owner availability, operation eligibility | application services using registry facts |
| Persistence, structural owner-ID consistency, publication and reindex | store/core mechanisms; no registry dependency in store |
| Editable field descriptions and row contributions | module; rendered by generic presentation adapters |

Retain exact serialized result bodies and existing hash/recipe encodings when
moving Rust types. A Rust module move is not a reason to rewrite archive history.
Keep module-specific data in lossless versioned envelopes; typed module views and
commands validate semantics. Do not replace domain types with BTreeMap<String,
String>, a universal collection-kind enum, arbitrary user JSON editing, or a full
schema interpreter. Core must not import concrete modules.

For metadata, keep common authority separate from an optional owner-bound
ModuleData envelope of typed domain overrides. Game/music own their override
structs; validate its module ID against the collection. Keep unknown legacy/raw
values as preserved provenance, never apply them as active typed overrides.
Candidate provider identity may differ from collection owner: MusicBrainz is a
provider, not a collection type. Likewise an observation's implementation module
need not equal the collection owner; distinguish domain operations from shared
host/media services explicitly in registry/application eligibility.

## Execution and checkpoints

| Child | Deliverable | Stop |
|---|---|---|
| CT.1 | Typed ownership and module contract refactor | Review before persistence |
| CT.2 | Creation, legacy assignment and durable invariants | Review before UI |
| CT.3 | All application mutation/job boundaries | Routine, finish listed regressions |
| CT.4 | Web/API integration and third-module proof | Final review before D04.1 |

Complete one child at a time. Stop at the stated review checkpoints; do not run
all four without review. Within a child, fix ordinary issues autonomously. If a
required contract conflicts with code, revise its owning plan explicitly instead
of adding an adapter that hides the discrepancy. No commits or subagents unless
requested. No reset of the user's actual archive/state for testing.

### CT.1 — Ownership and module contracts

1. Add a collection binding type in core: `Assigned { module: ModuleId }` or
   `LegacyUnassigned`. The latter is exclusively a read/recovery state for existing
   collections, never a valid typed create request. Define the binding and typed
   creation contract now; integrate the field into persisted Collection and its
   presented DTOs in CT.2 so this checkpoint does not half-change runtime creation.
   Do not default new collections to games or music.
2. Extend module descriptors with safe collection-type labels and bounded editable
   field descriptions. Keep domain validation in the module. For this child, cover
   current editable scalar domain fields only: optional bounded text and optional
   bounded integer (release_year). Descriptors contain stable field keys, labels,
   bounds and requiredness; never scripts, HTML, URLs to executable code or SQL.
   Unsupported structured fields (e.g. existing track arrays) remain preserved and
   read-only, explicitly described as such; a scalar edit must not overwrite them.
   Do not add dynamic field types until a concrete consumer needs them.
3. Define an owner-mediated domain edit command/patch. Omission preserves a field;
   explicit null clears an optional field. Known edits deserialize into module-owned
   typed patches; reject unknown editable keys and invalid scalar types. Retain
   unedited raw/unknown data rather than reserializing a lossy typed subset.
   This replaces client replacement of an entire arbitrary domain envelope.
4. Move concrete game/music processing types to their owners per the table above;
   update callers/tests together. Consolidate CD/PCM units only if semantics match
   exactly; preserve signed offsets and integer coordinates. Do not extract into
   junk-libs or add new algorithm implementations in this task.
5. Remove Artist/Platform from the universal override enum and add typed owner
   override structs/envelope as above. Update future D06 references. Preserve
   common title/artwork behavior and raw authority observations.
6. Make operation result validation an explicit owner hook invoked by registry
   dispatch before success publication. Do not use Item's validate_current for
   unrelated result schemas. Available handlers require a real validator; an
   absent handler/validator is unavailable, not a default successful no-op.
   The test-only third module provides its own result validator. Shared operations
   have explicit scope/eligibility; do not authorize by capability string prefix.

**Tests:** typed binding serde; invalid IDs; third module registers without core
changes; numeric form values remain numeric; null versus omission; unknown raw
field/track preservation; malformed operation output rejected; moved golden
fingerprints/recipe digests unchanged; domain overrides reject wrong owner.
**Review report:** moved-type table, dependency graph, proposed serialized binding,
edit/result contracts and real test evidence. Do not claim homogeneous runtime
collections yet. Compilation changes to consumers are allowed; production behavior
lands in the following children.

#### CT.1 implementation review checkpoint — 2026-09-11

CT.1 is reviewed and corrected; both reported regressions have permanent tests. The runtime `Collection`,
creation service, archive manifests, catalog projections, presented DTOs, HTTP and
React flows are deliberately unchanged; collections are not homogeneous yet and
`LegacyUnassigned` is not accepted by creation. CT.2 integration is recorded below.

Moved ownership:

| Contract | Previous owner | CT.1 owner |
|---|---|---|
| `GameProcessingResult`, `GameFileDigest`, `GameStructureValue`, game normalization and catalog-match result types | `collector-core::processing` | `collector-modules::games` |
| `CdProcessingResult`, AccurateRip status/checksum/confidence and per-track verification types | `collector-core::processing` | `collector-modules::music` |
| Signed sample-frame offsets, CD sectors, PCM frames and shared CD track layout | `collector-core::processing` | unchanged shared core media contracts |
| Common title/subtitle overrides and artwork choice | `collector-core::metadata` | unchanged common authority contract |
| Platform and artist overrides | universal core field enum | typed `GameMetadataOverrides` / `MusicMetadataOverrides` envelopes validated by their owner |

The dependency direction is `collector-core` ← `collector-modules` ←
`collector-app` ← `collector-store`/`collector-server`; core has no concrete-module import. The
proposed durable binding is exactly `{"state":"assigned","module":"org.junk.games"}`
or `{"state":"legacy_unassigned"}`. Deserialization validates the assigned ID and
rejects a module on the legacy state. `CreateCollectionCommand` requires a module
identity; CT.2 now integrates it into runtime creation.

Descriptors now carry a bounded collection-type label, at most 32 validated field
descriptions, and explicit operation scope. Game/music scalar fields are optional
typed text/integer fields; music tracks are declared structured and read-only.
`DomainEdit` is owner-bound and contains a bounded raw JSON object. Module-owned
typed patches distinguish omitted, null and typed values, reject unknown keys and
wrong scalar types, and rewrite only named values while retaining untouched raw
structured/unknown values. Operation dispatch validates every successful result
through a required owner hook before returning it for publication; an absent
handler remains unavailable. Metadata domain authority likewise requires the
collection owner and its typed module validator.

Evidence: `cargo test --workspace --locked` passed 178 tests. New regressions cover
binding wire validation, third-module registration/dispatch, bounded descriptors,
numeric edits, null versus omission, rejection of unknown/wrong-typed fields,
lexical preservation of untouched track/unknown values, wrong-owner metadata,
malformed successful operation output, and unchanged game/music result wire forms
including signed offsets. Existing source-fingerprint and recipe golden digests
remain unchanged. Also passed: architecture/document/fixture checks, formatting,
warning-free all-target Clippy, TypeScript checking, generated API drift checking,
web build and a fresh `collector-server` build. Browser smoke is not applicable to
this contract-only child because no route, DTO, web asset or runtime workflow changed.

### CT.2 — Persistence and existing collections

1. Add the binding to Collection and presented collection DTOs. Require a
   registered, available module in the typed CreateCollectionCommand.
   Include module identity in the command receipt digest and durable collection
   creation/recovery intent. Retry/restart returns the same typed collection and
   owner grant; same key with another type conflicts. Existing creation
   authorization/listener placement stays intact.
2. New collection manifests persist Assigned binding. Handle legacy manifests with
   no binding as LegacyUnassigned, without rewriting original files at startup.
   Such collections remain readable but content-read-only. Build a read-only
   administrator CLI inspection report using application services: count current
   item domain IDs, state eligibility for assignment and explain unresolved data.
   Do not guess type from a collection name, files or historic provider identities.
3. Provide explicit local-admin assignment for a LegacyUnassigned collection only.
   Require current revision and a chosen registered available module. Assign only
   if every current Item's domain belongs to it (including tombstoned items for
   conservative history handling). With no items, require explicit choice. Unknown
   module data is not silently coerced. Validate pending imports/jobs and known
   current authority envelopes; active pre-commit incompatible jobs block assignment
   with actionable IDs. Committed recovery completes first.
   Inspect `preserved_field_overrides` when planning assignment: they retain old
   artist/platform fields and language verbatim, not active domain authority.
   Report unresolved entries and preserve them; never silently discard language
   or activate them under a guessed owner. Domain conversion needs an explicit
   typed mapping and must respect existing active manual overrides.
4. Mixed collections remain LegacyUnassigned and read-only, with per-type counts
   and an explanation that splitting/moving their graph requires a separate task.
   Do not auto-split, rewrite ownership, delete records or invent a misc module.
   Implementing the generic graph split is explicitly out of scope. This is a
   migration holding state, not an option offered for new collections.
5. Persist assignment as a journaled collection metadata revision/sidecar plus
   projection update, preserving the original creation manifest and assignment
   provenance. Reuse the existing publication mechanism and optimistic revisions;
   do not mutate collection.toml in place or create another journal system.
   Readers select the latest valid binding; the directory name is not identity.
   Use one current format/conversion path, not a version per CT child.
6. Enforce generic ID consistency for current Item domain versus assigned owner in
   graph validation, normal publication and reindex. Store requires no concrete
   module to preserve/read records. Keep old envelopes/history exactly; an older
   observation/provider implementation ID does not make a collection mixed.

**Tests first:** game/music/third-module creation; same-key/different-type conflict;
crash at collection/grant and assignment commit points; archive-only binding
rebuild without grants; renames; empty/single-type/mixed/unknown legacy collections;
original files/history/private state unchanged; rejection leaves assignment and
projection unchanged; repeat conversion and restart are idempotent; corruption
cannot partially replace the catalog. Use synthetic fixtures, not real user data.

#### CT.2 implementation review checkpoint — 2026-09-11

Reviewed and corrected locally. `Collection`
and its presented DTO now carry the validated binding. Typed creation requires a
registered module, includes it in idempotency and durable recovery identity, and
persists it in new creation manifests. Missing historical bindings deserialize as
`LegacyUnassigned` without rewriting `collection.toml`; those collections remain
readable and reject graph publication. Complete application mutation/job gating
remains CT.3; this checkpoint does not claim every content entry point is gated.

The local `inspect-collection-type` and `assign-collection-type` commands cross a
narrow application service. Inspection reports exact current item-domain counts,
eligible registered modules, pending import/job and active-authority blockers, and
preserved historical override entries without activating or discarding them.
Assignment requires an explicit module and expected revision, accepts only a
homogeneous or empty legacy graph, and publishes one provenance-bearing schema-3
`collection_metadata` sidecar through the existing publication journal. Reindex
selects that validated sidecar by collection identity, not directory name; the
original manifest, item history and private grants are not rewritten.

Synthetic regressions cover game, music and a third registered module; type-bound
idempotency; all publication fault points; archive-only reconstruction without
grants; directory rename; read-only legacy behavior; empty, single, mixed and
unknown inspection; pending/authority/preserved-data reporting; rejected conversion;
repeat conversion; unchanged original bytes; and corrupt sidecar failure. The
initial implementation passed the locked workspace suite with 187 tests, along with architecture/fixture checks,
formatting, warning-free all-target Clippy, generated API drift checking, TypeScript,
web build and a fresh server build. No sibling checkout, real user data, commit,
graph split, domain coercion or CT.3 mutation/job-policy work was performed.

Review corrections validate full active metadata authority through the owning module,
including its schema and payload. The mechanism-only store snapshot retains those
facts; policy lives in `collector-app/src/collection_types.rs`. Shared-media
operations do not claim collection ownership, unknown active jobs block assignment,
and only explicitly selected import candidates constrain ownership. Same-owner
assignment retries require the original expected revision. Historical overrides
remain preserved and inactive.

The CLI opens the store exclusively with the server stopped. Inspection/assignment
are local administrative operations, not authenticated remote application APIs.
Do not expose this service over HTTP or run it alongside workers without adding
an authorization boundary and an atomic eligibility/commit fence.

A minimal web collection-type picker now uses the existing feature list and sends
an explicit module, restoring creation under the changed API. CT.4 must reuse it
when introducing authenticated descriptors and complete the remaining item forms.
Review validation: all 190 locked workspace tests, strict Clippy, architecture/fixture,
formatting, API drift, TypeScript/build and server rebuild pass. Both HTTPS/WebAuthn
and private local-setup browser smoke pass; see `validation.md` for coverage limits.

At the CT.2 checkpoint, CT.3 was next: centralize every content mutation/job
eligibility check and distinguish inspection reads from writes. Do not simply gate
all `Curate` authorization calls: existing provisional-import inspection also uses
that permission. Keep authorized legacy reads available. Its local implementation
is recorded below; D04.1 remains blocked until CT.3 and CT.4 pass review.

### CT.3 — Application policy and jobs

1. Centralize collection content-write eligibility: authorization, Assigned binding,
   installed/usable owner and supported schema. Use it from item/Work/Edition/
   specimen/loan/asset/Capture edits, import staging/review/publication, processing
   and metadata/recipe entry points. Do not duplicate owner checks in HTTP/React.
   Audit legacy compatibility endpoints as well as focused commands.
2. New Item domain is determined by collection; reject a conflicting submitted module
   identity. Work/Edition/Capture inherit collection ownership through their existing
   collection_id; do not add redundant mutable kind fields to every entity.
   Reject cross-collection associations as today. Ordinary edits cannot retype items.
3. Imports retain chosen collection type while unidentified. Filter domain analysis
   choices and validate at enqueue and execution. Shared integrity/media operations
   are permitted by explicit capabilities, not by pretending they own a collection.
   A game soundtrack can be preserved without becoming a music inventory item.
4. Queued jobs must recheck the current binding and registry ownership before doing
   new work. Freeze the binding where necessary in durable input. Binding/module
   changes or unavailable modules cannot publish new incompatible results. Once
   committed, recovery finishes independently of module availability and grants;
   future reads still check grants. Do not discard raw results or retry silently
   as another operation.
5. Generic fallback exposes only safe role-filtered presentation. Unavailable owners
   disable content actions but do not break authorized existing media/artwork access,
   personal activity, backup/reindex or access administration. Provider IDs and
   opaque extension provenance are not new collection types.

**Tests:** each mutation family denies wrong owner/unassigned/unavailable state;
viewer and cross-collection denial; disabled public curation/admin APIs stay
unreachable; forged module/request bypass; module disappearance while queued;
post-commit recovery with missing module; unidentified/multidisc/wishlist cases;
game with soundtrack/manual/scan; numeric/structured/raw metadata preserved.

#### CT.3 local implementation checkpoint — 2026-09-11

Reviewed and corrected locally; CT.4 has not started. `CollectionService` now
owns one content-write eligibility path over a mechanism-only Collection lookup.
It requires Curate authorization, an Assigned binding, an installed owner and the
owner's current supported schema. Item, compatibility Item, physical inventory,
loan, Work, Edition, Capture, Asset, legacy upload, provisional Import, integrity,
catalog-acquisition and processing entry points use that policy. Provisional Import
inspection remains a read and therefore does not accidentally become unavailable
merely because its owner is missing. Access administration, personal activity,
existing authorized media/Artwork reads, reindex and recovery retain their separate
policies.

New and edited Item domain envelopes must match the collection owner and current
schema. Import analysis is restricted to the owning collection-domain capability;
selected candidates are checked again at review and publication. Other processing
accepts a non-owner only when the registry declares the capability SharedMedia.
Provider identities remain provider identities. Frozen processing and acquisition
jobs include the exact Assigned binding; enqueue and execution both validate scope,
owner, schema/version and current binding. Old frozen inputs without this additive
field deserialize into an explicit legacy holding value and fail before new work.
Workers recheck grants and eligibility before work and again before publication.
Already committed snapshots/observations and revision-committed imports/integrity
work reconcile before those availability checks, so recovery does not depend on a
module or grant returning.

Permanent regressions cover wrong owner and unsupported schema, LegacyUnassigned
read-only behavior across Item/inventory/Import families, retained activity and
access administration, unavailable-owner mutation denial with Artwork reads still
working, owner disappearance after queueing, SharedMedia execution in a differently
owned collection, owner-filtered import analysis, wishlist/unidentified operation,
and post-commit snapshot recovery without the owner module. Existing HTTP tests
continue to cover viewer/cross-collection denials and public listener composition.
The currently mechanism-only metadata-authority/recipe publishers gained no
placeholder production API; D06/D07 must route their real application producers
through this same policy.

CT.3 review corrections: collection eligibility helpers now live beside assignment
policy in `collector-app/src/collection_types.rs`. Current stored Item schemas are
validated before ordinary and compatibility edits, so a request cannot downgrade
unsupported preserved data. Upload cancellation remains available independently of
module availability. Long import workers supply an application-policy callback at
the common Capture commit boundary; denial aborts only provisional publication
copies and preserves sources for retry. Integrity jobs recheck eligibility during
progress and before publication, while revision-committed recovery never rehashes
payloads. These callbacks do not claim a serializable transaction with concurrent
grant edits; they provide the required late policy recheck before publication.

Review validation: 197 locked workspace tests, strict all-target Clippy, architecture/
fixture/document checks, formatting, generated API drift, TypeScript/build and server
rebuild pass. Both normal HTTPS/WebAuthn and private local-setup browser flows pass.
See `validation.md` for the corrected smoke assertion and remaining coverage limits.

CT.4 must compute availability-aware content permissions for the UI, not expose the
current role-only flags as proof that content actions are executable. Preserve
independent read, personal activity, cancellation and access-management behavior.
Reuse the centralized helpers; do not copy their policy into React or HTTP.

### CT.4 — User-facing integration and completion

1. Expose safe registered collection-type descriptors through an authenticated client
   read API/application query; it does not expose configuration or enable management.
   Reuse the minimal CT.2 creation picker, replacing its feature-list source with
   this descriptor query. Creation requires choosing a type. Return binding and server-computed content
   permissions/availability in collection DTOs; do not make React infer authorization.
2. Remove per-item game/music selection. Item creation inherits collection owner;
   switching destination clears incompatible unsaved domain edits explicitly.
   Render module-declared scalar fields and labels without game/music ID switches.
   Existing structured fields are visibly read-only and preserved. Module validators
   remain authoritative for forged requests; the UI is not a schema security layer.
3. Browse/search rows keep artwork beside text and use safe module-supplied labels
   and summaries. Unknown-module and unassigned notices differ from missing artwork
   or uncertain identification. Keep Add versus Save wording. Aggregate results
   use each row's owning type; filtering does not change the underlying collection.
4. Legacy notices point administrators to the exact inspection/assignment CLI
   instructions. No automatic upgrade or user data reset on page load. No need for
   a polished migration wizard in this task.
5. Extend a test-only third collection module with a distinct field set and validator.
   Prove create/edit/browse/reindex through real services; exercise its descriptor
   with the generic web renderer using a test fixture. Do not register it in
   production. Adding it must not require a core type enum or frontend module-ID
   branch. Reject malformed descriptors at registration; bound keys/text/counts.
6. Regenerate OpenAPI/TypeScript. Update architecture, release requirements,
   implementation status, validation, backlog and Wave 4 plan with actual results.
   D04/D05 use module result owners; D06 uses module metadata authority; D07 keeps
   shared audio/media usable independently of collection type. C01/C03 carry type
   descriptors and fallback DTOs; offline caches retain binding and artwork. R01
   must assign imported archives explicitly, not recreate mixed collections.

**Checks:** architecture checker, cargo fmt, cargo test --workspace --locked,
all-target Clippy -D warnings, web check/check:api/build, then rebuild server.
Restore unavailable-owner browser fallback coverage using an archived collection
whose owning module is absent, not a forged mixed-type write into a music collection.
CT.2 replaced that invalid old smoke fixture with a wrong-owner rejection assertion.
Run browser smoke for new creation/edit/fallback flows with temporary state and
CHROME_PATH if needed. Document real manual steps: create one games and one music
collection, see different fields, reject wrong-type API edits, preserve a game
soundtrack, inspect a legacy mixed collection without changing it.

**Final review:** no new domain policy in core/store/frontend; no data deletion;
no universal per-kind enum, replacement JSON editor, fake capability or speculative
runtime plugin loader. CT.1–CT.4 complete locally and reviewed is required before
D04.1 resumes. Do not treat missing browser/external checks as passed.

#### CT.4 local implementation checkpoint — 2026-09-13

Reviewed and corrected locally. Authenticated clients now obtain the
actual registry's bounded `ModuleDescriptor` values from `/api/v1/collection-types`;
the unauthenticated feature probe no longer carries the module registry. Collection
and Item DTOs include application-computed content permissions and availability.
The legacy state includes exact stopped-server inspection/assignment commands,
while missing-owner and unsupported-item states remain distinct and read-only.
Role authority for access management remains independent.

The React creation picker consumes the authenticated descriptors. Generic Item
forms inherit the selected collection owner, render declared text/integer fields,
show structured fields as preserved read-only data, and submit an owner-bound
`DomainEdit`. Omission preserves existing values and an emptied scalar emits null.
Switching a new Item's destination clears incompatible draft fields. Browse rows
use descriptor labels and safe module presentations without game/music module-ID
branches; unavailable type, missing artwork and uncertain identity remain distinct.

A test-only postage-stamp module with required numeric/text fields, a structured
read-only field and its own validator passes authenticated descriptor, create,
owner-mediated edit, browse, malformed edit rejection and reindex tests without a
core enum or production registration. Descriptor registration rejects bad keys,
unsafe labels, excessive counts and ambiguous capabilities. Normal and private-local
Chromium smoke create separate music/game collections, exercise both generic form
sets and destination switching, reject a forged wrong-owner write, and rebuild a
temporary homogeneous archive whose assigned owner is absent to prove the browser
fallback without publishing a mixed graph.

Validation: 198 locked workspace tests; warning-free all-target Clippy; formatting;
architecture, link and Wave 4 fixture checks; generated OpenAPI/TypeScript drift;
TypeScript; web build; rebuilt embedded-assets server; normal HTTPS/virtual-WebAuthn
and `LOCAL_SETUP=1` Chromium smoke. The first browser iterations found and corrected
a two-collection mobile intrinsic-width regression and fixture assumptions about
collection order/schema-3 envelopes. No user archive, sibling checkout or commit was
modified. The initial implementation evidence above precedes the review corrections below.

CT.4 review corrections bind patch receipts to the submitted command, not its
resolved domain payload. Retrying an earlier patch after another edit returns its
original result without changing newer data; a different patch cannot reuse the key.
Permission/descriptor presentation helpers now live in the application collection-
types module. Descriptor validation also bounds capability counts and unavailable
reason text, with atomic failed-registration regressions.

The web form honors both Item and Collection content permissions while leaving
access administration available. Missing descriptor lists do not disable account or
access management. New items default to the browsed writable collection; changing
destination visibly explains clearing unsaved type-specific fields. Integer edits
reject values outside JavaScript's safe whole-number range rather than silently
rounding them; untouched preserved fields are not resubmitted. Browser fixtures
verify numeric third-module submission and unsupported-schema read-only controls.

Review validation: 200 locked workspace tests, strict Clippy, architecture/fixture/
document checks, formatting, API drift, TypeScript/build and server rebuild pass.
Both final HTTPS/WebAuthn and private local-setup browser suites pass.

CT.1–CT.4 are complete locally. D04.1 subsequently implemented its bounded
reader/result primitives and is awaiting review; follow its
reader-only extraction and independent-vector gates. Preserve module ownership,
patch receipts, safe fallback DTOs, artwork rows and the late publication-policy
callbacks. Do not introduce a checkpoint-specific schema version or compatibility
layer without an actual persisted-data requirement.
