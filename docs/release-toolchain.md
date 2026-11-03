# Release Rust toolchain

`rust-toolchain.toml` is the tested repository baseline, not an independently
established minimum supported version. CI and Docker builder pins must agree.
Individual version checks and qualification outcomes belong in release evidence,
not a repeated chronology in this procedure.

Before each later release, check official current-stable Rust, install that exact
version for qualification, and record the decision. Advance the repository pin,
CI, Docker and build documentation together only after actual qualification.
Select source checks using [the change-based testing policy](testing.md).
Text-only edits do not trigger unit tests. Beta candidates use checks for the
complete change since their validated baseline; RC/stable require full functional
qualification or applicable full baseline evidence for unchanged behavior. Keep
relevant shared-library/consumer checks. Build deployment executables in release
mode once per target inside their actual deliverable, with current embedded assets,
and run the relevant browser/image acceptance. A container-only release does not
need discarded standalone release executables. Do not suppress new lints to pass qualification,
change sibling projects silently, or treat local tests as physical-device evidence.

## Recorded qualification

These are retained maintenance observations, not assertions about today's upstream
stable release. The original release records retain test outcomes and limits.

| Recorded check | Observed toolchain and result | Evidence |
| --- | --- | --- |
| 2026-10-09 | Rust 1.99.0 selected for alpha.2; PCM serialization Clippy correction preserved integer bytes without suppressing a lint | [October maintenance](validation/october-todos.md) |
| 2026-10-21 | Manifest dated 2026-10-15 selected Rust 1.99.1 (`58cf8f9c3`, compiler date 2026-10-13); qualified with matching repository/CI/native Docker pins, without shared-library changes | [Alpha.3 qualification](validation/container-publication-2026-10-21.md) |
| 2026-10-22, 2026-10-23, 2026-10-26 (beta.8 and beta.9), 2026-10-30, 2026-11-02 | Stable manifest rechecks still selected that same 1.99.1 baseline; no new toolchain/shared-library update was required by those checks | [Original maintenance observations in Git](https://github.com/AberrantWolf/junk-collector/blob/7f4a56c8d6b38724a371549d09b978bbcb8e5cb1/docs/release-toolchain.md) |

Alpha.1's Rust 1.95 result remains dated evidence. No lower-version support guarantee
is inferred from a newer tested pin; affected sibling lint findings and temporary
qualification adjustments remain in the October evidence.
