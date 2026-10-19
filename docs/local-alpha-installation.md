# Installation, backup, and recovery

For Docker hosts, start with the [container installation guide](../deploy/container/README.md).
It includes external HTTPS, initial account setup, updates, and container-specific
recovery commands. For native builds, use the [README](../README.md#quick-start).
This runbook describes storage and the stopped-server CLI workflow for either
installation type. Historical acceptance results live in the
[dated installation record](local-alpha-installation-history.md).

## Before storing a collection

Choose permanent absolute archive/state/configuration paths and run the server
under an ordinary account that can access them. Keep SQLite/private state on a
local filesystem; original archive bytes can use separately qualified NAS storage.
Only one server may write an archive or use a restored server identity.

Configure the browser-facing HTTPS origin and exposure before initial setup.
External proxies/tunnels reach the public listener; private management remains on
explicit private ingress. Run `setup` once, then `doctor`, then `serve`. The setup
password is read through a no-echo prompt or standard input, never a command flag.
Optional provider/encoder failures must not prevent saving original files.
MusicBrainz metadata requests need a monitored contact address in provider
configuration; a MusicBrainz account is not required. Setup does not overwrite
existing accounts or TLS, and initialization does not overwrite existing config.
Update the actual configuration deliberately while stopped.

Before a bulk intake, create a collection, import one complete supported package,
download its originals and compare their digests with the sources. If using music
preparation, play a prepared track and check seeking. Restart and sign in again.
Make a complete backup and test a restore into separate empty roots before relying
on the installation as the only operational catalog. Keep source originals and an
independent backup. These checks establish the exercised behavior; they do not
prove catalog/audio accuracy, power-loss safety, or complete Internet security.

## Stopped-server backup

The backup unit is the complete archive root, complete state root, and configuration
file. The state root includes `private.sqlite`, the rebuildable `catalog.sqlite`,
TLS key/certificate material, resumable Import/upload scratch, derivative scratch,
durable Jobs, accounts, grants and sessions. Do not select individual SQLite, WAL,
scratch or TLS files. Protect the backup as credential-bearing private data.

Stop the server cleanly and wait for it to exit. Run the compiled server binary
from the installation's normal working directory (relative configuration paths
retain their existing meaning):

```sh
collector-server --config /absolute/path/collector.toml backup --destination /backup/new-collector-backup
collector-server validate-backup --backup /backup/new-collector-backup
```

The destination must **not exist**; its parent must exist, outside both storage
roots. The command requires an existing installation, holds both normal server
locks, settles known publication journals through startup recovery, and checkpoints
both SQLite databases before copying. A live lock or unresolved journal refuses
backup. Unknown staging is retained; symlinks and special files are refused rather
than followed. Every regular file and empty directory is included. Neither external
inboxes, external DAT source files nor encoder executables are copied.

`inventory.v1.json` lists relative paths, lengths and SHA-256 digests, original root
locations and a logical private-state digest used by snapshot conversion.
`complete.sha256` seals that inventory and is written/synced last, after verification.
Validation checks the entire file set and bytes without opening any database or
running migrations. New inventories also bind the directory set. Older conversion
backups remain readable, but cannot prove which empty directories existed.
Inventory metadata is bounded to 64 MiB (also enforced during traversal); directory
nesting is at most 256 levels. Large payloads stream through the copy and digest
checks without being loaded into memory.

On Unix, backup directories use mode `0700` and files `0600`. Keep the backup on
trusted storage: it includes account hashes, sessions, credentials and private TLS
keys. The checksum detects corruption, not deliberate replacement by someone who
can rewrite the backup and its seal. A failed/interrupted backup has no valid
completion marker. Keep it for diagnosis or remove that specific failed backup;
retry at a fresh destination. Do not manufacture a completion marker or edit the
inventory to bypass a failed check. `doctor` checks configuration and does not prove
quiescence; only commands acquiring both locks do that.

## Restore drill into empty roots

Keep the original server stopped throughout restoration and validation. Full
restore recovers the same ServerId, accounts, grants, sessions, pending Jobs,
upload scratch and TLS private key. **Retire the original instance before starting
the restored one. Never run both copies, even on different ports.** Restore locks
available original roots on this host and refuses a live original. Local locks
cannot fence an original on another host or an offline disk; retiring it remains
an operator responsibility.

