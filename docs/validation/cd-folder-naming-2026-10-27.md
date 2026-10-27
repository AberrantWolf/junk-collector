# Automatic CD capture folder naming — 2026-10-27

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
