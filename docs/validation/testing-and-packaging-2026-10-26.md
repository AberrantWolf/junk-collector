# Testing and packaging delivery — 2026-10-26

Delivered and qualified on source `2fe08847383c3c515b0c84d58cc96613cd71715c`.
This record distinguishes local measurements from hosted qualification. The
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
release-image browser acceptance also passes. Full hosted source qualification
and nonpublishing promotion rehearsal pass as recorded below. The isolated port-mapped TLS
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

The hosted full suite takes longer than the original 15-minute reuse wait. The
wait is now bounded at 30 minutes so a concurrently requested container candidate
can consume the successful source receipt instead of starting duplicate tests.
The enclosing job allows time for a fresh run if no usable receipt appears.

## Completed hosted qualification

[Checks 40272025020](https://github.com/AberrantWolf/junk-collector/actions/runs/40272025020)
passed the complete workspace, warnings-denied Clippy, web/model, tooling/provenance
and all 26 named browser journeys on the qualified source above. Its downloaded,
digest-verified receipt reports full coverage, committed source and unchanged HEAD/
working tree after execution. This is full functional coverage even though the
push workflow's requested level is beta; it selected the infrastructure profile.

[Candidate validation 40272028960](https://github.com/AberrantWolf/junk-collector/actions/runs/40272028960)
passed both native runtime/media smokes and AMD64 browser acceptance against the
actual optimized image. Image builds overlapped source checks. The source job
waited for and reused Checks' exact-SHA receipt; it skipped tool installation and
execution rather than running the suite twice. Failed earlier source checks had
correctly prevented assembly/sealing even with successful native image jobs.

| Measurement | Observed duration |
| --- | ---: |
| Complete source workflow | 15m12s |
| Workspace tests, including compilation | 351.92s |
| Workspace/all-target Clippy | 21.83s |
| Additional browser-server build | 1.12s |
| All 26 browser journeys | 443.29s |
| Short smoke within that suite | 5.07s |
| Retained broad integration | 137.63s |
| Routine pagination journey | 6.80s |
| Cached AMD64 / ARM64 image build steps | 7s / 5s |
| AMD64 / ARM64 runtime/media smoke steps, including tooling | 32s / 42s |
| AMD64 browser step, including installation | 27s |
| Complete candidate validation | 15m57s |

The hosted runners restored the Rust compilation cache. Both native dependency
cook and application-build layers were cached on the subsequent hosted run. The
separate local changed-web-asset experiment above establishes dependency reuse
when an application rebuild is actually needed; a fully cached image is not its
equivalent. The original candidate took approximately 24 minutes and did not run
the broad browser suite in CI. These runs have different coverage and cache state;
the measurements demonstrate the implemented overlap/reuse rather than guarantee
a fixed release latency.

[Promotion rehearsal 40273497267](https://github.com/AberrantWolf/junk-collector/actions/runs/40273497267)
passed in **47 seconds**. It authenticated and restored the saved exact-source OCI
exports, reverified both platform/source/digest records and packaged the installation
bundle with provenance. Source checks and builds were skipped. Registry login,
publication, channel promotion and release attachment were skipped. No release,
registry image or running installation changed.

`AGENTS.md` and the maintained guide now require change-based checks, no units for
reviewed text-only edits, supported browser launches and release-only deployment.
This record and the status/queue closeout are documentation-only changes after
the qualified code snapshot; they do not alter build inputs or testing policy.

## Publication follow-up — 2026-10-26

Published and installed on the NAS in beta.9 after fresh exact-source source/native
image/browser qualification. See [publication and deployment evidence](container-beta9-publication-2026-10-26.md).
The earlier measurements and source-only scope above remain dated historical evidence.
