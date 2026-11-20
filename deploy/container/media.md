# Direct HTTPS music on a separate hostname

Normal playback uses the website's authenticated connection, including through
proxies and tunnels. This optional setup serves audio from its own directly hosted
HTTPS hostname, for example when your tunnel provider restricts media traffic.
To make clients use it, enable **Require a separate audio connection** in
Administration → Server settings, or set `remote_media_allowed = false` when no
saved override exists. The endpoint can stay configured while normal playback is on.

## How it is isolated

Keep the existing control/login hostname (e.g. behind Cloudflare) and add a
DNS-only hostname for music pointing at your public IP. The `media` container:

- has no IP networking and reads the archive through a read-only mount;
- talks only over two local Unix sockets: one from the Caddy TLS proxy, one to the
  core's restricted media authorization socket, which can validate playback
  capabilities but cannot create credentials, grants or reach administration;
- gets no SQLite, config, TLS keys, Docker socket or host PID namespace.

A compromised media process could read the mounted archive but not change it.
The Caddy frontend does have Internet access (for clients and certificates) and
can reach other destinations unless a host firewall or DMZ restricts it. Exposing
your home IP directly also exposes it to denial of service; there is no upstream
DDoS protection. Keep Docker and the host patched.

The release image contains both executables, so core and media can run the same
image. `docker build --target media` gives a lean media-only image.

## Set up

Stop the server and make a recovery point as for an [update](README.md#update).
Keep your existing hostname, authentication origin and listeners. Make sure WAN
TCP 443 is not already forwarded to another service.

Add to `.env`:

```dotenv
COLLECTOR_MEDIA_HOST=collector-media.example.com
COLLECTOR_CONTROL_ORIGIN=https://collector.example.com
COLLECTOR_MEDIA_BIND_IP=192.0.2.20
COLLECTOR_MEDIA_HTTPS_PORT=9443
# Optional: pin a Caddy image digest instead of caddy:2.
COLLECTOR_MEDIA_PROXY_IMAGE=caddy:2
# Optional lean media image; COLLECTOR_IMAGE is used otherwise.
# COLLECTOR_MEDIA_IMAGE=collector-media:local
```

Check `docker info` for cgroup support. On DSM kernels without CPU CFS quotas, set
`COLLECTOR_MEDIA_CPUS=0` and `COLLECTOR_MEDIA_CPUSET` to one logical CPU (e.g. `3`).
Some DSM kernels ignore `pids_limit`, so the 64-process limit is not enforced there.

Create `media-certificates` and `media-proxy-config` under `COLLECTOR_DATA` (DSM
needs bind sources to exist), then initialize them:

```sh
docker compose -f compose.yaml -f compose.media.yaml run --rm initialize-media
```

Add to `config/collector.toml`:

```toml
# Top level, outside any [section].
remote_media_allowed = false

[remote_media]
base_url = "https://collector-media.example.com/"
socket = "/run/collector-authority/socket"
```

The core generates the service key and socket in the media-authority volume. Never
copy the key into a browser, proxy config or diagnostic report.

Start the stack (add your inbox overlay if used):

```sh
docker compose -f compose.yaml -f compose.media.yaml up -d
```

Create the DNS-only A record and forward **WAN TCP 443 → host TCP 9443** (your
`COLLECTOR_MEDIA_HTTPS_PORT`). Caddy obtains and renews its certificate with
TLS-ALPN on port 443 and proxies only media routes. No port 80 or UDP forward is
needed. Publish IPv6 only after checking its firewall separately. Confirm unknown
hostnames and the bare IP do not expose other services.

Caddy access logging is off and its log filter strips request URIs and credential
headers. Keep capability URLs out of any logging or monitoring you add.

## Test

Sign in through the normal URL and open `/remote-media`. On a phone, turn off Wi-Fi
and run the test; a success on the LAN may just be NAT loopback. With a track
queued, the test also checks byte ranges. Then play that track, seek both ways,
lock the screen, skip tracks and switch between Wi-Fi and cellular.

To test the container isolation (read-only archive, no network) without touching
real data:

```sh
python3 scripts/check_media_container.py YOUR_IMAGE --proxy
```

At startup media refuses to run if the archive or any nested mount is writable.

## Disable

Remove `[remote_media]` and restart the core, remove the WAN forward, and stop/remove
the `media` and `media-https` services and their DNS entry. Login and private
playback keep working.

## Single hostname variant

Without Cloudflare in front, media can share one directly hosted origin: proxy a
fixed path prefix to the media socket and everything else to the core's public
router, set `base_url` to that prefix, strip the prefix in the proxy, and strip
Cookie/Set-Cookie on the media upstream. Media still accepts only scoped playback
capabilities, never login cookies or device credentials. This keeps container
isolation but shares a browser origin with the control UI.

References: [Caddy automatic HTTPS](https://caddyserver.com/docs/automatic-https),
[Docker recursive read-only bind mounts](https://docs.docker.com/engine/storage/bind-mounts/#recursive-mounts).
