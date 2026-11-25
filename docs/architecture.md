# Architecture

Junk Collector is one Rust server (`collector-server`) with an embedded
React/TypeScript web UI. It stores collections in a portable on-disk archive and
keeps derived and private state in local SQLite. This page covers what a new
contributor needs to know before changing a subsystem. Feature-level behavior
lives in the guides linked at the end; proposed (unreviewed) design decisions are in
[decisions/](decisions/README.md).

## Crates and layering

| Crate | Owns | Must not depend on |
| --- | --- | --- |
| `collector-core` | Typed IDs, portable records, errors | HTTP, filesystem, database, GUI, devices |
| `collector-api-types` | Serializable read DTOs shared with clients | Services, storage, transport |
| `collector-modules` | Game and music schemas, interpretation, eligibility, result validation | Provider requests, scheduling |
| `collector-app` | Use cases, authorization, metadata authority, workflow policy, ports | Concrete adapters |
| `collector-store` | Archive publication, SQLite catalog and private state, recovery | GUI, server |
| `collector-providers` | Bounded HTTP clients, catalog/DAT parsing, mail, encoder processes | Store, server, GUI |
| `collector-server` | Listeners, routers, HTTP handlers, workers, embedded web assets | — |
| `collector-media` | Isolated audio delivery over a local socket, read-only archive access | App, store, providers |
| `collector-client` | Enrolled transport, endpoint trust, offline cache | Server storage |
| `collector-cd-identity` | Standalone CD capture naming tool | — |
| `junk-libs-pcm` | Streaming PCM decoding/adaptation for FLAC, MP3, WAV | Collector crates |

The rules:

- Application services own policy (who may do what, when work is eligible,
  which metadata wins). Adapters own mechanisms behind ports. HTTP handlers call
  application services, never the Store directly.
