# 0016: Pre-release evolution with data preservation

Status: proposed, not yet reviewed by the owner.

## Context

Before a stable release, interfaces and formats need to change freely, but real
archives already exist and must not be lost.

## Decision

- Rust interfaces, JSON APIs, private databases, catalog projections and archive
  schemas may change together. Update all in-repo callers, generated contracts,
  fixtures and tests in the same change; no deprecation periods, compatibility
  re-exports or reserved migration numbers.
- Don't remove working compatibility code just as cleanup; simplify it when
  touching that boundary and nothing depends on it.
- Sibling projects and external formats are independent consumers with their own
  requirements.
- Existing archives, originals, observations, overrides, loans, credentials, trust
  and configuration are preserved. A breaking stored-format change needs a
  backup-and-convert path or a clear unsupported-format error before any write.
  Never silently reset user state. Prefer one restartable conversion over a chain
  of historical readers; rebuild catalog projections instead of migrating them.
- Encodings that identify stored objects (hashes, recipes) are versioned, and
  existing references are converted or kept explicitly.
- While there is a single installation, the only upgrade that must work is from
  the version deployed there to the current schemas. That installation runs
  beta.33 or beta.34 (catalog schema 15, private schema 23). Older readers and
  conversions may be removed once that installation has moved past them. Keep a
  matched backup before upgrading.
- Crash recovery, expected revisions, idempotence and access checks apply now,
  regardless of compatibility policy.

## Consequences

A stable release will need an explicit declaration of upgrade, backup/restore and
unsupported-format commitments.

See: [processing identity](../../crates/collector-core/src/processing.rs).
