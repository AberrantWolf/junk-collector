# Isolated direct HTTPS music

Source delivery instructions; do not assume an older published image contains
`collector-media`. Build this source or select a release explicitly documenting
the feature. No DNS/router changes are made by these files.

Keep the normal Collector control/login hostname on Cloudflare. Use a separate
DNS-only hostname for music, pointing at the home static public IP. The media
container has **no IP networking**, reads the archive through a read-only mount,
and uses only two local HTTP Unix sockets: one from the TLS proxy, and one to the
core's restricted media authorization adapter. It cannot use that adapter to
create user credentials/grants or access administration.

Read-only means a compromised media process can disclose mounted data, but cannot
modify it through that mount. Core still publishes normally through its separate
writable view. Containers share the host kernel; keep Docker/DSM patched. The TLS
frontend has Internet networking for clients and certificate issuance, and remains
a separately maintained exposed component. Its ordinary bridge networking can
reach other destinations unless the host/DMZ firewall restricts it; media's lack
of networking does not confer that boundary on Caddy. A host firewall or separate
DMZ is the conventional way to restrict the frontend on the target installation.
Direct access also exposes the home
IP/link to denial of service. This setup does not provide upstream DDoS protection.

## Prepare

Use the existing stopped-server recovery/update procedure. Keep the current
Cloudflare hostname, authentication origin, endpoint list, existing private
listeners and working LAN bind. Do not re-enroll users or enable control-origin
media fallback. Confirm WAN TCP 443 is available before assigning it; do not
replace another service's existing forwarding.

The default release Dockerfile contains both executables so the exact same image
digest can run separate core/media containers. The media executable has no
application/store/providers dependency. An optional `--target media` build supplies
a lean image containing only media; it is not a separately published tag unless
a release explicitly says so. Using the shared release image does not give the
media container core's mounts, networks, credentials or entrypoint.

Set these additional values alongside the existing `.env`:

```dotenv
COLLECTOR_MEDIA_HOST=collector-media.example.com
COLLECTOR_CONTROL_ORIGIN=https://collector.example.com
COLLECTOR_MEDIA_BIND_IP=192.168.1.106
COLLECTOR_MEDIA_HTTPS_PORT=9443
# Optional: pin your reviewed Caddy image digest rather than the default caddy:2.
COLLECTOR_MEDIA_PROXY_IMAGE=caddy:2
# Optional separate source-built lean image; otherwise COLLECTOR_IMAGE is reused.
# COLLECTOR_MEDIA_IMAGE=collector-media:local
```

Check `docker info` for resource-controller support. On DSM kernels without CPU
CFS quotas, use `COLLECTOR_MEDIA_CPUS=0` and set `COLLECTOR_MEDIA_CPUSET` to one
available logical CPU, for example `3` on a four-thread NAS. This uses Docker CPU
affinity to retain a one-CPU bound. Check the actual container's allowed CPU list.
Some DSM kernels also discard `pids_limit`; do not report the configured 64-process
limit as enforced on those hosts. Retain the memory limits and record this host
limitation rather than changing host cgroup configuration during deployment.

Initialize only the dedicated runtime/certificate directories with the new image:

Create the `media-certificates` and `media-proxy-config` directories beneath
`COLLECTOR_DATA` first. DSM requires bind-mount source directories to exist before
starting the initializer; it will then set their ownership and permissions.

```sh
docker compose -f compose.yaml -f compose.media.yaml run --rm initialize-media
```

This does not recursively change archive permissions. The media container uses
UID/GID 10001 to read the existing private archive permissions; its protection
comes from the read-only mount, separate PID/network/mount namespaces and dropped
privileges, not from pretending a different numeric UID alone supplies isolation.
Only dedicated Docker named volumes share the two socket directories. This keeps
socket inodes on the Linux daemon filesystem, including on Docker Desktop; they
are not bind-mounted through a desktop file-sharing filesystem. Private SQLite/config/TLS keys,
Docker socket, host PID namespace and host filesystem are absent from media.

Add to `config/collector.toml`:

```toml
# Keep this top-level setting outside all [sections].
remote_media_allowed = false

[remote_media]
base_url = "https://collector-media.example.com/"
socket = "/run/collector-authority/socket"
```

The media-authority named volume contains the automatically generated service-key
and socket.
Core owns it (0750); the key is 0640 and socket 0660. The media mount is read-only.
The key can validate presented playback capabilities, never issue them. Do not
copy it into a browser, public proxy configuration or diagnostic report.

