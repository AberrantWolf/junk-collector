# Architecture

Junk Collector preserves evidence and owned collection history independently of how content is consumed. The server is a modular monolith: one process composes independent domain, application, storage, authentication, HTTP, and job capabilities. It is not a set of microservices.

## Dependency rules

- `collector-core`: portable typed identities, records and errors. No filesystem, database, networking, or presentation.
- `collector-modules`: module-owned schemas, source-preserving migrations, presentation values, and host-mediated operation contracts. No provider requests or application scheduling.
- `collector-app`: use cases, authorization, effective permissions, and explicit response DTOs over mechanism-only storage ports. Frontends do not bypass these rules.
- `collector-store`: archive publication, local catalog projection, private state, package/asset storage. It implements mechanism ports using validated identities/effects; application services own authorization policy.
- `collector-server`: HTTP composition, WebAuthn ceremonies, route exposure, worker limits, static UI delivery. Blocking storage/CPU work runs outside async workers.
- `collector-client`: endpoint trust, transport, offline cache policy. No server storage dependency.

Domain modules return observations, candidates, safe presentation values, and verification results. They never decide to replace an archive, switch a view, scrape another provider, or construct a scheduler. The host owns those decisions and exposes only authorized opaque `Read + Seek + Send` representation handles and bounded job submission. A test-only schema-2 stamp module proves registration, additive migration, byte access, job requests, and presentation do not require modifying a universal enum.

Collection reads cross `CollectionReadPort`, whose methods receive durable identities and queries rather than a Principal. `CollectionService` owns grant checks, role redaction, and conversion into explicit collection/item/evidence response types. Viewer JSON structurally omits acquisition, private notes, loans, metadata observations/overrides, detailed Evidence fields, and raw module envelopes; unavailable modules retain a safe non-executable notice without their raw generic value. Capture DTOs never expose archive locators. Owner and curator DTOs retain permitted curation detail, and every authorized collection/item response carries server-computed effective permissions. `CollectionService` is constructed from narrow ports; the combined `Catalog` trait remains only as an internal Store compatibility adapter while callers use the policy-owned service families.

Application mutations carry a validated `CommandContext`. Private SQLite schema 3 claims receipts by `(UserId, API major, operation, Idempotency-Key)` and binds the claim to a SHA-256 digest of the validated typed request, path identities, and operation. Authorization and validation run before a claim. A completed receipt replays its non-secret serialized result across restart; a changed digest or active claim returns a stable conflict; an effect error abandons an unprepared claim; a prepared publication remains recoverable until startup resolves its commit marker. Authentication ceremonies, logout, resumable upload chunks, and internal failure bookkeeping use their own single-use or offset state machines instead. The HTTP and React layers transport keys but never decide replay policy.

Command execution creates an explicit `CommandEffects` scope bound to a typed `CommandReceipt`. The scope shares the Store's locks and connections; it carries no Principal, thread-local state, or ambient authorization. Private mutations complete the response receipt in the effect's SQLite transaction. Filesystem mutations durably prepare the exact response in private `command_outcomes`, bound to a root-relative publication marker and digest before publishing. Startup validates the marker and replays committed results, or releases uncommitted claims, before opening services. Upload bookkeeping and committed artwork scratch cleanup are reconciled after projection rebuild. An operation must use one of these two commit mechanisms; returning an effect and subsequently writing its first durable receipt is not an acceptable adapter implementation.

Security rules live in `collector-app::security`: account/grant authority snapshots, default-account eligibility, credential revocation sets, and enrollment transitions. Store gathers facts under an immediate SQLite transaction, evaluates that policy, and applies its effects before committing. CAS and serialized snapshot loading are storage mechanisms; last-owner/admin/default and lifecycle decisions are application policy. Grant revision counters survive removal, so recreating access cannot make a stale revision current again.

API route groups are annotated once and supply both Axum runtime routing and the checked OpenAPI 3.1 document. Rust application/core contract types generate `api/openapi.v1.json`; `openapi-typescript` generates the React-only type surface. The regeneration check writes temporary outputs and fails on byte differences. Request-body limits are included as `x-body-limit-bytes`. The server does not expose OpenAPI or Swagger UI. The bounded Wave 1 item envelope is a deprecated application adapter: it loads canonical server-owned state, rejects differences, then delegates only editable fields to the current command.

Mutations cross bounded application commands. Item input contains editable inventory/module fields only; Capture representations, Assets, Evidence, metadata observations, revisions, deletion state, timestamps, and storage locators cannot be deserialized into it. Work/Edition commands preserve migration provenance and server-owned revision state, and Capture commands can change associations while preserving representations; legacy locations exist only in store adapter records/projections. Content, import, administration, authentication, enrollment, and access adapters receive validated identities/effects rather than Principal or Role. Media returns an opaque synchronous read/seek capability so neither application code nor HTTP sees an archive path; HTTP streams it on blocking workers with range semantics. The server source is checked against direct `state.store` calls. Authentication freshness, authority guards, passkey CAS, recovery, and native enrollment are application policies backed by transactional Store mechanisms.

