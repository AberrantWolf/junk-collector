# Architecture

Junk Collector preserves evidence and owned collection history independently of how content is consumed. The server is a modular monolith: one process composes independent domain, application, storage, authentication, HTTP, and job capabilities. It is not a set of microservices.

## Dependency rules

- `collector-core`: portable typed identities, records and errors. No filesystem, database, networking, or presentation.
- `collector-modules`: module-owned schemas, source-preserving migrations, presentation values, and host-mediated operation contracts. No provider requests or application scheduling.
- `collector-app`: use cases and authorization over storage ports. Frontends do not bypass these rules.
- `collector-store`: archive publication, local catalog projection, private state, package/asset storage. Its file services enforce the same collection roles.
- `collector-server`: HTTP composition, WebAuthn ceremonies, route exposure, worker limits, static UI delivery. Blocking storage/CPU work runs outside async workers.
- `collector-client`: endpoint trust, transport, offline cache policy. No server storage dependency.

Domain modules return observations, candidates, safe presentation values, and verification results. They never decide to replace an archive, switch a view, scrape another provider, or construct a scheduler. The host owns those decisions and exposes only authorized opaque `Read + Seek + Send` representation handles and bounded job submission. A test-only schema-2 stamp module proves registration, additive migration, byte access, job requests, and presentation do not require modifying a universal enum.

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
    assets/<asset-id>/original.<format>
    assets/<asset-id>/thumbnail.png
  captures/<capture-id>/
    revisions/<zero-padded-revision>/...
    payload/<representation-id>/<original-package-paths>
