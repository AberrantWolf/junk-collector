# 0011: Input-bound command receipts

Status: proposed, not yet reviewed by the owner.

## Context

A command can commit and then lose its response. A retry that re-resolves
configuration, or repeats an external effect, can mutate twice or differently.

## Decision

- A command's retry key is bound to its typed inputs before any mutable
  configuration is resolved.
- Repeating the same request returns the committed result; reusing the key with
  different inputs is rejected. Expected revisions separately reject stale writes.
- A receipt and its effect commit in the same transaction (or via an explicit
  recoverable marker). A success receipt never precedes its effect.
- Interrupted outcomes are reconciled before a new attempt. Malformed receipts are
  not discarded to start over.
- External effects that can't be replayed, such as AcoustID submissions, claim a
  receipt before sending; an interrupted send is recorded as unknown and is not
  re-sent.
- Shared effects have one implementation rather than nested independently
  committed commands. Authentication ceremonies, upload offsets and one-time
  issuance keep their own retry rules.

## Consequences

Commands replay safely; this is not exactly-once delivery to external services.

See: [API mutation contract](../../api/README.md),
[receipts](../../crates/collector-store/src/commands.rs).
