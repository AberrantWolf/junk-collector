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
A build resolves metadata from the captured key language, rather than re-reading
a preference that can change during the calculation.

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
have become compact. Beta.12 is now deployed; no new loaded-batch benchmark has
been performed.

## Preliminary validation and corrected candidate

- Architecture/document links, Rust formatting, web type checking/build and API
  contract generation pass.
- The new storage integration regression passes: cache reuse, compact response,
  per-item metadata invalidation, stale-publication refusal, private-artwork viewer
  isolation, language separation, revocation, restart persistence and reindex.
- The preliminary full local selection passed workspace tests, all-target Clippy,
  web/models and all **27** real-browser journeys. It began before the final
  variant bound, additive-upgrade decision, version bump and language-binding
  repair, so it is not exact-source release qualification. The archive suite
  reports **285 passed, three ignored**; existing opt-in cases remain unqualified.
- The real-browser Library fixture returns **62 cached rows**, **31,185 bytes**,
  in **8 ms** on each of two repeat requests. It also passes paging/history,
  metadata/artwork edits, live import progress and response recovery. These are
  local disposable-fixture measurements, not NAS or cold-cache timings.
- The corrected candidate's focused cache tests both pass: the SQLite read trap,
  bounded eviction/rebuild, additive install with catalog reuse and frozen language
  binding. The language case uses distinct English/Japanese candidates and denies
  preference reads after the key has captured its value, then exercises both
  cached language variants.
- Review superseded candidate `82650ba`: its two qualification runs
  **40622449540** and **40622466544** were cancelled before publication to repair
  a preference race between cache-key construction and metadata resolution.
  Complete exact-source CI and native-image qualification use the corrected
  candidate; no old candidate image is accepted or deployed.
- An initial focused unit-test invocation used an incomplete exact name and
  selected zero tests. It is not counted as acceptance; the full suite includes
  the qualified test.

## Exact-source release qualification and deployment

[Beta.12](../releases/v0.1.0-beta.12.md) source `2f169715d6a8083854b985b671b9bc2cb72f9545`
passes [Checks 40624264312](https://github.com/AberrantWolf/junk-collector/actions/runs/40624264312):
workspace tests, all-target Clippy, web/models and all 27 browser journeys, with
full coverage since beta.11 and unchanged source. Native AMD64/ARM64 runtime
smokes and AMD64 release-image browser acceptance pass in
[validation 40624319196](https://github.com/AberrantWolf/junk-collector/actions/runs/40624319196),
attempt 1. Publication reuses those validated bytes.

Beta.12 is installed on the NAS after a complete stopped-server snapshot and a
7.88-second interruption. Config/TLS/identity/mounts, all 18 unrelated containers,
public HTTPS and nine served assets pass. No jobs were unfinished at the update.
After browser access returned, signed-in Library navigation and full refresh
render all 40 entries with artwork/status. The first observed post-upgrade request
takes 4.86 seconds; cached HTTP-cache-bypassed repeats take 15/9/9 ms, and the
Library resource after full refresh takes 8 ms. The response is 29,800 UTF-8 bytes
without item/capture/track payloads. These are settled-batch production timings,
not a controlled cold-cache or concurrent-workload benchmark. See
[publication and acceptance](container-beta12-publication-2026-10-28.md).