Start the stack with its overlay (include your existing inbox overlay if used):

```sh
docker compose -f compose.yaml -f compose.media.yaml up -d
```

Create the DNS-only A record and forward **WAN TCP 443 → NAS TCP 9443** (or the
selected dedicated proxy port). The proxy listens on container port 8443, requests
and renews its public certificate through ACME TLS-ALPN on public 443 (HTTP-01 is
disabled), and forwards
only the media routes to the HTTP socket. It does not proxy to DSM/general core.
The proxy advertises HTTP/1.1 and HTTP/2 only; it does not advertise an unusable
HTTP/3/UDP path through this TCP-only forward. No UDP or port 80 forward is
required by this configuration. Publish IPv6 only
after its independent firewall/listener checks pass. Confirm unknown hostnames,
bare-IP/default sites and unrelated routes cannot expose other NAS services.

Caddy's access logging is off; its configured runtime log filter removes request
URIs and credential headers. Keep capability URLs out of any added proxy logging,
monitoring, analytics or error reporting. Use sanitized status/identity diagnostics
instead. Certificate storage is writable only in the proxy's dedicated volumes. The proxy
retains only `NET_BIND_SERVICE` in its capability bounding set because the official
Caddy binary carries that file capability; dropping it prevents execution on the
qualified runtime, even on port 8443. `no-new-privileges` remains enabled. Media
continues dropping every capability.

## Test and disable

Sign in through the normal Cloudflare URL and open `/remote-media`. On the target
**iPhone Safari**, disable Wi-Fi, check the user-confirmed cellular box, and run
the test. It checks a fresh one-use synthetic challenge. With a real track queued,
it also verifies early/late ranges against bounded source digests obtained through
control authentication. The page never changes routing or treats a client report
as authority. A local success may be NAT loopback and does not prove external access.

Return to the library, play that track and seek in both directions. Check audible
output, screen lock, next track, pause/resume, lease expiry and Wi-Fi/cellular
changes. Record the actual iPhone/iOS/Safari versions and limits. Desktop/emulated
browser checks do not qualify iPhone background behavior. Keep music refused on
the Cloudflare control listener even if the direct route fails.

For isolation acceptance, run the disposable container qualification rather than
attempting writes against the real archive:

```sh
python3 scripts/check_media_container.py YOUR_SOURCE_BUILT_IMAGE --proxy
```

The production profile passes `--require-read-only-archive`: startup checks Linux
mount flags for the archive and every visible nested mount, and refuses a writable
view (including nested mounts left writable by an older kernel). This does not
replace actual target-host qualification. Verify its read-only mount rejects
create/overwrite/rename/unlink, including nested mounts, and media has no external
network interfaces/routes. Killing core must stop new reads and ongoing reads at
the next authorization boundary. Ordinary Cloudflare frontend failure differs:
an admitted stream can continue if core and its authorization socket remain alive.

To disable, remove the `[remote_media]` configuration and restart core, remove the
dedicated WAN forwarding, and stop/remove the media and media-https services.
Remove only their dedicated DNS/proxy entries as needed. Existing control login
and private playback remain. Retain certificate/recovery files as operator policy;
never delete archive/state or broaden mounts as a troubleshooting workaround.

## One hostname for other operators

The separate media process also works behind a single directly hosted HTTPS
origin: proxy a fixed media base path to its HTTP socket and the control paths to
core's **public/client** router. Configure `base_url` with that prefix, strip it
in the proxy, and strip Cookie/Set-Cookie on the media upstream. Media still
accepts only scoped capabilities; long-lived native credentials and login cookies
are never its authority. Its allowed browser origin must match that deployment.

This preserves container/filesystem isolation, but shares a browser origin and
directly exposes control under that operator's policy. DNS cannot send `/media`
directly home while keeping other paths of the same proxied hostname on Cloudflare.
The selected two-hostname home setup retains that routing and browser-origin
separation. Never send this deployment's capability-bearing media requests through
the excluded Cloudflare media route.

The relevant upstream contracts are [Caddy automatic HTTPS](https://caddyserver.com/docs/automatic-https),
[Caddy server options](https://caddyserver.com/docs/caddyfile/options#servers), and
[Docker recursive read-only bind mounts](https://docs.docker.com/engine/storage/bind-mounts/#recursive-mounts).
The startup mount guard and actual target qualification remain necessary on older
kernels; configuration syntax alone does not prove nested mounts are protected.
