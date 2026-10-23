# Server operations

[← README](../README.md) · [Docker updates](../deploy/container/README.md#update) · [Configuration](configuration.md)

## Operations and backups

The examples below use the built binary; place `--config /absolute/path/collector.toml` before any command when needed.

| Command | Purpose |
|---|---|
| `setup [--username NAME]` | Initialize storage, TLS, the default account, and local browser configuration. Run while stopped. |
| `serve` | Run listeners and background workers. Stop with Ctrl-C or SIGTERM on Unix. |
| `doctor` | Check configuration and optional encoders without locking the archive; can run while serving. Does not test provider reachability, passwords, grants, or browser certificate trust. |
| `reindex` | Rebuild the catalog from archive records. Stop the server first. |
| `backup --destination PATH` | Lock the stopped installation, settle journals/checkpoint SQLite and write a verified complete backup to a new directory. |
| `export-archive --destination PATH` | Copy a stopped installation's portable archive to a new directory, excluding accounts/configuration/private work. |
| `validate-archive-export --export PATH` | Validate the complete portable file inventory and every checksum without configuration or databases. |
| `import-archive --export PATH --archive PATH` | Copy a validated export into an empty archive; use fresh setup and explicit Owner recovery afterward. |
| `validate-backup --backup PATH` | Check the sealed inventory, complete file set and every digest without opening databases or current configuration. |
| `restore --backup PATH --archive PATH --state PATH [--archive-only]` | Validate and copy into empty roots. `--config` names a new output configuration. Archive-only recovery requires a fresh identity and explicit grants. |
| `unowned-collections [--limit 50] [--cursor ID]` | List recovered collections with no Owner grant. Run while stopped. |
| `recover-collection-owner --collection-id ID --username NAME` | Explicitly assign the first Owner to an enabled account while stopped. Refuses replacing any existing Owner; repeating the same assignment leaves its revision unchanged. |
| `pairing` | Export server identity, public certificate, and endpoint data as JSON. Transfer it through a trusted channel; it contains no login credential and does not establish browser trust. Run while stopped. |
| `paths` | Show absolute configuration, account-storage, archive and public-certificate locations with existence status. Safe while serving; reads configuration only. |
| `accounts [--limit 50] [--cursor USERNAME]` | List sign-in usernames and enabled/admin/default status. Run while stopped. |
| `recover-account --username NAME` | Recover that sign-in username while stopped. Reads a new password, revokes browsers/apps/pending approvals/codes/passkeys, and preserves grants and enabled/admin/default status. |

These commands use `collector.toml` by default; select another configuration with `--config FILE` before the command. Account storage is the folder containing `private.sqlite` (normally `.collector/state`). Run `paths` to see its configured location. If configuration is damaged, `accounts --state PATH` and `recover-account --username NAME --state PATH` use that existing folder directly. They never create missing storage or open the archive. Recovery takes the username used for sign-in; no account ID is required.

The [account delivery ledger](account-usability-delivery.md) documents schema 19, preserved credentials and the native account-keyed conversion.

### Full backup

For a fresh installation without transferred credentials or grants, use
[portable full archive export/import](archive-export.md). Full backup below
also preserves operational identity, access and pending work.

Stop the server, then run:

```sh
collector-server --config /absolute/path/collector.toml backup --destination /backup/new-collector-backup
collector-server validate-backup --backup /backup/new-collector-backup
```

The destination must not exist; its parent must exist outside both storage roots.
Protect it like live private state. The completion marker is written last; failed
or interrupted backups must be retried at a fresh destination. Restart the source
only when the backup command has exited.

The state root includes pending uploads and work as well as SQLite databases and TLS keys. Do not copy live SQLite files as an ordinary filesystem backup. Keep inbox originals and external DAT source files in your backup plan too; those paths are outside the server roots.

Restore with the original server stopped:

```sh
collector-server --config /restore/collector.toml restore --backup /backup/new-collector-backup --archive /restore/archive --state /restore/state
```

The two roots must be absent or empty, their parents must exist, and the output
configuration must not exist. Review reported external paths and run `doctor`
before starting the restored server. Retire the original instance: full restore
preserves identity, accounts, grants and TLS keys, so never run both copies.
Interrupted roots are fenced from startup; retry with newly empty roots.
For a deliberate independent clone, use `--archive-only`, then `setup` for a fresh
identity. While still stopped, use `unowned-collections` to find its collection IDs,
then `recover-collection-owner --collection-id ID --username NAME` for each
collection you intend to claim. Sign in as that Owner to share access normally.
Existing Owners and disabled accounts cannot be bypassed through this command.
Missing original lockfiles are recreated when their writable directories survive;
live original locks still refuse restore. See the complete
[operator runbook](local-alpha-installation.md#stopped-server-backup).
