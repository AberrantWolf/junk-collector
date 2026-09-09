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

## A01 and U01 validation — 2026-09-09

- `python3 scripts/check_architecture.py`, `cargo fmt --all -- --check`, `cargo test --workspace`, and `cargo clippy --workspace --all-targets -- -D warnings` pass. The workspace still has 20 integration tests; the HTTP test now also fixes the `unauthenticated` and `forbidden` error-code contract used by the WebUI.
- `npm --prefix web run check` and `npm --prefix web run build` pass, followed by a successful `cargo build -p collector-server` with the rebuilt embedded assets.
- `npm --prefix web run test:smoke` passes. It covers collection creation from the empty state, Add versus Save behavior, structured viewer denial with retained input, revoked-session return to sign-in, WebAuthn, artwork/search and desktop/mobile layout.
- `LOCAL_SETUP=1 npm --prefix web run test:smoke` passes. It additionally exercises a rejected `127.0.0.1` origin, repaired `localhost` origin, repeated setup without replacing the TLS identity or account password, `doctor`, private management, client-only guidance, and absence of the public administration bundle.
- Both browser runs use temporary configurations, accounts, archives, state directories and synthetic artwork. No personal server state or archive was opened.

Container execution was not tested because Docker is unavailable in this environment. The 100k/1m benchmark, independent game/CD verification oracles, native audio hardware, Slint/iOS apps and real password-manager testing remain outstanding. See implementation-status.md for the full first-release gap list.

## Remaining Wave 1 foundation validation — 2026-09-09

A02/A03 executable coverage uses temporary archives/private databases and the
committed synthetic schema-1 fixture under
`crates/collector-store/tests/fixtures/schema-1`. It never opens the ignored local
`.collector` installation.

- `python3 scripts/check_architecture.py`: passes.
- `cargo fmt --all -- --check`: passes.
- `cargo test --workspace`: passes. This includes 4 core contract tests, 2 module
  contract tests, 4 HTTP tests, 18 archive/store tests, 3 client-policy tests, and
  the direct TLS test, plus crate/doc-test targets with no tests.
- `cargo clippy --workspace --all-targets -- -D warnings`: passes.
- `npm --prefix web run check` and `npm --prefix web run build`: pass.
- `cargo build -p collector-server`: passes after rebuilding committed embedded web
  assets.
- `npm --prefix web run test:smoke`: passes against a temporary installation,
  including generic unavailable-module disclosure and the existing WebAuthn/UI
  paths.
- `LOCAL_SETUP=1 npm --prefix web run test:smoke`: passes against a temporary
  installation, including repair preservation, exact-origin guidance and
  client-only bundle absence.

The first two normal smoke attempts were run in the filesystem sandbox and Chrome
aborted at launch with `SIGABRT`, before a page or application assertion ran. The
same normal and local commands passed when headless Chrome was allowed to launch
outside that sandbox. This was an execution-environment failure, not recorded as an
application pass on either failed attempt.

- Core tests cover canonical/invalid language, currency, coordinates and digests;
  exact raw-JSON lexical round trips; duplicate two-disc specimens; unattached
  Captures; invalid physical carriers; and representation cycles.
- Archive tests copy the schema-1 fixture to a temporary directory, verify original
  revision bytes remain unchanged, exercise initial migration, resume after each of
  the two durable revision checkpoints, completed replay, deterministic Component
  conversion, provisional Work/Edition records, changed-Capture revisioning,
  uppercase-digest normalization with the original revision retained as provenance,
  legacy payload download, and lossless module/provider observations. Existing
  archive/tamper/upload/inbox/asset/auth tests continue to pass.
- Private-state tests create a synthetic version-0 database and confirm the former
  `ORDER BY rowid LIMIT 1` account is pinned once, survives restart, and is not
  changed by later account creation.
- Module tests distinguish unknown/unavailable capability, unsupported schema,
  invalid/destructive migration and failed operation; the schema-2 Stamps module
  additively migrates schema 1, reads through a fake opaque host reader, submits a
  fake idempotent job, and returns presentation fields. HTTP coverage confirms an
  unavailable module's lexical payload round-trips and its generic presentation
  explicitly says it is not executable.
- D01/S01 are documentation gates. Their link/coverage/fixture-manifest and
  architecture checks are included in the repository architecture check and the
  fixture migration test. No sibling checkout was modified or represented as
  runtime-qualified by these gates.

### Reviewer discrepancy ledger

- **Resolved:** the backlog header no longer says every ticket is unstarted; each
  completed Wave 1 ticket now has an individual status.
- **Resolved:** the repository now has a committed, synthetic baseline-v1 migration
  fixture. The ignored `.collector` tree contains real local data and was not read,
  copied, migrated, or used as an oracle.
