# Btrfs snapshots and incremental exports

An optional alternative to full backups on Btrfs: take a read-only snapshot of the
stopped data directory before each update, and optionally export tar increments
to other storage. This is separate from Collector's full-backup format: do not pass
a tar export to `validate-backup` or `restore --backup`. For an NFS archive split
use [remote-archive.md](remote-archive.md) instead.

## Eligibility

```sh
sudo /sbin/btrfs subvolume show /absolute/collector-data
sudo /sbin/btrfs subvolume list -o /absolute/collector-data
```

Archive, state and config (everything under the shipped `/data/{archive,state,config}`
layout) must be inside that one subvolume, with no nested subvolumes, symlinks or
extra mounts pointing elsewhere. Snapshots do not include nested subvolumes. Inboxes
and external tools need their own backup policy. Keep snapshots and exports outside
the live data directory, on the same filesystem, with administrator-only access.

A snapshot on the same NAS does not protect against losing the NAS. With each
point, record the image ID, ServerId, cutoff time and parent point. Never downgrade
only the image against newer data.

## Take a snapshot

Stop Collector (in Portainer, or `docker stop YOUR_COLLECTOR_CONTAINER`) and wait
for it to exit. Keep anything else that opens the data stopped until the snapshot
completes.

```sh
collector_data=/volume1/junk-collector
collector_recovery=/volume1/collector-recovery
collector_point=before-update-UNIQUE
sudo install -d -m 0700 "$collector_recovery"
collector_cutoff_ns=$(python3 -c 'import time; print(time.time_ns())')
sudo /sbin/btrfs subvolume snapshot -r "$collector_data" "$collector_recovery/$collector_point"
```

Capture the cutoff before creating the snapshot and record it. Once the snapshot
succeeds the server can restart; exporting can happen while it runs.

## Optional: incremental exports

Use GNU tar from the Collector image (not BSD tar). Create a new private working
directory per export. For the first full export, `state.snar.part` must not exist;
for an increment, copy the previous accepted export's `state.snar` into the new
directory as `state.snar.part`. Never reuse or overwrite an earlier export.

```sh
collector_export="$collector_recovery/exports/$collector_point.part"
collector_old_image=sha256:REPLACE_WITH_RETAINED_IMAGE_ID
sudo install -d -m 0700 "$collector_recovery/exports"
sudo mkdir -m 0700 "$collector_export"
# Increment only: copy the parent's state.snar to "$collector_export/state.snar.part".

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

`freeze-tar-time.py` sets the `.snar` cutoff to the snapshot time, so files changed
between snapshot and export are not missed by the next increment.

Stop on any error. Then rename the two `.part` files to `data.tar` and `state.snar`,
write a `SHA256SUMS`, and rename the directory to drop `.part`. A failed export
stays incomplete; retry from the last good parent in a new directory.

Copy completed exports and all their parents over an encrypted channel to the
other storage, into a `.part` directory, check `SHA256SUMS`, then rename. Exports
contain accounts and TLS private keys. Restoring needs the full export and every
increment in order; if one is missing, use the last complete chain or start a new
full export. Tar selects changes by timestamps, so after a clock rollback start a
new full export.

## Restore

Keep the current server stopped. Use the point's image and restore into new empty
directories; never extract an incremental chain over a live installation (it
applies deletions).

**Quick rollback:** make a writable snapshot from the read-only point into a new
path and start the old image on it. Do not make the retained point itself writable,
and do not restore only SQLite or config.

**From exports:** verify each export's `SHA256SUMS` and order. Create the empty
target directory, mount it at `/recovered` and each export read-only at `/backup`
in a root container using the old image, then extract the full export followed by
each increment:

```sh
tar --extract --listed-incremental=/dev/null --file=/backup/data.tar \
  --numeric-owner --same-owner --acls --xattrs --directory=/recovered
```

Give the recovered root and an empty `backups` directory UID/GID 10001 and mode 0700.
Point the CLI and server at it, run `doctor`, and start exactly one copy. Check
login, grants, pending work, items, original bytes and playback.

`btrfs receive` failed on DSM in testing and is not a supported alternative.
