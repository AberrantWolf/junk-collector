# Private playback

By default, audio plays over the same authenticated website connection, including
through proxies and tunnels. Private-first playback is optional: enable **Require a
separate audio connection** in **Administration → Server settings**, or set
`remote_media_allowed = false` when no saved override exists. Collector does not
detect connection providers or enforce their terms. This feature does not deploy a
relay or change public hosting, DNS or Tailscale.

## For listeners

Sign in normally, or pair Trash Amp normally, and press Play. A browser may ask for
local network access; allow it to enable direct playback. If access is denied, the
player explains how to retry. Browsing never probes the private network. Browsers
without WebTransport report the limitation instead of silently streaming through
the public connection.

Trash Amp gets its HTTPS trust automatically through its enrolled connection: no
certificate import, DNS override, manual LAN URL or re-enrollment. Queues, saved
connections and offline files are unaffected. Manually configured HTTPS endpoints
still work.

## Server setup

The server starts a **media-only** TCP/UDP listener on port **9845**. It serves only
server identity and authorized audio — never login, library, curation or
administration. Management TLS on 9844 is unchanged.

Allow TCP and UDP 9845 from your private clients in the host firewall; no Internet
port forwarding is needed. In containers with isolated network namespaces, the
advertised addresses must be reachable by clients (the shipped container uses host
networking).

```toml
local_playback_enabled = true
local_playback_bind = "0.0.0.0:9845"
remote_media_allowed = false
```

`local_playback_enabled = false` disables the extra listener. An explicit private
IPv4/IPv6 bind overrides address discovery. Otherwise up to eight private, CGNAT or
loopback addresses are advertised, preferring the kernel's default private source,
then ordinary interfaces, then container bridges, then loopback. Discovery works
without a default route.

Each advertised address gets a short-lived in-memory P-256 certificate (renewed every
three days, valid for seven, regenerated on restart and when addresses change).
Clients learn the certificate hashes through the authenticated control connection.

While `remote_media_allowed` is false, audio requests on the public listener are
refused, whatever DNS or forwarded headers claim. Login, pairing, library metadata
and permission checks stay on the public listener.

## Trust and transfer

- The authenticated control connection is the only source of candidate addresses
  and certificate pins. A private IP is neither identity nor authorization.
- Native clients verify the certificate and enrolled server ID before sending
  credentials. Redirects are disabled.
- Browsers use certificate-hash-pinned WebTransport and send a short-lived grant for
  one immutable representation, bound to the page origin and session. Every range
  request rechecks the session and collection permissions.
- A same-origin service worker turns media requests into 256 KiB streamed chunks;
  no whole-track blobs. Playback never hashes whole files; integrity checks are
  separate.
- On connection failure the client refreshes routes and retries the same range once.
  Strong ETags and exact ranges prevent mixing changed content. Failures never fall
  back to the public route against the server policy. If the server's IP changes, a
  browser reload may be needed.

## Limits

Discovery and browser grant renewal need the control connection, so browsers cannot
start playback fully offline (native downloads still work offline). The feature
gives trusted private reachability, not proof of a physical LAN or zero WAN cost: an
OS route to a private address may cross a VPN or relay. No VPN provider integration
is used.
