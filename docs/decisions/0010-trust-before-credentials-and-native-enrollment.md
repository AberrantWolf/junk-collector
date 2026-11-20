# 0010: Trust before credentials; native enrollment

Status: proposed, not yet reviewed by the owner.

## Context

Discovery and reachability don't prove server identity, and browsers and native
apps use different trust stores.

## Decision

- Clients verify the endpoint's identity before sending credentials. Discovery
  hints and private addresses grant no trust or authority.
- Native app credentials are enrolled separately, bound to an account and
  revocable. Issuing one requires browser approval by the same user plus a
  server-generated secret; a client-supplied header can't select native auth.
- Browser certificate trust is separate from native pairing. Pairing doesn't
  install browser trust or grant a collection role.
- Alternate routes keep the enrolled identity. Unexpected pinned keys and invalid
  certificates are rejected, never used as a recovery shortcut.
- Re-enabling or restoring an account doesn't bring back revoked app credentials.

See: [security](security.md), [credential policy](../../crates/collector-app/src/security.rs),
[private playback](../automatic-private-playback.md).
