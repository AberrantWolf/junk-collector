# LAN candidate selection repair — 2026-10-23

State: source repair accepted; new-image publication/live NAS qualification pending.
Branch: `codex/prioritize-lan-playback-addresses`, based on main after the pipeline
merge. No new release or NAS deployment is part of this source validation.

## Observed failure and recovery

After updating the server, the published Trash Amp beta advanced past its former
missing-route API error but reported no trusted reachable route. The NAS private
TCP 9845 listener was reachable from the development machine. Its diagnostic
certificate contained only `172.17.0.1` through `172.24.0.1`, omitting the NAS LAN
address `192.168.1.106`. Both public and media listeners reported the same ServerId.
Only unauthenticated public identity/certificate metadata was read during this
diagnosis; no credentials were sent to an unverified endpoint or trust installed.

The old adapter sorted private IPs numerically, moved loopback last, then kept
eight: Docker bridges consumed the entire budget before the physical LAN address.
The user explicitly bound the existing private listener to the NAS LAN address
and confirmed Trash Amp playback worked. That restores listening on beta.4 but is
a deployment workaround, not automatic discovery acceptance.

## Repair boundary

The existing server networking adapter now preserves the kernel-selected private
source first, ordinary private interfaces next, known container bridge/veth
interfaces next and loopback last. A UDP connect performs local route selection
without sending a payload. No route is required for the ordinary-interface
fallback. Naming hints only rank candidates, never grant trust or filter private
networks. An ordinary `172.x` LAN and a host LAN bridge remain eligible. Generic
CGNAT/overlay and IPv6 ULA candidates remain; no overlay provider is consulted.
Deduplicate by IP using its best priority before the eight-candidate limit.
Explicit private binds remain authoritative and public binds remain rejected.

One selector feeds initial and periodically refreshed descriptor endpoints and
certificate SANs for both web WebTransport and native HTTPS. Client credentials,
scoped browser grants, pinned certificates/ServerId, media exposure and public
fallback policy are unchanged. No Trash Amp rebuild is needed for this repair.

## Acceptance

Five new regression tests passed, covering:

- More than eight Docker bridge/veth addresses cannot displace the NAS LAN route;
  the descriptor includes it and real rustls certificate-name validation accepts
  it using the advertised leaf pin. A neighboring unadvertised IP is rejected.
- The kernel-selected source survives a full budget of unrecognized interface
  names, with deterministic selection under enumeration reordering.
- Duplicate interface addresses consume only one slot and keep the best priority.
- Offline fallback retains ordinary `172.x` LANs, host bridges and generic CGNAT/
  overlay addresses; IPv6 ULA and loopback obey the bound listener's family.
  Public/link-local addresses and manufactured hints remain excluded.
- Explicit private binds override discovery and public binds remain rejected.

All nine local-playback unit tests passed, including existing actual HTTPS/
WebTransport certificate rollover and obsolete-trust rejection. Full
`cargo test --workspace --locked`, fmt, workspace/all-target Clippy with warnings
denied and server rebuild passed. Architecture/documentation checks and web
check/build passed with unchanged generated web assets.

Actual Chrome 156.0.8078.47 passed the private-player test with both its normal
explicit loopback fixture and the wildcard listener using the development host's
actual OS interfaces. These used the rebuilt production server, service worker,
pinned QUIC transport and real HTML audio: start, forward/backward seeks, control
listener media refusal, wrong-pin rejection, live logout/source release and
unsupported-transport refusal passed. The full web smoke additionally passed
normal playback/queue/list journeys, account/role/WebAuthn and mobile controls.
Chrome could not launch inside the filesystem sandbox; reruns used isolated
local fixture profiles outside it, without adding certificate or local-network
permission exceptions.

Reproduce the wildcard player journey after rebuilding the server:

```sh
COLLECTOR_TEST_LOCAL_PLAYBACK_BIND=0.0.0.0:0 node web/tests/local-playback.mjs
```

## Delivery boundary

This fixes the common server advertisement/SAN generation, so both browser and
Trash Amp consume the corrected routes with their existing protocol and trust
checks. No client rebuild is required. The crowded-interface case is controlled
fixture coverage; the wildcard browser run uses this development host and is not
a new physical work-Mac/NAS deployment claim. Selection remains bounded to eight
candidates, and names are priority hints rather than universal topology detection.
An isolated container network still needs host-reachable addresses/networking.

New-image publication and the user's eventual removal of the explicit NAS bind
require separate deployment acceptance. The working NAS, Tailscale and disabled
public media fallback are left untouched. Keep
`local_playback_bind = "192.168.1.106:9845"` until an updated image is deployed;
then restore the default wildcard bind and verify playback/seeking in both apps.
