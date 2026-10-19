# Security, recovery, and enrollment contract (S01)

**Current scheduling (updated 2026-10-19):** [remaining work](../remaining-work.md)
separates necessary public-server controls/qualification (HL-05) from advanced
rotation/routing and exhaustive assurance (HL-12/14). Preserve implemented defenses;
review old proposals for actual need under [the revised scope](home-lab-scope.md).
Production Keychain-backed Phono/Trash Amp adapters now exist; the older statement
below about missing native secure storage is historical. Phono is retired as a target.

Status: accepted design gate, 2026-09-09. S02 and S03 were implemented on
2026-09-10; S04 certificate rotation and R03 external qualification remain open.

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

Collection curation uses its normal role checks. Grant changes require both owner
authority and recent authentication because they alter durable access. Refusing
last-owner removal is mandatory. Reauthentication produces a narrow, time-bounded proof
for the requested account; it does not mint a more privileged role.

## Recovery codes and local recovery

Each replacement creates exactly ten independently generated 160-bit CSPRNG codes.
Codes are displayed once. The database stores only SHA-256 hashes plus batch ID,
creation time, optional consumption time, and audit metadata. Replacement and
consumption are transactions: replacement invalidates the prior batch atomically;
compare-and-swap consumption makes concurrent replay yield at most one success.
Successful online recovery atomically replaces the password, invalidates the
active batch, revokes existing browser sessions, and issues one fresh browser
session. Native credentials and collection grants are unchanged.

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
cookie-authenticated mutation require an exact `Origin` match. Missing, `null`,
malformed, sibling-host, and alternate-port origins are rejected. Missing Origin
is accepted for an authenticated paired-native request and for the narrow native
enrollment start/complete bootstrap protocol. Bootstrap requests grant no
authority before same-user recent browser approval and completion requires the
server-generated high-entropy secret whose hash is stored privately; a
caller-supplied header cannot select native semantics. Browser sessions and native
credentials have separate tables/types, labels, expiration and revocation. Native
secrets are returned once at enrollment and stored by a platform secure-storage
adapter, never in the portable archive.

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

## Implemented controls and remaining qualification

S02/S03 now implement recent-authentication proof, recovery-code lifecycle,
transactional last-administrator/owner refusal, passkey counter/session CAS,
account/peer/global browser and enrollment rate buckets, exact transport-aware
Origin handling, explicit local recovery, and browser-approved native enrollment.
The former `x-collector-client: native` issuance path is removed; only successful
lookup in the native-credential table selects native authentication. Public/private
routes remain separately composed, forwarded headers do not infer trust, cookies
are Secure/HttpOnly/SameSite=Strict, and upload/image/range bounds remain enforced.

The rate buckets are intentionally process-local and reset on restart. A production
native secure-storage implementation, real password-manager/hardware passkey runs,
browser-trusted remote HTTPS, independent security/protocol review, authenticated
TLS identity rotation, and deployment/load qualification remain external or later
ticket gates. These are not implied by virtual-authenticator or loopback tests.

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

## Wave 2 review boundary corrections

Application security policies operate on stable account/grant snapshots gathered within the adapter's transaction. Disable, recovery-code replacement, password recovery, and local recovery each return an explicit revocation set; adapters apply that set atomically. Disabling does not merely rely on an enabled-account join: it marks native credentials revoked and cancels pending enrollments, so re-enabling cannot restore them. Replacement recovery codes only invalidate previous recovery batches.

Passkey ceremony state includes the original stored credential revision and record. The final CAS uses that snapshot, never a post-verification reload; a browser test completes two overlapping ceremonies in reverse order and rejects the stale one. Persistent grant revision counters prevent removal/recreation from accepting an old optimistic revision.