Module wire payloads use a lossless raw-JSON envelope. Known current schemas are parsed into module-owned views without replacing the raw body. Supported older schemas migrate one adjacent version at a time. Additive checks are the default; a version-specific transformation may rename or convert fields only through an explicit policy with a rationale. Every exact input/intermediate envelope is returned as provenance and must be retained by the committing host. Unknown fields remain preserved in those envelopes. Capability ownership is indexed and validated at registration, never inferred by splitting a capability string at dispatch. Newer, gapped, unavailable, and invalid schemas remain distinct outcomes; unavailable data survives but is not described as executable. Generic presentation is an additive, post-redaction read model and does not expose operation commands before A04. Raw provider JSON is source evidence, not the common field model. Runtime plugins will adapt the same capability boundaries; a native ABI is deliberately not promised.

## Identity and ownership

Collection, item, work, edition, specimen, component, carrier, capture, representation, asset, evidence, user, device, and server IDs are distinct Rust newtypes. Paths and SQLite row numbers are never identity. Works and editions are collection-owned records; an inventory item may be unidentified or linked to a work or edition. Duplicate physical copies are separate specimens. Components describe physical parts, while only data/audio-bearing components have carriers. A capture is collection-level and may remain unattached to an item or physical carrier. No physical specimen or catalog identity is fabricated by importing a dump. Wishlists, ownership, possession, loan records, and disposal are independent. A tombstone does not delete payloads. The complete reviewed relationships are recorded in [foundation contracts](decisions/foundation-contracts.md).

Game and music data live in module-owned types. Core inventory remains useful without a dump or definitive identity. Metadata candidate ranking uses identity strength before preferred language. Manual choices and overrides are separate from observations. A provisional display selection never becomes checksum evidence.

## Archive and publication

The archive is portable and contains no credentials or access grants:

