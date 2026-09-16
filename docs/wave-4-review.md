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

## D07.3 implementation handoff — 2026-09-15 (review pending)

D07.3 is implemented locally without changing the reviewed D07.1/D07.2 recipe,
encoder or publication contracts. One application read DTO now projects ordered
source Representation, track position, exact integer PCM start/count, safe title,
profile and preparing/unavailable/available state. Available rows carry immutable
Representation/package-file identity, length, media type and digest. Private Job
identity/diagnostics, raw Evidence and archive paths are absent. Retained published
recipes keep their authorized rows readable after restart when the music module
and encoder are unavailable.

The server owns optional `[encoders]` configuration and probes each configured
binary for its complete version report. Client input selects only a fixed profile.
The client router exposes availability and the existing authorized opaque media
reader; the curation router alone exposes explicit one-track preparation through
existing receipts and Jobs. Disabled public curation keeps this command absent
despite forwarded headers. Import and media GET do not convert. The web curation
surface defaults to FLAC, offers explicit Opus 96/128/192 choices, explains absent
tools, polls durable progress and refreshes published availability.

Actual HTTP tests cover viewer reads, curator-only preparation, cross-collection
denial, private/public exposure, missing Opus, real FLAC completion and independent
source-PCM validation, restart reads without module/tool, revocation, strong digest
ETags and exact full/partial/suffix/open-ended/unsatisfiable/stale-or-weak If-Range
responses for originals and derivatives. OpenAPI/TypeScript contracts and embedded
web assets were regenerated. This is an implementation handoff, not D07 root
acceptance. Real `opusenc`, owned-disc and deployment qualification remain unrun.
Browser playback, seek and automatic/explicit track transitions belong wholly to
the next bounded player child; a successful browser fetch is not that acceptance.


## D07.3 review corrections — 2026-09-15

Reviewed and corrected locally. Fixed HTTP preparation replay after encoder
configuration changes: the receipt now binds the submitted item/Capture/track/
profile request before source selection or encoder resolution. Both preparation
entrypoints share the frozen-job effect, without nested receipts. HTTP regression
coverage replays the accepted request with no configured tool and rejects changed
input under the same key. Deleted-item preparation is rejected before tool work.

Availability now carries a typed `can_prepare` flag combining actor permission,
module/schema support, readable source and configured encoder. The UI no longer
searches English reason text to discover behavior. Unsupported/missing modules
cannot advertise preparation through retained recipes. Read-time music parsing
is owner-gated; missing Collector recipes on imported derivatives do not break
all track availability. Each retained recipe is loaded once per request rather
than repeatedly inside the track/profile search.

Fixed the unserved generated stylesheet by including music styles in the existing
served CSS build. Replaced the unbounded component polling loop with cleaned-up
request/timer effects. Availability resumes polling for already-running jobs after
reopening; completed outputs refresh, failed preparation stays visible, and a
manual refresh can recover a transient read failure. Removed internal PCM frame
coordinates from the user flow while retaining them in the typed API contract.

The isolated Chrome smoke workflow passed outside the filesystem sandbox,
including actual FLAC preparation/download, byte ranges, stylesheet application
and persistent error display. Earlier sandboxed Chrome launch failures remain a
historical environment result, not an outstanding UI acceptance gap. No sibling
code was changed and no commit was made. Actual browser playback/seek/queue
qualification belongs to the next bounded browser-player child; it is not inferred
from successful downloads. Root D07 local implementation is reviewed; external
opusenc, owned-disc, deployment and scale qualification remain outstanding.

## Bounded browser-player implementation handoff — 2026-09-15 (review pending)

The bounded child is implemented locally on top of the reviewed D07.3 contract.
It adds no Rust service, route, media endpoint, conversion trigger or persistent
queue state. A shared web runtime lets the client bundle mount the existing music
availability component directly; the management build still externalizes that
runtime and retains its curation-only preparation controls. Styles remain composed
through `web/build.mjs` into the served `app.css`.

The viewer player derives its in-memory queue only from authorized ordered
availability rows. It groups by Capture and source Representation, retains each
track's profile rows together, prefers an actually browser-decodable FLAC, and
uses the immutable available Representation/file media URL. It provides play,
pause, approximate seek, previous/next, current state and natural ended advance.
Unavailable codec and media failures provide download/open guidance. Availability,
Job and preparation effects abort their requests and timers on replacement or
unmount; closing an Item also pauses, clears and reloads the media element so an
obsolete ranged request cannot continue. Typed `can_prepare` remains the only
preparation eligibility signal.

The real two-track browser workflow passed in Google Chrome 153.0.8010.37 with
`audio/flac=probably` and actual decoded media-time progression. It used two real
`flac 1.5.0` derivative Jobs and a Viewer account, not a URL assignment or download
as a playback proxy. Pause/resume, a 1.5-second seek, immutable next/previous IDs,
natural ended advance, clean reload, mobile layout, close cleanup and grant
revocation passed. The local-setup run repeated playback from the client-only
origin with curation exposure disabled and no `curation.js` load. Existing D05 and
D07.3 curation preparation/error/download checks remain in the same workflow.

