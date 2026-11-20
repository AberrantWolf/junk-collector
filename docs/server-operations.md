# Server operations

[← README](../README.md) · [Docker installation](../deploy/container/README.md) · [Configuration](configuration.md)

For Docker, start with the [container guide](../deploy/container/README.md); the
commands below work in either installation.

## Commands

Commands read `collector.toml` by default; put `--config FILE` before the command to
use another. "Stopped" means the server must not be running.

| Command | Purpose |
|---|---|
| `setup [--username NAME]` | Create storage, TLS identity, the first account and local browser config. Stopped. |
| `serve` | Run listeners and workers. Stop with Ctrl-C or SIGTERM. |
| `doctor` | Check configuration and encoders. Safe while serving. |
| `paths` | Show configuration, state, archive and certificate locations. Safe while serving. |
| `reindex` | Rebuild the catalog from the archive. Stopped. |
| `backup --destination PATH` | Write a verified full backup to a new directory. Stopped. |
| `validate-backup --backup PATH` | Check a backup's files and digests. |
| `restore --backup PATH --archive PATH --state PATH [--archive-only]` | Restore into empty directories; `--config` names the new config file. |
| `export-archive --destination PATH` | Export collections and originals without accounts or config. Stopped. |
| `validate-archive-export --export PATH` | Check an export's files and digests. |
| `import-archive --export PATH --archive PATH` | Import an export into an empty archive. See [archive export](archive-export.md). |
| `accounts [--limit 50] [--cursor NAME] [--state PATH]` | List accounts. Stopped. |
| `recover-account --username NAME [--state PATH]` | Reset a password (read from stdin). Stopped. |
| `unowned-collections [--limit 50] [--cursor ID]` | List collections with no Owner. Stopped. |
| `recover-collection-owner --collection-id ID --username NAME` | Give an ownerless collection its first Owner. Stopped. |
| `pairing` | Print server identity, public certificate and endpoints as JSON for native clients. |
| `enroll-archive` | Bind an existing archive to this state directory. Stopped. See [NFS archive](configuration.md#nfs-archive). |
| `inspect-collection-type --collection-id ID` | Show whether a legacy collection can be given a type. |
| `assign-collection-type --collection-id ID --expected-revision N --module MODULE` | Assign it. Unknown data is kept. Stopped. |
| `inspect-snapshot-duplicates` / `apply-snapshot-duplicates --plan PATH --backup PATH` | [Deduplicate provider snapshots](snapshot-duplicate-conversion.md). Stopped. |

## Before relying on an installation

- Choose permanent absolute paths for archive, state and config, and run the server as an
  ordinary user. Keep state on local disk; the archive may be on a NAS.
- Set the HTTPS origin and exposure before `setup`. `setup` never overwrites existing
  accounts, TLS or configuration.
- Import one package, download its originals and compare digests with the source; play
  a track and seek; restart and sign in again.
- Make a full backup and test restoring it into separate empty directories.

## Account recovery

With the server stopped, `accounts` lists usernames and `recover-account --username NAME`
reads a new password from stdin. Recovery signs out all browsers and apps and removes
pending approvals, recovery codes and passkeys; roles and admin status are kept. If the
config is damaged, pass `--state PATH` (the directory containing `private.sqlite`).

## Full backup

A full backup contains the whole archive, the whole state directory (databases, TLS
keys, accounts, sessions, pending uploads and jobs) and the configuration. It includes
media. It does not include inboxes, external DAT files or encoder binaries; back those
up separately. Never copy live SQLite files by hand.

Stop the server, wait for it to exit, then:

```sh
collector-server --config /absolute/path/collector.toml backup --destination /backup/new-collector-backup
collector-server validate-backup --backup /backup/new-collector-backup
```

- The destination must not exist; its parent must exist outside both storage roots.
- The backup holds both server locks, settles pending publications and checkpoints
  SQLite first. A running server or unresolved journal makes it refuse.
- `inventory.v1.json` lists every file with its SHA-256; `complete.sha256` seals it and
  is written last. A backup without it is incomplete: retry to a new destination and
  never create the marker by hand.
- Files are `0600` and directories `0700`. Protect the backup like live state: it
  contains password hashes, sessions and the TLS private key.

In the container, backups go to fresh directories under `/data/backups`; see the
[container guide](../deploy/container/README.md).

## Restore

Keep the original server stopped. A full restore brings back the same server identity,
accounts, sessions, pending jobs and TLS key, so **retire the original before starting
the restored copy and never run both**, even on different ports. Restore refuses a live
original on the same host but cannot see one on another host.

Choose two absent or empty directories and a new config path whose parents exist. To
restore over the original paths, move the old directories and config aside first.

```sh
collector-server --config /restore/collector.toml restore --backup /backup/new-collector-backup --archive /restore/archive --state /restore/state
collector-server --config /restore/collector.toml doctor
collector-server --config /restore/collector.toml serve
```

Restore validates the whole backup, copies and rechecks each file, and writes the new
config last with only `archive` and `state` changed. It reports inbox, DAT and encoder
paths that it did not relocate; fix them for the new host, along with listeners, TLS
names, origins and endpoints. Pending work that refers to an old inbox needs that path
restored or the import retried.

An interrupted restore leaves `.restore-in-progress` in both directories and the server
refuses to start on them. Retry into fresh empty directories; do not delete the marker.

### Archive-only restore

For an independent copy, or to recover collections without the private state:

```sh
collector-server --config /independent/collector.toml restore --backup /backup/new-collector-backup --archive /independent/archive --state /independent/state --archive-only
collector-server --config /independent/collector.toml setup --username owner
collector-server --config /independent/collector.toml unowned-collections
collector-server --config /independent/collector.toml recover-collection-owner --collection-id COLLECTION_ID --username owner
```

This copies archive history only; `setup` creates a new server identity, account and
certificate. Assign an Owner to each collection you want, then share access through the
normal collection controls (an administrator does not get collection access
automatically). `recover-collection-owner` refuses collections that already have an
Owner, including a disabled one. Re-pair native clients afterwards.

To move collections to a fresh installation without accounts, use
[archive export/import](archive-export.md) instead.
