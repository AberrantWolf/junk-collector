# CHD/Zstandard dependency security repair — 2026-10-25

Authorized scope: upgrade Collector to `chd` 0.3.5, raise the minimum in
`junk-libs`, run affected sibling checks and push the shared change. This closes
the Zstandard deferral in the earlier [first-pass](dependency-security-2026-10-25.md)
and [XML follow-up](quick-xml-security-2026-10-25.md) records without rewriting
their historical results. No application policy, decoder orchestration, archive
schema, original data, release image or live installation changes.

## Changes and publication

Shared commit `e4d536b7c23d2075c8a98f6a1525e1bcaa32eef0`, based on the accepted
XML fix `b343ea5`, raises the workspace minimum from `chd = "0.3"` to
`chd = "0.3.5"` (compatible 0.3 releases, excluding 0.4). The released CHD
0.3.5 crate requires `ruzstd` 0.9.1. The only other shared changes are its codec
oracle test and README. No fork or local replacement decoder was introduced.

The user-authorized push to `git@github.com:AberrantWolf/junk-libs.git` succeeded
on `codex/fix-chd-security`. Independent `git ls-remote` confirms that branch's
head is the exact shared commit above. Collector fetched it through the normal
Git dependency mechanism and pins all seven shared crates to the same revision.
The targeted lockfile update changes `chd` 0.3.4 → 0.3.5 and `ruzstd` 0.8.3 →
0.9.1, retaining the previous security fixes. No path override is committed.

## Independent codec acceptance

The existing explicitly provisioned `chdman` 0.285 round-trip oracle now forces
`cdzl` (deflate), `cdzs` (CD Zstandard) and `zstd` (generic Zstandard) separately.
Each generated mixed data/audio input has three data and five audio raw main-channel
2352-byte frames. CHD alignment padding and its 96-byte subchannel slots are
excluded from the compared track bytes. Each forced codec must be advertised in
the generated header, and every complete ordered output track must match the
existing independently fixed SHA-256 and byte-length vectors. Truncating each
generated CHD to half its encoded length must fail hashing rather than return a
partial successful result. All three codec runs passed.

This qualifies lossless codec/track-byte handling for the generated vector, not
physical-drive behavior, catalog verification or all malformed CHDs. No full
decompression-bomb reproduction or platform memory-limit certification is claimed.
The upstream patched decoder version and advisory database establish resolution
of the known dependency advisory; the integration tests check supported behavior
and fail-closed handling for the selected truncated inputs.

## Checks

Shared checks passed in `/private/tmp/junk-libs-quickxml`:

- `cargo test -p junk-libs-disc -p junk-libs-audio -p junk-libs-game-formats --all-features --locked`.
- `cargo test -p junk-libs-game-formats --features chd --test chd_oracle --locked -- --ignored`,
  using installed chdman 0.285.
- `cargo clippy -p junk-libs-disc -p junk-libs-audio -p junk-libs-game-formats --all-targets --all-features --locked -- -D warnings`.
- `cargo fmt --all -- --check` and `git diff --check`.

Sibling checks use isolated snapshots under `/private/tmp/junk-quickxml-siblings`
from the XML delivery, preserving both primary checkouts and Trash Amp's unrelated
working changes. Retro Junk uses its normal relative shared-library paths. Trash
Amp uses temporary command-line patches for the consistent shared crate set,
including audio/disc/core, and its snapshot lockfile resolves CHD 0.3.5 and
ruzstd 0.9.1. Sibling production pins/lockfiles are not modified by these checks.

- Retro Junk tests and all-targets Clippy passed for `retro-junk-disc`,
  `retro-junk-lib` and `retro-junk-archive-import`.
- Trash Amp tests and all-targets Clippy passed for `amp-audio`, `amp-library`
  and `amp-collector`, including real-server online/offline integration. Existing
  explicitly ignored hardware/manual cases stay ignored; no full desktop/iOS
  build or physical-device acceptance is claimed.

Collector checks passed against the published shared pin:

- `python3 scripts/check_architecture.py`, including documentation links and
  fixture manifests.
- `cargo fmt --all -- --check` and `git diff --check`.
- `cargo test --workspace --locked`, including complete archive/recovery and doc
  tests. Existing annotated manual/hardware/scale cases remain ignored.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`.
- `npm --prefix web run check` and `npm --prefix web run build`.
- `cargo build --locked -p collector-server`, after the web build.
- `npm --prefix web run test:smoke`: real Chrome 156.0.8078.47 with a temporary
  local server/profile; authentication/WebAuthn, provider work, actual playback,
  bulk import and collection-list journeys pass.

All validation here is local macOS source acceptance. No new Linux/container,
NAS update or physical-device qualification is implied. The published shared
source is fetchable; Collector's working changes have not been released/deployed.

## Refreshed audit

`cargo audit --db /private/tmp/junk-collector-rustsec-advisory-db` fetched RustSec
on 2026-10-25, loaded 1,352 advisories, refreshed the crates.io index and scanned
414 locked dependencies with yanked-package checks. Exit status is zero: no
vulnerability advisories remain. Only the existing unmaintained warnings for
`paste` 1.0.15 and `rustls-pemfile` 2.2.0 remain; no ignore was added.
Their proposed replacements remain in the consolidated [remaining-work queue](../remaining-work.md).