- Frontends do not reimplement domain logic, importers or authorization.
- Parsing, hashing, decoding, normalization and audio algorithms go in narrow
  shared crates. Most live in the sibling [`junk-libs`](https://github.com/AberrantWolf/junk-libs)
  workspace, pinned by Git revision in `Cargo.toml`; no sibling checkout is
  needed to build. When extracting shared code, check the other consumers of
  that crate. Do not copy application orchestration from older projects.
- Blocking storage and CPU work runs outside async executor threads.
- If a new capability has no natural owner in this table, decide where it goes
  before writing it.

`python3 scripts/check_architecture.py` enforces the crate dependency directions,
rejects `state.store` use in `collector-server` handlers, and checks relative
links in the docs.

## Data model

A collection holds either games or music and is bound to the module that owns
its type. Inside it, these identities are separate Rust newtypes and never
interchangeable: work, edition, item, specimen (a physical copy), component,
carrier, capture, representation, package file, asset, evidence, processing
observation, snapshot and recipe. Paths and SQLite row IDs are never identity.

- An item can exist without files (inventory, wishlist) and a capture can exist
  without an item (an unidentified dump).
- Ownership, possession, wishlist, loans and disposal are independent facts. A
  sale or loan never deletes archived bytes; removal is a tombstone.
- Permanent deletion is a separate, confirmed action. A removed entry can be
  deleted entirely: its item, capture, asset, evidence, recipe and metadata
  folders, its unshared analysis results, and its catalog and private rows
  (imports, jobs, history), plus the placeholders of entries merged into it. Its
  id is recorded so it can't be published again. Alternatively, chosen originals
  of an entry that stays can be deleted, optionally with the copy records only
  they came from; the entry then gets a new revision. Works, editions, snapshots,
  list entries and analysis results another dump shares are kept, as are ids in
  a surviving entry's metadata lookup history and evidence owned by a merged
  placeholder. Originals and covers an older archive kept inside the entry's
  folder go with it, as do that entry's schema migration markers (startup
  checks each marker's entry exists, so they are removed first). An entry
  whose folder still holds another entry's files can only stay removed.
- A combine record names the exact files of every entry it combined, and is
  checked when the archive loads. Deleting any of those files retires the
  record: it keeps the surviving members, and its undo data is replaced by a
  note, so that combine can no longer be undone.
- Importing a dump does not invent a physical copy unless the user confirms one
  (guided physical imports default to one owned copy). Digital purchases create
  no specimen.
- Module data is stored as a lossless raw JSON envelope. Known schemas are parsed
  into module-owned views; unknown fields and newer or unsupported schemas are
  preserved and round-trip unchanged. Older schemas migrate one version at a
  time. Being readable and being writable are separate checks: an item whose
  module is missing stays readable but cannot be edited.
- Manual metadata and artwork choices override provider data and survive
  refreshes. Provider results are candidates or observations until accepted.

## Archive and storage

The archive is a readable directory tree of TOML manifests, immutable revisions,
original files, assets and JSON evidence. It holds no credentials or grants:

```
collections/<name>--<collection-id>/
  collection.toml
  works/<id>/revisions/<n>/...
  editions/<id>/revisions/<n>/...
  items/<title>--<id>/revisions/<n>/...
  captures/<id>/revisions/<n>/...
  captures/<id>/payload/<representation-id>/<original package paths>
  assets/<id>/payload/original.<ext>, thumbnail.png
  evidence/<id>/revisions/<n>/...
```

Two SQLite databases sit beside it on local disk (even when the archive is on a
NAS):

- **Catalog**: a rebuildable projection of the archive for browsing and search.
  On startup it is reused if its checkpoint matches the archive's metadata file
  observations; otherwise it is rebuilt from the archive. `reindex` forces a
  rebuild.
- **Private state**: accounts, credentials, grants, durable Jobs, command
  receipts, pending uploads and mail settings. This cannot be rebuilt from the
  archive.

Schema version constants live in `collector-store`.

### Writes and recovery

- One archive writer owns staging, journals and publication. The rename of an
  immutable revision directory is the commit point.
- On startup, interrupted publications are settled before any new write is
  accepted. Unknown staging is kept, not deleted.
- A permanent deletion saves its plan in `state/item-purges/` first. Step one
  retires combine records, moves folders into `archive/.purge/`, drops their
  rows and deletes the moved folders; startup repeats it for any saved plan
  before checking the catalog. Step two saves the surviving entry, if any, once
  the store is open. Until then a rebuild treats the plan's evidence as gone.
- Expected revisions reject stale edits.
- Archive identity is bound to the local state and remote-mount guards fail
  closed. Running two servers against one archive is not supported and is not
  detected across hosts — the operator must prevent it.
- Imports stage and verify payload bytes on the archive filesystem before taking
  the writer; a single publication worker then promotes them.
- Catalog reads use separate read-only WAL connections, so browsing stays
  responsive during imports.

### Commands

Mutations go through application commands that take an `Idempotency-Key`. The
receipt is bound to a digest of the validated typed request before any mutable
configuration is resolved: a retry with the same input returns the original
result, a changed input with the same key is rejected. Receipts commit together
with their effect (in the same SQLite transaction, or via a publication marker
recovered at startup). Authentication ceremonies, upload chunks and one-time
secret issuance use their own state machines instead. Details are in the
[API contract](../api/README.md).

## Background work

Durable Jobs run imports, integrity checks, catalog matching, metadata and
artwork lookups, fingerprinting and audio preparation.

- Jobs freeze their inputs at admission. A `SourceFingerprint` covers the typed
  source, every file's length and SHA-256 in order, and parameters; changed
  inputs make old results historical.
- Publication is fenced by the Job claim, generation, source identity,
  authorization and cancellation; the Store checks the claim itself. A cancelled
  or superseded Job cannot publish.
- Shared child work records its requesters; cancelling the last owner cancels it.
  Batch jobs (album preparation, bulk imports) fan out to per-track Jobs.
- Upstream results commit before downstream work advances. A periodic recovery
  loop resumes unfinished workflows after restart.
- Processing defaults to four workers; provider lookups are capped overall and to
  one active Job per provider. See [configuration](configuration.md).
- UI lifetimes never own Jobs: closing a panel aborts its reads, not server work.
- Missing optional tools or providers never block basic preservation.

## Evidence

Integrity hashes, structural checks, MusicBrainz identity, AccurateRip results
and game DAT matches are different claims and recorded separately. Evidence
records the exact source bytes, algorithm/version, coordinates and coverage.
Partial or ambiguous results never count as full verification, and MusicBrainz
identity never proves audio correctness. Repeated identical provider responses
are stored once; equivalent lookups do not need a permanent history.

## Reads and playback

- Library reads use revision-bound projections with independent counts,
  byte-bounded pages and tracks loaded on demand. Clients track their own refresh
  progress and caches; the server keeps no client refresh sessions.
- Browsing, searching, availability polling and job admission never read or hash
  full payloads, and never start providers or encoders.
- Playback streams requested bytes (with ranges and ETags) without verifying the
  whole file first. Read errors surface on the media request. Integrity checks are
  separate, explicit work.
- Live playback can transcode to FLAC or Opus per stream without touching the
  archive or creating Jobs. Saved preparations (FLAC/Opus derivatives) are
  explicit, recorded with their recipe, and lossless output is checked against
  the source PCM.
- Media is exposed through authorized opaque readers, never archive paths.
- Artwork is shown beside game and music text in browsing, search and offline
  views. Missing artwork and uncertain identity are different states.

## Exposure and authentication

- There are three surfaces: client, curation and administration. Public curation
  and administration are off by default; enabling public administration also
  exposes curation. Disabled surfaces are not routed at all (pages, scripts and
  APIs).
- Whether a request is public or private comes from which listener it arrived
  on, never from source IP, DNS or forwarded headers. The server provides a
  public loopback origin, a private loopback origin, a LAN TLS listener, and an
  optional private TLS listener on a named interface.
- Exposure grants no collection role; roles (Viewer, Curator, Owner) are always
  enforced, and server administrators get no implicit collection access.
- Passwords use Argon2id; passkeys use standard WebAuthn on one configured HTTPS
  origin. Browser sessions are HttpOnly, Secure, SameSite cookies, and
  state-changing requests require an exact allowed Origin.
- Native clients enroll through a browser approval and then trust only the
  server's enrolled certificate. There is no "accept invalid certificate" option.
- Authentication throttling is in-process and resets on restart.

See [configuration](configuration.md) and [security](decisions/security.md).

## API

Route groups are annotated once and drive both Axum routing and the OpenAPI
document `api/openapi.v1.json`; `web/src/generated/api.ts` is generated from it.
Neither is hand-edited. After changing a route or DTO, run
`python3 scripts/api_contract.py --write`; CI runs it without `--write` to check
for drift. See the [API contract](../api/README.md).

## Web UI

The UI lives in `web/src` and is bundled into the server binary at build time, so
rebuild the server after changing web assets. Asynchronous UI work uses
`OperationScope`, which ties a request to the current target so a late response
cannot act on a view the user has left. Visual style is described in
[design](design.md).

## Adding a module or subsystem

Before adding a collection type or subsystem, identify its data, invariants,
capabilities, persistence and UI contribution, and where each sits in the layer
table. If it would bypass an existing boundary, change the boundary first. Avoid
a second importer, authorization check, parser or playback queue in a frontend.

## Feature guides

- [Game identification and DAT catalogs](game-identification.md)
- [Digital purchases and album matching](digital-purchases.md)
- [Audio support](audio-preparation.md) and [game-disc audio](game-disc-audio.md)
- [Library reads](layered-library-refresh.md) and [player metadata](album-metadata-consumers.md)
- [Private playback](automatic-private-playback.md) and [direct HTTPS media](../deploy/container/media.md)
- [Email purchases](email-purchases.md) and [inbox cleanup](inbox-cleanup.md)
- [Archive export](archive-export.md) and [server operations](server-operations.md)
- [Product requirements](release-requirements.md)
