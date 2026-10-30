# Fi local state with a remote FatChocobo archive

Prepared 2026-10-30; **not a migration approval or a live deployment change**.
[Delivery/validation](../../docs/validation/remote-archive-safety-2026-10-30.md)
records what actually ran. The previous [single-NAS snapshot procedure](snapshot-recovery.md)
remains historical evidence for the old layout; one NAS snapshot is insufficient
for this split.

## Authority and layout

| Root | Host location example | Container location | Contents/access |
| --- | --- | --- | --- |
| `COLLECTOR_CONFIG` | `/var/lib/junk-collector/config` | `/data/config` | Local SSD; server read-only, offline CLI writable |
| `COLLECTOR_STATE` | `/var/lib/junk-collector/state` | `/data/state` | Local SSD; both SQLite databases, accounts, grants, Jobs, imports, uploads, TLS, processing scratch |
| `COLLECTOR_ARCHIVE` | `/srv/junk-collector/archive` | core/CLI `/data/archive`, media `/media` | One NFS filesystem containing the **complete archive**, including originals, manifests, revisions, accepted metadata/artwork, evidence and derivatives; media read-only |
| `COLLECTOR_INBOX` | `/srv/junk-collector/inbox` | `/inbox` | Separate NFS mount; read-only by default |
| `COLLECTOR_BACKUPS` | `/var/lib/junk-collector/backups` | CLI `/data/backups` | Local recovery records/explicit backups; not automatic full media copies |

Caddy runs on Fi. Its certificate/config directories remain under local
`COLLECTOR_DATA`; named authority/HTTP socket volumes remain private to core/media.
Media receives no SQLite, account state, configuration directory or inbox. Keep
its network namespace, authority socket, read-only archive and existing read-only
mount enforcement. A directory capability anchors media reads. The remote profile
also requires its expected archive UUID before cached file admission.

The assessment reports pinned beta.13 core/media, about 105 GB of archive and 76 GB
of private state (mostly `state/imports`). **Copy all private state.** Imports retain
accepted inventories, frozen requests, source/staging bytes and recovery bookkeeping;
this delivery establishes no deletion/exclusion policy for those 76 GB. Reserve
local capacity for them, databases, active uploads, simultaneous encoder scratch,
publication copies, recovery points and normal SSD free space. Size this from the
actual pending workload before approving migration.

Keep existing public control/media hostnames, authentication origin, keys and
endpoint identity. Review the old `192.168.1.106:9845` local playback bind and other
NAS listener addresses for Fi during the later migration. This preparation does
not change DNS, ingress, router, tunnels, export configuration or permissions.

## Existing mechanisms and new admission rules

Existing publication uses `.staging` and `.transactions` **inside the archive**.
Inputs/encoding scratch stay local, then copy into archive-local staging and sync
before renames. A committed revision rename is the authority boundary; a Job is
not complete merely because encoding or copying finished. Startup classifies known
journals, validates staged/committed bytes and completes projections; unknown
staging is retained. Private upload sources are retained after publication. The
catalog is derived; private accounts/grants/Jobs are not reconstructible from it.

Previously ordinary startup could mkdir a missing archive and rebuild from an
empty directory. Normal executable commands now require an enrolled installation:

- `.archive-identity` in the archive and `archive-binding` in local state must match.
  These UUID files are recovery data, not proof of payload integrity or a secret.
  They are synced and atomically published without replacement; unpublished
  identity temporaries do not become archive authority.
- Required archive directories must exist without aliases, and share the root's
  filesystem. Remote mode checks the actual Linux mount table, expected source,
  hard NFSv4.1/4.2 and absence of nested archive mounts before recovery/mutation.
- Store admission checks directory identity, required roots and the UUID at its
  database/writer gates and publication phases. A detected failure latches that
  Store closed; reconnect alone cannot resume it. Errors use the existing typed
  `unavailable` HTTP 503 response. There is no polling-time payload hashing.
- Linux archive mutation paths anchor to an open root descriptor through `/proc`.
  An already admitted operation cannot be redirected into an unmounted directory.
  `/proc/self/fd` must be available inside the container. Remote mode is Linux-only.
- Missing enrolled private SQLite is a recovery error; normal startup does not
  create a new identity/database. Explicit setup can resume an interrupted fresh
  bootstrap only when its durable initialization intent matches the archive
  binding; the intent is retired before accounts can be created. Unbound retained
  imports/scratch without their database are refused as non-fresh state. Local account recovery remains independent of
  archive availability.
