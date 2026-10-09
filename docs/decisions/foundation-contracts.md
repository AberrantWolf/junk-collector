# Foundation contracts

**Scope update (2026-10-09):** use [remaining work](../remaining-work.md)
and [home-lab policy](home-lab-scope.md). Older future contracts/order and blanket
history requirements below need reassessment. Preserve actual data/recovery and
valid current boundaries; Phono Junk is no longer a target.

**Status:** accepted for the first-release architecture (A01), 2026-09-09. These are target contracts. The [implementation status](../implementation-status.md) remains the source of truth for what is executable today.

This decision settles the collection foundation before A02, A03, A04 and P01 change persisted data or service boundaries. It deliberately does not migrate the current archive schema.

## Entity relationships and invariants

A collection owns its identity and inventory graph. IDs remain globally unique newtypes, but a work, edition, item, specimen, component or capture belongs to exactly one collection and cannot be moved between collections. Cross-collection identity merging and byte deduplication remain out of scope.

```text
Collection
├── Work ──< Edition
├── Item ──< Specimen ──< Component ──0..1 Carrier
│    └── optional identity link: none | Work | Edition
├── Capture ──< Representation ──< StoredFile
│    └── optional physical links: Item / Specimen / Component / Carrier
├── Asset, attached to a typed collection subject
└── Evidence, append-only and bound to exact source representations
```

- **Work and edition.** A Work represents the underlying creative work; an Edition represents a particular published/released version and may reference one Work. Confirmed editions require that relationship. A provisional migrated edition may temporarily lack a work rather than inventing one. One Work may have many Editions.
- **Item.** An Item is the collection-facing inventory/wishlist grouping. It retains a display title even when unidentified. Its identity link is one of: no catalog identity, Work, or Edition. If both the legacy work and edition references are present, the Edition relationship must agree with the Work. Identity certainty (`unidentified`, `tentative`, `selected`, `confirmed`) remains separate from the relationship itself.
- **Specimen.** A Specimen is one physical copy. Duplicate copies of one edition are separate Specimens under the same Item. Ownership, possession, acquisition, condition, loans, returns and disposal are specimen-specific histories. None of those states deletes captures, representations, assets or evidence.
- **Component and carrier.** A Component is a physical part of a Specimen: case, booklet, cartridge, disc, tape, card or another part. A Carrier is the data/audio-bearing facet of a Component, so cases and booklets are never Carriers. A Component has at most one Carrier in the initial model; a multi-disc release uses ordered Components with distinct Carrier IDs. Edition component/slot information describes expected contents and never asserts that a particular Specimen contains them.
- **Capture and representation.** A Capture is a collection-level event, not an Item child. It can optionally link to an Item, Specimen, Component and Carrier; every supplied link must form a consistent chain in the same Collection. No link is required. A Representation belongs to exactly one Capture and may derive from another Representation. Files belong to one Representation. Upload retry IDs stabilize the intended Capture and Representation before publication.
- **Asset.** An Asset is collection-owned and attaches through a typed subject reference (Work, Edition, Item, Specimen or Component). Visibility is enforced for the original, thumbnail and metadata. Presentation chooses a cover using explicit attachment/selection rules; a missing image and uncertain identity remain different states.
- **Evidence.** Evidence is immutable and source-bound. It identifies its Collection and exact Representation inputs, outcome, implementation/version, optional catalog snapshot and structured details. It may exist for an unattached Capture and therefore cannot require an Item. Integrity, structure, catalog matching, AccurateRip, reconstruction and derivative validation remain distinct kinds.
- **Metadata.** Provider observations and candidate alternatives are immutable source records. Selection and manual overrides are separately revisioned user authority. Refresh may add observations or change a provisional selection but cannot overwrite a manual selection/override. Metadata identity never implies content verification.
- **User/profile, device and server.** A User ID is the durable profile identity. Sessions, passkeys and devices refer to it in private state. Server identity is independent of hostnames and certificates. Grants relate Users to Collections; administrator status does not silently grant a collection role.
- **Job and transaction.** A Job is user-visible durable application work. A Transaction is an adapter recovery record for a multi-resource commit. A retry may reuse command identity, but Job, Transaction and domain entity IDs are never interchangeable.

### Contract examples

**Duplicate copies:** one Work, one Edition and one Item may have `Specimen A` (owned, present) and `Specimen B` (owned, lent). Returning or disposing of B changes only B's lifecycle history. Both continue to refer to preserved bytes and evidence.

