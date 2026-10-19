# Container installation and updates

Run Junk Collector on a Linux Docker host or a compatible NAS with Docker Compose
v2 or later. The recommended browser entry point is an HTTPS hostname managed by
an existing reverse proxy or Cloudflare Tunnel. Direct private HTTPS is an
alternative. The server runs as UID/GID 10001 with a read-only root filesystem;
configuration, accounts, uploads, archive files, and prepared audio persist outside
the image. FLAC and Opus encoders are included.

Builds remain experimental alpha software. Backed-up personal use of supported
features is the intended starting point. Storage formats may change; read the
selected build's notes before updating. Full Internet-ingress, active-work failure,
and power-loss qualification remain incomplete. The dated validation records
state which installation paths have actually been exercised.

## 1. Select an image and storage

Use an explicit image version or digest supplied by a release. Registry images
have not yet been published; for the current source checkout, build from the
repository root:

```sh
docker build -t junk-collector:local .
cd deploy/container
```

If building for a different machine, select its platform with `--platform
linux/amd64` or `--platform linux/arm64`, save with `docker save -o
junk-collector-image.tar junk-collector:local`, transfer that archive to the Docker
host, then load it with `docker load -i junk-collector-image.tar`. The DS1522+ uses
`linux/amd64`. Published multiarch images remove the need to build or transfer
images manually. Use a distinct local tag for subsequent builds you want to retain.

Extract the release installation bundle on the server, or use this directory from
a source checkout. Run the remaining commands beside `compose.yaml`; use `sudo`
for Docker commands if required by the host.

```sh
cp .env.example .env
```

Set these values in `.env`:

| Setting | Value |
|---|---|
| `COLLECTOR_IMAGE` | Selected image tag/digest, or `junk-collector:local` for the build above. |
| `COLLECTOR_DATA` | New permanent absolute data directory, e.g. `/srv/junk-collector` or `/volume1/junk-collector` on Synology. |
| `COLLECTOR_HOST`, `COLLECTOR_PRIVATE_IP` | Leave their loopback defaults for the proxy setup; see the direct HTTPS alternative below. |

Keep the data directory outside the source checkout and installation bundle.
SQLite/private state must live on the Docker host's own filesystem, not an SMB/NFS
client mount. Allow room for retained originals, prepared audio, resumable uploads,
and complete backups. Important backups also belong on independent storage.

The Compose package uses Linux host networking to retain separate loopback HTTP
listeners. Docker Desktop host-networking behavior is not this deployment target.

## 2. Configure HTTPS access

### Recommended: existing reverse proxy or Cloudflare Tunnel

Create a working configuration file:

```sh
cp collector.proxy.toml.example collector.toml
```

Edit `collector.toml`:

1. Replace every `https://collector.example.com` with the exact HTTPS URL browsers
   will use. Include a non-default port if applicable; use a hostname at the URL
   root, not an application subpath. Keep this origin stable for passkeys.
2. Choose the management exposure. For a personal installation administered
   through this same hostname, explicitly set `public_administration = true`.
   This also enables imports/curation; account and collection permissions still
   apply. For a browse/listen-only hostname, leave both exposure flags false and
   provide a separate private management route to create/manage collections.
3. Optionally uncomment `[providers]` and provide a real MusicBrainz contact.
   Metadata acquisition is otherwise unavailable; preservation still works.

Configure the proxy/tunnel route:

| Browser address | Upstream | Surface |
|---|---|---|
| `https://collector.example.com` | `http://127.0.0.1:9841` | Public router, with the exposure selected above. |
| Separate private management origin, if used | `http://127.0.0.1:9842` | Full management, reachable only through private ingress. |

The upstream addresses are relative to the proxy/connector's network namespace.
A host process or Linux host-network container on the same machine can reach them.
A bridge-network container's `localhost` means that container; neither its ordinary
service name nor the NAS's LAN address reaches Collector's loopback HTTP listener.
Use a host-local/host-network proxy or connector for this package. Do not point
an existing bridge connector at its own localhost, widen the HTTP binds, or route
an Internet hostname to port 9842. Changing a shared connector's network can also
break its other Docker service-name routes; choose its topology deliberately.

An existing host-level reverse proxy, including a NAS's built-in proxy, can also
provide the local connection to `127.0.0.1:9841`. A bridge-network tunnel connector
can reach that proxy's private service address instead. If that hop uses HTTPS,
verify its certificate. Use the local origin address, not a public hostname that
routes back into the same tunnel.

