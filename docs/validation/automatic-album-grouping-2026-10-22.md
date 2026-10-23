# Automatic MusicBrainz multidisc grouping — 2026-10-22

The user requested automatic grouping of clear different-disc matches, reserving
confirmation for ambiguity. This supersedes the earlier confirmation-only policy.

Application policy uses accepted MusicBrainz album identities, consistent medium
layouts and unique recorded disc-ID matches. All preserved sources must be covered;
positions in separate entries must be distinct. Coherent release/language variants
are accepted. Duplicate disc positions, inconsistent release media, absent/ambiguous
layouts, conflicting explicit metadata, independent inventory/unknown data and
explicit separation remain reviewed cases. Similar names/images do not qualify.
Physical copies are kept distinct; no physical set or ownership is invented.

The accepted-album projection supplies paginated, currently authorized cohorts on
startup and every recovery pass. Existing accepted metadata needs no new provider
lookup. Merges wait for affected guided imports, Jobs and accepted bulk members;
writer/publication gates recheck grant/currency and busy state. Job admission,
generic/batch retries and bulk acceptance share that fence. The ordinary group
manifest commits all item/capture/artwork/metadata changes atomically and retains
raw originals and meaningful evidence. Group outcomes identify their automatic
basis; guarded undo writes a portable opt-out. Existing destinations with more
captures are retained when later discs arrive. No payload hash/read is introduced
in candidate enumeration or eligibility.

Regression coverage includes existing accepted metadata after restart, queued work
deferral, later-disc append, repeat reconciliation, copy notes, unchanged source
representations, reindex/undo/restart separation, duplicate positions, conflicting
disc maps, track-layout mismatch, manual title conflicts, unknown domain data,
different album groups, coherent release variants, retained compatible manual
values, pagination and curator revocation. The existing group crash/recovery suite
continues to cover the shared publication mechanism.

A regression exposed a preexisting undo issue: group construction concatenated
Capture/Asset vectors while reads hydrate them by identity. Canonical ordering now
keeps an unchanged group undoable after hydration/reindex. This changes expected
ordering, not the meaning or identity of any source. Assertions check retained
content and successful guarded undo, without freezing discovery order.

Real Chrome 155.0.8059.85 acceptance passed using four separate inbox packages,
fixed offline MusicBrainz TOC vectors and real original files. Without grouping
confirmation, they became one album with four source Captures, physical-copy
records and track listings. Downloads of all four binary originals retained their
exact lengths and every byte. The real Library showed one row, an old Metadata
link followed the destination with functioning browser Back/Forward, and the mobile
grouping history identified the automatic
basis, and Undo plus restart preserved separation. Undo restores the immediately
preceding group transaction; an incrementally assembled album may restore a prior
combined entry plus its latest discs.

At this initial acceptance point the previous smoke journey used incomplete
release-medium snapshots for reviewed grouping. The follow-up below replaces that
incidental ambiguity with contradictory disc placements across equivalent releases;
the complete coherent fixture continues to test automatic grouping.
The 31-entry activity fixture has repeated disc positions and remains a review case.

Passed final validation:

- `python3 scripts/check_architecture.py`, `cargo fmt --all -- --check`, and
  `git diff --check`.
- `cargo test --workspace`; archive tests: 259 passed, three declared manual
  owned-media/scale qualifications ignored. The separate owned-disc oracle remains
  explicitly ignored without its user-owned fixture.
- `cargo clippy --workspace --all-targets -- -D warnings`.
- API contract regeneration/check, `npm --prefix web run check`,
  `npm --prefix web run build`, and Rust server rebuild with embedded assets.
- `npm --prefix web run test:smoke` and the final
  `AUTO_GROUPING_ONLY=1 npm --prefix web run test:intake-review` browser journey.

This is a local source delivery;
no image publication, NAS restart, live metadata fetch or live collection merge
was performed. Actual user archive and NAS throughput qualification are separate.

## Edge audit — 2026-10-22

The user asked about grouping independently of language/album ambiguity and after
an explicit album choice. Before the authorized follow-up below, three temporary integration cases reused the real CUE,
preserved-source and metadata fixtures; all confirmed the following behavior at the audit point:

1. Two language/release alternatives sharing one MusicBrainz release group on each
   disc group automatically when layouts and distinct medium positions agree.
2. Two genuinely different album-group alternatives on each disc remain separate.
   Choosing A on only the first disc still leaves them separate; choosing the same
   exact A candidate on both permits automatic grouping on reconciliation.
3. Explicitly selecting different but equivalent releases within one album group
   on the two discs incorrectly blocks grouping. The merge's manual-authority
   compatibility check compares complete candidate identities rather than album
   equivalence. That includes version identity, so separate snapshot/artwork-derived
   versions can present the same problem even for the same release ID.

No production behavior changed in this audit. The temporary harness was removed
rather than committing assertions that freeze the undesired behavior. The prior
permanent release-variant test covers automatic selection plus a compatible manual
text override; it does not prove the explicit-candidate selection transition.

The implementation also assesses one effective release per entry and enumerates
only accepted album peers. It does not prove invariant membership across genuinely
unresolved alternatives, propagate an album choice to ambiguous sibling entries,
or choose a safe subset when another entry in the same cohort causes a conflict.
Those audited boundaries are addressed by the follow-up below. Busy-work deferral, duplicate-position review, undo separation, same-
collection authorization and the 32-entry group bound continue to apply.


## Ambiguity follow-up — 2026-10-22

