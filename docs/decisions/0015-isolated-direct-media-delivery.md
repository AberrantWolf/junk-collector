# 0015: Isolated direct media delivery

Status: proposed, not yet reviewed by the owner.

## Context

Serving audio directly to remote clients shouldn't expose private state or
management functions, though read-only media access still carries disclosure risk.

## Decision

- The optional direct media service is a separate process behind its own
  authorization boundary. It has no private databases, long-lived account
  credentials, administration or writable archive access — only scoped media
  grants and a restricted local service key.
- It talks to Collector through a narrow local protocol and reads authorized
  archive files read-only. The supplied container setup keeps it in a
  network-disabled namespace with restricted mounts and privileges.
- Media grants still check the current account, session and collection permission
  and the immutable range identity; cached handles don't bypass revocation.
- The public TLS frontend needs its own hardening; media isolation doesn't
  isolate the frontend or the host.
- A compromised media process can still disclose mounted data. Read-only access is
  not confidentiality protection.

See: [media deployment](../../deploy/container/media.md),
[compose file](../../deploy/container/compose.media.yaml),
[media crate](../../crates/collector-media/src/lib.rs).
