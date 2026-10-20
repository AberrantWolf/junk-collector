# Authentication rate-limit retention — HL-05F, 2026-10-20

## Defect and repair

Review of parent HL-05's bounded abuse cases found an unbounded identity cache in
the process-local HTTP limiter. Authentication/enrollment checks inserted a peer
and hashed-account bucket before testing the global limit. Poll checks similarly
inserted a peer before rejecting a request. A flood of rejected aliases could keep
growing those maps. Old buckets were purged only when the same identity returned,
so even admitted identities accumulated across otherwise expired windows.

The adapter now evicts expired empty identity buckets on every check and creates
identity buckets only after admission. Every retained identity therefore belongs
to a currently admitted attempt: authentication is bounded by 100 global attempts
per window, enrollment by 50 and polling by 600. Existing 60-second windows,
peer/account thresholds, hashed account keys, HTTP 429/Retry-After behavior and
actual TCP peer selection remain unchanged. No credential/database format changes.

## Acceptance

Focused deterministic-clock tests pass: 1,000 distinct rejected aliases per limiter
cannot grow its identity state beyond admitted work; expiry reclaims inactive
buckets; the exact 60-second boundary works; and recent attempts remain protected
when older identities expire. Account, peer and global ceilings are exercised for
authentication/enrollment and peer/global ceilings for polling.

A new real-executable/TCP regression also passed on this Mac. It starts a disposable
installation, signs in, then alternates public/private HTTP listeners with rotating
`Forwarded`/`X-Forwarded-For` values. Ten admitted account attempts produce 429 with
typed `rate_limited`/Retry-After 60; changing usernames reaches the shared actual
peer ceiling of 30 and further header/alias rotation stays throttled. The existing
session still reads successfully. After process termination/restart, that session
survives and correct password sign-in works again. A foreign Origin is rejected.
This tests the actual executable and sockets, without replacing HTTP or storage.

Full workspace tests passed (**443 passed, 4 existing qualification tests ignored**),
along with strict all-target Clippy, Rust formatting, architecture/document/fixture
checks, web type checks/build and the rebuilt server. Generated web assets remained
byte-identical. Focused limiter tests were rerun after final review. The full actual
Chrome 155.0.8059.85 smoke passed: login/session expiry, real virtual-WebAuthn
registration/login, roles/inventory, artwork, concurrent/bulk preparation, playback/
revocation, collection lists and mobile workflows. All fixtures used temporary data.

## Reproduction and limits

```sh
cargo test -p collector-server --lib security_tests --locked
cargo test -p collector-server --test authentication_limits --locked
```

This is a source repair with bounded in-process memory guarantees and real loopback
TCP acceptance. It does not qualify a public tunnel, distributed limiting,
long-duration load, global limits from multiple real network peers, all enrollment
ceremonies under load, slow/oversized media or power loss. No new release/image or
production Collector/Portainer/tunnel was deployed. The earlier native container
candidate for the numeric-IP/metadata repair predates this additional source change.
