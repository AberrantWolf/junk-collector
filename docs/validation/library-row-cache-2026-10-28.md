# Compact Library row cache — 2026-10-28 JST

## Request and observations

After beta.11's cancellation-poll repair, the user rejected a 4.7-second idle
Library request as still too slow and requested server-side cached summaries.
The signed-in production Library returned all 40 entries in 4,702 ms and rendered
artwork/status after a full reload. The original AccurateRip batch had settled.
Repeat read-only requests measured:

| Entries returned | Response headers | Body decoded |
| --- | ---: | ---: |
| 1 | 90 ms | 98 ms |
| 10 | 1,212 ms | 1,213 ms |
| 40 | 4,041 ms | 4,052 ms |

This scaling locates the remaining delay before the response, rather than in
rendering or response transfer. The detailed browse path builds complete music
track/profile availability twice for album readiness, reads immutable recipes and
repeatedly resolves capture metadata for prepared tracks. These are metadata
reads, not original-audio hashing. The repair avoids that detailed computation
on ordinary cached Library loads instead of extending the request timeout.

## Delivery and boundaries

`GET /api/v1/library` supplies compact, authorized rows: title/subtitle/context,
module/identity, wishlist/removal, edit eligibility, selected artwork, tentative
metadata, import progress, album playback readiness and per-original AccurateRip
status. It sends no capture/file graph, full module payload or track inventory.
The web Library uses these rows; opening an album or its curator still obtains a
fresh full Item. Existing detailed browse, native/selector and playback consumers
retain their current APIs. Artwork remains beside the text rows.

Application policy derives missing rows through the existing presentation and
album resolver. Storage caches only that compact result, partitioned by item,
role, metadata language and versioned presentation/module identity. Cache storage
retains at most twelve role/language variants per item, evicting older derived rows
without touching collection records. Current collection authorization is checked before a cached result is used. A concurrent
role change cannot reuse a curator row for a viewer. Import progress always uses
the existing indexed operational query; job heartbeats do not expire cached rows.

Projection triggers change an item's opaque publication token in the same
transaction as inventory, captures, metadata authority/candidates/refreshes,
processing observations or conversion recipes. Work/Edition/artwork/collection
changes conservatively invalidate their collection to cover inherited artwork and
bindings. Cache writes compare the token, refusing mixed-revision publication;
the application retries observation at most twice if data changes during a miss.
The same indexed sort/filter/keyset selection serves compact and full browsing.
Playback-attention filtering retains its 200-candidate bound and sparse cursors.

Rows persist across unchanged restarts. Reindex clears them and derives them
again. The additive cache tables do not alter existing catalog record formats or
force a full index rebuild; private state and portable archive formats do not
change. Originals, manual metadata, accepted evidence and unknown module data are
not modified by cache construction or invalidation.

An uncached or invalidated row still runs the detailed resolver once. A new policy
version, first use of a language/role, reindex or conservative collection-wide
invalidation can therefore incur cold computation. This delivery does not claim
that a cold calculation itself is faster or that existing detail/native requests
have become compact. No production cache deployment or new loaded-batch benchmark
has yet been performed.

## Validation in progress

- Architecture/document links, Rust formatting, web type checking/build and API
  contract generation pass.
- The new storage integration regression passes: cache reuse, compact response,
  per-item metadata invalidation, stale-publication refusal, private-artwork viewer
  isolation, language separation, revocation, restart persistence and reindex.
- A preliminary full local run is exercising workspace tests, Clippy, models and
  all 27 real-browser journeys outside the macOS seatbelt. It began before the
  final disposable-variant bound, additive-upgrade decision and beta.12 version
  bump, so it is not exact-source
  release qualification. The release candidate will receive complete exact-source
  CI and native container qualification before publication.
  The full run includes a SQLite authorizer regression that forbids full item
  payload, capture, observation and recipe reads during a warm Library request,
  plus Opus-only preparation invalidation and the actual compact-list UI.
- An initial focused unit-test invocation used an incomplete exact name and
  selected zero tests. It is not counted as acceptance; the full suite includes
  the qualified test.

This is the [beta.12 candidate](../releases/v0.1.0-beta.12.md). Beta.11 remains
the NAS release while exact-source validation proceeds.
