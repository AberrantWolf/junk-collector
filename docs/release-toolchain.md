# Release Rust toolchain

Beta.8 preparation, 2026-10-26: the official current-stable manifest still selects
Rust/rustc 1.99.1 (`58cf8f9c3`, 2026-10-13), dated 2026-10-15. It matches the
installed toolchain, CI and Docker builder. No toolchain or shared-library update
is required for this release.

Beta.5 preparation, 2026-10-23: rechecked the official current-stable manifest;
its date remains 2026-10-15 and Rust/rustc remains 1.99.1
(`58cf8f9c3`, 2026-10-13). Installed toolchain, CI and Docker builder agree;
no toolchain or shared-library dependency update is required.

Beta.3 preparation, 2026-10-22: the official stable manifest was checked again;
its date remains 2026-10-15 and its Rust/rustc version is 1.99.1
(`58cf8f9c3`, 2026-10-13), matching the installed toolchain, CI and Docker builder.
No toolchain or shared-library dependency change is needed for this release.

Alpha.3 preparation, 2026-10-21: the official stable manifest dated
[2026-10-15](https://static.rust-lang.org/dist/2026-10-15/channel-rust-stable.toml)
selects **Rust 1.99.1** (`58cf8f9c3`, compiler date 2026-10-13). It was installed
side by side with 1.99.0 and passed source/release/browser qualification. Repository/CI pins, README and the exact
`rust:1.99.1-slim-bookworm` builder now agree. The builder tag exposes native
AMD64/ARM64 manifests. The [alpha.3 acceptance record](validation/container-publication-2026-10-21.md)
records passed source/browser/native-container gates, publication and isolated
actual-NAS Portainer update/full recovery.
No shared-library code or dependency pins changed in this delivery.


The October 9 maintenance delivery selects Rust 1.99.0, the current stable release
announced on [October 1](https://blog.rust-lang.org/2026/10/01/Rust-1.99.0/).
That was the tested baseline for alpha.2; the current alpha.3 baseline is above.
The existing warning-denied Collector Clippy finding in PCM serialization now uses
fixed-size `as_chunks_mut::<4>()` arrays, preserving the little-endian stereo bytes.
No lint was suppressed. The tested release baseline is not an independently
established minimum supported Rust version; no lower-version guarantee is declared.
The published alpha.1's dated Rust 1.95 validation remains historical evidence.

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
change sibling projects silently, or treat local tests as LA3/PJ1 hardware evidence.

[October TODO validation](validation/october-todos.md) records this upgrade,
including the separate sibling strict-Clippy findings and temporary-copy Phono
lockfile adjustment. Checkpoint/release names do not establish an MSRV or freeze
pre-release APIs/storage contracts.
