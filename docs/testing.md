# Testing and packaging

Choose checks from the complete change. Text-only changes do not require unit
tests. The selector records what ran and why; release images are always optimized
release builds. Current policy is `change-checks-v1`.

## Everyday commands

Preview selection before executing it:

```sh
python3 scripts/check_changes.py --dry-run
```

For a reviewed UI copy change, including staged, unstaged and untracked files:

```sh
python3 scripts/check_changes.py --classification ui-copy --journey fx04-review
```

Use `--journey browse`, `--journey documents`, or another existing feature name
for the affected screen. Repeat the option for multiple screens. Inspect desktop
and mobile wrapping, visible controls and accessible names; screenshots can be
saved by setting `COLLECTOR_SCREENSHOTS=/absolute/output/directory` for the short
smoke. A passing generic smoke alone does not qualify arbitrary changed copy on
an unvisited panel. Do not add unit tests that merely assert replacement text.

Documentation prose normally selects the architecture/link checker only. Use
`--classification cli-copy` for reviewed human-facing CLI output, then inspect
the changed executable output. CSS-only changes select web checks and focused
rendering. UI behavior also runs the queue/context/label model checks. Rust edits
select affected crate tests and transitive in-repository consumers, with Clippy.
Storage, authentication, contracts, dependencies, toolchain, build/release tooling,
policy behavior changes and unknown paths select broader workspace coverage.
Automatic selection treats policy documents conservatively; an explicitly reviewed
`--classification docs` may exempt a prose-only correction in `AGENTS.md` or a
policy document when its instructions and behavior are unchanged.

`ui-copy` and `cli-copy` are human declarations after reviewing the entire diff.
The selector rejects contradictory paths; it cannot prove that a source string is
only display text. URLs, identifiers, configuration/protocol values and persisted
or parsed strings require behavioral checks. A copied heading and a changed event
handler in the same candidate are UI behavior, not a copy-only change.

For a candidate, select the actual validated baseline rather than the last commit:

```sh
python3 scripts/check_changes.py --base v0.1.0-beta.8 --level beta --classification ui-copy --journey fx04-review
```

Local defaults compare HEAD with the whole working tree. An explicit `--base`
also includes every committed change after that baseline. A missing release
baseline requires broad checks. Full functional qualification is explicit:

```sh
python3 scripts/check_changes.py --level full --classification infra
```

Write a machine-readable outcome with `--report /tmp/check-report.json`. A failed
check stops execution and records failure; dry runs are marked planned. The
selector never treats an omitted or skipped check as passed. For a text-only RC
or stable candidate, use the prior full baseline and its check receipt through
`--baseline-report`; CI obtains that receipt from the trusted Checks workflow.
Text changes do not cause unit tests, but unqualified functional changes since
the last full baseline still need full qualification.

For ordinary pushes, CI can use the latest commit's `Check-Classification: ui-copy`
trailer after you review the **whole push diff**. An optional `Check-Base: vVERSION`
trailer supplies a validated baseline for a new branch. Without a reviewed trailer,
CI conservatively infers behavior from changed paths. Contradictory paths or an
unknown classification fail; missing release baselines still require broader checks.
Candidate dispatch inputs take precedence over push trailers. Cheap copy-only
candidate checks do not wait for an unrelated broader push run to finish.

```sh
git commit -m "Clarify metadata guidance" -m "Check-Classification: ui-copy
Check-Base: v0.1.0-beta.8"
```

Keep classification and baseline trailers together in the final paragraph. A blank
line between them ends the trailer block, so Git may omit the classification and
CI will conservatively select broader checks. Omit `Check-Base` when the normal
push/PR baseline already covers the reviewed diff.

## Browser tests

Install the Playwright-version-matched browser once per dependency update:

```sh
npm --prefix web ci
npm --prefix web exec -- playwright install chromium
```

On Linux CI, add `--with-deps`. Routine UI fixtures use headless Chromium.
PDF, media and the broad integration journey retain actual Chrome by default;
install Google Chrome on hosts running those suites. `COLLECTOR_TEST_BROWSER=chrome`
selects actual Chrome explicitly. `CHROME_PATH=/absolute/browser/path` overrides
the executable for controlled installations. A different browser does not establish
actual-Chrome compatibility.

Browser tests refuse macOS Codex `seatbelt` execution before launching a browser.
Run them in the permitted terminal context. This prevents the known abort during
macOS application registration; it does not suppress crash notifications or weaken
the browser/OS sandbox. Missing browsers and launch failures are infrastructure
failures and remain visible. Do not repeatedly retry the known-broken context.

