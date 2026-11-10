# ADR 0015: Isolated direct media delivery

Status: Accepted. Formalized 2026-11-03 from existing documented decisions and
implemented constraints; this is not a claim that implementation or qualification
first occurred on that date. Source reviewed: `eaff933`.

## Context

Exposing audio remotely need not expose private application state or management mechanisms. Read-only media access still carries disclosure risk.

## Decision

- Direct media delivery must run behind a separate bounded authorization boundary and must not be provisioned with private databases, long-lived account credentials, administration mechanisms or writable archive access. Scoped media grants and the restricted local service key are permitted only for their defined protocol authority.
- The isolated media process must use the narrow local authority protocol and read-only authorized archive capabilities. Its supported container composition must retain the network-disabled media namespace and restricted mounts/privileges.
- Media authorization must retain current account/session/collection permission checks and immutable range identity. Cached handles must not bypass revocation or archive identity checks.
- A public TLS frontend must retain its own network/security maintenance and restrictions; media-process isolation must not be claimed to isolate the frontend or host kernel.
- Documentation must state that compromised read access may disclose mounted data; read-only access must not be advertised as confidentiality protection.

## Consequences and scope

This favors a narrow media executable over putting full application authority on that boundary. It does not freeze the proxy product, hostname arrangement or container image packaging.

## Supporting requirements and evidence

- [Selected isolation contract](../direct-https-media-plan.md)
- [Deployment boundary and limitations](../../deploy/container/media.md)
- [Container isolation](../../deploy/container/compose.media.yaml)
- [Media capabilities and tests](../../crates/collector-media/src/lib.rs)

Implementation and tests are supporting evidence, not a fresh qualification run.
Detailed subsystem requirements and existing dated validation limits remain applicable.

## Reconsideration

Revisit for a different isolation mechanism, explicitly comparing disclosure, mutation, authorization and frontend exposure boundaries.
