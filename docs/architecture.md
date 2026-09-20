# Architecture

Pre-release evolution follows [the accepted compatibility and contract policy](decisions/pre-release-evolution.md). Earlier checkpoint compatibility is not a release requirement; data preservation and recovery remain required.

Junk Collector preserves evidence and owned collection history independently of how content is consumed. The server is a modular monolith: one process composes independent domain, application, storage, authentication, HTTP, and job capabilities. It is not a set of microservices.

## Local-alpha guided ingestion

The accepted [local-alpha plan](local-alpha-plan.md) schedules complete ingestion,
enrichment and listening journeys before native/public deployment work. LA1 is
implemented locally: application services coordinate existing Import, Job,
metadata, Asset and publication effects using a durable private intent and stable
output identities; modules own package interpretation, and adapters own I/O. No
browser-owned pipeline or generic workflow engine was added.
Automatic metadata protects manual authority; filename-derived defaults remain
provisional. Optional provider/preparation outcomes are independent of preservation.
Existing low-level/manual imports retain their behavior; only the new guided import
intent requests automatic enrichment. Optional preparation is LA2 work.
Module-owned readiness describes name-level anchor/companion constraints; staged
structural analysis validates actual contents. Failed analysis retains the package
for unidentified preservation. Worker reconciliation paginates all unsettled intents
on startup and every five seconds, after durable job outcomes are recorded.
Cancellation fences job admission in the same private-state transaction; discard
records its receipt and cleanup intent together, with actual cleanup progress read
separately. Settled and cancelled flags prevent repeated terminal effects.

## Dependency rules

Wave 4 reserves the `collector-api-types` boundary below; its DTO move remains
planned. D04.2 introduced `collector-providers` at its reserved boundary. The
architecture checker enforces both dependency directions. Archive v4 processing
records and projections remain implemented through existing core, application,
and store boundaries.

- `collector-core`: portable typed identities, records and errors. No filesystem, database, networking, or presentation.
- `collector-api-types`: shared serializable client-facing read DTOs. It depends only on `collector-core` plus serialization/schema libraries and owns no services, authorization, storage, or transport. Callers migrate to the shared DTO owner together.
- `collector-modules`: module-owned schemas, source-preserving migrations, presentation values, and host-mediated operation contracts. Reader-only game normalization delegates to narrow `junk-libs-game-formats` primitives; the module owns operation policy and result validation. Music modules likewise map package identities into shared `junk-libs-disc` integer PCM readers; byte decoding and source concatenation remain shared mechanisms. No provider requests or application scheduling.
- `collector-app`: use cases, authorization, effective permissions, and explicit response DTOs over mechanism-only storage ports. Frontends do not bypass these rules.
- `collector-providers`: bounded HTTP, provider/catalog parsing adapters, recorded-response fixtures, and optional encoder-process mechanisms. It may depend on `collector-core`, `collector-api-types`, application port contracts, module-owned catalog DTOs, and narrow `junk-libs` crates; it never depends on Store, server, or a GUI.
- `collector-store`: archive publication, local catalog projection, private state, package/asset storage. It implements mechanism ports using validated identities/effects; application services own authorization policy.
- `collector-server`: HTTP composition, WebAuthn ceremonies, route exposure, worker limits, static UI delivery. Blocking storage/CPU work runs outside async workers.
- `collector-client`: endpoint trust, transport, offline cache policy. No server storage dependency.

Domain modules return observations, candidates, safe presentation values, and verification results. Durable hosts supply cancellation-aware readers and cooperative checkpoints for CPU-only loops; cancelled work cannot publish successful observations. They never decide to replace an archive, switch a view, scrape another provider, or construct a scheduler. The host owns those decisions and exposes only authorized opaque `Read + Seek + Send` package-file handles and bounded job submission. Package inventories are bound to a Collection and either an Import or Representation; file identities and validated package-relative logical names cross the boundary, while adapter filesystem paths do not. Relative member references resolve lexically within that inventory and cannot escape it. The compatibility single-file reader succeeds only for a one-file package and reports multi-file ambiguity explicitly. A test-only schema-2 stamp module proves registration, additive migration, multi-file byte access, job requests, and presentation do not require modifying a universal enum.

