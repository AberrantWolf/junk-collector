# Layered library refresh — 2026-11-02

Source implementation for the requested web/Trash Amp workflow.
[Publication review](layered-library-refresh-review-2026-11-02.md) records the
subsequent isolated source review and main-publication checks. This is not a
published release, deployment or actual NAS/phone/cellular latency qualification.
[Contract](../layered-library-refresh.md); [single queue](../remaining-work.md).

## Behavior

Collector now supplies independent authorized counts and compact native/web title
pages from the existing projection boundary. Stateless revision-bound cursors and
byte budgets include the response envelope and continuation, including multibyte
sort keys. Count/page/header reads make no client-state writes. Track headers/pages
use a separate current-source/role/language/tool-policy projection; pending/changed
tracks never require full media reads. Track freshness is fenced per album rather
than by unrelated catalog activity. Import overlays and role checks are batched.

The web shows counts and accepted pages incrementally, preserves visible results
on refresh failure, uses revisioned artwork URLs and validates track-cache headers.
Trash Amp owns all refresh/progress/cache state in the separate worktree below.
It publishes partial title pages, retries pending details/revision conflicts with
cancellation and a bounded wait, loads album/list contents on demand and searches
songs through the existing server search surface. Its client manifest commit merges
an album that finished loading during enumeration. Source/selection/attempt fences
prevent late results from changing a different view or playback sequence.

Playback no longer hashes an entire prepared file before any of core HTTP, private
playback or direct HTTPS sends bytes. Native retained offline playback no longer
hashes its entire file first; native online playback no longer fetches the album
track list before opening its frozen target. Open/seek/length and premature EOF
failures remain real request/read failures. Existing native recovery records failed
queue files and continues. Completed downloads still validate length/digest; explicit
integrity Jobs and import/preparation validation are separate. Same-length silent
corruption may reach the decoder before independent integrity work detects it.

## Review corrections retained

- The old local-media test expected the complete file plus the requested range to
  be read. Its meaningful expectation now requires only the requested ranges;
  new core tests reject an attempted tail read and require a truncated body to fail.
- The old offline test still rejects a length mismatch; a same-length changed
  vector now proves that playback does not perform a hidden integrity pass.
- A new real-TLS interleaving found that a selected album could load after the last
  title-page callback and then be lost by the first complete manifest commit. The
  commit now merges the per-album cache at that boundary; the regression is retained.
- A broad run exposed an over-broad 200 ms read-lock change in general command
  authorization: concurrent scan admission returned busy instead of converging.
  General command/auth serialization was restored; the bound applies only to the
  new catalog-query ports. The concurrency regression passed in isolation afterward.
- Browser fixtures that consumed `client-tracks` as an array were updated to consume
  its actual revision-bound page envelope and all pages. The meaningful preparation,
  ordering and media assertions remain; a protocol change is not treated as copy.
- Initial native acceptance called synchronous artwork work from a Tokio test
  runtime. Tests now explicitly request the asynchronous artwork layer. Runtime
  failure is retained as a failed attempt, not playback evidence.
- One earlier Collector full run reached a rustdoc missing-crate failure while
  implementation and shared artifacts were changing. Final qualification uses the
  settled source; that earlier run is not represented as passing.

## Validation

Validation selected from the complete staged/unstaged/untracked candidate with
`python3 scripts/check_changes.py --dry-run` (infra, baseline `83ad25a`). This
candidate also includes the previously uncommitted digital-album-adoption delivery;
those changes were retained rather than reverted or treated as this task's work.

- Collector final workspace tests: **602 passed, zero failed, five existing
  ignores**, including the final asset revision regression assertions and doctests.
- Trash Amp workspace tests against the local override: **248 passed, zero failed,
  three existing ignores**, including real TLS and the stale same-target UI case.
  Strict workspace/all-target Clippy and format checks pass. Clippy initially found
  an equivalent nonminimal boolean predicate; its suggested simplification passes.
- Collector strict workspace/all-target Clippy, architecture/document links,
  formatting and diff checks pass. Generated OpenAPI/TypeScript contracts match.
