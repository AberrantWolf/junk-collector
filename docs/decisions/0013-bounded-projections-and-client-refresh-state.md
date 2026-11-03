# ADR 0013: Bounded projections and client-owned refresh state

Status: Accepted. Formalized 2026-11-03 from existing documented decisions and
implemented constraints; this is not a claim that implementation or qualification
first occurred on that date. Source reviewed: `eaff933`.

## Context

Whole-item hydration and serialized whole-library refresh couple browsing to expensive work and prevent progressive results. Clients need independently useful published data.

## Decision

- Library browsing must use authorized bounded projections, independently queryable counts, revision-bound pagination and demand-loaded track data.
- Counts, filters and pages must use consistent eligibility and revision semantics. Pending or failed derived state must remain explicit rather than presenting stale conclusions as current.
- Clients must own refresh progress, page accumulation, cancellation, retries and cache comparison. The server must not maintain client refresh sessions or cache inventories as catalog authority.
- Browsing, availability polling and cache validation must not read/hash full original payloads or initiate provider acquisition or encoding.
- Album admission must use recorded observations instead of full-payload reads/hashing. Explicitly requested acquisition or encoding must be admitted through durable background commands.
- Changed authorization, source, language and authority must invalidate the relevant projection/cache view. Read retries must remain bounded and cancellable; they must not resubmit mutations.

## Consequences and scope

This enables progressive rendering without promising a particular SQL layout, page size, contention timeout or universal latency bound. Explicit pending work remains distinct from background enrichment requests.

## Supporting requirements and evidence

- [Layered read contract](../layered-library-refresh.md)
- [Projection/count/page implementation and tests](../../crates/collector-store/src/library.rs)
- [Player read sequence](../album-metadata-consumers.md)

Implementation and tests are supporting evidence, not a fresh qualification run.
Detailed subsystem requirements and existing dated validation limits remain applicable.

## Reconsideration

Revisit when a real read workflow needs a different projection, retaining boundedness, current authorization and stateless server pagination.
