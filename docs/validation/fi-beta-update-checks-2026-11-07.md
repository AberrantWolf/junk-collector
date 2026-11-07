# Routine Fi beta update checks — 2026-11-07

The [routine procedure](../../deploy/container/fi-beta-update.md) applies to
image-only upgrades from beta.22 or later. The operator stops writers and seals
the full read-only Fi and NAS snapshots together before changing either image
pin. The checker compares the running stack against its before record and the
stopped Fi state. The NAS snapshot is attested in the operator's pair record.

Local fixtures passed for committed SQLite WAL reads, retained grants and
staged imports, active work refusal, image and mount mismatches, companion
recreation, Bridge report freshness, and session protection on redirects
or changed origins. The packaging check started the helper from the generated
release bundle. Container tooling, release policy, mail, architecture and
format checks passed. The rootless OCI test remained reserved for assembly.
Rust and web behavior use the prior evidence for the same source files.
Live Docker, Btrfs, Fi/NAS, Bridge sync, production TLS and the authenticated
production read remain unqualified for this procedure.
