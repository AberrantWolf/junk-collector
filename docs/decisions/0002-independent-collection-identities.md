# 0002: Independent collection and physical identities

Status: proposed, not yet reviewed by the owner.

## Context

A catalog description, an owned copy and bytes captured from media are different
facts. Merging them makes unidentified captures and duplicate copies unreliable.

## Decision

A collection owns its inventory graph. IDs are stable newtypes, never paths, row
numbers or titles. Every entity belongs to exactly one collection and cannot move
between collections; cross-collection merging and byte deduplication are out of
scope.

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

- **Work / Edition**: the creative work and a particular release of it. A
  confirmed edition references its work; a provisional one may not yet.
- **Item**: the collection-facing inventory or wishlist entry. It keeps a title
  even when unidentified. Identity certainty (unidentified, tentative, selected,
  confirmed) is separate from the link itself.
- **Specimen**: one physical copy. Ownership, possession, condition, loans and
  disposal are per-specimen history and never delete captures, assets or evidence.
- **Component / Carrier**: a physical part (case, booklet, disc, cartridge) and
  its data-bearing facet. Cases and booklets have no carrier; a multi-disc
  release has one component and carrier per disc.
- **Capture / Representation**: a capture is a collection-level event with
  optional links to item, specimen, component and carrier; any supplied links must
  form a consistent chain. Representations belong to one capture and may derive
  from another.
- **Asset**: collection-owned artwork or documents attached to a typed subject.
  Missing artwork and uncertain identity are different states.
- **Evidence**: immutable, bound to exact representation inputs, implementation
  version and optional catalog snapshot. It may exist for an unattached capture.
- **Users and grants** live in private state. Administrator status does not grant
  a collection role.
- **Jobs and transactions**: a Job is user-visible durable work; a transaction is
  an adapter recovery record. Their IDs are never interchangeable with each other
  or with domain IDs.

Rules:

- Importing digital files does not create physical specimens. Guided physical
  intake may create the copies the user reviewed. Importing bytes alone never
  confirms catalog identity.
- Disposal, tombstones and changed associations never delete originals or evidence.
- Language, currency, coordinates, dates and digests use validated types
  (`LanguageTag` as BCP 47, ISO 4217-style currency codes, lowercase hex digests);
  raw provider values stay inside provider observations.

## Consequences

Duplicate copies and unidentified captures work, at the cost of explicit
relationships. Workflows don't have to fill every optional link.

See: [core records](../../crates/collector-core/src/lib.rs).
