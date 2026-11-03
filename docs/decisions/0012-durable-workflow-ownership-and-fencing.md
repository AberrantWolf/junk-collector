# ADR 0012: Durable workflow ownership and fencing

Status: Accepted. Formalized 2026-11-03 from existing documented decisions and
implemented constraints; this is not a claim that implementation or qualification
first occurred on that date. Source reviewed: `eaff933`.

## Context

Panels disappear, workers lose claims and multiple requests share expensive child work. UI lifetime or a frozen ownership flag is insufficient to govern execution.

## Decision

- Admitted workflows must retain durable request identity and ownership independently of UI observation lifetimes. Closing a panel must not implicitly cancel server work.
- Shared work must retain durable requester references. Cancelling one owner must not cancel work still required by another; last-owner cancellation must fence further child admission.
- Convergent requests and retries must retain the first immutable Job inputs and creation timestamp.
- Job claims, generations, source currency, permissions and cancellation must be checked at execution/publication boundaries. Stale workers must not publish through an older claim.
- Child admission and parent ownership must be committed under the same effective claim fence. Upstream outcomes must commit before downstream work advances.
- Recovery must revisit all pages of unfinished workflows periodically and after restart. Terminal/cancelled state and retry fences must persist; running capacity must not be reused before work stops.

## Consequences and scope

Workers may recover or converge while preserving first immutable Job inputs/timestamps. Lease durations, scheduling weights and recovery intervals remain tunable mechanisms.

## Supporting requirements and evidence

- [Shared acquisition ownership](../../crates/collector-store/src/acquisitions.rs)
- [Transactional Job claims](../../crates/collector-store/src/jobs.rs)
- [Later-owner/last-owner and stale-worker tests](../../crates/collector-store/tests/archive.rs)
- [UI continuation scoping](../../web/src/operation-scope.ts)

Implementation and tests are supporting evidence, not a fresh qualification run.
Detailed subsystem requirements and existing dated validation limits remain applicable.

## Reconsideration

Revisit for a new scheduler or shared-work type, preserving late-owner, cancellation, response-loss and restart guarantees.
