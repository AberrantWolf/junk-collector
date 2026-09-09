# Junk Collector development

Read `docs/architecture.md` before adding or changing a subsystem. `docs/implementation-status.md` distinguishes implemented behavior from planned capabilities; keep it current.

- This is architecture-first. If a module has no natural place, revise the boundaries before implementing it.
- Domain/core code has no HTTP, filesystem, database, GUI or device dependencies. Application services own policy; adapters own mechanisms.
- Never describe integrity hashing, structural inspection, provisional metadata or MusicBrainz identity as catalog verification.
- Preserve raw masters and provider observations. Manual metadata authority survives refreshes. Unknown module data round-trips without loss.
- Public/private access is listener/router composition, never a guess from source IP or forwarded headers. Both pages and backing APIs must honor exposure settings.
- Artwork belongs beside game/CD text rows, including search and offline browsing.
- Share format/audio algorithms through narrow `junk-libs` crates. Do not import old app orchestration. Extract shared code separately and check affected sibling projects.
- Document incomplete capabilities honestly; never substitute a successful-looking mock for a missing implementation.

Checks: `python3 scripts/check_architecture.py`, `cargo fmt --all -- --check`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `npm --prefix web run check`, `npm --prefix web run build`. Rebuild the Rust server after changing embedded web assets. `npm --prefix web run test:smoke` uses a temporary local server/profile and a virtual WebAuthn authenticator; configure `CHROME_PATH` when needed.