```

The tree above is the current layout. Collection-level asset storage and standalone `evidence/<evidence-id>.json` records remain target layout changes for U04/P01; existing artwork remains under its item.

TOML manifests contain schema version, entity kind, typed identities, revision and the SHA-256 of `record.json`. Archive schema 2 has typed Work, Edition, Item, and Capture revision directories and capture-owned payload paths. The existing HTTP `Item` aggregate carries capture/asset compatibility projections, but standalone Capture revisions are authoritative. Reindex may materialize a missing capture revision from an immutable item snapshot to finish an interrupted upload, then reloads the latest standalone records. It never projects old embedded copies over them. Item reads and graph validation hydrate capture views from those canonical records. Ordinary capture commands can change associations but cannot invent or replace stored files or migration locators. Storage rejects stale embedded capture views before item publication. The module/provider envelope preserves lexical JSON, including key order, number spelling, and nulls.

The A02 migration preserves every schema-1 `record.json` and manifest byte-for-byte, writes schema-2 sidecars plus typed identity/capture records, and retains an explicit locator to the legacy revision/payload. A migration-only checkpoint under `.archive-migrations/schema-1-to-2` records immutable emitted revisions; its completion marker is last, and an incomplete run is not canonical. Sidecars publish through synced sibling temporary files and atomic renames; typed revisions share the normal staged revision publisher. Uncheckpointed sidecars can be regenerated deterministically. Checkpointed output must match exactly, and partial legacy typed output is retained in staging before recovery. Checkpoint and ancestor directories are synced before successful completion. This is intentionally not P01's general transaction/evidence design. Accumulated schema-1 evidence remains in a transitional legacy envelope until P01 emits standalone append-only evidence.

A single process lock owns archive writes. Current item/typed-entity publication checks expected revisions under the writer lock, stages and syncs content, atomically renames the revision directory, and updates SQLite in a transaction. The rename is the archive commit point. Capture records use the same typed revision publisher during migration, recovery and normal updates. Collection graph validation enforces unique physical/representation identities and transitive association ancestry, including omitted intermediate links; writes and rebuilds reject cross-collection identity reuse. The current implementation checks collection snapshots for correctness; indexed reference/ownership queries and write-cost qualification are required by P06 before scale acceptance.

**P01 target, not current behavior:** general durable transaction phase records, independent append-only evidence, and private grant recovery before accepting new writes. Current startup replay repairs projections, and the capture compatibility recovery described above covers the existing upload gap; it is not a general multi-resource transaction protocol. Portable recovery metadata must contain no credentials or grants. Unknown staging content remains retained.

Collection creation currently writes one immutable collection manifest. Account grants are private state; a crash before grant publication leaves an ungranted collection for explicit administrator recovery. There is no automatic grant reconstruction.

SQLite state must be local to the server. `catalog.sqlite` is rebuildable and projects Works, Editions, Items, Components, Captures, and transitional legacy payload locators. Reindex validates the collection graph before replacing those projections. `private.sqlite` contains users, devices, credentials, grants, jobs, personal activity, server identity, and the durable `default_user_id`. Its transaction-based migration selects the former first-row account once; later account or `multi_user` changes do not change it. A missing/deleted default is an actionable configuration error. Back up both the archive and private state. Copying only the archive restores records but no access. SQLite databases must be backed up through SQLite's backup mechanism or while stopped, not by copying active WAL files independently.

The first implementation reindexes metadata at startup; incremental startup/rebuild optimization remains in the performance milestone. Payloads are never read merely to browse. Writes are single-server; independent clones must not reuse operational identity. Replication requires additional protocol work, although IDs, revisions, immutable payloads and tombstones do not assume a server location.

## Ingest and assets

Uploads declare a complete file inventory, optional source SHA-256 values, item and expected revision. Names reject traversal, absolute paths, control characters and Windows separators. Chunks are bounded to 8 MiB; browser chunks use 4 MiB. Durable file lengths recover upload cursors after restart. Duplicate identical chunks are idempotent; conflicting overlaps fail.

Commit checks complete lengths/digests, copies into archive-local staging, rereads destination SHA-256, publishes files and then item/evidence. Source uploads are retained. Integrity means those bytes match, not catalog identity. Reverification appends evidence. Catalog verification, reconstruction, offsets and audio derivatives require separate providers and independent oracles.

Artwork is stored as an original image plus a bounded PNG thumbnail. Image decoding has dimension/allocation limits. Both asset and original-media requests enforce collection access and asset visibility. Private attachments are not merely hidden in HTML.

## HTTP exposure and trust

Three surfaces: client, curation, administration. Public curation and administration default off. Public administration implies curation exposure; roles remain mandatory. Routers omit disabled pages, JavaScript management entrypoints and mutation APIs. The public listener stays public regardless of `Forwarded` or `X-Forwarded-For`.

The process exposes two loopback HTTP origins for separate public/private proxies and one direct LAN TLS client listener. An optional `private_tls_bind` adds direct private management HTTPS on an explicitly named interface; wildcard private TLS binds are rejected. Connect a public tunnel only to the public origin. A private proxy or Tailscale Serve connects to the private origin. LAN TLS is public-surface by default; local connectivity alone never upgrades it to administration. Allowlisted browser origins are exact and configured, not derived from proxy headers.

Native pairing carries server ID, local certificate and its SHA-256 fingerprint. Trusted enrollment is required; a fingerprint downloaded unauthenticated is not trust. The local client installs only the enrolled certificate as its trust anchor and uses normal certificate/hostname/handshake validation. No accept-invalid-certificate option exists. Certificates must include the configured LAN names/IPs. Public routes use ordinary Web PKI separately. Credentials must never follow cross-origin redirects.

Passkeys use one configured HTTPS authentication origin and standard WebAuthn. Challenges are bounded, server-side, expiring and single-use. Registration is bound to the authenticated user/device. Credential IDs are unique across accounts. CLI setup explicitly allowlists the private listener’s localhost browser origin; it does not change public exposure or trust forwarded headers. Browser sessions are HttpOnly, Secure and SameSite; mutations validate Origin. Native login can explicitly request a revocable bearer token. Tokens are random, stored as hashes, and expire after 30 days. Single-user presentation hides account selection; it is not anonymous access.

## Clients and shared audio

Artwork is part of browsing data, not a decorative optional gallery: games and CDs show cover thumbnails next to text, including search and offline snapshots. Unidentified/tentative identity is distinct from missing artwork.

Client connection/cache logic stays below Slint and SwiftUI. LAN preference is independent of browser certificate trust or passkey origin. Offline cache identity includes server, representation, and conversion profile; pinned entries survive automatic eviction. Offline content cannot be recalled by server revocation.

`junk-libs` already allows per-crate Cargo dependencies. Keep one workspace unless releases diverge. Extract disc IDs, verification, decoding, queues and DSP as narrow reusable crates, without archive/accounts/UI dependencies. Trash Amp retains its standalone local-player role. Cross-project extractions are separate migrations with sibling regression checks.

## Extension discipline

Before adding a module: identify its owned data, invariants, capabilities, persistence needs, and presentation contribution. If its implementation would bypass an existing boundary, change this architecture and its contract tests first. Avoid a second importer, authorization policy, parser or playback queue inside a frontend.