**Multi-disc inventory:** a two-CD Specimen has a case Component, booklet Component, Disc 1 Component with Carrier 1, and Disc 2 Component with Carrier 2. Separate Capture events may link to each Carrier; a redumper package spanning both discs may instead remain linked at Item/Specimen level with its internal track/disc structure recorded by the music module.

**Unattached capture:** a configured inbox can preserve an unidentified package as a Capture with a preservation-master Representation and integrity Evidence while all optional physical and catalog links are absent. Later association is an authorized command; it does not rewrite the original bytes or evidence.

## Typed values and private defaults

- `LanguageTag` replaces free-form language fields at supported domain boundaries and validates/canonicalizes a BCP 47 tag while retaining raw provider values inside provider observations.
- `CurrencyCode` replaces the `String` in `Money`, accepts normalized ISO 4217-style three-letter uppercase codes, and does not infer a currency from locale. `minor_units` remains an integer; formatting/exponent policy is presentation data.
- `GeoCoordinate` stores a validated latitude/longitude pair with explicit finite ranges. Coordinates are optional and never inferred from free-form acquisition/source text.
- Existing typed dates, digests and outcomes remain explicit. Digest validation also canonicalizes lowercase hexadecimal form and applies algorithm-specific lengths.
- Private settings gain a durable `default_user_id`. Setup writes it when creating the initial account. A baseline private database without it is backfilled once with the same first-row account selected by current behavior, then resolved by User ID thereafter. Switching `multi_user` changes presentation only. Deleting or disabling the default profile requires explicit reassignment under the later account/recovery contracts.

A02 owns these concrete types, relationship validation, archive/private migrations and reference tests. A03 owns which versioned Work/Edition/Item fields a compiled module contributes.

## Evidence and transaction persistence

The approved archive model stores each evidence body once as an append-only JSON record. Entity revision snapshots retain stable evidence references needed for historical reconstruction; they do not embed a growing copy of every prior evidence record. SQLite evidence rows remain a rebuildable index.

The current `RevisionRecord { item, evidence }` format is an implementation shortcut: every Item revision copies the accumulated evidence array, and reverification increments the Item revision. P01 will migrate that shape without changing evidence IDs or claims.

Multi-resource writes use a durable phase record before mutation. Recovery distinguishes at least prepared, bytes staged, payload published, revision committed, projection applied and completed. The immutable revision-directory rename remains the archive commit point. On startup the archive adapter reconciles unfinished records before accepting writes; a full archive replay repairs or rebuilds projections but is not a substitute for transaction recovery.

Archive recovery metadata contains no credentials or grants. Any phase needed to complete a private grant/account update is stored in private operational state. Consequently, an intact installation can finish a collection-creation grant after interruption, while archive-only recovery still requires explicit access reassignment. Unknown `.staging` content is retained and reported until recovery classifies it; it is never presented as a committed revision.

P01 owns the evidence layout, transaction records, fault injection, grant/publication recovery and projection reconciliation. P02 builds durable job claims on top of that contract rather than treating transaction records as a scheduler.

## Baseline-v1 compatibility

The baseline is archive schema 1 written by commit `98e9187`: `collection.toml`, immutable numbered Item revisions, and `record.json` containing an Item plus accumulated Evidence. A02/P01 migration fixtures must include unknown module/provider JSON, multiple Item revisions, optional work/edition IDs, duplicate Specimens, legacy Carrier kinds, captures, assets, manual metadata and evidence.

- Existing Collection, Item, Work, Edition, Specimen, Carrier, Capture, Representation, Asset and Evidence IDs are preserved wherever present. Missing Work/Edition IDs remain missing; migration never creates a supposedly identified entity from a title or provider guess.
- A referenced legacy Work/Edition ID is materialized with that same ID and marked provisional when the baseline lacks a separate record. Conflicting legacy uses fail with an actionable migration error instead of silently choosing one.
- Media-bearing legacy Carriers migrate under Components while retaining their Carrier IDs. Legacy `Case` and `Booklet` values become non-carrier Components; the source revision and legacy ID/value remain in migration provenance so no field or historical identity is discarded.
- Nested Captures become collection-level records with the same Capture and Representation IDs and with their former Item association made explicit. A Capture with no Carrier remains valid.
- Accumulated evidence arrays are deduplicated by Evidence ID into append-only records. Repeated IDs must be byte-equivalent; conflicting bodies stop migration.
- Original revision files remain immutable. A02 writes versioned migrated state through its migration-only durable checkpoint and completion marker; P01 later supplies the normal multi-resource transaction adapter. Unknown module schemas and raw provider JSON round-trip byte-for-byte at their envelope boundary.
- The current default account is persisted by User ID once. Existing accounts, password hashes, sessions, passkeys, grants, server identity and TLS material are not regenerated.

