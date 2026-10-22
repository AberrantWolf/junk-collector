# HTTPS relay and automatic LAN connections

Planning record, 2026-10-22. Requested outcome; **not implemented or qualified**.
This is a delivery design, not another remaining-work queue. Tracking belongs to
[HL-12](remaining-work.md#hl-12--https-relay-lan-switching-and-deferred-certificate-rotation),
with deployment acceptance under HL-05. Preserve the unrelated active assignment
in that queue. This planning request does not provision infrastructure, change
live DNS/configuration, publish packages or authorize a paid service purchase.

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
listen. They do not install a VPN or join the operator's tailnet. Trash Amp uses
a verified local connection when available, moves to remote access after losing
it, and returns locally without another login. Browsers retain ordinary same-origin
HTTPS. Local DNS can route that same origin locally under the earlier home
milestone; application-managed cross-origin browser switching remains deferred.

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
per-endpoint trust, live transport state and reader integration.

## Library check and decision gate

Primary documentation was checked on 2026-10-22. This was a documentation/source
inventory, not runtime qualification or a security audit.

| Candidate | Finding | Decision |
| --- | --- | --- |
| [Iroh](https://docs.iroh.computer/what-is-iroh) | Embedded Rust networking with authenticated peer identity, direct paths and relay fallback; local address lookup and network-change handling already exist. | Strongest candidate. Trial it before recreating these mechanisms. |
| [iroh-http](https://github.com/Momics/iroh-http/blob/main/README.md) and its [Rust core](https://github.com/momics/iroh-http/blob/main/docs/architecture.md) | HTTP over Iroh exists, but its documented API is pre-v1 and targets several runtime bindings. | Inspect Rust API, dependencies, license and streaming behavior during the trial; do not adopt its complete runtime/FFI stack by default. |
| [Quinn](https://docs.rs/quinn/latest/quinn/struct.Endpoint.html#method.rebind) | QUIC socket rebinding exists; it does not by itself supply the complete application relay/discovery/trust workflow. | Do not assemble a new P2P system from Quinn for this delivery. |
| [Tailscale tsnet](https://tailscale.com/docs/features/tsnet) | Embedding is supported through Go/userspace networking. | Viable, but adds a Go integration and tailnet enrollment model. Prefer the Rust candidate for the trial. |
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

## Relay topology and deployment

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
  already verified remote route. Avoid switching an in-progress valid range just
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
| Web | Retain one HTTPS origin. First qualify local DNS and trusted local proxy under the LAN-first plan; later qualify the relay. No cross-origin media pipeline is required for home routing. |
| Deployment | Pinned relay package, home connector, DNS/cert runbook, restart/rotation/rollback and measured capacity/cost visibility. |

## Delivery sequence and acceptance

These are steps of HL-12's one delivery, not separate layer handoffs or a second
queue. Estimates are engineering time for one person, not calendar commitments.
The independently useful [home-listening milestone](lan-playback-first.md) now
comes first; the estimates below describe the larger remote/failover project.

1. **Select transport and freeze a bounded design (1–2 days).** Run the Iroh trial
   above, record selected versions/license/dependency findings, revise boundaries
   from real inputs, and select exactly one native implementation. Use local
   fixtures before any paid deployment.
2. **Deliver a real remote listening route (1–2 days).** Prepare/run isolated
   Caddy/rathole packaging, then qualify a chosen paid region after provisioning
   authorization. Browser and existing Trash Amp enroll/play/seek over normal
   HTTPS without listener VPN software. Prove direct raw port and private/admin
   API denial under configured exposure; exercise restart, TLS and tunnel refusal.
3. **Extract shared mechanisms separately (3–5 days).** Move the currently used
   transport/range code into the narrow crate, update actual consumers together,
   and preserve existing trust, playback and download failure guarantees. Publish
   and advance sibling pins only after their checks. Do not implement hypothetical
   general networking features to fill out the crate.
4. **Complete native automatic connections across layers (5–8 days).** Descriptor
   authorization/storage, selected transport, local discovery, all read consumers,
   Trash Amp status and playback recovery land together. No isolated backend-only
   endpoint or working-looking UI with a manual-only transport counts as delivery.
5. **Qualify operation and failure, then document rollout (3–5 days).** Run the
   matrix below on named hosts and real client output. Record costs and memory/
   latency observations. Update architecture, status, this delivery record and
   the authoritative queue together. Typical budget: **13–22 engineering days**
   including trial, extraction and meaningful acceptance; unexpected upstream
   integration gaps require reassessment, not hidden scope growth.

Acceptance focuses on guarantees rather than exact incidental probe counts:

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
failover, a hosted multitenant relay business, mobile-native apps, WebRTC, a new
audio pipeline, archive replication or cross-origin browser LAN switching. Manual
trusted endpoints and the original deployment remain available for rollback;
rollback never accepts unexpected trust or resets actual credentials/data.