Processing inputs bind an operation/version to a deterministic `SourceFingerprint` over the typed source, every declared file in order, exact lengths and SHA-256 digests, optional layout digest, and sorted string parameters. Its versioned fixed binary encoding contains no filenames, filesystem paths, unordered maps, debug output, or floating-point values. Changing any package member changes the fingerprint. Durable processing Jobs persist validated frozen inputs, preallocated output identities and timestamps before dispatch. Import- and Representation-scoped readers expose the exact frozen inventory through opaque handles. Provider responses retain bounded raw bytes and lossless parsed observations for domain policy to interpret. The local DAT adapter reads only administrator-configured source names, archives exact XML or ClrMamePro bytes, and rebuilds its typed row view from those bytes for each comparison. It checks the complete stream against the frozen snapshot length and SHA-256 before returning rows; malformed or truncated catalogs fail without dropping entries. The game module owns catalog byte-domain coverage: raw-2352 matching may exclude retained CUE metadata but rejects malformed or unknown payloads, and the matcher checks adapter coverage declarations. This is bounded parsing and linear matching, not a persistent indexed lookup or a scale-qualified catalog engine. Large catalog acquisition and snapshot access use bounded readers rather than buffered DTOs (512 MiB maximum); derived DAT indexes are capped at 64 MiB and small provider responses are capped at 8 MiB. Acquisition jobs freeze their query/output identity without inventing a file source. Potentially network-backed snapshot streams are bounded and spooled outside the archive writer lock; local spool bytes are checked against recorded length and SHA-256 before publication. Observation reads use scoped cursor pagination and curator authorization independent of module write eligibility, so missing modules do not make historical evidence unreadable. Source currency compares against server-owned committed file records, not caller-provided fingerprints; this does not perform live integrity verification. Stored operations and registered capabilities share one validated identity type, and dispatch uses registered capability ownership. Conversion recipes bind typed settings, source, ordered track ranges, encoder version reports and PCM formats to a checked canonical digest; optional junk-libs-audio derivative validation owns integer FLAC comparison and Ogg Opus inspection, while Collector owns subprocess lifecycle and cancellation/publication policy; metadata text overrides have one value per field and artwork has one authority. Unsupported layouts, missing dependencies, external qualification limits, and execution failures are distinct outcomes. CD sectors, PCM frames, and signed stereo-sample-frame offsets are distinct types.

Explicit music derivative requests use the existing command receipts and frozen
job executor. Each independently playable track has one sealed recipe and output
identity, determined by Capture, exact source/track range, preset and full encoder
report. The application maps music-owned PCM ranges and enforces source currency,
current authorization and cancellation. Store owns scratch handles and per-attempt
spools; subprocess encoding and shared validation run outside the archive writer
lock. The existing Capture revision journal treats output, immutable recipe and
validation Evidence as dependencies published before the Capture commit marker.
Cancellation wins until the final gated revision rename. Committed recovery uses
retained bytes/evidence without re-encoding or re-authorizing; current Capture
associations and Item edits survive. Reindex validates derivative references and
stored payload identity, without claiming a fresh codec or catalog verification.
The viewer-safe ordered availability contract carries source Representation,
track position and integer PCM coordinates, safe title, profile and only
preparing/unavailable/available state, plus a typed actor-specific preparation
permission. UI behavior never depends on human-readable reason text. Preparation
receipts bind the submitted request before resolving mutable tool configuration.
Published state adds immutable
Representation/package-file identity, length, media type and digest; archive
paths, raw evidence and private Job details remain server-side. The client router
serves this contract and existing opaque media readers with strong content ETags
and byte ranges. Only the curation router can enqueue preparation. Server-owned
optional encoder configuration supplies the probed full version report; browser
inputs cannot name tools or version claims. Published outputs remain readable
when the module or tool later disappears. Import and media reads never request
conversion automatically.

LA2 preparation keeps a frozen batch Job over existing derivative Jobs. Application
policy owns immutable member scope, admission bounds, outcome projection and the
shared-work cancellation rule. Private indexed batch/member references and direct
request pins are committed with admission. Store evaluates that application rule
under the same transaction that changes cancellation state. Fan-out rechecks the
parent and its claim before each child admission; cancellation cannot admit a late
child. A direct request made after batch acceptance protects that shared work too.
Frozen recipe/output identities determine equivalence; the first accepted creation
time is retained on reuse. Parent failures remain visible independently of child
outcomes, and retries retain attempt diagnostics and enforce guided cancellation.