For Cloudflare Tunnel, add a published application hostname with service type
**HTTP** and URL `127.0.0.1:9841` in the host-local connector. Cloudflare provides
browser-facing HTTPS; no Collector certificate import is needed for this HTTP
upstream. See [Cloudflare's routing documentation](https://developers.cloudflare.com/tunnel/concepts/routing/).
An Access policy can restrict initial browser use to selected users; Collector's
own login and collection permissions remain required.

Preserve browser Origin, cookies, request bodies, Range headers and response cache
controls. Bypass shared caching for the Collector hostname; do not add a Cache
Everything rule. Proxy/tunnel upload limits and timeouts still apply to individual
requests. Original package uploads are chunked, while other uploads can have their
own limits. Check the provider's [request-size limits](https://developers.cloudflare.com/support/troubleshooting/http-status-codes/4xx-client-error/error-413/)
and [cache behavior](https://developers.cloudflare.com/cache/concepts/cache-control/)
when configuring it. Forwarded headers do not grant private-router access.

### Alternative: direct private HTTPS

Skip the proxy configuration file. Before initialization set `COLLECTOR_HOST` to
a stable private DNS name or IP and `COLLECTOR_PRIVATE_IP` to the host's explicit
private IPv4 address. Reserve that address in DHCP. The initializer generates
management at `https://COLLECTOR_HOST:9844` and browsing/listening at
`https://COLLECTOR_HOST:9843`; management remains disabled on the latter.

Names in config do not create DNS or mDNS records. Setup generates a certificate
for these names, which browsers will not automatically trust. Obtain the public
certificate after startup through a trusted administrator session:

```sh
docker compose exec -T collector cat /data/state/tls/cert.pem > collector-cert.pem
docker compose logs collector
```

Compare `sha256sum collector-cert.pem` (or `shasum -a 256` on macOS) with the PEM
fingerprint in the logs, then install only this public certificate in the trust
store used by the browser/client. Never transfer `key.pem` or bypass certificate
validation. Direct certificate renewal/trust changes are operator-managed.

## 3. Initialize and start

For a published image, pull it first; skip pulling for an image built/loaded locally:

```sh
docker compose pull collector cli initialize
```

Prepare private directories and a starting configuration:

```sh
docker compose run --rm initialize
```

**For the proxy setup**, install the edited working config before account setup:

```sh
docker compose run --rm --entrypoint sh cli \
  -c 'cat > /data/config/collector.toml' < collector.toml
```

This copy is a fresh-install step. Do not repeat it over an existing installation
when updating; edit the actual retained configuration deliberately while stopped.
For direct HTTPS, keep the initializer's generated config and skip this copy.

Create the owner account, check configuration, and start:

```sh
docker compose run --rm cli setup
docker compose run --rm cli doctor
docker compose up -d collector
docker compose logs --tail 40 collector
```

Setup prompts for a password of at least 12 characters without echoing it. Open
the configured HTTPS origin and sign in as `owner`. If collection creation is
absent, check that the origin reaches a management-enabled router/exposure.

Initialization never overwrites existing config or recursively changes archive
ownership. Setup preserves existing accounts and TLS and must run while stopped.
Ordinary `up -d collector` never creates or resets credentials. Changes to `.env`
host/IP values do not rewrite retained config or certificates; the actual config
is `COLLECTOR_DATA/config/collector.toml`. The archive and complete private state
are in its sibling `archive` and `state` directories.

## 4. Check before bulk intake

Create a collection, import one complete supported package, download its originals,
and compare them with the sources. Restart the service and confirm login and item
access. For music preparation, prepare a supported source and check real playback
and seeking. Complete a backup/restore drill before treating the installation as
an operational catalog. Keep source originals and an independent backup.

These checks establish the workflows exercised through the chosen ingress, not
complete Internet security or power-loss certification. If an operation is
unsupported or a provider/tool is unavailable, keep its failure visible rather
than assuming identification, verification or playback succeeded.

## Update

Read the target build's storage/upgrade notes. Use an explicit version/digest or
a distinct local build tag, not automatic latest-tag updates. Save `.env` and
record the current image before changing it:

```sh
cp .env .env.before-update
docker compose images
docker compose stop collector
docker compose run --rm cli backup --destination /data/backups/before-update-UNIQUE
docker compose run --rm cli validate-backup --backup /data/backups/before-update-UNIQUE
```

Choose a fresh backup name. Stop if any command fails. Backups contain credentials
and TLS keys; copy important backups to independent storage.

Change `COLLECTOR_IMAGE` in `.env` to the selected build. Pull a published image,
or build/load a new local image using that tag; then:

```sh
# Published images only:
docker compose pull collector cli initialize

docker compose run --rm cli doctor
docker compose up -d collector
docker compose logs --tail 40 collector
```

Check fresh login, collection browsing, original download and playback if used.
`doctor` checks configuration/tools, not upgrade acceptance. Updates retain
configuration, credentials and archive/state mounts; do not rerun setup or replace
config from the example. If new code has opened/migrated data, changing back only
the image is not a safe rollback. Follow the [full restore procedure](recovery.md)
into empty roots using the matching earlier image, archive, state and config.
Never run original/restored server identities together.

## Stop or uninstall

```sh
docker compose down
```

Containers are removed; persistent data remains. Delete the selected data directory
separately only when its contents are no longer needed.

## Build and deployment status

The repository includes the Dockerfile and a release workflow for versioned
amd64/arm64 images plus this installation bundle. Registry publication has not yet
been exercised. The older NAS qualification passed original-byte round trips,
container recreation and full backup/restore. The revised Compose package and
proxy/tunnel setup still require runtime and actual-ingress acceptance. Current
status and dated evidence live in the repository's implementation/validation docs;
this guide describes the installation process, not a security certification.
