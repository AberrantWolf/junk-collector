# Local recovery snapshots and optional incremental exports

This is an optional Btrfs operations path over the existing stopped-server recovery
contract. It is separate from Collector's sealed full-backup format: do not pass a
tar increment to `validate-backup` or `restore --backup`. The full-copy
[backup/update](README.md#update) and [restore](recovery.md) remain available.
Qualification status and exact limits are recorded in
[the actual-host recovery report](../../docs/validation/snapshot-recovery-2026-10-20.md).
Use the local snapshot section for update rollback. The export/independent-copy
sections are optional for operators who choose to back up media elsewhere; no cloud
account or independent destination is required. These commands do not configure a
schedule or change a live installation automatically. The packaged local full-backup
area is `${COLLECTOR_DATA}/backups`; full backups include media and require an explicit
fresh child destination. Snapshot points belong outside the live data root.

## Eligibility and policy

On the inspected DS1522+, `/volume1/junk-collector` is one Btrfs subvolume,
`@syno/junk-collector`, with archive, complete private state and config as ordinary
children and no nested subvolumes. Read-only inspection established this without
changing production. Snapshot Replication/Hyper Backup were not installed. No DSM
package installation or conversion of the existing data layout is required by the
qualified command-line path.

For another installation, establish these facts first with administrator access:

```sh
sudo /sbin/btrfs subvolume show /absolute/collector-data
sudo /sbin/btrfs subvolume list -o /absolute/collector-data
```

All authoritative roots, pending uploads/scratch and journals must be inside that
one subvolume. Resolve absolute config paths, symlinks and additional mounts; the
shipped container config uses `/data/{archive,state}`. Stop if required data is
outside it, a nested subvolume is present, or the backend is SMB/NFS/non-Btrfs.
Snapshots do not recursively capture nested subvolumes. External inboxes, DAT/tool
paths and optional external tools need their separately recorded recovery policy;
saved originals must already be in the archive. Keep snapshots/exports **outside**
the live data root on the same Btrfs filesystem, with private administrator access.

Use local read-only snapshots for quick rollback. If choosing independent media
backups, keep a full baseline and every required incremental parent on the selected
other storage. A same-NAS snapshot/export cannot protect against loss of that NAS.
Reserve space, retain the last accepted recovery point/chain, and test restoration
and integrity according to the selected policy. GNU tar's incremental selection uses metadata, not proof of unchanged bytes;
clock rollback or unreliable/preserved timestamps require a fresh full baseline.

Record the exact old image ID/registry digest, config layout, ServerId, checkpoint
cutoff, parent point and upgrade/storage notes with each point. Ordinary application
changes may use this short snapshot policy; incompatible storage writes/conversions
require a validated complete recovery point and the old image before proceeding.
Never downgrade only the image against later incompatible data.

## Short stopped-server checkpoint

Choose new absolute paths/names; the following values are examples. Match the CLI
image to the running Collector's immutable image ID as described in the runbook.
Use Portainer to stop **only** Collector, or `docker stop YOUR_COLLECTOR_CONTAINER`
when deliberately operating that service. Wait for it to exit; do not snapshot a
running SQLite/private-state tree. Keep every other process opening these roots
stopped until the read-only snapshot completes.

```sh
collector_data=/volume1/junk-collector
collector_recovery=/volume1/collector-recovery
collector_point=before-update-UNIQUE
sudo install -d -m 0700 "$collector_recovery"
collector_cutoff_ns=$(python3 -c 'import time; print(time.time_ns())')
sudo /sbin/btrfs subvolume snapshot -r "$collector_data" "$collector_recovery/$collector_point"
```

Capture the cutoff **before** snapshot creation, on the NAS clock. Record it with
the checkpoint. If snapshot creation fails, retain the old installation/image and
resolve the failure. Once it succeeds, the selected service can restart/redeploy
with its retained mounts/config/groups; exporting the immutable snapshot can happen
while it serves. This step neither copies nor hashes the masters. Capture one
complete point before each update, even when independent backup export is deferred.

## Optional: full baseline once, then incremental exports

Use GNU tar from the retained Collector Debian image, rather than macOS/BSD tar.
`--no-check-device` prevents a different snapshot device number from forcing a full
copy. Use the same relative roots on every export. The helper corrects GNU tar's
cutoff to the filesystem snapshot time: without that correction, an existing file
changed between capture and export could be omitted from the next increment.
[GNU tar documents format 2 and its cutoff fields](https://www.gnu.org/software/tar/manual/html_node/Snapshot-Files.html).
The helper rejects unsupported formats, invalid/later cutoffs, aliases and published
files. It only changes a private `.part` working copy.

Create a **new** private export directory, e.g.
`$collector_recovery/exports/$collector_point.part`, with mode 0700. For the first
baseline, `state.snar.part` must not exist. For an increment, copy the previous
**accepted** point's `state.snar` to this new directory as `state.snar.part`.
Never overwrite the previous point or advance it after a failed export.
Set `collector_export` to this new directory and `collector_old_image` to the retained
immutable image ID, then run. For example, create the new working directory with:

```sh
collector_export="$collector_recovery/exports/$collector_point.part"
collector_old_image=sha256:REPLACE_WITH_RETAINED_IMAGE_ID
sudo install -d -m 0700 "$collector_recovery/exports"
sudo mkdir -m 0700 "$collector_export"
# For an increment only: copy the accepted parent's state.snar to state.snar.part here.
```

A pre-existing working directory is a refusal, not permission to reuse its `.snar`.
Run the export:

```sh
docker run --rm --user 0:0 --network none --read-only \
  -v "$collector_recovery/$collector_point:/snapshot:ro" \
  -v "$collector_export:/backup" \
  --entrypoint sh "$collector_old_image" \
  -c 'umask 077; exec tar "$@"' sh \
  --create --file=/backup/data.tar.part \
  --listed-incremental=/backup/state.snar.part \
  --no-check-device --numeric-owner --acls --xattrs \
  --directory=/snapshot config archive state

sudo python3 freeze-tar-time.py "$collector_export/state.snar.part" "$collector_cutoff_ns"
```

Abort on any nonzero exit. Before publication, record parent/image/layout/cutoff
notes; rename the two `.part` files to `data.tar` and `state.snar`; compute a
`SHA256SUMS` over the new archive and metadata. These hashes read the new export,
not unchanged masters. Atomically rename the completed **directory** from
`$collector_point.part` to `$collector_point` only after all outputs succeed.
Interrupted directories remain incomplete; retry from the last accepted parent in
a new working directory. Keep its source snapshot until the selected export/restore acceptance is complete.

Copy the completed point and all prerequisite parents using an authenticated,
encrypted transport to the chosen independent destination. Copy into a private
`.part` directory there, compare `SHA256SUMS`, and publish its directory only after
success. Retain the cutoff/parent/image notes. Backups contain accounts and TLS
private keys as well as collection data, so destination access must remain private.
The `.snar` is needed to create the next increment; restoring needs the base and
all increments in order. A missing/corrupt increment breaks that chain: use the
last complete chain or a new full baseline, never silently skip it.

## Full restore and interrupted-update rollback

Keep the current server stopped, even if its copy is on another host. Use the
checkpoint's exact image. Retain current data for diagnosis; recover into new empty
roots, with the recorded absolute/container config layout. Verify every independent
archive's digest and parent/order before extraction. Use only trusted self-created
backups and a private destination. GNU incremental extraction applies deletions;
never extract an incremental chain over the live installation.

Create the new empty host root before binding it (DSM requirement). Mount that root
at `/recovered` and each backup point read-only at `/backup`, then execute GNU tar
as root from the retained image for the full baseline followed by every increment:

```sh
tar --extract --listed-incremental=/dev/null --file=/backup/data.tar \
  --numeric-owner --same-owner --acls --xattrs --directory=/recovered
```

These are **container paths**, not commands to run against host `/`.
After extraction, prepare only the recovered outer root and its empty `backups`
directory with UID/GID 10001 and mode 0700. Keep captured child permissions/ownership.
Set the one-shot CLI and stopped server to that recovered root, run doctor, then
start exactly one copy of the recovered ServerId. Check account/session/grants,
config/TLS identity, pending work/resume, inventory/manual authority, original bytes
and playback as applicable. Periodically validate all retained originals, including
unknown data, rather than equating a digest-valid tar with catalog verification.

For immediate local rollback after an interrupted update, keep the updated service
stopped and make a new writable Btrfs snapshot **from the complete old read-only
point** into a new path. Start the old image against that whole recovered point;
do not flip the retained read-only point writable or restore only SQLite/config.
Only switch the real service/data path after recovered-instance acceptance. Do not
run two writable clones of one identity. The isolated qualification exercises this
rollback with a pending upload, plus SIGKILL/restart and independent full restore.

The stock/generic `btrfs receive` incremental route failed through the disposable
bind-mounted filesystem on this DSM host; it is not an accepted alternative here.
No remount of the host filesystem, raw-device access or production snapshot change
was attempted to bypass that failure.
