# Client PEM parser maintenance cleanup — 2026-10-25

Authorized scope: replace `rustls-pemfile` with `rustls-pki-types::PemObject` and
run affected checks. Utoipa is explicitly deferred. The earlier dependency,
XML and CHD validation records retain their dated results; this record supersedes
their remaining PEM-wrapper maintenance warning.

## Change and trust contract

`collector-client` directly depends on `rustls-pki-types = "1.9"`, whose
`CertificateDer::pem_slice_iter` implements `PemObject`. Cargo retains the already
locked 1.15.1 version and removes `rustls-pemfile` 2.2.0. No other package version
changes in this cleanup. `rcgen` 0.13 is added only as a client test dependency;
the same 0.13.2 package was already present for server tests.

The private `pinned_certificate` helper returns trust only when PEM parsing yields
exactly one certificate and its DER SHA-256 equals the advertised local playback
pin. Empty/key-only data, parser errors, extra certificates and pin mismatches
return no route. Valid non-certificate PEM sections keep their former ignored
semantics. The request client receives `reqwest::Certificate::from_der` using
the exact bytes checked, avoiding a second PEM parse. This preserves local
playback's DER pin coordinate; saved control/pairing certificate handling remains
unchanged.

The 128 KiB descriptor cap, maximum eight endpoints, private-address and HTTPS
checks, endpoint expiry, disabled proxy/system roots/redirects on local routes,
and enrolled ServerId check before bearer credential use are unchanged. Remote
system-HTTPS trust, application policy, credential persistence, archive data and
public API types are unchanged. No Utoipa upgrade, shared-library change, release
or deployment is included.

## Validation

Client tests passed, including three new security regressions generated from real
self-signed test certificates:

- Exact DER hash succeeds; a wrong pin or PEM-text hash does not grant trust.
- Empty/plain/key-only input, duplicate certificates, malformed base64 and
  malformed/truncated trailing certificate sections do not grant trust.
- A valid unrelated PEM key section does not change the one-certificate pin.

Passed on this macOS checkout:

- `cargo test -p collector-client --offline`: client unit/policy/doc tests,
  including the three new certificate regressions.
- `cargo test --workspace --locked`: complete workspace and doc tests, including
  `paired_lan_authentication_and_ranged_download_work_without_a_public_route` in
  `collector-server/tests/lan_tls.rs`. That real TLS test exercises the changed
  native media route with enrollment, ranged reads, seeking, cancellation and
  revocation. Existing annotated manual/hardware/scale cases remain ignored.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`.
- `python3 scripts/check_architecture.py`, `cargo fmt --all -- --check` and
  `git diff --check`.
- `npm --prefix web run check` and `npm --prefix web run build`, followed by
  `cargo build --locked -p collector-server` for the served embedded assets.
- `npm --prefix web run test:smoke`: Chrome 156.0.8078.47 with a temporary
  server/profile and virtual WebAuthn authenticator; provider work, real playback,
  bulk import and collection-list journeys pass.
- `npm --prefix web run test:local-playback`: actual browser pinned QUIC playback,
  forward/back seek, public media refusal, wrong-pin rejection, logout/source
  release and unsupported-transport refusal pass. This checks the browser journey;
  the native LAN integration above exercises the Rust PEM helper.
- Trash Amp `cargo test -p amp-collector` and all-targets Clippy with warnings
  denied, using a refreshed isolated snapshot under
  `/private/tmp/junk-quickxml-siblings/trash-amp`. Command-line patches in
  `/private/tmp/pem-sibling-patches.toml` bind the Collector crates to this local
  workspace and retain the consistent shared-library set. Its four real-server
  integration cases and unit/doc tests pass; the two existing explicitly ignored
  cases remain ignored. The refreshed snapshot includes current appearance
  changes; primary sibling source and lockfiles are retained. No full desktop/iOS
  build or hardware qualification is implied.

No shared-library source was changed in this cleanup. The previous published
security pins remain intact. These local checks do not imply a new Linux/container
release, NAS update or physical-device qualification.

## Audit

`cargo audit --db /private/tmp/junk-collector-rustsec-advisory-db` refreshed the
RustSec database on 2026-10-25 (1,352 advisories), refreshed the crates.io index
and checked 413 locked dependencies including yanked-package status. It exits
successfully with zero vulnerability advisories and only RUSTSEC-2024-0436
(`paste` 1.0.15, unmaintained). The `rustls-pemfile` maintenance warning is gone;
no ignore was added. Its remaining Utoipa follow-up belongs to the consolidated
[remaining-work queue](../remaining-work.md).