This is an implementation handoff, not self-review. Real opusenc, owned-disc,
physical audio output, other-browser, gapless/sample-accurate, deployment and
scale qualification remain unrun. Review this child before beginning D06.1; C01
and native C04 were not started.

## Bounded browser-player review corrections — 2026-09-15

Reviewed the client/runtime integration, Capture/source grouping, profile selection,
media events and request lifetimes. The player consumes existing authorized DTOs
and opaque media routes without changing backend publication, policy or queue
contracts. Native/shared playback services remain separate from this temporary
browser presentation state.

Fixed ref-detachment cleanup: passive-effect cleanup can observe an already-cleared
React ref. The stable audio ref callback now pauses the actual old element,
removes its source and releases pending media work. Item identity keys prevent
playback state crossing item changes. Tests retain the detached element and check
its paused/source state after closing during real playback and after session loss;
DOM disappearance alone is no longer treated as sufficient evidence.

Added generation fencing for delayed play rejections after switching tracks,
pausing or unmounting. Stale pause/ended events cannot overwrite a newer load.
Refreshed availability invalidates mismatched loaded media and out-of-range
selection, and missing-item responses clear stale playback. Aborted API mutations
are not needlessly retried. Corrected stored-file labels to distinguish originals
and derivatives; derivative inline controls no longer duplicate the disc player.

Browser regressions cover real viewer playback, natural ended advance, rapid
switching with an injected obsolete play rejection, active close, actual session
loss, grant revocation, unsupported-codec download guidance and retained playback
without the module. The fault injection tests exercise controller behavior; real
FLAC decoding/time progression remains a separate requirement. See the dated
validation entry for the exact commands and outcomes.

No commit or sibling edit was made. D07.3 plus this player is a useful backup
checkpoint before the ready-to-send D06.1 metadata-authority handoff. Other-browser,
real Opus, physical-output, gapless/sample-accurate, deployment and scale claims
remain outside this local qualification.

## D06.1 review corrections — 2026-09-15

Reviewed the typed metadata contracts, resolver, command receipts, archive
publication and reindex. Corrected the following before handing off providers:

- A repeated provider/record/version could replace previously selected content.
  Candidates now retain immutable content; changed versions must have new IDs.
  Both publication and reindex reject contradictory observations. Observation IDs
  are subject-bound and distinct from transaction/receipt identities.
- Inventory edits after the first authority revision could leave displayed text
  stale. Records now freeze inventory inputs; application reads/commands recognize
  subsequent manual corrections, while publication checks the inventory revision
  under the writer lock. Unchanged fields preserve explicit overrides and clears.
- Manual artwork/subtitle/domain choices could hide automatically ambiguous
  identity. Automatic ambiguity remains tentative; missing candidate subtitle
  falls back to inventory text when no candidate exists.
- Refresh artwork references now require an active asset belonging to the item;
  deleted items reject metadata mutations. Legacy selected record IDs only select
  a candidate when the provider match is unambiguous.
- Materialized display is checked against frozen inventory and candidate inputs
  before publication and during reindex. Effective reads no longer return a stale
  materialized record or independently reread authority.

Added resolver and archive regressions covering contradictory candidate/observation
identity, forged display, stale inventory publication, later inventory corrections,
ambiguity with manual artwork, and restart/reindex behavior. Existing authority,
unknown/raw history, revocation and interrupted receipt recovery tests remain.
No providers, routes, frontend changes, sibling extraction or commit in this review.
D06.2 is the next bounded implementation; the ready-to-send prompt was updated.

## D06.2 review corrections — 2026-09-15

Reviewed provider composition, shared acquisition/Asset/refresh effects, module
interpretation, frozen snapshots, network bounds and candidate history. Corrected:

- Acquisition retry receipts included newly generated timestamps. Provider commands
  now bind submitted item/query first and share Job-spec construction without
  nesting another independently committed command. Provider operations have separate
  deterministic job namespaces. Changed-input replay rejects before acquisition.
- Adding artwork or changing match strength reused an immutable candidate version.
  Versions now cover those inputs. Current identities are retained separately from
  historical candidates, so automatic resolution uses refreshed versions while
  explicit historical selections survive. Application DTOs expose that distinction;
  reindex validates the frozen current selection and materialized display.
- HTTP DNS preflight could validate different addresses from the eventual connection,
  and ambient proxies could bypass the check. The actual reqwest resolver validates
  addresses under the request timeout; proxy discovery is disabled. Private/mapped
  IPv6, link-local, shared-address space and reserved IPv4 are rejected. CAA permits
  narrowly matched HTTPS Archive storage hosts, not arbitrary redirects/subdomains.
- MusicBrainz caches were unbounded and cache hits ignored smaller caller limits.
  The cache now prunes expiry and enforces 128 entries/32 MiB; negative outcomes
  use the negative TTL even when HTTP status is 200. Admission spans dispatch and
  response headers, and Retry-After contributes to the shared cooldown.
