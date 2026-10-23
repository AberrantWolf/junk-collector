# Direct HTTPS media with Cloudflare control access

Selected design and source contract, **2026-10-23**. **Implemented and deployed
on the NAS; basic external iPhone Safari playback/seek/track navigation is
operator-accepted, including an audible track change while locked. Extended
sessions remain unqualified.** See the
[beta.5 publication and NAS update](validation/container-beta5-publication-2026-10-23.md),
[dated acceptance and limits](validation/direct-https-media-2026-10-23.md) and
[operator setup](../deploy/container/media.md). The earlier design below is retained
with the implementation refinements incorporated.
The user selected Cloudflare for login/authentication/control and direct public
HTTPS for music delivery. This document defines one complete web/Trash Amp
delivery; it does not authorize DNS, firewall, certificate, NAS or release changes.
[HL-12](remaining-work.md#hl-12--https-relay-lan-switching-and-deferred-certificate-rotation)
owns the work, with actual deployment acceptance under
[HL-05](remaining-work.md#hl-05--linux-nas-and-optional-public-internet-deployment).
Those entries remain the only queue. This selection supersedes relay/Iroh-first
ordering for this remote-listening outcome, not the existing private playback.
**Isolation refinement, 2026-10-23:** the user's follow-up selects a separate
media executable/container with read-only content and a narrow core authorization
connection. This replaces the initial in-process recommendation. One-hostname
deployment remains supported through proxy composition; the selected home setup
retains separate control/media hostnames to keep music outside Cloudflare.
**Implementation authorized, 2026-10-23:** prioritize preventing archive mutation
and lateral access to the rest of the NAS/LAN. The owner accepts disclosure risk
for the read-only archive mount described below; an audio-only publication tree
is not a prerequisite. Implement the complete delivery and self-review before
commit checkpoints. Live DNS/firewall/NAS changes remain a separate deployment.

## Outcome and recommendation

Open the existing Cloudflare-hosted Collector page, sign in and press Play.
Prefer an authorized private media route when usable; otherwise use the configured
direct HTTPS media origin. Preserve queues, account/server identity, playback
position and native cache identity. Never fall back to Cloudflare for music bytes.
The same ordinary HTTPS contract supports a future mobile Trash Amp; building that
mobile app is outside this delivery. Qualify the user's actual phone browser now.

Use an established TLS reverse proxy and normal HTTP media/range requests.
Do not introduce a new peer protocol, relay, VPN integration, decoder, HLS pipeline,
transcoder or public file server. A stable media endpoint is an authenticated
service, not a directory of anonymously readable files.

The user's security intuition is substantially right: a media-only router has
fewer public operations and less untrusted input to review. It must not mount
Collector's general router and hide selected pages with proxy rules. However:

- The tunneled control application still accepts untrusted requests. Cloudflare
  is an additional protective layer, not Collector authentication or a proof that
  its grant issuer, login and authorization code are secure.
- A separate, correctly restricted container supplies a **meaningful process and
  filesystem boundary**. Do not give it Collector's writable archive/private state,
  host namespaces, management APIs or broad service credentials. The current core
  Compose setup mounts archive/state writable, so its service template cannot
  simply be copied for the media service.
- Publishing a direct home origin exposes the home IP and permits attacks on the
  home link. Keeping login on Cloudflare does not preserve origin-IP secrecy or
  upstream DDoS protection for that direct endpoint, and a flooded shared link can
  also interrupt the control tunnel.

Use a separate `collector-media` executable/container behind the TLS frontend.
Core remains the authority for grants, current account/collection access and
immutable file resolution; media owns only bounded HTTP/file delivery. Read files
directly from a read-only mount rather than forwarding all music through core.
The internal connection authorizes a presented capability and resolves that exact
track; it never delegates account, library-management or filesystem-write powers.
This is one small same-host service split, not another identity provider, library
catalog, storage format or orchestration framework. Packaging in the same Compose
stack is operational convenience, not itself an isolation boundary.

## Process, storage and internal authorization boundary

### Storage and compromise scope

The media container runs as a separate non-root process with read-only rootfs,
`no-new-privileges`, dropped capabilities, default seccomp and bounded memory,
CPU, process count, open handles and temporary storage. Do not use privileged mode,
host networking/PID namespace, Docker socket access, broad host mounts, or core's
state/config/secrets volume. TLS keys belong only to the frontend. In-memory media
grants/read handles and a bounded tmpfs need no persistent writable application
state. Dedicated Docker named volumes hold the HTTP and authority sockets; only
the HTTP socket directory is writable by media. It uses the existing archive UID
10001 for readability, without sharing core's mount, PID or network namespaces.

Mount the selected archive root read-only. Core returns only relative locators
under that root, not host paths. The production `--require-read-only-archive`
guard inspects Linux mount flags and refuses writable parent or nested mounts. Verify effective
read-only behavior on the target NAS, including nested mounts; do not rely on
the Compose spelling alone. Core can continue publishing to its own writable
archive mount while media observes committed immutable files through its separate
read-only view. Media cannot repair, encode, import, edit manifests or update an
index. Do not share private SQLite files, even read-only: they contain credentials
and make the process boundary depend on the whole database schema.

**Current layout limitation:** prepared derivatives and original audio live under
collection/capture payload paths, alongside masters and collection records. There
is no existing independent audio-only root. The initial supported mount may be
the selected archive/collection roots read-only, without private operational state.
This materially protects file integrity from a media-process compromise, but a
compromised process can read/exfiltrate **everything mounted**, including other
users' collections, retained metadata and raw masters where present. Grant checks
protect normal requests; they cannot restrict malicious code's direct filesystem
reads. Root host/daemon/kernel compromise is also outside this container barrier.

Do not label that initial mount audio-only or confidentiality-isolated per account.
An audio-only publication view can later reduce exposure, but would require an
explicit lifecycle/recovery design covering already committed tracks, purchased
originals, immutable validation, updates and cleanup. It is not a hidden archive
migration or full-library duplication prerequisite. If the owner requires that
stronger confidentiality boundary, settle that storage design before publishing.
Do not propose per-request privileged bind mounts or a custom file-descriptor
passing protocol merely to avoid stating the actual mount scope.

### Minimal core-to-media communication

Use bounded **HTTP over a private Unix-domain socket**, following the conventional
authorization-subrequest/token-introspection pattern. This same-host transport
works with the core's current host networking without opening another core TCP
port. Mount only the dedicated socket directory into media; permissions and a
service-specific secret authenticate it. The secret permits validation of an
already presented media capability only; it is not a user credential, grant-
signing key, administrator token or permission to mint arbitrary capabilities.
Do not mount core's general management socket or reuse its private HTTP router.

The dedicated internal router supports only the following fixed operations:

1. **Authorize/open:** accept the presented opaque media grant plus its exact
   request context. The grant freezes the target/audience; core verifies current parent authority,
   target eligibility, endpoint generation and admission expiry, then returns an
   ephemeral read handle and a bounded file-locator DTO: relative immutable locator,
   length, digest/ETag, media type and the existing transfer-validation requirement. The storage adapter derives the locator after
   authorization; no client supplies an arbitrary path, URL or collection lookup.
   Metadata is sufficient to read the existing file, not a second catalog.
2. **Recheck:** validate that read handle against current parent/session/device,
   collection permissions, endpoint generation and stream deadline. It cannot
   change the target or open a new resource. It retains the existing distinction
   between an admitted stream and admission-lease expiry. No full-file read/hash
   occurs in an authorization/recheck call.
3. **Release:** discard the bounded ephemeral read handle. A lost release is
   recovered by a 30-second renewable idle lease and connection/process teardown,
   without a durable job. Five-second media heartbeats keep admitted reads alive;
   a crashed media process cannot hold all user admission slots for four hours.

Public media never proxies requests to this socket verbatim. Parse fixed DTOs
with small body/field limits and bounded concurrency/deadlines. Reject extra
routes, unknown operations, malformed handles, path/URL parameters and authority
changes. Assume a compromised media process can call every internal operation
with its own service secret: those calls must still fail without an actual valid
user-issued capability. Rechecks reuse the original immutable binding, not a
client-supplied replacement principal or a cached "authorized" boolean.

No shared account database or offline grant-signing key is needed. Core keeps the
bounded hashed-grant store and current role policy; media keeps bounded active
read state only. Core failure/unavailability is fail-closed: new reads fail and
existing responses stop when their next bounded authorization check cannot pass.
Do not add stale-authorization grace merely to hide a core outage. Core restart
invalidates grants/read handles; media restart loses open streams; clients renew
through normal control authentication and resume only the same immutable content.
Cloudflare/control-frontend unavailability alone differs from core failure:
existing admitted streams can continue if their internal checks still succeed.

Media opens only under configured read-only roots using a maintained capability-
or descriptor-relative file API that rejects traversal, symlink escapes and
unexpected file types. Check length/digest/validation coordinates and retained
handle identity so publication or substitution cannot splice a different file
into a resumed read. Keep store parsing/resolution in core's storage adapter;
media never reparses archive manifests or scans/reindexes collections. Reuse the
existing bounded range/validation mechanisms through an appropriately narrow
shared mechanism; do not duplicate algorithms or import Collector orchestration.

### Network boundary and deployment modes

The implemented profile uses **no media network interface/routes**
(`network_mode: none`). Both frontend HTTP and private core authorization use
filesystem Unix sockets in separate dedicated Docker named volumes. The
frontend cannot access the core authorization volume. Media's authority volume
is read-only, and its router cannot proxy arbitrary operations to it. Named
volumes keep socket inodes on the Linux daemon filesystem, including on Docker
Desktop; desktop bind-mounted file shares are not a supported socket transport.
Verify no external routes/active interfaces and denied outbound connection on
the actual runtime. Kernel-provided down tunnel devices are not usable network
paths. Core's existing host/private networking stays unchanged.

Only the dedicated TLS frontend publishes a TCP port. It has certificate-renewal
networking and no archive, core state or authority mounts. Its configured proxy
routes cannot access general core APIs, but a compromised frontend's ordinary
bridge networking may reach host/LAN services unless the target host firewall or
DMZ blocks them. Media's network isolation does not make that separate claim.
The frontend retains only NET_BIND_SERVICE in its capability bounding set because
the qualified official Caddy image's file capability otherwise prevents execution;
no-new-privileges remains enabled. Media drops all capabilities.

Support these hosting arrangements with the **same separate executables**:

| Arrangement | Proxy composition | Security/traffic implication |
| --- | --- | --- |
| Selected home setup: two origins | Cloudflare control hostname → core public/client router; direct media hostname → media container | Music bypasses Cloudflare and backend process/storage isolation remains |
| One directly hosted origin | A maintained reverse proxy maps a fixed media path to media and control paths to core's public/client router | Same process/storage separation, simpler same-origin browser loading, but control is directly Internet-facing under that operator's policy |
| One origin behind another suitable ingress | Operator supplies a proxy/service that explicitly supports the media traffic | Same backend boundary; traffic still traverses the selected ingress |

DNS chooses an address by hostname, not by URL path. Keeping one public hostname
behind the existing Cloudflare proxy cannot make `/media` secretly go directly to
home. The additional hostname buys independent ingress routing **and a separate
browser origin**, not the container isolation itself. A separate media origin,
host-only login cookies and strict control-Origin/CORS rules also limit what
malicious active content returned by a compromised media service could do as the
logged-in user. Same-site sibling hostnames are not a substitute for those rules.
Do not require two hostnames for all operators, or a one-process mode
to support one hostname. Same-origin media requests may arrive at the frontend
with login cookies; strip Cookie/Set-Cookie on the media upstream and never forward
those cookies to the media executable or introspection socket. The selected
cross-origin home mode continues avoiding login cookies on media requests entirely.
Cookie stripping in the one-origin mode does not restore browser-origin isolation:
malicious content served under that shared origin could act against the control
application with the user's ambient browser session. Document that additional
tradeoff even though filesystem/process separation remains. Keep media responses
non-active, correctly typed and nosniff in either deployment mode.

## Ingress and deployment boundaries

| Surface | Network path | Permitted operations |
| --- | --- | --- |
| Existing control origin | Phone/client → Cloudflare public tunnel → existing public/client listener | Existing login, passkeys, pairing, metadata/artwork, availability and scoped media-grant issuance; current exposure/role rules remain |
| Direct media origin | Phone/client → static public IP TCP 443 → dedicated TLS frontend → separate media container through its HTTP Unix socket | Authenticated audio HEAD/GET, a minimal fixed-shape identity response, narrowly bounded CORS OPTIONS and setup probes only |
| Internal media authority | Media → dedicated core Unix socket | Presented-capability authorization/open, recheck and release only; never a general core/admin API |
| Existing private media | Existing HTTPS/WebTransport private listeners | Existing private playback; its generated trust and permissions remain separate |
| Private management | Existing explicit private listeners | Existing administration/curation with authentication and role checks; never a public-media upstream |

Use a provisional hostname such as `collector-media.awkwardexplorer.com`;
confirm the exact name and unused ports during deployment preparation. Keep the
existing control hostname, authentication origin, tunnel destination and browser
session unchanged. Do not add a shared cookie Domain or a direct route to login.

Recommended home deployment:

1. Create a DNS-only A record for the dedicated media hostname and static public
   IPv4. Publish AAAA only after separately qualifying its firewall and external
   path. Check that CNAME/HTTPS/SVCB answers do not select Cloudflare or another
   unintended destination. The existing control DNS continues selecting Cloudflare.
2. Use a dedicated Caddy TLS frontend (or an equivalent already maintained proxy
   with the same isolation). Forward WAN TCP 443 to its dedicated NAS listener
   port, **not blindly to the NAS's shared DSM port 443**. Inspect current WAN
   forwarding first; do not replace a mapping used by another service.
3. Serve only the media hostname. Reject unknown Host/SNI/default-site requests;
   no DSM, Immich, control pages, generic file browsing or arbitrary upstreams.
   The dedicated frontend reaches only the media container, never
   `private_bind`, management TLS, or the general public router.
4. Automate public-CA certificate issuance/renewal and proxy reload. Prefer the
   standard ACME TLS-ALPN challenge through forwarded port 443 where supported;
   documented DNS-01 is an operator alternative with narrowly scoped credentials.
   This delivery does not require opening UDP or changing the private 9845 listener.
   Do not forward port 80 unless the selected certificate/redirect setup needs it.
5. Keep proxy configuration and TLS secrets outside source and outside media
   responses. Run the proxy with minimum privileges, read-only configuration/root
   where practical and bounded writable certificate state. Give media its separate
   read-only content/runtime profile and socket-only authority connection above;
   do not inherit the core's writable mounts or host networking.

Security follows explicit listener/router composition and configured endpoint
authority. Never enable media, management, or trust based on client IP, a private
address, Host, `Forwarded`, `X-Forwarded-For`, low latency or a displayed ServerId.
The proxy preserves the explicit public media origin in a fixed deployment
contract; it cannot grant access by adding a header.

## Media authority and public request contract

Extend the existing application-owned playback grant policy for both browser
sessions and enrolled native credentials. The direct listener must **not** accept
passwords, browser login cookies, recovery codes or long-lived native credentials.
Native credentials continue authenticating only to the enrolled control origin
to obtain short-lived media grants for this route.

An authenticated descriptor supplies one explicitly owner-configured remote HTTPS
base URL (origin and optional fixed media-path prefix) and its enabled state/
capabilities, alongside the current private routes. This supports one-hostname
proxy composition without clients inventing paths or weakening endpoint authority.
Bind grants to the enrolled ServerId, exact configured audience and endpoint
revision, parent session/device identity, account, immutable playable track and
expiry. An enabled address is operator authority, not a client-supplied URL.
Changing/disabling that endpoint invalidates its outstanding grants. Do not infer
an alternate URL from DNS, subnet, a redirect or an untrusted descriptor response.

Use opaque CSPRNG capabilities with hashed server-side lookup, extending the
existing bounded in-memory grant store in core rather than introducing custom signatures
or a second account database. Retain only bounded operational state. Store parent
identity references, not another copy of a long-lived bearer credential. Core
restart invalidates these ephemeral grants; clients reacquire through control
authentication. Media queries the narrow private authority port and never gains
direct access to that store.

For the browser, first implement a conventional expiring media URL that an ordinary
`HTMLAudioElement` can load directly. This avoids making public HTTPS playback
depend on WebTransport, a live page broker or service-worker execution while the
phone is locked. Native uses the same scoped authority in an Authorization header.
Reject conflicting supplied credentials. A URL capability is a bearer secret:
possession permits its limited reads until expiry/revocation, and CORS/Origin is
not a substitute for that authorization or proof of who holds the URL. Cross-origin
browser grants require the exact allowed Origin. Same-origin GET/HEAD may omit
Origin only when the frozen browser origin equals the configured media audience
origin; other supplied origins still fail. The proxy strips login cookies on
that media upstream.

- Authorize grant issuance through the existing parent authentication and actual
  collection permissions. Allow only admitted playable music tracks, including
  supported purchased FLAC/MP3 and prepared audio. No generic representation,
  raw-disc/package export, arbitrary path, upload, conversion or provider operation.
  Do not expose another file just because its representation ID exists.
- Recheck the current parent session/device, account and collection access on
  every HEAD/GET, including cache hits. During a long response, check again at
  least every five seconds and before each additional 256 KiB; cancel on revocation,
  disable or failed authorization. Bound queued output so checking does not merely
  happen after an entire file has already been buffered. Already received bytes
  cannot be recalled, and permitted downloads retain existing offline semantics.
  Revalidation calls a narrow authority check; it must not reopen/hash the full
  representation or restart preparation on each interval.
- Use an initial **60-minute maximum admission lease**, capped by parent validity;
  new requests after expiry must obtain a fresh grant. An already admitted response
  may finish after lease expiry while live parent/role checks continue. Keep a
  bounded total response lifetime. Qualify expiry, long tracks, pause/resume and
  later seeking; do not mechanically reuse the current 120-second WebTransport
  grant lifetime or rely on a background JavaScript timer for renewal.
- Permit only GET/HEAD and narrow OPTIONS. Reject request bodies, malformed or
  excessive headers, unsupported multipart ranges and unsupported methods before
  expensive work. Support ordinary full GET, single explicit/open/suffix ranges,
  HEAD, exact Content-Length/Content-Range, strong ETag and If-Range semantics.
  Never silently truncate a requested range to the private protocol's 256 KiB
  message size, fabricate lengths, or splice bytes from different representations.
- Preserve existing immutable-reader/integrity guarantees. Validate bytes only
  in the appropriate cancellable preparation/open path and reuse safe retained
  handles. Routing, identity, HEAD, availability and reachability must not trigger
  full-media hashing; seeking cannot cause a full payload scan per range.
- No redirects, accepted login cookies or Set-Cookie in the media service. The
  one-origin frontend strips login cookies before proxying its media path. Set media type and
  `X-Content-Type-Options: nosniff`; prevent shared caching of capability responses.
  The authenticated grant-issuance response necessarily passes through the
  Cloudflare control connection. For the selected home setup, never proxy a
  capability-bearing media request through Cloudflare or an unapproved third-party
  service, and never send it to external failure reporting. Other operators may
  explicitly choose an eligible media ingress; that choice must be represented
  in endpoint policy rather than silently inferred from a failed direct route.

Browser grants carry the allowed control origin; require it for CORS media loads.
Native grants have a distinct application-owned audience type, not a magic request
header that bypasses browser rules. Use `crossOrigin="anonymous"` before setting
the audio source, exact CORS origin allowlists, `Vary: Origin`, no credentialed CORS
and only needed exposed range/validator headers. Permit the media origin in the
control page's exact CSP `media-src`/`connect-src`. The remote HTTPS path must work
without a local-network permission or third-party cookies.

Exclude capability values and full secret-bearing URLs from proxy/application
logs, analytics, browser storage, queue records, crash messages and exported
diagnostics. Use a no-referrer control-page policy and ensure media responses
contain no active content or onward navigation. Keep only an in-memory playback
lease, and persist logical track/position identity rather than its URL. These
measures reduce URL-secret leakage; they do not make a bearer URL non-transferable.

## Resource limits and remaining exposure

Apply bounds independently at TLS/proxy and application layers: connection and
header deadlines, maximum header/URI sizes, authenticated stream concurrency per
principal and globally, capped retained handles/grants, bounded buffers, an idle
write/read timeout and prompt disconnect cancellation. Restrict unauthenticated
requests before opening files or scheduling validation; give them a separate small
budget so floods cannot allocate the authenticated playback pool. Return bounded
errors and Retry-After for exhausted budgets without revealing protected filenames.

Start tuning for a few home listeners using the existing 32-stream ceiling as an
upper bound, with a smaller per-account allowance and reserved control capacity.
Measure memory, file handles and cancellation under slow readers and repeated
seeks, then record actual limits in the implementation contract. Avoid bitrate
limits that interrupt normal lossless playback. Server-side per-peer limits see
the actual proxy peer unless an explicit trusted-proxy mechanism is introduced;
do not trust arbitrary forwarded client addresses or claim per-user filtering
before the user has authenticated. Proxy connection controls can see its direct
Internet peer independently.

No Internet request starts encoding, imports, filesystem writes to the archive,
provider lookups or expensive discovery. A media outage must not create jobs.
These bounds mitigate application exhaustion; they cannot prevent upstream link
saturation. Use normal update/rollback operations and retain the residual home-IP,
shared-kernel/container-escape, mounted-content disclosure, trusted-control-page/XSS
and token-theft risks in deployment notes. Authentication IPC needs its own small
budget so a compromised media service cannot consume all core authorization work.
Neither a successful reachability test nor a green suite constitutes a security
audit of Collector or DSM.

## Client behavior and architecture ownership

| Owner | Responsibility |
| --- | --- |
| `collector-app` | Remote endpoint eligibility/audience, playable-target policy, shared grant/read-handle issuance, current authorization/revocation and setup-test policy |
| `collector-store` | Core-side immutable file resolution and validation coordinates through a narrow application port; no database/archive parser in media |
| `collector-api-types` | One shared typed descriptor, grant, internal authorize/recheck/release DTO and diagnostic/result contract; no duplicate private DTOs |
| `collector-server` | Existing control/private listeners and a separately composed socket-only authorization adapter; never a general management upstream for media |
| `collector-media` executable | Small media HTTP router, read-only file opens, range delivery, private authority client, cancellation/resource bounds and secret-safe diagnostics; no app/store/providers dependency |
| `collector-client` | Enrolled control identity, remote grant acquisition, normal system HTTPS trust, immutable ranged reads, retries and scoped credential placement |
| Web player / Trash Amp | Existing player/decoder, queue, state and actionable route/error presentation; no parallel authorization or provider pipeline |
| Operator/proxy/OpenWrt | DNS, certificate lifecycle, dedicated TLS ingress and firewall mapping; no router/provider credentials held by Collector |

Core/domain modules gain no networking, HTTP, filesystem or database dependencies.
The media crate/layer is registered in the architecture checker, permitting shared DTO/core value types and narrow external mechanisms
without importing the Collector application/store/provider orchestration. Keep
grant policy in core's application service. Reuse maintained HTTP/TLS/file APIs;
extract shared range/validation mechanisms separately where the two executables
need them, following `junk-libs` instructions and affected sibling checks. Do not
create an umbrella networking framework. The default release image includes both binaries to retain build-once promotion;
separate services receive distinct entrypoints, mounts and networking. An optional
lean `media` Docker target omits core, management assets and encoders. No core
operational state is mounted into either media image. Read
Trash Amp's sibling instructions before modifying it.

Selection is private first, then the explicitly enabled direct HTTPS media route.
Cloudflare remains control-only even when every media candidate fails. Replace
the current public-origin fallback decision, rather than setting
`remote_media_allowed=true` and inadvertently enabling tunneled media. Keep refusal
of music bodies on the control listener as a server-side invariant. Remote grants
do not authorize the private listener's long-lived native-credential path.

Use ordinary public-CA/hostname validation for the remote origin and expected
server identity obtained through the authenticated descriptor before sending a
media grant. A bounded anonymous identity probe can return only ServerId/protocol
version under the small unauthenticated budget; identity is informational until
the configured origin's HTTPS trust is established. No certificate exceptions,
IP-only trust, redirect following or account
credential copied to the alternate origin. Keep initial probes small/cancellable,
reuse recent reachability and back off failures. Browsing and idle polling do not
probe media or prompt for LAN permission. Route changes preserve account, queue,
cache and immutable bytes; cancel/fence superseded async work. Do not restart a
working request merely to chase a faster route or promise gapless handoff.

If a lease expires, renew through control and retain position; account/role
denials terminate playback rather than cycling routes. An unavailable control
tunnel prevents new grants/renewal but need not interrupt an admitted authorized
response. A network failure offers retry and preserves position; it never unlocks
Cloudflare media. Explicit native audio downloads follow the same endpoint/grant
policy. Existing offline files, artwork and control browsing retain their contracts.

## Remote reachability test

Provide a management setup action, **Test remote media**, using the configured
origin only. Collector never becomes a general-purpose URL fetcher/port scanner
and does not open ports or edit DNS. Use bounded timeouts/bytes, no redirects and
no untrusted client override of the target host. The diagnostic contract separates
these claims:

| Result | Meaning |
| --- | --- |
| Local checks passed | Configured DNS/HTTPS/identity and local listener/proxy checks passed from this observer; public Internet access is unproven |
| External transport confirmed | An identified outside observer completed a fresh bounded HTTPS challenge on the configured media origin |
| Authenticated media read confirmed | That observer received exact early/late ranges of an authorized real audio representation with matching validators/lengths |
| Playback qualified on named device | Actual playback, seeking and required background/network transitions passed on that named client |

First implementation uses the user's **phone with Wi-Fi disabled, on cellular**
as the outside observer. Open a test page under the existing Cloudflare control
origin and authenticate normally. A setup link/QR contains only a non-secret
attempt identifier, not a login credential or media capability. The administrator
creates the bounded test attempt; a signed-in listener must still have permission
for the selected track and cannot change endpoint configuration.

1. The control service issues a short-lived, single-purpose test challenge bound
   to the configured origin, ServerId, endpoint revision and test attempt. The
   direct listener returns a small fixed response with the challenge, expected
   identity and no library information. Its dedicated probe capability grants no
   audio access. Authenticate/restrict probe creation and consumption.
2. Test the actual cross-origin browser request with normal TLS/CORS. Reject
   redirects, wrong origin/identity, stale challenges and mismatched endpoint
   revisions. Consume the challenge once with bounded retry by obtaining a fresh
   challenge. A matching displayed ServerId alone is insufficient.
3. Obtain a separate playback grant through the existing control origin. Perform
   HEAD and two exact small ranges near the beginning/end of one authorized audio
   file, at most **64 KiB total media payload**. Compare expected immutable length,
   ETag and bounded byte digests from the authorized source. Do not hash the whole
   track to construct this test. No eligible track means transport can pass while
   authenticated playback remains untested.
4. Report typed stages, observer, time, endpoint revision and bounded failure
   categories. Browser fetch may conceal whether DNS, TCP, TLS or CORS failed;
   report that uncertainty and supplement it with server/native diagnostics rather
   than guessing from display text. Never log capability URLs or media contents.
5. Then play real audio, seek forward/backward and check actual output. Qualify
   locked-screen playback, next-track behavior, expiry/renewal, pause/resume and
   Wi-Fi/cellular transitions on the user's actual phone/browser. A range test does
   not prove those behaviors. Record browser/OS and whether audio was audible.

The page asks the user to disable Wi-Fi; browser JavaScript cannot reliably attest
that the phone is on cellular. Label that observation as **user-confirmed cellular**,
not machine-proven physical location. A LAN self-request through the public IP can
only establish hairpin/NAT-loopback reachability. An optional existing external
monitor can independently test transport later; operating a new checking service
is not a dependency. Successful certificate issuance alone does not prove media
authorization, CORS, playback or that music bypassed Cloudflare.

Keep the bounded recent diagnostic result only in the open page's memory, with
its endpoint and timestamp/observer evidence, separate from collection history.
No diagnostic history or authority is persisted. The fixed endpoint configuration
requires a core restart; that process generation invalidates prior grants/handles
without a durable endpoint-revision migration. A client report is diagnostic
data, not authority: it cannot enable a route, open a firewall or grant access.
Configuration changes make previous results stale. A successful run confirms that
path at that time, not universal network reachability or future availability.

## Complete delivery and acceptance

Implement the authority contract, media listener, public-HTTPS browser path,
shared native reader, setup test and operator runbook together. Internal steps
are an implementation sequence, not separate layer handoffs:

1. Confirm deployment boundary/ports, socket permissions, mount scope, threat
   assumptions and current proxy inventory. Define the media executable/layer,
   narrow authority contract and disabled-by-default remote endpoint. Record the
   mounted-content confidentiality limit before preparing WAN access.
2. Complete the separate container/image, read-only file adapter, socket authority,
   bounded ordinary HTTP reads and negative exposure/isolation cases on disposable
   installations. Adapt both clients, preserving existing private playback,
   control identity and native downloads. Support separate and single-origin
   proxy composition through the same media process, not a second auth pipeline.
3. Deliver the real setup UI/test, browser lease lifecycle and actual player
   acceptance. Include the user's phone browser, current desktop Chrome and
   supported Trash Amp. Qualify Safari/Firefox explicitly when claiming support;
   no extrapolation from Chrome, browser emulation or future mobile Trash Amp.
4. Prepare reviewable operator DNS/proxy/OpenWrt/certificate/rollback changes.
   Actual deployment is a separate authorized action, followed by the short
   cellular acceptance, exposure check and evidence that music used the direct
   listener while login/control used Cloudflare.

| Acceptance area | Required meaningful evidence |
| --- | --- |
| Listener boundary | Login/pairing/grant issuance/library/curation/admin/uploads/exports unavailable at media ingress; spoofed headers, unknown Host and default proxy sites cannot expose them; music bodies remain refused through Cloudflare control |
| Container/storage boundary | Actual media identity cannot create/overwrite/unlink/rename content, including nested mounts; state/config/core secrets and Docker socket are absent; attempts to read outside declared roots fail; documented in-root readability is not misreported as per-account containment |
| Internal authority/network | Media service secret alone cannot mint grants or resolve arbitrary files/users; forged/changed/stale handles fail; core loss/restart fails closed and cancels active reads; management/host/LAN/Internet access from media is denied as configured; resource exhaustion cannot monopolize core control work |
| Grant authority | Missing/expired/stolen-scope grants, other server/account/track/origin/audience, parent expiry/logout/device revocation, collection revocation, endpoint disable/change and restart fail closed; valid multi-range reads and renewal work |
| Read correctness | Real byte-exact full/single/open/suffix ranges, HEAD/ETag/If-Range/416, truncated source and changed representation; no protected raw/package target admission or path traversal |
| Abuse/recovery | Malformed/oversized input, repeated unauthorized calls, slow readers, concurrent seeks, pool exhaustion and disconnects remain bounded; revocation interrupts an already open response within its bound; freed capacity permits subsequent playback |
| Secret handling | No long-lived credential sent to media; no media capability in retained logs, browser storage, queue data, referrers or diagnostics; no shared-cache/cross-account response leak |
| Reachability | Negative cases for hairpin-only success, absent forward, wrong TLS/identity/CORS, redirect, stale challenge/revision and fake client success; actual cellular transport/read success recorded separately from playback |
| User workflow | Normal Cloudflare sign-in → automatic selected route → audible media/seek; phone lock/next-track/lease-boundary and network changes; unavailable route shows useful retry without Cloudflare fallback; native download/offline identity preserved |
| Operations | Dedicated ingress/default-host boundary, IPv4 and any published IPv6, certificate renewal/reload, control/private isolation and reversible disable/rollback verified |
| Hosting flexibility | Two-origin home deployment and one-origin path composition both route to the same separate media executable; one-origin proxy strips login cookies before media; neither DNS nor a header is treated as path-specific Cloudflare bypass |

Prefer focused fixture tests for hostile inputs and failure cases; live acceptance
uses one short known track, small range probes and named devices. No stress/load
attack on production, bulk remote playback or repeated full-payload integrity
scans. Confirm public/private route and grant behavior in code, not solely through
a proxy's path filters. Check affected sibling projects when shared client code
changes; run repository architecture, Rust and web gates, rebuild the Rust server
after embedded assets change and run the real browser smoke. The dated acceptance records actual suite runs; reading this contract alone does
not qualify an implementation.
The default source release image contains both executables; the optional lean
media target is separate. Public release acceptance must exercise both published
architectures with the core/media pair from the same release; local ARM64 evidence
does not pass that gate. Bound internal protocol version
handling and report mismatches; do not add checkpoint-only compatibility layers.

Update architecture, implementation status, the implemented playback contract,
HL-12/HL-05 and a dated validation record when delivery occurs. Record unrun phone,
browser, deployment, renewal or stronger isolation cases honestly. Preparing this
plan creates no successful validation record and performs no release/deployment.

Rollback disables remote descriptor/grant issuance, cancels active remote reads,
removes the dedicated WAN forwarding and stops the dedicated TLS frontend as
appropriate. Invalidate outstanding grants; remove only the dedicated DNS/proxy
entries when needed. Keep existing Cloudflare login/control and private listening
usable, along with accounts, originals and state. Stop the media container and
retire its dedicated socket/service credential as appropriate without deleting
shared content. No archive conversion is required.

## Source basis and existing implementation

The conventional design inputs below were checked on 2026-10-23; they describe
mechanisms, not evidence that this Collector delivery already works.

- [Cloudflare proxy status](https://developers.cloudflare.com/dns/proxy-status/):
  DNS-only records send clients to the origin rather than Cloudflare's HTTP proxy.
- [Caddy automatic HTTPS](https://caddyserver.com/docs/automatic-https): maintained
  public certificate issuance/renewal and supported ACME challenge mechanisms.
- [HTTP semantics / ranges](https://httpwg.org/specs/rfc9110.html#range.requests):
  normal HTTP representation, validator and range behavior.
- [Audio element](https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/audio):
  browser media URL and cross-origin loading behavior; actual phone qualification
  remains necessary.
- [Browser same-origin policy](https://developer.mozilla.org/en-US/docs/Web/Security/Same-origin_policy):
  separate media/control origins provide a browser boundary in addition to
  separate process/storage privileges; path routing alone does not.
- [Expiring media URLs](https://docs.aws.amazon.com/AmazonCloudFront/latest/DeveloperGuide/private-content-signed-urls.html):
  an established content-authorization pattern, not a proposal to use CloudFront
  or duplicate its signing infrastructure.
- [OWASP REST security](https://cheatsheetseries.owasp.org/cheatsheets/REST_Security_Cheat_Sheet.html)
  and [resource-exhaustion controls](https://cheatsheetseries.owasp.org/cheatsheets/Denial_of_Service_Cheat_Sheet.html):
  authorization, secret handling and bounded work require application enforcement.
- [Docker security](https://docs.docker.com/engine/security/) and
  [read-only bind mounts](https://docs.docker.com/engine/storage/bind-mounts/):
  process/filesystem restrictions provide a meaningful shared-kernel boundary;
  read-only access still permits disclosure of mounted files.
- [Docker bridge behavior](https://docs.docker.com/engine/network/drivers/bridge/):
  shared network membership does not restrict access to one intended service port.
- [NGINX authorization subrequests](https://nginx.org/en/docs/http/ngx_http_auth_request_module.html):
  established delegated authorization pattern; not a requirement to use NGINX or
  a claim that a single initial subrequest provides ongoing revocation checks.
- [Current private playback](automatic-private-playback.md),
  [security contract](decisions/security.md),
  [existing home HTTPS deployment](validation/lan-https-2026-10-22.md) and
  [container configuration](../deploy/container/compose.yaml): reusable mechanisms
  and concrete deployment constraints, not remote-media qualification.
