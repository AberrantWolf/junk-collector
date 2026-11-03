# ADR 0001: Application policy and mechanism boundaries

Status: Accepted. Formalized 2026-11-03 from existing documented decisions and
implemented constraints; this is not a claim that implementation or qualification
first occurred on that date. Source reviewed: `eaff933`.

## Context

Collection policy is consumed by HTTP, background workers and independent clients. Mixing it with infrastructure creates competing authorization and workflow implementations.

## Decision

- Domain/core code must remain independent of HTTP, filesystem, database, GUI and device mechanisms. Portable reader interfaces are permitted; adapter paths and infrastructure ownership are not.
- Application services must own authorization, eligibility, metadata authority and workflow policy. Adapters must implement mechanisms behind explicit ports; frontends must not create parallel domain pipelines.
- Collection modules must own domain schemas, interpretation and result validation. Shared parsing, byte decoding, normalization and audio algorithms must live behind narrow reusable library interfaces.
- Shared extraction must preserve real independent consumers and validate affected sibling projects. It must not import an older application's orchestration.
- New subsystem boundaries must be documented before introducing dependencies that bypass the owning layers.

## Consequences and scope

This permits multiple adapters without moving collection policy into them. It rules out direct HTTP-to-Store policy shortcuts. Crate names and the current location of a particular shared crate are not permanent commitments.

## Supporting requirements and evidence

- [Architecture and layer ownership](../architecture.md#layers-and-ownership)
- [Dependency enforcement](../../scripts/check_architecture.py)
- [Module/host interfaces](../../crates/collector-modules/src/lib.rs)

Implementation and tests are supporting evidence, not a fresh qualification run.
Detailed subsystem requirements and existing dated validation limits remain applicable.

## Reconsideration

Revisit when a real consumer needs a different boundary; document policy ownership and data/recovery consequences before changing dependencies.
