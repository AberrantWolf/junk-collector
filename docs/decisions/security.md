# Security, recovery, and enrollment contract (S01)

Status: accepted design gate, 2026-09-09. Implementation is assigned to S02, S03,
S04, and R03. This decision does not make the current authentication mechanisms
conformant.

## Trust boundaries and ingress

Authority is derived from an authenticated principal and an explicit grant, never
from an address, a forwarded header, or use of the private listener. Server
administration and collection access are independent authorities. In particular,
an administrator cannot read or curate a collection without a collection grant,
and private ingress never bypasses authentication.

| Caller or state | Authentication and boundary | Permitted authority | Required defenses and owner |
|---|---|---|---|
| Anonymous caller | No credential; either composed listener | Only explicitly public pages and client APIs | Route composition, body limits, generic errors; HTTP/A04 |
| Browser session | Secure, HttpOnly, SameSite cookie tied to an account | Account grants and server-administrator flag | Exact-Origin CSRF, expiry/revocation, recent-auth timestamp; S02 |
| Configured browser authentication origin | Exact normalized scheme/host/port, with browser-trusted TLS where required | May begin browser ceremonies; grants no role by itself | Exact-Origin checks and challenge binding; S02 |
| Private management ingress | Separately composed loopback/private router | Makes management routes reachable; grants no authority | Authentication plus role checks on every route; A04/U02 |
| Paired native credential | Independently enrolled, device-labelled credential in platform secure storage | Account grants within its explicit scope | Expiry, revocation, device binding, replay defense; S03 |
| Local CLI recovery | Exclusive access while the server is stopped, explicit private database and UserId | Credential recovery for exactly that account | File ownership, lock/exclusivity, audit record; S02 |
| Provider or local tool | Host-owned adapter invoked through an application port | Only the bounded operation and data passed by the host | Time/byte/process bounds and secret redaction; P02/D04-D07 |
| Portable archive | Untrusted-but-preserved collection history | No account or grant authority | Structural/hash validation and explicit reassignment; P01/P05 |
| Private SQLite state | Host-local mutable security state | Source of accounts, grants, credentials, sessions and settings | Transactions, restrictive permissions, migrations and backup; S02/P05 |

Forwarded addresses are ignored. A future deployment may define a specific trusted
proxy contract; until then `Forwarded` and `X-Forwarded-For` are attacker input.
Loopback, LAN placement, or a proxy hop never turns a request into private ingress.

## Authentication freshness and destructive account actions

A browser or native authentication records its method and authentication instant.
The following operations require an authentication event no more than five minutes
old; ordinary session activity does not refresh that instant:

- adding or removing passkeys;
- generating or replacing recovery codes;
- changing a password or administrator status;
- enrolling or revoking a native device credential;
- disabling an account; and
- rotating server TLS identity.

Collection curation and ordinary owner-authorized grant changes use their normal
role checks and do not require recent authentication. Refusing last-owner removal
is nevertheless mandatory. Reauthentication produces a narrow, time-bounded proof
for the requested account; it does not mint a more privileged role.

## Recovery codes and local recovery

Each replacement creates exactly ten independently generated 160-bit CSPRNG codes.
Codes are displayed once. The database stores only SHA-256 hashes plus batch ID,
creation time, optional consumption time, and audit metadata. Replacement and
consumption are transactions: replacement invalidates the prior batch atomically;
compare-and-swap consumption makes concurrent replay yield at most one success.
Successful recovery revokes existing browser sessions before issuing a new one.

Local recovery is a server-stopped, exclusive CLI operation against explicitly
selected private state and an explicit `UserId`. It resets credentials, revokes all
sessions, paired native credentials, and passkeys for that account, and records the
event. It never reconstructs or changes collection grants. Online APIs refuse to
remove or demote the last enabled administrator and refuse to remove or demote the
last owner of a collection. Explicit local recovery is the escape hatch; implicit
promotion is not.

## Browser, passkey, and native credential rules

