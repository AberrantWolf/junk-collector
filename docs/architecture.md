# Architecture

Junk Collector preserves evidence and owned collection history independently of how content is consumed. The server is a modular monolith: one process composes independent domain, application, storage, authentication, HTTP, and job capabilities. It is not a set of microservices.

## Dependency rules

- `collector-core`: portable typed identities, records and errors. No filesystem, database, networking, or presentation.
- `collector-modules`: module-owned schemas and capability declarations. No provider requests or application scheduling.
- `collector-app`: use cases and authorization over storage ports. Frontends do not bypass these rules.
- `collector-store`: archive publication, local catalog projection, private state, package/asset storage. Its file services enforce the same collection roles.
- `collector-server`: HTTP composition, WebAuthn ceremonies, route exposure, worker limits, static UI delivery. Blocking storage/CPU work runs outside async workers.
- `collector-client`: endpoint trust, transport, offline cache policy. No server storage dependency.

Domain modules return observations, candidates, and verification results. They never decide to replace an archive, switch a view, scrape another provider, or schedule work. The host owns those decisions. A test-only stamp module proves registration does not require modifying a universal enum.

Module wire payloads are versioned JSON, deserialized into domain-owned Rust types for supported schemas. Unknown schemas survive unmodified. Raw provider JSON is source evidence, not the common field model. Runtime plugins will adapt the same capability boundaries; a native ABI is deliberately not promised.

## Identity and ownership

Collection, item, work, edition, specimen, carrier, capture, representation, asset, evidence, user, device, and server IDs are distinct Rust newtypes. Paths and SQLite row numbers are never identity. No physical specimen is fabricated by importing a dump. Wishlists, ownership, possession, loan records, and disposal are independent. A tombstone does not delete payloads.

Game and music data live in module-owned types. Core inventory remains useful without a dump or definitive identity. Metadata candidate ranking uses identity strength before preferred language. Manual choices and overrides are separate from observations. A provisional display selection never becomes checksum evidence.

## Archive and publication

The archive is portable and contains no credentials or access grants:

```
collections/<readable-name>--<collection-id>/
  collection.toml
  items/<readable-title>--<item-id>/
    revisions/<zero-padded-revision>/
      manifest.toml
      record.json
    files/<representation-id>/<original-package-paths>
    assets/<asset-id>/original.<format>
    assets/<asset-id>/thumbnail.png
```

TOML manifests contain schema version, typed identities, revision and the SHA-256 of `record.json`. The companion JSON contains the typed snapshot and accumulated evidence. This split preserves arbitrary future module/provider JSON (including null values) without TOML conversion loss or stringifying every domain field. Original JSON is readable and schema-versioned by its manifest/module envelopes.

A single recoverable process lock owns archive writes. An update checks expected revision under the writer lock, writes and syncs an immutable revision in `.staging`, atomically renames it to its final revision directory, syncs the directory, then updates the SQLite projection in a transaction. The rename is the commit point. Startup reindexing verifies metadata digests and replays committed revisions; incomplete `.staging` work is ignored and retained. Payloads publish before their referencing revision; interrupted operations can leave unreferenced files, never a published reference to partial bytes. Imports retry against stable representation IDs.

Collection creation currently writes one immutable collection manifest. Account grants are private state; a crash before grant publication leaves an ungranted collection for explicit administrator recovery. There is no automatic grant reconstruction.

SQLite state must be local to the server. `catalog.sqlite` is rebuildable; `private.sqlite` contains users, devices, credentials, grants, jobs, personal activity and server identity. Back up both the archive and private state. Copying only the archive restores records but no access. SQLite databases must be backed up through SQLite's backup mechanism or while stopped, not by copying active WAL files independently.

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
