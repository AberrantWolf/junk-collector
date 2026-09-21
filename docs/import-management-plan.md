# Import history and cleanup — P04 completion follow-up

**Planning update 2026-09-21:** [FX-03/04/05/07](ux-fix-plan.md) incorporates the
remaining searchable history, bulk cleanup, dismiss/restore and completion scope,
plus newly requested manual item removal and clean reimport. These are proposed
deliveries, not completed P04 criteria; retain the implemented LA1 subset below. FX-02 adds shared scan ownership and resumable, nonblocking analysis polling; it does not complete P04 history search, dismiss/restore or bulk cleanup.

**Scheduling update 2026-09-16:** [Local alpha](local-alpha-plan.md) is active.
LA1 integrates the relevant acceptance below across layers. Individual child
assignments at the bottom are reference scope, not separate required handoffs.
Keep unmet broader criteria open; alpha does not certify every web interface.

User requirement, 2026-09-15. LA1 implements the focused single-import journey;
the broader completion gate remains planned. Required before calling
the web curation UI complete.
P04.1–P04.3 remain historically validated but do not satisfy this added gate.
See the [backlog](handoff-backlog.md#p04--inbox-import-review-and-job-ui) and
[music-first milestones](music-first-milestones.md).

## Problem and ownership

Cross-interface companion gate: [U05](interface-readiness-plan.md) owns smart
defaults, eligible choices and up-front prerequisite guidance across all screens.
P04.6 must validate import/history/cleanup against that standard; the broader audit
cannot be satisfied by correcting only the user-reported import examples.

Three “Import analysis” rows, two successful and one failed with two attempts,
cannot identify the packages/albums, explain each attempt, distinguish analysis
from publication, or help remove accidental imports. JobView currently omits
Import identity and attempt history; the editor restores one browser-local Import
handle rather than offering a discoverable import history.

P04 owns the complete workflow. P02 supplies job/attempt history, P03 supplies
import discovery and discard semantics, and P01 supplies publication/recovery
boundaries. Application services own authorization, projections and lifecycle
commands; store adapters own enumeration and staging reclamation. HTTP/React use
generated typed contracts. This belongs outside D06 enrichment and P05 backup;
it does not depend on C06 event streaming.

## Child ledger

| Child | State | Scope and dependencies |
|---|---|---|
| P04.4 — Identifiable import and attempt history | LA1 subset implemented | Bounded submitter-scoped list/detail, step/attempt projections and reopen are implemented; search/filter/dismissed history remain planned |
| P04.5 — Import cleanup lifecycle | LA1 subset implemented | Cancel/fence and discard unpublished staging with actual byte results plus published association correction are implemented; archive/restore, bulk and cleanup-failure UI remain planned |
| P04.6 — Web completion review | Planned | LA1 real-browser journey is evidence for one path, not the full mixed-role/bulk/mobile completion matrix |

### P04.4 — Identifiable import and attempt history

- Browse/search/filter imports by collection, album or unattached state, package/
  file names, dates and lifecycle status. Reopen after reload without relying on
  the last browser-local Import ID. Use bounded server-side pagination.
- Group staging, analysis, publication and attempts beneath each Import. Show
  useful names plus IDs for disambiguation, source type, file count/bytes,
  timestamps, association, and links to any published Capture.
- Summarize actual findings, limitations, failures and next actions. Successful
  analysis does not imply identification, audio verification or publication.
- Show retained attempt start/end, outcome and safe diagnostics; retry preserves
  previous failures. Label missing legacy attempt detail unavailable rather than
  inventing a timeline from a counter.
- Preserve submitter scoping and current collection authorization. Typed relation/
  permission fields drive the UI; raw requests, paths and worker secrets stay private.

### P04.5 — Import cleanup lifecycle

- Explicitly discard/delete accidental unpublished imports, including successful
  analysis awaiting review. Confirm exact packages and staged bytes affected and
  whether re-upload is required; preserve external source files.
- Cancel/fence active work before reclamation; reject stale upload, review, retry
  and publication after discard. If publication already committed, cleanup must
  preserve the Capture and its bytes. Coordinate through existing services/jobs.
- Reclaim discarded staging payloads using durable, restartable cleanup state.
  Retain the minimal audit needed for recovery/diagnosis; show failures and actual
  reclaimed/pending bytes. Hiding a row alone does not satisfy cleanup.
- Separately dismiss/archive terminal history with a discoverable archived view
  and restore action. Support selected/bulk cleanup with explicit targets,
  eligibility and per-import outcomes, rather than silently clearing all history.
- For mistaken published imports, navigate to the Capture and allow association
  correction or archiving the erroneous entry with retained history. Clearly
  distinguish this from deleting its published originals/evidence. Permanent
  archive-wide payload purging needs a separate retention/recovery design.
- Use revision/idempotency and command-effect contracts. Define typed lifecycle,
  permissions and cleanup semantics before implementing; do not preallocate schema
  versions or introduce a separate scheduler.

### P04.6 — Acceptance and completion gate

Reproduce the user's three-import case: two successful analyses and one failed
analysis attempted twice. Without database inspection or raw JSON, identify every
package/album, inspect both failed attempts, distinguish analysis from publication,
and select the next action.

Through the real UI: discard an erroneous unpublished import and verify staging
bytes are reclaimed; dismiss/restore history; reopen another pending import after
reload; correct a published association while retaining originals/evidence. Cover
mixed-eligibility bulk selection, cleanup failure/retry, worker/publication races,
restart during reclamation, stale commands, cross-user/cross-collection denial,
missing modules, confirmation/focus and mobile layout. Existing publication,
playback and recovery checks must pass.

Update this ledger, P04 status, milestones and validation together. P04.4–P04.6
are a necessary web UI completion gate, not a claim that other requirements are met.

## Historical child assignment (use the local-alpha handoff instead)

Implement P04.4 from docs/import-management-plan.md. Read AGENTS.md,
docs/architecture.md, docs/release-requirements.md and the P02/P03/P04 backlog
entries. Inspect current stores/callers; define the bounded authorized read and
attempt contract, then implement the complete history workflow and real browser
tests. Preserve historical data and label missing legacy attempt detail honestly.
Do not implement P04.5 cleanup in this child. Update the ledger, current status,
validation and next assignment together.