Collection reads cross `CollectionReadPort`, whose methods receive durable identities and queries rather than a Principal. `CollectionService` owns grant checks, role redaction, and conversion into explicit collection/item/evidence response types. Viewer JSON structurally omits acquisition, private notes, loans, metadata observations/overrides, detailed Evidence fields, and raw module envelopes; unavailable modules retain a safe non-executable notice without their raw generic value. Capture DTOs never expose archive locators. Owner and curator DTOs retain permitted curation detail, and every authorized collection/item response carries server-computed effective permissions. `CollectionService` is constructed from narrow ports; the combined `Catalog` trait remains only as an internal Store compatibility adapter while callers use the policy-owned service families.

Application mutations carry a validated `CommandContext`. Private SQLite schema 3 claims receipts by `(UserId, API major, operation, Idempotency-Key)` and binds the claim to a SHA-256 digest of the validated typed request, path identities, and operation. Authorization and validation run before a claim. A completed receipt replays its non-secret serialized result across restart; a changed digest or active claim returns a stable conflict; an effect error abandons an unprepared claim; a prepared publication remains recoverable until startup resolves its commit marker. Authentication ceremonies, logout, resumable upload chunks, and internal failure bookkeeping use their own single-use or offset state machines instead. The HTTP and React layers transport keys but never decide replay policy.

Command execution creates an explicit `CommandEffects` scope bound to a typed `CommandReceipt`. The scope shares the Store's locks and connections; it carries no Principal, thread-local state, or ambient authorization. Private mutations complete the response receipt in the effect's SQLite transaction. Filesystem mutations durably prepare the exact response in private `command_outcomes`, bound to a root-relative publication marker and digest before publishing. Startup validates the marker and replays committed results, or releases uncommitted claims, before opening services. Upload bookkeeping and committed artwork scratch cleanup are reconciled after projection rebuild. An operation must use one of these two commit mechanisms; returning an effect and subsequently writing its first durable receipt is not an acceptable adapter implementation.

Security rules live in `collector-app::security`: account/grant authority snapshots, default-account eligibility, credential revocation sets, and enrollment transitions. Store gathers facts under an immediate SQLite transaction, evaluates that policy, and applies its effects before committing. CAS and serialized snapshot loading are storage mechanisms; last-owner/admin/default and lifecycle decisions are application policy. Grant revision counters survive removal, so recreating access cannot make a stale revision current again.

API route groups are annotated once and supply both Axum runtime routing and the checked OpenAPI 3.1 document. C01.2 will move shared read DTOs to `collector-api-types`; application command types remain in `collector-app`, with callers updated together. Those Rust contract types generate `api/openapi.v1.json`; `openapi-typescript` generates the React-only type surface. The regeneration check writes temporary outputs and fails on byte differences. Request-body limits are included as `x-body-limit-bytes`. The server does not expose OpenAPI or Swagger UI. The bounded Wave 1 item envelope is a deprecated application adapter: it loads canonical server-owned state, rejects differences, then delegates only editable fields to the current command.

Authenticated collection-type discovery returns only registry-validated module descriptors: bounded labels, scalar field contracts, read-only structured-field notices and capability availability. It exposes no module configuration or management switch. Application read DTOs combine grants, durable binding and registry/schema availability into explicit content permissions; access management and personal activity remain separate. Domain-patch receipts bind the submitted patch under their own operation rather than its state-dependent resolved payload. Generic clients submit owner-bound domain patches, never replacement raw envelopes, and render type labels/summaries without branching on built-in module IDs. Legacy assignment guidance names the stopped-server inspection and assignment commands; unavailable owners remain safe read-only presentations.

Mutations cross bounded application commands. Item input contains editable inventory/module fields only; Capture representations, Assets, Evidence, metadata observations, revisions, deletion state, timestamps, and storage locators cannot be deserialized into it. Work/Edition commands preserve migration provenance and server-owned revision state, and Capture commands can change associations while preserving representations; legacy locations exist only in store adapter records/projections. Content, import, administration, authentication, enrollment, and access adapters receive validated identities/effects rather than Principal or Role. Media returns an opaque synchronous read/seek capability so neither application code nor HTTP sees an archive path; HTTP streams it on blocking workers with range semantics. The server source is checked against direct `state.store` calls. Authentication freshness, authority guards, passkey CAS, recovery, and native enrollment are application policies backed by transactional Store mechanisms.

