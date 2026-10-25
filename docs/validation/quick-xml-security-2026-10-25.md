# Shared XML security repair — 2026-10-25

The user authorized the quick-xml fix in `junk-libs`, Collector adoption and
affected sibling checks, leaving `ruzstd` for afterward. The earlier
[dependency update record](dependency-security-2026-10-25.md) remains the dated
first-pass result; this record supersedes its XML deferral.

## Source and publication

The shared change is commit `b343ea540dcc5a389718e55e2be590040071118c` on
`codex/fix-quick-xml` in `/private/tmp/junk-libs-quickxml`, based directly on
Collector's former `193e0a8` pin. Only the shared manifest, DAT parser, its tests
and README change. The primary `junk-libs` checkout stays unchanged.

Collector's seven shared-crate pins move together to that commit. Its lockfile
retains the previous security upgrades and updates quick-xml from 0.37.5 to
0.41.0; `chd` 0.3.4 and `ruzstd` 0.8.3 remain unchanged. No local path override
is committed. The local commit was copied into Cargo's Git cache solely for
offline validation of the exact remote-form pin.

**Source publication completed, 2026-10-25:** automatic approval review initially
rejected pushing the new branch because source egress to that destination needed
explicit authorization. The user subsequently approved the push.
`git push -u origin codex/fix-quick-xml` succeeded to
`git@github.com:AberrantWolf/junk-libs.git`; an independent `git ls-remote origin
refs/heads/codex/fix-quick-xml` returned the exact validated commit
`b343ea540dcc5a389718e55e2be590040071118c`. The source pin is now remotely fetchable.
No Collector image, NAS installation, archive or public API was changed.

## Parser behavior

The parser uses quick-xml's plain `Reader`, not `NsReader`. Attribute iteration
retains duplicate checks and now benefits from the patched bounded-complexity
implementation. The nondeprecated XML 1.0 normalized-attribute API replaces the
old unescape call. Predefined/numeric text entity references are separate events
in modern quick-xml: text/CDATA/reference fragments are accumulated before a
single trim, preserving internal spaces and complete field values. Unknown,
malformed and user-defined DTD references fail instead of disappearing or being
expanded. Accumulated field text is capped at 64 KiB, including input split across
many CDATA sections. Existing total-input/token/depth/entry bounds remain.

The neutral DAT types and application policy are unchanged. Regression vectors
cover Unicode, all predefined entities, decimal/hex references, one-time decoding,
serial inheritance, split text/CDATA, one-byte reader buffers, malformed/unknown
entities, DTD refusal, thousands of attributes with duplicate rejection and size
bounds. Collector's existing real browser catalog journey now includes an escaped
ampersand in game/ROM names and in version text, with complete displayed values
checked after upload, activation, manual override and refresh.

## Validation

Passed against the exact locally cached shared commit on this macOS checkout:

- `python3 scripts/check_architecture.py`, including documentation links and
  fixture manifests.
- `cargo fmt --all -- --check` and `git diff --check`.
- `cargo test --workspace --locked --offline`, including archive/recovery tests
  and doc tests. The existing generated scale qualification remains ignored by
  its annotation.
- `cargo clippy --workspace --all-targets --locked --offline -- -D warnings`.
- `npm --prefix web run check` and `npm --prefix web run build`.
- `cargo build --locked --offline -p collector-server`, after the web build.
- `npm --prefix web run test:game-catalog`: complete entity-containing game/ROM
  names and version text survive the real mixed DAT/ZIP upload, activation,
  manual override, immediate refresh/search and future-import workflow.
- `npm --prefix web run test:smoke`: real Chrome 156.0.8078.47, isolated temporary
  server/profile, authentication/WebAuthn, provider jobs, actual playback, bulk
  import and collection-list journeys.

`cargo audit --db /private/tmp/junk-collector-rustsec-advisory-db --no-fetch`
completed with yanked-package checks and reports exactly one vulnerability and
two maintenance warnings across 414 dependencies. Its nonzero exit is expected
for the explicitly deferred `ruzstd` advisory. No advisory was ignored. These
local checks do not establish Linux/container or physical-device acceptance.

Sibling validation uses isolated snapshots under `/private/tmp/junk-quickxml-siblings`
and the patched shared DAT crate. Retro Junk's clean HEAD snapshot uses its normal
relative path dependency. Trash Amp's current working source snapshot retains its
unrelated in-progress work and uses a command-line DAT patch for its Collector
integration; its existing Collector and other shared-crate pins are unchanged.
Build caches were reused, without changing either sibling's working source or
lockfile.

- Shared DAT: all 10 parser tests, doc tests, workspace formatting and all-targets,
  all-features Clippy passed.
- Retro Junk: tests for `retro-junk-dat`, `retro-junk-catalog`, `retro-junk-import`
  and `retro-junk-archive-import` passed. Production-library Clippy passed. The
  all-targets run fails only on the pre-existing 102-line `releases_to_enrich_query`
  test in `retro-junk-import/tests/scraper_import.rs:380`; the same failure was
  reproduced against the untouched original checkout with its old parser. No
  lint suppression or unrelated test rewrite was added.
- Trash Amp: `amp-collector` tests and all-targets Clippy passed with the patched
  DAT crate in the integration dependency graph. Its existing two ignored tests
  remain ignored. No full desktop/iOS build or physical-device qualification is
  implied by this targeted check.

## Audit and maintenance follow-ups

The local lockfile audit clears RUSTSEC-2026-0194/0195. The remaining vulnerability
is RUSTSEC-2026-0344 (`ruzstd` 0.8.3), deliberately deferred. Maintenance warnings
remain for `rustls-pemfile` 2.2.0 and `paste` 1.0.15; neither was changed by this
delivery and no advisory ignores were introduced.

`rustls-pki-types` exposes PEM parsing through `PemObject` and is the direct
replacement for the client's small `rustls-pemfile` use. `utoipa-axum` 0.3.0
replaces `paste` with `pastey` but requires Utoipa 6 and Rust 1.88, so that option
needs a coordinated schema/router upgrade and API-contract validation. The
consolidated [remaining-work queue](../remaining-work.md) owns these recommendations.
