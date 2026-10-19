# Container installation packaging — 2026-10-19

This follows the [actual Synology drill](synology-deployment-2026-10-19.md).
That test installation is disposable; no test data is required for the permanent
install. The new package is not yet an actual-host or published-release claim.

## Delivered

- `deploy/container/compose.yaml` separates the non-root serving process,
  explicitly invoked offline CLI, and root one-shot directory/config initializer.
  The initializer retains existing configuration and never recursively changes
  archive ownership. Persistent roots are independent of image replacement.
- Linux host networking preserves loopback-only HTTP and explicit private TLS
  management/client listeners. Initial config supplies bundled FLAC/Opus paths,
  exact browser origins and a stable private HTTPS authentication origin.
- The [operator runbook](../../deploy/container/README.md) covers first setup,
  certificate trust, provider contact configuration, versioned-image updates,
  full stopped-server backup and recovery into empty roots. No database migration
  compatibility promise, automatic update service or Internet-ingress claim is added.
- The server accepts SIGTERM as well as SIGINT via the existing stop path.
  The Dockerfile explicitly uses SIGTERM. This fixes the observed idle-stop
  timeout without claiming active-job draining or power-loss safety.
- A release workflow runs the reusable repository checks, builds/smokes the native
  Linux Compose package, builds amd64/arm64 images and packages the Compose files,
  environment example, recovery instructions and checksum. Only a published
  version-matching release pushes to GHCR/attaches the bundle. Manual runs do not
  publish. No workflow was dispatched and no image/release was published here.
- `scripts/check_container.py` exercises initialization twice, setup, real HTTP
  login, both bundled tools, recreation with preserved TLS/login, default SIGTERM
  exit, stopped-server backup validation and fresh-root restore/login with
  preserved TLS. It uses a temporary installation on
  Linux, with bounded readiness/process teardown. It has not run in this session.

## Validation

- Real executable SIGTERM/SIGINT integration test passed on the development Mac:
  both signals return exit 0 and release storage for reopening. Readiness uses
  an actual HTTP response; this is not an assertion about signal-helper internals.
- All three compiled backup/restore CLI integration tests passed.
- `cargo test --workspace --locked` passed, including the new shutdown test,
  HTTP/TLS tests, all backup CLI tests and 232 archive integration tests (three
  existing manual qualifications remain ignored).
- `cargo clippy --workspace --all-targets --locked -- -D warnings` and
  `cargo build --release --locked -p collector-server` passed. Formatting,
  architecture/documentation links and diff whitespace pass.
- Compose config parses with both default and tools profiles. ShellCheck passes
  for the initializer; both workflow files parse as YAML. The smoke driver compiles
  as Python. These checks do not establish container runtime behavior.
- Docker daemon access was denied at the local Unix socket; the revised image,
  initializer, Compose smoke, bundled encoders, registry workflow and multiarch
  builds could not be executed here. Run the manual release workflow first, then
  the actual DSM install and trusted-private-HTTPS acceptance before publishing
  supported platform claims. No permission escalation or deployment was attempted.

The original NAS test remains qualification of the earlier image only. Future
acceptance should record immutable image identity, host filesystem, browser/trust
behavior, active-work interruption and chosen backup failure model separately.

## Operator documentation follow-up

The README and container guide now separate current builds from the old tagged
release, describe backed-up personal alpha use, and recommend external HTTPS.
A generic proxy config example binds HTTP to loopback, uses an explicit browser
origin, leaves public management disabled unless deliberately enabled, and keeps
unused direct TLS on loopback. It is included in the installation bundle.
The guide explains connector network namespaces, per-request upload limits,
cache controls, setup, upgrades and recovery without relying on the test host or
test data. The native operator runbook retains backup/restore commands; historical
owned-corpus execution records moved intact to a separate dated-history file.
Retired Phono-specific setup instructions were removed from the main README.

These are documentation/example/bundle-list changes, not a new ingress
implementation or a passed Cloudflare qualification. TOML/Compose syntax,
architecture/documentation links and whitespace are checked; earlier source-code
test results above remain dated and are not claims of a new runtime deployment.