Module wire payloads use a lossless raw-JSON envelope. Known current schemas are parsed into module-owned views without replacing the raw body. Supported older schemas migrate one adjacent version at a time. Additive checks are the default; a version-specific transformation may rename or convert fields only through an explicit policy with a rationale. Every exact input/intermediate envelope is returned as provenance and must be retained by the committing host. Unknown fields remain preserved in those envelopes. Capability ownership is indexed and validated at registration, never inferred by splitting a capability string at dispatch. Each registered operation declares collection-domain or shared-media scope, and every successful output passes its owner's required result validator before publication. Module descriptors expose only bounded labels and typed scalar/read-only field descriptions. Owner-mediated patches distinguish omission from explicit null and preserve untouched raw values; clients do not replace arbitrary module envelopes. Newer, gapped, unavailable, and invalid schemas remain distinct outcomes; unavailable data survives but is not described as executable. Generic presentation is an additive, post-redaction read model and does not expose operation commands before A04. Raw provider JSON is source evidence, not the common field model. Runtime plugins will adapt the same capability boundaries; a native ABI is deliberately not promised.

## Identity and ownership

Collection, item, work, edition, specimen, component, carrier, capture, representation, package file, processing observation, snapshot, recipe, asset, evidence, user, device, and server IDs are distinct Rust newtypes. Paths and SQLite row numbers are never identity. Works and editions are collection-owned records; an inventory item may be unidentified or linked to a work or edition. Duplicate physical copies are separate specimens. Components describe physical parts, while only data/audio-bearing components have carriers. A capture is collection-level and may remain unattached to an item or physical carrier. No physical specimen or catalog identity is fabricated by importing a dump. Wishlists, ownership, possession, loan records, and disposal are independent. A tombstone does not delete payloads. The complete reviewed relationships are recorded in [foundation contracts](decisions/foundation-contracts.md).

Game and music data, processing results and domain metadata overrides live in module-owned types. Core retains common title/subtitle/artwork authority and genuinely shared CD/PCM coordinates and layout. Unknown historical field overrides are retained verbatim in `preserved_field_overrides`, including language and unknown values; they are not applied as active authority until explicit owner-aware conversion. Reading/reindexing does not rewrite their original archive records. The collection-binding contract is Assigned to a ModuleId or LegacyUnassigned. New creation persists an Assigned owner; a missing historical field becomes LegacyUnassigned. Graph publication rejects that holding state. Application content writes centrally require Curate authority, an Assigned binding, an installed owner and supported schema; collection-domain operations must belong to that owner, while explicitly shared-media operations need not. Durable processing inputs freeze and recheck the binding, but already committed recovery completes without module availability or grants. Collection write helpers live in `collection_types.rs`; stored unsupported Item schemas cannot be overwritten through ordinary edits. Long import adapters invoke application policy after payload staging and before a new Capture commit. A denied guard clears provisional publication copies and retains source uploads. Integrity progress also rechecks eligibility; committed integrity recovery never rehashes payloads. These checks are not a serializable transaction with concurrent grant changes. Explicit local assignment is application policy in `collection_types.rs` and publishes a journaled collection-metadata sidecar without changing the creation manifest. Its caller holds exclusive stopped-server access throughout inspection and assignment. Module validators check active authority; shared-media operations do not imply collection ownership, and unknown pending jobs block assignment. Core inventory remains useful without a dump or definitive identity. Metadata candidate ranking uses identity strength before preferred language. Manual choices and overrides are separate from observations. A provisional display selection never becomes checksum evidence.

## Archive and publication

The archive is portable and contains no credentials or access grants:

```
collections/<readable-name>--<collection-id>/
  collection.toml
  collection_metadata/<collection-id>/revisions/<zero-padded-revision>/...
  works/<work-id>/revisions/<zero-padded-revision>/...
  editions/<edition-id>/revisions/<zero-padded-revision>/...
  items/<readable-title>--<item-id>/
    revisions/<zero-padded-revision>/...
  captures/<capture-id>/
    revisions/<zero-padded-revision>/...
    payload/<representation-id>/<original-package-paths>
  assets/<asset-id>/
    revisions/<zero-padded-revision>/...
    payload/original.<format>
    payload/thumbnail.png
  evidence/<evidence-id>/revisions/<zero-padded-revision>/...
```

