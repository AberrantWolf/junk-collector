# Local-alpha review

## LA1 submitted implementation — 2026-09-16

LA1 is implemented locally and submitted for integrated review. This record is not
an independent review or permission to begin LA2.

### Implemented boundary

- `collector-app` owns typed plan/accept/history/lifecycle contracts and a focused
  guided-ingest coordinator. It composes existing provisional Import, durable Job,
  processing, publication, metadata-resolution and Asset effects.
- `collector-store` private schema 13 adds only guided intent and restartable cleanup
  state. Portable sources, Captures, Representations, observations, Assets and
  authority records retain their existing archive ownership.
- `collector-server` resolves configured direct inbox selections, freezes/rechecks
  their inventory, exposes generated `/api/v1/ingests` routes, and runs paginated
  startup and periodic reconciliation alongside the existing durable workers.
- The React collection action plans browser or inbox packages, uploads/resumes exact
  browser files, renders durable progress/attempts/history, and submits lifecycle
  decisions. It does not schedule provider or publication steps.

### Authority and failure behavior to review

- A filename-derived title is the initial inventory baseline, not a title override.
  Later automatic candidates may display; a later manual inventory edit survives
  refresh and retry.
- CAA bytes publish as a normal cover Asset and bind to the immutable automatic
  metadata candidate. Guided enrichment does not set manual artwork authority.
- Missing analysis support, provider outage or no configured provider does not roll
  back source preservation. The history exposes a partial/failed optional outcome.
- Retry reuses deterministic Job/output identities. Cancellation retains bytes until
  explicit discard. Discard is forbidden once publication begins/commits and uses a
  durable cleanup record for restart reconciliation.
- Published correction changes only the Capture association through its normal
  revision contract; originals and evidence are retained.

### Submitted regression evidence

Focused Rust tests cover receipt replay and changed result ownership, stable output
identities, intent-to-job restart recovery, successful MusicBrainz/CAA automatic
enrichment, unavailable-provider/manual-title authority, retry, cancellation,
discard byte reclamation, cross-user denial and published association correction.
HTTP coverage resolves and stages a configured inbox, replays acceptance, completes
the server-owned pipeline and hides history from another user. The real Chrome
smoke adds a browser package without a pre-created Item, closes the dialog, reopens
durable history, waits for preservation, opens detail and observes the new entry.

### Deliberately remaining

P04 search/filter, terminal dismiss/archive/restore, selected/bulk cleanup and its
full mixed-race completion review remain open. U05 still requires its complete
operation matrix and application-wide interface audit. LA2 preparation controls,
native clients, a new online game provider and P05 are untouched. Live
MusicBrainz/CAA, user-owned dumps, target deployment, other browsers and real-dump
format qualification were not run; deterministic providers and synthetic package
fixtures do not satisfy LA3.

### Review checklist

1. Trace plan-token and receipt binding before mutable inbox/provider resolution;
   verify changed inputs cannot reuse acceptance and all stable identities replay.
2. Inspect recovery at intent, queued/running Job, publication commit and cleanup
   boundaries; look for duplicate effects or authorization bypass after grant loss.
3. Challenge readiness on missing/ambiguous anchors and ensure preservation remains
   available without describing structural analysis as catalog verification.
4. Race manual title/artwork edits with provider completion and retry; verify no
   automatic result becomes manual authority or silently mutates candidate history.
5. Exercise stale revisions, cross-user/collection reads, cancellation before and
   after commit, discard with active work, and association correction retention.
6. Re-run generated-contract, workspace, embedded-asset and temporary-profile
   browser gates. Keep unrun external qualification explicit.

## Independent review corrections — 2026-09-16

The integrated review found and corrected receipt/preflight ordering, first-page-only
recovery, missing periodic inbox dispatch, post-publication polling, ambiguous
terminal workflow state, initial metadata-authority crash recovery, replay against
only the latest observation list, duplicated provider mapping, cancellation/retry
re-admission, premature cleanup receipts, upload/cleanup races, and stale association
responses. Readiness now belongs to modules, shared result DTOs replace duplicated
JSON field probing, and failed structural analysis does not block preservation.

Browser coverage now imports a CD package with enrichment enabled, closes/reopens
history and waits for provider metadata and artwork. The first expanded run exposed
a mismatched disc-ID fixture (correctly rejected) and a close-before-upload-complete
test race; both test issues were corrected. Existing playback checks remain included.

Current limitations are explicit in implementation notes: name-level preflight,
atomic multi-package admission, no guided split/anchor editor, and unknown historical
attempt start times. Broader P04/U05, real dumps/live providers and LA3 remain open.
Final gate results are recorded in validation.md after execution.

### Review disposition

LA1 is accepted for the bounded local-alpha journey, with the documented intake and
external qualification limits above. Both HTTPS and local CLI-setup Chrome runs
passed; the workspace run passed 285 tests with the owned-disc oracle left ignored.
Final focused regressions cover progress projection and generic provider-Job
cancellation. Architecture, contracts, formatting, Clippy, web build/check and
embedded server build passed. The next assignment is LA2; no commit or LA2 work was
performed. This is a useful backup checkpoint before that delivery.
