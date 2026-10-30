# Digital album matching application review — 2026-10-30

A fresh review of Collector and Trash Amp found and corrected:

- Collector's automatic artwork step could use the previously selected release
  after applying a reviewed mapping. It now uses the authority record committed
  by that application. The real storage regression starts from another selected
  release and checks every artwork lookup. Removing the fix makes that assertion
  fail with requests for both the old and new release; restoring it passes.
- Collector advertised discovery retry but its application dispatcher rejected
  the job kind. Retry now checks write eligibility and frozen inventory, metadata
  and source inputs. The durable fixture covers provider failure, successful
  same-job retry with the original timestamp, and refusal after a manual edit.
- Collector's raw history presentation bypassed shared completeness revalidation.
  Presentation now downgrades invalid completeness while retaining unknown payload
  fields and the immutable stored result.
- Collector's browser could process a superseded mutation against a new request
  scope. Continuations now belong to the captured controller. Status exposes
  input staleness; inventory and metadata refreshes disable obsolete mapping and
  retry actions. Polling errors and action failures have separate lifetimes.
  Saved receipt cleanup is fenced too. A new search acknowledges a lost admission
  response before admitting fresh work, instead of silently replaying a completed
  search and requiring a second click.
- Trash Amp could accept a late enrichment callback after a reviewed mapping in
  the same scan generation. Each callback now carries its expected album baseline;
  a changed album rejects it before in-memory or persistent publication.
- Trash Amp offline restoration required unchanged scan order. It now compares
  complete unique track identities independently of order, retaining the existing
  source validation before applying the accepted mapping.

The shared dependency remains `0becbd0c382df3d494244c7ccc20de05cba580cf` in both
applications. Audio masters and manual authority are preserved.

Validation:

- Collector's source checks in the full `infra` selector pass: architecture/links,
  format, web
  checking/build, tooling/container/release policy checks, 539 workspace tests,
  strict all-target workspace Clippy and web models. Five existing opt-in/scale
  tests remain ignored. OpenAPI and generated TypeScript contract checking passes.
- Its browser run passes the first 23 journeys, then reaches the runner's
  180-second outer deadline in intake review. The existing provider settlement
  step already allows 240 seconds, following review and cleanup phases. The
  runner now gives this combined journey the same bounded 600-second budget as
  broad integration. Retained fixture jobs were waiting on paced providers at
  shutdown. An isolated rerun completes in 100.09 seconds; intake review and the
  four unexecuted journeys all pass. All 28 browser journeys therefore complete
  successfully across these runs; the initial full-selector report retains its
  failed status rather than being relabeled as a clean uninterrupted run.
- The final frontend receipt correction is followed by web checking/build,
  rebuilding the embedded server assets, served-asset smoke and the expanded
  matching browser journey. The regression first fails against the earlier
  executable (one job instead of the expected two), then passes with the fix.
  Matching acceptance covers obsolete responses, stale inventory and metadata,
  reopen, one-click fresh search after a lost receipt, artwork and applied tracks.
  The 390-pixel stale-result screen was inspected and has no horizontal overflow.
- Trash Amp passes 238 workspace tests, formatting and strict all-target Clippy;
  three existing opt-in tests remain ignored. Its reviewed fixes are on main in
  `5b0028eeeb4fccf3f618630f1f1b0cda0fe19957`.

Browser and AVFoundation checks ran outside macOS seatbelt. Offline fixtures
exercise real FLAC/MP3 audio, durable jobs and the served browser workflow. No live
AcoustID recognition, deployment or new physical-device qualification is claimed.