User authorization: group clear membership even before the metadata choice,
accept equivalent manual selections, and do not let one ambiguous entry block a
safe subset. The application compares the same viable album-option sets with
compatible distinct source positions under every option; an overlap alone is not
proof. Option-dependent disc numbering can stay unresolved. A unique established
album may admit unresolved siblings; competing established albums retain review.
Identical closed compatibility neighborhoods form safe cliques, prioritizing
accepted subsets before unresolved attachments without arbitrarily choosing among
duplicate discs or conflicting authority. One bounded class is
published per connected component/pass, with fresh reassessment next time.

The existing atomic group publisher is the only write path. Ephemeral read-set
guards cover every assessed neighbor plus new indexed competitors. Manual choice,
unknown data, physical copies, source Captures and original bytes remain retained;
all group options supported by every member remain current. Historical candidates
and source selections remain available without fabricating an accepted identity.
Group history explains that metadata can still need a choice. Metadata detail
polls follow a background merge and refresh its matches/undo eligibility after a
choice; obsolete reads and continuations are fenced. Chosen authority is rebased
against inventory reset markers before composition, preserving explicit clearing
as well as saved choices. Explicit undo still
suppresses regrouping. Limits: 200 related entries and 32 MiB of input metadata per
assessment, 32 entries per publication, 128 masters per entry; larger/incomplete
proofs stay available for reviewed grouping.

Catalog schema 15 replaces the single accepted-album projection with a
many-to-many viable-option projection. Existing older catalogs are retained and
rebuilt from portable records; private state and originals are unchanged. The
upgrade case retains pending Jobs, access, inventory and tentative alternatives,
recovers peers without duplicate proposal rows, and merges only after busy work
settles. No payload hashing was added to browsing, admission or reconciliation.

Focused preserved-CUE integration tests passed for invariant A/B ambiguity,
option-dependent numbering, equivalent manually selected release IDs and versions,
manual-choice propagation, competing accepted targets, non-proof from overlapping
options, safe subsets beside duplicate/manual/unknown/placement conflicts, reindex,
undo separation, old-catalog rebuild and changed/new competitor publication fences.

Follow-up acceptance completed in Chrome 155.0.8059.85 against isolated temporary
profiles and deterministic offline providers:

- `AUTO_GROUPING_ONLY=1 GROUPING_AMBIGUITY=1 npm --prefix web run test:intake-review`:
  four discs grouped before any metadata choice; both album options remained
  current and tentative. Every binary original retained its exact length and bytes.
  Canonical old links, Back/Forward, mobile review, explicit selection, immediately
  disabled stale undo and selection persistence after restart passed.
- `AUTO_GROUPING_ONLY=1 npm --prefix web run test:intake-review`: coherent automatic
  grouping, all source/copy/track records, byte-exact downloads, old links, mobile,
  guarded Undo and restart separation passed.
- `npm --prefix web run test:smoke`: the full application suite passed, including
  reviewed grouping with contradictory release placements, actual FLAC playback,
  provider concurrency, inventory, Activity, access control and WebAuthn.
- Fourteen focused preserved-source grouping cases passed (the thirteen-case suite
  plus the inventory-reset case). The latter ensures grouping cannot resurrect an
  explicit metadata choice cleared by inventory lifecycle authority.
- Architecture/link/fixture checks, formatting, diff checks, API contract checks,
  TypeScript checks/build, server rebuild with embedded assets and workspace Clippy
  with warnings denied passed.

`cargo test --workspace` passed on the final sources: archive integration tests
270 passed, zero failed, three declared owned-media/scale qualifications ignored.
Worker scheduling, server/API, module, library, adapter and documentation tests
also passed. The independent owned-disc oracle remains ignored without its fixture.

Source-only delivery: no live collection merge, provider lookup, NAS restart or
image publication.

## Commit review repairs — 2026-10-22

Two regression cases reproduced unsafe grouping before the repairs:

- Incomplete track coverage or contradictory disc placements in one viable album
  alternative removed that option, allowing the remaining option to justify a
  merge. Unproven alternatives now remain in the membership assessment. Every
  retained option must prove compatibility; a later explicit album choice can
  still establish a unique valid target and admit its unresolved sibling.
- The membership projection reused historical candidates covered by an inventory
  automatic-metadata reset, although display correctly hid their automatic use.
  Membership and display now share current-candidate reset policy. The reset
  prevents grouping while preserving history for deliberate later selection.

Both cases assert deferred grouping and successful explicit resolution using real
preserved CUE sources. Existing coherent and ambiguous grouping, preservation,
publication fences and undo behavior remain covered by the grouping suite.

The 31-package browser run also exposed completed imports becoming labelled
cancelled after grouping: removed-item recovery cancelled settled intents for
merged source entries. Recovery now stops unfinished workflows while retaining
terminal outcomes. Browser grouping acceptance checks successful import outcomes
after restart. The progress fixture now deliberately supplies contradictory disc
placements across equivalent releases; repeated disc positions alone cannot
prevent a safe distinct-disc subset from grouping as metadata arrives. Its
reviewed-grouping assertions therefore test a real ambiguity rather than timing.

Final review validation passed: workspace tests (272 archive tests passed; three
declared manual qualifications ignored), all sixteen focused grouping cases,
workspace Clippy with warnings denied, architecture/link/fixture checks, Rust
formatting, API contracts, TypeScript/build and rebuilt embedded server. Chrome
navigation, ambiguous grouping with completed-outcome retention after restart,
and the one-worker 31-package progress/settlement journey passed. The latter now
allows four minutes for its paced provider acquisitions; this is not a throughput
qualification.
The final rebuilt server also passed the complete Chrome smoke suite, including
real FLAC playback, provider concurrency, reviewed grouping, collection lists,
access control, WebAuthn and mobile behavior. No P0 issue was found in this review.
