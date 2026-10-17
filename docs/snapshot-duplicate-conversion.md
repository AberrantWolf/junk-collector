# Stopped-server snapshot payload conversion

AccurateRip, MusicBrainz and Cover Art Archive refreshes retain one snapshot for equivalent bytes,
inputs and parsed outcomes. Older redundant payloads require an explicit conversion;
normal serving never compacts existing snapshots. Historical IDs, JSON envelopes,
evidence, accepted metadata and audio-verification dates remain unchanged.

Stop Collector and every process using either root. The CLI acquires the same
exclusive archive and private-state locks as the server. Run these commands with
the installation's configuration and a backup destination **outside both roots**:

```sh
collector-server --config /path/collector.toml snapshot-duplicate-backup --destination /backup/collector-before-conversion
collector-server --config /path/collector.toml inspect-snapshot-duplicates > /backup/reviewed-snapshot-plan.json
collector-server --config /path/collector.toml apply-snapshot-duplicates --plan /backup/reviewed-snapshot-plan.json --backup /backup/collector-before-conversion
```

The first command copies the complete archive, private state and configuration,
including credentials, grants, journals, unknown files and unresolved work. It
checkpoints SQLite, writes a length/digest inventory and a completion checksum,
then verifies copied files and private database contents. An incomplete backup
cannot authorize conversion. Keep the backup securely: it contains credentials.
Review the exact duplicate/canonical IDs, collections, bytes saved and skipped IDs
in the plan. `saved_bytes` counts removed payload bytes; `reference_bytes` counts the exact added sidecar bytes. Tiny or empty responses can therefore increase logical file size. Directory, journal and database overhead are additional; physical disk allocation depends on the filesystem. Inputs must be provably equivalent; old records without sufficient
recorded acquisition inputs are skipped. Unknown record fields, unfinished Jobs
and unresolved publication journals are skipped. Corrupt payloads refuse conversion.
The private database and portable files must still match the backup at first apply.
Make a new backup and inspect again if either root changes before applying.

For each duplicate, apply verifies original and canonical bytes and interpretations,
publishes a checksummed `payload/reference.v1.json`, syncs and verifies that reference,
then removes only `payload/source.bin`. The canonical response remains a normal
payload. Chains and cross-collection references are refused. Reads, reindex and
archive-only rebuild resolve the portable sidecar, preserving historical identities.
The reported savings are payload bytes removed; sidecars add a small filesystem cost.

Apply keeps a durable receipt in `state/snapshot-conversions`. If interrupted, keep
the server stopped and rerun the **same plan and backup**. Each completed file step
is verified and safe to repeat. A changed/corrupt journal, reference, canonical
payload or backup fails closed; do not delete a journal to bypass that refusal.

To restore instead, keep the server stopped. Move the current archive and state
roots aside without deleting them. Copy the backup's `archive` and `state`
directories into the original, now-empty root locations, preserving permissions;
restore `collector.toml` if required. Verify every file against `inventory.v1.json`
and its `complete.sha256` before starting. Restore both roots together: copying
only the archive cannot restore credentials, grants or unfinished command receipts.
The backup is the pre-conversion installation; archive-only rebuild preserves
collection evidence but deliberately cannot reconstruct private operational history.
Never restore by overlaying a backup onto nonempty roots.

Development validation uses temporary installations and offline provider fixtures.
No conversion has been run against the user's live archive.

Unknown snapshot-directory members (including extra payload or revision files)
make a record ineligible. Applying a plan rechecks this layout before removing any
payload. A complete temporary reference left by an interrupted conversion is
validated against the journal mapping before cleanup; inconsistent contents require
restoration rather than being silently discarded.


The 2026-10-17 [bounded retention audit](retention-audit.md) extends this same
conversion to CAA listings/images/negative responses. It does not delete Snapshot
envelopes, Assets or authority history. New automatic cover Assets are item-owned,
so repeated requests by different curators reuse the same item's materialization;
two items sharing the same image response still have distinct Assets. Existing
requester-derived Assets are retained. Other providers are outside this conversion.


The 2026-10-18 review repair also permits explicit image selection to restore an
archived provider cover through a new Asset revision. Its tombstone and original
bytes remain preserved; automatic refresh does not restore archived images. This
is an ordinary authorized metadata workflow, separate from stopped-server payload
conversion. See [repair validation](validation/retention-review-2026-10-18.md).