- Publication/recovery presence checks propagate I/O errors. An inaccessible
  artifact is not interpreted as an absent artifact eligible for cleanup.

`setup` is the explicit fresh-installation path. It may initialize a new empty
archive, or create fresh accounts for an explicitly supplied portable archive
when private state does not exist. It cannot replace an established unbound
installation. For a beta.13 upgrade, first stop every writer, retain a complete
pre-change recovery point, verify the intended archive inventory and state, then
run the **new qualified image's** `enroll-archive` once against those exact roots.
Enrollment creates only identity/binding and required operational directories;
it does not open/migrate SQLite or validate all media. Record the UUID and back
up both files. Repeating enrollment against a matching pair is safe. A missing
mount, mismatched binding or incomplete restore is a refusal. Never remove the
binding to bypass a refusal. The initial preparation ran no production enrollment.
The later [beta.14 update](../../docs/validation/container-beta14-publication-2026-10-30.md)
enrolled the retained local NAS installation after a complete stopped-server snapshot;
that is separate from Fi/NFS qualification and migration approval.

## Filesystem contract and interruption behavior

Require Linux NFSv4.1 or 4.2 with `hard`, ordinary coherent directory lookups,
exclusive file creation, no-replace hard links for identity publication,
same-filesystem file/directory rename, file `fsync` and
directory `fsync` that report failures. The server must honor stable writes/COMMIT
and durable metadata, including after server restart. Use the NAS's supported
synchronous/stable export behavior; do not enable asynchronous acknowledgement to
improve benchmark numbers. The application cannot prove server-side stable media
from a mount flag. Qualify the actual client/server versions and storage stack.
Do not use `soft`, `softerr`, SMB, cross-filesystem staging, nested archive mounts,
write-behind substitutes, or overlapping state/archive roots. Leave normal NFS
cache defaults until measured qualification justifies a change.

A hard NFS outage can block a kernel filesystem call until service returns. The
identity guard does not impose a userspace deadline on a blocked kernel call;
requests, workers and shutdown may wait. Do not interpret a hung call as an empty
archive, force-unmount it, lazily unmount it, replace the directory, or start another
writer. Observe mount/server health from outside the application. The existing
active-encoder shutdown timeout follow-up remains open. Once an I/O error or changed
identity is observed, operations fail closed; earlier archive commits remain
commits even if the client lost their acknowledgement.

| Interruption | Recovery expectation |
| --- | --- |
| Browse/download | A storage error/refused admission, or a hard-mount wait; no catalog rewrite, tombstone or absence-driven deletion |
| Inbox/upload import | Local staged sources and durable request remain; incomplete archive publication is reconciled after reconnect/restart; a queued source copy can wait for inbox recovery |
| Encoding | Masters remain in the archive and input/request remains durable; local disposable encoding scratch may be recomputed after restart; failed availability cannot publish success |
| Publication | Keep archive-local journal and staged/published artifacts. After a lost rename acknowledgement, inspect actual committed bytes before aborting; no rollback of a committed revision |
| Startup recovery | Identity/mount admission precedes directory creation, database opening, reconciliation and catalog replacement; unavailable roots block startup |
| Explicit inbox cleanup | Archive validation and claim/permission checkpoints still apply; remaining sources are retained on errors. Keep cleanup disabled/read-only during migration acceptance |

