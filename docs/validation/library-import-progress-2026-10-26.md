# Library import progress — 2026-10-26

Album rows previously showed playback attention while imports were still
uploading, publishing, enriching metadata or preparing audio. They now prefer
“Import in progress” with a muted clock. The list polls while visible imports are
unfinished, and importing albums are excluded from playback-attention results.
Playback readiness itself retains its existing meaning and detail explanation.

The derived, authorized item field exposes only a boolean. The indexed import
query reads durable workflow/job state without media payloads or hashing. Settled,
cancelled and terminally blocked imports stop displaying progress. Separate
unfinished requests keep the album in progress. No archive format or stored
workflow state changes.

Validation on the working tree based on `ae2e35b2d483b4ebcae25e257e103f6e22e1e304`:

- Selector preview classified the complete change as infrastructure because it
  includes storage/contracts and pre-existing script edits. Its selected source
  checks and complete named browser set were run individually.
- Architecture/documentation links, formatting, generated OpenAPI/TypeScript
  contract agreement and diff whitespace checks pass.
- `cargo test --workspace --locked` and workspace Clippy with all targets and
  warnings denied pass. The extended regressions exercise upload waiting,
  queued/running work, failure, retry, cancellation, cross-collection denial,
  attention filtering, downstream preparation and settlement.
- Web type checking, build and queue/context/label models pass. The server and
  media test executables were rebuilt with the current embedded assets.
- All named browser journeys pass in Chrome 156.0.8078.47 across the runs. The
  updated Library journey uses real imports, checks concurrent requests, confirms
  automatic updates after cancellation/completion, and checks accessible names
  and mobile overflow. Desktop/mobile screenshots were visually inspected.
- Testing/container tooling checks pass, with the existing rootless-skopeo test
  skipped for missing infrastructure. The owned-disc oracle and optional manual
  scale/performance tests remain ignored; the ignored native-reader subprocess
  helper is exercised through its passing parent test.

The first broad browser run stopped on a five-second Chrome shutdown timeout
after the metadata-review assertions passed; that journey's rerun exited cleanly.
A later intake-cleanup run encountered an upstream metadata revision conflict and
failed its expected successful-import outcome assertion. The isolated rerun passed
intake, cleanup, restart and 31-album progress acceptance. No assertions were
relaxed, and no import/enrichment mutation policy was changed for these failures.

No release, container-image acceptance or deployment is requested or performed.
The pre-existing CD capture script edits remain separate from this delivery.