```sh
npm --prefix web run test:smoke        # short real-server acceptance
npm --prefix web run test:integration  # retained broad provider/media/recovery journey
npm --prefix web run test:models       # queue, async work context, evidence labels
npm --prefix web run test:full         # all named browser journeys, serially isolated
node web/tests/run-suite.mjs browse fx04-review
node web/tests/browser-launch-check.mjs
npm --prefix web run test:scale        # optional 3,000-entry/import qualification
```

Short smoke covers login, artwork beside rows, search/detail navigation, a
management bundle, viewer denial, mobile overflow and current served assets.
Preparation, slow providers/encoders, restart/reindex and long playback remain in
named integration journeys. Pagination uses 102 entries/imports to cross both page boundaries; the optional
scale mode retains the original 3,000-entry/import workload. No meaningful
regression assertion was deleted.

Real-server fixtures default to `target/debug/collector-server` and, where needed,
`collector-media`. The selector builds them once when required. Set
`COLLECTOR_SERVER_BINARY` and `COLLECTOR_MEDIA_BINARY` to test explicit executables.
Some offline provider fixtures intentionally need debug-only endpoint hooks;
deployment builds never enable those hooks. The short smoke also accepts
`COLLECTOR_TEST_ORIGIN` and `COLLECTOR_TEST_PASSWORD` for a disposable, already
initialized release-image fixture. The container helper passes its exact generated
certificate in `COLLECTOR_TEST_CERTIFICATE_PEM` when enabling
`COLLECTOR_TEST_SELF_SIGNED=1`; this pins the disposable key across browser requests
and reloads. Never point its mutation workflow at production.

The shared harness bounds shutdown, reaps owned processes on interruption/failure
and leaves unrelated Chrome sessions alone. `node web/tests/harness-check.mjs`
checks process ownership/reaping without a browser. The repeated launch check
also exercises assertion failure and interruption with real browser processes.

## Packaging and release evidence

The existing Container release workflow remains the only publication pipeline.
Dispatch validation with `base_ref`, `classification`, optional comma-separated
`journeys`, and `check_level=beta|full`. No release tag means no publication.
For example, after pushing the candidate:

```sh
gh workflow run container-release.yml --ref codex/your-candidate -f base_ref=v0.1.0-beta.8 -f classification=ui-copy
```

The source-check job reuses an applicable successful same-SHA Checks receipt when
workflow/repository, baseline, required commands, run attempt and immutable artifact
digest agree. A successful broader browser/workspace result can cover a narrower requested
subset. Otherwise it runs the required checks. Copy/layout-only container validation without explicit feature journeys
uses the actual release image for its short browser acceptance and does not require
a separate debug build. Explicit feature journeys still run and may need debug
fixtures. UI behavior without named affected journeys conservatively runs the
complete browser suite; specify the affected journeys for faster focused coverage.
Full qualification retains the broad debug-fixture suites.

Source checks and native AMD64/ARM64 builds run concurrently. Both native runtime
smokes and AMD64 release-image browser acceptance must succeed before validation
is sealed. CI no longer compiles discarded standalone release binaries, including for
CLI-only text edits; their source-format checks and image CLI acceptance remain. Requested
standalone deliverables still need their own release build.

Source checks cache Rust dependencies and workspace outputs by
toolchain/platform/profile/input keys, including after an unrelated browser test
fails. A cache entry is never qualification evidence; Cargo still validates its
inputs and the selected checks must pass.
Docker uses pinned cargo-chef dependency recipes and ordinary cached image layers,
exported with BuildKit `mode=max`; unchanged dependencies survive web-only edits
and hosted-runner replacement. Source/assets/lockfile/toolchain changes still
invalidate their corresponding outputs. A cold cache remains a full build.

Validation schema 2 records selected source checks and native/browser outcomes.
The consumer rejects failed/skipped checks, wrong source, stale attempts, corrupt
archives and insufficient RC/stable functional qualification. Text-only RC/stable
qualification may cite its verified full baseline. Old schema-1 validation exports
need fresh validation; their published images and installed user data are untouched.
Publication continues to copy the exact accepted exports without recompilation.

See [container publication](container-release-pipeline.md) for artifact recovery,
source identity, expiry and promotion, and the [delivery plan](testing-and-packaging-plan.md)
for the original baseline and acceptance targets. See the
[dated delivery record](validation/testing-and-packaging-2026-10-26.md) for measured
local and hosted timings, cache effectiveness and completed promotion rehearsal.
