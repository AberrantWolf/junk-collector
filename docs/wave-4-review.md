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

## D05.1 review — 2026-09-14

The finished agent task and local changes were D05.1, not D05.2. Provider/network
acquisition and runtime music jobs remain unimplemented.

Corrected the following before the D05.2 handoff:

- A v2-only nonzero match without a v1/frame-450 anchor could be reported as a
  mismatch. The shared result now records v2 candidate offsets separately from
  the v1 interval and returns `IncompleteSearch` when appropriate; Phono displays
  this as inconclusive. Literal independent +6 v2 vectors reproduce the gap.
- Frame-450 candidate probes could read outside a 451-sector disc at positive
  shifts. Optional anchors are skipped when their search window lacks physical
  bytes; full checksums still run. A short-disc regression covers this boundary.
- Collector duplicated integer PCM decoding/source traversal. Those mechanisms
  now live in the shared disc crate with bounded buffers and explicit timeline
  origin/range validation. Collector only maps authorized package IDs/readers.
- CUE source aliases, indexes outside stored spans, and four-channel flags now
  fail closed. Synthetic gaps retain structural results without fabricating PCM
  or disc IDs. Tests cover these cases and cross-file signed integer reads.
- Shared PCM documentation now permits bounded short reads at internal source
  boundaries, matching its implementations. Cancellation is checked at verifier
  entry as well as inside the processing loops.

The shared verifier's `Verified` status identifies at least one full-track match;
D05.2 must require all expected tracks for whole-disc success and retain incomplete
v2 coverage as inconclusive. No exhaustive v2-search or real-dump qualification is
claimed. No schema migration, HTTP surface, or new runtime capability was added.

## D05.2 review — 2026-09-14

The provider mechanism and music interpretation policy remain in their intended
layers. No runtime music capability, route, job or UI is registered yet; D05.3
owns that integration. No sibling implementation change or migration was needed.

Corrected findings:

- Partial results lost the offset used to compute their checksums when the
  disc-wide selection was cleared. Every track now retains `computed_offset`.
- Interpretation trusted supplied match records without checking the referenced
  dBAR pressing, checksum, confidence or track computation offset. Inconsistent
  records now fail closed. The result validator also rejects incomplete searches
  relabeled as mismatches and selected offsets inconsistent with computations.
- Redirect requests bypassed the provider's rate limiter. Every wire request now
  passes through it. Local tests cover same-origin redirects, rejected foreign or
  credential-bearing origins, and IPv6 loopback fixture configuration.
- Added a real shared-search-to-policy test using an independent constant-sample
  arithmetic oracle, alongside adversarial result mutation tests.

D05.3 is next. Reuse the existing acquisition executor with the bounded provider
adapter and bind results to the verified snapshot and frozen package. Local tests
are not live AccurateRip, owned-disc or complete D05 workflow qualification.

## D05.3 review — 2026-09-15

Corrected architecture and user-workflow findings:

- Replaced the second provider map/executor branch with a bounded-response adapter
  registered in the single streaming acquisition pipeline. Duplicate provider IDs
  now have one registration boundary.
- Consolidated duplicated dBAR DTOs into music-owned catalog contracts, consumed
  by the provider adapter. The dependency checker explicitly permits this inward
  adapter-to-domain dependency; domain code acquires no infrastructure dependency.
  Supported-layout-to-query policy also moved from application code into music.
- Bounded actual CUE reads, rather than trusting only the inventory's stated size.
  The private oracle now hashes payloads incrementally rather than allocating a
  full raw dump for each digest.
- The UI now displays track checksums, confidence, physical edge coverage and
  limitations, including nested Import structural details. Large candidate-offset
  lists can be expanded. A snapshot is labeled frozen, not verified audio.
- Added viewer-denial coverage to the integrated CD acquisition workflow. The
  original redumper workflow covered only a deliberately unsupported layout;
  added a supported raw-redumper package through import, verification and reindex.

D07.1 is the next bounded implementation ticket for optional playback derivatives.
D05 does not implement music playback or qualify owned dumps/live AccurateRip.
See the dated validation entry for executed checks and environment limitations.

## D07.1 review — 2026-09-15

Corrected the following before derivative publication:

