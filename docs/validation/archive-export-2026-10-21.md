# HL-10 full portable archive export — 2026-10-21

Source delivery on the development Mac, using temporary local installations only.
The published alpha.2 image and live NAS installation were not changed. This closes
the standalone full-export/import delivery; deltas, merging and Retro Junk adoption
remain outside it. See the [operator workflow](../archive-export.md).

## Completed workflow

The actual compiled CLI exports a stopped installation, validates with a missing
current configuration, and imports into a fresh archive. The source can resume
ordinary writes after export while the frozen copy is imported. A later source
list is absent from the imported collection.

The existing synthetic schema-1 corpus supplies two revisions, two original payloads,
artwork, manual authority, meaningful evidence, physical inventory and lexical
unknown module/provider JSON. The source is opened through the normal migration
and indexing path. The fixture also adds an unknown root record, an empty directory
and a current collection list with repeated entries. Every inventoried file in the
import matches the export byte for byte. Cold startup and explicit reindex preserve
the Item, evidence and list records, including occurrence IDs and order.

Original accounts, grants, TLS key and unfinished upload scratch are absent from
the portable copy. Fresh setup creates a different server identity and TLS key;
the old credentials fail and the new administrator initially has no collection
grant. Local discovery/first-Owner recovery restores authorized collection access.
The source and export remain usable and unchanged by import. Export through a
second process refuses a live source before creating a destination.

## Focused failure coverage

- Interrupted export cannot produce a valid completion seal. Retry uses a fresh
  destination; it cannot overlay the failed attempt.
- Import interruption before copying, after a file, and before completion leaves
  the startup fence. Store startup refuses before creating a private database;
  retry into a new root succeeds.
- Same-length source modification after initial import validation is detected by
  the per-copy digest check and leaves the startup fence.
- Changed, missing and extra files, extra top-level private data, traversal,
  operational paths, separators and unsafe names are refused. Validation remains
  read-only. Nonempty unknown staging is retained and prevents export.
- Source symlinks, destination aliases into source storage, export/destination
  overlap and populated destinations are refused. Existing bytes remain intact.
  Unix export modes are private (`0700` directories, `0600` files).

The shared backup/export copy mechanism hashes the bounded source stream and
compares its digest with the synced destination, rather than inventing an expected
digest solely from the copied output. Existing backup/restore regressions remain
part of the workspace checks.

## Validation scope

Focused Store interruption/integrity tests and the actual CLI journey pass. Final
repository gates pass: architecture/document/fixture checks, formatting, workspace
tests (448 passed; four existing opt-in qualification cases ignored), strict Clippy,
web type checking and build, and rebuilt Rust server. Served web assets are
unchanged. Self-review checked frozen copy ownership, private-state exclusion,
bounded reads, completion ordering and the startup fence; it strengthened the shared
copy mechanism to compare the source-stream digest with the saved bytes. No browser
workflow changed; this is a local operator CLI feature. No new browser, phone,
physical-device, NAS, multi-terabyte throughput or format-verification claim is made.
Byte preservation and checksum validation do not verify catalog identity or the
audio content of a disc. External writers modifying storage outside Collector's
locks and hostile replacement of an entire unsigned export are not supported
concurrency/authenticity guarantees.
