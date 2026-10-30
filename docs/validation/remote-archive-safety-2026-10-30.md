# Remote archive safety — 2026-10-30

Prepared on `codex/remote-archive-safety` in the separate worktree, based on
`0ed9a017519dc713fd9c82cc85a159af0dd28d25`. Existing uncommitted work in the original
checkout was not incorporated or changed. No production storage, deployment,
exports/permissions, router or tunnels were accessed or changed.

[Architecture](../architecture.md) and the
[split deployment/recovery runbook](../../deploy/container/remote-archive.md)
record the boundaries, existing guarantees, new admission rules, interruption
policy and exact migration readiness criteria. A NAS snapshot alone no longer
captures the installation. Complete local state/config, including **all imports**,
must be paired with the entire NAS archive under a stopped-writer fence.

## Implemented delivery

- Normal executable startup/maintenance requires an enrolled archive/local-state
  pair. Explicit setup supports fresh storage; stopped-server enrollment adopts
  retained pre-guard data without opening/migrating its databases.
- Archive UUID, filesystem/source/options, root identity, required directories and
  private layout checks precede normal recovery. Missing enrolled private SQLite
  cannot silently become a fresh identity. Detected archive failures latch the
  process unavailable; publication phases and database/writer admission are fenced.
- Linux archive and enrollment mutation paths anchor to an open directory
  descriptor. Error-aware artifact presence checks preserve I/O errors during
  publication, migration/reindex, compaction and recovery. Staging/journals remain
  on the archive filesystem; local encoding/import sources remain local.
- Split host binds cover core, media and CLI. The remote overlay disables Docker
  automatic restarts and the root initializer, keeps media read-only and waits for
  core's authority socket/key before media's first key read. Host preflight and
  systemd templates gate the intended archive/inbox mounts and local private roots,
  including Caddy's certificate/config directories.
- The runbook supplies stopped-writer paired points, full local-state/key capture,
  reconnect/restart, rollback after new writes and restoration acceptance. It does
  not require routine full-media copying/hashing or authorize import cleanup.

## Validation results

Environment: macOS/aarch64, Rust 1.99.1, Node 24.3.0, Python 3.14.6. Development
executables only; no deployment executable/image was published or installed.
The maintained selector was previewed and selected **full/infra** coverage for
storage/recovery and deployment changes. Checks were completed across the broad
run and focused corrections below; no failed monolithic receipt is relabeled as
an uninterrupted pass.

| Check | Result |
| --- | --- |
| Architecture, concrete documentation links and fixtures | Passed, including final documentation |
| `cargo fmt --all -- --check` | Passed |
| `cargo test --workspace --locked` | **551 passed, 5 marked ignored**, zero failures on the accepted test run |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed after moving unchanged runtime declarations before test modules |
| Web type checking/build | Passed; no generated web asset changes |
| Web queue/context/presentation models | All three passed |
| Maintained testing policy | 12 tests passed, rerun after registering the new journey |
| Container tooling/preflight/probe fixtures | 12 tests passed on final configuration; synthetic mount tables and owned local probe data only |
| Release-policy fixtures | 6 passed |
| Container-validation fixtures | 11 passed, one rootless-skopeo case skipped because its prerequisite was unavailable |
| Browser journeys | **All 29 passed**, including the new `storage` journey |
| Effective remote Compose model | Rendered with all four overlays and tools profile; split paths, groups, restart policy, read-only media, disabled initializer and healthy-core media dependency checked |
| CLI help / source diff | New command/option inspected; complete tracked/untracked diff and whitespace reviewed |

The five Rust ignored cases are the user-owned CD oracle, generated bulk/metadata
scale benchmarks, local processing benchmark and a subprocess helper that its
parent remote-media test invokes. They are not new NFS qualification. Existing
native/HTTP/backup tests cover account/grant identity, accepted authority, unknown
records, source retention and committed recovery. Independent phase-fault and
SIGKILL tests remain intact.

New isolated tests cover missing versus empty mountpoint refusal without writes,
wrong UUID, missing collection tree without empty catalog replacement, missing
private SQLite, redirected private databases/import roots, remote policy rejecting
local directories, unfinished Job completion refusal and retained import sources.
A real import loses archive identity at every publication phase and resumes with
stable Capture/Representation identity, one evidence record and unchanged bytes.
Loss immediately after encoding retains the master and original durable request,
refuses startup while unavailable and completes that same Job after restart.

