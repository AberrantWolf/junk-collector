# Remote archive safety review and integration — 2026-10-30

Reviewed the remote archive preparation against current `main` (`62fec3b`),
including the outstanding digital matching/retry fixes. The preparation is
preserved as `4e446dd`; main was integrated into its review branch before testing.
No live deployment, NAS, exports/permissions, router or tunnels were touched.

## Findings fixed

1. UUID/binding writes could expose a truncated file after interruption. They now
   sync a private temporary and publish without replacement, then sync its parent.
   Enrollment retries also sync existing identities. The isolated filesystem probe
   now checks the no-replace hard-link fallback used by identity publication.
2. A crash after binding but before SQLite creation could prevent fresh setup
   from resuming. A durable UUID-bound bootstrap intent now permits **explicit**
   setup recovery on either side of binding publication. Every store opener retires
   the intent before exposing writable state. Missing established private state
   remains a refusal. Retained unbound imports/scratch without their database are
   rejected before archive creation or identity publication.
3. SQLite accepts a zero-byte file as a fresh database. An established empty
   `private.sqlite` is now refused rather than silently initialized. A matching
   unfinished bootstrap intent is the only exception; tests cover refusal through
   normal startup, setup and enrollment after truncating a real account database.
4. A deny-list could miss an unfamiliar remote private filesystem. Split mode now
   accepts only known local persistent filesystem types and rejects unknown/FUSE,
   network and ephemeral backends. Host deployment policy remains stricter.
5. Paired recovery instructions now explicitly drain pending writeback, enumerate
   private bind paths outside `COLLECTOR_DATA`, preserve the authority key alongside
   read-only snapshots, and account for restored point-in-time security state
   before public ingress. Post-point passwords/grants/revocations need review.

## Recovery and repeat-review evidence

The new paired stopped-copy fixture creates a point with credentials, an unchanged
server identity, queued Job/timestamp, retained import source and opaque archive
bytes. After later account/archive writes and Job completion, restoring both old
members into fresh roots resumes the original Job. The complete newer pair remains
intact and independently reopenable for roll-forward. No duplicate live identity is
served. This is a local isolated recovery test, not a Fi/NAS snapshot qualification.

Repeated review traced mount admission, Linux descriptor lifetime, publication
rename/receipt gaps, journal abort/reconciliation, identity durability, setup
interruption, private-state loss and rollback ownership. After the fixes above,
the final review found no further actionable issue in those paths. Real NFS and
container/host qualification remain explicit migration gates, not inferred passes.

## Validation

- Maintained full/infra selection: architecture/link checks, format, web type/build,
  testing/release/container tooling, **555 passing workspace tests**, five marked
  ignored, Clippy and web models. No Rust test failed in that broad run.
- Final refinements received **26 passing store unit tests and eight passing
  storage/recovery integration tests**, including bootstrap, empty SQLite, local
  filesystem policy and paired rollback. Final workspace Clippy passed with
  `--all-targets --locked -- -D warnings`. Browser binaries were rebuilt from the
  final runtime code. No unit-test-only declaration ordering change was treated
  as a change to a storage algorithm.
- All **29 configured browser journeys passed** across the broad run and its
  completion run, including real setup/import, original/FLAC/MP3 playback,
  publication recovery, metadata/manual authority, separate media, and storage
  unavailable/reconnect/restart. Assertions and deadlines were not weakened.
- Two FX-05 attempts timed out before replacement because the WAV clock did not
  advance, despite the player showing Pause. The unchanged fixture later passed
  in an explicit Google Chrome run (18.63 s), including uninterrupted playback,
  reimport, removal and restoration. The two failures remain recorded; no root
  cause was established, and they are not relabeled as passes or attributed to
  a different browser engine. Other actual-playback journeys also passed.
- Final offline container/preflight/probe tests: 12 passed. Effective remote
  Compose model was checked during preparation; no daemon was available here.
  Formatting, whitespace, links and tracked/untracked changes were re-reviewed.

The full run's raw log is `/tmp/remote-review-full.log`; final storage and Clippy
logs are `/tmp/remote-review-final-storage.log` and
`/tmp/remote-review-final-clippy.log`; browser completion is
`/tmp/remote-review-chrome.log`. These are development evidence, not a release-image
qualification receipt. The original dated [preparation evidence](remote-archive-safety-2026-10-30.md)
is retained, including its ignored/skipped cases and platform limits.

## Integration and remaining gates

All pre-existing Collector local/tracked remote branches were ancestors of main;
there were no open pull requests or uncommitted changes in the primary checkout.
The clean, already-merged beta.9 temporary worktree was removed. The reviewed
remote-archive branch is integrated into main, with the temporary review checkout
closed after preserving its commits. No unrelated work is discarded.

The [migration runbook](../../deploy/container/remote-archive.md) still requires
actual Linux/NFS stable-write/disconnect/server-restart qualification, effective
container/systemd boot fences, local SSD capacity and numeric access, one writer,
and a coordinated Fi/NAS restore before approving migration. Hard NFS calls can
still block until service returns. No migration, release or deployment is approved
or performed by this merge.
