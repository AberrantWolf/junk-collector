# Validation recorded for the foundation

Validated locally on macOS, Rust 1.95.0 and Node 24.3.0.

- `cargo test --workspace`: 20 integration tests pass, covering module registration, archive recovery and tamper detection, upload/inbox source preservation and retries, authorization/redaction, image bounds, search/pagination, credential revocation, exposure routing, HTTP ranges, cache keys, and direct enrolled TLS.
- `cargo clippy --workspace --all-targets -- -D warnings`: passes.
- `cargo fmt --all -- --check`: passes.
- `python3 scripts/check_architecture.py`: passes.
- `npm --prefix web run check` and `npm --prefix web run build`: pass.
- `npm --prefix web run test:smoke`: passes against a temporary real server with an isolated headless Chrome profile. Exercises password sessions, separately loaded management bundles, collection creation, CD inventory, original/thumbnail artwork, text-row browsing/search, WebAuthn registration/passwordless login using a virtual authenticator, and a 390px layout without horizontal overflow.
- Desktop and mobile screenshots inspected. Generated screenshots live in ignored `web/test-results/` and contain synthetic fixture data only.

The direct TLS test starts only a loopback HTTPS server, logs in over the enrolled certificate, retrieves a protected byte range and rejects an unrelated certificate. No public route is configured. This validates the encrypted LAN transport mechanism; mDNS, network handover and native-app platform lifecycle are not implemented or validated yet.

Browser tests accept the test server's self-signed certificate only within their temporary Playwright context. Product client code uses enrolled trust anchors and normal hostname/handshake verification. A virtual authenticator establishes WebAuthn protocol integration, not password-manager-specific certification.

Container execution was not tested because Docker is unavailable in this environment. The 100k/1m benchmark, independent game/CD verification oracles, native audio hardware, Slint/iOS apps and real password-manager testing remain outstanding. See implementation-status.md for the full first-release gap list.
