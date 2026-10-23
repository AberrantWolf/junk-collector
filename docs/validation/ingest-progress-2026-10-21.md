# Import execution visibility — 2026-10-21

Requested outcome: during a 31-album inbox import, distinguish albums actually
running from queued/waiting albums and show their current ingest stage.

`IngestSummary.work` separates current execution from overall import outcome.
Application policy owns typed execution states; the private adapter reads grouped
Job rows through import, preparation-member and current acquisition-wait ownership.
It reads no original payloads, changes no stored formats and adds no worker slots.
Provider execution can be shared; its actual queue/claim/phase is projected without
exposing internal Job identity or another requester's details. Unfinished child
tracks remain visible after batch admission completes. Source/analysis/conversion/
publication phase updates occur at real execution boundaries.

The existing real-server `test:intake-review` journey now adds 31 actual inbox
packages. The offline provider fixture pauses MusicBrainz responses so Chrome can
inspect running versus queued metadata work and shared downloads, then releases
responses and requires all imports to finish. The journey checks reload, mobile
width and terminal polling. No application responses or Job execution are mocked.
Existing inbox review, grouped metadata/artwork and cleanup/recovery acceptance
also remain part of this command.

The initial run reproduced the old FIFO problem: all initially admitted copies
could precede later analysis. The user then requested progress in both directions.
Dispatch now alternates copy/other-processing turns. When both have runnable work,
the claim transaction reserves one of the existing two processing slots for each;
either can borrow idle capacity. Running work is not preempted, and a newly waiting
lane receives the next released slot. FIFO remains within each lane. Provider
capacity stays independent. The transient turn resets safely on restart without
changing durable Jobs.

Storage regressions hold analysis open while another copy advances, then hold a
copy open while another analysis advances. Sequential dispatch must alternate;
copy-only and processing-only queues must each use both slots, and cancellation
must retain capacity until acknowledgement. Existing stale-claim/recovery gates
remain required. The 31-package browser journey additionally requires a published
album while other packages still have unfinished copying.

Inbox bytes still copy through private import staging and archive publication
staging. Eliminating that extra copy remains a separate queue entry; the dispatch
repair changes scheduling without changing publication I/O.

The focused Store regression checks partial browser upload, publication queue,
worker claim and phase, cancellation until acknowledgement, and requester denial.
The existing full audio-preparation workflow now requires its pending track to
appear after batch admission, expose its running conversion phase during the
encoder call, and disappear after actual output publication. Inbox-only batches
also hide browser upload/resume controls and label the staging step as copying.

The final mobile check found the generic library-detail rule hiding the intake's
rows under 1150 px. Intake rows now use their own container and wrap actions on
phones; the browser regression requires an actual album row to remain visible.

Local gates passed: architecture/document links, Rust formatting, workspace tests
(including the fair-dispatch/claim/cancellation regressions), Clippy with warnings
denied, TypeScript, web build, generated API contract checks and rebuilt server.
The complete Chrome inbox review/cleanup/31-album journey and general smoke passed,
including two concurrent encodes and independent provider execution. The corrected
mobile row/wording journey also passed on Chrome 155.0.8059.85, using a fresh
31-package server/profile: rows visible, no horizontal overflow, published albums
while copying remains unfinished, active/queued/shared stages, reload and terminal
settlement. Its temporary artifacts are in
`/var/folders/5w/77h8k79d3z7dftcv8mt3mgzm0000gp/T/collector-intake-review-S7XLQ4`.
The archive integration suite finished with 255 passed and three explicitly
ignored manual scale/performance qualifications; the rest of the workspace passed.

This delivery is source-only; no NAS process, active user import, Docker release
or installed image is changed by this work.

## Commit review follow-up — 2026-10-22

A real-browser run reproduced an initial draft-loading race: choosing the server
inbox before IndexedDB restored the draft could be overwritten by the default
browser source. Source and review controls now wait for the current draft to
finish loading. The progress provider fixture deliberately supplies conflicting
release disc placements, keeping this workflow a reviewed-grouping case as the
separate automatic-grouping delivery arrives. Completed import outcomes also
survive merged-source recovery; see the
[grouping review repairs](automatic-album-grouping-2026-10-22.md#commit-review-repairs--2026-10-22).