The tree above remains the schema-3 layout for existing records. Archive envelope schema 4 additively introduces ProcessingObservation, provider/catalog Snapshot, MetadataAuthority, and ConversionRecipe record kinds without rewriting schema-1/2/3 bytes. Mixed-version reindex preserves unknown record and module envelopes. Import-scoped observations are rebound at Capture publication to the committed Representation only after the ordered file identities, lengths, and digests match; their provisional fingerprint and explicit file mapping remain provenance. Migrated schema-1/2 artwork remains byte-for-byte at its item-local path and is reached through adapter-only locators; it is not moved or rewritten.

MetadataAuthority revisions keep typed common title/subtitle and artwork authority,
an owner-bound domain override envelope, and preserved unknown historical fields.
A refresh appends its bounded lossless observation in the same revision as the
materialized display while leaving authority unchanged. Reindex reads every
authority revision to rebuild stable provider-record/version candidate history,
then projects only the newest authority/display revision. Resolution applies field
overrides, explicit selection, then deterministic automatic ranking by complete
identity, evidence strength, English preference, provider priority and stable
provider identity. Metadata identity remains distinct from integrity, catalog
verification and conversion validation.

Candidate identities are immutable: changed normalized content or raw responses
require a new provider record version (a response digest when no upstream version
exists). Observation IDs are also immutable and bound to their item/collection;
receipt identity, observation identity and publication transaction identity are
separate. Publication rejects contradictory history before committing bytes, and
reindex checks the same invariants. Catalog v6 projects candidates and refreshes.
Each new authority record freezes its inventory revision/title/subtitle. Later
inventory text edits become manual authority at the application read/next command;
unchanged inventory fields do not undo explicit overrides or clears. Publication
checks that inventory baseline under the archive writer lock. Reindex validates
materialized display against frozen inputs and candidate history at that revision.
Automatic ambiguity remains tentative even when unrelated manual choices exist.

Authority revisions additionally retain current candidate identities for automatic
resolution; superseded versions remain available for explicit historical selection.
Provider-derived versions include raw/normalized content, ranking and artwork inputs.
MusicBrainz/CAA adapters join the existing acquisition and catalog-parser registries;
module DTOs own provider interpretation, and application services share existing Job,
Asset and metadata-refresh effects. Acquisition receipts bind submitted item/query
inputs before timestamps or other frozen state. Non-JSON negative responses remain
exact snapshots referenced by metadata; malformed successful responses are failures.
The HTTP connector validates its actual DNS answers, disables ambient proxies and
pins HTTPS provider/storage origins. MusicBrainz has one composed admission/cooldown
limiter and a process-local cache bounded to 128 entries/32 MiB. Curation-only metadata
HTTP/UI exposes explicit acquisition and authority changes; imports do not implicitly
request enrichment. Application services dispatch completed, caller-owned, item-bound
acquisitions to the existing single metadata or Asset effect. Artwork publication
preserves an image for explicit cover selection without requiring another metadata
request. Clients persist completion receipt inputs across panel reopen and abort
obsolete requests. Effective display and rebuildable search share application-owned
resolution, including later inventory corrections; adapters do not invent precedence.

TOML manifests contain schema version, entity kind, typed identities, revision and the SHA-256 of the corresponding record. Archive schema 3 uses deterministic additive sidecars over schema-1/2 history. Canonical Item records contain inventory state but no embedded Captures or Assets; standalone Capture revisions and revisioned collection-level Asset records are authoritative. Append-only Evidence has collection identity, optional Item identity, one or more exact Representation identities, and is referenced by digest from revision envelopes. Legacy payload locations are adapter-only records. Item responses remain compatibility views hydrated from these canonical rows. The module/provider envelope preserves lexical JSON, including key order, number spelling, and nulls.

The A02 migration preserves every schema-1 `record.json` and manifest byte-for-byte and writes schema-2 sidecars plus typed identity/capture records. P01.3 likewise preserves all schema-1/2 bytes and adds schema-3 Item/Capture sidecars, standalone Evidence, Asset metadata revisions, legacy locators, per-Item checkpoints, and a completion marker written last. Outputs and deterministic transaction identities must match on restart; conflicting reused Evidence identities fail migration. Neither migration reconstructs private grants, so archive-only recovery has no access assignments.

