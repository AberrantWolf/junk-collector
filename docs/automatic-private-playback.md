# Automatic private playback

Source delivery, 2026-10-22. Web and Trash Amp keep their normal authenticated
control connection and prefer a reachable, cryptographically trusted private
media listener. Public/Cloudflare media fallback defaults to **disabled**. This
feature does not deploy a relay or change public hosting, DNS or Tailscale.

## Listener experience

Sign in to Collector normally, or pair Trash Amp normally. Select Play. A browser
may ask for Local network access; allow it for Collector to enable direct playback.
If access is denied or managed, the player explains how to retry without changing
VPN or browser security policy. Browsing and availability polling do not probe the
private network or prompt for permission. Unsupported WebTransport browsers report
the limitation; they do not silently stream through the public tunnel.

Trash Amp obtains native HTTPS trust automatically through its enrolled control
connection. There is no private certificate import, DNS override, helper, manual
LAN URL, second account or route-change re-enrollment. Existing queues, saved
connections and downloaded/offline files keep their identities. Native streaming
and explicit downloads share the same range reader and route policy.

The personal `https://collector-lan.awkwardexplorer.com/` address remains the
**deployed manual stopgap** until the new server and clients are installed. It is
not detection and does not become the normal endpoint for other users.

## Server setup

The server automatically starts a **media-only** TCP/UDP listener at port **9845**.
It exposes server identity and authenticated representations, never login, library,
curation or administration. Existing management TLS on 9844 is unchanged. Host
networking in the Linux container keeps these listener boundaries explicit.

Allow TCP and UDP 9845 between intended private clients and the server in the host
firewall. No Internet port forwarding is needed. Container hosts using isolated
network namespaces need a network layout that advertises addresses reachable by
clients; software cannot make an unreachable container address reachable.

Relevant top-level configuration:

```toml
local_playback_enabled = true
local_playback_bind = "0.0.0.0:9845"
remote_media_allowed = false
```

`local_playback_enabled = false` disables the additional listeners. An explicit
private IPv4/IPv6 bind is supported; wildcard discovery follows that bind's address
family. At most eight generic private/CGNAT/loopback candidates are advertised.
The authenticated descriptor supplies each candidate's short-lived P-256 leaf
certificate and DER SHA-256 hash. The identity is generated in memory, renewed
every three days with seven-day validity, and refreshed when OS interface addresses
change. Neither public certificates nor the existing enrollment certificate are
replaced. Restart generates a fresh private-media identity; clients refresh trust
through the control connection.

Only explicitly composed direct listeners permit ordinary same-origin media.
Public listener media is refused while `remote_media_allowed` is false, including
when DNS or forwarded headers claim a local path. Enabling that operator policy
would permit public fallback; leave it false while Cloudflare media is unresolved.
Public login, pairing, library metadata and permission checks remain available.

## Trust and transfer policy

The public authenticated control connection is the authority for candidate pins.
A private IP is neither identity nor authorization. Native clients validate the
advertised certificate and enrolled ServerId before sending their credential to a
candidate. Redirects are disabled. Browsers connect using certificate-hash-pinned
WebTransport and send only a short-lived grant for one immutable representation,
scoped to the exact allowed browser origin and original authenticated session.
Every range rechecks session/account and current collection permissions.

A same-origin service worker adapts HTML media HEAD/GET/range requests to a broker
in the owning authenticated document. Logical responses stream through bounded
256 KiB chunks; the app does not create whole-track Blobs or replace its decoder.
Streams, requests, grants and retained open readers are bounded. Cancellation
stops superseded network reads and interrupts derivative validation. HEAD never
hashes media. A prepared derivative is validated once for a retained open handle,
not once per range. Permission checks still run before every range, including
when that handle is reused.

Connection failure refreshes authenticated routes and retries the same immutable
range once. Strong ETags, exact content ranges and lengths prevent changed content
from being spliced into playback. Denial, malformed responses and unavailable
private routes remain failures. The player offers Play to retry after a prolonged
outage. A server IP change may require reloading the browser to obtain a matching
CSP allowlist; certificate renewal alone does not. No connection failure enables
public fallback against the server policy.

## Network and accounting limits

Initial discovery and browser grant renewal require the normal control
connection to be reachable. This is not offline browser bootstrapping; native
retained downloads keep their existing offline behavior.

The feature establishes **trusted private reachability**, not physical same-LAN
membership or an ISP billing guarantee. It races verified candidates for a usable
direct path. An existing OS private route can work without provider integration,
but may traverse WAN links or an independent relay. Same-tailnet membership alone
does not guarantee endpoint reachability, firewall permission or UDP support.
No provider SDK, CLI, API, peer enumeration, configuration or path telemetry is used.

Direct physical LAN playback avoids media leaving through the public tunnel.
Upload budgets, billing counters, physical-path classification, remote relay/Iroh
work and browser support beyond the qualified matrix remain separate work in
[the consolidated queue](remaining-work.md#hl-12--https-relay-lan-switching-and-deferred-certificate-rotation).
See [source acceptance](validation/automatic-private-playback-2026-10-22.md).