Original directories can survive with their lockfiles lost. Restore recreates and
acquires the normal lock in a writable surviving directory, without opening or
modifying its records. Existing locks are acquired read-only and still refuse a
live writer. Symlink/special lockfiles are refused. Completely absent original
roots stay absent. The backup's own lockfiles remain required, unchanged inputs.

Choose two separate absent or empty roots and a **new** output configuration path;
all parent directories must already exist. Restore never overlays existing data.
If recovering to the original paths, first move both old roots and configuration
aside without deleting them.

```sh
collector-server --config /restore/collector.toml restore --backup /backup/new-collector-backup --archive /restore/archive --state /restore/state
collector-server --config /restore/collector.toml doctor
collector-server --config /restore/collector.toml reindex
collector-server --config /restore/collector.toml serve
```

`--config` names the **output** during restore; no current configuration is read.
The command first validates the backup, then locks both empty destination roots.
It copies and rechecks each selected file, preserving empty directories, and writes
the relocated configuration last. Only `archive` and `state` values change;
other configuration values, including unfamiliar fields, survive serialization.
No restored database is opened/migrated during restore or validation. Subsequent
`reindex`/`serve` may perform normal current-format startup recovery and migrations.
On Unix, copied files/config use `0600` and directories use `0700`.

Review the command's external inbox, DAT source and encoder path report. Those
paths were retained, not relocated or copied. Update them for this host and check
listener addresses, TLS names, allowed origins, endpoints and available disk space.
Frozen pending work may still refer to an old inbox/tool path: restore that path
or explicitly retry/reselect its input through the normal workflow. Do not change
private database rows to make a frozen request silently mean something different.

Interrupted restore leaves `.restore-in-progress` in both roots. Normal startup
refuses either marker **before** opening databases or recovering journals. Preserve
the partial roots/configuration for diagnosis and retry into newly empty roots.
Never remove the markers to start a partial restore. The backup remains unchanged.

For a deliberate independent clone, or recovery of archive history without private
credentials, use the same command with `--archive-only`:

```sh
collector-server --config /independent/collector.toml restore --backup /backup/new-collector-backup --archive /independent/archive --state /independent/state --archive-only
collector-server --config /independent/collector.toml setup --username owner
collector-server --config /independent/collector.toml unowned-collections --limit 50
collector-server --config /independent/collector.toml recover-collection-owner --collection-id COLLECTION_ID --username owner
```

This copies archive history only. Setup creates a **fresh** ServerId, account and
TLS identity. No old grants, sessions, credentials or unfinished private jobs are
copied. `unowned-collections` lists IDs/names and a cursor for additional pages.
Run `recover-collection-owner` for each collection you explicitly choose, naming
an existing enabled account by its sign-in username. Both commands require the
server stopped and acquire both normal process locks. The assignment writes a
private security audit entry and refuses collections with any existing Owner,
including disabled accounts; recover that existing account or use normal owner
controls instead. Repeating the same successful assignment is safe and leaves its
grant revision/audit unchanged. Unknown or unavailable modules do not prevent
recovering access to their retained history.

Then sign in as that Owner and grant other accounts access through the ordinary
collection controls. A fresh server administrator alone cannot perform that first
grant through Administration: server administration does not confer collection
ownership. Archive records never infer grants. Pair clients with the new identity. This mode still
validates the complete backup before copying; salvage of a damaged private backup
is not implemented.

For an acceptance drill, record a partially uploaded Import or pending Job before
stopping. After full restore, sign in with the recorded accounts, confirm roles,
original bytes, loans, unknown data, manual authority, evidence and accepted cover
sources. Resume the same Import/Job identity and retained progress. Stop cleanly
and explain new records created by that resumed work. Do not replace production
roots during a drill. [HL-03 validation](validation/backup-restore-2026-10-18.md)
records temporary-root CLI and recovery tests; the separately retained
[historical installation results](local-alpha-installation-history.md) remain dated evidence.
