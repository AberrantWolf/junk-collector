# Wave 4 review — D04 checkpoint

Reviewed 2026-09-13. This covers D04.2–D04.3 on top of the previously reviewed
D04.1 and CT changes. It is not acceptance of the remaining Wave 4 work.

## Architecture assessment

The shared reader algorithms, game-owned matching policy, application authorization
and frozen jobs, provider parsing, and archive publication use their intended
boundaries. No new checkpoint compatibility layer or archive migration was needed
for the review fixes. Private active-snapshot selection remains operational state;
portable snapshots and observations retain their identities through reindexing.
The DAT row view is rebuilt in memory for each comparison. It is bounded and
reproducible, but is not a persistent search index or a qualified large-corpus path.

## Corrected findings

- Snapshot parsing checked metadata but not exact payload identity. A changed ROM
  checksum with unchanged header metadata could be attributed to the old SnapshotId.
  Every parse now verifies complete length and SHA-256, including trailing bytes.
  Acquisition checks derived-row size limits before making a snapshot available.
- The shared DAT parser silently skipped malformed ClrMamePro ROMs and accepted
  truncated XML/text catalogs. It now rejects those inputs and missing file lengths;
  explicit nonempty XML ROM elements are parsed too. The sibling parser change is
  separate from Collector integration and is checkpointed in Junk Libs `6350026`.
- Missing checksums in one plausible DAT row were masked by nonmatching checksums
  elsewhere in the catalog. Such comparisons now stay inconclusive.
- Processing-history reads incorrectly required an installed writable module.
  Curator authorization now permits historical reads independently of write
  eligibility, with a missing-module regression. Viewer/other-collection denial
  remains in place.
- Long hashing and matching loops did not observe cancellation until completion.
  Application-owned package readers check the job lease/cancellation on reads;
  domain scans use host checkpoints. Executors classify cancellation before
  interpreting an operation error or publishing an observation.
- The new UI used numeric file indexes without names, omitted older history, and
  displayed nested import observations generically. It now shows package names,
  supports older-history paging, and labels nested results. Currency wording says
  recorded source, avoiding a claim that opening history rehashes payloads.
- A newly created Games collection was absent from the open admin catalog panel.
  Collection creation refreshes its list immediately. The browser smoke suite now
  imports and activates a real synthetic DAT through that panel.

## Qualification and next handoff

Synthetic fixture acceptance does not qualify owned real dumps, large DAT corpora,
real mixed-mode CUE inputs, Linux/NAS hosting, or runtime CHD exposure. Those limits
remain explicit. No new catalog source is downloaded or user archive modified by
these tests. See [validation](validation.md) for commands and results.

D05.1 is the next bounded ticket: narrow integer PCM/offset/checksum primitives
with independent vectors and bounded readers. Stop before D05.2 networking.
Preserve the integrity, cancellation, curator-read, unknown-data, and recovery
contracts above; do not copy application orchestration into the shared libraries.
