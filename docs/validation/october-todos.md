# October 9 TODO delivery validation

Scope: music-only actions, clear missing game catalog results, manual DAT upload/
review/activation, and the Rust release toolchain. Owning contracts are
[game catalog upload](../game-catalog-upload.md) and
[release toolchain maintenance](../release-toolchain.md).

The four recent TODOs are implemented and locally qualified. The dated checks
below do not accept the separate physical LA3/PJ1 gates.

Passed on the selected macOS ARM64 host with Rust 1.99.0:

- Architecture/documentation/fixture checks, formatting, TypeScript check and web build.
- `cargo test --workspace`: 385 passed, three existing tests ignored. Final focused
  regressions subsequently passed for exact-byte private-spool repair/retry and
  retained intake plans lacking the new presentation field. The retained-plan
  round trip preserves every original field and defaults eligibility safely.
- Strict Collector all-target Clippy with warnings denied; optimized release server
  build and executable help; final embedded-asset debug rebuild.
- Generated OpenAPI/TypeScript contract check and whitespace check.
- HTTP DAT-upload route/authorization/body bound and receipt-conflict regressions.
- Upload restart, advancing-clock replay, immutable bytes/history/reindex, malformed
  input, denied role, staging tamper and cancellation regression.
- Seven consumed junk-libs crate test suites: 204 passed, six existing ignored.
  Trash Amp amp-collector/amp-audio: ten tests passed.
- Phono's 41 library tests (one existing ignored test) in an isolated copy of its
  current source; its original `--locked` run failed because Collector's alpha
  package version differs from the old lock entries. Only the temporary copy's
  lockfile was refreshed offline. Sibling working-tree changes were preserved.
- Docker Linux ARM64 build from `rust:1.99-slim-bookworm` and actual non-root
  UID 10001 setup/doctor, assets, authentication denial/login, certificate-validated
  localhost HTTPS, and recreated-container account/session/TLS persistence.
  Disposable containers/volumes were removed. Result identity is recorded in
  `/private/tmp/collector-todo-container-validation.json`.

`npm --prefix web run test:game-catalog` passed in actual Chrome 153 against a
real disposable server. It imported raw game bytes, verified saved-original/
no-catalog wording and music-control omission in guided and bulk intake, checked
manual source links without visiting/downloads, rejected ZIP/empty/oversized and
malformed inputs, reviewed actual parsed contents, activated two versions while
retaining history, and ran a complete DAT match on the already-imported game.
Music controls remain visible for a music package. Desktop catalog review and
390-pixel mobile layout were visually inspected; no horizontal page overflow was
found. The test rejects external browser requests; none occurred. Screenshots are
at `/tmp/collector-game-no-catalog.png`, `/tmp/collector-game-dat-review.png`,
`/tmp/collector-game-dat-mobile.png` and `/tmp/collector-game-existing-match.png`.

`npm --prefix web run test:smoke` passed: sign-in/expiry, WebAuthn virtual
authenticator, listener/role exposure, inventory/artwork, imports, bulk metadata/
grouping/FLAC, list operations and actual playback/revocation. Its configured-DAT
path now opens the server-source disclosure; that source mechanism is retained.

Qualification repairs were exercised before recording success: duplicate provider
registration was removed, catalog review styling made names/variant/sample legible,
private staging is repairable only with freshly supplied bytes of the exact digest,
and new plan fields have a safe default for existing durable intake records. Browser
test selectors were corrected to the actual collection chooser, entry-edit and
processing controls; oversized fixture bytes use a file because Playwright's
buffer-input limit is 50 MiB.

Additional sibling strict-Clippy diagnostics are not suppressed: the unchanged
junk-libs-disc source reports five `chunks_exact_to_as_chunks` findings on Rust
1.99: `cue_audio.rs:337`, `pcm.rs:509`, `redumper/cdtext.rs:129`,
`redumper/pcm.rs:44` and `redumper/sidecars.rs:211`. Collector's warning-denied workspace Clippy passes with its pinned Git
libraries; the sibling tests pass. Updating/releasing junk-libs is separate work,
and no sibling files or dependency tags were changed here.

No claim of second-device LA3/PJ1, new live provider matching, all-platform Docker,
full-release acceptance, or a minimum supported Rust version follows from these checks.
