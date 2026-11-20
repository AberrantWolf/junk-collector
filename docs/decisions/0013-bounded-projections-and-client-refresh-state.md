# 0013: Bounded projections; clients own refresh state

Status: proposed, not yet reviewed by the owner.

## Context

Hydrating whole items and refreshing the whole library in one pass ties browsing
to expensive work and blocks progressive results.

## Decision

- Library reads use authorized bounded projections: independently queryable
  counts, revision-bound pagination and tracks loaded on demand.
- Counts, filters and pages share eligibility and revision semantics. Pending or
  failed derived state is shown as such.
- Clients own refresh progress, page accumulation, retries and cache comparison.
  The server keeps no client refresh sessions or cache inventories.
- Browsing, polling, cache validation and album admission never read or hash full
  originals, and never start provider lookups or encoding. Requested acquisition
  and encoding go through durable background commands.
- Changes to authorization, source, language or authority invalidate the affected
  views. Read retries are bounded and never resubmit mutations.

See: [library refresh](../layered-library-refresh.md),
[library queries](../../crates/collector-store/src/library.rs).
