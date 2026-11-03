# ADR 0009: Exposure composition and independent authorization

Status: Accepted. Formalized 2026-11-03 from existing documented decisions and
implemented constraints; this is not a claim that implementation or qualification
first occurred on that date. Source reviewed: `eaff933`.

## Context

A proxy may connect from loopback while serving Internet callers. Exposure and authorization therefore need independent, explicit authorities.

## Decision

- Public/private surfaces must be selected by listener/router composition, not source IP, DNS, forwarded headers or apparent network proximity.
- Pages, bundles and backing APIs must honor the selected exposure. Public administration must include public curation; disabled management surfaces must remain absent from the public composition.
- Authentication and collection roles must be enforced independently of exposure. Private ingress must not bypass them, and server administration must not imply collection read/write authority.
- Public ingress must target the public composition even when its local proxy hop uses loopback. A private management listener must not be published as a substitute for explicit public exposure.
- Forwarded metadata must not confer trust without a separately accepted trusted-proxy contract.

## Consequences and scope

This rules out address-based privilege guesses and keeps deployment topology from granting roles. Exact ports, proxy products and hostnames remain configuration.

## Supporting requirements and evidence

- [Security trust boundaries](security.md#trust-boundaries-and-ingress)
- [Router composition](../../crates/collector-server/src/lib.rs)
- [Exposure configuration](../configuration.md#public-exposure)
- [Ingress review evidence](../validation/ingress-review-2026-10-21.md)

Implementation and tests are supporting evidence, not a fresh qualification run.
Detailed subsystem requirements and existing dated validation limits remain applicable.

## Reconsideration

Revisit only for an explicit new ingress/proxy authority model with spoofing, role and page/API denial acceptance.
