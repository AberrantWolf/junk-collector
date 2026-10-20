# Numeric-IP authentication startup repair — 2026-10-20

## Behavior and ownership

The server now starts with a numeric-IP HTTPS `authentication_origin`, retaining
password login and recovery. Passkeys are unavailable there with the typed reason
`dns_name_required`; a configured HTTPS DNS origin enables the real WebAuthn
verifier. An absent origin reports `not_configured`. This follows WebAuthn's
[DNS-based relying-party identity](https://www.w3.org/TR/2026/REC-webauthn-3-20260825/#relying-party-identifier),
without relaxing TLS or inventing an IP-based passkey identity.

The server WebAuthn adapter owns one enabled/disabled configuration enum.
Setup/doctor/serve validate it before opening archive/private state. Invalid URLs,
HTTP, embedded credentials, paths, queries and fragments are rejected before account
or TLS creation. `/features` exposes availability plus a typed reason; Account &
security explains the hostname requirement while keeping password/recovery usable.
Generated API contracts, embedded assets and container instructions are updated.

TLS validation, Origin allowlisting, listener exposure and stored credentials are
unchanged. This is a source repair; the already published alpha.2 digest predates it.
No release tag, public image, production account/configuration or phone trust was
changed by this delivery.

## Actual executable and browser acceptance

`cargo test -p collector-server --test authentication_origin --locked` runs the
actual setup, doctor and server executable against temporary data. It switches the
same installation through IPv4 → DNS → IPv6 → IPv4 authentication origins, verifies
features and password login over certificate-validated IPv4 HTTPS, preserves prior
sessions/ServerId/TLS certificate, and checks that real registration options work
only with DNS. IPv6 here qualifies configuration/startup, not an actual IPv6 network
listener or browser. Malformed-origin cases exercise setup/doctor/serve with bounded
process teardown, unchanged config and no archive/private-state directories.

`npm --prefix web run test:numeric-ip` passed in actual Chrome 155.0.8059.85:
setup/doctor/startup, independently verified TLS/IP identity, password sign-in,
typed passkey guidance without an Add button, 390px layout and real recovery-code
password/session issuance. The mobile screenshot was inspected. The fixture's
self-signed certificate was independently validated before the isolated browser
test context accepted it; no system/phone trust was installed. Temporary data,
credentials and TLS keys were cleaned after the browser test.

The numeric-origin executable tests passed (2 tests). Rust formatting, full
workspace tests (438 passed, 4 existing ignored), strict all-target Clippy,
architecture/docs/fixtures, generated API consistency, web type checks/build and
the full real-browser smoke passed. The smoke includes actual virtual-WebAuthn
registration/login at a DNS origin, playback/revocation, imports and lists. The Rust
server was rebuilt after embedding the changed security assets. Both new test
harnesses bound startup/teardown; the browser fixture was cleaned.

## Subsequent native-container/NAS acceptance

The unpublished native candidate from
`f2efdfadc1183c9d76eae1cb2639a78ddf223153` passed both architecture builds/smokes and
OCI verification, then the actual Synology NAS startup/login/browser/full restore
journey at a numeric-IP HTTPS origin. Typed passkey unavailability, real Chrome
playback and exact originals passed; live services were unchanged and the fixture
was cleaned. See [candidate identity and full evidence](storage-permission-recovery-2026-10-20.md#actual-nas-candidate-acceptance).
The published alpha.2 image remains unchanged and still predates this fix.
