# Edition fit and equivalent matches — 2026-10-25

Source repair for the reported 2-disc normal / 3-disc limited edition confusion.
This delivery does not deploy, change a live collection, or publish an image.

## Contract

Application metadata policy reads committed, source-fingerprint-bound CD layout
observations. It keeps the accepted album stable while comparing its editions by
compatibility with known disc IDs/positions, represented media, remaining media,
then the existing evidence/language/provider policy and deterministic tie break.
A smaller compatible edition is a preference, not proof of ownership or pressing.
Unknown/unprocessed sources do not count as a matching or missing disc. Explicit
release selections remain authoritative. Disc-by-disc provider responses retain
other current strong matches and replace returned release versions.

Reads reassess immediately; the existing server recovery loop also runs a bounded
100-entry keyset scan through all live music metadata with an authorized curator.
It publishes changed choices/current identities through existing optimistic
metadata/inventory fences. Maintenance also rechecks curator permission and
workflow idleness inside the publication/job-admission gate; active jobs, guided
imports and bulk library work defer the write. Equal subsequent sweeps publish nothing. Search uses
the saved decision; current item/review/playback reads reassess from recorded facts.
The open Metadata panel polls background changes, cancels obsolete polls during
mutations/unmount, and preserves unsaved text edits.

Equivalent versions require the same provider/record ID, title/subtitle/languages,
artwork, completeness, evidence strength, priority and parsed observation. They
share one active/review representative, preferring referenced identities. Existing
archive revisions, candidates and sources are retained: no physical compaction or
reference rewrite occurs. New MusicBrainz candidate hashes exclude the surrounding
response digest and hash normalized candidate content/ranking/artwork instead.
Old materialized displays validate with policies 1–3; new policy 4 freezes the
chosen automatic MusicBrainz edition so ordinary language ranking cannot undo
its evidence-based preference during projection/reindex.

Album/list expansion accepts available unambiguous discs even when the selected
provider release lists additional media. Coverage still reports that media;
ambiguous sources, incomplete structure, unprepared audio and permissions retain
their real restrictions.

## Validation

Passed focused regressions:

- Incremental 1/2/3-disc fit, manual limited-edition selection and return to
  automatic, and retention of a viable normal edition omitted by a later
  per-disc response. Immediate reads and periodic saved choices agree.
- Publication defers while work is queued; an unauthorized maintenance publisher
  is denied. Cancellation releases the deferred entry for ordinary updates.
- Playback/list target expansion includes available discs despite absent release
  media; source alternatives and preparation remain meaningful restrictions.
- Equivalent versions retain a referenced selectable identity and immutable
  sources across idempotent sweeps/reindex. Changed artwork/ranking/text/evidence/
  observation/provider record IDs remain distinct choices.
- 101 old-policy duplicate entries recover across multiple maintenance pages,
  and both historical and new materialized displays survive reindex.
- Reordered/equivalent provider responses reuse candidate versions; changed
  release content produces a different version. The former test requiring new
  versions for raw response formatting was deliberately corrected.
- The existing metadata refresh test now expects two visible semantic choices
  while independently asserting three retained immutable source versions.

Passed: `python3 scripts/check_architecture.py`, `cargo fmt --all -- --check`,
`cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`,
`npm --prefix web run check`, `npm --prefix web run build` and rebuilding
`collector-server` with the served assets. The incremental-edition regression was
also rerun after adding the maintenance publication guard, including queued-work
deferral and unauthorized publisher denial; it passed.

Final Chrome runs passed: `npm --prefix web run test:fx04-review` and
`npm --prefix web run test:smoke`. The metadata workspace updates an automatic
edition in the background while preserving an unsaved title correction. Smoke
also covers actual playback/seek/track changes/revocation, concurrent provider
requests, bulk import/grouping/preparation, list playback, role denial, passkeys
and mobile layout. Chrome required execution outside the filesystem sandbox.
Existing explicitly ignored owned-corpus/scale tests remain unrun.

## Limits

No live NAS data has been changed. The automatic sweep runs when the updated
server starts and periodically thereafter. Provider versions/raw archive history
still occupy their existing storage; safe physical compaction remains HL-01.
Identical layouts can describe different pressings; preferences remain descriptive
and do not claim catalog/audio verification. Missing provider data remains missing.