The `storage` browser journey uses the shared harness and its own temporary roots.
It creates a collection through the real UI, retains a manual title, detaches only
that fixture archive, observes a visible typed HTTP 503 on refresh, checks mobile
wrapping, verifies reconnect remains latched, proves missing-root startup creates
nothing, then restarts and displays the same entry/title. Ordinary integration
also exercises real FLAC/MP3 playback, separate media origins, grants and recovery.

### Corrections and interrupted checks

Early runs exposed fixtures relying on implicit adoption of old private state or
switching archives. They now explicitly enroll retained data. The private-lock
case reopens the same bound archive and independently rejects switching it after
lock release. Corrupt-journal/unknown-staging cases initialize enrolled fixtures
before injecting artifacts, preserving their actual recovery assertions.

The journal-root corruption case now verifies unavailable admission, a latched
process, no command receipt and successful metadata publication/backup only after
restoring storage and restarting. Its old expectation of immediate in-process
continuation would contradict the new guard. A first revision used a phase fault
with a different error family; the final case tests the real unavailable-root
condition instead. It passes in isolation and in the full workspace run.

An earlier validation run was interrupted to include final enrollment safeguards.
The accepted workspace run then passed all enabled Rust tests. Clippy identified
only declaration ordering after a test module; runtime code was moved unchanged,
Clippy/format were rerun and subsequent browser acceptance used the rebuilt final
binaries. Later changes were deployment preflight/configuration and test coverage;
those received their focused tooling/model checks, without repeating unchanged
Rust algorithms. This is development evidence, not a reusable release-image receipt.

The broad browser run passed 20 journeys before a local fixture port collision
stopped `game-catalog`. That failure was retained as infrastructure failure, never
counted as a pass. The ports were confirmed released without stopping unrelated
processes; `game-catalog` and all seven remaining journeys passed on rerun. The
new storage journey separately passed. All browser launches used the supported
execution context outside macOS seatbelt.

## Unqualified limits and approval gate

Docker daemon was unavailable. No container runtime/systemd boot, automatic-restart
failure test, actual Linux anchored-directory execution, NFS mount/disconnect,
server durability test or coordinated Fi/NAS restore was performed. The filesystem
probe's unit test uses synthetic NFS admission over disposable local files; it is
**not** evidence of NFS behavior. Actual NFS qualification uses the separate
marked disposable export and VM procedure in the runbook, never production paths.

Hard NFS calls may block requests, workers and shutdown until service returns;
there is no promised userspace deadline. Existing active-work shutdown follow-up
remains open. Archive locks are not qualified cross-host writer fencing. Host
preflight owns inbox source admission; manual core/CLI startup is not a supported
bypass for that service owner. Docker's data root/named private volumes need an
actual local-SSD check; preflight does not relocate Docker storage.

**A later Fi migration remains unapproved until:**

1. A qualified release image/digest for core and media is available and its actual
   container/Linux filesystem, first-boot and missing/wrong-mount restart tests pass.
2. Isolated tests on the intended Fi/NAS stack establish required NFS atomicity,
   synchronization, disconnect/server-restart recovery and numeric UID/GID access,
   with media unable to write and inbox independently mounted.
3. Fi has measured SSD capacity/headroom for the complete 76-GB private tree,
   concurrent work and recovery points; SQLite/config/Caddy/Docker private storage
   are confirmed local. No `state/imports` exclusion/deletion is assumed.
4. The old NAS writer, CLI schedules and other restart owners are fenced; the
   effective host unit, Compose model and existing container restart policies
   enforce one writer and refuse absent/wrong mounts across host/daemon reboot.
5. Both members of a complete stopped-writer recovery point exist, and an isolated
   paired restore demonstrates credentials/grants/manual data, original/derivative
   reads and pending-work resumption, including deliberate rollback after new writes.
6. Fi-specific listener addresses (including the old `192.168.1.106:9845` bind),
   retained public hostnames/authentication origin/TLS/ServerId and the real
   sign-in/import/playback cutover acceptance plan have been reviewed.
