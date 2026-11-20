# 0005: Lossless module data and write eligibility

Status: proposed, not yet reviewed by the owner.

## Context

Which modules are installed can change independently of the archives they wrote.
Treating unknown content as executable, or dropping it during edits, loses data.

## Decision

- Module payloads and unknown data round-trip without loss at their envelope.
- Reading is separate from writing. A missing module or unsupported schema doesn't
  hide authorized evidence, artwork or media.
- Mutation and execution require the owning module, a supported schema, the
  current binding and authorization.
- Migrations keep the exact source envelope and unknown fields. Transformations
  need an explicit version-specific policy; ordinary edits never downgrade or
  overwrite unsupported data.
- Core and generic clients don't reinterpret module payloads as a universal schema.

## Consequences

Modules are compiled in. Runtime plugins and a stable native ABI are not offered.

See: [collection types](../../crates/collector-app/src/collection_types.rs),
[module registry](../../crates/collector-modules/src/lib.rs).
