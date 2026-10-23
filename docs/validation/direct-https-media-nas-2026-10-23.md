# Direct HTTPS media NAS deployment — 2026-10-23

The user authorized deployment after the [source acceptance](direct-https-media-2026-10-23.md).
The permanent Synology installation now runs the separate media service and a
dedicated DNS-only HTTPS frontend. **Basic iPhone Safari cellular playback is
accepted from the operator's report.**

The subsequent [beta.5 publication and release-image update](container-beta5-publication-2026-10-23.md)
supersedes this initial source-candidate installation; its dated results remain below.

## Installed candidate and recovery

- Application source: `93ebf404e51f33fc8386baef6f04930414cbd51f`, built locally
  for Linux AMD64, transferred over authenticated SSH and loaded on the NAS.
  Local tag: `junk-collector:direct-media-93ebf40`; NAS runtime image ID:
  `sha256:596e0d4d8871553c465e1d91244d543ab7398580504913f1c8a27bf1248c4b69`.
  This is a source candidate, not a new published release or a change to `preview`.
- Caddy 2.11.7, pinned index:
  `sha256:504a44d7d74bd2bc28a66afc4774912fc21223ca8a67734af1c54ab2f999b489`.
- Retained the old exact beta.4 image and a complete, stopped-server read-only
  Btrfs recovery snapshot before applying the candidate. The private deployment
  metadata retains paths, original configuration and stack bytes, image IDs and
  the snapshot cutoff. No full archive copy or payload hash was required.
- Updated the retained Portainer stack definition and only its Collector service
  plus the two new media services. Collector stopped cleanly with exit 0. Observed
  interruption: **5.79 seconds**. ServerId and existing TLS identity are unchanged;
  all **17** other running containers retained their IDs/images/start times,
  including both existing Cloudflare connectors.
- Preserved account state, existing mounts/inbox group, authentication/allowed
  origins, endpoints, exposure policy and explicit working private media bind.
  Added only the remote media configuration, disabled control-origin music,
  and mounted the dedicated authority socket volume into core.

## NAS qualification and resource limits

DSM 7.4.1 / Linux 4.4.302+ / Docker 24.0.2 / Compose 2.20.1 / x86-64:

- The shipped media/Caddy qualification passed against disposable NAS fixtures:
  archive create/overwrite/rename/unlink and authority/root writes denied,
  original fixture digests unchanged, writable parent/nested mounts refused,
  no external routes or active interfaces, and outgoing connections denied.
  Caddy's real Unix upstream, unrelated route/Host refusal and redacted error
  logging passed using a fixture certificate. Real archive mutation was never
  attempted as a test.
- Actual production inspection confirms media's read-only archive/root and
  authority mount, `network_mode: none`, no route entries and CPU affinity `3`.
  It has no private account/config/TLS/Docker mounts. Caddy gets only its own
  certificate/config storage, fixed Caddyfile and the media HTTP socket.
- DSM requires bind source directories to exist before initialization. The two
  dedicated certificate/config directories were created first; the initializer
  sets their UID/GID and modes without recursively changing archive permissions.
- CPU CFS quotas are unavailable. The deployment uses `cpus: 0` plus `cpuset: "3"`
  for both media services; the actual allowed CPU list confirms one logical CPU.
  Memory limits are 256 MiB for media and 128 MiB for Caddy.
- **The kernel does not support Docker's process-count limit.** Docker reports
  that the configured `pids_limit: 64` is discarded. This is a deployment limit,
  not an enforced protection. No host cgroup/kernel configuration was changed.
- Media initially restarted twice while core created its service key/socket,
  then remained running; Caddy started without restarts. Restart policy handles
  this initial dependency race. This is not a tested host-reboot recovery drill.
- The networked Caddy frontend remains trusted infrastructure with ordinary
  bridge networking. A separate DMZ/host egress restriction has **not** been
  installed. Media's no-network boundary does not prevent a compromised Caddy
  from reaching other destinations. Shared kernel/daemon and upstream DoS risks
  remain as described in the [setup boundaries](../../deploy/container/media.md).

## DNS, forwarding and trusted HTTPS

- Operator created the dedicated DNS-only A record pointing at the configured
  static public IPv4. Public and home DNS agree; no AAAA record is published.
  The existing control hostname/tunnel remains unchanged.
- Operator added OpenWrt's IPv4-only TCP WAN 443 forward to the NAS's dedicated
  **19443** listener, with NAT loopback enabled. SSH inspection confirms both
  the saved UCI rule and effective nftables mapping. NAS port 9443 belongs to
  Portainer and is not the forwarding target. No port 80/UDP forward was added.
- Let's Encrypt production TLS-ALPN validation succeeded through the public
  mapping. Caddy obtained the exact media hostname's certificate, expiring
  **2027-01-21 12:08:33 UTC**, with SHA-256 leaf fingerprint
  `b4ae1627db391e70ee9e413dc2d5b16e41d93b03522e075d9e08e186d2d360b4`.
  Automatic renewal is configured; an actual later renewal is not yet observed.
- Normal CA/hostname-validated HTTPS returns identity HTTP 200 with the retained
  ServerId. A home request reaches the public IP through NAT loopback, so that
  request alone is not cellular evidence. The independent successful public CA
  challenge establishes external TCP/TLS reachability at issuance time.
- Production media administration path returns 404, missing grant returns 401,
  mismatched HTTP Host returns 421, and the exact control Origin gets the intended
  CORS response. An explicit Cloudflare edge request still serves control HTTP 200.

## Operator-reported iPhone cellular acceptance

In response to the requested Safari test with Wi-Fi off, signing in through the
existing Cloudflare control URL and running `/remote-media`, the operator reported:

> The test seemed to work, and I can listen to music, scrub, and go next/previous and it claims Direct HTTPS playback

This accepts the basic external listening workflow: audible music, seeking,
next/previous track and the displayed Direct HTTPS route on the actual iPhone.
The setup test was reported as apparently successful; its exact challenge/range
messages were not supplied, so no independent per-stage result is asserted.
Device model and iOS version were not reconfirmed for this run. These are operator
observations, distinct from the independently captured NAS/network checks above.

The operator subsequently reported hearing a track change while the phone was
locked. This accepts continued audible playback across that observed locked-screen
track transition. It does not establish a session duration or exercise grant-expiry
boundaries.

## Remaining acceptance

Extended background sessions, explicit pause/resume, long-track lease
boundaries, core restart and Wi-Fi/cellular transitions remain untested.

Public certificate renewal, deployment disable/rollback and frontend DMZ/egress
policy remain separate unexercised operations. Trash Amp still needs its matching
shared-client build. The [single queue](../remaining-work.md) retains these limits.
