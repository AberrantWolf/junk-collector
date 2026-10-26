# Faster testing and packaging with reliable browser launches

Approved and implementation authorized 2026-10-26. The implemented commands and
policy are maintained in [testing](testing.md); acceptance and limits are recorded
in [the delivery evidence](validation/testing-and-packaging-2026-10-26.md).
[Remaining work](remaining-work.md) remains the only assignment queue; the steps
below define this delivery rather than creating another backlog.

Make small changes inexpensive to validate, stop the recurring Chrome launch
crashes, and build each deployment image once. Text-only edits must not trigger
unit tests. Beta releases should use change-based coverage; RC and stable releases
must have full qualification of their functional changes. All deployed binaries
remain release builds.

## Evidence and current costs

- Three local Chrome reports from October 25–26 show `SIGABRT` in macOS
  `_RegisterApplication` / `TransformProcessType`, within approximately 0.2–0.3
  seconds of launch. The latest names Node as its parent. These point to browser
  startup rather than a failure during a Collector page assertion.
- The [metadata notice validation](validation/metadata-attention-notice-2026-10-26.md)
  records a sandboxed launch failure followed by a successful permitted retry.
  A restricted launch environment is the leading explanation, not a confirmed
  complete diagnosis. Reproduction and launch stderr must establish the fix.
- Twenty-six browser scripts default to the installed Google Chrome application.
  There is no shared browser-launch policy. Most real-server journeys hardcode
  `target/debug/collector-server`; remote-media also uses the debug media binary.
- `web/tests/smoke.mjs` has grown to 1,284 lines. It includes provider fixtures,
  uploads, encoding, pending-work behavior, playback, restart/reindex, permissions
  and mobile checks. Its encoder fixture deliberately waits 3–9 seconds. This is
  a broad integration suite rather than a short smoke test.
