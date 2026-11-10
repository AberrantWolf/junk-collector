# ADR 0003: Portable collection authority and private operational state

Status: Accepted. Formalized 2026-11-03 from existing documented decisions and
implemented constraints; this is not a claim that implementation or qualification
first occurred on that date. Source reviewed: `eaff933`.

## Context

Collection history must remain useful independently of one installation. Accounts and access grants must not be reconstructed from portable archive contents.

## Decision

- Portable archive records must retain originals, inventory, manual authority, accepted sources, evidence and unknown collection data.
- Accounts, credentials, grants, server trust/identity and durable operational work must remain private operational authority. Catalog projections must remain rebuildable rather than an independent source of collection truth.
- SQLite operational state must use supported local storage, including when archive payloads reside on supported remote storage.
- Full recovery must preserve a consistent archive, private state and configuration together. A restored identity must not run concurrently with another copy of that identity.
- Portable export/import must exclude operational credentials and grants. Recovery into a fresh identity must require explicit initial ownership assignment; administrator status alone must not supply collection ownership.

## Consequences and scope

Full backup and portable transfer intentionally have different security and recovery semantics. This does not freeze database filenames, archive serialization or the current offline-only tools.

## Supporting requirements and evidence

- [Foundation persistence contract](foundation-contracts.md#evidence-and-transaction-persistence)
- [Full recovery implementation](../../crates/collector-store/src/backup.rs)
- [Portable transfer implementation](../../crates/collector-store/src/archive_export.rs)
- [Recovery and unknown-data regressions](../../crates/collector-store/tests/archive.rs)

Implementation and tests are supporting evidence, not a fresh qualification run.
Detailed subsystem requirements and existing dated validation limits remain applicable.

## Reconsideration

Revisit when adding replication, online recovery or a new portable format; explicitly preserve authority separation and matching-point recovery.