## Ownership audit

| Current shortcut | Approved owner and boundary | Follow-up |
|---|---|---|
| `Item` embeds nearly the whole graph; Work/Edition are optional IDs only | Core defines collection-owned records and reference invariants; application commands coordinate changes | A02 |
| `CarrierKind` includes cases/booklets and Capture is nested under Item | Core separates Components, Carriers and collection-level Captures | A02 |
| Language/currency are strings; coordinates are absent | Core validated value objects; adapters retain raw provider observations | A02 |
| First SQLite user row is the default profile | Private store persists a selected User ID; application/account services govern reassignment | A02 |
| Unknown compiled-module schemas validate as success and modules expose descriptors only | Modules own versioned validation/migration/operations/presentation through host ports | A03 |
| Store methods directly check roles for uploads, assets, activity and jobs | Application services authorize commands/queries; Store implements mechanisms behind narrow ports | A04 |
| Store collection creation also grants ownership | Application command owns policy; transaction adapter atomically/recoverably performs requested effects | A04, P01 |
| HTTP handlers perform administrator and import/job decisions | Application services own authorization, command semantics and job policy; HTTP retains principal extraction, CSRF, limits and listener composition | A04 |
| Evidence is copied into each revision and SQLite ignores duplicate IDs | Append-only archive evidence with strict identity/content validation and rebuildable projection | P01 |
| Startup reindex is relied on after a revision rename; `.staging` is only ignored | Durable transaction recovery precedes writes; replay remains projection repair | P01 |

## Capability inventory

| Accepted capability | Current foundation | Missing owner |
|---|---|---|
| Collection inventory, browse/search and artwork rows | Basic Item aggregate, FTS projection and cover assets exist | A02, A04, U03, U04 |
| Work/edition/specimen/component relationships and lifecycle | Specimen basics exist; Work/Edition records and Components do not | A02, U03 |
| Unidentified preservation and resumable upload/inbox | Item-bound uploads retain sources and stable representation IDs | A02, P01–P04 |
| Append-only provenance and independent verification | Typed evidence exists; only integrity checks run and evidence is accumulated in snapshots | P01, D01–D06 |
| Compiled module schemas, migrations, operations and presentation | Registration and validation descriptors exist | A03 |
| Authorized commands, generated contracts, pagination/idempotency/events | Partial service boundary and hand-written DTOs exist | A04, P02, C06 |
| Durable jobs and archive recovery/export/reindex | Persisted job status and full startup reindex exist | P01, P02, P05, P06 |
| Provider metadata/manual resolution and assets | Candidate resolution and cover pipeline exist | D06, U04 |
| Media readers, derivatives, playback and offline cache | Trust/cache policy primitives and ranged original reads exist | D03, D07, C03–C07 |
| Accounts, grants, recovery, enrollment and rotation | Passwords, grants, sessions and WebAuthn basics exist | U02, S01–S04 |
| Listener exposure, local setup and browser guidance | Separate routers/listeners and setup/doctor exist | U01, R01, R03 |
| Discovery, health checks and route failover | Endpoint preference policy only | C01, C02 |
| Legacy import, deployment and performance evidence | Not implemented | P05, R01–R03 |

No new backlog ticket is required by this decision: the foundation gaps are bounded by A02, A03, A04 and P01, with downstream capability work retaining its existing ticket ownership.

## Wave 1 review corrections

Standalone capture revisions are authoritative. Embedded Item captures are transitional recovery snapshots/read projections, not a second mutable owner. Reindex cannot lower capture revisions from those snapshots; stale item writes conflict with newer independent capture state. Capture associations resolve complete ancestry even when callers omit intermediate links. IDs for nested physical entities and representations are unique within and across collections. Assets reference existing graph subjects.

The migration-specific checkpoint is real durability, not an exemption from it: publish complete files/revision directories atomically, sync their ancestor directories, regenerate only uncheckpointed outputs, and refuse changes to checkpointed content. The same typed revision publisher is used by migration and current runtime publication. General transaction recovery remains P01.

Module migrations default to additive validation, but an explicit version-specific transformation policy can rename/convert fields with a recorded rationale. The registry retains each exact source/intermediate envelope as provenance; consumers must commit it with the migrated result. Capability registration validates module ownership and uniqueness into an explicit dispatch index. Preservation does not require retaining obsolete field layouts forever.

Language values validate BCP 47 syntax through the `language-tags` parser, including private-use and extension tags, without requiring membership in a potentially stale registry. Raw provider observations remain untouched.
