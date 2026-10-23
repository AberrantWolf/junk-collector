# Direct HTTPS media source acceptance — 2026-10-23

Scope: `codex/direct-https-media`, following the user's separate-executable,
read-only archive and no-lateral-network preference. This record distinguishes
source acceptance from activation on the home server. The earlier plan checkpoint
is `e841ffb`; implementation is the subsequent reviewed source checkpoint.
No release, DNS/OpenWrt/NAS configuration, public certificate or installed client
was changed by this work.

The subsequent separately authorized [NAS deployment](direct-https-media-nas-2026-10-23.md)
records actual-host isolation, DNS/forwarding and public certificate issuance.
The source-only results and remaining gates below describe this initial checkpoint.

## Delivered boundaries

- Authenticated control issues bounded, hashed, exact-playable-track browser/native
  capabilities. Browser origin, configured audience and current parent/session/
  device/account/role remain bound. A media service key cannot issue capabilities.
  A grant admits new reads for at most 60 minutes (one minute and one use for the
  synthetic challenge), capped by parent expiry. Admitted reads have a four-hour
  ceiling, a renewable 30-second idle lease and continuous live authorization;
  crashed-process handles are reclaimed rather than monopolizing user slots.
  Admission expiry alone does not end
  an already authorized response.
- `collector-media` imports no Collector app/store/provider orchestration. It opens
  capability-confined relative paths, rejects escape/special-file inputs and
  streams bounded 256 KiB chunks. Full/single/open/suffix ranges, HEAD, immutable
  length/ETag/If-Range and malformed/multipart rejection use ordinary HTTP.
  HEAD/routing/availability/probes perform no full-payload hashing. Existing
  prepared-file validation happens cancellably before GET and is retained on
  metadata-checked bounded handles rather than repeated per seek.
- Live authority is checked before each additional chunk and by a five-second
  monitor. Failures stop further output; already delivered/bounded queued bytes
  cannot be recalled. A stalled transport has a 30-second I/O deadline, 10-second
  header deadline and four-hour total lifetime. There is no stale-auth grace.
  Limits include 64 connections, 32 streams, eight admission operations, 128 file
  handles, 32 core authority requests, 1,024 grants/32 per user, and 64 read handles/
  eight per user. File opens, HTTP headers/bodies/URLs, probes and IPC responses
  have independent bounds.
- The actual Compose media profile is non-root, read-only root/archive, no
  capabilities/no-new-privileges, no host namespaces/core state/TLS/Docker socket,
  and `network_mode: none`. Only two named-volume filesystem sockets connect it
  to the frontend and core's restricted authorization router. Named volumes avoid
  unsupported desktop bind-share socket inodes. A Linux startup guard rejects
  effective writable parent/nested mounts; core's separate archive view remains
  writable. Read-only mounted archive content may be disclosed after compromise.
- The separate Caddy frontend owns HTTPS/certificate state. It permits only fixed
  media routes/methods, strips cookies and filters capability URLs from runtime
  error logs. The official image requires NET_BIND_SERVICE in its capability
  bounding set to execute its file-capability-marked binary on this runtime;
  no-new-privileges remains. It has ordinary networking for clients/ACME: a host
  firewall/DMZ is required to restrict its own NAS/LAN egress. Media's networking
  boundary does not apply to Caddy or protect the shared home uplink from flooding.
- Both clients prefer authorized private media, then the configured direct origin.
  The Cloudflare/control router refuses audio even when direct access fails.
  Browser grants stay in memory; native sends only a scoped media bearer to the
  direct origin with system HTTPS trust, independently of enrolled control trust.
  Same-origin path composition works with cookie stripping; missing Origin is
  allowed only for grants whose frozen browser origin equals the media audience.
  Other supplied origins fail. Sharing a browser origin adds the documented XSS
  tradeoff despite retaining container/filesystem isolation.

## Local acceptance

Host: personal macOS workstation, Docker Desktop Linux/aarch64. Browser:
Google Chrome **156.0.8078.47**. Data, accounts, archive, credentials and listeners
are disposable fixtures. No real archive mutation was attempted.

The real Store/application/socket/media integration covers exact early/late/suffix
bytes, HEAD/ETag/If-Range, multipart rejection, parent expiry/admitted-read
semantics, session and collection revocation, curator demotion, abandoned-handle recovery,
other-user/target/
origin/audience denial, forged handles, duplicate-grant collision and single-use
probes. Core loss fails closed. A subprocess native ranged reader uses distinct
control/media certificates; only its disposable media CA is provided through the
child's root environment. The OS trust store and parent test environment stay
unchanged. A rejected admission is injected once; the old capability remains rejected, so
successful reads require a newly issued same-track grant through real control.
Exact reads, seeks, bounded lease renewal and cancellation pass through the real control,
media and authority services.

