# Standard-game identification validation — 2026-10-10

Delivery: [supported matrix and workflow](../game-identification.md).
Local host: macOS, Rust workspace and headless Google Chrome; temporary archives,
private state, browser profiles and owned/generated synthetic fixtures. No live
catalog scraping, real collection edits, provenance cleanup occurred. The later delivery commits pin the pushed shared
revision, without changing the tested identification mechanisms.

## Independent mechanisms and policy

`collector-modules/tests/automatic_games.rs` exercises independently computed SHA1
vectors for trainer-free NES payloads, all three N64 source byte orders, headerless
NES, SNES and PC Engine payloads/copier-header candidates, and ordered CUE tracks.
Misleading suffixes do not change identification. Bank-aligned random data remains
unrecognized without complete catalog evidence. Headerless exact identity works;
duplicate equivalent snapshots converge, competing systems remain ambiguous and
partial track coverage remains unaccepted. Catalogs containing exact malformed NES,
truncated N64 and unsupported UNIF bytes cannot supply raw fallback identity.
Forged result envelopes cannot invent title, completeness, input hashes or coverage.

Shared DAT tests cover XML/ClrMamePro, BOM/whitespace content detection, author/URL
fields, malformed lengths, dropped/unsupported rows, truncation and bounded tokens.
Existing shared qualified vectors cover trainer/misc-ROM/trailing data, NES 2.0
exponent lengths, hostile N64 chunk boundaries, early EOF, invalid copier layouts,
CUE references/index boundaries/stored versus synthetic gaps and cancellation.
Provider batch tests cover mixed content-detected ZIP/standalone catalogs, constrained
conflicts, unknown systems/conventions, malformed documents, traversal, links,
nested archives, member counts and expansion bounds.

## Durable application and storage

The new real-service/store test creates more than 100 game entries, processes the
first admission page, closes/reopens the store with an advancing clock and recovers
the later page containing a preserved game. A subsequent direct request converges
with admitted work, retains its immutable timestamp/receipt and survives parent
scan cancellation. Manual title authority survives computation, catalog replacement
and full reindex. Changed payload bytes, revoked curation and superseded catalog
sets fail publication. Last-owner cancellation stops queued work. Existing catalog
upload/restart/idempotency/history and game evidence/reindex tests remain exercised;
obsolete custom-system fixtures were replaced with supported Game Boy fixtures.

## Real browser acceptance

`npm --prefix web run test:game-catalog` passed with the real server and React UI,
no external requests or page errors. The journey preserves a provisional `disc`,
uploads a PlayStation Redump ZIP (DAT stored under a misleading `.txt` name) and
Game Boy No-Intro DAT together, reviews detection without technical fields, imports
and activates both. The existing entry immediately resolves to
**Final Fantasy Tactics (USA)** with canonical **Sony - PlayStation** system; both
systems remain active. Search finds the accepted title. Manual title and system
controls save deliberate values, another Identify game preserves the manual title,
and Use detected controls restore automatic fields. A later guided import receives
the same automatic identity. Invalid ZIP and malformed DAT uploads remain visible
and unactivatable. Mobile catalog review has no horizontal overflow.

Screenshots were recorded at `/tmp/collector-game-dat-review.png`,
`/tmp/collector-game-dat-mobile.png` and `/tmp/collector-game-existing-match.png`.
These synthetic track bytes prove the workflow and ordering; they are not an
owned-disc dump oracle or a claim of physical-media verification.

## Repository and sibling gates

Initial delivery gates passed with the other pre-existing working changes present:

- `python3 scripts/check_architecture.py` (architecture, documentation links and
  fixture manifests), `cargo fmt --all -- --check` and `git diff --check`.
- `cargo test --workspace`: 407 passed, no failures, four existing ignored tests.
- `cargo clippy --workspace --all-targets -- -D warnings`.
- `python3 scripts/api_contract.py`: annotated routes, OpenAPI and generated
  TypeScript agree.
- `npm --prefix web run check`, `npm --prefix web run build` and subsequent
  `cargo build -p collector-server` to rebuild embedded assets.
- `npm --prefix web run test:game-catalog` and `npm --prefix web run test:smoke`:
  automatic game workflow plus existing authentication, music metadata, bulk
  imports, collection lists, provider concurrency and actual browser playback.
- Shared `junk-libs-dat` / `junk-libs-game-formats` tests: 19 passed; targeted
  Clippy with `--all-targets --no-deps -- -D warnings` passed. The shared dependency
  crate `junk-libs-disc` has existing Rust 1.99 lint failures outside these edits;
  this is not a claim that the whole sibling workspace passes Clippy.
- Retro Junk `cargo check --workspace --offline` and tests for `retro-junk-dat`,
  `retro-junk-import` and `retro-junk-lib`: 256 passed, no failures.

A successful format result is limited to the published matrix; no unsupported
format is represented by a mock. Collector now pins pushed shared-library revision
`83177af57ca36a5b284298020fea9d5e8fb0c415` without local Cargo patches.

## Pushed revision and isolated commit qualification

Collector now uses pushed shared revision `83177af57ca36a5b284298020fea9d5e8fb0c415`
for all shared Git crates, with no local patches. The related Collector commit was
assembled and tested in an isolated temporary tree, excluding the pending audio,
provider-acquisition and snapshot-conversion work.

Passed: locked workspace tests (397 passed, no failures, 3 existing ignored),
strict all-target Clippy, Rust formatting, architecture/documentation checks,
generated API contract checks, TypeScript/build, embedded-server rebuild, the real
game catalog journey and the existing browser smoke journey. Three legacy game
tests now wait for activation work or exercise the shared identification service;
these updated assertions passed in the complete isolated workspace run.
Retro Junk consumer changes were committed separately; its unrelated scripts are
excluded. Only the shared-library commit was pushed by this assignment.
