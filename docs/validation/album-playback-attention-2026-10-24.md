# Album playback attention — 2026-10-24

Requested outcome: find albums needing playback work without trying each album.
Library album rows show a warning icon and text; detail gives the actual reason.
A separate Show filter selects recorded whole-album playback blockers, retaining
collection/search/sort, scoped cursors, history and access controls. The existing
private-work attention filter is unchanged.

Application policy shares the album/list resolver with a recorded-only read mode,
then checks published track availability. No audio is opened or hashed to browse.
No permanent data, archive schema, originals or metadata authority is changed.
Readiness is recomputed on reads, including after preparation and reindex. At
most 200 music candidates are examined per request; the web automatically follows empty continuation pages, with cancellation on a
scope change, so healthy albums cannot hide later matches. Device codec, network-route and undetected on-disk corruption
are outside recorded readiness. This status makes no verification claim.

The initial validation below preceded publication. The later
[beta.6 publication record](container-beta6-publication-2026-10-24.md) supersedes
its release limitation; NAS configuration/restart and physical iPhone acceptance
of the new UI remain outstanding.

## Validation

Passed: architecture/documentation checks, formatting, generated OpenAPI/TypeScript
contract consistency, web type check/build, rebuilt embedded server, and workspace
Clippy with warnings denied. Nineteen targeted album tests passed, including
missing release discs before playback, unresolved originals, alternate-encoding
selection, preparation/admission and paginated authorized readiness through reindex.
The full archive test binary completed with 268 passed, 3 ignored and 6 failures.
All six failures occurred at the existing provider HTTP fixture's listener bind
(`archive.rs:240`, `EPERM`); no new feature test failed. Passed cases include
preparation changing filter membership, manual alternate selection removing the
blocker immediately, and Viewer readiness agreeing with Owner without exposing
the raw module payload.

`cargo test --workspace` was attempted and stopped at the existing AccurateRip
provider HTTP fixtures: all six could not bind their local listener (`EPERM`,
Operation not permitted) in this session's sandbox. It is not a passing full-suite
result. The real browse and purchase browser journeys and `test:smoke` were also
attempted; temporary listener startup was denied before browser interaction.
The updated journeys cover the visible filter/warning/reason, automatically
following an empty API page to later matches, and removal after track choices
without a page reload. Their JavaScript syntax checks pass, but browser behavior
is unqualified here. Physical iPhone
and deployed-server acceptance remain outstanding.


## Review correction — Opus-only preparation

The review reproduced a false warning for an album with every track published as
Opus. Readiness incorrectly required a primary (FLAC) availability row, and
recorded reads only preferred FLAC recipe coordinates over structural layout
coordinates. The repair accepts any available supported format and falls back to
retained Opus input coordinates when no FLAC recipe exists. FLAC stays preferred.
This changes no audio algorithm or stored recipe, and does not inspect media bytes.

The new regression failed before the repair and passes after it: two tracks at
different Opus bitrates, a warning/filter match after only one completes, removal
from the filter after both complete, agreement with live playback targets and
exact recorded coordinates, and preserved readiness through reindex. The encoder
fixture exercises publication/eligibility, not real Opus decoding or codec quality.

Correction revalidation (2026-10-24): both `album_playback_attention` archive
regressions pass, including the Opus-only case. Architecture/documentation checks,
`cargo fmt --all -- --check`, workspace Clippy with warnings denied, and
`git diff --check` pass. The browser acceptance limitation above remains unchanged.

## Release candidate revalidation — 2026-10-24

The beta.6 candidate passes architecture/documentation checks, Rust formatting,
web type checking/build, generated API contract consistency, workspace Clippy
with warnings denied, and the release-tool checks. Server and media binaries
were rebuilt after the web build. Real Chrome 156.0.8078.47 browse and digital
purchase journeys now pass, including the visible warning/filter/reason, sparse
continuation handling and immediate warning removal after alternate-track choices.
Chrome required an unsandboxed temporary-profile launch. These results supersede
the earlier browser-start limitation; physical iPhone acceptance remains open.

Full workspace tests and the broader browser smoke are running during candidate
preparation; exact-source CI and both native container smoke jobs must pass before
publication. See [beta.6 release notes](../releases/v0.1.0-beta.6.md).

The broader real Chrome smoke also passes, including actual audio start, seek,
next/previous, natural advance and revocation, bulk import/preparation, collection
lists, authentication/WebAuthn and mobile layout. Initial exact-source container
validation [40098766682](https://github.com/AberrantWolf/junk-collector/actions/runs/40098766682)
stopped at the existing generated fake encoder: Linux returned ETXTBSY while
probing its just-written script. The repair uses checked-in executable fixtures
for process failure and I/O-worker termination; the meaningful assertions remain.
Fresh exact-source validation is required after this repair.

## Final release outcome — 2026-10-24

Full local workspace tests pass: 506 passed, 0 failed, 5 ignored. Both exact-source
CI checks and native AMD64/ARM64 build/smoke acceptance pass after the fixture
repair. Beta.6 is published with verified public digests, source labels, assets
and `preview` promotion; see the [publication record](container-beta6-publication-2026-10-24.md).
NAS deployment and physical iPhone acceptance of the attention UI remain open.
