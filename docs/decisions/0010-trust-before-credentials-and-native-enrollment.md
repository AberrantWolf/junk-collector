# ADR 0010: Trust before credentials and native enrollment

Status: Accepted. Formalized 2026-11-03 from existing documented decisions and
implemented constraints; this is not a claim that implementation or qualification
first occurred on that date. Source reviewed: `eaff933`.

## Context

Discovery and network reachability do not establish server identity. Browser trust and native enrollment also use different trust stores and authorization paths.

## Decision

- Clients must establish trusted endpoint identity before sending credentials. Discovery hints and private addresses must not become trust or account authority.
- Native credentials must be independently enrolled, account-bound and revocable. Issuance must require the supported browser approval and secret-bound completion protocol; a caller-supplied native marker must not substitute for authentication.
- Browser certificate trust must remain separate from native pairing. Pairing material must not be described as installing browser trust or granting a collection role.
- Alternate routes must preserve enrolled identity and immutable content identity. Unexpected pinned keys, untrusted endpoints and invalid certificates must not be accepted as recovery shortcuts.
- Credential change and revocation must follow explicit transactional security policy; restored/re-enabled accounts must not resurrect revoked app access.

## Consequences and scope

This permits trusted route changes without confusing transport, identity and permission. It does not require universal route failover or a general authenticated rotation framework.

## Supporting requirements and evidence

- [Security/enrollment contract](security.md#browser-passkey-and-native-credential-rules)
- [Private transport trust](../automatic-private-playback.md#trust-and-transfer-policy)
- [Credential policy](../../crates/collector-app/src/security.rs)
- [Revocation regressions](../../crates/collector-store/tests/wave2_regressions.rs)

Implementation and tests are supporting evidence, not a fresh qualification run.
Detailed subsystem requirements and existing dated validation limits remain applicable.

## Reconsideration

Revisit for a real enrollment, trust rotation or client-platform change; preserve identity checks before credential use and explicit revocation semantics.
