# Junk Collector Wave 3 execution plan

Status: execution started 2026-09-10. Wave 2 is the implementation baseline. A
child is complete only when its behavior, tests, documentation, and review
checkpoint are complete; compiling scaffolding is not a completed capability.

## Status vocabulary

- **Implemented:** executable end-to-end behavior whose acceptance tests pass.
- **Not implemented:** planned behavior that is not exposed as a finished
  capability.
- **Externally unqualified:** locally implemented behavior still awaiting a
  named hardware, private-fixture, target-Linux, or independent-review gate.

## Live ledger

| Child | State | Repositories/commits | Result and evidence | Explicitly not yet implemented |
|---|---|---|---|---|
| P02.1 durable job repository | implemented | Junk Collector `29ee3ca` | Private schema v8 stores typed job ownership, targets, state/phase, progress, attempts, timestamps, safe diagnostics, bounded request/result bodies, and future claim fields. The private-v2 fixture and current uploads migrate without lost Job IDs or history. Architecture, fmt, 96 Rust tests, and clippy pass. | Claims, leases, scheduler execution, cancellation/retry controls, and new job routes remain P02.2-P02.4. |
| P02.2 claim/worker state machine | implemented | Junk Collector `bf8a9e4` | FIFO claims are transactionally capped at two active jobs and one per user. Thirty-second leases, five-second worker heartbeats, claim-token fencing, immediate prior-instance takeover, monotonic/throttled progress, and two startup worker loops are executable. Integrity verification is the first durable consumer; HTTP returns a real queued Job, and deterministic Evidence makes post-publication reclaim idempotent. Architecture, API drift, fmt, 102 Rust tests, clippy, TypeScript check/build, and rebuilt server pass. Review corrected a storage-global cap gap and nondeterministic retry Evidence before completion. | Cancellation, retry, publication-boundary controls, upload/inbox migration off request workers, cursor `JobView`, and job mutation/detail routes remain P02.3-P02.4. |
| P02.3 cancellation/retry/publication reconciliation | implemented | Junk Collector `ae4cc12` | Upload source staging is distinct from executable queue state. Browser publication and configured-inbox copying/publication run through the durable workers; the process semaphore is removed. Queued cancellation is immediate, running cancellation is checked between bounded units, payload-boundary cancellation aborts pre-commit artifacts, and post-`RevisionCommitted` cancellation has a stable conflict while recovery completes. Retry keeps the same Job/Capture/Representation IDs, increments attempt, retains source and prior failures. Architecture, fmt, 109 Rust tests, clippy, TypeScript check/build, and rebuilt server pass. | P02.4 cursor `JobView` and submitter-scoped detail/cancel/retry routes are not implemented. Generic event streaming remains C06. |
| P02.4 job API and review | implemented/reviewed | Junk Collector `e36d7b5` | `GET /jobs` is cursor-paged and `GET /jobs/{id}`, `POST .../cancel`, and `POST .../retry` return bounded `JobView` records. Views expose state/phase/progress/attempt/flags/safe diagnostics/timestamps and omit requests, claim tokens, workers, and failure history. Submitter scoping has no administrator bypass; mutations require idempotency keys. Architecture/API drift/fmt, 112 Rust tests, clippy, TypeScript check/build, rebuilt server, and Chromium smoke pass. P02 root acceptance review passed locally. | Event streaming remains C06. Target-Linux process/filesystem qualification and independent review are externally unqualified; distributed scheduling is out of scope. |
| U03.1 focused lifecycle commands | implemented | Junk Collector `0a84bcb` | The broad Item command and generated TypeScript no longer accept specimens; its legacy adapter requires an exact canonical specimen snapshot. Revisioned, idempotent curator commands create/update copies and ordered components, remove unreferenced components, start/return loans, and dispose copies. Command-bound IDs are deterministic, returned loans remain history, published relationships/payloads survive disposal, and reindex/restart preserves two-copy state. Focused HTTP routes are executable and role checked. Architecture/API drift/fmt, 115 Rust tests, clippy, TypeScript check/build, rebuilt server, and Chromium smoke pass. | U03.2 lifecycle editors and U03.3 privacy/browser review were not implemented in this child. The obsolete add-copy shortcut was removed rather than left as a non-working facade; no specimen deletion exists. |
| U03.2 lifecycle editor | implemented | Junk Collector child commit titled `Build physical inventory lifecycle editor` (SHA recorded by the next ledger update) | The archival-workbench curation surface now has copy-level condition, ownership, possession, acquisition, private-note, ordered part/carrier, loan/return-history, and disposal controls backed by U03.1. A real concurrent Item edit proves drafts remain in component state; the editor shows old/current revisions and offers discard/reload or manual reapply. The Chromium interaction covers a two-disc copy, loan/return, conflict reapply, disposal, keyboard order, and a 390px no-overflow screenshot. | U03.3 structural privacy/reindex/browser review is not implemented. There is intentionally no specimen-delete affordance. |
| U03.3 privacy/reindex review | not implemented | — | Existing viewer redaction and the new restart fixture pass locally, but the full structural-absence, independent-copy reindex matrix and browser review remain this child. |
| U04.1-U04.3 supporting assets | not implemented | — | Entire ticket. |
| P03.1-P03.4 provisional imports | not implemented | — | Entire ticket. |
| P04.1-P04.3 import/job UI | not implemented | — | Entire ticket. |
| D02.1-D02.5 shared format/disc work | not implemented | — | Entire ticket. |
| D03.1-D03.5 shared audio work | not implemented | — | Entire ticket. |
| W3.R integration review | not implemented | — | Entire gate. |

## Dependency order and fixed decisions

Execute P02, U03, U04, P03, P04, D02, D03, then W3.R. Each numbered
child is a separate review checkpoint. Private schemas advance from v7 to v8
for jobs and v9 for imports; archive schema remains v3. `JobId`, `ImportId`,
`TransactionId`, and domain entity IDs are distinct. Current upload and inbox
routes remain working adapters until an API-major removal.

Jobs are submitter-private. The local scheduler has two global workers and at
most one active job per user, uses a 30-second lease with five-second
heartbeats, and fences writes by claim token. Cancellation is cooperative until
the archive revision commit point. Retry reuses the Job identity, increments
the attempt, and retains source material.

No unavailable analyzer is presented as verification or as a successful
candidate. D04-D07, C06 events, document attachments, distributed scheduling,
runtime plugins, and externally gated hardware/private-fixture qualifications
remain outside Wave 3.

## Child completion discipline

Every child updates this ledger, `handoff-backlog.md`,
`implementation-status.md`, and `validation.md` in the same commit. Sibling
work additionally updates its owning validation/TODO documentation. Public
routes, generated contracts, migrations, and UI are added only with a real
end-to-end consumer. Tests lead state-machine, recovery, authorization, parser,
hash, and sample-coordinate changes; UI uses interaction, accessibility,
responsive, and visual checks.
