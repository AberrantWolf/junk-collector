# 0008: Meaningful provenance, bounded lookup history

Status: proposed, not yet reviewed by the owner.

## Context

A lookup is not a collection event. Keeping every provider fetch forever costs
storage without recording anything new about the collection.

## Decision

- Preserve originals, user-entered facts, manual authority, substantive evidence
  and enough source data to back retained metadata and artwork claims.
- Lookup times, request counts and identical responses are not permanent history.
  Bounded cache and diagnostic state is fine where it helps retries.
- Reuse of a provider response follows that provider's policy and checks actual
  inputs, bytes and interpretation; never equivalence by name alone or across
  collections.
- Compacting existing records requires explicit reference rules, a dry run, a
  verified backup and interruption-safe conversion. Unknown data and retained
  claims stay intact.
- Completed jobs, expired credentials, dismissal or unreachability alone don't
  authorize deletion while retry or recovery still depends on the data.

## Consequences

Equivalent-response reuse is allowed; generic garbage collection is not.

See: [snapshot compaction](../../crates/collector-store/src/snapshot_compaction.rs),
[snapshot duplicate conversion](../snapshot-duplicate-conversion.md).
