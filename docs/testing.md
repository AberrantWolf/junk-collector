# Testing and packaging

Choose checks from the complete change. Text-only changes do not require unit
tests. The selector records what ran and why; release images are always optimized
release builds. Current policy is `change-checks-v3`.

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
Browser selection is independent of workspace coverage. Local and beta checks
use the short smoke, any named journeys, and journeys whose test files change.
Changes to shared test helpers select the full browser set. When a behavioral
web change has no selected journey, the broad integration journey is the
fallback. Name the affected journeys whenever a browser workflow
changes, including backend-only changes. Full functional qualification and
beta qualification without a usable baseline retain the complete browser
suite. An isolated workspace
version bump with matching lockfile version updates adds no source test
requirements. Dependency and other manifest changes keep their normal coverage.
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
python3 scripts/check_changes.py --base v0.1.0-beta.22 --level beta --source-only --journey opus-preparation --journey email-purchases
```

Local defaults compare HEAD with the whole working tree. An explicit `--base`
also includes every committed change after that baseline. For release source
checks in CI, supply a published version tag. The release workflow checks its
published image record and successful validation run, and requires the tagged
commit to be a strict ancestor of the candidate. Missing or unverified evidence
selects full source and native checks. To
request full functional qualification, run

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
Candidate dispatch inputs take precedence over push trailers. Container
qualification can wait up to 30 minutes for an in-flight Checks run in the
same repository at the same commit, including when the dispatch names
journeys. Cheap plans without Rust tests or browser journeys check only
completed runs. Release tag pushes do not start a second Checks run.

```sh
git commit -m "Clarify metadata guidance" -m "Check-Classification: ui-copy
Check-Base: v0.1.0-beta.22"
```

Keep classification and baseline trailers together in the final paragraph. A blank
line between them ends the trailer block, so Git may omit the classification and
CI will conservatively select broader checks. Omit `Check-Base` when the normal
push/PR baseline already covers the reviewed diff.

## Standalone CD capture checks

For the standalone operator CD capture helper, use focused offline checks:

```sh
cargo test --locked -p collector-cd-identity
cargo clippy --locked -p collector-cd-identity --all-targets -- -D warnings
cargo build --release --locked -p collector-cd-identity
python3 -B scripts/test_rip_cd.py
COLLECTOR_CD_IDENTITY="$PWD/target/release/collector-cd-identity" python3 -B scripts/test_cd_naming.py
```

The naming suite uses the release utility with fake redumper/HTTP inputs and
checks timeout/termination cleanup through process-state inspection. On macOS,
run these executable fixtures outside Codex's seatbelt context; sandbox launch
delays/process-inspection denial are infrastructure failures. These tests never
open an optical drive or contact MusicBrainz. See
[the naming acceptance record](validation/cd-folder-naming-2026-10-27.md).

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
npm --prefix web run test:cd-library   # saved CD defaults and a real library AccurateRip batch
npm --prefix web run test:full         # all named browser journeys, serially isolated
node web/tests/run-suite.mjs browse fx04-review
node web/tests/run-suite.mjs opus-preparation # real opusenc, or set COLLECTOR_OPUS
node web/tests/browser-launch-check.mjs
node web/tests/run-suite.mjs storage # isolated archive loss, visible 503, restart and retained authority
npm --prefix web run test:scale        # optional 3,000-entry/import qualification
```

Short smoke covers login, artwork beside rows, search/detail navigation,
management bundles, viewer controls, mobile overflow and current served assets.
Worker settings and CD defaults use their dedicated checks.
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
`journeys`, and `check_level=beta|full`. Use `qualify_runtime=true` to request
the extended native Compose and media proxy checks for a beta. Publication
requires a release tag.
For example, after pushing the candidate:

```sh
gh workflow run container-release.yml --ref codex/your-candidate -f base_ref=v0.1.0-beta.22 -f journeys=opus-preparation,email-purchases
```

The source-check job reuses an applicable successful same-SHA Checks receipt when
workflow, repository, required commands, run attempt and immutable artifact
digest agree. The full candidate diff still determines the required commands.
A broader browser or workspace result can satisfy those requirements even
when its own change selection used a different baseline. Text-only RC or
stable qualification that borrows a full receipt still requires the same
baseline. Otherwise the job runs the required checks. Container validation runs short
browser smoke once, against the actual AMD64 release image. Selected feature
journeys can still need debug fixtures. Full qualification retains those suites.

Source checks and native AMD64/ARM64 builds run concurrently. Both native
runtime checks and AMD64 release-image browser acceptance must succeed before
validation is sealed. Routine native checks cover Compose setup, packaged tools,
health, sign-in, mounts, clean shutdown and media isolation. Extended checks add
proxy, inbox, restart and backup/restore coverage on both platforms for full
qualification, changes to the packaged deployment, candidates without a usable
baseline, or an explicit request. The image CLI covers container release
acceptance. Requested standalone deliverables still need their own release build.

Source checks cache Rust dependencies and workspace outputs by
toolchain/platform/profile/input keys, including after an unrelated browser test
fails. A cache entry is never qualification evidence; Cargo still validates its
inputs and the selected checks must pass.
Docker uses pinned cargo-chef dependency recipes and ordinary cached image layers,
exported with BuildKit `mode=max`; unchanged dependencies survive web-only edits
and hosted-runner replacement. Source/assets/lockfile/toolchain changes still
invalidate their corresponding outputs. A cold cache remains a full build.

Validation schema 2 records selected source checks and native/browser outcomes,
including the native runtime scope for this policy.
The consumer rejects failed/skipped checks, wrong source, stale attempts, corrupt
archives and insufficient RC/stable functional qualification. Text-only RC/stable
qualification may cite its verified full baseline. Establish a full baseline
under the current policy before reusing it for a text-only RC or stable candidate.
Publication continues to copy the exact accepted exports without recompilation.

See [container publication](container-release-pipeline.md) for artifact recovery,
source identity, expiry and promotion, and the [delivery plan](testing-and-packaging-plan.md)
for the original baseline and acceptance targets. See the
[dated delivery record](validation/testing-and-packaging-2026-10-26.md) for measured
local and hosted timings, cache effectiveness and completed promotion rehearsal.
