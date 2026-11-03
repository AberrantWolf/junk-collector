# ADR 0016: Coordinated pre-release evolution with data preservation

Status: Accepted. Formalized 2026-11-03 from existing documented decisions and
implemented constraints; this is not a claim that implementation or qualification
first occurred on that date. Source reviewed: `eaff933`.

## Context

Checkpoint completion does not create a supported compatibility baseline. Actual archives and independent consumers nevertheless have preservation and recovery requirements now.

## Decision

- Before a supported baseline is explicitly declared, repository interfaces and formats may evolve together. Checkpoint names must not create compatibility wrappers, schema reservations or migration chains by themselves.
- Changes must update affected repository callers, generated contracts, fixtures and tests together. Independent consumers and external formats must have their actual requirements assessed separately.
- Existing data, identities, unknown envelopes, manual authority and recovery guarantees must be preserved. Breaking stored formats must supply an explicit safe conversion/recovery path or actionable refusal before writes; silent resets are prohibited.
- Changes to encodings identifying stored objects must be versioned and affected references converted or retained explicitly.
- A supported baseline must declare versions, upgrade/conversion, backup/restore and unsupported-format commitments before claiming those promises.

## Consequences and scope

This permits coordinated simplification without discarding real data. It does not authorize removing useful compatibility with existing archives or independent clients merely as cleanup.

## Supporting requirements and evidence

- [Accepted evolution policy and rationale](pre-release-evolution.md)
- [Stored-input/recipe identity](../../crates/collector-core/src/processing.rs)
- [Existing data migration/recovery tests](../../crates/collector-store/tests/archive.rs)

Implementation and tests are supporting evidence, not a fresh qualification run.
Detailed subsystem requirements and existing dated validation limits remain applicable.

## Reconsideration

Revisit when declaring a supported release baseline or adding sustained external consumers; document the compatibility promise explicitly.
