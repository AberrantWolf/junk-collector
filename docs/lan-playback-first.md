# Automatic home LAN playback before the relay/Iroh work

Planning record, 2026-10-22. **Not deployed or qualified.** The user can configure
local DNS and wants browser and Trash Amp listening at home while remote transport
is investigated. This selects an earlier, independently useful delivery within
[HL-12](remaining-work.md#hl-12--https-relay-lan-switching-and-deferred-certificate-rotation),
with installation acceptance under HL-04/05. It is not a second work queue.
This request authorizes planning; no live DNS, certificates, services or clients
were changed. Preserve unrelated implementation and staged sibling release work.

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
`Resolve-DnsName collector.awkwardexplorer.com`. The override is not installed yet,
so Cloudflare answers are currently expected and do not prove VPN DNS bypass.
Repeat resolution checks after the local override is installed.

For an initial HTTPS data-path check, use `curl -I --connect-timeout 5 --max-time 10
https://192.168.1.106/` (Windows: `curl.exe`). This unauthenticated check makes no
configuration changes. The NAS currently presents its default Synology certificate,
so an issuer/name validation failure is expected and only establishes that a TLS
endpoint responded; identify it before attributing it to the NAS. A timeout,
corporate block response or connection refusal needs separate diagnosis.
TCP connection success alone is not sufficient: Palo Alto documents that its
endpoint enforcement bypasses TCP handshakes. Once local HTTPS is ready, verify
the actual browser and Trash Amp
route, trusted TLS and real playback; a successful DNS utility query alone does
not establish which resolver a managed browser uses. Keep required VPN settings
in place for these checks. Any managed DNS/LAN exception belongs to the employer's
supported configuration, rather than silently editing hosts files or VPN policy.

Do not count the current personal Mac's Tailscale settings as evidence about the
work machine. The native fallback below only helps where LAN connectivity is
available; Iroh does not confer permission or reachability through a blocked local
route. An allowed remote route can still serve the work machine, but the existing
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
browser cannot install Collector's native certificate pin or perform general
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
This is why split DNS is selected for the browser. If an application-managed
cross-origin path is later requested, first run a bounded Chrome/Safari feasibility
spike and write its authentication contract; it is not part of getting home
listening working now.

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
