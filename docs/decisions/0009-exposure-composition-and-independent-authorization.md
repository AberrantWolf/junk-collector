# 0009: Exposure by router composition, independent authorization

Status: proposed, not yet reviewed by the owner.

## Context

A reverse proxy or tunnel may connect from loopback while serving Internet
traffic, so network location can't decide what is exposed or who is allowed.

## Decision

- Public and private surfaces are chosen by listener/router composition, never by
  source IP, DNS, forwarded headers or apparent proximity.
- Pages, bundles and their APIs follow the selected exposure. Public
  administration implies public curation; disabled surfaces are absent from the
  public router.
- Authentication and collection roles apply regardless of exposure. Private
  ingress doesn't bypass them, and server administration doesn't imply collection
  access.
- Public tunnels and proxies target the public listener even when their upstream
  hop is localhost.
- `Forwarded` and `X-Forwarded-For` are untrusted input.

See: [security](security.md), [router](../../crates/collector-server/src/lib.rs),
[configuration](../configuration.md).
