# Layered library refresh and demand-driven playback

Architectural rules: [ADR 0013](decisions/0013-bounded-projections-and-client-refresh-state.md), [ADR 0014](decisions/0014-playback-delivery-separate-from-integrity.md).
This document retains detailed requirements, rationale and evidence; the ADRs do
not replace its supported-scope and recovery limits.

Contract and source implementation, 2026-11-02, for the Collector web interface
and remote players such as Trash Amp. [Delivery evidence](validation/layered-library-refresh-2026-11-02.md)
records validation and limits. Merge/deployment follow the single assignment in
[remaining work](remaining-work.md).

Library refresh means observing published collection state. It does not scan an
archive, refresh providers, prepare audio, validate integrity or download music.
Those are separate explicit commands and background workflows.

## Implemented shape and scope

Counts are independent `GET /api/v1/client/library` (listening albums) and
`GET /api/v1/library/summary` (web scope) requests. Both return `total_items`,
`total_is_exact` and `catalog_revision`. Pages use the existing album/Library routes,
`revision`, `max_bytes` and self-contained cursors; they do not repeat totals or
store where a client has reached. The generated OpenAPI is the exact DTO authority.

Album rows carry artwork identity/revision and a row revision. Track validation
is deliberately a separate on-open header, not an inline track manifest. The new
`client-tracks/summary` and `client-tracks` routes expose pending state, track revision,
availability-row count and bounded pages. The count is of availability rows (including
format variants), not an invented logical-track total. Explicit album-play commands
retain the existing source-selection snapshot policy; library refresh does not call
that detailed route. JSON cache validation uses opaque revisions; thumbnail/media
responses additionally support private ETag validation.

Catalog queries lease bounded read-only SQLite connections. The local catalog
and attached private state must use WAL so a query can read committed rows while
an import writes. Library SQL page selection and cursor validation use one short
snapshot. The service rechecks current authorization and the membership fence
before returning rows. Native clients that submit a full catalog revision retain
that stronger refresh fence.

On a configured remote archive, local-state requests must use the bounded-freshness
mount health result. The archive guard must still check the live mount for
archive access and publication. A failed probe must latch unavailable, and an
overdue probe must make local guarded requests fail promptly. Archive access
itself remains subject to the host's filesystem behavior.

The web first page and count are independent requests. The web client must use a
count only with matching catalog revision. It may refresh up to 50 already
visible rows per request through the ordinary Library projection using IDs and
the current membership revision. Cursor fences must cover ordering, membership
and authorization. Updates to a derived summary may leave ordinary cursors
usable. Filters whose membership depends on workflow or readiness must fence
those changes as well. A changed membership revision requires a fresh page.
The server retains no client refresh session.

Trash Amp owns a complete manifest, transient pages and per-album caches locally.
It re-queries counts/pages for changed generations or pending details, with bounded
cancellable waits and visible timeout errors. Selected-album headers retry pending
work independently. Title publication merges concurrent album loads at commit;
artwork/playlist reads and online song searches use their own paths. All of this
is client state; Collector owns only its shared derived projections.

## User-visible contract

1. Establish trusted server identity and check the current login independently of
   the library. Show connected, authentication required, forbidden, unreachable or
   identity mismatch distinctly. A reachable control server does not prove that
   its media route or archive is available.
2. Fetch an authorized count independently when the client needs a total. Render
   the first page as soon as it arrives. An empty library is a successful result,
   distinct from a failure.
3. Fetch compact album title/artist rows in bounded pages; render each page as it
   arrives. Rows include small artwork and row revision descriptors, never
   track arrays, media payloads, provider responses or full item graphs.
4. Fetch missing/changed thumbnails independently, prioritizing visible rows.
   Existing valid artwork remains usable. Artwork failure does not fail titles.
5. On album open, revalidate its track descriptor and fetch track pages only when
   absent or changed. Album play/queue/download is also an explicit demand for
   those tracks. Ordinary library refresh never enumerates every album's tracks.

