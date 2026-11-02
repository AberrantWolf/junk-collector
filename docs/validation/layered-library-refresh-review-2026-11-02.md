# Layered library refresh: publication review — 2026-11-02

Follow-up to [the initial delivery](layered-library-refresh-2026-11-02.md).
The user authorized review, fixes and Collector main publication. Trash Amp remains
in its separate worktree; no deployment or native merge/cleanup is included.

## Scope and findings

Reviewed count/page authorization and revision fences, pending/empty responses,
byte budgets and continuation boundaries, asset revision authorization, playback
range/EOF behavior, native incremental manifest commit, lazy track/list reads and
source/selection/request cancellation. Client refresh state remains entirely local;
shared derived projections and their invalidation are server-owned catalog state.

The shared client's explicit full-track-list helper lacked consistent revision,
total and cursor validation. An empty repeated continuation could loop forever;
a changed page revision could mix snapshots, and its size guard missed the last
page. It now requires contiguous forward progress in one frozen revision and exact
completion, with a 10,000-row bound before accumulation. Independent regression
cases cover empty albums, valid multi-page completion, empty/repeated/skipped
continuations, truncation, excess rows, changed revision/total and pending data.
The same validation runs on raw track pages consumed by native album-open reads,
so direct callers cannot bypass progress and completion checks. An initial pending
page remains valid; a revision-bound continuation becoming pending is a conflict.

Native review also found that the count was available before the first title
request but was only published to progress after that page returned. Trash Amp
now publishes zero-of-total immediately after the count response, so a slow first
page cannot leave the UI saying it is still checking login. Its real TLS progress
regression asserts that transition using typed counts rather than display text.

The user explicitly directed that all completed work be published together.
Collector publication includes the digital-album-adoption/manual mapping and
automatic-cover fixes, their tests, generated assets and documentation, alongside
the layered-refresh work and review fixes. No completed edits are excluded.

The preliminary review isolated the layered-refresh changes; those supplemental
results are retained as preliminary evidence. The final publication candidate is
the complete original checkout, with contracts and web assets regenerated there.
A first combined browser attempt exposed stale shared build artifacts from the
temporary checkout: the served server lacked the manual-mapping workflow and the
API generator omitted its assignment type. Affected Collector artifacts were
cleaned, the complete source rebuilt, and the regenerated schema now includes
`DigitalTrackAssignment`. The failed attempt is retained in
`/private/tmp/collector-complete-review-browser.log`; it is not counted as passing.
Temporary checkout artifacts are no longer shared with the final build.

Final qualification reruns the complete Rust workspace, strict workspace Clippy,
generated contracts, web types/build/models and the six affected browser journeys
on that fresh combined build. The original 602-test/all-30-browser record remains
a dated baseline for unchanged journeys; the preliminary isolated run is not a
substitute for acceptance of the complete work. Native qualification uses the
actual published Git dependency after Collector is pushed.

Actual healthy NAS/phone latency, cellular access, hardware fidelity and
release-image qualification remain unclaimed. General archive-guard filesystem
probes retain their existing behavior; a kernel-blocked mount is not made bounded.

## Publication candidate validation

- Fresh complete workspace: **606 passed, zero failed, five existing ignores**,
  including doctests. Log: `/private/tmp/collector-complete-final-tests.log`.
- Strict workspace/all-target Clippy passes:
  `/private/tmp/collector-complete-final-clippy.log`.
- Fresh combined-source browser acceptance passes **smoke, browse,
  digital-matching, intake-review, game-audio and remote-media**. The mapping
  journey includes partial recognition, complete manual mapping and effective
  saved artwork. Intake covers reviewed copies and 31-album activity; remote
  media exercises actual separate-origin streams/seeks and revocation. Log:
  `/private/tmp/collector-complete-final-browser.log`.
- Web type checking/build/model tests, generated OpenAPI/TypeScript agreement,
  architecture/document links, formatting and diff checks pass. The prior dated
  all-30-journey baseline covers unchanged journeys. Preliminary review also ran
  the full selector, but final claims above refer to the complete rebuilt source.
- Pre-pin native source review: **248 passed, zero failed, three existing ignores**,
  including the zero-of-total transition and real TLS journeys. Final Git-pin
  qualification is recorded in the retained Trash Amp worktree's validation doc.
- Source: base `83ad25a3057f4aad5e0a90e52c795d74e80d821e`; SHA-256 of
  `git diff 83ad25a --binary -- Cargo.toml Cargo.lock crates api web`:
  `73ef505160d84db40c795275416530f068daac1def5e9e1c2ac49d1f1fff627e`. Documentation is excluded from this digest.

All completed Collector source, tests, contracts, assets and delivery records are
included in the main publication. No release tag, deployment, native merge or
Trash Amp worktree cleanup is part of this publication.
