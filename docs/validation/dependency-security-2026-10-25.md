# Dependency security updates — 2026-10-25

Scope: apply straightforward dependency fixes requested after a Cargo audit.
No application policy, archive format, shared-library source, published image or
live installation is changed. Existing unrelated deployment documentation edits
are retained.

## Changes

| Crate | Before | After |
| --- | --- | --- |
| quinn-proto | 0.11.17 | 0.11.19 |
| rustls | 0.23.44 | 0.23.46 |
| rustls-webpki | 0.103.15 | 0.103.16 |
| webpki-roots | 1.0.9 | 1.0.11 |
| tokio | 1.53.1 | 1.54.1 |
| zip | 2.4.2 | 9.1.0 |

All changes except ZIP use existing compatible manifest requirements. ZIP's
provider manifest requirement becomes version 9 with the same disabled defaults
and `deflate` feature. The adapter continues to inspect members in memory, reject
unsafe paths/types and enforce member/expanded-byte bounds; it does not call the
filesystem extraction APIs cited by the ZIP advisories. Existing behavioral tests
exercise that contract against the upgraded library. ZIP 9's fallible filename
decoding is mapped to an invalid-input error. The upgraded `enclosed_name` strips
leading roots, so the adapter explicitly rejects leading `/` to preserve the
existing absolute-path rejection contract. Existing traversal/absolute-path,
nested-archive, member/expansion-limit and symlink tests remain meaningful; an
additional NUL member-name regression checks rejection without staging data.

## Audit

RustSec database freshly fetched on 2026-10-25, containing 1,352 advisories.
`cargo audit --db /private/tmp/junk-collector-rustsec-advisory-db --no-fetch`
reports three vulnerabilities across 414 locked dependencies, down from 16 across
415 dependencies. The Tokio unsoundness warning is also cleared. No advisory
ignores were introduced. The final audit includes yanked-package checks, using
approved cache access after the initial sandbox-blocked attempt.

Remaining advisories are `quick-xml` 0.37.5 (RUSTSEC-2026-0194/0195) through
`junk-libs-dat`, and `ruzstd` 0.8.3 (RUSTSEC-2026-0344) through `chd` 0.3.4 and
`junk-libs-disc`. Both are behind the pinned shared-library revision. `paste`
1.0.15 and `rustls-pemfile` 2.2.0 retain unmaintained warnings. Follow-up scope is
recorded only in [the consolidated queue](../remaining-work.md).
An advisory match does not establish reachability of its vulnerable API.

## Validation

Passed on this macOS checkout:

- `python3 scripts/check_architecture.py` (architecture, documentation links and
  fixture manifests).
- `cargo fmt --all -- --check` and `git diff --check`.
- `cargo test --workspace --locked`, including the existing ZIP safety tests and
  the new NUL-name rejection test. The generated scale qualification remains
  explicitly ignored by its existing test annotation.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`.
- `npm --prefix web run check` and `npm --prefix web run build`.
- `cargo build --locked -p collector-server` after the web build and final source
  adjustment.
- `npm --prefix web run test:smoke`: real Chrome 156.0.8078.47, temporary
  server/profile, WebAuthn, provider work, actual playback, bulk and collection-list
  journeys.
- `npm --prefix web run test:game-catalog`: rerun against the final server;
  mixed DAT/ZIP detection, malformed rejection, activation, manual authority and
  immediate library/search refresh with offline provider inputs.
- `npm --prefix web run test:local-playback`: real pinned QUIC playback/seeking,
  wrong-pin rejection, public media refusal, logout/source release and unsupported
  transport refusal with the upgraded TLS/QUIC dependencies.

The initial workspace run identified ZIP 9's changed absolute-path handling;
the adapter repair preserves the old application's rejection policy, and the
complete final suite passes. Shared-library source and sibling projects were not
changed. Linux/container/device deployment qualification is not implied by these
local results. Audit remains nonzero for the explicitly deferred advisories above.