Each control request and thumbnail request should complete within one second on
the supported healthy installation; target server work below 500 ms p95, with a
750 ms internal work budget leaving response overhead. Measure client-visible
latency as well. WAN latency, throughput and unreachable hosts cannot have an
absolute one-second success guarantee. Bound client waits and show their outcome;
do not turn an exceeded deadline into a success or silently serve stale data as live.
The entire multi-page refresh or all artwork transfers may take longer than one
second. Each page is independently useful. A full track transfer naturally takes
as long as the consumer/network needs; measure first-byte latency separately.

Published data stays readable during unrelated work. If a directly required
projection is being rebuilt, return its typed pending/retrying state promptly with
the safe basic row or a typed pending detail response. Include bounded retry guidance;
never hold a request waiting for an encoder/provider/whole-library rebuild.
Missing tools/modules cannot hide independently authorized published media.

## Baseline findings before implementation

These dated findings motivated the changes above; they describe the pre-delivery paths, not current gaps or a live-host performance diagnosis.

| Surface | Current behavior | Required change |
| --- | --- | --- |
| Web Library | `CollectionService::library_entries` uses basic/cached rows built by a separate worker; role/language/token fences exist | Retain this foundation; add count, response-byte limits, reusable revisions and conditional reads |
| Native album list | `listening_albums` calls `browse_presented`, which hydrates captures and resolves/presents each full item | Read the same compact projection/query service as web, with listening eligibility and a player-safe row shape |
| Trash Amp refresh | `Adapter::refresh` serially loads collections, album pages of 20, every album's `music_tracks`, artwork and playlist contents; persists one final snapshot | Persist/render title pages progressively; independent artwork work; demand-loaded tracks and list entries |
| Album playback | `album_playback_snapshot` recomputes availability and full item presentation; returns all tracks without a cache validator | Versioned bounded recorded track projection, with separately paginated playback snapshots |
| Native transport | `decode_request` buffers up to 8 MiB; thumbnail reads allow 2 MiB; neither offers conditional-result handling | Explicit small catalog contracts and typed modified/not-modified/pending results; cancellation and stage deadlines |
| Core media | `file_response` invokes `verify_transfer_bytes` for prepared-media GET before range selection/body delivery | Remove playback whole-file verification; keep exact ranges and bounded body reads |
| Private playback | `local_playback` invokes `verify_open_reader` before its first prepared-media read | Remove that prerequisite, including cached-reader initialization |
| Direct media service | `collector-media` hashes a prepared file before starting a GET body | Remove that prerequisite too; preserve grant rechecks and resource bounds |
| Store admission | Catalog/private state uses one guarded SQLite connection; lock admission probes archive state | Prevent slow archive probes or long writer transactions from stalling every read; retain fail-closed mount safety |

Relevant owners: `collector-app/src/library.rs`, `disc_audio.rs`, `music_search.rs`;
`collector-store/src/library.rs`, `browse.rs`, `storage_guard.rs`;
`collector-server/src/lib.rs`, `local_playback.rs`; `collector-media/src/lib.rs`;
`collector-client/src/library.rs`, `media.rs`; `web/src/app.tsx`.
Sibling changes belong in Trash Amp's `amp-collector` adapter, `amp-app` catalog
model/source notifications and `amp-ui` album/track loading and playback handling.
Native source and dependency qualification are recorded in the linked delivery
evidence; use the client repository's current dependency pin rather than a temporary
implementation checkout.

## Boundaries and projections

Keep one application-owned catalog query service, built on the existing Library
projection, rather than porting the slow native resolver to another route. Shared
player read DTOs live in `collector-api-types`; web rows can add separately composed
curation fields. Authorization, effective language, listening inclusion and pending
semantics belong to application policy. SQL selection, projection storage and HTTP
serialization belong to adapters. No new domain dependency on I/O is needed.

Store compact effective title/artist/sort/search fields, listening eligibility,
artwork descriptor, and track revision token with the item generation. Update cheap
inventory facts transactionally with publication; compute expensive recorded
presentation asynchronously, fenced by source generation, role, language and
module/presentation policy. Pending is explicit and may not reuse stale artwork
or playback conclusions under a new revision. Count, filtering and paging must
use the same eligibility predicate and safe fallback policy. Game-disc listening
entries remain eligible; this is not just a music-module filter. Artwork remains
beside game/CD rows, in search and retained offline views.

