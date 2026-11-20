# Library refresh and playback reads

This is the read contract for the web interface and remote players such as Trash Amp.
A library refresh only observes published collection state. It does not scan the
archive, query providers, prepare audio, check integrity or download music; those are
separate commands and background jobs.

## Layers

Clients refresh in independent layers, each useful on its own:

1. **Connection and login.** Check server identity, then `GET /api/v1/me`. Show
   connected, authentication required, forbidden, unreachable and identity mismatch
   distinctly. A reachable server does not prove media or archive availability.
2. **Count.** Fetch the authorized count when a total is wanted. An empty library is a
   successful result.
3. **Title pages.** Fetch compact title/artist rows in bounded pages and render each as
   it arrives. Rows carry artwork and row revisions — never tracks, media, provider
   responses or full item graphs.
4. **Artwork.** Fetch missing or changed thumbnails separately, visible rows first.
   Artwork failure never fails titles.
5. **Tracks on demand.** On album open, play, queue or download, revalidate the album's
   track header and fetch track pages only if absent or changed. A normal refresh never
   enumerates every album's tracks.

Each control and thumbnail request should finish within one second on a healthy
installation (server work target below 500 ms p95). WAN latency cannot be guaranteed;
clients bound their waits and show the outcome rather than presenting stale data as live.

Published data stays readable during unrelated work. If a required projection is
being rebuilt, the server promptly returns a typed pending state with retry guidance,
never waiting on an encoder, provider or whole-library rebuild.

## HTTP routes

The generated [OpenAPI](../api/openapi.v1.json) defines exact fields.

| Request | Result |
| --- | --- |
| `GET /api/v1/identity`, `GET /api/v1/me` | Identity and account check; no catalog enumeration |
| `GET /api/v1/client/library?collection_id=…` | Listening count: `total_items`, `total_is_exact`, `catalog_revision` |
| `GET /api/v1/library/summary?…` | The same count contract for the web scope, including filters |
| `GET /api/v1/client/albums?…` | Compact listening rows; `revision`, opaque cursor, `max_bytes` |
| `GET /api/v1/library?…` | Web rows from the same query service, with curation fields |
| `GET /api/v1/collections/{c}/assets/{a}/thumbnail?revision=…` | Revision-bound thumbnail with private ETag/304; never regenerated in the GET |
| `GET /api/v1/items/{item}/client-tracks/summary` | `tracks_revision`, availability-row count (including format variants), `pending` |
| `GET /api/v1/items/{item}/client-tracks?revision=…&cursor=…` | Revision-consistent track pages |
| `GET /api/v1/items/{item}/client-playback?…` | Source-selection snapshot for explicit play/queue; never used by refresh |
| Media HEAD/GET and private-media reads | Requested bytes, current authorization, no whole-file verification |

The playback-attention filter reports a non-exact count, because pending summaries and
live imports can change inclusion. Sort is part of the paging scope but not the count.

**Page size:** default target 64 KiB of uncompressed JSON, hard maximum 128 KiB, at
most 200 rows. `max_bytes` may lower the target; the hard cap always wins. A page stops
before the row that would exceed the budget and always advances. If a single row cannot
fit, the server returns a typed invalid-request error instead of truncating.

Native album rows contain item/collection IDs, title, artist, `row_revision`, artwork
asset/revision and `summary_pending`. Track rows contain source/track coordinates,
title, duration, availability and media identity/length/digest. No archive paths,
provider payloads or private jobs cross this boundary. A digest is recorded identity,
not a fresh verification.

## Counts and stateless paging

Collector answers independent reads. It keeps no refresh sessions, stored client
cursors, progress jobs, client counters or cache inventories. A client wanting progress
queries the count, counts the rows it accepts and shows e.g. `120 of 400 albums`. Pages
do not repeat the count. Do not derive a page total from the count, since byte-bounded
pages vary in row count.

Cursors are self-contained and bound to query, sort, scope, catalog generation and the
last sort key/ID, so the server can reject a stale cursor without remembering anything.
Clients must not mix a count from one revision with pages from another, or report an
incomplete enumeration as complete. On a 409 `conflict`, re-query and restart with
backoff while keeping the last complete cache.

## Revisions and caching

- Cache identity includes server identity, account, collection/item, effective role,
  language and presentation-policy version. Authorization runs before every 304 or
  pending response.
- The catalog revision changes on visible row or membership changes (title, artwork,
  track token, pending→ready, grants, language). Reindex starts a new epoch.
- Keep staged pages separate from the last complete manifest. Remove old entries only
  after a complete enumeration; an interrupted refresh never wipes the cache. Access
  denial hides the denied scope immediately.
- The track token covers inventory, recorded layout and tags, selected sources, disc
  inclusion, effective metadata, prepared audio, visibility, role and language.
  Artwork-only edits do not invalidate tracks; title edits do not invalidate audio files.
- An artwork change invalidates only that rendition. Old revision URLs never serve new
  bytes. Thumbnails use private caching only. The native reader limits thumbnails to 2 MiB.

The web client keeps current results, scroll and focus during background reads, leaves
the view alone when nothing changed, and may refresh up to 50 visible rows per request
by ID. It uses a count only with a matching catalog revision.

Trash Amp keeps its manifest, staged pages and per-album track caches locally. An
unloaded album is not an empty album. Artwork uses its own cancellable worker; playlist
entries load on demand; online song search uses Collector's paginated music search.

## Playback does not verify files

Playback must not perform whole-file verification before serving media. On a media
request the server authorizes the target, opens the file, checks its recorded length,
seeks if needed and streams bounded chunks. HEAD reads no body; a small range reads
only that range. 200/206/416, Content-Range, If-Range and immutable validators behave
normally. Silent same-length storage corruption may therefore be heard before an
integrity check finds it; that is accepted.

Import, publication and conversion still validate their inputs and outputs, explicit
integrity jobs may hash whole files, and clients may hash a completed download before
keeping it offline.

### Read failures

Before headers are sent, missing files, open/seek failures, length mismatches and
unavailable storage return a structured error without filesystem paths;
unauthenticated, forbidden and revoked stay distinct. After the body has started, the
server aborts the stream and logs a diagnostic. Read errors never become an empty
success, trigger preparation, or fail a catalog refresh.

The client decides what happens next. Trash Amp records the failed file and attempt,
shows the failure and advances to the next track, with a bound so a failing queue
cannot loop forever; it does not mark the album unavailable or delete downloads.
The web player shows the failure and offers retry or skip. The server never chooses
the next track.

## Storage

Catalog queries use bounded read-only SQLite connections; catalog and private databases
use WAL so reads see committed rows during imports. Reads do not hold writer or
publication locks while serializing responses. On a remote archive, local-state
requests use the cached mount-health result and fail promptly when the probe is
overdue or failed; archive access still checks the live mount.