`npm --prefix web run test:remote-media` exercises real login/import, the separate
media executable and Unix sockets through a fixture HTTPS frontend. Actual HTML
audio starts and seeks, logout releases the detached media element, control audio
is refused, and the test page verifies a fresh challenge plus real early/late
track bytes. `COLLECTOR_TEST_SINGLE_ORIGIN=1` exercises the same workflow through
a `/media/` prefix with upstream cookie stripping. Both pass, including `COLLECTOR_TEST_MEDIA_REJECT_FIRST_READ=1` fault runs that
keep rejecting the old URL and require a fresh control-issued grant for playback.
Real logout still releases playback; authorization denials do not cause route loops.
Certificate errors
are ignored only within those disposable browser contexts: these checks prove
browser routing/CORS/player behavior, **not public-CA trust or iPhone acceptance**.

The shipped Caddy configuration is independently parsed by Caddy **2.11.7**.
`python3 scripts/check_media_container.py junk-collector:direct-media-test --proxy`
qualifies the shipped media Compose profile with disposable mounts. It checks
create/overwrite/rename/unlink denials, unchanged original digests, absence of
external routes/active interfaces and an outbound connection denial, writable
parent/nested-mount startup refusal, fixed Caddy routes/Host boundary and omission
of a capability from a deliberately triggered upstream-error log. The TLS fixture
trusts only its generated certificate in that Python process; no ACME/WAN is used.
Kernel-provided down tunnel devices are harmless and are not mistaken for active
network reachability. The fixture's missing authority is an intentional failure
case; successful authenticated playback is established by the real-core tests.

Existing browser smoke and private pinned-WebTransport playback/seek/logout checks
pass. Trash Amp's current `amp-collector` tests pass against this shared client in
a disposable copy with local dependency patches, including real-server digital
FLAC/MP3 streaming, seeking, download/offline reopening and connection policy.
Its existing unrelated uncommitted source and published dependency pin were not
modified. The matching shared-client revision must be consumed for an installed
Trash Amp build to gain this feature.

Passed source gates: architecture/document links; `cargo fmt --all -- --check`;
`cargo test --workspace` plus the final expanded remote-media integration;
`cargo clippy --workspace --all-targets -- -D warnings`; generated API contract;
web type check/build; Rust server/media rebuild after embedded assets; browser
smoke and both remote hosting workflows; native/sibling compatibility; default
ARM64 image build, optional lean-media image build/start, and disposable media/Caddy qualification. The release-policy
suite passes six tests; promotion validation passes ten with the existing
rootless-skopeo qualification skipped. The normal release workflow now runs the
media/Caddy qualification against each native exported image before promotion;
this wiring does not count as a completed two-architecture release run.
No skipped physical/public gate is counted
as passed. Earlier failures were repaired before acceptance: stale handle recovery,
same-origin Origin omission, abandoned-handle capacity reclamation, socket-volume
portability, explicit SNI/Host refusal and official Caddy capability requirements.

## Remaining deployment/device acceptance

HL-05/HL-12 in the [single queue](../remaining-work.md) retain:

- Actual NAS mount/kernel/resource qualification and frontend firewall/DMZ policy;
  the shared kernel and Docker daemon remain trusted. Validate nested mounts on
  that installation rather than relying only on the Compose `ro` spelling.
- Dedicated DNS-only hostname, unused OpenWrt WAN TCP 443 mapping, correct public
  DNS/IPv4 and any published IPv6, unknown Host/SNI/default-site behavior, public-CA
  issuance/renewal and disable/rollback. None was activated here. Docker ARM64
  source build is local evidence, not AMD64 release or current preview publication.
- **Actual iPhone Safari over cellular with Wi-Fi off**: fresh challenge/ranges,
  audible playback, forward/back seek, screen lock/next track, long-track admission
  expiry and renewal, pause/resume, core restart and Wi-Fi/cellular changes. The
  page's cellular checkbox is a user declaration; local hairpin success cannot
  establish external reachability. Browser timers/background execution are not
  assumed to keep a grant alive. An admitted response can continue after lease
  expiry; a subsequent request renews through normal control authentication.
- Rebuild/consume the matching client for the real Trash Amp installation, retain
  account/queue/offline identity, and qualify its real device/output/network path.
  A mobile Trash Amp port, relay/VPN integration and audio-only storage projection
  are outside this delivery.

Passing these tests is not a security audit of Collector, Caddy, Docker or DSM.
