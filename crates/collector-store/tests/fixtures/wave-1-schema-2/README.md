# Synthetic Wave 1 upgrade fixture

This contains no personal data. `archive/` was materialized from the committed
schema-1 fixture through the Wave 1 schema-2 reader/writer contract, then its
standalone Capture was advanced once. It therefore contains:

- two immutable legacy item revisions plus additive schema-2 sidecars;
- the same Evidence ID repeated byte-equivalently across historical revisions;
- original payload and artwork bytes, manual metadata, loans, identifiers, and
  lexically preserved unknown module/provider JSON;
- Work and Edition sidecars; and
- an embedded Capture compatibility copy older than standalone Capture revision
  3, so migration/reindex must select the standalone record.

Regenerate into a new directory with:

```sh
cargo run -p collector-store --example materialize_wave1_fixture -- \
  crates/collector-store/tests/fixtures/schema-1 NEW_DIRECTORY
```

`private-v2.sql` is a separate deterministic private-state fixture. It preserves
synthetic accounts, password-hash text, grants, sessions, passkeys, settings,
activity, and jobs without placing any private state in the archive.
