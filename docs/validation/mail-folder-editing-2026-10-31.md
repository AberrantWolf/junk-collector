# Connected Bridge folder editing — 2026-10-31

User-requested source delivery in the working tree based on `56b6618`.
No production deployment or mailbox mutation was performed.

## Delivered workflow and boundaries

Purchase inbox shows each owned connection's configured folders and **Edit scanned
folders**. Enter 1–16 exact IMAP folder names, one per line, then **Save folders**;
**Cancel folder editing** discards the draft. Existing scan enablement is retained.
Before first enablement, saving associates the profile with the selected collection
and leaves scanning paused; the form explains that association.

`mail.configure` owns application validation, collection write eligibility,
profile ownership, expected revisions and input-bound command receipts. Folder
overrides are private connection data in SQLite. Existing records with no override
continue using their provisioned profile defaults; immutable credentials are neither
rewritten nor requested again. Owner-scoped read DTOs include folder names without
credentials. Existing curation listener composition applies to the updated API.

The provider receives the frozen override with its read-only request. Saving
increments the connection revision so old batches and old failures cannot publish.
Storage retains the latest committed cursor even when a scan advanced after the
editor opened. New folders start history; retained/removed/re-added folders retain
their recorded positions. Existing accepted/dismissed suggestions and source-key
deduplication remain intact. Changes take effect without restarting Collector.

The form preserves drafts during status polling, freezes revision/enablement,
keeps failed saves visible and supports cancel/reopen after a conflict. Separate
profile/account/collection keys and abort controllers fence obsolete continuations.
Folder access is checked during the next enabled scan, rather than during save;
missing folders remain visible scan failures without advancing history. Exact
folder names still come from the operator; folder discovery and reset/rescan controls
are outside this delivery. Initial Bridge-sync completeness is unchanged.

## Validation

Selection: `python3 scripts/check_changes.py --dry-run --journey email-purchases`
classifies the complete staged/unstaged/untracked change as `infra`, selecting
workspace-wide source checks and all 30 browser journeys. The working tree is not
a sealed release receipt.

Passed:

- Architecture/documentation links, `cargo fmt --all -- --check`, TypeScript check,
  web build and generated OpenAPI/TypeScript contract verification.
- `cargo test --workspace --locked`: 566 passed, five existing ignores.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`.
- Test-selector, container-tool, release-policy and container-validation scripts;
  the container-validation script retains its rootless-skopeo prerequisite skip.
- `npm --prefix web run test:models`.
- `python3 -B scripts/test_purchase_mail.py`: 17 passed, including new-folder history,
  retained existing positions, removal/re-addition and read-only access.
- Focused `collector-providers` tests after the typed folder-request refinement,
  and all six `collector-store --test email_purchases` tests after the final
  authorization cases. The new regression covers retained latest cursor, replay,
  changed-input receipt rejection, stale revision/batch/error refusal, bounds,
  owner separation, Viewer denial while paused, Curator edits before enablement,
  preserved paused state, deduplication and restart persistence.
- Final focused email browser acceptance: initial connection, invalid list/recovery,
  edit before enablement, save while enabled/paused, reload, polling draft retention,
  cancel, concurrent conflict/reopen, cross-user denial, server restart and actual
  IMAP access to the saved folder. Desktop and 390px mobile rendering inspected.
- `node web/tests/run-suite.mjs`: all 30 journeys passed outside macOS seatbelt.
  Includes the retained provider/media/recovery/WebAuthn journeys and email editing.

The selector's source steps passed; its browser step was refused by the known
macOS seatbelt launch guard before starting Chrome. Browser qualification above
uses the separate permitted execution context, not that refused attempt. The
first focused browser attempt also found the textarea's implicit label included
its initial contents; explicit label association corrected it, and subsequent
focused and broad browser runs passed. Later final UI validation/copy and focused
authorization cases received the checks listed above; unchanged broad evidence
was reused rather than repeating the workspace and unrelated journeys.

The five Rust ignores are existing owned-disc/manual scale/performance tests and
an independently invoked remote-reader subprocess fixture. Real personal receipt
calibration, production deployment, release-image qualification and physical
mobile/cellular acceptance were not run for this source-only feature. All mail
fixtures are synthetic and isolated; no real credentials or messages entered chat.
