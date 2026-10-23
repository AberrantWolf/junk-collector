# Optional backup-age reminder acceptance — 2026-10-21

Post-alpha.3 source; published images and production are unchanged. The operator
opts in with `[backup_reminder] after_days = 30` (1–3650 days), optionally selecting
explicit external completion/failure reports. Administrator Server settings and
`doctor` display typed recent/due/failed/unconfirmed/unknown status. Configuration
removal disables the notice. No scheduler, media backup or cloud service is added.

Local full-backup attempts write one atomic private completion receipt. The start
is recorded before copying; only successful final validation advances completion.
A failed attempt preserves the previous consistent completion. Abrupt interruption
stays unconfirmed. Missing/corrupt/oversize/future/unsupported history stays unknown.
An outcome-receipt persistence failure cannot invalidate an already validated
backup; the CLI logs it and status remains based on the available receipt. The
receipt contains no destination, key or credential. Status reads at most 4097
bytes from one regular file, without opening databases or scanning/hashing media.

External dates are operator reports, visibly labelled unobserved/unvalidated. A
later reported failure takes precedence over a prior completion. Future reports
do not appear as recent success. Local receipts establish a recorded completion,
not present backup availability, independent storage or restore acceptance. Old
backups have no receipt. A full restore retains captured pre-completion history;
it does not invent a validation for the restored copy. `validate-backup` remains
an explicit byte check and does not advance a backup-creation date.

Meaningful policy tests use an advancing time: before/at the due threshold,
failed versus interrupted attempts, impossible completion envelopes, clock rollback,
unsupported versions and labelled external reports. A real Store backup/validation
regression checks recent→due, failed destination reuse without overwrite or date
loss, unconfirmed/corrupt/oversize history, recovery to a later completed backup,
and private Unix receipt permissions. Self-review additionally fences copying a
completion from an unsupported receipt into a later failed attempt.

Actual Chrome 155 against disposable storage passed sign-in, initial unknown
status, stopped-server full backup and validation, recent completion, a real failed
retry preserving its prior date, `doctor`, an explicitly reported overdue external
backup and mobile overflow checks. The journey is `web/tests/backup-reminder.mjs`;
its commands execute the real server binary. No successful backup is mocked.

The first workspace run exposed two genuine recovery-fence conflicts: recording
completion changed the live receipt after the backup captured it. The repair
exempts only `state/backup-history.v1.json` from live-versus-backup equality, just
as operational SQLite/lock changes already have separate treatment. The backup’s
captured receipt still undergoes full byte verification; unknown private files,
credentials, meaningful evidence and raw masters keep their existing guards.
The retention ledger records this explicit operational-file policy. The existing
conversion regressions also exercise corrupted backups and unbacked unknown files.

Final acceptance: all 462 workspace tests passed, with four existing opt-in
checks ignored, including full backup/restore and both conversion recovery tests.
The application policy and bounded receipt regression also passed after review.
All 32 actual HTTP integration tests, strict workspace/all-target Clippy, formatting,
architecture/link/fixture checks, generated OpenAPI/TypeScript comparison, web type
checking/build and rebuilt embedded server passed. The affected Chrome workflow
passed again after the repairs. Reuse the preceding document milestone’s full
Chrome smoke for unaffected listening/list/authentication behavior. External-report
status skips local receipt reads, and closing administration cancels its read. No NAS/deployment matrix is repeated for this bounded status feature.