Index selection/counting on projected fields. Avoid per-row full graph loads,
provider reads, media stat/open/hash, and repeated operational queries. Batch live
import/progress overlays for selected IDs. For derived filters such as playback
attention, expose pending counts explicitly rather than asserting that unknown
rows do not need attention. Count generation changes when relevant inclusion does.

Catalog reads have bounded connection-lock admission and do not hold writer or
publication gates while serializing or transferring bodies. Reads/metadata policy
stay separate from archive publication. A separate read pool and isolated filesystem
health probes were considered, but are not prerequisites added by this delivery.
The existing exact archive-binding checks are retained; stalled-kernel I/O and
actual remote-storage latency remain explicit qualification limits.

## HTTP contract

The table is an overview; generated OpenAPI defines exact fields and query parameters.
Use existing `/api/v1` surfaces and update consumers together under the
[pre-release policy](decisions/pre-release-evolution.md); no compatibility wrapper
is needed solely to keep old checkpoints working.

| Request | Result and cost |
| --- | --- |
| `GET /api/v1/identity`, then authenticated `GET /api/v1/me` | Reuse identity/trust and account validation; no catalog enumeration. The new count/header routes are the layered-catalog surface |
| `GET /api/v1/client/library?collection_id=…` | Authorized listening `total_items`, `total_is_exact`, `catalog_revision`; normalized search/filter scope |
| `GET /api/v1/library/summary?…` | Same count contract for the web scope, including general entries and filters |
| `GET /api/v1/client/albums?…` | Compact listening rows from the shared query service; add `catalog_revision`, opaque cursor and bounded `max_bytes` |
| `GET /api/v1/library?…` | Web collection rows using that same service, count/revision/paging rules; retain general collections, search, filters and curation fields |
| `GET /api/v1/collections/{collection}/assets/{asset}/thumbnail?revision=…` | Existing thumbnail route with revision-bound lookup, private ETag/304 behavior and bounded prebuilt thumbnail; no regeneration in this GET |
| `GET /api/v1/items/{item}/client-tracks/summary` | Lightweight `tracks_revision`, availability-row count and `pending`; no track enumeration |
| `GET /api/v1/items/{item}/client-tracks?revision=…&cursor=…` | Revision-consistent availability pages and pending state; numeric continuation requires its matching revision |
| `GET /api/v1/items/{item}/client-playback?…` | Existing explicit source-selection snapshot for album play/queue; never called during title refresh |
| Existing media HEAD/GET and private-media reads | Recorded metadata or requested byte stream, current authorization, no pre-transfer whole-file verification |

The summary's count covers the normalized result scope. The derived playback-attention
filter marks its count non-exact because pending summaries and live ingest overlays
can change final inclusion; clients must not present that count as an exact denominator. Sort is bound into
paging scope but does not change count. A web caller can request an all-entry count;
a player caller requests listening entries. These use one counting mechanism.
Collection descriptors and playlist summaries must also have bounded pages where
needed; list entry downloads are independent of album title refresh.

Default title/track page target: **64 KiB uncompressed JSON**, hard maximum
**128 KiB**, and at most 200 rows. `max_bytes` may lower the target within documented
bounds; the server's hard cap always wins. Include envelope/cursor bytes when
measuring. Compression does not relax limits. Select with a bounded row limit,
serialize incrementally into a bounded buffer, and stop before the next row would
cross the budget. Always advance by the last returned row. If a single row/continuation cannot fit the requested bound, return a typed
invalid-request error so the client can request a larger allowed page. Do not
truncate accepted text, silently omit entries or return a nonadvancing cursor. Define these bounds in shared DTO validation
and OpenAPI, with exact-byte boundary tests.

