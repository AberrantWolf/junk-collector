# Removing duplicate provider snapshots

AccurateRip, MusicBrainz and Cover Art Archive refreshes store one snapshot for
equivalent responses. Archives created before that may contain redundant payloads,
which can be converted with a stopped-server command. Normal serving never compacts
snapshots. Snapshot IDs, evidence, accepted metadata and verification dates do not
change.

Stop Collector and every process using either root; the CLI takes the same locks as
the server. Use a backup destination **outside both roots**:

```sh
collector-server --config /path/collector.toml backup --destination /backup/collector-before-conversion
collector-server --config /path/collector.toml inspect-snapshot-duplicates > /backup/reviewed-snapshot-plan.json
collector-server --config /path/collector.toml apply-snapshot-duplicates --plan /backup/reviewed-snapshot-plan.json --backup /backup/collector-before-conversion
```

The backup is a complete, verified copy of archive, private state and configuration
(it contains credentials; keep it secure). Apply refuses an incomplete backup, or one
that no longer matches the installation; if anything changed, back up and inspect again.

Review the plan's duplicate and canonical IDs, collections, saved bytes and skipped
IDs. `saved_bytes` counts removed payload bytes; `reference_bytes` counts the added
sidecar bytes, so tiny responses can grow slightly. Records are skipped when their
equivalence cannot be proven (old records without recorded inputs, unknown fields or
directory members, unfinished jobs, unresolved journals). Corrupt payloads refuse the
conversion.

For each duplicate, apply verifies both payloads, writes a checksummed
`payload/reference.v1.json` pointing at the canonical snapshot, verifies it, then
removes `payload/source.bin`. Chains and cross-collection references are refused.
Reads, reindex and archive-only rebuilds follow the reference.

Progress is recorded in `state/snapshot-conversions`. If apply is interrupted, keep
the server stopped and rerun it with the **same plan and backup**; completed steps are
safe to repeat. A changed or corrupt journal, reference, payload or backup fails
closed; do not delete the journal to get past it.

To undo the conversion, restore the backup into empty roots as described in
[server operations](server-operations.md). Restore archive and state together to keep
credentials, grants and unfinished work.

Explicitly selecting an archived provider cover restores it as a new artwork revision;
automatic refresh never does.
