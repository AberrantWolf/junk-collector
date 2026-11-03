# ADR 0002: Independent collection and physical identities

Status: Accepted. Formalized 2026-11-03 from existing documented decisions and
implemented constraints; this is not a claim that implementation or qualification
first occurred on that date. Source reviewed: `eaff933`.

## Context

A catalog description, an owned copy and bytes captured from media represent different facts. Combining them makes unidentified preservation and duplicate-copy inventory unreliable.

## Decision

- Collections must own their inventory graph. Entity IDs must remain stable identities rather than paths, row numbers or display titles.
- Works, editions, inventory Items, specimens, components, carriers, captures and representations must retain distinct roles. Ownership, possession, wishlist state and catalog identity must not be inferred from one another.
- Captures must support preservation without physical or catalog associations. Any supplied associations must form a consistent chain within the owning collection.
- Digital-file intake must not fabricate physical specimens. Guided physical intake may establish the explicitly reviewed copies; importing bytes alone must not establish confirmed catalog identity.
- Disposal, tombstones and changed associations must not implicitly delete originals or source-bound evidence.

## Consequences and scope

This supports duplicate copies and unidentified captures at the cost of explicit relationships. It does not require every workflow to fill every optional relationship or preserve an incidental UI arrangement.

## Supporting requirements and evidence

- [Foundation relationships and examples](foundation-contracts.md#entity-relationships-and-invariants)
- [Core identities and records](../../crates/collector-core/src/lib.rs)
- [Retained inventory through removal/reindex](../../crates/collector-store/tests/archive.rs)

Implementation and tests are supporting evidence, not a fresh qualification run.
Detailed subsystem requirements and existing dated validation limits remain applicable.

## Reconsideration

Revisit for a concrete collection model that the graph fails to represent, retaining identities, originals and meaningful inventory facts.
