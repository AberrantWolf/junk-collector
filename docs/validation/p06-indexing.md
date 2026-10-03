# P06 — Catalog startup and write performance

Date: 2026-10-03. User-prioritized local performance delivery; R02 deployment
qualification remains separate. P05 backup/export UI remains paused. This delivery
establishes the projection restore contract required by P06 without claiming P05
completion.

## Delivered behavior

- Completed publications maintain metadata file observations and seal the catalog
  checkpoint after projection commit, before the durable journal is removed.
- Warm startup compares the canonical archive path, checkpoint format, projection
  epoch and complete metadata/migration/snapshot file inventory. It stats files;
  it does not reread records or media, rehash snapshots, or revisit completed
  migration manifests. Media payload directories are excluded.
- SQLite quick_check and required projection/trigger presence checks precede
  reuse. Missing/damaged/incomplete catalogs rebuild; damaged files are retained
  for diagnosis. Private accounts, grants and workflows are not replaced.
- Interrupted publications, stale or malformed checkpoints, and changed/restored
  archives cause an atomic full rebuild. External archive changes are not
  selectively replayed. Explicit reindex retains full integrity checks.
- Ordinary item/work/edition/capture writes resolve indexed ownership and reference
  neighborhoods, then apply existing domain graph validation. Asset subject,
  artwork hydration and representation package lookups use the same indexes.
  Whole-collection owner reassignment still validates the complete collection.

Filesystem observations are change hints, not integrity evidence or catalog
verification. Unix observations include size, mtime, device, inode and ctime;
other targets use size/mtime. A successful publication updates only its own
artifact observations, not a full archive inventory on every write. Projection,
FTS and reference-index changes advance the epoch in SQLite transactions.
Projection semantic changes must advance the catalog version/checkpoint format.

## Recovery and correctness evidence

`collector-store/tests/archive.rs` includes permanent tests for:

- completed-write restart reuse and equivalence with explicit full rebuild;
- deleted projection rows invalidating reuse and restoring authorized results;
- absent/unsupported checkpoint, missing ownership/item tables and missing
  reference triggers rebuilding rather than silently omitting records;
- corrupt SQLite preservation with existing private authorization surviving;
- restoring an older archive with a newer local projection, and malformed
  metadata failing closed;
- corrupt prepared-audio bytes allowing checkpoint startup while explicit
  integrity reindex still rejects them and leaves raw masters unchanged.

The existing publication fault and actual process-kill matrices exercise journal
recovery through the new startup decision. Existing cross-collection identity,
physical ancestry, asset subject, metadata authority/history, unknown-module,
authorization, and targeted-versus-full-reindex tests exercise indexed validation.
The fixture schema expectation advances from catalog v8 to v9 because P06 adds
rebuildable tables/triggers; immutable archived fixture bytes remain unchanged.

## Reproducible generated measurements

Run:

```sh
cargo test -p collector-store --test archive checkpoint_generated_scale_measurements -- --ignored --nocapture
```

The ignored experiment generates valid immutable schema-3 archives in temporary
profiles. It checks exact projected record counts, performs a full rebuild,
measures 30 authorized inventory edits and 30 authorized 50-item browse requests,
then measures five Store opens against the maintained catalog and asserts
reuse. Each size has its own archive/private profile. No owned data or network
provider is used.

Host: local macOS arm64 filesystem, Rust 1.95.0, unoptimized test profile with the
workspace's SHA-256 optimization. Hardware model/memory were unavailable under
the tool sandbox. Timings are elapsed wall time; p50/p95 use sorted sample rank.
Startup includes store recovery/private reconciliation but excludes compilation,
HTTP listener startup and UI load. Fixtures contain wishlist album metadata with
one initial revision; the edited album gains 30 revisions. They do not model media
size, rich capture graphs, provider history or NAS latency. No timing threshold is
asserted in ordinary tests.

| Items | Full rebuild ms | Startup p50/p95 ms | Edit p50/p95 ms | Browse p50/p95 ms |
| ---: | ---: | ---: | ---: | ---: |
| 100 | 64.0 | 22.5 / 24.3 | 102.6 / 162.0 | 12.0 / 19.5 |
| 1,000 | 789.1 | 81.4 / 109.7 | 95.8 / 146.9 | 14.7 / 21.2 |
| 10,000 | 19,231.2 | 805.2 / 846.1 | 83.7 / 132.6 | 21.8 / 28.7 |

An earlier run recorded 10,000-item startup p50 856.1 ms and edit p50 87.4 ms;
the repeated run above was concurrent with browser smoke work and has visibly
noisier edit tails. Neither run establishes a controlled hardware performance gate.

Warm startup still scales with filesystem metadata entries and SQLite size; it is
not constant-time. Full rebuild remains expensive and is retained as the recovery
path. These results do not qualify million-record collections, the owned corpus,
NAS hardware, or R02's release performance targets.

## Validation gates

Passed: architecture/documentation/fixture checks, format check, workspace tests,
strict workspace Clippy, TypeScript check, web build, rebuilt embedded-assets server
and diff whitespace check. The scale experiment passed twice; the focused
prepared-media/reuse regression passed on the final implementation.

Browser smoke initially could not launch Chrome inside the sandbox. The first
outside-sandbox run reached concurrent playback acceptance but timed out waiting
for playback. The rerun passed on Chrome 153.0.8010.37, including actual FLAC start,
pause/resume, seek, next/previous, natural end advance, reload, grant revocation,
playback while preparation continued, metadata, inventory, typed collection,
artwork, authorization, WebAuthn and mobile layout. The sandbox-launch failure
and earlier concurrent-playback timeout are retained here rather than hidden.
The final payload-tree pruning refinement passed focused checkpoint tests and
strict Clippy/server rebuild; the workspace gate above preceded that refinement.