A single process lock owns archive writes. Current item/typed-entity publication checks expected revisions under the writer lock, stages and syncs content, atomically renames the revision directory, and updates SQLite in a transaction. The rename is the archive commit point. Capture and collection-binding records use the same typed revision publisher during migration, recovery and normal updates. Collection graph validation enforces unique physical/representation identities, assigned-owner/domain consistency and transitive association ancestry, including omitted intermediate links; writes and rebuilds reject cross-collection identity reuse. The current implementation checks collection snapshots for correctness; indexed reference/ownership queries and write-cost qualification are required by P06 before scale acceptance.

Durable publication journals contain transaction/entity identities, expected/target revisions, safe relative paths, hashes, and monotonic phases only—never users, grants, credentials, or secrets. Startup discards recognized interrupted journal replacements and aborts incomplete Prepared staging. After BytesStaged it validates staged digests, and published bytes are always validated. It retains unknown staging, removes durably completed known scratch, and blocks writes on corrupt known transactions. Entity and import writers use the same phases; resumable upload chunks stay private and separate. An import's stable Job identity also identifies its payload transaction, so recovery before the Capture commit removes known uncommitted artifacts while recovery after it preserves the Capture and completes deterministic Evidence/Item projections. Collection creation instead uses a private recovery row written before archive mutation because its intended owner grant is not portable archive data. Startup completes or aborts that row under the writer lock before any service is exposed.

Collection creation writes an immutable collection manifest and a private recovery intent for the owner grant before archive mutation. Startup completes the intended grant when the collection commit exists or removes the abandoned intent when it does not. Copying only the archive never reconstructs a grant.

SQLite state must be local to the server. The current rebuildable `catalog.sqlite` is schema v6 and `private.sqlite` is schema v13. Catalog v4 adds portable processing, snapshot, metadata-authority, and recipe projections to the v3 projections; catalog v5 adds the snapshot creation-time index used for stable pagination. Private v10 adds frozen processing Job inputs and provisional Import observations to the v9 authentication, access, Import, Job, command, and recovery state. Private v11 adds collection/provider active-catalog selection; exact snapshot bytes and observations remain portable archive records, while this replaceable selection is local policy. Private v12 was an unused planning reservation. Private v13 adds guided-ingest intent and restartable staging-cleanup records; source bytes and published archive records remain outside that private workflow state. These are additive pre-release migrations: older on-disk user data is converted in place and is never reset automatically; no permanent reverse-compatibility layer is promised solely for earlier Wave checkpoints. Client connection storage is independently versioned: C01 starts at client schema v1 and S04 reserves client schema v2 for rotation state. Future version assignments follow actual persisted state and the pre-release policy; W4.0 reservations are advisory. Reindex validates the collection graph and processing envelopes and replaces all catalog projections in one transaction. Browser sessions and native credentials are distinct types and tables; migrated sessions are non-fresh and never become native credentials. Transaction-based migrations select the former first-row account once; later account or `multi_user` changes do not change it. A missing or disabled default is an actionable configuration error. Back up both the archive and private state. Copying only the archive restores records but no access. SQLite databases must be backed up through SQLite's backup mechanism or while stopped, not by copying active WAL files independently.

The first implementation reindexes metadata at startup; incremental startup/rebuild optimization remains in the performance milestone. Payloads are never read merely to browse. Writes are single-server; independent clones must not reuse operational identity. Replication requires additional protocol work, although IDs, revisions, immutable payloads and tombstones do not assume a server location.

## Ingest and assets

Uploads declare a complete file inventory, optional source SHA-256 values, item and expected revision. Names reject traversal, absolute paths, control characters and Windows separators. Chunks are bounded to 8 MiB; browser chunks use 4 MiB. Durable file lengths recover upload cursors after restart. Duplicate identical chunks are idempotent; conflicting overlaps fail. Completed staging still accepts byte-identical chunk replay read-only. Browser resumption validates the reselected inventory and compares retained prefixes before appending remaining bytes. Startup resolves abandoned cancellation acknowledgments atomically with their Import state; once a Capture is committed, recovery completes its existing publication rather than reopening pre-commit Item revision review.

