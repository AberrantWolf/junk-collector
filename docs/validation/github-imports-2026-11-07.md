# Imports review and pagination — 2026-11-07

[Issue 12](https://github.com/AberrantWolf/junk-collector/issues/12),
[issue 13](https://github.com/AberrantWolf/junk-collector/issues/13),
[issue 14](https://github.com/AberrantWolf/junk-collector/issues/14) and
[issue 15](https://github.com/AberrantWolf/junk-collector/issues/15) are delivered
together in source.

- Saved groups use the existing creation timestamp and an ID tie-breaker before
  the page limit. The index works with retained intake records and requires no
  record rewrite. A new group arriving between requests does not shift the page
  boundary.
- The editor selects published albums in a saved import group, across pages,
  and deduplicates targets shared by multiple packages. Each request applies the
  checked title, artist and source fields in one revision-fenced authority update.
  Per-album failures stay visible. Retrying a mixed result visits the remaining
  albums. Source uses the existing digital-only eligibility rule. Private
  purchase details retain their existing editor.
- Saved groups, published packages and the pre-import package review use
  alternating backgrounds. Pagination shows a disabled loading control with
  visible text and a decorative spinner that respects reduced motion.

## Acceptance

- Rust regressions passed for timestamp precision, tied creation times, insertion
  between pages, stale metadata revisions, reindex and atomic rejection of a
  source edit on an ineligible album.
- The real-server digital purchase journey passed in Chrome 156. It withheld
  responses to observe disabled loading controls, checked alternating backgrounds,
  and selected beyond the visible member pages while excluding one album. It
  exercised a partial save failure and retry, and verified retained title, artwork
  and the untouched album. The mobile editor fit a 390-pixel viewport and restored
  focus to its trigger.
- The Library browser journey verified a disabled spinner during a held page
  request and fetched just the continuation page.
- Strict workspace Clippy, generated OpenAPI and TypeScript contract checks,
  formatting, web type checking, web build and the architecture/link check passed.
- The locked workspace test suite and all 30 browser journeys passed. The first
  browser run stopped in FX-05 after a replacement conflict and an error in its
  polling helper. FX-05 passed on an isolated rerun, and the remaining journeys
  passed in a separate run.
