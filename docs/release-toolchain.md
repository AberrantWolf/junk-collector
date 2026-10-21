# Release Rust toolchain

Alpha.3 preparation, 2026-10-21: the official stable manifest dated
[2026-10-15](https://static.rust-lang.org/dist/2026-10-15/channel-rust-stable.toml)
selects **Rust 1.99.1** (`58cf8f9c3`, compiler date 2026-10-13). It was installed
side by side with 1.99.0 and passed source/release/browser qualification. Repository/CI pins, README and the exact
`rust:1.99.1-slim-bookworm` builder now agree. The builder tag exposes native
AMD64/ARM64 manifests. The [alpha.3 acceptance record](validation/container-publication-2026-10-21.md)
records completed checks and publication separately; pending builds are not acceptance.
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
Run architecture/format, workspace tests and all-target Clippy with warnings denied,
web checks/build, the release executable, container build/runtime and the relevant
shared-library/consumer checks. Rebuild the server with current embedded assets and
run affected real-browser journeys. Do not suppress new lints to pass qualification,
change sibling projects silently, or treat local tests as LA3/PJ1 hardware evidence.

[October TODO validation](validation/october-todos.md) records this upgrade,
including the separate sibling strict-Clippy findings and temporary-copy Phono
lockfile adjustment. Checkpoint/release names do not establish an MSRV or freeze
pre-release APIs/storage contracts.