- [Beta.8 candidate validation](https://github.com/AberrantWolf/junk-collector/actions/runs/40254283704)
  took approximately 24 minutes: Cargo tests 7m 02s, Clippy 1m 09s, a standalone
  release build 7m 01s, then concurrent native container builds about 6m 25s.
  Container runtime smokes took approximately 29–33 seconds per platform.
  The Cargo test step included 2m 44s of compilation; the archive integration
  target alone took about 192 seconds. These are CI timings, not local estimates.
- `.github/workflows/check.yml` has npm caching but no Rust compilation cache.
  It compiles release executables that are not consumed by the Docker build.
  `Dockerfile` separately compiles the release executables for each platform.
  Copying changed web assets before Cargo compilation invalidates that build
  layer; a Docker layer cache alone does not preserve Cargo intermediates after
  that invalidation.
- The source checks workflow does not currently run browser journeys. Publication
  already reuses the exact validated OCI exports without rebuilding. Preserve
  that behavior and its source/digest checks.

## Test selection policy

Selection uses the complete candidate change, not just its last commit. For local
work include staged, unstaged and relevant untracked changes. For a beta compare
with the prior validated release baseline; for RC/stable account for every
functional change since the last full qualification. Missing or uncertain
baselines require broader checks. Existing unrelated work must not accidentally
qualify for an exemption because the latest edit only changes copy.

| Change | Local checks | Beta candidate checks |
| --- | --- | --- |
| Documentation prose only | Link/Markdown checks relevant to the edit | Same; no unit tests, Rust compilation or container build unless packaging is explicitly requested |
| UI display text only | Web type check/build; focused rendering and desktop/mobile inspection | Same plus short acceptance on the release image if packaging; no unit tests or Rust-wide Clippy |
| CLI display text only | Compile the affected executable; inspect changed output | Release build plus focused output acceptance; no unit tests |
| CSS or layout only | Web type check/build; affected screen, mobile overflow and focus/visibility checks | Same plus short release-image acceptance; no unrelated Rust tests |
| UI behavior | Cheap web model tests, focused browser journeys and web type check/build | Same plus short release-image acceptance and related real-server journeys |
| Rust implementation | Architecture/format checks, affected package/target tests and Clippy, plus affected consumer coverage | Same plus release-image acceptance; broaden across layers where the change crosses boundaries |
| Authentication, permissions, storage/recovery, shared contracts, dependencies or toolchain | Relevant cross-layer regressions and full workspace checks | Full workspace checks and affected browser/packaging acceptance |
| Build, test selection or release tooling | Meaningful selection/provenance/failure regressions and affected real workflow rehearsal | Rehearse candidate validation and nonpublishing artifact reuse; broaden product checks if build outputs change |

"Text-only" means human-facing prose or display strings with unchanged behavior,
data flow, markup structure and contracts. A string used as a URL, identifier,
selector, configuration value, enum, request field, stored value, security rule or
parsed protocol output is not automatically display text. Changing help examples,
deployment instructions or policy files also needs the corresponding review.
Do not infer text-only status merely from `.md` or `.tsx` file extensions.

The developer/agent explicitly reviews mixed source diffs and records the
classification. A selector may validate that declaration against changed paths
and reject obvious contradictions; it must not claim to prove semantic equivalence
from extensions or a regex. Unrecognized paths default to broader coverage.

Text-only edits do not trigger unit tests at any release level. RC/stable still
require full qualification of all functional changes since the previous full
baseline. Prior evidence may cover unchanged functional inputs; record its source
and scope. If that equivalence cannot be established, full qualification is
needed for the unqualified functional change. Never describe a small beta test set
as full RC qualification.

Do not add unit tests that simply assert replacement copy. Update an existing
browser locator only when necessary, retaining its actual behavioral assertion.
Prefer a stable accessible control/region when exact prose is incidental.

## Browser launch and lifecycle repair

1. Capture Playwright launch stderr and correlate browser PID/time with the local
   crash reports. Reproduce in an isolated fixture using the failing execution
   context and the known-working permitted context. Stop repeating a known-broken
   launch once it is identified. Do not suppress macOS crash notifications.
2. Add a shared harness under `web/tests/support/` for browser selection, logging,
   isolated contexts, server readiness and bounded teardown. Keep this mechanism
   outside Collector domain/application code.
3. Evaluate the Playwright-version-matched headless browser as the ordinary UI
   default. Keep an explicit actual-Chrome mode for PDF-viewer, codec, media,
   WebAuthn or other cases whose coverage depends on that browser. Establish the
   required modes from existing tests, rather than assuming browser equivalence.
   [Playwright browser guidance](https://playwright.dev/docs/browsers) documents
   the headless/browser differences and versioned installations.
4. Run native-browser tests through the execution context already demonstrated to
   work, within the caller's existing permissions. An unavailable context is an
   explicit infrastructure failure with remediation; do not turn it into a passed
   or silently skipped test. A different executable alone is not proof that the
   macOS registration failure is fixed. Do not broadly disable OS protections or
   add speculative browser flags to hide it.
5. Close pages/contexts and the browser on success, assertion failure and ordinary
   interruption. Bound browser and server shutdown, clear timers, and stop/reap
   only processes owned by the fixture. Do not kill the user's ordinary Chrome.
   Collect diagnostics without allowing screenshot/body-read failures to mask
   the original failure or prevent teardown.
6. Migrate all direct launch sites to the harness, retaining test isolation and
   actual error reporting. Make missing test-browser installation actionable
   before starting expensive fixture setup.

Acceptance: ten consecutive representative launches/exits without a new matching
crash report; focused desktop/mobile copy checks; actual-Chrome PDF, passkey and
playback/seek checks where applicable; intentional assertion failure and ordinary
interruption with bounded cleanup and no owned child process left running. Keep
unrelated browser crashes distinct from the tested process. No success claim until
the original launch failure has been reproduced and the repaired path exercised.

## Split smoke coverage and expose predictable commands

Retain existing coverage while extracting independent journeys from the large
smoke script. The new short smoke exercises sign-in, library rendering with
artwork, search, opening an entry, loading an affected management bundle, a
permission denial and mobile overflow. Use small deterministic fixture data and
one isolated server/browser worker. Preparation, provider races, restart/reindex,
recovery, long playback and deliberately slow encoders belong in named suites.

Expose a single selector, proposed as `python3 scripts/check_changes.py`, with
`--base`, `--level local|beta|full`, an explicit change classification and
`--dry-run`. It prints the selected checks and reasons, executes them without
duplicate builds, and reports passed/failed/omitted checks accurately. An explicit
`--level full` remains available. Keep mappings small and reviewable, using the
existing package scripts and Cargo package/target filters rather than introducing
a general dependency-analysis framework.

Add or rename npm entry points so callers can select a short smoke, the complete
browser suite and a feature journey. Keep the current broad journey identifiable
as integration coverage during extraction. Include currently standalone model
scripts such as queue and work-context in the appropriate web-behavior set.

Replace arbitrary waits with observed UI/network/job conditions where practical.
Retain purposeful delays that exercise cancellation, stale responses or pending
work. Measure repeated setup, hashing, fixture creation and process teardown before
removing a slow test. Delete a test only after showing its assertion is pointless,
duplicates the same guarantee, or tests a retired requirement. Keep preservation,
permission, failure/recovery and independent algorithm-vector coverage.

Use bounded browser concurrency only after giving each worker isolated state,
ports and provider fixtures. Serial execution remains appropriate for mutually
exclusive resources. Evaluate a different Rust test runner only if measurements
show a remaining benefit; it is not required for this delivery.

Acceptance: selection examples for copy, CSS, UI behavior, backend, dependency,
mixed/untracked work, unknown paths and missing baselines; a failed selected check
fails the command; the full suite still contains the retained guarantees. Target
the warmed short browser smoke at 30 seconds or less and routine focused copy
validation at one minute or less, excluding a required fresh Rust compilation.
Record actual timing instead of treating these targets as promises.

## Build once and validate the deployment artifacts

1. Remove the unconditional standalone release build from the source-check job
   when the requested deliverable is the container image. The Docker build already
   establishes optimized compilation and supplies the actual runtime bytes.
   If standalone binaries are separately requested, build and retain them as
   explicit deliverables rather than compiling and discarding them.
2. Add a Rust compilation cache to source checks, keyed by platform, toolchain,
   profile and dependency inputs. Preserve dependency/intermediate build outputs
   in the Docker builder across web-only changes using a measured BuildKit/Cargo
   cache strategy. Verify its persistence on hosted runners: an ephemeral cache
   mount alone is not sufficient evidence of reuse. Cached builds must still
   invalidate correctly on source, assets, lockfile and toolchain changes.
3. Start independent source checks and native image builds concurrently after
   planning the exact candidate. Remove the current build dependency on completed
   source checks. Sealing reusable validation must explicitly require successful
   selected checks, both native image smokes and source/digest verification.
   A skipped or cancelled prerequisite must never look like successful coverage.
4. Avoid running the same source checks twice for a push and its manually
   dispatched candidate. Reuse successful same-SHA CI evidence only when workflow
   identity, required checks, policy/profile, run attempt and completion are
   verified; otherwise run them. This reuses test evidence, never an image with
   different source labels.
5. Make browser fixtures accept explicit server/media binary paths. Routine local
   behavioral work can use debug binaries; some offline provider fixtures depend
   on `cfg(debug_assertions)` hooks. Do not enable those hooks in deployed builds.
   For text-only preview, prefer a lightweight static/fixture preview when it
   faithfully renders the affected UI. For packaged acceptance, use the actual
   release container and a bounded fixture through supported interfaces, following
   the existing container-browser fixture pattern where useful.
6. Build packaged executables only in release mode, once per architecture and
   candidate. A Mac test binary and a Linux deployment binary are separate targets;
   do not claim they are reusable substitutes. Rebuild embedded assets before
   building the binary that serves them, then verify served assets on the image.
7. Keep native AMD64/ARM64 runtime checks. Their measured cost is small. Preserve
   proxy/exposure, startup, isolated-media authorization, import and recovery
   checks already exercised by the image smokes. Do not rerun real NAS/device
   qualification for an unaffected copy edit.

Publication continues to copy the exact validated OCI exports. It must not rebuild
images, accept the wrong source SHA, weaken artifact integrity checks, or promote
beta evidence as RC/stable evidence. Extend the sealed validation record with the
selected test level, policy revision, changed scope, required results and any
reused test evidence. The consumer must enforce the required level and functional
coverage for the requested release, not merely inspect a successful job title.
Retain the existing source/run/attempt/archive/platform binding and the
[pre-release evolution policy](decisions/pre-release-evolution.md); no compatibility
wrapper or migration is needed solely to preserve an old validation checkpoint.
Older unqualified artifacts can fail with an actionable fresh-validation request.

Acceptance: both release images pass current native runtime smokes; a warm web-only
change reuses unchanged Rust dependencies; no redundant standalone release build;
source checks overlap image builds; check failure/cancellation prevents sealing;
wrong-SHA, insufficient-level, stale-attempt and corrupt evidence is refused; a
nonpublishing promotion rehearsal consumes the validated bytes without compilation.
Demonstrate both cold and warm timings. Removing the observed redundant build
alone should recover approximately seven minutes on a comparable run, but cache
and parallelism savings must be measured rather than added as guaranteed totals.

## Proposed AGENTS.md replacement

Replace the unconditional final `Checks:` paragraph with the following policy and
a link to the maintained command mapping once implemented. Preserve the existing
architecture, preservation and meaningful-test instructions above it.

> Validate the complete change with the smallest meaningful check set. Inspect
> staged, unstaged and relevant untracked changes; release selection covers the
> whole candidate since its validated baseline. Documentation and human-facing
> text-only edits do not require unit tests. Do not add tests that merely freeze
> copy. Check documentation links; for UI copy run web type checking/build and
> inspect the affected screen, including mobile wrapping/overflow and accessible
> names. CLI copy requires compiling the affected executable and inspecting output.
> Machine-readable strings, identifiers, URLs and protocol/configuration values
> require checks for their actual behavior.
>
> CSS/layout changes require focused rendering checks. UI behavior changes require
> related model/browser tests. Rust changes require affected package/consumer tests
> and Clippy plus architecture/format checks. Authentication, authorization,
> storage/recovery, shared contracts, dependency/toolchain changes and uncertain
> scope require broader workspace coverage. Beta releases use the documented
> change-based set; RC/stable require full qualification of functional changes.
> Report the selected scope and omitted checks honestly. Reuse unchanged evidence
> only with a recorded applicable baseline.
>
> Use the shared browser harness and supported launch context. A failed browser
> launch is an infrastructure failure, never passed UI acceptance. Avoid repeating
> a known-broken launch. Keep temporary state isolated and teardown bounded.
>
> Deploy release-mode executables only. Build each deployment target once, test the
> resulting artifact, and publish those same validated bytes. Debug builds are
> optional development/test tools, not deployment prerequisites. Rebuild a serving
> binary after changing its embedded web assets. Do not run duplicate standalone
> release builds when the container is the sole deliverable.

The command mapping retains the existing full architecture, formatting, workspace
test, Clippy, web and broad browser commands. It identifies them as the full set,
rather than instructions to run every command after every edit.

## Implementation sequence and completion record

Deliver this as one usable developer workflow, completing the browser harness,
selection policy, artifact checks and documentation together. Work in this order:

1. Record cold/warm timing baselines and reproduce the browser startup failure.
2. Repair and adopt the shared browser lifecycle; validate its actual Chrome modes.
3. Extract short smoke/feature coverage and implement transparent test selection.
4. Remove redundant compilation, establish effective caches, overlap independent
   jobs and enforce profile-aware artifact sealing/publication.
5. Update `AGENTS.md`, `docs/release-toolchain.md`,
   `docs/container-release-pipeline.md`, `docs/release-readiness.md` and relevant
   getting-started/test instructions so none still mandates full tests for copy.
   Update architecture documentation only if actual ownership/boundaries change.
6. Validate the selection and provenance failure cases, representative browser
   modes, both images and nonpublishing promotion. Use the full check set once to
   qualify this testing/build infrastructure change; do not mistake that one-time
   qualification for the future cost of a text-only edit.
7. Update current implementation status, the existing remaining-work queue and a
   dated validation record together. Preserve historical validation documents.
   Report before/after timings, retained coverage and any unmet targets.

The planned implementation touches development tooling, tests, CI, container
builds and documentation. It adds no Collector subsystem, stored collection schema,
provider workflow or automatic production deployment. Publication and production deployment are separate actions; this delivery changes
neither published images nor installed services.
