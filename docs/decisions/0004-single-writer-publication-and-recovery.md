# ADR 0004: Single-writer publication and recovery

Status: Accepted. Formalized 2026-11-03 from existing documented decisions and
implemented constraints; this is not a claim that implementation or qualification
first occurred on that date. Source reviewed: `eaff933`.

## Context

Archive publication spans filesystem artifacts and database projections. Interrupted work must be distinguishable from committed history before new mutation proceeds.

## Decision

- One active archive writer must own publication. Cross-host deployment must fence other writers operationally; local locks or matching archive UUIDs must not be treated as leader election.
- Publication must journal recoverable phases and bind artifacts to expected identities and revisions. Immutable revision publication must define the archive commit boundary.
- Startup must reconcile known interrupted publications before accepting writes. Reindex must repair projections without replacing transaction recovery.
- Recovery must preserve committed revisions and unknown staging, and must fail closed on inconsistent artifacts rather than silently reverting authority.
- Ordinary startup must require the intended enrolled archive/private-state pair and supported mount conditions. Unavailable or substituted storage must not be initialized as an empty installation.

## Consequences and scope

This favors explicit recovery over concurrent archive writers. Hard filesystem waits and external storage durability remain deployment constraints; this decision makes no universal power-loss or deadline guarantee.

## Supporting requirements and evidence

- [Publication phases and fault boundaries](../../crates/collector-store/src/publication.rs)
- [Archive admission guard](../../crates/collector-store/src/storage_guard.rs)
- [Fail-closed storage tests](../../crates/collector-store/tests/storage_guard.rs)
- [Publication failure/recovery tests](../../crates/collector-store/tests/archive.rs)

Implementation and tests are supporting evidence, not a fresh qualification run.
Detailed subsystem requirements and existing dated validation limits remain applicable.

## Reconsideration

Revisit only with a concrete multi-writer or storage protocol, explicit commit authority and demonstrated failure/recovery behavior.
