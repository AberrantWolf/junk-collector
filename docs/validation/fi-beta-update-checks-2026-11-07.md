# Routine beta update checks — 2026-11-07

The [routine procedure](../../deploy/container/beta-update.md) applies to
image-only upgrades from beta.22 or later. The operator stops writers and seals
the full read-only application and archive host snapshots together before changing either image
pin. The checker compares the running stack against its before record and the
stopped application host state. The NAS snapshot is attested in the operator's pair record.

The review follow-up adds a stopped cutoff command before activation. It
refuses an active unit, running Compose containers, or unfinished work at the
snapshot, then records the snapshot UUID and state fingerprints. The after
check requires this receipt. Bridge startup has a bounded wait with separate
timeout and interrupted-sync results. Focused local fixtures passed for these
paths.

Local fixtures passed for committed SQLite WAL reads, retained grants and
staged imports, active work refusal, image and mount mismatches, companion
recreation, Bridge report freshness, and session protection on redirects
or changed origins. The packaging check started the helper from the generated
release bundle. Container tooling, release policy, mail, architecture and
format checks passed. The rootless OCI test remained reserved for assembly.
Rust and web behavior use the prior evidence for the same source files.
Live Docker, Btrfs, both hosts, Bridge sync, production TLS and the authenticated
production read remain unqualified for this procedure.
