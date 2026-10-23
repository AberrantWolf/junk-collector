# Home LAN playback before the relay/Iroh work

**Source update, 2026-10-22:** [automatic private playback](automatic-private-playback.md)
is now implemented for web and Trash Amp. See [dated acceptance](validation/automatic-private-playback-2026-10-22.md)
and the [sole queue](remaining-work.md#hl-12--https-relay-lan-switching-and-deferred-certificate-rotation)
for deployment/device limits and deferred public relay work. The manual LAN URL
remains the deployed stopgap. The older planning/feasibility statements below
are retained as dated history, not current implementation status.

## Current delivery: explicit LAN hostname — 2026-10-22

**Accepted stopgap, not the real solution.** The intended outcome remains
automatic detection of and connection to the trusted local server in **both the
browser and Trash Amp**, with remote access when LAN is unavailable. It remains
open under [HL-12](remaining-work.md#hl-12--https-relay-lan-switching-and-deferred-certificate-rotation).
Manual URL selection and Trash Amp re-enrollment do not satisfy that outcome.

The [expanded route policy and browser plan](https-relay-and-lan-switching.md#route-policy-trust-reachability-and-transfer-cost)
prioritizes avoiding Cloudflare media, listener simplicity and actual LAN egress
savings. Existing authorized private routes can work generically, with no Tailscale
integration, discovery or configuration work. Private reachability alone is not
proof of a local physical path or avoided ISP upload. The unresolved Cloudflare
media route must not become an automatic fallback when a local probe fails.

The user further clarified that the general private experience should avoid this
manual certificate/DNS/proxy procedure. Public hosts retain their own hosting
choices. The [minimal-setup plan](https-relay-and-lan-switching.md#minimize-private-setup-keep-public-hosting-independent)
evaluates software-managed native trust, pinned WebTransport and WebRTC data
channels; [loopback primitive tests](validation/direct-transports-2026-10-22.md)
have passed without certificate imports in Firefox and Chrome. That is promising
transport evidence, not a replacement already implemented for this stopgap.

The later [actual NAS/LAN test](validation/webtransport-lan-2026-10-22.md) uses the
real player with a disposable Collector: pinned transport, exact ranged bytes,
authorization and revocation pass. The operator now confirms Chrome playback and
forward/backward seeking on the work Mac under the requested GlobalProtect test
conditions, with no errors or unexpected prompts noticed. Saved diagnostics
corroborate ranged seeks and superseded stream cancellation. The personal Mac's
unavailable audio output remains a separate device limit.
This experiment has not changed the accepted stopgap or implemented detection.
The subsequent [public-origin trial](validation/public-origin-webtransport-2026-10-22.md)
also passes Chrome playback/scrubbing, permission deny/allow, trust/authorization
and CSP controls from the normal origin. It used a temporary scoped test path,
now retired to 404 with its fixtures stopped; the real automatic feature remains
unimplemented.

After trusted direct NAS HTTPS succeeded on the work Mac with GlobalProtect
connected, normal hostname resolution still reached Cloudflare. The user chose
manual home access at **https://collector-lan.awkwardexplorer.com/** for browser
and Trash Amp, without installing a Mac helper. This is the current temporary
deployment; automatic home/away selection for both clients remains the real
solution to implement.

The dedicated name has a DNS-only public A record for the reserved NAS address,
`192.168.1.106`, so a VPN resolver can look it up without using home DNS. No WAN
forwarding is added; the private address is useful only where the NAS is reachable.
Home DNS also serves this name explicitly, avoiding upstream private-address
filtering without disabling rebinding protection. The operator confirmed both
clients work on the actual work Mac with GlobalProtect connected; this does not
qualify other managed machines or profiles.

A separate production DNS-01 certificate is installed and assigned to the NAS
proxy, forwarding only to `127.0.0.1:9841`. Collector's exact
`allowed_origins` now includes the new HTTPS origin, with a configuration backup,
successful doctor check and container restart. The existing authentication origin
is retained: use the existing account password at the LAN origin; existing passkeys
remain on the original authentication hostname. No new account is needed.

Trash Amp uses the LAN URL with system HTTPS trust and browser-approved enrollment.
Its current persistence stores one endpoint/credential per server/account, so
reconnecting through this URL updates that saved connection; it does not add an
independent second connection for the same account. Returning to remote access
requires reconnecting at the remote URL. Existing offline data is keyed to the
same server/account, not to a second LAN catalog.

Ordinary DNS-based HTTPS now reaches `192.168.1.106` with normal CA/hostname
validation, the exact issued certificate and expected ServerId. Firefox renders
the login page with Let's Encrypt trust. The operator confirmed work-Mac browser
password login, playback and scrubbing, and confirmed the requested Trash Amp
enrollment/playback/seek workflow works with GlobalProtect connected. This bounded
manual home-listening stopgap is accepted. Unattended certificate renewal remains
outstanding. The earlier same-origin plan and its broader matrix below
remain a dated design record, not claims that this manual route switches
automatically. See [deployment evidence](validation/lan-https-2026-10-22.md).

### Use the LAN address

1. On the home LAN, open **https://collector-lan.awkwardexplorer.com/** and sign in
   with the existing Collector password. Do not use the passkey button on this
   alternate origin; passkeys retain the configured original hostname.
2. In Trash Amp, open **Settings → Servers**, enter that URL, leave the
   owner-provided trust file empty, and choose **Connect/Reconnect**. Approve the
   pairing code in the browser using the same Collector account. This updates
   the saved address for that server/account to the LAN URL.
3. Play and seek in each client. Keep required GlobalProtect settings in place
   on the work machine. Away from home, use the original browser URL and reconnect
   Trash Amp there when remote access is needed; this manual LAN route has no
   automatic fallback.

## Earlier same-origin plan and deployment

Planning record, 2026-10-22. **Live setup started; not yet qualified.** The user can configure
local DNS and wants browser and Trash Amp listening at home while remote transport
is investigated. This selects an earlier, independently useful delivery within
[HL-12](remaining-work.md#hl-12--https-relay-lan-switching-and-deferred-certificate-rotation),
with installation acceptance under HL-04/05. It is not a second work queue.
The initial request authorized planning only. Later explicit assignments authorized
router SSH-key setup, NAS HTTPS and the router reservation/DNS override; see the
[dated partial setup](validation/lan-https-2026-10-22.md). The production certificate
is installed and assigned; direct local HTTPS validates normally against the
expected ServerId. Home DNS now returns the reserved NAS address and suppresses
public IPv6/HTTPS/SVCB answers for Collector. Unattended renewal and trusted playback
acceptance remain pending. Preserve unrelated implementation and staged sibling
release work.

## Selected approach: the same HTTPS origin over local DNS

Override the existing Collector hostname on the home DNS server so it resolves to
the NAS's LAN address. A local HTTPS reverse proxy serves that **exact hostname
and port**, using a publicly trusted certificate, and forwards to Collector's
existing public/client HTTP listener. Public DNS continues to describe the remote
route. The certificate and local DNS must be installed before enabling the override.

```text
At home: browser / Trash Amp -> home DNS -> NAS HTTPS proxy -> public_bind
Away:    browser / Trash Amp -> public DNS -> existing remote ingress
```

For example, `https://collector.example.com/` must remain that origin locally;
changing to `https://collector.example.com:9843/` changes the browser origin.
The default Collector LAN certificate is not automatically browser trusted and
does not solve local HTTPS for the existing public hostname.

Both current clients already benefit: the browser builds relative media URLs and
uses same-origin cookies; Trash Amp's system-HTTPS enrollment retains the enrolled
origin and uses normal certificate/hostname validation. DNS can change the network
destination without changing their URL, ServerId, account, credentials, queue or
content identity. No new player, CORS mechanism, browser enrollment token or
native route manager is needed for this path. Retain any existing pinned-native
connection separately; it cannot silently adopt a proxy's different certificate.

This is automatic **routing when a connection resolves the hostname**, rather
than application discovery. Opening the normal URL at home sends browsing,
artwork and music locally. It does not move an already-open connection, guarantee
an uninterrupted Wi-Fi/cellular handoff, or detect a failed local proxy and select
a remote route. DNS/connection caches may require a reload or reconnect after a
network change. Do not add a Local label based only on a private-looking address
or a forwarded header. Prove the actual route during acceptance.

The away route retains its current limitations; selecting this approach does not
qualify Cloudflare for media or fix remote playback. Home listening must succeed
with that route unavailable.

## Work-machine VPN requirement — added 2026-10-22

The user will frequently listen from a work machine on the home LAN with its
required corporate VPN connected. Local DNS availability on personal devices
does not establish that this machine will use it. Treat work-machine acceptance
as unresolved until tested with its normal VPN and browser policy in place.
The user identified **macOS and Windows with Palo Alto GlobalProtect**. Test
both platforms separately; the managed profiles need not have identical behavior.

Check two independent properties before relying on split DNS:

| Work-machine behavior | Consequence |
| --- | --- |
| Uses the home hostname override and can reach the NAS directly | The same-origin local proxy approach can work without player changes. |
| Uses VPN/public DNS but can reach the NAS directly | The router override alone is insufficient. Evaluate an IT-supported hostname DNS exception; native selection of an explicitly authorized reachable endpoint is another option. |
| Direct NAS access is blocked, intercepted or conflicts with corporate address routes | Local playback cannot be supplied merely by changing DNS or selecting a native endpoint. Use an allowed remote HTTPS route, an IT-provided LAN exception, or a personal listening device. |

Full-tunnel Internet routing alone does not prove LAN is blocked. GlobalProtect
can control DNS and direct local access separately; inspect the actual profile.
Its Windows setting **Resolve All FQDNs Using DNS Servers Assigned by the Tunnel**
can send hostname queries through corporate DNS. **No Direct Access to Local
Network** and endpoint traffic enforcement affect local connectivity on supported
Windows/macOS clients. These are administrator-controlled capabilities, not claims
about the user's profile. See [Palo Alto's Windows DNS documentation](https://knowledgebase.paloaltonetworks.com/KCSArticleDetail?id=kA14u000000HB0rCAG)
and [GlobalProtect local-network enforcement](https://docs.paloaltonetworks.com/globalprotect/administration/globalprotect-portals/endpoint-traffic-policy-enforcement).

Record the GlobalProtect version on each work OS, inspect effective hostname
resolution with the VPN connected, and test connectivity to the known NAS
address/HTTPS port separately. On macOS, use the system resolver via
`dscacheutil -q host -a name collector.awkwardexplorer.com`; on Windows use
`Resolve-DnsName collector.awkwardexplorer.com`. The home override is now installed:
clients using it should receive only `192.168.1.106`, with no public AAAA or HTTPS
address hints. Cloudflare answers on a client require checking effective resolver
selection and caches. The current personal Mac still reaches Cloudflare with
Tailscale DNS active; this is not evidence about the work machine's GlobalProtect.

For an initial HTTPS data-path check, use `curl -I --connect-timeout 5 --max-time 10
https://192.168.1.106/` (Windows: `curl.exe`). This unauthenticated check makes no
configuration changes. A direct-IP request still does not test Collector's hostname
certificate. The operator reported a certificate error from this preliminary test
before Collector HTTPS was installed; it only establishes that a TLS endpoint
responded. A timeout,
corporate block response or connection refusal needs separate diagnosis.
TCP connection success alone is not sufficient: Palo Alto documents that its
endpoint enforcement bypasses TCP handshakes. Local Collector HTTPS is now ready; verify
the actual browser and Trash Amp
route, trusted TLS and real playback; a successful DNS utility query alone does
not establish which resolver a managed browser uses. Keep required VPN settings
in place for these checks. Any managed DNS/LAN exception belongs to the employer's
supported configuration, rather than silently editing hosts files or VPN policy.

Do not count the current personal Mac's Tailscale settings as evidence about the
work machine. The native fallback below only helps where LAN connectivity is
available; Iroh does not confer permission or reachability through a blocked local
route. The operator has now reported successful trusted direct NAS HTTPS with
GlobalProtect connected, while normal resolution still takes the public route.
A supported split-DNS exception could therefore enable this topology. If the user
can administer local hostname configuration, another bounded deployment option is
a Mac helper managing only its own exact hostname mapping: probe the NAS using
the existing hostname with normal TLS validation and verify expected ServerId;
enable the mapping when reachable, remove it when unavailable, preserve unrelated
host entries, and verify browser/native resolver and connection-cache behavior.
This requires an actual implementation, installation and home/away acceptance;
none is installed. A permanent static mapping alone is insufficient for roaming.
An allowed remote route can still serve the work machine, but the existing
Cloudflare media concern remains unresolved. The 0.5–1.5 day estimate below is
conditional on usable client DNS/LAN behavior and excludes corporate IT changes.

## Delivery steps

1. **Inventory the real installation and freeze rollback.** Record the exact
   current URL/port, authentication origin, NAS address/reservation, proxy bindings,
   local resolver and client trust modes. Check the existing NAS proxy before
   adding software; identify who already owns LAN port 443. Save only the affected
   DNS/proxy configuration and use existing recovery policy for any Collector
   update. This deployment should not require an application or image update.
2. **Prepare local trusted HTTPS.** Prefer an existing suitable reverse proxy;
   Caddy is an alternative if needed, not a required extra service. Obtain a
   certificate for the exact existing hostname using automated DNS-01 validation
   and scoped DNS credentials (or an existing qualified renewal mechanism).
   Persist keys/certificate state privately and test renewal. Use a public CA
   certificate trusted by both browser and Trash Amp; a Cloudflare Origin CA
   certificate or the default self-signed LAN identity is insufficient. Test the
   proxy with explicit name-to-IP resolution before changing clients' DNS.
3. **Keep listener authority explicit.** The same-host local proxy forwards to
   `public_bind` (normally loopback `:9841`), never `private_bind` or private TLS.
   Bind/expose the proxy only as intended on the home network, without new WAN
   forwarding. Preserve paths, Origin/Host semantics, streaming/ranges, validators
   and existing page/API exposure. Preserve the authentication origin and exact
   allowed origin; locality grants no additional role. Do not enable management
   merely to make listening work.
4. **Enable the home DNS override.** Reserve a stable NAS address and serve the
   exact hostname locally. Handle both A and AAAA: an unhandled public AAAA must
   not send part of the traffic through the tunnel. Check HTTPS/SVCB records if
   advertised. Configure home clients to use this resolver; test actual browser
   Secure DNS, VPN and iPhone Private Relay behavior rather than assuming the OS
   DNS answer proves the browser route. Use a narrow supported exception where
   necessary. The local record must remain available when upstream DNS/WAN fails.
   Use a short practical TTL for rollout; it cannot force existing connections
   or every client cache to refresh.
5. **Accept the entire listening workflow, then record operation.** Exercise the
   matrix below on the Mac and physical iPhone, document exact device/client
   versions, proxy/certificate renewal ownership, DNS behavior and rollback.
   Remove only the hostname override to restore prior name resolution; retain
   the proxy until cached local answers have expired. Unrelated NAS services and
   remote ingress configuration remain outside this delivery.

DNS-01 validates domain control without requiring a publicly reachable web server;
Caddy supports this with a DNS provider module and credentials. Renewal still
requires Internet access; ordinary playback with a valid certificate must not.
See [Let's Encrypt challenge types](https://letsencrypt.org/docs/challenge-types/)
and [Caddy automatic HTTPS](https://caddyserver.com/docs/automatic-https).

**Estimate:** about 0.5–1.5 engineering days for inventory, configuration and actual
browser/native qualification if the home resolver and proxy are usable. This is
an estimate, not measured acceptance or a calendar commitment. Stop and reassess
if certificate automation, port ownership or resolver bypass prevents this simple
topology; do not turn it into a router replacement project.

## Acceptance for the first delivery

- The existing URL presents a normally trusted certificate on Mac browser,
  physical iPhone Safari and Trash Amp. No certificate-warning bypass or temporary
  test CA substitutes for this acceptance. The proxy reaches the expected
  Collector ServerId and existing account/collection data.
- Existing browser sign-in, refresh and passkeys (where already configured) work
  at the unchanged origin. Existing system-HTTPS native enrollment reconnects
  without a second server/account record; Keychain state and queues survive restart.
- FLAC and MP3 originals play, seek and advance through a queue in both clients;
  prepared tracks, artwork, search and an explicit original download work. Use
  real native output and visible browser media progress, not only HTTP success.
- Capture actual destination/connection evidence on clients or the proxy, with
  secrets redacted. Local media bytes must bypass the public tunnel. A DNS lookup
  or an idle tunnel graph alone is insufficient evidence.
- Disconnect WAN while keeping LAN and local DNS alive. Restart the browser and
  Trash Amp, reload the library, play an uncached track, seek and download. This
  must use valid retained authentication and local storage, with no remote DNS,
  identity lookup or provider request required for listening. Also exercise local
  password sign-in and the existing passkey flow where available. Record any
  certificate-validity/revocation-check behavior observed on the named clients.
- Return from another network and reconnect at home. Check stale DNS, pooled
  connections, IPv6 and browser-specific resolver behavior. A reload/reconnect is
  acceptable for this milestone; an uninterrupted active-stream transition is not
  claimed. Away from home, retain existing remote browsing behavior and honest
  remote-media limitations; no new remote playback acceptance is implied.
- On the actual work machine with its required VPN connected, establish effective
  browser/native DNS and direct NAS reachability separately, then verify local
  destination, normal TLS and real playback/seek. Record the selected supported
  route or a concrete VPN limitation; personal-device success does not pass this.
- Viewer/collection denials and session/device revocation still stop new media
  reads. Disabled curation/admin pages **and APIs** stay absent on this local
  public surface. Do not rerun the entire exposure matrix without a new reason.
- Local proxy failure and certificate errors produce real failures; neither client
  accepts invalid TLS or silently gains management access. Verify renewal and
  rollback without disturbing other NAS services. Record unrun cases explicitly.

Use an isolated proxy/client fixture first, then the narrowly assigned live setup.
The earlier [physical Safari journey](validation/lan-device-preparation-2026-10-20.md)
proves useful local playback behavior but used a temporary CA and did not test
WAN-disconnected operation; it does not close these new acceptance items.

## If DNS routing cannot meet the need

**Next choice: Trash Amp automatic selection before opening reads.** This is a
bounded home-listening delivery, independent of Iroh and remote relay purchase:

- Collector application policy supplies an authenticated descriptor binding the
  same ServerId to explicitly eligible client endpoints and their HTTPS trust.
  Server/config adapters own addresses and TLS details. Do not simply untruncate
  system-HTTPS enrollment or trust current `healthy` flags/route labels.
- Retain endpoint authority and trust atomically with the existing server/account
  enrollment. Bootstrap through an existing trusted connection or explicit owner
  pairing, never from unauthenticated discovery. Existing credentials and caches
  survive actual-state conversion; old single-origin connections remain usable.
- A connection service probes only the small configured candidate set with
  cancellation, bounded time/bytes and no media hashing. Validate the enrolled TLS
  identity and expected server before sending credentials. Do not scan subnets;
  use a stable configured local name/address for this first home deployment.
- Select the validated LAN client for catalog/artwork, newly opened media and
  downloads. Share that selected client/trust with worker-created range readers;
  changing only `api_url` would leave their TLS client wrong. Preserve existing
  fixed-reader range/length/ETag checks and immutable source/cache identities.
- Re-evaluate on reconnect, refresh, new playback, sleep/wake and bounded active
  retry; fence callbacks after removal/account changes. A later reachable LAN
  must recover without re-enrollment. Display Local, Checking LAN or LAN
  unavailable from typed connection state. Probe failures cannot block the UI.
- Retain the queue/position on loss and support explicit retry locally. Existing
  readers may fail when their route disappears; continuous mid-track failover is
  deferred. Distinguish authorized remote browsing from media eligibility: do not
  introduce automatic audio fallback through the unresolved public tunnel.
- Keep HTTP/TLS/probe mechanisms in narrow `junk-libs-net` infrastructure when
  extracted, separately from consumer behavior changes. Collector owns endpoint
  policy/identity and Trash Amp owns Keychain, lifecycle and presentation. Avoid
  a second general route/resume manager before the Iroh trial; do not move the
  whole range reader merely to enable selection at open.

Allow roughly **3–6 engineering days**, including the necessary descriptor/trust,
client/UI integration, affected-sibling checks and real LAN acceptance. Reassess
against actual contracts before implementation. Test forged endpoints, mismatched
certificates, wrong-server responses, redirects, account isolation, cancellation,
revocation and restart without WAN, alongside the real listening workflow. This
would satisfy automatic native selection; it would not complete mDNS discovery or
live LAN/remote failover in the wider plan.

**Separate public-page/browser LAN media selection is the expensive option.** A
browser cannot apply Collector's native certificate pin to ordinary fetch/audio or perform general
Bonjour discovery. A separate LAN origin needs browser-trusted HTTPS, explicit
endpoint authority, cross-origin authentication suitable for media elements,
restricted CORS/Origin handling, HEAD/range/seek behavior, revocation and browser
permission recovery. The current player uses a same-origin authenticated HEAD and
an ordinary audio-element URL; changing the hostname alone is insufficient.
Avoid putting long-lived account tokens in audio URLs or buffering entire albums
into blobs to work around authentication.

Chrome gates public-site requests to local destinations behind Local Network
Access permission; current guidance supersedes its older PNA-preflight proposal.
Safari and other browsers need their own real qualification, including cookie
restrictions and autoplay after recovery. See [Chrome LNA guidance](https://developer.chrome.com/blog/local-network-access).
Split DNS was the earlier browser deployment choice; the VPN bypass and manual
hostname led to the accepted stopgap above. The user's subsequent clarification
requires proper browser automatic LAN detection as part of the real solution.
Run a bounded browser feasibility step and define the HTTPS/origin/authentication
contract before choosing a mechanism. A cross-origin pipeline is one possible
design, not a preselected implementation or grounds to defer the browser outcome.
The browser and native mechanisms may differ; both need real local/remote
selection acceptance. This does not require replacing the existing audio pipeline.

## Relationship to the remote plan and validation

This home milestone precedes the [relay/Iroh design](https-relay-and-lan-switching.md)
and supersedes its earlier assumption that every useful LAN improvement must wait
for the Iroh trial. The trial still precedes building a general native route/resume
manager. DNS-based home routing remains useful whichever native transport wins.
General mDNS discovery, seamless failover, certificate identity rotation, a VPS and
cross-origin browser playback are not dependencies of this milestone.

Update architecture, implementation status, HL-12 and dated deployment evidence
together when actual work lands. This prose-only plan receives architecture,
documentation-link/fixture and whitespace checks; no runtime qualification is
claimed. If fallback implementation changes code, apply the repository checks and
affected sibling gates, rebuild embedded assets/server where changed, and exercise
actual web/native journeys. Do not repeat unrelated release or hardware matrices.
