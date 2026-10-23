# HTTPS relay and automatic LAN connections

**Remote direction selected, 2026-10-23:** use the
[direct HTTPS media plan](direct-https-media-plan.md): existing Cloudflare control/
login plus a dedicated direct public media-only origin. This is design only and
supersedes the relay/Iroh-first remote ordering below for that delivery. Existing
private playback remains implemented separately; HL-12 is still the only queue.
The same-day isolation refinement selects a separate read-only media executable/
container with a narrow core authorization socket. One-hostname proxy composition
can retain that process boundary; the chosen home setup uses two ingress origins.

**Source update, 2026-10-22:** [automatic private playback](automatic-private-playback.md)
is now implemented for web and Trash Amp. See [dated acceptance](validation/automatic-private-playback-2026-10-22.md)
and the [sole queue](remaining-work.md#hl-12--https-relay-lan-switching-and-deferred-certificate-rotation)
for deployment/device limits and deferred public relay work. The manual LAN URL
remains the deployed stopgap. The older planning/feasibility statements below
are retained as dated history, not current implementation status.

Planning record, 2026-10-22. Requested outcome; **not implemented or qualified**.
This is a delivery design, not another remaining-work queue. Tracking belongs to
[HL-12](remaining-work.md#hl-12--https-relay-lan-switching-and-deferred-certificate-rotation),
with deployment acceptance under HL-05. Preserve the unrelated active assignment
in that queue. This planning request does not provision infrastructure, change
live DNS/configuration, publish packages or authorize a paid service purchase.

**Scope clarification, 2026-10-22:** the real solution requires automatic LAN
detection and route selection in **both the browser and Trash Amp**. The deployed
`https://collector-lan.awkwardexplorer.com/` is an accepted manual stopgap, not
completion of this plan or HL-12. Both clients must detect a reachable trusted
local server from their normal entry point/saved connection, prefer local media
traffic and retain/recover authorized remote access without manual URL selection
or route-change re-enrollment. A Mac helper is not the intended solution.
Native-only selection or a successful manual/split-DNS deployment does not qualify
browser detection. This supersedes earlier text deferring the browser outcome;
the browser mechanism still requires a bounded design/feasibility step.

**Priority and scope clarification, 2026-10-22:** first avoid the unresolved public
Cloudflare media tunnel, then make normal listening simple. Prefer actual direct
LAN transfer to conserve the owner's Internet upload allowance and infrastructure.
Existing private network reachability may help without any provider integration.
There is **no Tailscale integration work** in this feature: no SDK/CLI/API access,
peer enumeration, network configuration, Serve/Funnel provisioning or Tailscale
path monitoring. An endpoint already reachable through the OS network stack is
just another authorized HTTPS candidate. Same-tailnet membership alone does not
discover a Collector endpoint or prove physical LAN delivery. The earlier tsnet
research below is not a selected dependency or implementation assignment.

**Setup boundary, clarified 2026-10-22:** target install → sign in/pair → use for
listeners, and avoid making private-server users repeat manual certificate imports,
internal DNS changes or router/proxy setup. Public server operators own their
chosen HTTPS/certificates/tunnels; these are supported hosting inputs, not a mandate
for Collector to provision a particular provider or service. The earlier Caddy/
rathole topology is an optional deployment example, not a universal dependency.
Manual public-CA LAN setup is a supported path and accepted stopgap, not the desired
general private-network onboarding. Prefer software-managed peer trust and encrypted
direct transports where they meet the real browser/native playback contract.

## Minimize private setup; keep public hosting independent

“Private overlay” here means an encrypted connection between authorized peers over
the network they already have. It need not create a virtual network interface,
require a separately installed VPN, or involve Tailscale. Collector can bind a peer
identity to an account's existing trusted enrollment/session, then let maintained
transport libraries establish a direct path. Discovery locates candidates; trusted
enrollment/signaling authenticates them; normal collection authorization still
decides what may be read. Encryption by itself supplies neither discovery nor
account access. A relayed private path is not automatically local or upload-free.

Public operators choose the public HTTPS/certificate/tunnel provider and maintain
that ingress. Collector accepts explicit public URLs and exposure/media policy,
without provider credentials, a compulsory DNS domain, a vendor-managed wildcard
certificate/private key, mandatory relay subscription or particular reverse proxy.
Optional deployment examples must remain optional. A browser still needs a trusted
entry point/bootstrap; these transport experiments do not solve secure first login
to an arbitrary untrusted HTTP page on an entirely private network. Establish that
trust through a supported sign-in/pairing workflow, not warning bypasses or a
silent trusted-on-first-network-response decision.

The desired private experience is one normal sign-in/pairing and, where the browser
requires it, a local-network permission action. Internal peer keys/certificates,
their authenticated delivery and necessary renewal should be software-managed.
Do not turn the manual DSM/DNS-01 procedure into the normal install instructions.

| Candidate | Private certificate experience | Current evidence / decision |
| --- | --- | --- |
| Trash Amp with authenticated retained native certificate/peer trust | No OS trust import; application learns authorized trust during enrollment/refresh | Existing native generated-certificate enrollment/range fixture passes. Automating alternate-endpoint authority and selection is still work; Iroh remains a native transport trial. |
| Browser WebTransport with `serverCertificateHashes` | Browser authenticates the supplied server certificate hash without a public CA for that local transport | Pinned NAS connections, real Collector ranged bytes/authorization and operator-confirmed work-Mac Chrome playback/forward-backward seeking pass in an isolated real-player adapter under the requested GlobalProtect conditions. This does not change ordinary fetch/audio certificate rules. |
| Browser WebRTC reliable data channel with authenticated signaling | Transport manages DTLS certificates; no local public-CA import | Actual Firefox/Chrome host-to-host primitive passed on one computer. Evaluate if WebTransport compatibility, UDP/path constraints or playback adaptation are unsuitable. It still needs signaling, peer binding, authorization and bounded byte-stream adaptation. |
| Operator-provided browser-trusted HTTPS | Public operator/advanced private operator owns certificate/proxy lifecycle | Already works in the accepted manual LAN setup; retain as supported transport/control and a fallback when supplied. It is not a mandatory private-user certificate exercise. |
| Iroh in the browser | Encrypted peer transport without ordinary local HTTPS configuration | Current upstream browser support is relay-only, so it is not the solution for same-LAN WAN avoidance. Reassess only if upstream capability changes. |

See [dated tests and limits](validation/direct-transports-2026-10-22.md). These
primitives were exercised on the personal Mac only, not across the NAS LAN or
inside the work VPN. They do not deliver automatic playback or a secure enrollment
contract. Do not build all transports into production pre-emptively.

The subsequent [NAS/LAN player experiment](validation/webtransport-lan-2026-10-22.md)
extends that evidence across real devices: pinned transport, byte-exact early/late
ranges, live session/collection authorization, revocation and the actual player's
source release. Its experimental service-worker/page bridge leaves the real
player source unchanged and disables HTTP media fallback. No audio device is
reported on the personal Mac and independent audio output fails, so its frozen
playback clock could not qualify playback/seek. The subsequent work-Mac Chrome
test passes playback and forward/backward seeking with no errors or unexpected
prompts noticed under the requested GlobalProtect-connected conditions. Saved
diagnostics show late/earlier ranges, cancellation and passing pin/authorization
controls. Production transport/discovery and the public-origin private-network
transition remain open.

The [public-origin experiment](validation/public-origin-webtransport-2026-10-22.md)
subsequently qualifies Chrome's actual deny/allow behavior through an approved,
scoped test path on that origin. Denied Local network blocks connection; granting
it permits pinned WebTransport with passing descriptor/pin/authorization controls.
The normal product will need an explanatory first-use permission action when
permission is `prompt`, actionable denied/managed feedback and automatic reuse
when already granted. No private certificate import is needed for this transport.
The operator also confirms public-origin playback/scrubbing; a stricter CSP
correctly rejects the same endpoint even when LAN permission is granted. The test
forwarding is retired and its fixtures stopped. The path-scoped cookie/worker do
not convert the production app into that delivery; authenticated production
integration and automatic selection remain open.

WebTransport's pinned-certificate mode needs a suitable short-lived certificate
(currently less than two weeks under the documented contract). Collector must
generate/refresh it and deliver its hash through an already authenticated channel,
binding server identity, endpoint, purpose and validity before credentials are sent.
Wrong/expired/unbound hashes fail closed. Design rollover, cached trust, restart and
offline behavior explicitly; this required operational lifecycle is not the
separately deferred general local-identity-rotation framework. HTTP/3/WebTransport
is not present in the current LAN TLS listener, and an Iroh QUIC endpoint is not
automatically a WebTransport endpoint.

With WebRTC, bind the offer/answer's peer fingerprint to the authenticated Collector
session and intended account before accepting the channel. Keep ICE/signaling data
out of public logs; a candidate address is a hint, not an identity. Use a reliable
binary transport for existing file/range semantics rather than voice-call audio
transcoding. Neither a successful WebTransport pin nor DTLS handshake grants a
collection role or access to the private management surface.

The browser still uses the normal app/login origin. A streaming adapter must bridge
the selected transport into the real player's bounded byte/range reads, preserving
seek, cancellation and immutable identity. A worker-based stream response is a
candidate to test; neither WebTransport nor WebRTC is directly an `<audio src>` URL.
The existing HTTPS/lease experiment below remains the comparison/control for hosts
that provide trusted local HTTPS, not a requirement to deploy private PKI.

**Revised first gate:** use the now-tested pinned WebTransport primitive with a
separate real LAN Collector fixture and the actual browser player. Compare an
authenticated WebRTC data-channel adaptation only if the chosen contract cannot
meet browser/network requirements. Include UDP denial, real work-Mac permission/VPN
behavior, Safari, authorization/revocation, bounded streams, later seek and actual
LAN/WAN measurements. On native, test authenticated automatic delivery of alternate
trust using the existing generated-certificate path before requiring user certificate
files. Keep the Iroh trial bounded before a general native transport manager.
The product requirement is minimal setup in both clients, not one particular library.

Sources: [WebTransport certificate hashes](https://developer.mozilla.org/en-US/docs/Web/API/WebTransport/WebTransport#servercertificatehashes),
[WebRTC encrypted data channels](https://developer.mozilla.org/en-US/docs/Web/API/WebRTC_API/Using_data_channels),
[Iroh browser limitations](https://docs.iroh.computer/languages/wasm-browser).

**Later request, 2026-10-22:** deliver home listening before this remote project.
The user can configure local DNS. The [LAN-first plan](lan-playback-first.md)
selects the existing HTTPS hostname through a trusted local proxy for **both
browser and Trash Amp**, with bounded native selection as the fallback. This
supersedes the earlier trial-first ordering for that home milestone only;
the Iroh trial still precedes a general native route/resume manager.
The user's required work VPN adds a client qualification gate: verify DNS and
direct NAS reachability independently. Neither split DNS nor native route
selection guarantees LAN access through a profile that denies it.

## Recommended outcome

Listeners install Trash Amp or open a normal HTTPS URL, sign into Collector, and
listen. They do not install a VPN or join the operator's tailnet. Both the browser
and Trash Amp automatically detect and prefer a verified reachable local server,
retain remote access when local access is unavailable, and return locally without
manual address selection or route-change re-enrollment. Preserve normal browser
HTTPS trust and deliberate origin/authentication boundaries. A bounded browser
feasibility step must choose how to meet this outcome under actual browser and
work-VPN constraints; do not assume native mDNS/Iroh capabilities exist in browsers.
Local DNS remains useful where effective, but its deployment alone does not close
the automatic browser requirement.

Use existing components for the public HTTPS relay. Make a bounded Iroh native
integration trial the first remote/native-transport implementation step, before
writing a general route manager. Home listening need not wait for that trial.
Extract reusable mechanisms into **`junk-libs-net` in the existing junk-libs
workspace**. If Iroh fits, this crate wraps it; otherwise it owns the small HTTPS
route/resume mechanism over established HTTP/TLS libraries. Do not implement both
native stacks or a new VPN/NAT traversal protocol as a prerequisite.

A separate network-junk repository is unnecessary for the initial two consumers.
The crate should still build and test independently, contain no Collector/Trash
Amp dependencies, and be easy to move if its consumers or release cadence diverge.

## Route policy: trust, reachability and transfer cost

Detect **a reachable, authorized instance of this server**, rather than declaring
that two devices share a LAN from their addresses. Matching private subnets, low
latency, a displayed ServerId and forwarded headers are not sufficient evidence.
Separate endpoint authority, current reachability, media eligibility and knowledge
of the underlying traffic path. Only verified, media-eligible candidates compete.

| Candidate/path | Playback policy | Internet-upload implication |
| --- | --- | --- |
| Owner-configured direct home HTTPS endpoint, validated and reachable | Prefer over a remote route; prove the local path during installation acceptance | Actual LAN transfer avoids the media upload through the home WAN |
| Another explicitly authorized reachable private HTTPS endpoint | Useful generic fallback; can bypass Cloudflare without provider-specific code | Physical path/cost may be unknown; do not call it LAN or zero-WAN solely because its address is private |
| Separately qualified remote media endpoint | Use when allowed and no preferred local route is usable | Home Internet upload is still consumed, including with direct remote peer-to-peer transport |
| Existing unresolved Cloudflare tunnel endpoint | May continue serving small login/descriptor/control requests; **not an automatic media fallback** until explicitly qualified/enabled | Sending home playback out through that route defeats the upload-saving goal |

The operator supplies endpoint authority and media eligibility once at server
setup; listeners do not type several addresses or manage DNS. An authenticated
descriptor carries the permitted candidate set and capabilities. Treat advertised
route kind as policy/configuration, not measurement: record a path as unknown
when the client cannot establish it. An already configured overlay/private path
can work unchanged, but there is no promise that merely installing Tailscale makes
the server discoverable. The server-side descriptor must already name a usable
endpoint, with acceptable HTTPS trust and network access.

Tailscale documentation distinguishes direct connections from relay paths. That
explains why a private endpoint can avoid Cloudflare yet still carry packets over
the Internet; this plan does not inspect or control those paths. See
[relay/direct connection documentation](https://tailscale.com/docs/reference/derp-servers).

Default to local preference and an operator-declared set of allowed remote media
routes. If none is eligible, keep browsing available where possible and show
“Local server unavailable” with a useful retry; do not quietly stream over the
excluded Cloudflare route. A conservative owner setting can prohibit remote media
fallback for a capped connection. It must treat unknown-cost private paths
conservatively too; it cannot advertise a hard ISP-budget guarantee from endpoint
labels alone. No billing integration or monthly quota engine is required here.

One hour at a constant 1 Mb/s is about 450 MB of payload; five hours daily for
30 days is about 67.5 GB before overhead/retries. Hairpinning that playback through
an external service can consume that much home upload despite the listener being
at home. If the ISP meters combined directions, the returning download also
matters. Relays and remote peer-to-peer do not remove the home uplink leg. Measure
actual WAN counters in controlled tests; application byte totals are not an ISP
bill or proof of the physical path.

## Simple discovery and secure endpoint selection

1. After normal sign-in/enrollment, fetch the bounded authenticated connection
   descriptor once and retain the authorized endpoint/trust information. Refresh
   on configuration changes or expiry, not on each track. Existing single-origin
   sessions remain usable while the new contract is introduced.
2. Probe the known owner-approved HTTPS candidates with small, cancellable
   identity requests and no account credential. Authenticate the exact approved
   TLS origin (or enrolled certificate/peer for native transport), then compare
   expected server identity. A matching ServerId on an unapproved origin is not
   proof. Only then establish a scoped authenticated read context.
3. Select eligible direct local playback before opening a media source. Initial
   tuning: two parallel probes, roughly one second of foreground local checking,
   and a two-second per-candidate deadline. Reuse successful recent checks. A
   working local route avoids opening a competing remote media request. If the
   quick check expires, only an eligible remote route may start while bounded
   local checking finishes; measure the latency/egress tradeoff in the spike.
4. Native mDNS/DNS-SD may supply address hints for a known server, bounded by
   enrolled trust. Browsers start with descriptor-provided HTTPS names; do not
   depend on multicast enumeration or scan private/overlay address ranges. A
   remembered authorized endpoint may work through ordinary routing even where
   discovery multicast does not. The mechanism is network-provider independent.
5. Recheck on open/reconnect, real connection failure and network/sleep-wake
   changes where observable. A tab becoming visible or a new playback request can
   trigger a cheap stale-health check; browser `online` events alone do not prove
   LAN reachability. Share one selection/probe lifecycle per server/account.
6. Preserve queue, position, account and immutable content across selection. Do
   not abandon a valid in-flight range just to chase slightly lower latency.
   Apply permission/identity/authorization failures distinctly, fence late
   callbacks and keep existing revocation semantics. No gapless browser handoff
   is promised by this first selection delivery.

Keep health short-lived and candidate trust durable. Prefer event-triggered work,
exponential failure backoff and pooled connections over constant polling. The
earlier 15-second active polling figure below is a fallback ceiling to measure,
not a requirement to poll every idle tab. Do not fetch media as a health probe,
hash library payloads for routing, duplicate whole-track downloads or add parallel
playback/transcoding simply to select a path. Local preference is policy ahead of
latency; a slightly faster public relay must not win over an eligible local route.

## Browser path: one normal entry point, local media

Retain the current app/login origin and queue while its playback transport selects
a validated encrypted local media endpoint. The private-setup gate above now
prioritizes certificate-hash WebTransport, with WebRTC as an alternative. The
following HTTPS media-lease baseline is a comparison for operator-provided trusted
HTTPS endpoints, using the accepted LAN URL we already have.
This avoids moving the whole application between origins and recreating login,
passkey and local-state continuity. Small control requests may still use the
existing public frontend; media bytes must use the selected allowed path. Reuse
the local read context for artwork/catalog reads where supported, without adding
a second metadata/publication pipeline. No listener helper or VPN is required.

The current player uses same-origin HEAD/range fetches and a normal audio-element
URL. It cannot safely gain alternate-host authentication by replacing that URL.
Build the spike against that actual player and server, not a mock fetch-only demo:

- For ordinary browser HTTPS candidates, use public-CA trust and exact Origin/CORS allowlists,
  with HEAD, GET, Range, Content-Range and strong-validator behavior exercised.
  Keep normal certificate checks; native certificate pins cannot establish browser
  trust for ordinary fetch/audio. The WebTransport-specific hash mode above can
  authenticate a software-managed certificate separately. Public-host HTTPS
  renewal is operator-owned; private peer trust should be managed by the app. Update
  CSP `connect-src`/`media-src` for the authorized candidate origins; no wildcard
  policy or arbitrary origins supplied by discovery. Set the media element's CORS
  mode before assigning its source, and avoid sending login cookies to alternates.
- Do not widen the login cookie's Domain to all subdomains or assume third-party
  cookies are usable. The existing trusted session can request an explicit,
  audience-bound local read grant; local media requests still enforce the account,
  collection, representation and revocation rules. The grant contract is application
  policy, not a CORS permission masquerading as authorization.
- First test ordinary audio-element streaming using a short-lived, resource-scoped
  opaque media lease that the approved local endpoint validates. A lease must
  support the browser's multiple HEAD/GET/range requests, not be consumed by the
  first request. Expiry/refresh must preserve position and later seeking; revoked
  access stops subsequent requests. Treat any URL capability as a secret: bounded
  audience/lifetime, no redirect, no referrer leakage, and redaction from proxy/app
  logs and diagnostics. Never put the account session or long-lived native token
  in a media URL. Do not assume a random URL prevents unauthorized use.
- Compare that baseline with a thin streaming service-worker transport only if
  browser URL-secret handling, expiry or range behavior makes the ordinary element
  approach unsuitable. The worker would attach scoped authorization to local
  fetches and return bounded streaming responses to the existing player. It adds
  lifecycle/permission/range complexity and must earn its place through real tests;
  no whole-album blobs or new decoder pipeline are justified by this feature.
- If cross-origin media cannot meet the tested browser contract, assess a secure
  automatic top-level origin handoff with a single-use login transfer and explicit
  state continuity. That is a fallback design to measure, not permission to ship
  manual bookmarks, repeated passwords or browser trust exceptions as completion.

Local-network permission may be unavoidable. Ask once in a clear user action such
as “Use nearby server”, remember the browser's decision where supported and show
honest denial/retry instructions. Do not require browser flags or disable managed
VPN/browser policy. Chrome and Firefox document local-network access permissions;
test actual Firefox, Chrome and Safari versions rather than infer behavior from
one browser or a WebKit development change. Media `crossOrigin` and CORS control
fetch/credential behavior but do not create a login session or server authority.
Sources: [Chrome LNA](https://developer.chrome.com/blog/local-network-access),
[Firefox permissions](https://support.mozilla.org/en-US/kb/control-personal-device-local-network-permissions-firefox),
[media crossOrigin](https://developer.mozilla.org/en-US/docs/Web/API/HTMLMediaElement/crossOrigin),
[CORS](https://developer.mozilla.org/en-US/docs/Web/HTTP/Guides/CORS).

**HTTPS comparison gate:** the earlier estimate was 1–2 engineering days on this
baseline; the revised first gate above must
record actual login/grant/permission/range/seek/revocation results in Firefox on
the work Mac with GlobalProtect connected, plus Chrome and Safari smoke checks.
Record required user interactions and a measured route-selection delay. Reassess
the complete implementation estimate and authentication contract from those
results. The accepted manual LAN URL is a useful real endpoint for the experiment,
not proof that automatic cross-origin playback already works.

### Delivery order for the current priorities

The next source delivery should be one complete browser-and-Trash-Amp workflow:
owner-configured candidate authority/media policy, authenticated descriptor and
scoped local-read authorization, bounded selection at playback open/reconnect,
real UI status, and actual local playback/seek acceptance. Use a separate real
LAN fixture for the private transport and retain the working home HTTPS endpoint
as a control, with one controlled unavailable/remote alternative.
Listeners use the normal login/saved server; one browser permission interaction
may be required. The chosen private transport must manage its trust lifecycle;
public ingress/certificate maintenance belongs to its operator. No second LAN
account, address picker, hosts entry or privileged listener helper is required.

Use existing native HTTPS for this bounded selection if suitable. The Iroh trial
still gates a new general native transport/resume manager, but is not a dependency
of simple verified HTTPS candidate selection or of browser local playback. Stage
native mDNS address refresh after known-candidate selection, retaining it in the
full automatic-discovery outcome. Seamless mid-stream handoff, relay deployment,
provider path telemetry and budget accounting must not silently become dependencies
of the first complete local playback selection delivery. Keep their honest limits
visible and deliver the subsequent DHCP/network-change cases before claiming the
wider automatic-discovery plan complete.

Acceptance for this bounded delivery must prove:

- Both actual clients choose local media from the normal entry point with no
  manual URL switch/re-enrollment, including the VPN-connected work Mac. Verify
  playback and later seeking; revoke authorization and deny browser permission
  to exercise real failure paths. Unsupported/denied paths are visible failures.
- On home playback, a controlled router capture/counter comparison shows media
  bytes stay on LAN and the public Cloudflare tunnel carries no media payload.
  Small descriptor/authentication and TLS/discovery traffic are recorded separately;
  no zero-packet Internet claim is made. Measure idle/active probe overhead too.
- Removing LAN never starts media on the excluded Cloudflare route. An eligible
  remote alternative works only when the owner's policy allows it; a conservative
  no-remote-media policy stops rather than consuming an unknown-cost route.
- A preconfigured generic private routed endpoint works without network-provider
  code when it is already authorized/reachable, while remaining cost/path-unknown
  if unobservable. Do not provision or inspect Tailscale to run this work.
- Forged discovery, wrong TLS/server identity, expired/revoked grants, cross-origin
  redirects, stalled probes and an unrelated private host receive no account
  credentials or successful-looking fallback. Late results after logout/removal
  cannot revive playback. Existing queue/content/cache identity is retained.

Use simple statuses such as Nearby server, Private connection, Remote and
Unavailable, driven by typed evidence. Keep diagnostic route details optional.
An unobservable overlay path is not shown as confirmed LAN or free upload.
Prefer existing retained/offline media when already eligible under its current
contract; do not create a permanent cache or extra transcoding as a side effect
of route detection. This feature is efficient connection selection, not a new
traffic-monitoring product.

## What exists and what must change

The current checkout has useful pieces, but no complete automatic transport:

- `collector-client/src/lib.rs` defines server/account identity, endpoint types,
  certificate trust, enrollment and a `best_endpoint` LAN preference helper.
- `library.rs::api_url` selects the first healthy HTTPS endpoint; it does not run
  the preference helper or establish current reachability. Configuration currently
  labels all advertised endpoints LAN, and marks them healthy without a probe.
- System-HTTPS enrollment intentionally retains only the authenticated origin.
  Its one trust mode cannot safely cover a public Web-PKI relay and a locally
  pinned server certificate simultaneously. Removing the truncation is unsafe.
- `media.rs::RangeReader` has bounded, cancellable 256 KiB reads, strong validator
  checks, exact lengths and seek support. A reader currently captures one URL and
  client. Preserve its byte checks while changing transport selection.
- Trash Amp's `amp-collector` owns Keychain credentials, server/account snapshots,
  downloads and the playback adapter. Its connection ledger explicitly defers
  discovery/failover. Existing playback and download identities must survive a
  route change; route URLs must not become content or cache identity.

The new public relay can serve the existing web app and native enrollment without
an API redesign. Native switching requires new authenticated endpoint information,
per-endpoint trust, live transport state and reader integration. Browser automatic
selection additionally needs a chosen browser-compatible mechanism and qualified
HTTPS/origin/authentication, permissions and media integration. Those pieces do
not exist yet; the working manual LAN hostname is deployment evidence only.

## Library check and decision gate

Primary documentation was checked on 2026-10-22. This was a documentation/source
inventory, not runtime qualification or a security audit.

| Candidate | Finding | Decision |
| --- | --- | --- |
| [Iroh](https://docs.iroh.computer/what-is-iroh) | Embedded Rust networking with authenticated peer identity, direct paths and relay fallback; local address lookup and network-change handling already exist. | Strongest candidate. Trial it before recreating these mechanisms. |
| [iroh-http](https://github.com/Momics/iroh-http/blob/main/README.md) and its [Rust core](https://github.com/momics/iroh-http/blob/main/docs/architecture.md) | HTTP over Iroh exists, but its documented API is pre-v1 and targets several runtime bindings. | Inspect Rust API, dependencies, license and streaming behavior during the trial; do not adopt its complete runtime/FFI stack by default. |
| [Quinn](https://docs.rs/quinn/latest/quinn/struct.Endpoint.html#method.rebind) | QUIC socket rebinding exists; it does not by itself supply the complete application relay/discovery/trust workflow. | Do not assemble a new P2P system from Quinn for this delivery. |
| [Tailscale tsnet](https://tailscale.com/docs/features/tsnet) | Earlier research noted Go/userspace embedding. | Outside the current feature: no Tailscale integration. Existing routed HTTPS endpoints may work through the ordinary OS network stack. |
| [reqwest retry](https://docs.rs/reqwest/latest/reqwest/retry/index.html), [mdns-sd](https://docs.rs/mdns-sd/latest/mdns_sd/), [if-watch](https://docs.rs/if-watch/latest/if_watch/) | Established HTTP, discovery and interface-event building blocks. | Reuse if the HTTPS route implementation is selected. Check against the project's actual reqwest 0.12 API; current documentation may describe a newer release. |
| [Caddy](https://caddyserver.com/docs/caddyfile/directives/reverse_proxy) + [rathole](https://github.com/rathole-org/rathole) | Existing HTTPS frontend and outbound reverse-tunnel mechanisms. | Baseline relay deployment; consume as pinned external tools, not copied or imported app orchestration. |

**Trial budget: 1–2 engineering days**, using an isolated Collector installation,
an actual FLAC/MP3 stream and a disposable native harness. Stop with a recorded
decision and measurements, rather than growing a second implementation project.

Accept Iroh for native use only if the trial demonstrates all of:

1. Existing authenticated HTTP semantics over a bounded streaming adapter, with
   HEAD, Range, seek, cancellation, command receipts and structured errors intact.
   Test the existing HTTP-over-Iroh library before writing an adapter. No new
   catalog/media protocol, mandatory FFI runtime or local open proxy is needed.
2. One peer identity securely bound to the enrolled Collector, and credentials
   never transmitted to a peer learned only through discovery. Network encryption
   does not replace Collector account/collection authorization.
3. Local traffic, forced relay traffic with UDP blocked, network loss/recovery,
   and resumed exact bytes. Use the library's path selection and observation;
   do not layer a second competing LAN/relay state machine over it.
4. Offline LAN use after restart, with retained authorized trust and discovery,
   without an Internet lookup being required to rediscover the known server.
5. Build/dependency/license feasibility for supported macOS Trash Amp and Linux
   amd64/arm64 Collector packaging, acceptable memory and no privileged client
   network installation. Pin the examined releases and record dependency findings.
6. A bounded, restricted production relay configuration can be operated. Public
   Iroh relays are rate limited and intended for development/hobby use, so they
   are not the production capacity plan. See [public relay policy](https://docs.iroh.computer/iroh-services/relays/public).

If any necessary condition fails within the budget, select HTTPS LAN/remote route
selection for the first delivery and record why. The fallback is fully specified
below. The HTTP-over-Iroh integration trial is not a claim that a drop-in library
already meets Collector's recovery and security contract.

Iroh has browser/Wasm support, but its current [browser documentation](https://docs.iroh.computer/languages/wasm-browser)
says browser connections are relayed. Using it would also change the web media
integration. Keep the existing browser player on ordinary HTTPS in this delivery.

## Optional relay topology and deployment example

This is an operator-selectable example for the earlier remote-hosting request.
Automatic private playback must not require users to adopt it; consume the host's
chosen authorized ingress and leave its provider/certificate/tunnel maintenance
to that operator.

```text
Browser / Trash Amp remote HTTPS
       |
       | HTTPS, normal public certificate
       v
VPS Caddy :443 -> loopback :18080 -> rathole server
                                       ^
                                       | encrypted, authenticated tunnel
                                       | initiated from home
                                 rathole client at home
                                       |
                                       v
                             Collector public_bind :9841

Trash Amp local HTTPS -> Collector lan_bind :9843
```

The numeric ports are deployment examples, not protocol constants. Caddy and the
rathole server share a host/network namespace so the forwarded service is bound
to loopback. The home connector must reach Collector's public loopback origin;
use the existing host-network deployment pattern where appropriate. Never route
either public frontend to `private_bind :9842` or private management TLS.

Use a normal `collector.example.com` DNS record pointing to the VPS, **DNS-only**
if Cloudflare hosts DNS. Caddy owns the browser-facing public certificate and
renewal. Do not put media back through an orange-cloud public Cloudflare proxy.
[Cloudflare's Tunnel/media guidance](https://developers.cloudflare.com/cloudflare-one/faq/cloudflare-tunnels-faq/#large-file-and-streaming-traffic-through-tunnel)
is the reason for this route. The VPS terminates HTTPS and is consequently a
trusted processor of credentials and content; this is not end-to-end encryption
through an untrusted VPS. The home tunnel separately encrypts that leg.

Start with rathole's documented [Noise transport](https://github.com/rathole-org/rathole/blob/main/docs/transport.md):
pin the relay public key on the home connector and require a unique service token.
Use the fixed Collector target only. Ship no unauthenticated transport defaults,
open forwarding proxy, shared installer secrets or arbitrary user-selected target.
Restrict the VPS forwarded port to loopback; expose only HTTPS/certificate setup,
the authenticated tunnel port and operator-restricted SSH as needed.

Package `deploy/relay/` with reproducible pinned images/binaries, example secret
files, Caddy/rathole configuration, systemd or Compose startup, restart policy,
bounded logs, health diagnostics, certificate renewal and key/token replacement
instructions. Redact authorization, session cookies and private query material.
Test reboot and connector restart; a healthy tunnel process alone is not an
end-to-end healthy Collector. Forward original paths and required HTTP headers;
do not enable proxy replay of mutations, payload caching or content compression
for validator-bound media ranges. Preserve Collector's authentication-before-body
behavior and configured page/API exposure. Allow proxy configuration of request
bounds without buffering complete uploads or responses on disk.

Serve the web app and APIs together at the new HTTPS origin; this avoids a second
browser media origin and its cookies/CORS issues. Add its exact allowed origin.
Choose the passkey authentication origin deliberately: passkeys and browser local
queues are origin-scoped, and an old hostname's sessions/queues do not silently
move. If replacing the existing public hostname, plan DNS, credentials/passkeys
and operator rollback before cutover. Keep administration access independently
configured. Remote native access uses normal HTTPS enrollment/browser approval.

If the Iroh trial passes, retain this ordinary HTTPS route for browsers and
bootstrap. Embed Iroh in Collector/Trash Amp only for native transport, and run a
restricted `iroh-relay` on the same VPS if capacity permits. That relay has its
own endpoint/port configuration and access limits; it is not interchangeable
with the Caddy/rathole HTTP forwarder. Resolve TLS/port ownership explicitly
before packaging; never let two services independently bind :443. Use the
upstream version's supported setup rather than assuming a reverse proxy supports
its upgrade protocol. Native direct connections can reduce relay bytes; cost the
system conservatively as if all media were relayed. Do not ship operator-wide
Iroh service keys in desktop apps. See [self-hosting guidance](https://docs.iroh.computer/iroh-services/relays/self-hosted).

## Relay costs and region choice

Prices checked 2026-10-22, USD before tax, for a continuously running VM. Domain
registration, optional snapshots and operator time are extra. No managed database,
load balancer, audio storage or transcoding is needed. A 1 vCPU/1 GB VM is a
starting capacity hypothesis to measure, not a qualified concurrency guarantee.

| Option | Monthly VM | Included transfer | Overage |
| --- | ---: | --- | --- |
| DigitalOcean Basic, 1 GB | $6 | 1,000 GiB outbound | $0.01/GiB |
| DigitalOcean Basic, 2 GB | $12 | 2,000 GiB outbound | $0.01/GiB |
| Akamai/Linode Nanode, 1 GB | $5 | 1 TB, as published | $0.005/GB on standard regions; regional exceptions exist |

Sources: [DigitalOcean plans](https://www.digitalocean.com/pricing/droplets),
[DigitalOcean transfer billing](https://docs.digitalocean.com/platform/billing/bandwidth/),
[Akamai pricing](https://www.akamai.com/cloud/pricing). Preserve each provider's
published units rather than equating TB and GiB. Availability and final regional
checkout price must be checked before purchase. Small DigitalOcean allowances
accrue with VM lifetime; a short trial does not receive a whole month's allowance.

For the $6 plan, a conservative full-month model is:

```text
bill = $6 + max(0, measured VPS outbound GiB - 1,000) * $0.01
```

Count both forwarded directions: listening/downloads leave the VPS toward the
listener, while uploads leave it toward the home server. Free inbound traffic does
not make relayed uploads free. Add protocol overhead and retries. A 1 TiB upload
plus 1 TiB download is approximately 2,048 GiB outbound, costing **$16.48/month**
on that plan before overhead/tax. Budget alerts are not hard spending caps; make
large-transfer warnings visible and document an operator shutdown/limit procedure.

Payload-only estimates below use 30 days and decimal codec bitrates. FLAC is
variable; 1 Mb/s is an example, and 1.4112 Mb/s is uncompressed CD PCM reference.

| Usage | 320 kb/s | 1 Mb/s | 1.4112 Mb/s |
| --- | ---: | ---: | ---: |
| One listening hour | 0.134 GiB | 0.419 GiB | 0.591 GiB |
| Five listeners, two hours/day each | 40.2 GiB/month | 125.7 GiB/month | 177.4 GiB/month |
| Ten listeners, four hours/day each | 160.9 GiB/month | 502.9 GiB/month | 709.7 GiB/month |

Ordinary listening by a few people should fit this baseline by these assumptions;
bulk dumps and offline collections are the more likely overage driver. High-rate
audio, bots or leaked credentials can change the outcome. LAN bytes bypass the VPS.
For a shared home uplink, concurrent remote listeners consume the sum of their
bitrates; more relays do not increase that uplink's capacity.

The user reports listeners spread across regions. Start with one $5–6 relay and
measure time-to-first-audio, range latency, sustained upload/download throughput
and loss from Japan plus real remote regions. Compare Singapore, US West and
Europe using short disposable tests before choosing a permanent location. Budget
**$6–12/month initially**; two 1 GB relays are about $12, three about $18, before
overage/tax. Additional regions are a measured follow-up, not initial deployment.
An HTTP relay near a listener still needs a path back to Japan; chaining relays
can add delay and charged transfer. Do not design global anycast or multi-region
account replication for one home origin.

## Reusable crate boundary

`junk-libs-net` is an infrastructure library, with small feature-gated mechanisms.
Use existing rustls/HTTP/Iroh/discovery libraries; do not implement cryptography,
DNS parsing, NAT traversal or a certificate authority. Separate extraction from
consumer feature changes and check Collector plus affected junk-libs/Trash Amp
consumers against the extracted code before advancing pinned Git revisions.

| Shared mechanism | Product-owned responsibility |
| --- | --- |
| Validated origins, per-endpoint HTTPS trust and bounded connection establishment | Collector authorizes endpoint bindings and maps ServerId/account identity |
| Cancellable HTTP streaming and validator-bound range/seek reader | Collector supplies authorized representation/file coordinates and expected length/digest |
| Transport status and typed network/identity/protocol failures | Trash Amp owns status wording, playback/queue policy and user interaction |
| Generic endpoint selection/probes, only if HTTPS fallback is chosen | Collector chooses eligible client surfaces; Keychain and durable receipts stay in its adapters |
| Optional Iroh transport, only if selected by the trial | Collector binds peer identity during enrollment and enforces application authorization |
| Discovery/interface events, using the selected upstream mechanism | Collector interprets service hints; no automatic trust from mDNS or matching subnets |

The library receives opaque target identities, authorized endpoint handles,
request-specific authorization supplied by its caller, byte validators, limits
and cancellation. It does not depend on `collector-core`, account DTOs, SQLite,
Keychain, archive layout, audio formats, UI frameworks or environment-specific
configuration discovery. Let HTTP-library request/response types cross the
mechanism boundary where useful rather than inventing a parallel universal HTTP
model. Keep secrets out of debug/status events. Runtime and background-worker
lifetimes must be explicit; one managed transport per live server/account context
must not strand pooled connections on a dropped per-reader runtime.

Do not move all provider networking, app orchestration or server routing into the
new crate. Only extract mechanisms exercised by this actual delivery. A small
second fixture consumer proves reuse without importing old app orchestration.

## Endpoint authorization and retained state

During enrollment or an authenticated refresh, Collector returns a bounded client
connection descriptor: server identity, eligible client endpoint(s), route kind,
LAN certificate PEM/fingerprint and the matching TLS server name; if Iroh is
chosen, its peer identity and allowed relay configuration. The descriptor comes
only through an already authenticated channel. Never construct it from forwarded
headers, source IPs or unauthenticated discovery. A displayed ServerId is not
cryptographic proof. Service hints contain no tokens or private metadata.

For the first delivery, retain trust atomically per enrolled server/account, with
the existing HTTPS origin preserved. If discovery returns a changed LAN address,
connect only after the enrolled LAN certificate and its server name validate.
Restrict candidate counts, acceptable local addresses, ports, probe concurrency,
redirects and response bytes. DNS rebinding and mDNS aliases must not expand the
credential allowlist. Initial trials should use known configured addresses before
adding mDNS; final delivery includes local discovery for DHCP/network changes.

Only eligible client listeners enter this descriptor. An explicit owner-provided
private-management enrollment must not silently become a remote/public management
route. Network locality grants no additional role. Existing single-origin
credentials remain usable when no descriptor is available; there is no implicit
trust of previously advertised alternates. Convert only the real persisted shape
being changed, with interruption recovery and no credential reset. No permanent
checkpoint compatibility wrappers or full certificate-rotation framework.

Store endpoint trust/configuration in private operational state, never catalog
evidence. Keep health, latency, cooldowns and interface state ephemeral; recheck
after startup. Trust changes require an authenticated update or explicit
re-enrollment. Wrong/expired certificates are excluded and reported; do not silently
accept new keys. A different already trusted route can still remain usable.
Server/account removal cancels probes, streams and downloads and fences queued
callbacks so a late network result cannot revive access or selection.

## Automatic HTTPS switching, if selected by the trial

Keep one connection manager shared by catalog/artwork, stream and download reads
for each live server/account. Prefer a validated LAN route over a remote relay.
Filter by the owner's media-eligibility/cost policy before ranking or retrying;
verified reachability alone cannot enable the unresolved Cloudflare media route.
Remote enrollment/login must not wait for a full LAN timeout. In the background,
probe configured local candidates and mDNS hints with strict limits; use interface
events plus occasional health checks while active. Multicast failure on a guest
Wi-Fi network must not break the known remote route.

Initial tuning targets, to adjust from measurements rather than freeze as API:

- At most eight candidate addresses, two concurrent probes, two-second candidate
  deadlines. Periodic local retry about every 15 seconds while active, backed off
  when idle; immediately re-evaluate on network changes and sleep/wake.
- Require two successful checks spanning about five seconds before moving back
  to recovered LAN. A failed local read/connection can move immediately to an
  already verified, media-eligible remote route allowed by policy. Avoid switching an in-progress valid range just
  because another route's estimated latency changed.
- No more than one alternate-route retry per read operation. Treat network reset,
  temporary connect failure and bounded stalls as transport recovery candidates.
  Honor Retry-After; 401/403/revocation, inconsistent byte validators and invalid
  protocol responses are visible failures, not triggers for credential spraying.
- Target a four-second stalled-read deadline and remote recovery within six
  seconds in a controlled healthy-remote test. Explicit interface loss should
  trigger earlier. These are acceptance targets to measure, not an unconditional
  promise across arbitrary Internet outages.
- Read-ahead may target eight seconds with a 4 MiB ceiling if needed to hide
  ordinary transitions. Measure existing decoder buffering before adding another
  buffer. Cancel promptly; do not turn streaming into permanent offline storage.

The media reader retains its immutable content key, exact position, total length
and strong ETag. On failure, discard the unvalidated partial range and request it
again from an authorized alternate, requiring matching content identity, length,
validator and exact Content-Range before returning bytes. Use committed file
metadata; do not hash entire masters to detect reachability or validate a probe.
Seek, queue order, current account and artwork remain stable. Reuse the existing
decoder/voice where byte recovery permits; show reconnecting if buffering expires.
Do not advertise gapless failover until actual output measurements support it.

Reads may retry under this policy. Mutations must retain their submitted inputs
and command receipt key, and resolve uncertain outcomes through existing receipt
semantics. A transport layer must never blindly replay enrollment, uploads or
playlist edits. Switching routes must preserve the existing user/server cache and
download identity, limits, generation fences and revocation behavior.

## Changes by repository/layer

| Owner | Concrete change |
| --- | --- |
| junk-libs | New independently tested `junk-libs-net`; first move applicable trust/stream/range mechanisms without changing behavior. Add selected transport after the trial. Check affected siblings; pin published Git revisions. |
| Collector application services | Authenticated connection-descriptor policy, client-surface eligibility, endpoint/peer authority and refresh. No networking or device APIs in domain/modules. |
| Collector server/CLI adapters | Typed route/trust configuration replacing guessed route labels; bounded descriptor DTO/API and enrollment integration; local discovery advertisement and configuration diagnostics. Keep public/private router composition explicit. |
| collector-client | Adapt generic transport to typed Collector reads/errors, enrollment, authorization and command receipts. Replace fixed-origin media reading with the shared transport. Migrate real trust state safely. |
| Trash Amp amp-collector | Share transport lifecycle across refresh/download/playback workers; persist authorized trust through Keychain; react to sleep/wake/interface loss and cancel on removal. Preserve snapshots and immutable track identities. |
| Trash Amp UI/playback adapter | Automatic preference, compact Local/Remote/Reconnecting/Offline status, useful trust/access failures, unchanged queue/seek and real playback recovery. No network policy in Slint callbacks. |
| Web | Implement automatic trusted local-server detection/preference and remote fallback from the normal browser entry point. First resolve HTTPS/origin/authentication, local-network permissions and network-change behavior in a bounded feasibility step; preserve media/range, account and access semantics. Split DNS/manual URL success alone is insufficient. |
| Deployment | Pinned relay package, home connector, DNS/cert runbook, restart/rotation/rollback and measured capacity/cost visibility. |

## Delivery sequence and acceptance

These are steps of HL-12's one delivery, not separate layer handoffs or a second
queue. Estimates are engineering time for one person, not calendar commitments.
The independently useful [manual home-listening stopgap](lan-playback-first.md)
is accepted. It does not complete automatic detection in either client. The
estimates below are the earlier native/remote baseline; re-estimate the complete
browser-and-native delivery after the required browser feasibility work rather
than presenting those numbers as covering the added browser requirement.

1. **Select transport and freeze a bounded design (1–2 days).** Run the Iroh trial
   above, record selected versions/license/dependency findings, revise boundaries
   from real inputs, and select exactly one native implementation. Use local
   fixtures before any paid deployment. In the same design phase, run a bounded
   browser feasibility step covering automatic local selection, HTTPS/origin and
   authentication continuity, local-network permission recovery, and the actual
   VPN-connected work Mac. Select a supported mechanism without weakening browser
   trust or managed VPN policy. The native trial cannot waive browser delivery.
2. **Qualify an operator-provided remote listening route.** The earlier 1–2 day
   estimate covered an optional Caddy/rathole deployment; neither that package nor
   a paid region is mandatory. Provision only if specifically authorized. Browser
   and existing Trash Amp enroll/play/seek over normal
   HTTPS without listener VPN software. Prove direct raw port and private/admin
   API denial under configured exposure; exercise restart, TLS and tunnel refusal.
3. **Extract shared mechanisms separately (3–5 days).** Move the currently used
   transport/range code into the narrow crate, update actual consumers together,
   and preserve existing trust, playback and download failure guarantees. Publish
   and advance sibling pins only after their checks. Do not implement hypothetical
   general networking features to fill out the crate.
4. **Complete automatic connections in both clients.** The earlier native-only
   estimate was 5–8 days; browser work needs estimation after feasibility. Descriptor
   authorization/storage, selected transport, local discovery, all read consumers,
   Trash Amp status and playback recovery land together. No isolated backend-only
   endpoint or working-looking UI with a manual-only transport counts as delivery.
   Deliver the selected browser mechanism and real browser media integration too;
   native success cannot close the browser portion.
5. **Qualify operation and failure, then document rollout (3–5 days).** Run the
   matrix below on named hosts and real client output. Record costs and memory/
   latency observations. Update architecture, status, this delivery record and
   the authoritative queue together. Earlier native/remote budget: **13–22 engineering days**
   including trial, extraction and meaningful acceptance; this is not an estimate
   for the complete browser-and-native scope. Unexpected upstream
   integration gaps require reassessment, not hidden scope growth.

Acceptance focuses on guarantees rather than exact incidental probe counts:

- In both the browser and Trash Amp, use the normal entry point/saved connection
  at home and on an away network. Automatically verify/select the local server
  when reachable, retain/recover remote access when unavailable and select local
  again on return, without choosing the LAN URL or reconnecting/re-enrolling to
  change routes. Prove actual local media traffic and real playback/seek in each
  client, with server identity, account, queue and immutable content retained.
  Exercise the work Mac with GlobalProtect connected where LAN access is allowed;
  distinguish browser permission/trust failures from genuine route unavailability.
  The manual LAN hostname's accepted workflow does not pass these cases. This
  requirement alone does not promise gapless mid-track browser handoff.
- Remote browser and Trash Amp installation/enrollment need no VPN client. Full
  real FLAC/MP3 playback, seek, artwork, playlists and download work through relay.
  A known large upload succeeds by ordinary chunks and resumes after interruption
  when curation is explicitly exposed/authorized. Verify actual served assets.
- On a second physical LAN device, stream/seek/download locally with WAN physically
  unavailable, then restart Trash Amp and reconnect locally with retained trust.
  Confirm route bytes and client output, not just a Local label or mocked response.
- During real native playback, remove LAN while remote remains reachable (for
  example Ethernet to another working uplink); verify exact range recovery,
  position/queue continuity and actual decoded/output samples. Separately test
  complete network loss, restored LAN and sleep/wake. Unplugging the only Internet
  link cannot demonstrate remote failover.
- Force relay-only connectivity (including UDP blocked if Iroh is selected),
  repeated flapping and restored local paths. Check bounds and cancellation under
  header/body stalls and mid-range truncation, including a seek during recovery.
- Wrong server/certificate, forged hints, matching private IP on another LAN,
  DNS rebinding/aliases and cross-origin redirects receive no credentials.
  Revoked access/401/403 stop recovery; failure on one account does not revive
  another. Probe/update completion after removal cannot restore the connection.
- Never concatenate changed/truncated/oversized/encoded or ignored-range bytes.
  Download retry/restart retains the same reservation/identity and only complete
  digest-checked files publish. Lost playlist-write responses keep receipt inputs
  and do not duplicate effects after route change/restart.
- Relay/home connector reboot, key/token refusal and certificate renewal produce
  honest diagnostics; existing archive/credentials/queues survive. Measure one
  relay's throughput with actual intended concurrency and a throttled bulk upload
  while listening; choose transfer concurrency/rate limits from the result.
- Run required Collector architecture/format/test/Clippy/web/build checks and
  affected junk-libs/Trash Amp gates at implementation boundaries. Rebuild server
  after embedded assets change; run actual web/native smoke journeys. A prose-only
  plan receives architecture/link checks, not claims of runtime acceptance.

No first-delivery requirement for automatic certificate rotation, multi-region
failover, a hosted multitenant relay business, mobile-native apps, a new
audio pipeline or archive replication. Browser automatic LAN detection remains
required; the feasibility step determines whether cross-origin media is needed.
Manual
trusted endpoints and the original deployment remain available for rollback;
rollback never accepts unexpected trust or resets actual credentials/data.
