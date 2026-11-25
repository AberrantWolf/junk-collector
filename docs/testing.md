# Testing and releases

## Checks

CI (`.github/workflows/check.yml`) runs these on every push to `main` and every pull
request. Run the relevant ones locally before pushing:

```sh
python3 scripts/check_architecture.py                       # crate layering, doc links, fixture digests
python3 -B -m unittest discover -s scripts -p 'test_*.py'   # helper script tests
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
npm --prefix web ci
npm --prefix web run check                                  # TypeScript
npm --prefix web run test:models                            # screen logic in plain Node, no browser
npm --prefix web run build && git diff --exit-code -- web/dist
```

The server embeds `web/dist`, so commit the rebuilt assets with any web change and
rebuild the server binary afterwards.

The music preparation tests call real command-line encoders: `opusenc` (from the
`opus-tools` package), `flac` and `ffmpeg`. Install them once:

```sh
brew install opus-tools flac ffmpeg                  # macOS
sudo apt-get install -y opus-tools flac ffmpeg       # Debian or Ubuntu
```

Some Rust tests use real encoders when
`COLLECTOR_TEST_FLAC=/usr/bin/flac` and `COLLECTOR_TEST_FFMPEG=1` are set.

Documentation-only changes need only the architecture check (it validates relative
links in `README.md`, `AGENTS.md` and `docs/`).

## Browser tests

Install the Playwright browser once per dependency update (add `--with-deps` on Linux
CI). PDF, media and the broad integration journey use real Google Chrome, so install it
too where you run those.

```sh
npm --prefix web exec -- playwright install chromium chrome
cargo build --locked -p collector-server -p collector-media

npm --prefix web run test:smoke          # short end-to-end check against a real server
npm --prefix web test -- browse          # one or more named journeys
npm --prefix web test                    # every journey, run serially (what CI runs)
npm --prefix web run test:scale          # optional 3,000-entry workload
node web/tests/harness-check.mjs         # harness process/teardown checks, no browser
```

Journey names are listed in `web/tests/journeys.json`. When you change a screen, run its
journey and look at the result on desktop and mobile widths; set
`COLLECTOR_SCREENSHOTS=/absolute/dir` to save smoke-test screenshots. Do not add tests
that only assert copy text.

Useful environment variables:

| Variable | Effect |
|---|---|
| `COLLECTOR_SERVER_BINARY`, `COLLECTOR_MEDIA_BINARY` | Executables under test (default `target/debug/…`). |
| `COLLECTOR_TEST_BROWSER=chrome` | Use real Chrome for every journey. |
| `CHROME_PATH` | Chrome executable location. |
| `COLLECTOR_OPUS`, `COLLECTOR_FLAC` | Encoders for the preparation journeys. |
| `COLLECTOR_TEST_ORIGIN`, `COLLECTOR_TEST_PASSWORD` | Run the smoke test against an existing disposable server. Never point it at a real installation; it modifies data. |

Each journey starts its own temporary server and data directory. The runner continues
after a failure and exits non-zero if any journey failed. Some offline provider
fixtures rely on debug-only hooks that release builds never enable.

On macOS, browser tests refuse to start inside the Codex `seatbelt` sandbox (Chrome
aborts there); run them from a normal terminal.

## CD capture helper

```sh
cargo test --locked -p collector-cd-identity
cargo build --release --locked -p collector-cd-identity
python3 -B scripts/test_rip_cd.py
COLLECTOR_CD_IDENTITY="$PWD/target/release/collector-cd-identity" python3 -B scripts/test_cd_naming.py
```

These use fake redumper and HTTP inputs; they never open a drive or contact MusicBrainz.

## Disk usage

Debug and test builds use line-table debug info and no incremental compilation to keep
`target/` small. Set `CARGO_PROFILE_DEV_DEBUG=2` (or `CARGO_PROFILE_TEST_DEBUG=2`) when
you need full debugger symbols. To reclaim space:

```sh
cargo clean --profile dev
```

This clears `target/debug` and keeps `target/release`.

## Releases

Publishing a GitHub release runs `.github/workflows/container-release.yml`:

1. The full check workflow above runs first.
2. Images are built natively on amd64 and arm64 runners and each is checked with
   `scripts/check_container.py IMAGE --smoke` (Compose setup, health, sign-in, mounts,
   shutdown) and `scripts/check_media_container.py`.
3. Those per-architecture images are pushed by digest and combined into
   `ghcr.io/aberrantwolf/junk-collector:<version>`, also tagged `preview` for a
   prerelease or `latest` for a stable release. The pushed images are the ones that
   passed the smoke checks; nothing is rebuilt after testing.
4. `junk-collector-container.tar.gz` (deploy/container, deploy/proton-bridge and the
   Proton/update helper scripts) is attached to the release, with `.env.example`
   pinning `COLLECTOR_IMAGE` to the version tag.

To release: bump the workspace version, make sure `main` passes CI, tag `vVERSION`,
and publish the GitHub release (mark it as a prerelease for preview builds).

### Rust toolchain

`rust-toolchain.toml` pins the toolchain used for development and releases; the
Dockerfile's `rust:` builder image must use the same version. To move to a newer
stable Rust, update both together and fix any new Clippy lints rather than silencing
them. The pin is the tested version, not a promised minimum.