One writer is an **operations fence**: stop/disable the old NAS core, scheduled
CLI jobs and any other writer before Fi is admitted. Do not run both hosts on the
same export even with separate local state. Private/archive filesystem locks
remain useful same-host checks. Neither an NFS lock nor a matching UUID is tested
or accepted as cross-host leader election. NFS client identity must remain unique
and stable; see the [Linux NFS client documentation](https://docs.kernel.org/admin-guide/nfs/nfs-client.html).

## Guarded deployment templates

Use `compose.yaml`, `compose.media.yaml`, `compose.inbox.yaml`, then
`compose.remote.yaml` **in that order**, with a reviewed copy of
[remote.env.example](remote.env.example). The last overlay disables the generic
initializer and Docker automatic restarts for core, media and proxy. Media waits
for the core authority socket/key health condition before its first key read,
including when its named volume is new. Retain the [isolated media configuration](media.md),
including the dedicated authority socket and disabled control-origin fallback. Its source
requirement also reaches CLI import/restore via `COLLECTOR_NFS_SOURCE` (equivalent
to global `--nfs-source SOURCE`). Source conflicts with configuration are refused.
Directory existence and `create_host_path: false` alone do not establish a mount.
Long bind syntax prevents Docker from creating missing sources; see
[Docker bind mount behavior](https://docs.docker.com/engine/storage/bind-mounts/).

In the local `collector.toml`, retain container paths and add:

```toml
archive = "/data/archive"
state = "/data/state"

[archive_mount]
nfs_source = "FatChocobo:/REPLACE/archive-export"
```

Use the exact source spelling observed in Fi's mount table (a hostname may be
resolved/normalized). The host preflight compares both NFS source/target pairs,
UUID/binding, non-overlapping roots, local ext4/Btrfs/XFS state/config/backups,
Caddy certificate/proxy-config directories and
the container path/mount policy. It is read-only and never mounts or repairs.
The [systemd unit template](junk-collector-remote.service) requires the two mount
units, binds service lifetime to them and runs that preflight before Compose.
Review path-derived unit names using `systemd-escape --path --suffix=mount PATH`.
Review with `systemd-analyze verify` on Fi. Mount configuration must wait for the
network and use the intended source; this repository does not install fstab or
change exports. See [systemd mount dependencies](https://www.freedesktop.org/software/systemd/man/latest/systemd.unit.html).

Do not enable a Portainer stack, cron entry, Docker restart policy or second
systemd service that bypasses this owner. During later adoption, inspect the
**effective** Compose model and existing containers' restart policies, then
recreate them under this unit; editing YAML does not alter existing containers.
Before acceptance, confirm Docker's actual data root and named authority/socket
volumes also reside on Fi's local SSD; the preflight does not query or relocate
Docker's storage driver. A Docker daemon restart must not autonomously start these containers. A host reboot
must either mount both intended sources and pass preflight or leave services down.
If the host loses a mount unit, systemd stops the stack; a mere network stall does
not necessarily make that unit inactive. Restart requires operator inspection.

Keep core UID/GID `10001:10001` and validate supplemental GID `65538` against actual
NAS numeric ownership/ACLs; remote and inbox overlays apply their configured groups
to CLI/core, and the archive group to media. Group existence/name is insufficient.
NFS root squashing is compatible with a prepared export; do not recursively chown
or chmod the NAS. A root initializer is disabled in this profile. Have an operator
prepare only new local SSD directories and confirm real UID/GID access on a
**disposable fixture export**, including `fsync`/rename and media read-only refusal.
Do not infer permissions from a root shell or weaken the media mount.

## Reconnect/restart procedure

1. Stop admission at the service owner; keep the NAS core and offline tools stopped.
   Record logs and pending Job/import identities. Do not clear Jobs, receipts,
   `.transactions`, `.staging`, uploads or `state/imports`.
2. Restore the intended server/export/network through the authorized operator.
   Wait for hard-mounted operations to settle. If mount objects are stale, first
   stop all owners and then remount/recreate containers through the host owner;
   never stack a new mount over a running writer. Do not force/lazy-unmount to
   hurry a blocked writer or assume a timed-out stop has released it.
3. Run host `remote-preflight.py` using the reviewed environment. Verify UUID,
   exact sources, local SSD state and numeric access. On UUID/source mismatch,
   preserve both trees and investigate; do not initialize or enroll a substitute.
4. Start one core through the guarded unit. Startup reconciles the original
   journal/commits and projections before serving. Corrupt/unknown journals require
   inspection; preserve originals and sources rather than deleting blockers.
5. Confirm accepted manual metadata/artwork, original downloads, account grants,
   stable pending work IDs/timestamps and derived playback. Retry visible failures
   with existing workflow commands; allow unexpired leases to settle. Resume writes
   only after these checks. Keep ingestion source cleanup disabled until accepted.

## Coordinated recovery points and rollback

Take one coordinated **stopped-writer** point before upgrades or migration. No full
media copy/hash is required for a routine point:

1. Stop core, media, Caddy and every offline tool on both hosts. Disable automatic
   restart owners. Wait for actual process exit and completed writes; blocked NFS
   calls are not a checkpoint. Use the same point ID on both machines. Record image
   digests, source revision, ServerId, archive UUID, paths, source/export/mount options,
   numeric groups, schema/tool versions and pending work IDs/timestamps.
2. Verify the intended mounts, then drain pending writeback with `sync -f` on
   the archive mount and local state/config filesystems on the fixture/approved
   deployment host; abort the checkpoint on any error or blocked call. This reads
   no media for hashing. While all writers stay stopped, take a read-only NAS snapshot covering the
   **entire archive** and a read-only Fi snapshot covering **all** local state/config.
   They need not be simultaneous when the writer remains stopped between them.
   Include `state/imports`, uploads, scratch, WAL/database files as one stopped
   tree, TLS keys, Caddy data/config, Compose/env and host lifecycle/mount settings.
   Record any other authoritative external path explicitly. Socket files themselves
   are ephemeral; copy the authority volume's persistent service key into the
   pair's private recovery directory alongside the snapshots, retaining its
   numeric ownership and mode, and sync that copy before sealing the pair. Keep these
   private. An inbox is separate; retain unfinished source packages under its own
   snapshot/retention policy.
3. On Fi Btrfs, verify the actual local data subvolume and absence of excluded
   nested subvolumes/mounts, then use a read-only snapshot outside it. Enumerate
   every resolved host bind: overriding `COLLECTOR_STATE`, `COLLECTOR_CONFIG` or
   Caddy paths can place private data outside `COLLECTOR_DATA`. Capture each such
   filesystem/tree as another member of the same stopped-writer point and list
   all members in the pair record; a snapshot of `COLLECTOR_DATA` alone is not
   sufficient in that layout. On ext4/XFS
   without a coordinated snapshot tool, copy the **complete stopped local tree**
   with ownership/ACLs to a new private point and sync it. This may copy 76 GB locally;
   it does not copy 105 GB of NAS media. Check free space and copy exit status.
4. Publish a private pair record only after **both** sides succeed. Include NAS
   snapshot ID and local snapshot/copy ID, config/systemd/env copies and the exact
   old image digests. A failed half is incomplete, never a usable paired point.
   Keep the last accepted pair. Restart only after the pair is sealed.

Existing `backup` remains a verified **full** copy of both roots/config. It is an
optional independent copy, not a lightweight split snapshot command. A NAS archive
snapshot alone, local SQLite alone or copying active WAL files independently is not
a recoverable installation. Same-device points do not protect against device loss;
independent full-media backups remain operator policy.

Restore into newly named roots/export paths, with the writer still fenced. Clone
both members of the **same** pair, preserve the current pair for diagnosis, restore
numeric permissions/config/keys, adjust only the recorded host bindings and exact
NFS source, and run admission plus acceptance in an isolated network. A matching
UUID identifies an archive lineage; it does not prove matching checkpoint times.
Check the pair record, not just the UUID. Only one writable copy of a ServerId may
serve. Use the point's image and never restore only SQLite beside a later archive.

**After new Fi writes**, restarting beta.13 on the old NAS private state against
the now-newer shared archive is unsafe: Jobs, grants, uploads and command outcomes
may disagree. Either roll forward with the new complete state, or stop all writers,
retain a new current pair and restore **both** old members plus old config/images.
The latter deliberately loses post-point changes in the active view; reconcile or
export wanted new records before approving that rollback. Do not merge databases,
rsync old files over live roots or assume append-only originals make mixed points
safe. Restoring private state also restores point-in-time passwords, grants and
revocations: reapply known post-point security changes and revoke restored
sessions/device credentials as appropriate in the isolated restored instance
before reopening public ingress. This change does not supply an automatic
merge/downgrade tool.

Verify restoration before switching ingress: sign in with retained credentials,
check ServerId/TLS/passkeys/grants, inventory counts and representative accepted
manual metadata/artwork/unknown data, read a sample original and compare its recorded
digest, play/seek a published derivative, resume one pending upload/import/encoding
request and restart once more. Use fixture-sized integrity checks, not routine
full-library hashing. Record outcomes and point IDs; a successful snapshot command
alone is not a restoration drill.

## Isolated qualification and approval gate

Use two disposable Linux VMs (one NFS server, one Fi-like client), a dedicated
throwaway export and a new local SSD fixture tree. Never use production archive or
inbox paths, production exports, credentials or routes for interruption tests.
The fixture export must offer the same supported NFS version, numeric UID/GID and
stable-write behavior intended for deployment. Repeat acceptance on the actual
Fi/NAS stack only in a separately approved disposable test area.

1. Run `cargo test --workspace --locked` on Linux, including `storage_guard`'s
   anchored-root and mount-table tests. Run the isolated filesystem probe below
   as UID/GID 10001 with the intended supplemental group. Save mountinfo, kernel,
   NFS server/storage versions and probe output. This tests fixture data only.
2. Render Compose with all four files and the fixture environment; inspect binds,
   groups, network isolation, read-only media and restart policies. Use qualified
   release binaries/images. Enroll/setup fixture storage, then run the ordinary
   browser import/metadata/audio/media workflows through the real UI.
3. Keep a fixture upload pending and another import/encode active. On the
   **disposable NFS server only**, interrupt service before staging, while copying,
   after encoding, after payload rename and after revision rename (the Rust fault
   tests deterministically cover these phases without any network outage).
   Assert no success without committed bytes, no empty reindex, no new underlying
   archive, no lost accepted manual values and retained source/private work.
4. Restore NFS, follow the reconnect procedure, restart and finish those same
   request IDs. Compare fixture bytes and accepted fields/evidence, including
   post-commit lost acknowledgements. Restart the NFS server again to test durable
   commits. Test full-disk/read-only/I/O refusal with the fixture backend only.
5. With the archive absent/wrong, try direct core/CLI startup, Docker daemon restart
   and client reboot. Separately remove the fixture inbox mount and verify host
   preflight/unit refusal. The application guard attests archive identity; inbox
   source identity is the host preflight's responsibility, so direct manual core/CLI
   startup is not a supported bypass for the guarded service owner. Services must remain down and underlying mountpoint directories
   unchanged. Restore mounts and verify one writer starts. Check source mismatch,
   wrong UUID, remote SQLite and nested staging mount refusals. Media must be unable
   to write. No cross-host lock result authorizes a second writer.
6. Create a paired fixture point with pending work, add a later manual edit/import,
   restore both members into isolated new roots with old images and verify the
   point plus pending work. Separately verify retained post-point data is available
   for a deliberate roll-forward decision. Save a restoration receipt.

For a repeatable fixture filesystem setup, first install the distribution's NFS
server/client tools in two **disposable Linux VMs**. The following example uses
server `192.0.2.11` and client `192.0.2.12` on an isolated test network; replace both
with the fixture addresses. These commands are **never for FatChocobo or any
production host**. On the fixture server only:

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

On the fixture client only, create mountpoints as root with mode 0755, and mount
the three exports using `mount -t nfs -o vers=4.1,hard,rw` at separate
`/mnt/collector-fixture/{archive,inbox,probe}` paths. For example:

```sh
sudo install -d -m 0755 /mnt/collector-fixture/archive /mnt/collector-fixture/inbox /mnt/collector-fixture/probe
sudo mount -t nfs -o vers=4.1,hard,rw 192.0.2.11:/srv/collector-fixture/archive /mnt/collector-fixture/archive
sudo mount -t nfs -o vers=4.1,hard,ro 192.0.2.11:/srv/collector-fixture/inbox /mnt/collector-fixture/inbox
sudo mount -t nfs -o vers=4.1,hard,rw 192.0.2.11:/srv/collector-fixture/probe /mnt/collector-fixture/probe
findmnt -T /mnt/collector-fixture/archive
```

Run the probe with numeric UID/GID 10001 and supplemental GID 65538 (for example,
Linux `setpriv --reuid=10001 --regid=10001 --groups=65538`), using the separate
`probe` export so the application archive stays empty for fresh setup. Retain
client-local SSD state separately. For outage cases, stop/start `nfs-server` only
on that fixture VM, then repeat with a fixture server reboot and a fixture client
reboot. Save the actual observed waits/errors, unchanged underlying mountpoint
contents and recovery results. An application or service-manager stop timeout is
not proof of released NFS ownership. Do not reuse fixture credentials or route
any fixture into production ingress.

Run the small probe only after preparing an empty disposable directory containing
`.collector-nfs-qualification-only` with exactly
`DISPOSABLE COLLECTOR NFS QUALIFICATION ONLY` and a trailing newline:

```sh
python3 scripts/qualify_archive_filesystem.py \
  --root /mnt/collector-fixture/probe \
  --source 192.0.2.11:/srv/collector-fixture/probe
```

It refuses any other entries and mutates only a uniquely created child. It does
not qualify server power-loss durability, outages or application recovery by itself.

**Approve a later migration only when:** exact-source checks pass; Linux/container
and actual-stack isolated NFS restart/atomicity/durability tests have recorded
results; Fi has measured SSD headroom and numeric access (including local Docker volume storage);
effective boot/restart
fences pass absent/wrong-mount tests; old-host writer/CLI owners are fenced; a complete
paired recovery point and demonstrated restore/rollback exist; and the reviewed
Fi listener addresses, retained hostnames/identity and real login/import/playback
acceptance plan are ready. Docker/NFS/Fi execution unqualified in the current session
is an open gate, not permission to migrate.
