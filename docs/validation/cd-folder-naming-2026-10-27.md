# Automatic CD capture folder naming — 2026-10-27

## Beta.10 release follow-up — 2026-10-27

Included in the signed beta.10 source release. Collector core/media are deployed
on the NAS after complete exact-source and native image qualification; the
standalone CD helper remains operator source tooling. See [publication, recovery
point and actual-host acceptance](container-beta10-publication-2026-10-27.md).
The dated source-delivery results and unrun physical/live cases below are retained
as historical evidence; no live-library batch or physical capture/eject was run
as part of deployment.

## Repair: Disc ID includes and native macOS eject — 2026-10-27

The operator's HTTP 400 confirms the earlier `inc=releases+labels+discids`
request was invalid. The earlier experiment's inferred requirement to add
`releases` was wrong; its HTTP 503 did not establish a valid request contract.
The MusicBrainz [Disc ID controller](https://github.com/metabrainz/musicbrainz-server/blob/master/lib/MusicBrainz/Server/Controller/WS/2/DiscID.pm)
accepts `labels` and supplies releases itself, explicitly enabling media and disc
IDs internally. The helper now requests only `inc=labels`, retaining `fmt=json`
and `cdstubs=no`. No retries or extra lookups were added; the existing total budget
and diagnostic/fallback behavior remain in place. Offline provider fixtures now
reject unsupported Disc ID includes instead of accepting the broken request.

The operator also reports redumper cannot eject their drives on macOS. Darwin
now calls `diskutil eject /dev/diskN` directly, normalizing `diskN`, `/dev/diskN`
and `/dev/rdiskN` from the pinned selection. Unknown/slice identifiers are refused,
and failed/incomplete capture never invokes eject. Other platforms retain the
existing redumper eject behavior and zero-exit-warning handling. Native eject
failures report the command's diagnostic/status without invalidating the capture.
All ejection commands are replaced by explicit fixture executables during tests;
neither real diskutil nor an optical drive was exercised.

Validation:

- `python3 -B scripts/test_rip_cd.py`: ten tests pass in 8.637 seconds, including
  all three macOS identifier forms, native eject failure, unknown/slice refusal,
  auto-selected drive retention, no eject on failed/incomplete capture and the
  retained Linux command/failure paths.
- `COLLECTOR_CD_IDENTITY="$PWD/target/release/collector-cd-identity" python3 -B
  scripts/test_cd_naming.py`: twelve tests pass in 9.548 seconds, no skips. Actual
  supervised lookup/naming uses a provider-contract fixture and fake native eject,
  including generated-name fallback. Both suites ran outside the macOS sandbox.
- One bounded live GET of the published six-track Disc ID with `inc=labels`
  returned HTTP 200. No provider refusal was retried. This validates the corrected
  request and response shape. The real response contained one release and resolved
  through `catalog_folder` to `DR E006`; no capture directory was changed. This
  does not qualify an owned-disc identification or physical eject.
- Focused shell/Python syntax, help inspection, architecture/documentation links
  and diff whitespace are checked for this operator-script repair. The selector
  preview conservatively selects infra/full for generic scripts; broad Rust/web,
  container, sibling and physical-drive qualification are omitted because this
  repair changes no compiled/shared/application code. The existing release identity
  utility is reused without rebuilding. Earlier records below retain their dated
  results and are superseded only for the corrected request/native-eject behavior.

## Follow-up: specific diagnostics and automatic eject — 2026-10-27

The user reported repeated generic naming errors and requested automatic eject.
The worker now returns a JSON result/error envelope instead of losing its cause
through suppressed stderr. Identity failures include exit status and the utility's
bounded, single-line diagnostic. Provider errors distinguish HTTP status (including
404/429/503 and refused redirects), connection failures, socket/overall deadline,
malformed/oversized JSON, no release, multiple releases, missing/ambiguous/unsafe
catalog numbers and inconsistent medium data. A rename collision reports its
destination path; other filesystem failures preserve the OS diagnostic. Missing
Python/identity executables are identified separately. These are functional error
propagation changes, not a copy-only classification. The fifteen-second total
budget, no-retry policy, exclusive rename and generated fallback are retained.

After successful capture, required-file checks and naming, the helper calls
`redumper eject --drive=<pinned device>`, including in manual mode and after
naming fallbacks. The eject command receives no image options, preventing writes
to the capture log. Upstream `redumper_eject` can log `warning: failed to eject`
while returning zero; that warning and nonzero exits both produce an eject-failure
message without reclassifying a saved capture as failed. Failed/incomplete captures
never reach eject. The installed executable's `--help` confirms the command;
no physical eject/read was performed during this delivery.

Focused validation uses the unchanged existing release identity utility and fake
HTTP/redumper fixtures. The naming suite covers actual worker diagnostic transport
through the shell, 503/404/429/refused redirects, connection/socket timeout and
identity details, ambiguous/no-match metadata, occupied paths, deadline/termination
cleanup, preserved originals and eject after naming fallbacks. The capture suite
covers automatic and explicit drive pinning, no eject after failed/incomplete
capture and both nonzero/zero-exit-warning eject failures. Final check outcomes:

- `python3 -B scripts/test_rip_cd.py`: eight tests passed in 3.479 seconds.
- `COLLECTOR_CD_IDENTITY="$PWD/target/release/collector-cd-identity" python3 -B
  scripts/test_cd_naming.py`: twelve tests passed in 6.637 seconds, no skips.
  This includes propagation of the provider's bounded JSON explanation for HTTP
  400, rather than merely showing a generic request rejection.
- Both fixture suites used the permitted outside-sandbox execution context;
  no drive was opened and no provider contacted. The unchanged release utility
  was reused without rebuilding.
- `bash -n`, actual `--help` inspection, Python syntax/whitespace checks,
  `python3 scripts/check_architecture.py` and `git diff --check`: passed.
- Selector preview conservatively selected infra/full coverage for the generic
  script paths. Focused shell/process/HTTP-fixture checks were selected because
  this follow-up changes only operator scripts and documentation. Rust/shared
  consumers, web/browser, container and live-drive/provider checks were omitted;
  their earlier evidence is not represented as new follow-up validation.

The original delivery evidence below remains dated
historical qualification; no Rust, shared library or deployed binary changes are
part of this follow-up.

## Original automatic naming delivery — 2026-10-27

The standalone [capture helper](../../scripts/rip-cd.sh) accepts `--auto
DESTINATION [DRIVE]`. It captures into a unique UTC timestamp/random directory,
retaining that name for failed/partial capture. Successful capture first checks
required files, then attempts descriptive naming with a 15-second total lookup
budget. Existing manual `NAME DESTINATION [DRIVE]` mode performs no lookup.

The release-built `collector-cd-identity` adapter reads at most 65,538 bytes per
TOC sidecar and reuses `junk-libs-disc` strict audio TOC/full TOC validation and
`junk-libs-disc-id` calculations. No audio payload or optical drive is opened.
Unsupported/malformed layouts, including data/four-channel/negative-start and
multisession evidence, cannot produce a naming identity. This small operator
adapter has no Collector domain, application, store or HTTP dependencies.

`scripts/name_cd_capture.py` supervises a separate process group for identity
extraction and the HTTPS request. The total deadline includes DNS/TLS and slow
response bodies; timeout/interruption kills the group. The body is limited to
1 MiB, redirects are refused and no failure/refusal is retried. The request asks
for releases, labels and disc IDs, with CD stubs disabled. Exactly one release,
one matching medium/track count and one safe non-placeholder catalog number
permit a folder name; multi-disc releases append ` - Disc N`. Multiple releases
remain ambiguous even when they report the same catalog number. Missing tools,
provider failures, malformed responses and unavailable names keep the generated
directory. Linux `renameat2(RENAME_NOREPLACE)` or macOS
`renamex_np(RENAME_EXCL)` performs the atomic rename; unsupported platforms/filesystems
retain the generated directory. Existing empty/nonempty directories and symlinks
are never replaced or used as a nesting destination. No capture file is rewritten.

This is optional operator naming policy, outside Collector's metadata acquisition
and evidence publication. The result neither identifies a particular physical copy
nor verifies catalog identity or audio. Original sidecars and logs remain intact;
lookup responses are not added to the captured package.

Validation completed on macOS:

- `cargo test --locked -p collector-cd-identity`: both adapter tests pass,
  including the independently published six-track MusicBrainz vector without any
  audio files and rejection of unsupported/truncated/oversized sidecars.
- `cargo clippy --locked -p collector-cd-identity --all-targets -- -D warnings`
  and `cargo fmt --all -- --check`: pass.
- `cargo build --release --locked -p collector-cd-identity`: built once; the
  resulting executable was used by wrapper acceptance. SHA-256:
  `07a2d70915fd96bd50de5f01f192815abfc838fd5a9149c16c60b107e4f08774`.
- `python3 -B scripts/test_rip_cd.py`: all five existing regression tests pass.
- `COLLECTOR_CD_IDENTITY="$PWD/target/release/collector-cd-identity" python3 -B
  scripts/test_cd_naming.py`: all eleven tests pass with no skips. Fixtures cover
  actual shared-library identity through the shell workflow, exact/multi-disc
  catalog names, conflicting/malformed/unsafe metadata, bounded response bytes,
  refusal without retry, redirect refusal, wall deadline and child termination,
  SIGTERM cleanup, atomic rename/collision protection, missing identity tools,
  unsupported TOCs, original-byte retention, ordinary shared-folder permissions,
  and capture failure before any lookup. HTTP and redumper are explicit fakes.
- `bash -n`, help-output inspection, Python syntax/whitespace inspection,
  architecture/documentation links and scoped diff whitespace checks: pass.

Initial sandbox fixture launches exceeded their deadlines and process-state
inspection was denied. Both fixture suites were run through the permitted
outside-sandbox execution context. A later expanded wrapper run also exceeded
its original eight-second outer fixture timeout; that timeout was shorter than
the documented fifteen-second naming budget. The outer fixture now allows thirty
seconds and kills its process group on failure. Dedicated half-second deadline
tests still require prompt cleanup; the final complete naming suite passed in
44.432 seconds including all wrapper cases and temporary executable startup.

The required selector preview proposed infra/full workspace coverage for the
complete dirty tree. Focused package/consumer (this executable has no in-repository
Rust consumers), release-artifact, shell and offline provider/workflow checks were
selected for this isolated operator delivery. The lockfile only registers the new
package with existing pinned dependencies; no third-party version changed. Broad
application/workspace, web/browser, container and sibling-project suites were
omitted: no shared library, Collector application contract or deployed artifact
was changed by this delivery. The tree contains pre-existing unrelated Rust, web
and documentation work, including earlier helper/refinement changes; these focused
checks do not qualify that work.

No physical disc was read. A live MusicBrainz request exposed the requirement to
include `releases` when requesting `discids`; the corrected request returned HTTP
503 and was not retried. Successful live lookup/naming and physical-CD timing
remain unqualified. Offline fixtures are explicitly synthetic and make no live
provider or audio-verification claim.
