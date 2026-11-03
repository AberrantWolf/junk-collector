# ADR 0011: Input-bound command receipts and effects

Status: Accepted. Formalized 2026-11-03 from existing documented decisions and
implemented constraints; this is not a claim that implementation or qualification
first occurred on that date. Source reviewed: `eaff933`.

## Context

Commands can commit while their responses are lost. Retrying against newly resolved configuration or uncoordinated effects risks duplicate or different mutations.

## Decision

- Application commands using receipts must bind retry identity to the submitted typed inputs before resolving mutable configuration.
- Repeated matching requests must recover the committed result; reusing the identity with changed inputs must be rejected. Expected revisions must independently reject stale mutations.
- Receipts and effects must share their owning transaction or an explicit recoverable publication-marker protocol. A success receipt must not precede its committed effect.
- Interrupted command outcomes must be reconciled before a new attempt performs the effect. Malformed or inconsistent receipts must not be silently discarded to start a fresh command.
- Shared effects must use one effect implementation rather than nesting independently committed commands. Authentication ceremonies, upload offsets and one-time issuance must retain their explicit alternative retry semantics.

## Consequences and scope

This gives commands recoverable replay semantics, not universal exactly-once execution across arbitrary external services. Key syntax, endpoint DTOs and exceptions belong in the API contract.

## Supporting requirements and evidence

- [API mutation contract and exceptions](../../api/README.md#mutation-contract)
- [Receipt/effect persistence](../../crates/collector-store/src/commands.rs)
- [Concurrent/restart replay tests](../../crates/collector-store/tests/archive.rs)

Implementation and tests are supporting evidence, not a fresh qualification run.
Detailed subsystem requirements and existing dated validation limits remain applicable.

## Reconsideration

Revisit for new mutation families or external side effects, defining both response-loss and interrupted-publication recovery.
