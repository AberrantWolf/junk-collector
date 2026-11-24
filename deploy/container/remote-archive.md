# Local state with a remote NFS archive

This layout runs Collector on a Linux application host with private state on local
SSD and the archive on a separate NFS host (e.g. a NAS). Substitute your own paths,
mount sources and addresses. State and archive live on different hosts, so snapshot
both; see [scheduled snapshots](#scheduled-snapshots) and
[recovery points](#recovery-points).

## Layout

| Root | Host example | Container path | Contents |
| --- | --- | --- | --- |
| `COLLECTOR_CONFIG` | `/var/lib/junk-collector/config` | `/data/config` | Local SSD; read-only to the server, writable by offline CLI |
| `COLLECTOR_STATE` | `/var/lib/junk-collector/state` | `/data/state` | Local SSD; both SQLite databases, accounts, grants, Jobs, imports, uploads, TLS, scratch |
| `COLLECTOR_ARCHIVE` | `/srv/junk-collector/archive` | `/data/archive` (core/CLI), `/media` (media, read-only) | One NFS filesystem with the complete archive: originals, manifests, revisions, metadata, artwork, evidence, derivatives |
| `COLLECTOR_INBOX` | `/srv/junk-collector/inbox` | `/inbox` | Separate NFS mount, read-only by default |
| `COLLECTOR_BACKUPS` | `/var/lib/junk-collector/backups` | `/data/backups` (CLI) | Local recovery records and explicit backups |

The HTTPS frontend runs on the application host with its certificate/config under
local `COLLECTOR_DATA`. The [isolated media service](media.md) gets only the
read-only archive and its authority socket.

Private state includes retained imports (source and staging bytes), databases,
uploads, encoder scratch and publication copies. Size local SSD for your actual
pending workload.

## Archive identity

Publication stages files in `.staging` and `.transactions` inside the archive and
syncs before renaming; a committed revision rename is the point of no return. The
catalog can be rebuilt from the archive, but accounts, grants and Jobs cannot.

Normal commands require an enrolled installation:

- `.archive-identity` in the archive must match `archive-binding` in local state.
  These UUID files are recovery data, not integrity proof or secrets.
- Required archive directories must exist, without aliases, on the same filesystem.
  Remote mode checks the Linux mount table for the expected source, a `hard`
  NFSv4.1/4.2 mount and no nested mounts.
- A detected mismatch closes the store until restart (HTTP 503 `unavailable`).
- Archive writes go through an open root descriptor via `/proc/self/fd`, so an
  admitted operation cannot be redirected into an unmounted directory. Linux only.
- Missing private SQLite is a recovery error; startup never creates a new identity.
- I/O errors during publication or recovery are errors, never treated as "absent".

`setup` is only for fresh installations. For an existing installation without
identity files: stop every writer, keep a complete recovery point, verify the
archive and state, then run `enroll-archive` once against those exact roots. It
creates only the identity/binding files and required directories. Back up both
files. Repeating it on a matching pair is safe. Never delete the binding to get
past a refusal.

## NFS requirements

Use Linux NFSv4.1 or 4.2 mounted `hard`, with exclusive create, no-replace hard
links, same-filesystem rename, and `fsync` on files and directories that reports
failures. The server must honor stable writes/COMMIT, including across restarts;
do not enable asynchronous export acknowledgement. Do not use `soft`, `softerr`,
SMB, nested archive mounts or overlapping state/archive roots. Leave NFS cache
options at their defaults.

During a hard NFS outage, filesystem calls block until service returns, and
requests, workers and shutdown may wait. Never force- or lazy-unmount, replace the
directory, or start another writer to get past it. After an I/O error or identity
change, operations fail closed.

| Interruption | Behavior |
| --- | --- |
| Browse/download | Error or wait; never a catalog rewrite or deletion |
| Import/upload | Local staged sources and the request remain; publication is reconciled after restart |
| Encoding | Masters stay in the archive; scratch is recomputed after restart |
| Publication | Journal and staged files remain; committed bytes are checked before aborting |
| Startup | Identity and mount checks run before anything else; missing roots block startup |
| Inbox cleanup | Sources are kept on any error; keep cleanup disabled during migration |

**Only one writer.** Stop the old core, scheduled CLI jobs and any other writer
before starting the application host, and never run two hosts against the same
export. Neither NFS locks nor the archive UUID act as cross-host leader election.
NFS client identities must be unique and stable; see the
[Linux NFS client documentation](https://docs.kernel.org/admin-guide/nfs/nfs-client.html).

## Deployment

Use `compose.yaml`, `compose.media.yaml`, `compose.inbox.yaml` and
`compose.remote.yaml`, **in that order**, with a reviewed copy of
[remote.env.example](remote.env.example) at `/etc/junk-collector/remote.env`
(mode 0600). The remote overlay disables the generic initializer and Docker
automatic restarts; the systemd unit restarts the stack 30 seconds after any exit,
running the preflight first, so wrong mounts keep it down. Media waits for the
core to be ready. CLI import/restore get
the NFS source through `COLLECTOR_NFS_SOURCE` (same as `--nfs-source`).

In the local `collector.toml`:

```toml
archive = "/data/archive"
state = "/data/state"

[archive_mount]
nfs_source = "archive-host:/REPLACE/archive-export"
```

Use the source exactly as shown in the host's mount table.

The [systemd unit](junk-collector-remote.service) requires both NFS mount units,
ties the service lifetime to them, and runs the read-only `remote-preflight.py`
before Compose. The preflight checks NFS sources and targets, the UUID and binding,
non-overlapping roots, local ext4/Btrfs/XFS for state/config/backups, and the
container mount policy. It never mounts or repairs anything. Check mount unit
names with `systemd-escape --path --suffix=mount PATH` and the unit with
`systemd-analyze verify`. Mounts must wait for the network.

Do not let Portainer, cron, Docker restart policies or another systemd unit start
these containers. Recreate existing containers under this unit, since editing
YAML does not change existing containers' restart policies. Docker's data root and
named volumes must also be on local SSD. A reboot must either mount both sources
and pass preflight, or leave the services down.

Permissions: core runs as `10001:10001`; set `COLLECTOR_ARCHIVE_GID` and
`COLLECTOR_INBOX_GID` to the numeric groups that own the NAS data. Root squashing
is fine. Do not recursively chown/chmod the NAS. Test real UID/GID access,
including `fsync`, rename and media's read-only refusal, on a disposable export first.

### Proton Bridge and Cloudflare Tunnel

To add them, put `-f compose.mail.yaml -f compose.remote-mail.yaml -f compose.tunnel.yaml`
after `compose.remote.yaml` in all three Compose commands of the unit, add
`cloudflared` to the `ExecStart` service list, and start Bridge before the stack
(it isn't tied to the core's lifetime):

```ini
ExecStartPre=/usr/bin/docker compose --env-file /etc/junk-collector/remote.env -f compose.yaml -f compose.media.yaml -f compose.inbox.yaml -f compose.remote.yaml -f compose.mail.yaml -f compose.remote-mail.yaml -f compose.tunnel.yaml up -d --no-deps --pull never proton-bridge
```

Set the optional variables at the end of [remote.env.example](remote.env.example).
The tunnel's token file stays on the host, mode 0600 and owned by uid 65532 (the
user the `cloudflared` image runs as); Compose mounts it with the host's owner and
mode, so a root-owned file stops the connector, and with it the stack. Docker restarts Bridge
only if it crashes; the connector keeps its metrics on `127.0.0.1:20241`.

## Reconnecting after an outage

1. Stop the service and keep offline tools stopped. Save logs and pending Job and
   import IDs. Do not clear Jobs, receipts, `.transactions`, `.staging`, uploads
   or `state/imports`.
2. Restore the NFS server or network and let blocked calls settle. If mounts are
   stale, stop everything first, then remount and recreate containers.
3. Run `remote-preflight.py`. On a UUID or source mismatch, keep both trees and
   investigate; do not initialize or enroll a substitute.
4. Start the service. Startup reconciles journals and projections before serving.
   Inspect unknown journals rather than deleting them.
5. Check metadata, artwork, original downloads, grants, pending work and playback
   before resuming writes.

## Scheduled snapshots

Keep the local roots (config, state, backups, Caddy data and mail) in one Btrfs
subvolume, `COLLECTOR_DATA` (for example `/var/lib/junk-collector`), and snapshot
it on a timer. A
snapshot is atomic, so each SQLite database is captured together with its WAL and
recovers as it would after a power loss; it can be taken while Collector runs.

```sh
snapper -c junk-collector create-config /var/lib/junk-collector
snapper -c junk-collector set-config TIMELINE_CREATE=yes TIMELINE_LIMIT_HOURLY=24 \
  TIMELINE_LIMIT_DAILY=7 TIMELINE_LIMIT_WEEKLY=0 TIMELINE_LIMIT_MONTHLY=0 \
  TIMELINE_LIMIT_YEARLY=0 NUMBER_LIMIT=10
systemctl enable --now snapper-timeline.timer
```

The timer serves every snapper config, so check that others (such as `root`) have
`TIMELINE_CREATE="no"` unless you want them on it too.

Snapshot the archive's share on the NAS as well. On a Synology with a Btrfs volume,
use Snapshot Replication: daily, keeping 14 daily and 8 weekly snapshots, with
**Make snapshot visible** off so the snapshot folder doesn't appear over NFS.
Btrfs snapshots only use space for data that later changes or is deleted.

Snapshots share disks with what they protect. Keep a separate copy of the archive
(for example Hyper Backup to a USB drive or another site) for NAS failure.

## Recovery points

Before an update, stop the service, then take one local snapshot and one NAS
snapshot with the same description (for example `before v0.1.0-beta.35`). Lock the
NAS snapshot so its retention policy can't remove it. Delete both once the new
version has run for a few days.

```sh
snapper -c junk-collector create --description "before <version>"
```

The media service key lives in Docker's `media-authority` volume, outside the
snapshot. Copy it into the recovery point's private directory, keeping its
ownership and mode.

**Restore** both snapshots of the same point together, with the old images and
`remote.env`. Never restore only the local state beside a newer archive, or rsync
old files over live roots. Restoring local state also restores old passwords,
grants and sessions: reapply later security changes and revoke sessions as needed
before reopening public access. Sign in, check item counts and play a track before
switching traffic back.

## Routine updates

When the release notes list no data conversion and the Compose files, preflight
and unit are unchanged, only the image changes:

1. Before downtime, pull the image and note its digest and the Library counts:

   ```sh
   docker pull ghcr.io/aberrantwolf/junk-collector:<tag>
   docker image inspect --format '{{index .RepoDigests 0}}' ghcr.io/aberrantwolf/junk-collector:<tag>
   ```

2. `systemctl stop junk-collector-remote.service`, check that `docker ps -a` shows
   every container exited, and take a [recovery point](#recovery-points).
3. Copy `remote.env` aside, set `COLLECTOR_IMAGE` and `COLLECTOR_MEDIA_IMAGE` to the
   new digest, `diff` the two files, then `systemctl start junk-collector-remote.service`.
4. Sign in, play a track and compare the counts. The first Library load rebuilds
   its cached rows.

To roll back a release without a data conversion, copy the old `remote.env` back
and restart, before using anything the release notes say the old version can't
read. When a release converts data or changes mounts, Compose files, the unit or
config, apply those changes during step 3 and test the affected workflows; going
back after a conversion means restoring the recovery point.

Once the new version has proven itself, remove the recovery point and the images
older than the previous release.

To run root commands on a host without sudo, use the Docker operator route with
an already-installed helper image pinned by its local image ID:

```sh
host_admin() {
  docker run --pull=never --rm -i --user 0:0 --privileged --pid=host \
    --network=none --entrypoint nsenter "$helper_image_id" -t 1 -m -- "$@"
}
host_admin /usr/bin/id -u  # prints 0
```

## Testing on disposable VMs

Never use production exports, paths or credentials for interruption tests. Use
two disposable Linux VMs (NFS server and client) with the same NFS version, numeric
IDs and stable-write behavior as the real setup. On the fixture server only:

```sh
sudo install -d -m 2770 -o 10001 -g 65538 /srv/collector-fixture/archive /srv/collector-fixture/inbox /srv/collector-fixture/probe
sudo tee /etc/exports.d/collector-fixture.exports <<'EXPORTS'
/srv/collector-fixture/archive 192.0.2.12(rw,sync,root_squash,no_subtree_check)
/srv/collector-fixture/inbox 192.0.2.12(rw,sync,root_squash,no_subtree_check)
/srv/collector-fixture/probe 192.0.2.12(rw,sync,root_squash,no_subtree_check)
EXPORTS
sudo exportfs -ra
sudo systemctl start nfs-server
```

On the fixture client:

```sh
sudo install -d -m 0755 /mnt/collector-fixture/archive /mnt/collector-fixture/inbox /mnt/collector-fixture/probe
sudo mount -t nfs -o vers=4.1,hard,rw 192.0.2.11:/srv/collector-fixture/archive /mnt/collector-fixture/archive
sudo mount -t nfs -o vers=4.1,hard,ro 192.0.2.11:/srv/collector-fixture/inbox /mnt/collector-fixture/inbox
sudo mount -t nfs -o vers=4.1,hard,rw 192.0.2.11:/srv/collector-fixture/probe /mnt/collector-fixture/probe
findmnt -T /mnt/collector-fixture/archive
```

The filesystem probe runs against an empty directory containing only
`.collector-nfs-qualification-only` with the line
`DISPOSABLE COLLECTOR NFS QUALIFICATION ONLY`. Run it as UID/GID 10001 with group
65538 (e.g. `setpriv --reuid=10001 --regid=10001 --groups=65538`):

```sh
python3 scripts/qualify_archive_filesystem.py \
  --root /mnt/collector-fixture/probe \
  --source 192.0.2.11:/srv/collector-fixture/probe
```

It checks filesystem semantics only, not power-loss durability or application
recovery. Worth exercising on the fixture: restarting `nfs-server` during staging,
copying, encoding and the final renames with work pending; starting with the
archive missing or wrong; Docker daemon restart and client reboot (services must
stay down); and restoring a recovery point into new roots.
