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

| Child | State | Result and evidence | Explicitly not yet implemented |
|---|---|---|---|
| P02.1 durable job repository | implemented | Private schema v8 stores typed job ownership, targets, state/phase, progress, attempts, timestamps, safe diagnostics, bounded request/result bodies, and future claim fields. The private-v2 fixture and current uploads migrate without lost Job IDs or history. Architecture, fmt, 96 Rust tests, and clippy pass. | Claims, leases, scheduler execution, cancellation/retry controls, and new job routes remain P02.2-P02.4. |
| P02.2 claim/worker state machine | in progress | P02.1 is the reviewed storage prerequisite. | Durable claiming and executable scheduling are not implemented yet. |
| P02.3 cancellation/retry/publication reconciliation | not implemented | — | Entire child. |
| P02.4 job API and review | not implemented | — | Entire child. |
| U03.1-U03.3 physical lifecycle | not implemented | — | Entire ticket. |
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