Passkey sign-in updates the stored counter with a transactional compare-and-swap
against the value used to verify the assertion. A stale concurrent update fails
closed and cannot issue a session. Authenticators that legitimately report a zero
counter follow the WebAuthn library's counter semantics, but bypassing the CAS is
not permitted.

Browser login, passkey registration/authentication ceremonies, and every
cookie-authenticated mutation require an exact `Origin` match. A missing `Origin`
is rejected for cookie and browser-ceremony requests. It is accepted only with a
verified paired-native credential; a caller-supplied header cannot select native
semantics. Browser sessions and native credentials have separate tables/types,
labels, expiration and revocation. Native secrets are returned once at enrollment
and stored by a platform secure-storage adapter, never in the portable archive.

Chromium's localhost treatment permits the tested local Secure-cookie workflow.
That is a qualified onboarding path, not a promise for Safari or every embedded
browser. Remote browser authentication requires browser-trusted HTTPS. LAN and
loopback proxy traffic remain ordinary untrusted transport regardless of source IP.

## Rate and resource controls

Password login, passkey challenge creation, and enrollment each receive independent
token buckets at three scopes: account (where known), transport peer, and process
global. Initial limits are configuration with conservative defaults, bounded burst,
and monotonic refill; responses do not disclose whether an account exists. A peer
is the directly connected transport peer because forwarded addresses are ignored.
Persistent/distributed limiting is a deployment follow-up; process restart must be
called out rather than presented as a reset-resistant defense.

HTTP body/range limits, concurrent job limits, provider response limits, archive
path safety and bounded image/audio parsing remain mandatory alongside auth rate
limits. Logs contain stable error categories and opaque IDs, not passwords,
recovery codes, bearer credentials, cookies, WebAuthn challenges, provider secrets,
filesystem payload paths, or raw credential-bearing request bodies.

## Current-code audit and assigned defects

The following are observed defects, not hypothetical risks:

- There is no recent-authentication proof, recovery-code lifecycle, last-admin or
  last-owner guard, or native enrollment implementation. S02 owns the first four;
  S03 owns enrollment.
- Passkey counters are updated by a read/modify/write sequence and can lose a
  concurrent update. S02 owns the transactional CAS.
- Password limiting is global and process-local, and passkey challenge creation has
  no equivalent account/peer buckets. S02 owns browser limits; S03 adds enrollment
  limits; R03 qualifies load behavior.
- `x-collector-client: native` currently asks login to issue a bearer token. It is
  caller-selected and is not a paired-device credential. S03 replaces this path;
  it must not be treated as proof in the interim.
- Missing-Origin handling is coupled to the presence of a cookie rather than an
  authenticated transport type. S02 replaces it with the rules above.
- Local password reset revokes sessions but does not implement the complete local
  recovery revocation/audit contract. S02 owns it.

Existing positive properties remain requirements: public/private routes are
composed separately, forwarded headers do not infer trust, cookies are Secure,
HttpOnly and SameSite=Strict, and upload/image/range bounds exist. Their presence
does not close the defects above.

## Attack-test assignments

| Owner | Required attacks and failure assertions |
|---|---|
| S02 | stale-auth bypass; recovery-code replay and concurrent consumption; replacement races; passkey counter races; last-admin/last-owner removal; cookie mutation and passkey/login with missing, null, sibling, alternate-port and malformed Origin; challenge exhaustion; proxy-address spoofing; credential/log scanning |
| S03 | enrollment replay, cross-device completion, wrong-user approval, expiry/cancellation, simultaneous completion, caller-supplied native header, revoked/expired native credential, secret leakage in URLs/logs/archive |
| S04 | hostile or replayed certificate rotation, wrong old identity, interrupted rotation, offline device, expired transition, credential use before authenticated trust update |
| R03 | per-account/peer/global exhaustion and restart behavior; oversized bodies and ranges; slow upload/provider responses; job flood/cancellation; parser CPU/memory abuse; direct-peer versus spoofed-forwarded-address behavior |

All tests use temporary private state and synthetic credentials. Hardware password
managers, remote browser-trusted HTTPS, and deployment/load gates are recorded as
external qualifications when unavailable; a virtual authenticator alone is not a
release security certification.