Native album rows expose item/collection IDs, title, artist, `row_revision`,
artwork asset/revision and `summary_pending`. Web rows retain their existing
curation/presentation fields. Published track rows retain source/track coordinates,
title, duration/profile, typed availability and media identity/length/digest.
No archive paths, provider payloads or private Jobs cross that read boundary.
A digest is recorded content identity, not a fresh verification claim.

### Counts and stateless pagination; clients own progress

Collector reports collection facts and answers independent read requests. It does
not create or manage refresh sessions, client cursors in storage, progress Jobs,
refresh attempt IDs, client counters, cache inventories or completion receipts.
Fetching a count or first page allocates no client-specific refresh state. Existing
login sessions remain authentication only. Catalog revisions identify shared
published state; they do not identify a client's loading session.

A client that wants progress must query the count for its authorized, normalized
scope. That response includes the count, catalog revision and whether the count
is exact (or a typed pending result when it cannot yet be determined). Album page
responses contain the rows, catalog revision and a self-contained continuation
cursor or end-of-results indication. The client counts accepted rows itself and
can show `120 of 400 albums` after receiving 120 rows against that count. Pages do
not need to repeat the count or report how much the client has already loaded.
An album's track count and track pages follow the same independent-read pattern.

Simple `page N of M` metadata is acceptable where it can be computed statelessly
and accurately. Byte-bounded pages contain varying numbers of rows, so do not infer
a page total by dividing the count by a nominal row limit. Row count plus a
continuation cursor is sufficient; no server-maintained paging session or exact
page-count prepass is required. Revisions and query scope in the cursor allow the
server to reject stale/mismatched requests without remembering earlier requests.

Clients own progress presentation, cache comparisons, deduplication, retries,
cancellation and partial-refresh recovery. They may count unique accepted rows
against the queried total, compare artwork descriptors with their own cache and
track their resulting artwork requests. Collector does not accept a client's
progress report or return per-client succeeded/reused/failed/pending counters.
Artwork discovery and selected-album track loading remain client decisions; do
not enumerate all tracks merely to obtain an overall refresh denominator.

Clients must not combine a count from one revision with pages from another, or
report a failed/incomplete enumeration as complete. They handle a revision conflict
by querying the count again and restarting their own enumeration as appropriate.
An exact zero count is a valid empty result. A client without a count can still
browse pages, but cannot claim a determinate library-wide percentage. Typed pending
and error responses describe the requested resource, not a refresh session. Retry
guidance may accompany those responses without tracking whether a client retries.

### Revisions, conditional reads and consistency

- Cache identity includes enrolled server identity, account, collection/item,
  effective access/role, language and presentation-policy version. A URL or title
  alone cannot identify cached content. Authentication/access checks run before
  every 304 or pending response; caches confer no permission.
- Advance a catalog revision on visible row/membership changes, including title,
  artwork, track token, relevant pending-to-ready changes, grants and language.
  Reindex/reset must use a new epoch; an old revision cannot silently regain meaning.
- Bind cursors to query, sort, scope, catalog generation and last sort-key/ID.
  Use generation-fenced keyset pages, not a long-lived SQLite transaction. Count
  and first page refer to the same generation. On change return typed 409
  `conflict`; the client restarts with backoff while retaining its last
  complete cache. Never mix generations and label them a complete refresh.
  Under continuous relevant updates, show progress/pending rather than spinning.
- Keep progressive staging separate from the last complete manifest. Show new
  pages as live partial results with loaded/total progress, but remove unseen old
  entries only after successful complete enumeration. Interrupted refresh never
  wipes the last complete catalog. A direct access denial immediately hides the
  denied online scope; retained offline-copy policy remains explicit.
- The on-open track token covers inventory, recorded layout/tags, selected source and
  disc inclusion, effective/manual metadata, preparation publication, visibility,
  role/language and policy version. Metadata-only edits invalidate track text;
  artwork-only edits need not invalidate tracks. Media byte identity is separate,
  so changing a title does not discard a valid offline audio file.
- Album open revalidates its lightweight header even when the title manifest is
  old. Track pages require the same track revision and reject supersession.
  Current role/language/item binding is rechecked; unrelated catalog rows do not
  invalidate a selected album's track pages. No incomplete page set becomes a
  complete album cache or queue.
