# HL-03 stopped-server backup and restore validation — 2026-10-18

Scope: temporary installations on the development Mac. No user archive was
modified. This is executable CLI/recovery evidence, not Linux filesystem,
power-loss, physical-device, online-backup or portable-export qualification.
The [runbook](../local-alpha-installation.md#stopped-server-backup) describes the
operator workflow and one-active-instance responsibility.

## Contract and exercised behavior

- CLI `backup` refuses a live installation and requires existing storage/config;
  `validate-backup` works independently of current configuration and never opens
  SQLite. `restore` refuses live original roots, nonempty destinations, symlink
  aliases, overlapping backup/root paths and an existing output configuration.
- A complete backup includes archive/private state, TLS/config, unknown regular
  files, empty directories and pending scratch. Every copied file has a length and
  SHA-256 digest. A synced completion marker seals the bounded inventory last.
  Symlinks/special files are refused. Files/directories are private on Unix.
- Full restore compares every copied byte with the backup before normal Store
  opening. The integration drill checks exact pre-open trees, original payload,
  evidence, manual metadata authority, active loan/private inventory, unsupported
  schema payload, original ServerId, both accounts and Viewer grant. The same
  partially uploaded Import resumes from its recorded two-byte prefix.
- Archive-only recovery copies no private database, uses a different ServerId,
  starts with no accounts/grants and requires an explicit new Owner grant.
  Unknown data and manual authority survive reindex in that new identity.
- A separate compiled-CLI drill checks TLS bytes, identity/account login,
  retained external inbox/encoder paths, relocated configuration, `0700` directory
  and `0600` file permissions, and refusal before creating roots for corrupt input.
- Faults before backup completion and after restore preparation, copied bytes and
  configuration publication leave no usable partial result. Startup detects restore
  markers before touching even an intentionally invalid database. A fresh-root
  retry succeeds. Both original root locks are tested independently.
- Missing, changed, unlisted and unsafe inventory paths, extra directories and
  symlink entries fail validation. Metadata reads/traversal are bounded; file copies
  stream and refuse changing lengths. Snapshot conversion shares these mechanisms.

## Initial checks

| Check | Result |
|---|---|
| `python3 scripts/check_architecture.py` | Passed, including documentation links |
| `cargo fmt --all -- --check`; `git diff --check` | Passed |
| `cargo test --workspace` | Passed, including doctests; archive integration: 225 passed, 3 existing manual qualifications ignored; owned-disc oracle remains separately ignored |
| Final `collector-store` backup unit rerun | 4 passed, including account-recovery fencing and read-only validation of unchanged bytes |
| Final `collector-store` stopped backup/restore integration rerun | Passed, including exact pre-open trees, resumption and independent identity/grants |
| Final compiled `collector-server --test backups` rerun | Passed, including full restore and archive-only restore followed by real setup |
| `cargo clippy --workspace --all-targets -- -D warnings` | Passed |
| `npm --prefix web run check`; `npm --prefix web run build` | Passed |
| `npm --prefix web run check:api` | Passed; no HTTP contract changes |
| `cargo build -p collector-server` after web build | Passed |

The focused reruns include the final file-set and parent-directory durability
checks. Full-workspace log: `/private/tmp/hl03-final-workspace-tests.log`.
Final focused logs: `/private/tmp/hl03-final-recovery-unit.log`,
`/private/tmp/hl03-final-workflow.log`, `/private/tmp/hl03-final-cli.log`.
Earlier exploratory workspace runs interrupted while finishing the implementation
are not acceptance results. Browser smoke was not repeated for this CLI-only
delivery; the preceding HL-01 [actual Chrome smoke record](retention-review-2026-10-18.md)
is preserved. No manual scale, owned-disc or actual-host qualification is newly
claimed by the automated suite.

## Self-review repairs — 2026-10-18

The initial suite missed two operator failures. Archive-only validation assigned
an Owner directly through `Store::grant`, bypassing the application's existing
Owner requirement; the documented Administration step could not bootstrap access.
A relocated full restore also failed when original directories survived empty,
because their lost lockfiles were required to exist. Both were reproduced in
`/private/tmp/hl03-self-review-2026-10-18.log` before repair. Those initial passing
checks alone were not acceptance of the complete operator workflow.

`unowned-collections --limit … --cursor …` now discovers ownerless collections
while stopped. `recover-collection-owner --collection-id … --username …` applies
the first explicit Owner grant to an existing enabled account. Application policy
refuses replacing any existing Owner, including a disabled one. The storage
transaction evaluates that policy and shares the normal grant effect, then writes
a private recovery audit event in the same commit. Repeating the same assignment
does not advance its revision or append another audit event. Ordinary authenticated
grant endpoints still require an Owner. The earlier integration drill now calls
the recovery application service rather than using the storage grant shortcut.

Restore now creates/acquires a missing lockfile in a writable surviving original
root. Existing original lockfiles are still locked read-only; backup lockfiles
are required, read-only inputs. A missing sibling lockfile does not permit bypassing
a live lock. Missing original roots stay absent, and existing original records
are neither opened nor changed by this fencing step.

The compiled-CLI regressions in `crates/collector-server/tests/backups.rs` exercise:

- `archive_only_cli_recovers_first_owner_and_preserves_normal_access_policy`:
  real backup → archive-only restore → setup → paged discovery → Owner assignment.
  The new account initially cannot see collections or grant itself ownership through
  ordinary application policy. Local recovery makes its collection visible and
  permits normal Owner sharing; the Viewer still cannot promote itself. Live locks,
  missing accounts/collections, disabled targets, replacement of an existing Owner
  and invalid page limits are denied. Repeated recovery preserves the grant revision
  and other collections remain unowned.
- `restore_cli_recovers_with_missing_original_lockfiles_and_still_refuses_live_sources`:
  real full backups restore with emptied original roots or either original lockfile
  lost. Existing private database bytes and the backup remain unchanged. Holding the
  other original lock still refuses restore before destination creation. Source lock
  symlinks are refused without changing their targets. Full restored identity is
  preserved, and the backup still passes validation.

Checks after review repairs:

| Check | Result |
|---|---|
| `python3 scripts/check_architecture.py` | Passed, including updated record/runbook links |
| `cargo fmt --all -- --check`; `git diff --check` | Passed |
| `cargo test --workspace` | Passed, including doctests, the 3 compiled backup/recovery CLI tests and 225 archive integration tests; 3 existing manual archive qualifications and the separate owned-disc oracle remain ignored |
| `cargo clippy --workspace --all-targets -- -D warnings` | Passed |
| `npm --prefix web run check`; `npm --prefix web run build` | Passed |
| `npm --prefix web run check:api` | Passed; no new HTTP routes or contract changes |
| `cargo build -p collector-server` after web build | Passed |

Final workspace log: `/private/tmp/hl03-review-fix-workspace-tests.log`.
Clippy/build/API logs: `/private/tmp/hl03-review-fix-clippy.log`,
`/private/tmp/hl03-review-fix-server-build.log`, `/private/tmp/hl03-review-fix-api.log`.
The focused CLI run is `/private/tmp/hl03-review-fix-cli.log`; the subsequent
workspace run also includes the final source-lock alias assertions. No live user
installation was changed. Actual-host and
Linux/NAS qualification remain HL-04; the browser-smoke scope/limits above remain
unchanged.

## Limits

Local locks cannot fence an old instance on another host. Full restore operators
must retire it before starting the recovered identity. External inboxes, original
DAT files and encoder executables are reported, not copied or silently rewritten;
frozen work can require their old paths or explicit workflow retry. Archive-only
recovery still requires a complete valid backup; damaged-private-state salvage is
not implemented. Historical conversion inventories preserve files but did not list
empty directories. No online backup, scheduler, dashboard, export or delta format
was introduced. The historical LA3 physical installation drill remains dated in
the runbook; these temporary tests do not rerun its device/provider qualification.
