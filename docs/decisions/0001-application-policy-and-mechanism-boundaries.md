# 0001: Application policy and mechanism boundaries

Status: proposed, not yet reviewed by the owner.

## Context

HTTP handlers, background workers and separate clients all consume collection
policy. If policy leaks into infrastructure, each ends up with its own
authorization and workflow rules.

## Decision

- Domain and core crates do not depend on HTTP, the filesystem, the database, a
  GUI or devices. Portable reader traits are fine; adapter paths are not.
- Application services own authorization, eligibility, metadata authority and
  workflow policy. Adapters implement mechanisms behind explicit ports. Frontends
  do not build their own domain pipelines.
- Collection modules own their domain schemas, interpretation and result
  validation. Parsing, byte decoding, normalization and audio algorithms live in
  narrow reusable `junk-libs` crates.
- Code shared with other projects is extracted as a library, not by importing
  another application's orchestration. Check the other consumers when changing it.
- Production code has no placeholder successes, fake jobs or no-op trait
  defaults. An operation that isn't implemented is not routed; a capability may
  report a typed "unavailable" reason when that is real behavior. Fakes belong in
  tests.
- Decide where a new subsystem belongs before adding dependencies that bypass
  the existing layers.

## Consequences

Several adapters can share one policy implementation. HTTP code calls
application services, never the Store directly (enforced by
`scripts/check_architecture.py`).

See: [architecture](../architecture.md),
[module interfaces](../../crates/collector-modules/src/lib.rs).