- Artwork descriptor changes invalidate only that rendition. Do not serve current
  bytes at an older revision URL. Return a stale-resource error if the requested
  version is unavailable. Use authenticated private caching; never public caching
  of role-sensitive art. Support `If-None-Match` after authorization.
- Thumbnail creation/replacement happens at publication or in bounded background
  work. Missing/pending artwork has a placeholder and retry state. Use the existing published rendition; the native reader retains its existing
  2 MiB transfer bound. Original artwork stays preserved and is not fetched for
  title rows. A new universal 128 KiB rendition policy is not introduced here.

A delta feed is optional later. Revision-validated summary/page/track requests and
revisioned thumbnails solve this delivery without a permanent change-history store.

## Playback is consumption, integrity checking is separate

The user explicitly confirmed that **playback must not inject whole-file
verification before consuming media**. `verify_on_transfer` and its three server
pre-read gates have been removed, along with the native offline pre-read hash. Do not replace it with a mandatory chunk-hash
sidecar, validation Job or delay of the first range. Prepared and original tracks
follow the same demand-driven consumption rule.

On a media request: authorize the exact target and route, safely open the pinned
file, check recorded length against available file metadata, seek if requested,
then stream bounded chunks with backpressure/cancellation. HEAD does no body read.
A small range reads that range, plus any strictly bounded mechanism overhead;
it never reads the complete file for verification or buffering. Browser decoder
probes/seeks and native 256 KiB range-reader chunks remain normal consumption.
An unrestricted GET streams progressively; it does not materialize a whole track.
Retain correct 200/206/416, Content-Length/Content-Range, If-Range and immutable
representation validators. A validator denotes published byte identity, not a fresh
integrity attestation; same-length silent storage corruption may be heard before an
independent integrity check detects it. This is an accepted playback limitation.

Import/publication and conversion workers still validate their actual inputs and
outputs with cancellation before recording successful results. Explicit integrity
Jobs may read/hash full files. Downloads read all requested bytes and may compute
an incremental client digest, finalizing a retained offline copy only after length
and digest match; no server-side verification pass is a prerequisite to transfer.
Do not add new download verification infrastructure as a playback dependency.

### Read failures belong to the media request

Before headers are sent, return a structured, sanitized error for missing files,
open/seek failure, length disagreement or storage unavailability. Keep unauthenticated,
forbidden and revoked distinct. Define stable error codes in the shared contract;
human-readable text does not drive client behavior. Do not expose filesystem paths.
Do not disguise a read error as an empty successful stream, automatic preparation,
or a reason for the catalog refresh to fail.

Once headers/body have started, HTTP cannot change the status or append a JSON
error to the audio. Abort the stream on read failure, retain the correct declared
length, and record a correlated server diagnostic. Private chunk protocols return
their typed per-read failure; native transport maps premature EOF/connection loss
to a file-read failure. Browser media errors may be less specific: report the
available failure without inventing a server diagnosis or fetching the whole file.

Playback policy belongs to the consuming client. For Trash Amp, record the failed
server/file/representation and playback attempt, show the failure, and continue to
the next queued track. Fence late callbacks from the old attempt, stop/release the
old reader, and bound skipping so a failing queue cannot retry forever. Do not
permanently mark the album/library unavailable or delete cached metadata/downloads.
A later explicit play may retry. Authentication/revocation should surface a
connection-level problem and avoid hammering every track with denied requests.
Web playback should likewise show the media failure and offer retry/skip through
its existing player; server code never chooses the next track.

## Client delivery

Web: reuse existing read scopes/abort controllers; separate connection, count,
titles, artwork and selected-album state. Refresh pages progressively rather than
publishing only after the requested page loop finishes. Keep valid rows visible
with explicit refresh/error state. Use revisioned thumbnail URLs and lightweight
track revalidation. Pending-summary polling stays cancellable, bounded and separate
from playback; refreshing the Library must preserve its media element and position.