- Empty/malformed successful bodies could look like no-match; successful envelopes
  now require their collection fields. Snapshots retain HTTP status for replay.
  Non-JSON 404 bytes remain in snapshots and metadata references them without
  inventing JSON or failing an otherwise valid negative refresh. English ISO 639
  codes normalize for preferred-language ranking while raw language remains retained.
- Local HTTP fixture teardown could hang after assertions failed. The archive fixture
  now stops boundedly and explicitly uses blocking accepted sockets on macOS.

Regression coverage includes actual acquisition replay, changed-input conflicts,
cache-limit enforcement, candidate rank/artwork versioning, current/history selection
through reindex, denied destinations, malformed success and non-JSON negative
refresh. Failed image decoding still leaves manual artwork usable. No metadata
routes/UI, new provider, sibling extraction or commit was added. D06.3 is next.

Reviewed the official [MusicBrainz API](https://musicbrainz.org/doc/MusicBrainz_API)
and [Cover Art Archive API](https://musicbrainz.org/doc/Cover_Art_Archive/API), including
CAA listing/image routing through Archive storage. These documentation checks are
not live-provider or deployment qualification; deterministic local fixtures remain
the exercised network surface.

## D06.3 local implementation — 2026-09-16 (awaiting review)

Implemented the bounded metadata review child without beginning P05, native or
offline work. The review should inspect these surfaces:

- Application-owned curation routes now expose safe review, authority mutation,
  provider acquisition, snapshot refresh and CAA artwork publication. Generated
  contracts carry typed provider availability/guidance, server-owned lookup choices,
  permissions, safe candidate history and catalog-snapshot Job results. Raw provider
  observations, private inventory baselines and unreviewed URLs do not enter viewer
  DTOs.
- MusicBrainz/CAA workflow commands reuse the D06.2 acquisition registry, receipts,
  Asset effect and metadata refresh effect. Unconfigured providers fail before a
  doomed Job is created. CAA publication precedes the refreshed candidate artwork
  link, and failed/absent providers leave inventory and existing artwork usable.
- Presented items and FTS use effective metadata. Curators additionally receive
  explicit inventory title/subtitle fields for the inventory editor; viewers receive
  neither those baselines nor private artwork. Browse/detail show tentative metadata
  independently of the missing-art placeholder.
- The React workbench uses candidate identities and Job IDs rather than array
  positions or display/error strings. It preserves field drafts across polling and
  revision conflicts, persists pending Job identity for reopen, aborts obsolete
  review loads, resumes polling, and exposes source defaults, configured-provider
  guidance, status/cancel/retry, explicit selection, overrides and clear actions.

The browser smoke uses loopback MusicBrainz and Cover Art Archive fixtures through
the real provider adapters and durable Jobs. It reopens an active refresh, retains
three alternatives and a Japanese label, preserves a draft across a concurrent
authority revision, publishes actual PNG bytes through the Asset pipeline, searches
effective text, checks shared-versus-curator artwork as a viewer, exercises keyboard
order, and captures/checks the review at 390px. Google Chrome 153.0.8010.37 passed.
Live providers, other browsers and deployment remain unrun. No commit was made;
D06.3 and the D06 root now await review before P05.

## D06 root review corrections — 2026-09-16

D06.3 is reviewed and corrected locally. P05.1 is the next serial implementation
child; this review does not implement it or claim full web curation completeness.

- Moved acquired-result completion out of React into an application dispatch that
  checks caller Job ownership, collection/Item binding and curation authorization.
  It uses one existing receipt/effect, not nested independently committed commands.
  CAA image publication preserves an Asset for explicit manual cover selection;
  it neither requires another MusicBrainz request nor silently changes authority.
- Acquisition replay now checks stored receipts before mutable provider availability.
  The panel persists completion keys/revisions, aborts every detached request,
  resumes safely on reopen and keeps failed completion available for explicit retry.
  Field drafts survive conflicts and trigger discard confirmation on close. Music
  and metadata panels have distinct stable keys through repeated refreshes.
- Reused application-owned inventory/metadata precedence for search projection on
  ordinary Item updates and full reindex. Later manual inventory corrections win,
  while unrelated edits retain resolved provider text. Reindex discards stale
  disposable metadata rows before rebuilding authoritative history.
- Lookup choices use recorded CD MusicBrainz IDs and exclude unrelated disc IDs;
  default search text respects provider bounds. Unsupported Item schemas expose
  authorized history with disabled authority controls.
- Added HTTP exposure/role checks for completion, inventory/reindex search coverage,
  and a browser regression that holds completion, closes the panel before submission,
  verifies unchanged authority, then reopens to apply the result once. The artwork
  workflow now tests explicit selection and independent subsequent metadata lookup.

Historical implementation records above describe the submitted checkpoint; this
section supersedes its browser-owned publication/re-refresh sequencing. Final
validation is recorded in `validation.md`. Live providers, other browsers and
external deployment remain unqualified. No commit made.