Commit checks complete lengths/digests, copies into archive-local staging, rereads destination SHA-256, publishes files and then item/evidence. Source uploads are retained. Integrity means those bytes match, not catalog identity. Reverification appends evidence. Catalog verification, reconstruction, offsets and audio derivatives require separate providers and independent oracles.

Artwork is stored as an original image plus a bounded PNG thumbnail. Image decoding has dimension/allocation limits. Both asset and original-media requests enforce collection access and asset visibility. Private attachments are not merely hidden in HTML.

## HTTP exposure and trust

Three surfaces: client, curation, administration. Public curation and administration default off. Public administration implies curation exposure; roles remain mandatory. Routers omit disabled pages, JavaScript management entrypoints and mutation APIs. The public listener stays public regardless of `Forwarded` or `X-Forwarded-For`.

The process exposes two loopback HTTP origins for separate public/private proxies and one direct LAN TLS client listener. An optional `private_tls_bind` adds direct private management HTTPS on an explicitly named interface; wildcard private TLS binds are rejected. Connect a public tunnel only to the public origin. A private proxy or Tailscale Serve connects to the private origin. LAN TLS is public-surface by default; local connectivity alone never upgrades it to administration. Allowlisted browser origins are exact and configured, not derived from proxy headers.

Native pairing carries server ID, local certificate and its SHA-256 fingerprint. Trusted enrollment is required; a fingerprint downloaded unauthenticated is not trust. The local client installs only the enrolled certificate as its trust anchor and uses normal certificate/hostname/handshake validation. No accept-invalid-certificate option exists. Certificates must include the configured LAN names/IPs. Public routes use ordinary Web PKI separately. Credentials must never follow cross-origin redirects.

Passkeys use one configured HTTPS authentication origin and standard WebAuthn. Challenges are bounded, server-side, expiring and single-use. Registration is bound to the authenticated user and ceremonies retain the original credential record/revision, and counter updates compare-and-swap that verified snapshot together with session issuance. Reloading a newer record after verification is forbidden; zero-counter passkeys retain the WebAuthn library’s verification semantics. Credential IDs are unique across accounts. CLI setup explicitly allowlists the private listener’s localhost browser origin; it does not change public exposure or trust forwarded headers. Browser sessions are HttpOnly, Secure and SameSite; browser ceremonies and cookie mutations require an exact normalized Origin. Native enrollment start/complete are the narrow unauthenticated bootstrap exception: they grant no authority before same-user, recently-authenticated browser approval and authenticate completion with a stored-hash high-entropy secret. The issued 30-day native credential is returned once, stored only as a hash server-side, and selected only by successful native-credential lookup—never by a caller header. Single-user presentation hides account selection; it is not anonymous access.

## Clients and shared audio

The bounded browser player lives in the client bundle and consumes authorized
availability rows and opaque media URLs. Its short-lived disc selection is grouped
by Capture/source and chooses one representation per track. HTML media seconds
are a presentation projection; this adds no reusable native queue, DSP engine,
server player or persisted activity. The component owns and releases its media
element at ref detachment, fences superseded play promises, and discards obsolete
playback when availability changes. Native queue/PCM services remain separate
shared-library work. Client playback never requires the curation bundle or surface.

Artwork is part of browsing data, not a decorative optional gallery: games and CDs show cover thumbnails next to text, including search and offline snapshots. Unidentified/tentative identity is distinct from missing artwork.

Client connection/cache logic stays below Slint and SwiftUI. LAN preference is independent of browser certificate trust or passkey origin. Offline cache identity includes server, representation, and conversion profile; pinned entries survive automatic eviction. Offline content cannot be recalled by server revocation.

`junk-libs` already allows per-crate Cargo dependencies. Keep one workspace unless releases diverge. Extract disc IDs, verification, decoding, queues and DSP as narrow reusable crates, without archive/accounts/UI dependencies. Trash Amp retains its standalone local-player role. Cross-project extractions are separate migrations with sibling regression checks.

## Extension discipline

Before adding a module: identify its owned data, invariants, capabilities, persistence needs, and presentation contribution. If its implementation would bypass an existing boundary, change this architecture and its contract tests first. Avoid a second importer, authorization policy, parser or playback queue inside a frontend.
