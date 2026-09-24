# Album preparation and nonblocking background work

**FX-03 update (2026-09-21):** Work & history, scoped server filtering/pagination,
workflow/attempt grouping, dismissal/restoration, selected cleanup with durable
settlement and byte outcomes, server-backed import reopening, and removal of
ineligible derivative analysis forms are implemented. Local acceptance passed on 2026-09-21; see [validation](validation.md#fx-03-work-and-history--2026-09-21). Broader interface, mistaken-item removal/reimport and release gates
remain open; this does not mark all U05/P04 criteria complete.

**Planning update 2026-09-21:** [FX-02–FX-05 and FX-07](ux-fix-plan.md) applies the
remaining whole-application nonblocking rule to scan lifecycle, durable progress,
cleanup and reimport. Equivalent active scans must converge; completed results
become the primary view, with equivalent actual rechecks recorded as lightweight
occurrences. FX-02 implements these scan semantics and removes the whole-dialog execution lock from processing and manual import analysis; [local acceptance passed](validation.md#fx-02-scan-lifecycle--2026-09-21). FX-03 implements grouped work/history and durable selected cleanup using the completed LA2 patterns below; its local acceptance passed. Clean reimport remains FX-05.

**Scheduling update 2026-09-16:** [Local alpha](local-alpha-plan.md) is active.
LA2 integrates the relevant acceptance below across layers. Individual child
assignments at the bottom are reference scope, not separate required handoffs.
Keep unmet broader criteria open; alpha does not certify every web interface.

User requirement, 2026-09-15. LA2 was reviewed and corrected on 2026-09-17;
its bounded workflow acceptance passed. Required before web UI completion alongside
[P04](import-management-plan.md) and [U05](interface-readiness-plan.md); broader
application-wide U05 acceptance remains open.

## Finding and ownership

Before LA2, the music component held one jobId and disabled every Prepare button
while it was set. Encodes already ran as durable background jobs, but the UI
prevented submitting independent work. LA2 removes that client-side serialization.
Worker concurrency and queue admission are distinct:
a bounded worker can run one encode while other accepted encodes wait. Unlimited
concurrent execution is not required.

D07 owns album preparation through its existing per-track conversion/publication
pipeline. P02 owns shared admission, worker bounds and durable job state.
Application services own batch scope, deduplication and dependencies; store
adapters own persistence. U05 owns the application-wide audit/fixes, and P04
supplies identifiable history. No browser scheduler or parallel album encoder.

## Child ledger

| Child | State | Scope and dependencies |
|---|---|---|
| D07.4 — Album preparation and independent enqueueing | Reviewed and corrected; acceptance passed | D07.1–D07.3/P02/A04: durable batch submission, eligibility, album/track controls |
| D07.5 — Batch and queue workflow review | Reviewed and corrected; acceptance passed | D07.4: actual UI, interruption, partial outcomes, duplicate/race and playback checks |

### D07.4 — Whole-album preparation

- Offer Prepare album as FLAC, producing separate playable files for all selected
  eligible tracks through existing track jobs. Keep individual-track actions.
- Resolve associated discs/source captures explicitly. Default to all tracks
  across unambiguous discs; explain missing discs or alternative captures rather
  than encoding duplicates. Show scope, profile, track count, existing outputs,
  queued tracks and blockers before submission.
- Submit durably and return promptly with an identifiable batch and child-job
  outcomes. Closing/reloading the browser or restarting the server must not lose
  accepted work. Define bounded admission and crash-safe batch/fan-out semantics
  before coding; a browser loop cannot own the remaining submissions.
- Reuse valid outputs and active identical requests. Bind receipts to submitted
  scope and frozen source/profile identities. Repeated album clicks, overlapping
  track requests and retries must not duplicate jobs or outputs.
- Report accepted, already available, already queued and ineligible members.
  Mixed eligibility/capacity must not look like full acceptance. An album is not
  fully prepared until its requested scope succeeds; partial results stay visible.
- Show aggregate/per-track queued, running, succeeded, failed and cancelled
  states. Retry only unfinished eligible members. Allow cancelling a track or
  the batch's outstanding work; define ownership of shared/pre-existing jobs so
  cancelling a batch cannot cancel unrelated work.
- Preserve masters, validation, authorization/source checks and per-track
  publication. Failure of one track must not undo successes or block independent
  jobs. No new recipe algorithm or separate publication path is needed.

## Nonblocking rule for every interface — U05.2/U05.3

- Separate submitting a command from awaiting its background result. Guard the
  identical pending submission where needed, then show queued state once accepted.
  Other eligible tracks, albums and independent actions remain available.
- A generic busy flag must not lock a whole page/dialog for a background job's
  lifetime. Users can navigate, edit unrelated records, play available media and
  submit more independent work while jobs run.
- Block only real dependencies, conflicting mutations, duplicate requests,
  missing permissions/inputs or explicit admission/resource limits. Explain what
  is blocking the action and what will unblock it. Verification may need a
  structure result; another supported track need not wait for an encode.
- If dependent work is offered as queueable, represent/enforce that dependency
  server-side, including failure/cancellation. Never fabricate prerequisite inputs
  or use browser polling as a substitute for dependency handling.
- Bound concurrency and admission in the service, with clear capacity feedback;
  do not simulate resource policy by globally disabling unrelated UI.
- Make durable progress accessible outside the originating dialog and restore it
  on reopening. Closing a progress view is not cancellation. Failures remain
  visible and attributable to the package/album/track and attempt.
- Audit imports, analysis, verification, providers, preparation, inventory,
  artwork and administration, and future interfaces as they land. For every
  disabled action, record its actual dependency/conflict and owner. Blanket locks
  are defects even when the user has not encountered them.

## D07.5 and U05 acceptance

During a deliberately slow real encode, enqueue another track and another album
before it finishes; observe multiple accepted jobs with bounded execution. Use
Prepare album on a multi-disc fixture, close/reload, and verify all accepted tracks
stay scheduled and become independently playable.

Cover duplicate/batch overlap, mixed eligibility, missing encoders, admission
limits, partial failure/retry, track/batch cancellation, permission loss and server
restart during admission/execution. Never advertise partial files or remove
completed outputs because another track failed.

Across the U05 audit, prove independent actions stay available during real work
and dependent/conflicting actions remain guarded. A spinner or a test that waits
for each job before submitting the next does not establish this requirement.
Record actual UI/server evidence and update the child ledger and status together.

### LA2 submitted result — 2026-09-16

The implementation uses a frozen, bounded durable batch Job over existing per-track
derivative Jobs. It defaults unambiguous associated discs, requires explicit
alternative sources, converges identical active/output work, derives partial state
from child Jobs plus published Representations, retries only unfinished work, and
cancels only owned unshared children. Prepare album and guided-import preparation
call the same application service. React restores server-owned progress without a
page-wide background busy lock.

Permanent tests cover two-disc selection, ambiguity, missing encoders, overlap,
shared cancellation, partial validation failure, retry, restart, revocation through
the existing derivative checks, guided preparation and valid-output reuse. The
temporary-profile Chrome workflow used a delay wrapper around real FLAC 1.5.0,
submitted overlapping album batches, kept inventory controls enabled and played one
completed track in a second page while another member remained active. Exact results and unqualified external
inputs are recorded in [validation](validation.md). D07.5 passed independent integrated review on 2026-09-17; broader U05 interface
auditing is not claimed. See the review corrections in local-alpha-review.md.

## Historical child assignment (use the local-alpha handoff instead)

Implement D07.4 from docs/background-work-plan.md. Read AGENTS.md,
docs/architecture.md, docs/release-requirements.md and D07/P02 contracts. Define
album/source selection, durable bounded admission, idempotency and cancellation
ownership before implementation. Reuse track effects/workers; add album FLAC and
independent track enqueueing with real UI tests. Update this ledger, current
status, validation and the D07.5 review assignment. U05's broader review remains
a separate completion gate.
