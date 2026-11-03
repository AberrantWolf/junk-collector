# ADR 0005: Lossless module data and write eligibility

Status: Accepted. Formalized 2026-11-03 from existing documented decisions and
implemented constraints; this is not a claim that implementation or qualification
first occurred on that date. Source reviewed: `eaff933`.

## Context

Module availability changes independently of retained archives. Treating unknown content as executable or dropping it during edits would lose data or misstate support.

## Decision

- Module payloads and unknown historical authority must round-trip without loss at their defined envelope boundary.
- Authorized historical reads must remain separate from write and execution eligibility. A missing module or unsupported schema must not hide independently authorized historical evidence, artwork or media.
- Content mutations and execution must require the owning installed module, supported schema, current binding and appropriate authorization.
- Migration must preserve exact source/intermediate envelopes and unknown fields. Transformations must have an explicit version-specific policy and rationale; ordinary edits must not downgrade or overwrite unsupported data.
- Core and generic clients must not reinterpret arbitrary module payloads as a universal domain schema.

## Consequences and scope

Unknown data remains preserved without being advertised as executable. Compiled registration is the current mechanism; runtime plugin loading and a stable native ABI are not promised.

## Supporting requirements and evidence

- [Lossless envelopes](../../crates/collector-core/src/lib.rs)
- [Migration and module validation](../../crates/collector-modules/src/lib.rs)
- [Read/write eligibility policy](../../crates/collector-app/src/collection_types.rs)
- [Assignment and unknown-schema tests](../../crates/collector-app/tests/collection_types.rs)

Implementation and tests are supporting evidence, not a fresh qualification run.
Detailed subsystem requirements and existing dated validation limits remain applicable.

## Reconsideration

Revisit with a real new schema or extension consumer, preserving authorized historical reads and explicit conversion semantics.
