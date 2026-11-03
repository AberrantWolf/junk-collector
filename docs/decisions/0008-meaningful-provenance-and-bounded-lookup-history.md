# ADR 0008: Meaningful provenance and bounded lookup history

Status: Accepted. Formalized 2026-11-03 from existing documented decisions and
implemented constraints; this is not a claim that implementation or qualification
first occurred on that date. Source reviewed: `eaff933`.

## Context

Earlier blanket history assumptions conflated meaningful collection evidence with repeated provider operations. Equivalent lookup copies add storage without explaining a new collection fact.

## Decision

- Originals, user-authored collection facts, manual authority, substantive evidence and sufficient sources for retained metadata/artwork claims must be preserved.
- Lookup time, request occurrence or equivalent response bytes must not by themselves impose a permanent collection-history requirement.
- Response reuse must follow the owning provider policy and validate equivalence against actual inputs, bytes and interpretation; it must not infer cross-collection or merely name-based equivalence.
- Existing record compaction must require explicit reference/replay rules, dry-run effects, verified backup and interruption-safe recovery. Unknown data and retained claims must remain intact.
- Completed Jobs, expired credentials, dismissal and current graph reachability must not alone authorize deletion. Existing retry/recovery dependencies must be respected until a safe expiry/conversion policy exists.

## Consequences and scope

This permits justified response reuse without authorizing generic garbage collection. Current operational-row retention is an implementation dependency, not a perpetual requirement for future formats.

## Supporting requirements and evidence

- [Accepted scope and retention revision](home-lab-scope.md#collection-provenance-versus-operational-history)
- [Owned retention policies](../retention-audit.md)
- [Explicit conversion](../../crates/collector-store/src/snapshot_compaction.rs)
- [Conversion and replay regressions](../../crates/collector-store/tests/archive.rs)

Implementation and tests are supporting evidence, not a fresh qualification run.
Detailed subsystem requirements and existing dated validation limits remain applicable.

## Reconsideration

Revisit for demonstrated growth and a complete consumer/reference assessment; a new cleanup engine must not silently redefine preservation.