- **Retained and assigned:** A02 necessarily spans several archive records while P01
  owns the eventual general transaction/evidence protocol. A02 therefore uses only
  a migration-specific durable checkpoint and retains accumulated evidence plus
  explicit legacy revision/payload locators. P01 must replace that transitional
  envelope with append-only evidence and general recovery records.
- **Retained and assigned to P01/A04:** typed Capture revisions/projections are
  canonical in schema 2, but the persisted `Item` aggregate still carries a capture
  compatibility copy so the current pre-P01 upload publication can recover across
  its item/capture commit gap. Application reads hydrate that view from canonical
  Capture rows. The original implementation let compatibility snapshots overwrite
  newer canonical rows; the review follow-up below corrects this and adds regressions. P01's durable multi-record protocol must remove the persisted copy;
  A04 owns the final transport DTO separation.
- **Resolved at the required envelope boundary:** module payloads and provider
  observations no longer use `serde_json::Value`; `RawJson` retains lexical JSON,
  including key order, number spelling, and nulls. Other deliberately structured
  fields such as manual override maps and evidence details still use semantic JSON
  and are not claimed to preserve lexical spelling.
- **Resolved:** future known module schemas no longer validate successfully. They
  are preserved and return `unsupported_schema`; unavailable modules remain a
  separate non-executable state.
- **Bounded ownership:** generic module presentation crosses the A03/A04 seam only
  as an additive read model calculated after redaction. It contains safe values and
  limitations, not HTML, executable URLs, command authorization, or production
  operation dispatch. A04 still owns command DTOs, authorization, and idempotency.
- **Open, assigned to S02/S03:** no recent-authentication proof, recovery-code
  lifecycle, last-administrator/last-owner enforcement, or paired native enrollment
  exists yet.
- **Open, assigned to S02/R03:** passkey counters still have a lost-update race, and
  password limiting is global/process-local while challenge creation lacks the
  reviewed per-account/per-peer buckets.
- **Open, assigned to S03:** native bearer issuance is still selected by the
  caller-supplied `x-collector-client` header and is not a paired-device credential.
- **Open, assigned to D02-D05:** junk-libs and Retro Junk contain divergent CUE/CHD
  implementations; Phono Junk pins an older junk-libs revision; Trash Amp consumes
  a sibling path and has unrelated dirty changes. All remain user-owned.
- **Open, assigned to D02/D04:** Retro Junk's MIT file retains placeholder
  attribution. Its Track-1/largest-track hash helpers are identification only and
  must not be promoted to complete verification.
- **Open, assigned to D04:** Retro Junk's NES DAT normalization skips the 16-byte
  iNES header but does not separately skip a trainer advertised in that header;
  trainer-bearing images remain unqualified.
- **Open, assigned to D02/D05:** Phono documentation still describes offset work as
  future even though offset-search code exists. Qualification must cite the code and
  independent vectors, then correct the documentation in the owning checkout.
- **Unqualified, assigned to D03/D05/R03:** real redumper/audio/hardware tests are
  ignored or require environment-supplied owned dumps/devices and were not run.
  Missing private dumps, `chdman`, hardware, or network remain explicit skips and
  do not expand the supported operation matrix.

## Wave 1 review follow-up validation — 2026-09-09

The independent review reproduced six failures not covered by the original suite.
Those probes are now permanent tests in the owning archive/module test files and
pass after the fixes. Additional tests cover legacy reassociation and downloads,
restart/reindex equivalence, stale compatibility saves, private file-locator edits,
partial typed migration output, committed-checkpoint tampering, cross-collection
nested identity reuse, component projection removal, explicit schema transforms,
atomic capability registration and BCP 47 private-use/extension handling.

- `cargo test --workspace --locked`: 45 tests pass (5 core, 5 module, 27 store,
  4 HTTP, 3 client policy, 1 direct TLS).
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: passes.
- `cargo fmt --all -- --check`: passes.
- `python3 scripts/check_architecture.py`: passes. It checks dependency boundaries,
  document links and fixture hashes. Prose keyword checks were removed: design
  completeness requires review and cannot be established by matching phrases.
- `npm --prefix web run check` and `npm --prefix web run build`: pass.
- The server is rebuilt with the embedded web assets and current Rust changes.
- Normal HTTPS/WebAuthn and `LOCAL_SETUP=1` browser smoke flows both pass with
  isolated Chrome profiles and temporary servers, including inventory/artwork,
  unavailable-module presentation, role denial, session expiry and setup repair.

Tests use only temporary state and synthetic fixtures. No local user archive,
credentials or configuration was opened or migrated. General transaction recovery,
full-scale performance and later-wave security/client capabilities remain explicitly
unimplemented or unqualified as described in implementation-status.md.
