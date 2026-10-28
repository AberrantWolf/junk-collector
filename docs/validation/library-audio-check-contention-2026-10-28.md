# Library latency during AccurateRip work — 2026-10-28 JST

## Report and read-only diagnosis

The user started approximately thirty album audio checks and reported that Library
returned `Request failed (504)` while other views populated. The user confirmed
that jobs were still running. The signed-in Firefox session showed that error.
A read-only one-row browse succeeded with the original album, artwork and playback
readiness; its navigation timing reported 1,441 ms waiting for the server. Sequential
read-only requests for that album measured metadata 469 ms, recorded client tracks
463 ms, disc coverage 332 ms and full item presentation 1,432 ms. These are live
observations under load, not a controlled benchmark or an idle baseline.

The code resolves Library playback and audio summaries through many metadata and
job reads per album. It does not hash media merely to browse. Meanwhile the
processing reader consulted the same serialized job database before every read,
and the host consulted it at each CPU checkpoint. The shared redumper reader caps
reads at 588 stereo frames (one sector), including separate quality-state/audio
reads. This creates sustained competing durable queries while checking raw albums.
It is a concrete contention mechanism consistent with the observed page scaling;
production tracing and an after-deployment comparison are still outstanding.

No production jobs, settings or records were changed. A diagnostic tab was closed
after the reads. No new batch or remote provider request was submitted. Normal
batch-mode SSH access was unavailable, so no NAS database/log inspection was done.

## Repair and acceptance

The application processing host now shares one ephemeral cancellation poll among
its package readers, CPU checkpoints and catalog parser for each claimed execution.
Only a successful clear query may be reused, for at most one second on a monotonic clock.
The timestamp precedes the query so storage wait cannot extend the reuse window.
Cancellation and errors never grant cached clearance. Final source validation,
authorization, cancellation and publication gates keep their direct durable checks.
Import analysis, standalone scans and automatic game identification use the same
host. No algorithms, source bytes, archive formats, projections or public DTOs change.

Deterministic regressions cover ten thousand sector/checkpoint visits sharing one
clear query inside the window, cancellation at the window boundary, repeated
cancellation, expired claims/storage errors, and a reader stopping without consuming
the next sector once cancellation is observed. Validation results are recorded below. The live timeout is not claimed fixed until deployment and
an actual loaded-Library check; changing the HTTP timeout is not part of this repair.

## Validation

- Architecture/documentation links, formatting and `git diff --check`: pass.
- Web type checking, build and queue/context/label model tests: pass.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: pass on the
  final one-second interval and clock-sampling implementation.
- Final polling regressions: **3 passed**, covering query coalescing/window expiry,
  cancellation, failed claims/storage and an unread next sector.
- Final real-browser journeys: `cd-library` **23.67 s**, `audio-checks` **0.63 s**,
  `game-catalog` **8.74 s**, all pass. These use disposable real-server state and
  offline provider fixtures. The CD journey runs the actual workflow and observes
  the refreshed Library result, including the previously present local batch-view
  changes. It is not a thirty-real-album production benchmark.

The workspace-wide Rust run began before the user requested the one-second interval
and before the follow-up clock-sampling adjustment. Its broader results are an
intermediate implementation check; final targeted workflow results are recorded
separately below. Initial restricted compiler attempts were interrupted/restarted;
they are not counted as successful tests. macOS dynamic-loader waits were visible
in a sampled compiler. Browser checks ran in the permitted external context.

The other browser journeys, release/container qualification, a production update
and a post-update loaded Library comparison are not part of this local repair.

Final targeted Rust workflows pass:

- `music_cd_workflow_publishes_shared_accuraterip_results_and_survives_reindex`:
  **7.43 s**, real BIN/CUE and supported/unsupported redumper processing, actual
  AccurateRip fixture comparisons, response reuse/no-data and reindex retention.
- `expired_processing_claim_cannot_exceed_capacity_after_older_waiters_resume`:
  **0.14 s**, expired claims cannot resume work/admit shared acquisitions or
  publish completion after replacement claims.

The broader intermediate `cargo test --workspace --locked` run completed
successfully: **530 passed, zero failed, five entries ignored**. This run used the initial 50-ms poll;
the final one-second interval/clock sampling and subsequent test-only cleanup were
validated by the focused reruns above, current workspace Clippy and browser checks.
No exact-source full release qualification is claimed from that intermediate run.
The ignored entries include owned-disc, scale and manual performance qualifications, which remain unrun, plus a subprocess-only fixture exercised by its parent media-isolation test. Ignored entries are not counted as passes.
