# Testing and packaging delivery — 2026-10-26

Implementation and acceptance are in progress. This record distinguishes completed
local checks from remaining native/hosted qualification. The
[approved plan](../testing-and-packaging-plan.md) records the prior workflow costs;
[testing instructions](../testing.md) describe current commands and selection.

The completed pre-existing raw-CD helper, its documentation, historical deployment
records and approved plan were committed first as `edf46df`. Tooling changes began
from that checkpoint; no production code or deployment setting is changed.

## Local source and browser evidence

- Workspace tests, Rust formatting, warnings-denied workspace/all-target Clippy,
  web type checking and production web build pass.
- Selector regressions cover text-only omission at every level, complete staged/
  unstaged/untracked/base diffs, downstream crates, unknown/mixed scope, failure
  propagation and missing baselines. Source-receipt tests cover wrong source,
  stale attempts, insufficient scope and required command mismatches. Release
  evidence rejects missing/skipped platform checks and insufficient RC coverage.
- The harness refuses macOS `CODEX_SANDBOX=seatbelt` before browser startup. The
  original launch failure was established by the October 25–26 crash reports and
  the [recorded failing sandbox / successful permitted retry](metadata-attention-notice-2026-10-26.md).
  No further deliberately crashing launch was required.
- Ten headless Chromium launches/exits, intentional assertion failure and ordinary
  interruption passed, with the owned browser process confirmed reaped. Separate
  process tests check forced reaping and descendant cleanup after interruption.
  No new Google Chrome crash report appeared; the latest remains October 26 11:31.
- Short smoke passed in 5.50 seconds (subsequent suite run 4.86 seconds), including
  actual login, collection/item creation, artwork upload and row rendering, search,
  details, management bundle, viewer denial, mobile overflow and served-asset checks.
- Retained broad integration passed in 134.74 seconds with actual Google Chrome,
  virtual WebAuthn, provider concurrency, FLAC playback/seek/revocation, bulk intake
  and collection lists. Browse, AccurateRip panel/status and overflow-action suites
  also pass. All named local journeys pass, including security/reauthentication, certificates,
  local and remote playback, documents, game audio, purchase imports and lifecycle
  replacement. The revised broad integration passes again in 134.03 seconds.

The existing scan journey needed to expand the current server-configured catalog
section before importing. Its actual import/activation/scan assertions are retained.
The scan fixture also moved from unsupported historical NES/raw catalog coordinates
to a supported Game Boy/No-Intro raw-byte fixture, retaining the real 64 MiB scan,
ambiguous result and no-match checks. The pagination fixture now defaults to 102
entries/imports, crossing both page boundaries with its original assertions;
`test:scale` retains the explicit 3,000-entry/import workload. The unmodified scale
setup exceeded the new ordinary three-minute fixture deadline before reaching
browser assertions. Exact-wording label and account-error assertions were removed. Their replacements
retain known-state labels, distinct audio outcomes, refusal of wrong-account work
and the guarantee that tentative/unknown results cannot look verified. Git-diff
fixtures disable inherited signing/hooks and bound Git commands; unit tests never
need the user's signing agent. No copy-only unit test or shallow layout assertion
was added.

## Container and CI evidence

A native ARM64 release-mode image builds successfully with the pinned cargo-chef
recipe layer. The dependency cook took 99.7 seconds; the final application build
102.8 seconds on this local Docker host. A separate temporary build context changed only a web asset: the dependency cook
was cached and the complete rebuild took **91.73 seconds**, including an 87.63-second
application build. These are local timings, not an estimate of hosted release latency.

The existing host-network Compose smoke did not become reachable from macOS Docker
Desktop. No Docker setting or production configuration was changed to work around
that host-specific limitation. Both native Linux architecture runtime/media jobs now pass; the corrected AMD64
release-image browser acceptance also passes. Full source qualification and
nonpublishing promotion rehearsal remain to
be recorded before claiming full acceptance. The isolated port-mapped TLS
release-image browser smoke passes in 2.70 seconds. Its readiness request validates
against the fixture's exact generated certificate; browser self-signed acceptance
is test-only. The deployed loopback/TLS listener requirements remain enforced.

YAML parsing and actionlint pass. Publication is not requested and no registry
image, release tag or production installation has been changed.

Source checks verify HEAD and the working tree again after execution. A mutated
source or regenerated uncommitted asset cannot produce a consumable successful
receipt. A successful full browser/workspace result can cover a requested subset;
its declared commands are still validated before reuse. CLI-copy container
candidates rely on the actual image build rather than compiling another standalone
release binary. Held playback promises replace the old sub-second timing assumption,
and the lifecycle journey explicitly opens its known entry after global import.

The first complete Linux sweep also exposed a pre-existing Homebrew-only FLAC
path in the game-audio fixture. It now resolves the real encoder from PATH (or
`COLLECTOR_TEST_FLAC`) and checks availability before creating the fixture.
Production encoder selection is unchanged. The corrected prior source run reached
and passed the superseded-playback, pagination, lifecycle, security, local-route,
account, purchase and intake cases before this fixture prerequisite stopped it.
