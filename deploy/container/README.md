# Container installation and updates

For Linux Docker hosts and compatible Synology NAS devices, using Compose v2.
The image includes the FLAC and Opus tools. The server runs as UID/GID 10001 and
keeps all data outside the image.

Install: [image and storage](#1-select-an-image-and-storage) → [HTTPS](#2-configure-https-access) → [initialize](#3-initialize-and-start).
Already installed: [update](#update), [Portainer](#portainer-managed-stacks), [recovery](recovery.md).
For local state with the archive on a separate NFS host, use [remote-archive.md](remote-archive.md) instead.

## 1. Select an image and storage

Each [GitHub release](https://github.com/AberrantWolf/junk-collector/releases)
publishes `ghcr.io/aberrantwolf/junk-collector:<version>` for AMD64 and ARM64, and
attaches `junk-collector-container.tar.gz`. The `preview` tag follows prereleases
and `latest` follows stable releases. Read the release notes before updating.

Extract the bundle on the server (or use this directory from a source checkout)
and run the remaining commands beside `compose.yaml`, with `sudo` if your host
requires it for Docker.

```sh
cp .env.example .env
```

Set in `.env`:

| Setting | Value |
|---|---|
| `COLLECTOR_IMAGE` | The bundle pins its release version; change it only to pick another version. |
| `COLLECTOR_DATA` | A new permanent absolute directory, e.g. `/srv/junk-collector` or `/volume1/junk-collector`. |
| `COLLECTOR_HOST`, `COLLECTOR_PRIVATE_IP` | Keep the loopback defaults for the proxy setup; see [direct private HTTPS](#alternative-direct-private-https). |

Keep the data directory outside the checkout and bundle. SQLite and private state
must be on the Docker host's own filesystem, not an SMB/NFS mount. Leave room for
originals, prepared audio, resumable uploads and full backups.

The Compose file uses Linux host networking for its separate loopback listeners;
Docker Desktop is not a supported target.

## 2. Configure HTTPS access

### Recommended: reverse proxy or Cloudflare Tunnel

```sh
cp collector.proxy.toml.example collector.toml
```

Edit `collector.toml`:

1. Replace every `https://collector.example.com` with the exact HTTPS URL browsers
   will use (hostname at the URL root, including any non-default port). Keep it
   stable: passkeys are bound to it.
2. Choose management exposure. To administer through this same hostname, set
   `public_administration = true` (this also enables imports and curation; account
   and collection permissions still apply). For a browse/listen-only hostname,
   leave both exposure flags false and manage through a separate private route.
3. Optionally uncomment `[providers]` and give a MusicBrainz contact address.
   Without it metadata lookup is unavailable, but preservation still works.

Proxy routes:

| Browser address | Upstream | Surface |
|---|---|---|
| `https://collector.example.com` | `http://127.0.0.1:9841` | Public router with the exposure chosen above |
| Optional private management origin | `http://127.0.0.1:9842` | Full management; private ingress only |

**Never route Internet traffic to port 9842.** Forwarded headers do not grant private access.

The proxy or connector must reach the host's loopback: run it as a host process or
a host-network container (a bridge container's `localhost` is itself), or use a NAS
reverse proxy for the local hop.

For Cloudflare Tunnel, use service type **HTTP** with URL `127.0.0.1:9841` in a
host-local connector. Cloudflare provides the browser certificate. See
[Cloudflare routing](https://developers.cloudflare.com/tunnel/concepts/routing/).

The proxy must preserve Origin, cookies, request bodies, Range headers and response
cache headers. Bypass shared caching for this hostname (no Cache Everything rules)
and check its [upload size limit](https://developers.cloudflare.com/support/troubleshooting/http-status-codes/4xx-client-error/error-413/).
Verify the certificate of any HTTPS upstream hop.

To serve music from a separate direct HTTPS hostname, see [media.md](media.md).

### Alternative: direct private HTTPS

Skip `collector.toml`. Before initializing, set `COLLECTOR_HOST` to a stable private
DNS name or IP and `COLLECTOR_PRIVATE_IP` to the host's private IPv4 address
(reserve it in DHCP). The initializer configures management at
`https://COLLECTOR_HOST:9844` and browsing/listening at `https://COLLECTOR_HOST:9843`.

With a numeric IP, use passwords and recovery codes; passkeys need a stable DNS
hostname with trusted HTTPS. Changing the origin later keeps accounts, but existing
passkeys stay bound to the old hostname; update certificate names and
`allowed_origins` too.

Setup generates a self-signed certificate (it does not create DNS records). After
startup, fetch it and compare its fingerprint with the one in the logs:

```sh
docker compose exec -T collector cat /data/state/tls/cert.pem > collector-cert.pem
docker compose logs collector
sha256sum collector-cert.pem
```

Then trust only this certificate in the browser or client. Never copy `key.pem` or
disable certificate validation. Renewing this certificate is up to you.

## 3. Initialize and start

Pull a published image (skip for a locally built one), create the data directory
(DSM rejects missing bind sources), and initialize:

```sh
docker compose pull collector cli initialize
mkdir -p /volume1/junk-collector   # your COLLECTOR_DATA
docker compose run --rm initialize
```

This creates `config`, `archive`, `state` and `backups` owned by 10001. It never
overwrites existing config or recursively changes an existing archive.

For the proxy setup, install your `collector.toml` (fresh installations only) and
check the two digests match:

```sh
docker compose run --rm -T --entrypoint collector-install-proxy-config cli < collector.toml
sha256sum collector.toml
docker compose run --rm --entrypoint sha256sum cli /data/config/collector.toml
```

Create the owner account, check the configuration, and start:

```sh
docker compose run --rm cli setup
docker compose run --rm cli doctor
docker compose up -d collector
docker compose logs --tail 40 collector
```

Setup asks for a password of at least 12 characters. Open the configured origin
and sign in as `owner`. If you cannot create collections, the origin is not reaching
a management-enabled router.

Setup runs only while the server is stopped and never overwrites existing accounts
or TLS. The live config is `COLLECTOR_DATA/config/collector.toml`; changing
`.env` host/IP values later does not rewrite it or the certificate.

### Optional SMB/server inbox

Grant the server read access through a dedicated host group and share ACL (a
bind mount's `ro`/`rw` flag does not grant filesystem permission). Add to `.env`:

```dotenv
COMPOSE_FILE=compose.yaml:compose.inbox.yaml
COLLECTOR_INBOX=/volume1/junk-collector-inbox
COLLECTOR_INBOX_GID=REPLACE_WITH_INBOX_GROUP_GID
```

The inbox directory must already exist. It is mounted read-only at `/inbox` for
the server and CLI. Add to the server config while stopped:

```toml
[inboxes]
cds = "/inbox"
```

Check access:

```sh
docker compose run --rm --entrypoint id cli
docker compose run --rm --entrypoint ls cli /inbox
```

The group needs traverse/read permission on the share and all package
subdirectories, including files created over SMB. To allow the optional
[inbox cleanup](../../docs/inbox-cleanup.md) action, set
`COLLECTOR_INBOX_READ_ONLY=false`, grant write ACLs and recreate the service.
Write access alone never deletes sources. Do not recursively chown an existing music share.

## 4. Check before bulk import

- Import one complete package, then download and compare its originals.
- Restart and confirm login and item access.
- If using audio, prepare a track, play it and seek.
- Do a [recovery drill](recovery.md).

## Backups

| Option | Use |
| --- | --- |
| Full backup | `cli backup` into a fresh directory under `/data/backups` (`${COLLECTOR_DATA}/backups` on the host). Copies originals, archive records, private state and config. |
| [Btrfs snapshot](snapshot-recovery.md) | Fast local rollback without copying unchanged media. |
| Independent copy | Optional copy to another disk/NAS/cloud, against loss of the host's storage. |
| [Backup-age reminder](../../docs/server-operations.md) | Optional reminder; does not run backups. |

Backups contain credentials and TLS keys; keep them private.

## Update

Make a recovery point first: a [Btrfs snapshot](snapshot-recovery.md) if eligible,
otherwise a full backup. Use an explicit version tag, not automatic `latest` updates.

```sh
cp .env .env.before-update
docker compose images
docker compose stop collector
docker compose run --rm cli backup --destination /data/backups/before-update-UNIQUE
docker compose run --rm cli validate-backup --backup /data/backups/before-update-UNIQUE
```

Stop if any command fails. Then set `COLLECTOR_IMAGE` in `.env` to the new version and:

```sh
docker compose pull collector cli initialize
docker compose run --rm cli enroll-archive   # one-time for installations created before archive identity; safe to repeat
docker compose run --rm cli doctor
docker compose up -d collector
docker compose logs --tail 40 collector
```

Run `enroll-archive` only against the existing, verified data directory, never an
empty replacement. Check login, browsing, an original download and playback. Do not rerun setup or
replace the config from the example.

Once a new version has opened the data, switching back only the image is not a
safe rollback. Restore the recovery point with its matching old image instead
([recovery.md](recovery.md)). Never run the original and a restored copy at the same time.

## Portainer-managed stacks

Keep the existing Portainer stack definition, mounts, groups and tunnel connector.
Use the Compose files here only for one-shot CLI commands. Compose may warn that
the connector is an orphan; ignore it. Never run `--remove-orphans` or
`compose down` against the live Portainer project.

Before a backup or any other command that opens the data, make sure the CLI image
matches the running server image:

```sh
docker ps --filter label=com.docker.compose.service=collector --format '{{.Names}} {{.Image}}'
docker inspect --format '{{.Config.Image}} {{.Image}}' YOUR_COLLECTOR_CONTAINER
docker compose config --images
docker image inspect YOUR_COLLECTOR_IMAGE --format '{{.Id}}'
```

To update: stop Collector in **Containers**, make the recovery point with the old
CLI, pull the new image in **Images**, run `enroll-archive` with the new image if
needed, then change the image in both the stack definition and `.env`. If the
stack contains other services, update it with forced re-pull/redeploy disabled so
they keep their containers; a Collector-only stack can use **Re-pull image and redeploy**.
Compare the server and CLI image IDs again afterward.

## Private media port

The server exposes media-only TCP/UDP **9845** for private playback, with
automatically managed short-lived certificates. Allow it on the private firewall;
do not forward it from the Internet. Browsers may ask to allow local network access;
Trash Amp obtains trust when it pairs.

Audio uses the normal authenticated website connection by default
(`remote_media_allowed = true`). To force private or separate media, enable
**Require a separate audio connection** in Administration → Server settings. See
[private playback](../../docs/automatic-private-playback.md).

## Optional email purchase inbox

The bundle includes `compose.mail.yaml`, the `proton-bridge/` build context and
`scripts/configure_proton_mail.py`. Only the Bridge image needs a local build
(Linux AMD64). Follow the [Bridge instructions](../proton-bridge/README.md).

## Stop or uninstall

```sh
docker compose down
```

This removes containers; data remains. Delete the data directory separately if no longer needed.