- Web type checking, build and queue/context/label model tests pass.
- All 30 broad browser journeys pass across the broad and five-journey completion
  runs. The game-audio fixture initially assumed an array and then did not retry a
  legitimate 503; both failed attempts were corrected to the real page contract
  and retryable response policy before passing. Subsequent explicit denial checks
  pass. The final artwork/busy-response follow-up passes browse, accounts,
  documents and separate-origin remote-media; smoke passed in the preceding run.
- Tooling, container-tools, release-policy, container-validation and purchase-mail
  policy checks pass, with their existing fixture skip retained.
- A subsequent artwork/authorization run passed smoke but exposed a real short
  catalog-busy response during removal in browse. The web now retries 503 through
  its client-owned poller; a deterministic one-shot 503 fixture covers recovery.
  The first follow-up reached the persistent-503 case and found its manual Retry
  control missing. Automatic retry now retains that explicit recovery control;
  the failed acceptance attempt is not counted as a passing run. The final browse
  run passes both automatic one-shot recovery and explicit persistent-error retry.
- Asset reads now require the revision that the application authorized; the store
  refuses a changed revision before resolving the immutable payload. Regression
  assertions cover stale versus current revision after a visibility edit.

Local compact Library fixtures returned 62 rows in 32,783 bytes in 6–17 ms across
repeat runs; the final follow-up recorded 12/8 ms. These are loopback fixtures,
not NAS/WAN/device measurements. Local rendering artifacts are `/private/tmp/trash-layered-phone.png` and
`/private/tmp/trash-layered-desktop.png`; actual Slint phone 390×844 and desktop
1100×760 loading states were visually inspected. Progress, loading text, title
wrapping and playback controls fit. The temporary preview source was removed.

## Source coordinates

Uncommitted source is identified by base plus SHA-256 of `git diff --binary --
Cargo.toml Cargo.lock crates api web` (documentation is excluded from that digest):

- Collector base `83ad25a3057f4aad5e0a90e52c795d74e80d821e`, source diff
  `4424ca75d2530b0cb569aa3896843465ba5ab45391a4ea05ee7ec82737fe01f4`.
- Trash Amp base `61f105f6ee1bab199ffc1c14ff5a6ab3ff8edf2d`, source diff
  `70cd190aa615c834965aa477fa0a9db2150d7010b9faf57a3ecf5b49743e445f`.
- Native `local-collector.toml` SHA-256:
  `c1d0b0a955dc8d52c65b7a5a7e0273e3925e2d30ee1300c827387e083955bc77`.

Local logs: `/private/tmp/collector-layered-final-tests.log`,
`/private/tmp/collector-layered-final-clippy.log`,
`/private/tmp/trash-layered-settled-tests.log`,
`/private/tmp/trash-layered-settled-clippy.log`, broad browser logs
`/private/tmp/collector-layered-broad-browser.log` and
`/private/tmp/collector-layered-final-browser-recovery.log`; artwork follow-up logs
retain both failures and final acceptance separately:
`/private/tmp/collector-layered-artwork-browser.log`,
`/private/tmp/collector-layered-artwork-recovery-browser.log` and
`/private/tmp/collector-layered-artwork-final-browser.log`. The final web-only busy
recovery change was type-checked, built and tested through the rebuilt served
binary after the Rust workspace compilation; its Rust code is unchanged.

## Native worktree and integration

- Original checkout: `archive host/Programming/rust/trash-amp`, retained on main.
- Worktree: `/private/tmp/trash-amp-layered-refresh`.
- Branch: `codex/layered-library-refresh`, based on `61f105f`.
- `local-collector.toml` is an explicit development-only Cargo patch configuration
  pointing to this Collector checkout. Qualification uses `--config
  local-collector.toml --locked` and the shared native target cache.
- The local lockfile resolves those path overrides. Replace the development
  configuration/lock resolution with the intended reviewed Collector Git commit
  before merging/releasing the native change. No commit/push, native merge,
  worktree cleanup or deployment is performed by this implementation task.
- Existing downloaded audio, profiles, credentials and unresolved list receipts
  are preserved; disposable projections/caches gain explicit new fields. Archive
  originals, manual authority and unknown module records are unchanged.

The existing archive guard remains exact and fail-closed. A kernel-blocked remote
filesystem probe is not made cancellable by this delivery. The subsecond target
needs its actual healthy NAS/device measurement; local fixture timings do not
prove cellular reachability, audio fidelity or signed release-image acceptance.
