# Portable full archive export

HL-10's full export/import workflow is included in public **alpha.3**, 2026-10-21;
see [publication acceptance](validation/container-publication-2026-10-21.md). The
[local acceptance record](validation/archive-export-2026-10-21.md) describes the
tested commands and recovery cases.

Use this to transfer preserved collection data to a fresh installation. It keeps
originals, artwork, inventory, manual metadata, collection lists, recipes, evidence,
source snapshots, history and unknown archive records byte for byte. It copies the
whole portable archive, including empty directories. It does not merge collections
into an existing archive, prune history, convert a legacy product, or make a delta.

Accounts, grants, passwords, sessions, native credentials, TLS, server identity,
private pending work and configuration are not copied. Private collection facts,
receipts, notes and original files **are** collection data and can be included.
Treat the export as private. This is not a redacted sharing feature or a complete
installation recovery point; use [full backup/restore](local-alpha-installation.md#stopped-server-backup)
when you need credentials, access assignments and unfinished work preserved.

## Export and check

Stop the source server. Choose a new destination whose parent already exists,
outside both configured storage roots, with room for a full copy.

```sh
collector-server --config /source/collector.toml export-archive --destination /transfer/collection-export
collector-server validate-archive-export --export /transfer/collection-export
```

The command refuses a running installation. Startup first reconciles known archive
journals; remaining nonempty staging/journals require inspection before export.
They are not silently discarded. Source archive files remain unchanged. Once the
export command exits, the source can resume normal use; later writes are excluded
from this export.

The export contains `archive/`, `archive-export.v1.json` and `complete.sha256`.
The inventory contains relative names, lengths and SHA-256 values, with no source
machine paths. The completion marker seals that inventory and is written last.
Copying streams and checks bytes without buffering whole media files; the inventory
is bounded to 64 MiB and directory nesting to 256 levels. Unix directories use
`0700` and files `0600`. Symlinks/special files and unsafe names are refused.

Validation checks the complete file/directory set and every file's length/digest.
It reads no configuration and opens no database. Checksums detect corruption, not
malicious replacement of both the data and its inventory. An interrupted export
has no valid completion seal; keep it for inspection or remove that specific
incomplete copy, and retry into a new directory. Never overlay an earlier attempt.

## Import into a fresh installation

Choose an empty or absent destination archive with an existing parent:

```sh
collector-server import-archive --export /transfer/collection-export --archive /new-install/archive
```

Import validates before creating the destination, checks copied bytes again and
does not change the export. It reads no source configuration and creates no private
state or configuration. The original installation may continue running: the imported
archive will receive a fresh operational identity, not a clone of its credentials.

Create `/new-install/collector.toml` with the imported archive and a new, separate
state directory, plus listener/HTTPS settings appropriate to this installation:

```toml
archive = "/new-install/archive"
state = "/new-install/state"
```

Then use the ordinary setup and explicit access-recovery commands:

```sh
collector-server --config /new-install/collector.toml setup --username owner
collector-server --config /new-install/collector.toml reindex
collector-server --config /new-install/collector.toml unowned-collections
collector-server --config /new-install/collector.toml recover-collection-owner --collection-id COLLECTION_ID --username owner
collector-server --config /new-install/collector.toml doctor
```

Setup reads a new password and creates fresh accounts, server identity and TLS.
Archive records are indexed during startup; explicit reindex can check that rebuild
again. Repeat first-Owner recovery for each desired collection, then sign in and
share through the normal controls. Review listener addresses, allowed origins and
TLS names before serving; fresh private TLS still needs the normal trust setup.

An interrupted import leaves `.restore-in-progress`; normal startup refuses that
archive before opening a database. Retry into a newly empty archive. Do not remove
the marker to start an incomplete copy. Failed or existing destinations are never
overlaid, and neither command removes originals, exports or recovery inputs.
