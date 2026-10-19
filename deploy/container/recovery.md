# Recover a previous container installation

Use the full backup made before updating and the exact image used to create it.
Restore archive, state and configuration together into a fresh data directory;
do not just downgrade an image against upgraded state.

From the installation directory, stop the current service:

```sh
docker compose stop collector
```

Retain the current data and `.env` for diagnosis. Edit `.env` to set
`COLLECTOR_IMAGE` to the earlier image and `COLLECTOR_DATA` to a **new, empty**
absolute directory, e.g. `/volume1/junk-collector-recovered`. Do not start the
service yet, and do not run the default initializer or setup: restore supplies the configuration
and account/TLS identity restored from the backup. The old instance must remain stopped even
if it lives on another host. Local locks cannot enforce that across hosts or
when old paths are hidden by different mounts.

Create only the destination directories, without creating a config file:

```sh
docker compose run --rm --entrypoint install initialize \
  -d -m 0700 -o 10001 -g 10001 \
  /data/config /data/archive /data/state /data/backups
```

Replace `/absolute/path/to/old/backups/before-update-UNIQUE` below with the
actual sealed backup directory, outside the new data directory:

```sh
docker compose run --rm \
  -v /absolute/path/to/old/backups/before-update-UNIQUE:/backup:ro \
  cli validate-backup --backup /backup

docker compose run --rm \
  -v /absolute/path/to/old/backups/before-update-UNIQUE:/backup:ro \
  cli restore --backup /backup --archive /data/archive --state /data/state

docker compose run --rm cli doctor
docker compose up -d collector
```

The CLI entrypoint supplies `--config /data/config/collector.toml`; restore writes
that file last. Abort if any command fails. An interrupted restore leaves markers
that prevent serving partial data; retry using a newly empty destination, not an
overlay. After success check fresh login, collection access, original download
and playback as applicable. Retain the source/backup until acceptance is complete.
Only one copy of a restored server identity may run.
