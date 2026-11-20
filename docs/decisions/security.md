# Security, recovery and enrollment

Status: proposed, not yet reviewed by the owner.

Authority comes from an authenticated principal plus an explicit grant, never from
an address, a forwarded header or use of the private listener. Server
administration and collection access are separate: an administrator can't read or
curate a collection without a grant. See also
[0009](0009-exposure-composition-and-independent-authorization.md) and
[0010](0010-trust-before-credentials-and-native-enrollment.md).

## Trust boundaries

| Caller or state | Boundary | Authority | Defenses |
|---|---|---|---|
| Anonymous | No credential | Only explicitly public pages and client APIs | Router composition, body limits, generic errors |
| Browser session | Secure, HttpOnly, SameSite=Strict cookie | The account's grants and admin flag | Exact-Origin CSRF check, expiry, revocation, recent-auth time |
| Authentication origin | Exact scheme/host/port, browser-trusted TLS where needed | May start browser ceremonies; no role | Origin and challenge binding |
| Private management listener | Separate loopback/private router | Makes routes reachable; grants nothing | Authentication and role checks on every route |
| Native app credential | Enrolled per device, kept in platform secure storage | The account's grants | Expiry, revocation, device binding |
| Local CLI recovery | Server stopped, exclusive access to private state | Credential reset for one named account | File permissions, lock, audit record |
| Provider or external tool | Adapter behind an application port | Only the data passed to it | Time/byte/process limits, secret redaction |
| Portable archive | Untrusted but preserved | None | Structural/hash validation, explicit owner assignment |
| Private SQLite state | Host-local | Accounts, grants, credentials, sessions, settings | Transactions, restrictive permissions, backup |

## Recent authentication

These actions require an authentication no more than five minutes old (ordinary
activity doesn't refresh it): adding or removing passkeys, replacing recovery
codes, changing a password or admin status, enrolling or revoking a native
credential, disabling an account, rotating the server TLS identity. Grant changes
need owner authority plus recent authentication. Removing or demoting the last
enabled administrator or the last owner of a collection is refused.

## Recovery

- Recovery codes: each replacement creates ten random 160-bit codes, shown once;
  only SHA-256 hashes are stored. Replacement invalidates the previous batch
  atomically, and consumption is compare-and-swap so a replayed code succeeds at
  most once.
- Online recovery with a code replaces the password, invalidates the batch,
  revokes browser sessions and native credentials, and issues one new session.
  Passkeys and grants are kept.
- Local recovery (`recover-account --username NAME`, server stopped) resets the
  account's credentials and revokes its sessions, native credentials and passkeys.
  It never changes collection grants.
- Disabling an account revokes its native credentials and cancels pending
  enrollments, so re-enabling it doesn't restore them.

## Browser, passkey and native rules

- Passkey sign-in updates the signature counter with compare-and-swap against the
  stored record captured when the ceremony began; a stale concurrent ceremony
  fails and issues no session.
- Login, passkey ceremonies and every cookie-authenticated mutation require an
  exact `Origin` match. Missing, `null`, malformed, sibling-host and other-port
  origins are rejected. Missing Origin is accepted only for authenticated native
  requests and the native enrollment start/complete exchange.
- Native enrollment grants nothing until the same user approves it in a recently
  authenticated browser session, and completion requires the server-generated
  secret (stored hashed). Native secrets are returned once and never stored in the
  archive.
- Remote browser sign-in requires browser-trusted HTTPS. Chromium's localhost
  handling makes local Secure cookies work; that is not a promise for other
  browsers.

## Rate and resource limits

Password login, passkey challenges and enrollment each have token buckets per
account, per direct transport peer and globally. Responses don't reveal whether an
account exists. The buckets are in-process and reset on restart.

Body and range limits, job concurrency limits, provider response limits, archive
path checks and bounded image/audio parsing apply everywhere. Logs carry error
categories and opaque IDs, never passwords, recovery codes, bearer tokens,
cookies, WebAuthn challenges, provider secrets or payload paths.
