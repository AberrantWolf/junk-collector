# Change-driven Library summaries — 2026-10-28 JST

## Request and boundary

After beta.12 reduced cached production Library requests to 8–15 ms, the user
requested building summaries when data changes rather than charging the first
Library load the remaining 4.86-second calculation. The previous measurements
remain in [beta.12 acceptance](container-beta12-publication-2026-10-28.md).

The application listing no longer invokes the detailed resolver. A compact basic
row is projected with inventory and returned immediately when a full summary is
missing/stale. It has an explicit pending/retrying state and no stale artwork,
playback-readiness or AccurateRip conclusions. Current role authorization remains
required, and pending rows do not offer management actions. The normal detail
endpoint still resolves a fresh authorized Item.

A separate single server worker discovers missing/stale summaries for current
enabled accounts' distinct collection-role/language variants. Existing
transactional item tokens provide durable dirty state, without creating user Jobs
or relying on a browser request. Startup, reindex, new grants/preferences and
publication all converge through the same query. Builds freeze language and
source token; conditional publication refuses calculations superseded by edits.
A newer generation bypasses a previous generation's retry delay. Failures persist
a five-second retry time, remain visible and do not monopolize other entries.

Variants are bounded by current account/grant/preference demand; publication
prunes obsolete variants/namespaces. This replaces beta.12's arbitrary twelve-row
limit, which could repeatedly evict summaries still needed by more active variants.
Import progress is still read live. Full-graph file/playability/listening filter
predicates are now materialized when data changes and reused by Library reads;
their existing predicate definitions remain shared with detailed browsing.

The additive basic-row/retry tables remain outside archive checkpoints and force
no full index replay. Existing installations backfill basic metadata at startup.
Reindex clears derived views; source records, originals, manual authority and
accepted evidence are preserved. Background builds use existing recorded metadata;
neither building nor browsing scans or hashes original audio.

The API returns typed page and row summary state. Web Library displays basic rows
and an updating notice, then polls until complete. An empty playback-attention
page also retains pending/retry state, avoiding a false empty result and ensuring
that later matches appear automatically. Retry failures stay visible. Existing
scope/actor/navigation cancellation fences apply to these refreshes.

## Validation in progress

- Architecture/document links, formatting, generated OpenAPI/TypeScript, web type
  checking/build and web model suites pass.
- Five focused storage tests pass: payload-denied cold/warm/invalidated reads
  across every filter and listening-only selection; language changes during a
  selected build; durable failure delay/restart/peer fairness/new-generation retry;
  fourteen actual demanded variants settling without eviction churn; and rollback
  of an additive-install failure after inserting a basic row but before its filters.
- Storage integration passes metadata invalidation, stale-publication refusal,
  private artwork/role isolation, language changes, revocation, restart and reindex.
- The Opus-only readiness regression passes with explicit background rebuilds and
  checks materialized file/playability predicates before/after preparation.
- The focused real-browser run passes browse (31.78 s), short smoke (4.51 s) and
  CD Library (17.93 s), using Chromium 153.0.8010.12 through the shared harness.
  The disposable derived index contains all 62 valid summaries before first
  Library navigation. Typed pending rows render immediately and poll to ready;
  an empty retrying attention page avoids a false empty result and refreshes.
  Desktop and 390-pixel mobile pending-state screenshots were visually inspected;
  wrapping and accessible names pass without horizontal overflow. Existing paging,
  navigation/edit/artwork, live imports and one-click AccurateRip journeys pass.
  Two repeat requests return 62 rows / 32,697 UTF-8 bytes in 8 ms each. These are
  local fixture measurements, not production or a new load benchmark.
- Review made additive installation atomic so interruption cannot retain basic
  rows with half-built filter flags. The final candidate includes that adjustment
  and the beta.13 version; complete exact-source qualification is pending.

This is the [beta.13 candidate](../releases/v0.1.0-beta.13.md). Beta.12 remains
installed while final source/container qualification proceeds. New production
Library acceptance has not yet been claimed.
