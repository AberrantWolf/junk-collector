# HL-09 opt-in inbox cleanup — 2026-10-21

Source delivery tested with temporary local installations; later native-container
and isolated NAS acceptance is linked below. Production services, real inboxes
and OS trust were not changed. The [operator guide](../inbox-cleanup.md)
defines opt-in, processing timing, writable mounts, recovery and limits.

## Focused source checks

Six Store/application regressions exercise real preservation and filesystem changes:

- Sources stay intact after import; unconfirmed cleanup is refused. Explicit cleanup
  retains archive downloads and unimported housekeeping. Completed command replay
  does not resolve new configuration or delete a replacement source package.
- Changed bytes, a corrupt archive master, a new meaningful file and a symlink
  prevent deletion. A fresh explicit retry can accept restored source observations;
  generic Job retry cannot bypass checks.
- Faults after quarantine rename, after unlink/before acknowledgment, and after
  directory removal recover with a cold Store. Replacement packages at the original
  source name remain unchanged.
- An unfinished running Job resumes after cold open without a new request. An old
  claim cannot act afterward. Cancellation inside validation retains the package.
- Pending inbox imports prevent cleanup; cleanup fences new parent admission in
  the same transaction. Cancellation and loss of curation authority retain sources.

The new endpoint participates in the existing public/private curation composition
test. Private schema 21 adds operational records only. Reads/hashing are bounded
and checkpointed; frozen observation admission has a 4 MiB metadata budget.

## Actual Chrome journey

Chrome 155.0.8059.85 exercised actual setup, inbox import, grouped/manual title
review, artwork and restart using offline provider fixtures. The new cleanup journey
opens the real Work & history detail: removal is disabled until confirmation,
changed-source cleanup fails visibly, restoring the bytes permits retry, and reload
shows completion. Archive download bytes still match; housekeeping is retained.
Server restart and receipt replay leave a newly created package at the old name
untouched. A 390-pixel viewport exercises the recovery UI without page overflow.
No physical Safari or system-trust qualification is inferred from this.

Self-review also caught the local-account recovery schema ceiling; it now shares
the Store's current supported private version and its recovery test passes. Two
incidental hardcoded version assertions were removed from upgrade tests; their
actual record, access, unknown-data and restart assertions remain.

Self-review found and fixed prior cleanup failures incorrectly keeping Work & history
failed after a successful new attempt. Cancelled cleanup no longer cancels the
already-preserved import. Active cleanup uses typed indeterminate progress; source
removal counts are separate from validation. Optional metadata/preparation failures
remain visible and can retry from the archive after source cleanup.

## Packaging and limits

Actual Compose rendering preserves default read-only inbox mounts and supplies
read/write mounts to both server and CLI only with the explicit environment option.
Both native AMD64/ARM64 Docker jobs and the isolated actual-NAS Portainer upgrade
passed the real read-only refusal, write-enabled retry and preserved-download drill.
See [public alpha.3 publication and full recovery](container-publication-2026-10-21.md).
The final actual Chrome journey passed. Architecture/documentation/fixture checks,
format, API generation/drift check, strict all-target Clippy, TypeScript check,
web build, container tooling and release-tooling checks passed. The server was
rebuilt with current embedded assets. `cargo test --workspace` passed: 454 tests, zero failures and four existing
opt-in qualification tests ignored. This source milestone used the pinned Rust
1.99.0; the next release separately qualifies its current-stable baseline.
No large-library throughput, hostile concurrent writer or power-loss claim is made.
