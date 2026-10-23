# Configurable workers, metadata priority and Work navigation — 2026-10-21

The user requested four processing workers by default, an allowed range of 1–8,
a separate copy cap, one provider execution, and a policy that favors completing
ready discs without monopolizing work. They also reported the stretched Work &
history filter layout and Activity's ineffective Back button.

Application-owned JobSettings validates startup settings; missing keys default to
four processing workers and one copy. Invalid values fail before Store opening.
Server loops and Store's atomic claims read the same settings through JobPort.
Provider capacity stays one. Copy capacity includes guided/retained staging and
the older combined inbox import command. No stored-format conversion is added.

A bounded, account/import-keyed preference favors analysis and original publication
for copied discs. One worker can normally finish those stages before the next copy;
optional metadata waits and track encoding do not permanently own a worker. Two
processing admissions give copying its next turn, with one ready publication
allowed to finish the just-analyzed disc before that turn; after two preferred import
continuations, processing resumes normal FIFO preference. With multiple workers,
contending copying/processing leave at least one slot for each; copying never
exceeds its independent cap. Non-copy processing can borrow idle capacity. Existing
cancellation, lease, immutable timestamp and publication fences remain enforced.

The later metadata-priority request adds a bounded preference for ready metadata
resolution/applying archived metadata over encoding and ordinary processing. After
two preferred metadata/ready-disc admissions another processing task gets its
ordinary scheduling turn;
copy turns and ready-original publication still apply. Tests exercise both metadata
kinds against both older/newer encoding backlogs and a competing affinity
continuation at all worker counts 1–8. This also covers the final-review repair
that prevents alternating metadata/disc priorities from resetting each other
and starving ordinary work. Guided metadata
starts after original publication for its destination album group, independently
of unrelated intake members; text and album identity commit before optional artwork. The existing grouping
feature proposes accepted MusicBrainz album identities and requires confirmation
before combining entries. Matching titles/artwork do not trigger a merge.

Storage tests cover processing limits 1–8, every valid copy cap, both backlogs,
older copy queues versus ready discs, failure, cancellation capacity, provider
capacity under full processing load, and restart from eight workers to one with
old claim rejection and unchanged Job timestamps. CLI tests cover defaults,
configuration reporting and invalid settings before filesystem state creation.
Real Chrome acceptance includes the 31-album inbox journey with one and eight
workers. The eight-worker run also verifies all 30 peer grouping proposals after
accepted metadata, while all 31 entries remain distinct pending confirmation.

The Work filter panel now aligns normal-height search/status/collection controls,
collapses collection choices and shows bulk actions only after selection. Browser
history records each work link's immediate parent. Embedded Activity details clear
on a return to the main Activity URL; reload/Forward do not reopen a second modal.
Direct links retain list fallback; the library's nested Work return still works.
Chrome navigation checks exercise preserved filters, Back/Forward, reload, mobile,
selection keyboard focus, and screenshots; the existing library return and Work
cleanup/retry journeys remain required.

Passed in Chrome 155.0.8059.85 against temporary local profiles: navigation,
Library browsing/nested Work returns, Work history paging/cleanup/retry, the full
smoke journey (including metadata, confirmed grouping, real playback and passkeys),
and the 31-album workflow at both one and eight workers. Desktop/mobile Work filter
screenshots were inspected. Eight-worker storage admission is covered; this is not
an eight-worker NAS throughput benchmark. The extra inbox staging copy remains a
separate follow-up. No image is published and the active NAS is not restarted.

Validation passed:

- `python3 scripts/check_architecture.py`, formatting and diff whitespace checks.
- `cargo test --workspace` (archive suite: 255 passed, three declared manual qualification tests ignored
  for owned media/generated-scale runs); final scheduler changes additionally passed all five
  focused `job_dispatch` tests, including every valid worker/copy-cap combination
  and mixed-priority regressions.
- `cargo clippy --workspace --all-targets -- -D warnings`.
- `npm --prefix web run check`, `npm --prefix web run build`, API contract check,
  and rebuilt Rust server with embedded assets.

The final rebuilt server's one-worker 31-album journey passed again after the
shared-budget adjustment, including all 30 peer grouping proposals.