```
collections/<readable-name>--<collection-id>/
  collection.toml
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

The tree above is the schema-3 layout for new records. Migrated schema-1/2 artwork remains byte-for-byte at its item-local path and is reached through adapter-only locators; it is not moved or rewritten.

TOML manifests contain schema version, entity kind, typed identities, revision and the SHA-256 of the corresponding record. Archive schema 3 uses deterministic additive sidecars over schema-1/2 history. Canonical Item records contain inventory state but no embedded Captures or Assets; standalone Capture revisions and revisioned collection-level Asset records are authoritative. Append-only Evidence has collection identity, optional Item identity, one or more exact Representation identities, and is referenced by digest from revision envelopes. Legacy payload locations are adapter-only records. Item responses remain compatibility views hydrated from these canonical rows. The module/provider envelope preserves lexical JSON, including key order, number spelling, and nulls.

The A02 migration preserves every schema-1 `record.json` and manifest byte-for-byte and writes schema-2 sidecars plus typed identity/capture records. P01.3 likewise preserves all schema-1/2 bytes and adds schema-3 Item/Capture sidecars, standalone Evidence, Asset metadata revisions, legacy locators, per-Item checkpoints, and a completion marker written last. Outputs and deterministic transaction identities must match on restart; conflicting reused Evidence identities fail migration. Neither migration reconstructs private grants, so archive-only recovery has no access assignments.

A single process lock owns archive writes. Current item/typed-entity publication checks expected revisions under the writer lock, stages and syncs content, atomically renames the revision directory, and updates SQLite in a transaction. The rename is the archive commit point. Capture records use the same typed revision publisher during migration, recovery and normal updates. Collection graph validation enforces unique physical/representation identities and transitive association ancestry, including omitted intermediate links; writes and rebuilds reject cross-collection identity reuse. The current implementation checks collection snapshots for correctness; indexed reference/ownership queries and write-cost qualification are required by P06 before scale acceptance.

Durable publication journals contain transaction/entity identities, expected/target revisions, safe relative paths, hashes, and monotonic phases only—never users, grants, credentials, or secrets. Startup discards recognized interrupted journal replacements and aborts incomplete Prepared staging. After BytesStaged it validates staged digests, and published bytes are always validated. It retains unknown staging, removes durably completed known scratch, and blocks writes on corrupt known transactions. Entity and import writers use the same phases; resumable upload chunks stay private and separate. An import's stable Job identity also identifies its payload transaction, so recovery before the Capture commit removes known uncommitted artifacts while recovery after it preserves the Capture and completes deterministic Evidence/Item projections. Collection creation instead uses a private recovery row written before archive mutation because its intended owner grant is not portable archive data. Startup completes or aborts that row under the writer lock before any service is exposed.

Collection creation writes an immutable collection manifest and a private recovery intent for the owner grant before archive mutation. Startup completes the intended grant when the collection commit exists or removes the abandoned intent when it does not. Copying only the archive never reconstructs a grant.

SQLite state must be local to the server. Rebuildable `catalog.sqlite` schema v3 projects Works, Editions, Items, Components, Captures, standalone Evidence/Assets, and adapter-only legacy payload locators. Reindex validates the collection graph before replacing those projections. `private.sqlite` schema v7 contains revisioned/enabled accounts and grants, browser sessions, passkeys, recovery-code hashes, native credentials, enrollment state, redacted security audit, jobs, personal activity, server identity, the durable `default_user_id`, command receipts, prepared command outcomes, persistent grant revision counters, and private publication-recovery rows. Browser sessions and native credentials are distinct types and tables; migrated sessions are non-fresh and never become native credentials. Transaction-based migrations select the former first-row account once; later account or `multi_user` changes do not change it. A missing or disabled default is an actionable configuration error. Back up both the archive and private state. Copying only the archive restores records but no access. SQLite databases must be backed up through SQLite's backup mechanism or while stopped, not by copying active WAL files independently.

The first implementation reindexes metadata at startup; incremental startup/rebuild optimization remains in the performance milestone. Payloads are never read merely to browse. Writes are single-server; independent clones must not reuse operational identity. Replication requires additional protocol work, although IDs, revisions, immutable payloads and tombstones do not assume a server location.

## Ingest and assets

Uploads declare a complete file inventory, optional source SHA-256 values, item and expected revision. Names reject traversal, absolute paths, control characters and Windows separators. Chunks are bounded to 8 MiB; browser chunks use 4 MiB. Durable file lengths recover upload cursors after restart. Duplicate identical chunks are idempotent; conflicting overlaps fail.

Commit checks complete lengths/digests, copies into archive-local staging, rereads destination SHA-256, publishes files and then item/evidence. Source uploads are retained. Integrity means those bytes match, not catalog identity. Reverification appends evidence. Catalog verification, reconstruction, offsets and audio derivatives require separate providers and independent oracles.

Artwork is stored as an original image plus a bounded PNG thumbnail. Image decoding has dimension/allocation limits. Both asset and original-media requests enforce collection access and asset visibility. Private attachments are not merely hidden in HTML.

## HTTP exposure and trust

Three surfaces: client, curation, administration. Public curation and administration default off. Public administration implies curation exposure; roles remain mandatory. Routers omit disabled pages, JavaScript management entrypoints and mutation APIs. The public listener stays public regardless of `Forwarded` or `X-Forwarded-For`.

The process exposes two loopback HTTP origins for separate public/private proxies and one direct LAN TLS client listener. An optional `private_tls_bind` adds direct private management HTTPS on an explicitly named interface; wildcard private TLS binds are rejected. Connect a public tunnel only to the public origin. A private proxy or Tailscale Serve connects to the private origin. LAN TLS is public-surface by default; local connectivity alone never upgrades it to administration. Allowlisted browser origins are exact and configured, not derived from proxy headers.

Native pairing carries server ID, local certificate and its SHA-256 fingerprint. Trusted enrollment is required; a fingerprint downloaded unauthenticated is not trust. The local client installs only the enrolled certificate as its trust anchor and uses normal certificate/hostname/handshake validation. No accept-invalid-certificate option exists. Certificates must include the configured LAN names/IPs. Public routes use ordinary Web PKI separately. Credentials must never follow cross-origin redirects.

Passkeys use one configured HTTPS authentication origin and standard WebAuthn. Challenges are bounded, server-side, expiring and single-use. Registration is bound to the authenticated user and ceremonies retain the original credential record/revision, and counter updates compare-and-swap that verified snapshot together with session issuance. Reloading a newer record after verification is forbidden; zero-counter passkeys retain the WebAuthn library’s verification semantics. Credential IDs are unique across accounts. CLI setup explicitly allowlists the private listener’s localhost browser origin; it does not change public exposure or trust forwarded headers. Browser sessions are HttpOnly, Secure and SameSite; browser ceremonies and cookie mutations require an exact normalized Origin. Native enrollment start/complete are the narrow unauthenticated bootstrap exception: they grant no authority before same-user, recently-authenticated browser approval and authenticate completion with a stored-hash high-entropy secret. The issued 30-day native credential is returned once, stored only as a hash server-side, and selected only by successful native-credential lookup—never by a caller header. Single-user presentation hides account selection; it is not anonymous access.

## Clients and shared audio

Artwork is part of browsing data, not a decorative optional gallery: games and CDs show cover thumbnails next to text, including search and offline snapshots. Unidentified/tentative identity is distinct from missing artwork.

Client connection/cache logic stays below Slint and SwiftUI. LAN preference is independent of browser certificate trust or passkey origin. Offline cache identity includes server, representation, and conversion profile; pinned entries survive automatic eviction. Offline content cannot be recalled by server revocation.

`junk-libs` already allows per-crate Cargo dependencies. Keep one workspace unless releases diverge. Extract disc IDs, verification, decoding, queues and DSP as narrow reusable crates, without archive/accounts/UI dependencies. Trash Amp retains its standalone local-player role. Cross-project extractions are separate migrations with sibling regression checks.

## Extension discipline

Before adding a module: identify its owned data, invariants, capabilities, persistence needs, and presentation contribution. If its implementation would bypass an existing boundary, change this architecture and its contract tests first. Avoid a second importer, authorization policy, parser or playback queue inside a frontend.
