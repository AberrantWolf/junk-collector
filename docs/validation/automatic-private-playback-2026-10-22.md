# Automatic private playback — 2026-10-22

Production source delivery for Junk Collector web and Trash Amp. This record is
separate from the earlier Python feasibility bridge and manual DNS/certificate
stopgap. No production NAS deployment, public route change or Tailscale operation
is performed by this delivery.

## Production acceptance

- `web/tests/local-playback.mjs` starts the actual compiled server with public
  media disabled, creates a real account/collection and a synthetic 45-second
  stereo PCM WAV, publishes it through real uploads, restores the normal personal
  queue and uses the actual production app/player/worker.
- Chrome **155.0.8059.85**, headless in a fresh temporary profile: pinned QUIC
  connects automatically from the normal app, the HTML audio clock advances,
  forward seek to 30 seconds and backward seek to 8 seconds pass. The test uses
  secure-context localhost HTTP for control, with **no certificate-error bypass or
  extra browser security flags**. This is same-computer integration, not a new
  cross-device or public-origin qualification of this build.
- An independent HTTP client receives 503 for media on the control listener.
  The worker owns the real media requests. A wrong browser pin rejects. When
  WebTransport is unavailable, the player leaves its source unset, displays an
  actionable error and emits no ordinary HTTP media request. Server-side logout during actual
  WebTransport playback also releases the detached audio element and clears its
  source; the listener can sign in again with the saved queue intact.
- Real production HTTPS/QUIC integration covers exact ranged bytes, seeks,
  cancellation, wrong certificate pins, unapproved CONNECT origin, wrong grant,
  wrong target, expired grant, oversized range, browser logout and collection
  grant removal. Native reads admitted before credential revocation fail on the
  next uncached range; new reads also reject the revoked credential.
- Rollover uses the actual production identity installation function and both
  listeners: old TLS trust and old QUIC pin reject new connections; refreshed
  trust succeeds at the same addresses. The media listener returns 404 for admin
  routes. No OS certificate import or persistent private-media key is involved.
- Instrumented real readers establish that HEAD performs no payload reads,
  prepared audio is hashed once for the retained open handle, subsequent ranges
  read only their bounded bytes, and oversized ranges do not read. Cancelled,
  changed and truncated prepared-media validation rejects.
- Trash Amp's actual Collector adapter integration disables all media on its
  enrolled control URL, starts a distinct private listener, and runs its existing
  real FLAC decoding/seek, explicit download, authorization/revocation, retained
  cache and offline playback journeys. The saved server/account identity remains
  unchanged. Temporary command-line dependency patches are only a development
  check; the delivery uses a pinned Git dependency, not checked-in local paths.

## Regression checks

The full Collector workspace tests, architecture/docs/fixture checks, strict
Clippy, Rust formatting, generated OpenAPI/TypeScript contract checks, TypeScript
check, web build and compiled-server rebuild are required. The complete browser
smoke test passes actual FLAC playback, pause/resume, seek, queue progression,
lists, source detachment and grant/session revocation, along with management,
WebAuthn, imports and mobile layout. This legacy HTTP-media fixture explicitly
disables the additional transport; the dedicated production test above covers it.
Affected server tests are rerun after subsequent transport refinements. Trash Amp
passes its full workspace tests, formatting and strict all-target Clippy against
the resolvable Collector Git pin `14d56ab4cceaf82c09086d56d987484bab46d618`,
without command-line patches or local path dependencies.

## Deliberate limits

The earlier [public-origin work-Mac test](public-origin-webtransport-2026-10-22.md)
qualifies browser permission deny/allow and CSP enforcement for the feasibility
transport, with user-confirmed audible playback/seeking under GlobalProtect. It
is not counted as testing this new Rust build on that machine. The new build still
needs NAS/work-Mac installation qualification, managed permission/UDP denial and
actual network-change recovery. Firefox/Safari playback and other managed VPN
profiles remain explicitly unqualified. Headless clock advancement is not an
independent audible-output or all-format oracle.

Private reachability is not proof of physical LAN membership, direct routing
inside a third-party overlay, zero WAN traffic or ISP billing. No upload-budget
accounting or public relay/Iroh solution is implemented here. Extended outages
fail closed and offer Play to retry. A changed server IP can require browser
reload for its CSP allowlist; automatic certificate renewal does not. Production
continues using the accepted manual LAN hostname until updated builds are installed.
