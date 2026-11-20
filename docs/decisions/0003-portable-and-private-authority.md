# 0003: Portable collection data and private operational state

Status: proposed, not yet reviewed by the owner.

## Context

Collection history should outlive any one installation, but accounts and access
grants must never be reconstructed from portable archive contents.

## Decision

- The archive holds originals, inventory, manual authority, accepted sources,
  evidence and unknown module data.
- Accounts, credentials, grants, server identity and durable jobs live in private
  operational state. The SQLite catalog is a rebuildable projection of the archive,
  not a source of truth.
- SQLite stays on local storage even when archive payloads are on a NAS.
- A full backup captures archive, private state and configuration together. A
  restored installation must not run concurrently with another copy of itself.
- Portable export/import excludes credentials and grants. Importing into a fresh
  installation requires explicitly assigning an owner; being an administrator does
  not grant collection ownership.

## Consequences

Full backup and portable transfer have deliberately different security semantics.

See: [backup](../../crates/collector-store/src/backup.rs),
[portable export](../../crates/collector-store/src/archive_export.rs).
