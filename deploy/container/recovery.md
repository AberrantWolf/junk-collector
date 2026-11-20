# Restore a full backup

This restores a backup made with `cli backup`. For Btrfs snapshots or tar
increments see [snapshot-recovery.md](snapshot-recovery.md); for an NFS archive
split see [remote-archive.md](remote-archive.md).

Use the image that made the backup, and restore archive, state and config together
into a fresh data directory. Do not just downgrade the image against upgraded data.

Stop the current server and keep its data and `.env` for diagnosis:

```sh
docker compose stop collector
```

In `.env`, set `COLLECTOR_IMAGE` to the earlier image and `COLLECTOR_DATA` to a
**new, empty** directory, e.g. `/volume1/junk-collector-recovered`. Do not run the
initializer or setup: the backup supplies config, accounts and TLS identity. The
old instance must stay stopped, including on other hosts.

Create the directory and its empty children:

```sh
mkdir -p /volume1/junk-collector-recovered
docker compose run --rm --entrypoint install initialize \
  -d -m 0700 -o 10001 -g 10001 \
  /data/config /data/archive /data/state /data/backups
```

Validate and restore from the backup directory (outside the new data directory):

```sh
docker compose run --rm \
  -v /absolute/path/to/backups/before-update-UNIQUE:/backup:ro \
  cli validate-backup --backup /backup

docker compose run --rm \
  -v /absolute/path/to/backups/before-update-UNIQUE:/backup:ro \
  cli restore --backup /backup --archive /data/archive --state /data/state

docker compose run --rm cli doctor
docker compose up -d collector
```

Restore writes the config file last. Stop if any command fails; an interrupted
restore leaves markers that prevent serving partial data, so retry into a new empty
directory rather than over it. Afterward check login, collection access, an
original download and playback. Keep the backup until you are satisfied. Only one
copy of a restored server may run.

For a Portainer-managed installation, keep its Collector service stopped and use
Compose only for these one-shot commands.

If the restored image predates `enroll-archive`, run the newer image's
`enroll-archive` against the restored data before starting that newer image
(see [update](README.md#update)).
