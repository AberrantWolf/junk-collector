# Container installation and updates

**For Linux Docker hosts and compatible Synology NAS devices.** Requires Compose v2.
FLAC and Opus tools are included. The server runs as UID/GID 10001; data stays outside the image.

**Install:** [image and storage](#1-select-an-image-and-storage) → [HTTPS](#2-configure-https-access) → [initialize](#3-initialize-and-start).
**Already installed?** See [updates](#update), [Portainer](#portainer-managed-stacks-and-ssh-maintenance) or [recovery](recovery.md).

## 1. Select an image and storage

Use an explicit image version or digest supplied by a release. On Synology DS1522+,
automatic platform selection uses linux/amd64.

<!-- release-image:start -->
- **Image:** `ghcr.io/aberrantwolf/junk-collector:v0.1.0-beta.3` (AMD64 and ARM64).
- **Download:** [GitHub release](https://github.com/AberrantWolf/junk-collector/releases/tag/v0.1.0-beta.3). The installation bundle pins the verified digest.
- **Channels:** `preview` follows prereleases; `latest` is reserved for stable releases.
- **Before updating:** read the [beta notes](../../docs/releases/v0.1.0-beta.3.md).

<!-- release-image:end -->

Extract the release installation bundle on the server, or use this directory from
a source checkout. Run the remaining commands beside `compose.yaml`; use `sudo`
for Docker commands if required by the host.

```sh
cp .env.example .env
```

Set these values in `.env`:

| Setting | Value |
|---|---|
| `COLLECTOR_IMAGE` | Selected image tag/digest; use the bundle's pinned default. |
| `COLLECTOR_DATA` | New permanent absolute data directory, e.g. `/srv/junk-collector` or `/volume1/junk-collector` on Synology. |
| `COLLECTOR_HOST`, `COLLECTOR_PRIVATE_IP` | Leave their loopback defaults for the proxy setup; see the direct HTTPS alternative below. |

Keep the data directory outside the source checkout and installation bundle.
SQLite/private state must live on the Docker host's own filesystem, not an SMB/NFS
client mount. Allow room for retained originals, prepared audio, resumable uploads,
and complete backups. Optional independent copies protect against loss of the host's storage.

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
3. Optionally uncomment `[providers]` and provide a monitored MusicBrainz contact
   address. A MusicBrainz account is not required.
   Metadata acquisition is otherwise unavailable; preservation still works.

Configure the proxy/tunnel route:

| Browser address | Upstream | Surface |
|---|---|---|
| `https://collector.example.com` | `http://127.0.0.1:9841` | Public router, with the exposure selected above. |
| Separate private management origin, if used | `http://127.0.0.1:9842` | Full management, reachable only through private ingress. |

**The connector must reach the host's loopback.** Use a host process or Linux host-network container.
A bridge container's `localhost` points to itself. A NAS reverse proxy can provide the local hop instead.
Keep any existing connector routes intact when choosing this topology.

For Cloudflare Tunnel, choose service type **HTTP** and URL `127.0.0.1:9841` in a
host-local connector. Cloudflare manages the browser certificate; phones need no
Collector certificate import. Collector login and collection permissions still apply.
See [Cloudflare routing](https://developers.cloudflare.com/tunnel/concepts/routing/).

**Never route Internet access to port 9842.** Forwarded headers do not grant private access.

Proxy requirements:

- Preserve Origin, cookies, request bodies, Range headers and response cache controls.
- Bypass shared caching for this hostname; avoid Cache Everything rules.
- Check [upload limits](https://developers.cloudflare.com/support/troubleshooting/http-status-codes/4xx-client-error/error-413/)
  and [cache behavior](https://developers.cloudflare.com/cache/concepts/cache-control/).
- For an HTTPS upstream hop, verify its certificate. Avoid a public origin URL that routes back into the tunnel.

### Alternative: direct private HTTPS

Skip the proxy configuration file. Before initialization set `COLLECTOR_HOST` to
a stable private DNS name or IP and `COLLECTOR_PRIVATE_IP` to the host's explicit
private IPv4 address. Reserve that address in DHCP. The initializer generates
management at `https://COLLECTOR_HOST:9844` and browsing/listening at
`https://COLLECTOR_HOST:9843`; management remains disabled on the latter.

Numeric-IP HTTPS addresses support password sign-in and recovery codes. Passkeys
require a stable DNS hostname with trusted HTTPS; setup/doctor and Account & security
explain this limit. Changing the authentication origin does not reset accounts or
delete passkeys, but existing passkeys stay bound to their original DNS identity.
Review certificate names and exact `allowed_origins` when changing addresses.
Malformed authentication origins fail before setup creates account/TLS state.

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

Create the exact host directory selected as `COLLECTOR_DATA` before initialization.
For example, with `COLLECTOR_DATA=/volume1/junk-collector`:

```sh
mkdir -p /volume1/junk-collector
docker compose run --rm initialize
```

Use the same absolute path as `.env`, and use host administrator privileges for
`mkdir` if required. DSM rejects a bind source that does not exist. Initialization
creates the `config`, `archive`, `state` and `backups` children with UID/GID 10001;
it does not recursively change an existing archive.

**For the proxy setup**, install the edited working config before account setup:

```sh
docker compose run --rm -T --entrypoint collector-install-proxy-config cli < collector.toml
sha256sum collector.toml
docker compose run --rm --entrypoint sha256sum cli /data/config/collector.toml
```

The two digests must match. `-T` passes redirected input without a pseudo-terminal.
The helper is for fresh installations only; it refuses existing archive data or private state.
Setup later normalizes formatting. Check the retained origins and exposure with `doctor`.
For direct HTTPS, skip this copy and keep the initializer's config.

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
ownership. Setup does not overwrite existing accounts or TLS and must run while stopped.
Ordinary `up -d collector` never creates or resets credentials. Changes to `.env`
host/IP values do not rewrite retained config or certificates; the actual config
is `COLLECTOR_DATA/config/collector.toml`. The archive and complete private state
are in its sibling `archive` and `state` directories.

### Optional SMB/server inbox

Give the non-root process read access through a dedicated host group and share
ACL. A bind mount's `ro`/`rw` flag does not grant host filesystem permission.
The live Synology installation uses `collector-inbox-readers`, GID 65538; discover
the actual GID on another host rather than copying that number blindly.

For the shipped Compose setup, add these values to `.env`:

```dotenv
COMPOSE_FILE=compose.yaml:compose.inbox.yaml
COLLECTOR_INBOX=/volume1/junk-collector-inbox
COLLECTOR_INBOX_GID=65538
```

The inbox source must already exist; the override refuses to create a misspelled
source path. It mounts `/inbox` read-only by default and applies the same supplemental group
to server and CLI tools. Add this section to the actual server config while stopped:

```toml
[inboxes]
cds = "/inbox"
```

Then inspect `docker compose config` and test the effective runtime identity/read:

```sh
docker compose run --rm --entrypoint id cli
docker compose run --rm --entrypoint ls cli /inbox
```

Grant traverse/read permission to the host group on the share and package
subdirectories, including SMB-created files. For explicit per-import cleanup, set
`COLLECTOR_INBOX_READ_ONLY=false` in `.env`, grant the corresponding host write/
traverse ACLs and recreate the service. Both server and CLI use that bind choice.
If you previously edited `read_only` directly, retain that choice in `.env` when
using a new bundle. Write access alone never deletes sources: use the separate
confirmed [inbox cleanup action](../../docs/inbox-cleanup.md) after preservation.
Do not recursively change ownership of an existing music share.

## 4. Check before bulk intake

- Import one complete supported package. Download and compare its originals.
- Restart. Confirm login and item access.
- If using audio, prepare a track, play it and seek.
- Complete a [recovery drill](recovery.md). Keep source originals according to your backup policy.

## Portainer-managed stacks and SSH maintenance

Keep the existing Portainer Collector definition, mounts, supplemental groups and
connector topology. The CLI-only files here do not declare your tunnel connector;
Compose may warn that it is an orphan. That warning is expected. Do not use
`--remove-orphans`, or `compose down`, against the live Portainer project. Keep the
existing bridge connector and unrelated routes; the Collector host-local connector
can remain a separate service.

Before maintenance, compare the running Collector's image with the CLI image:

```sh
docker ps --filter label=com.docker.compose.service=collector --format '{{.Names}} {{.Image}}'
docker inspect --format '{{.Config.Image}} {{.Image}}' YOUR_COLLECTOR_CONTAINER
docker compose config --images
docker image inspect YOUR_COLLECTOR_IMAGE --format '{{.Id}}'
```

The local image ID used by CLI tools must match the running image before a backup
or other storage-opening command. Set `.env`'s `COLLECTOR_IMAGE` accordingly; for a
locally loaded image the exact `sha256:...` image ID also avoids mutable-tag drift.
Do not open live data using a newer/older CLI merely because its tag looks familiar.

For an update, open Collector in Portainer's **Containers** view and choose **Stop**.
Make the selected consistent recovery point with the old CLI, then pull the selected
public image through **Images**. Record its digest. Change the Collector image in
the retained stack definition and the CLI `.env` together, keeping its mounts/groups.

For a stack that also contains a connector or another service, keep the other
services' resolved images and configuration unchanged, and update the stack with
whole-stack forced re-pull/redeployment disabled. The changed image replaces
Collector; services with unchanged configuration and images retain their containers.
The explicit image pull above supplies the new image even when the retained
definition uses the default pull policy. A Collector-only stack can use
**Re-pull image and redeploy** for a deliberate moving-tag update; that operation
forces redeployment across its stack.

Compare the resulting server/CLI image IDs again. Do not reset accounts,
replace retained config from the sample or remove a tunnel to clear an orphan warning.
The standalone Compose commands below apply to installations actually managed by
those complete files; do not use them to overwrite an incomplete copy of a live
Portainer definition.

## Local backup area and optional media copies

| Choice | Use |
| --- | --- |
| Local backup directory | `${COLLECTOR_DATA}/backups` on the host; `/data/backups` in CLI commands. Choose a fresh child directory for each backup. |
| [Btrfs recovery point](snapshot-recovery.md) | Routine local rollback without copying terabytes of unchanged media. |
| Full backup | Copies originals, archive records, private state and config. There is no database-only mode. |
| Independent media copy | Optional disk/NAS/cloud copy for protection against loss of the host's storage. |
| [Backup-age reminder](../../docs/local-alpha-installation.md#optional-backup-age-reminder) | Optional reminder; does not schedule or perform backups. |

## Update

Read the target build's storage/upgrade notes. For an eligible Btrfs installation,
use the [consistent snapshot/incremental recovery procedure](snapshot-recovery.md)
for routine local rollback. Optional independent exports and integrity checks can
follow the operator's media-backup policy. This avoids
copying/rehashing unchanged masters before every update. The full-copy fallback
below is for other installations or deliberately chosen full recovery points.
Use an explicit version/digest or
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
and TLS keys; keep them private. Copying media backups to independent storage is
optional and protects against risks a local rollback point cannot cover.

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

## Qualification records

- [Beta.2 publication and isolated NAS update](../../docs/validation/container-beta2-publication-2026-10-21.md).
- [DS1522+ installation, playback and full restore](../../docs/validation/nas-operations-2026-10-20.md).
- [Btrfs/incremental recovery](snapshot-recovery.md).

Each record states its tested scope. Live production updates are separate operator actions.

### Automatic private media

New builds automatically expose media-only TCP/UDP **9845**, with internally
managed short-lived certificate trust. Allow that port on the private host
firewall; do not forward it from the Internet. Browser listeners sign in normally
and may need to allow Local network access; Trash Amp pairs normally and obtains
trust automatically. No listener certificate import or private DNS is needed.
The existing direct management port 9844 is unchanged. Public media fallback
defaults to disabled; keep `remote_media_allowed = false` while the public media
path is unresolved. Public login/library access remains available. See the
[configuration and trust contract](../../docs/automatic-private-playback.md).