Trash Amp: replace the all-or-nothing `Snapshot` refresh dependency with a complete
manifest plus staged pages and per-album track caches. The catalog model must
represent `not_loaded/loading/ready/failed` tracks explicitly; an unloaded album
is not an empty album. Emit source updates after each phase/page, preserving source
selection, current queue and originating server coordinates. Artwork work uses a
separate cancellable worker with bounded responses, caches by rendition revision
and does not block count/title completion. Playlist entry loads follow list-open/play
demand. Song search uses Collector's paginated music-search API online; offline
search covers retained metadata and reports that coverage, without eagerly loading
all tracks merely to preserve a misleading complete-search claim. Explicit offline
album retention obtains its track text/artwork/audio and preserves existing downloads.

Share transport/DTO/conditional-response handling in `collector-client`, not duplicate
application orchestration between clients. Keep catalog, artwork and media request
deadlines separate. Source/account/query changes cancel obsolete reads and fence
every completion before cache publication; abandoned requests cannot overwrite
the next selected source. Offline/cached, refreshing, complete, partial and failed
remain distinct from online route availability.

## Complete delivery and acceptance

Implement one cross-layer delivery: shared projections/revisions and count/pages;
HTTP/OpenAPI/generated types and client handling; progressive web/native catalogs,
lazy tracks/artwork; all playback paths and client file-failure behavior. Preserve
existing downloads and meaningful archive records. Rebuild disposable projections;
version/convert private client caches explicitly without removing retained audio or
unresolved playlist-command receipts. Do not add archive migrations for this cache.

Acceptance must prove behavior rather than merely time an ordinary fixture:

- Instrument test ports/readers: count/title/artwork descriptors/track metadata and
  HEAD cause zero media-body reads; a tiny range in each media transport does not
  inspect the file tail. Test sparse/large prepared files as well as originals.
- Multi-page multibyte titles hit exact byte budgets; one oversized row, empty
  library, last-page count, duplicate sort keys and invalid cursors remain correct.
  Concurrent title/grant/language changes and reindex cannot mix generations,
  leak redacted rows or erase a complete cache after partial failure.
- Repeated refresh makes no per-album track calls and no unchanged-artwork GETs
  when descriptors/caches are valid. Album open gets unchanged/pending/changed
  results correctly; manual track edits and newly prepared audio invalidate it.
- Verify count and page requests independently, including interleaved clients
  with different scopes, empty results, unequal byte-bounded pages and stale
  cursors. No refresh session or client-progress state is created or required.
  In web/native UI, a client that queried the count can show its own loaded/total
  progress; failed pages and revision changes cannot produce false completion.
  Cache reuse, artwork failures and retries remain client-owned state.
- Measure per-phase/per-page/first-byte p50/p95/max, response bytes, media read
  bytes, query counts and lock waits on the intended NAS installation at its real
  scale and a 3,000-album fixture. Include cold projection backfill, active unrelated
  integrity/encoding work and blocked storage. Report healthy success latency and
  prompt pending/unavailable outcomes separately; no mean hides multi-second tails.
- Open, seek, truncation and mid-stream read failures produce real request/stream
  errors. Trash Amp records the exact failed file and actually advances to a good
  next track; a fully failing queue terminates without a loop. Stale attempt errors
  cannot skip a newly selected track. Explicit retry can recover repaired storage.
- In real web/native UI, count and titles appear before slow art; an unopened
  album fetches no tracks; rapid source changes and mid-refresh disconnect preserve
  valid cached text/artwork and source choice. Verify mobile wrapping/accessibility,
  persistent web playback, real seek/download/offline use and immediate post-edit
  revalidation. Auth/grant/device revocation still denies the next uncached request.
- Run [change-based checks](testing.md), affected sibling tests and strict Clippy,
  generated-contract checks and the relevant browse/media/list/storage browser
  journeys in the supported launch context. Broaden workspace qualification for
  shared contract, authorization and storage changes. Deploy only the resulting
  validated release artifacts when deployment is separately requested.

Source validation and its limits are recorded in the
[dated delivery record](validation/layered-library-refresh-2026-11-02.md). Historical
baseline findings above do not claim that their old paths remain in use.