- Integer FLAC comparison and Opus inspection lived inside the Collector provider.
  Extracted these reader-only algorithms into the optional `derivative-validation`
  feature of `junk-libs-audio`. Collector retains configured subprocess handling
  and recipe/preservation policy. The filesystem scratch implementation belongs
  in the D07.2 storage adapter, not application services.
- Cancellation was only observed while waiting for the child. Validation and
  final hashing now checkpoint too, and cancellation discards output. A regression
  cancels on the independent validation pass rather than before spawning.
- Failed writer/output/diagnostic workers could leave the parent waiting for the
  six-hour process timeout. Worker failures now terminate the child promptly;
  isolated Unix process groups also prevent descendants retaining pipes. Timeouts
  cannot become successful conversion results. A disk-write-failure fixture uses
  a child that continues running to prove prompt termination.
- The Opus fixture was not a valid Ogg stream and double-counted pre-skip in end
  padding. Replaced it with independently generated FFmpeg 8.1.2/libopus output:
  588 CD frames produce 640 output frames, pre-skip 312, and end padding 8 (not
  320). Inspection rejects bad CRCs, truncated headers/bodies, missing EOS,
  inconsistent stream sequence/serial/continuation/granules and unsupported
  OpusHead fields. This is structural/duration validation, not lossy PCM equality.
  The timing rules follow [RFC 7845](https://www.rfc-editor.org/rfc/rfc7845.html#section-4).
- Encoder identity now retains the bounded complete version report, including
  reported codec-library versions, instead of discarding everything after a
  numeric version token. Opus raw endianness/VBR/complexity/loss assumptions are
  explicit, following [opusenc's options](https://opus-codec.org/docs/opus-tools/opusenc.html).

D07.2 must publish validated derivatives through the existing recovery protocol,
with an adapter-owned scratch handle and a final cancellation/authorization check.
The optional FLAC library remains opt-in. No publication or playback is added by
this checkpoint; real opusenc and owned-disc qualification remain outstanding.


## D07.2 implementation handoff — 2026-09-15 (review pending)

D07.2 is implemented locally, not yet reviewed. It adds explicit per-track frozen
jobs and atomic Capture publication of output, recipe and validation Evidence.
Tests exercise synthetic BIN/CUE ownership, concurrent encoders, replay, source
and tool changes, failed validation, cancellation/failure and process kills at
all six journal phases, no-tool/no-grant committed recovery, Item edits and
reindex corruption rejection. The shared journal abort now preflights revision
markers before deleting any dependent payload, including a rename preceding its
journal-phase update. Recovery also recognizes committed derivatives whose private
job checkpoint lagged as failed with no claim token. See the dated validation
entry for commands and outcomes.

Review this boundary before D07.3. There are no new routes or browser controls,
no automatic FLAC generation, and no conversion on media GET. Real opusenc,
owned-disc, live-provider and external mastering/deployment qualification remain
unrun; structural Opus inspection is not lossless PCM equality.

## D07.2 review corrections — 2026-09-15

Reviewed the application command/frozen job, scratch adapter, Capture publication,
startup reconciliation and journal abort paths. Encoding and independent codec
validation remain outside the archive writer lock. Publication appends to current
Capture state and uses the existing journal; no new scheduler, route or migration
was required. The agent's phase-fault, process-kill and real-FLAC tests were rerun.

Fixed a retained-record consistency gap: valid output bytes and a valid validation
envelope were sufficient even when a Capture advertised a different format, role
or profile. One Store cross-record validator now serves rebuild, completed-job
recovery and concurrent-output convergence. It also checks the explicit recipe
reference and Evidence identity. A regression exercises contradictory format,
lossless/lossy role and profile with unchanged output bytes; both job recovery and
reindex reject them without re-encoding. Expanded the densely written preparation
command into ordinary formatted steps for maintainability.

D07.2 is reviewed and corrected locally. D07.3 is next; its ready-to-send prompt
now specifies server-owned encoder probing/configuration, viewer-safe ordered
availability, curation-only preparation, HTTP range tests and actual web workflow
checks. Browser queue/playback qualification remains the following bounded child.
No commit was made during this review. The D07.1 shared-library extraction also
remains uncommitted and should be checkpointed separately from Collector.
