# Installation, backup, and recovery

For Docker hosts, start with the [container installation guide](../deploy/container/README.md).
It includes external HTTPS, initial account setup, updates, and container-specific
recovery commands. For native builds, use the [source setup](getting-started.md#build-from-source).
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
on the installation as the only operational catalog. Keep source originals; choose
any independent media copies according to the [clarified backup scope](decisions/home-lab-scope.md#local-recovery-and-optional-media-backups--clarified-2026-10-20). These checks establish the exercised behavior; they do not
prove catalog/audio accuracy, power-loss safety, or complete Internet security.

## Optional backup-age reminder

After updating to source that includes this feature, add this section to the
server configuration and restart:

```toml
[backup_reminder]
after_days = 30
```

Administration → Server settings and `doctor` then show a recorded full-backup
completion date and recent/due/failed/unconfirmed/unknown status. Only a successful
final full-backup validation advances the receipt. A later failed attempt retains
the previous completion; interruption remains unconfirmed. Older backups have no
receipt and remain unknown. The reminder does not discover destinations, recheck
backup existence or validate its bytes. Use `validate-backup` for an explicit
check; that command does not advance a backup-creation receipt. It never
schedules a backup, hashes media during status reads, or creates
cloud traffic. Remove the section to disable reminders.

For backups managed outside Collector, explicitly report their dates instead:

```toml
[backup_reminder]
after_days = 30
externally_managed = true
external_completed_at = "2026-10-01T00:00:00Z"
# external_failed_at = "2026-10-02T00:00:00Z"
```

These are operator reports; Collector does not observe or validate external
backups. A later failure is shown even when a previous completion exists. Future
or malformed local history never becomes a recent-success claim. A restored full
backup contains its captured pre-completion receipt, so it can correctly show an
unconfirmed attempt until another full backup completes. Local receipts do not
prove independent storage, current backup presence or a successful restore drill.
The existing explicit full backup still includes media; this adds no lightweight
catalog/settings-only backup mode.

## Stopped-server backup

To transfer only portable collection data into a fresh installation, see
[archive export/import](archive-export.md). The full recovery workflow below also
preserves accounts, access, server identity and unfinished work.

The packaged container initializes a local `${COLLECTOR_DATA}/backups` area;
its CLI uses explicit fresh children under `/data/backups`. Native installations
choose an explicit destination outside archive/state. These are manual full backups,
including media. No automatic full-library backup, cloud account or independent
storage destination is required. See the [container local backup guidance](../deploy/container/README.md#local-backup-area-and-optional-media-copies).

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
