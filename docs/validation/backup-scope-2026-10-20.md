# Backup scope clarification — 2026-10-20

The user clarified that a multi-terabyte media library should not imply automated
full-media backups, paid cloud storage or a separately chosen independent destination.
A predetermined local backup area and optional backup-age warnings are the intended
operating direction. The earlier destination question and publication blocker were
an overextension, not a missing user requirement.

## Evidence and corrected current guidance

- `deploy/container/initialize.sh` already creates `config`, `archive`, `state` and
  `backups` under the selected data root. The shipped one-shot CLI binds that root
  at `/data`. Its local backup area is therefore `${COLLECTOR_DATA}/backups`, or
  `/volume1/junk-collector/backups` for the inspected NAS layout.
- `collector-server backup` still requires an explicit destination. It creates a
  fresh sealed **full** archive/state/config backup, including originals. No scheduled
  media backup, automatic destination selection or lightweight database/catalog-only
  mode is implemented by the existing CLI or this clarification.
- The archive carries inventory, manual metadata and meaningful evidence; SQLite
  alone is not the complete catalog. A future small backup mode needs an explicit
  restore contract before it can claim to recover those facts and required assets,
  private state and config.
- Local consistent Btrfs points remain the qualified cheap update-rollback path.
  Independent GNU tar exports/other copies remain optional operator policy. Complete
  local recovery, honest storage notes and restore acceptance remain required;
  external storage/cloud selection is removed as a release/update prerequisite.
- Backup-age reminders are an optional unimplemented follow-up in the single queue.
  Failed attempts must not reset a success date, externally managed backups must
  remain unknown unless observed, and reminder queries must avoid payload scans.

The scope decision, architecture/status, release requirements, active remaining-work
entry/assignment, alpha.2 draft notes and operator guides were updated together.
Existing dated NAS/snapshot qualification reports remain historical evidence; their
successful optional independent-storage drill is retained without becoming a mandate.

## Validation and limits

Architecture/documentation-link/fixture checks, six release-policy/bundle regression
tests and `git diff --check` passed. Self-review checked initializer/CLI/mount behavior
against the documented paths and searched current guidance for the obsolete
independent-destination blocker. This is a documentation/scope correction; runtime
behavior and accepted backup formats are unchanged, so Rust/browser suites were not
repeated. No production file, backup, schedule, destination or cloud account was
created or modified.
