# Phono Collector integration — local evidence

2026-10-07, PJ1 delivery. This records local evidence, not physical second-device
acceptance. The integration spans Collector, Phono, junk-libs and the narrow Trash
Amp decoder adaptation. Existing unrelated Trash Amp changes are outside this
record.

## Repository gates

Passed locally on 2026-10-07:

- Collector: architecture/documentation checks, generated OpenAPI/TypeScript drift
  check, workspace tests, all-target clippy with warnings denied, format check,
  web type check/build and real Chrome smoke. Rebuilt the Rust server after assets.
- Phono: workspace tests, GUI build, all-target clippy with warnings denied and
  format check. Actual opt-in Keychain round trip passed separately.
- junk-libs: workspace tests and all-target clippy with all features, format check.
- Trash Amp: amp-audio tests/clippy and workspace check. Retro Junk: workspace tests.

Existing ignored hardware/data-dependent tests remain ignored; none are counted
as successful physical acceptance. Temporary logs are under `/private/tmp/phono-*`.
The disposable Collector was stopped and Phono settings confirm no saved test
server remains. No production credentials or local evidence were removed.

## Automated evidence

- Independent signed-16 stereo FLAC vector: all decoded samples, exact seek,
  frame count and truncation rejection. Shared sequence capture compares split
  sources to a joined source sample-for-sample; mixed 44.1/48 kHz sources preserve
  expected output count. Cancellation and inconsistent duration are rejected.
- Metadata-only song search: literal wildcard characters, Unicode normalization,
  manual artist/title changes, pagination scope, collection denial, unavailable
  songs, reindex and reopen. Search uses recorded source fingerprints and does
  not open/hash original audio.
- Real pinned TLS enrollment and media reader: byte seek/ranges, cancellation,
  cross-origin rejection, wrong certificate, playlist create/idempotent replay,
  rename and revision conflict. A reader opened before device revocation rejects
  its next uncached read; opening another reader rejects revoked credentials.
  This test found and repaired reuse of connection tasks across worker runtimes.
- Client policy rejects credentials returned for a different server, mismatched
  pairing identity and expired authority. Range validator unit cases reject
  inconsistent ranges/length/validator/encoding. A stalled future cancels promptly.
- Phono journals exact submitted command inputs before transport, rejects changed
  inputs/competing edits and restores uncertain commands after restart. Superseded
  reads cancel; corrupt journals fail closed. Actual macOS Keychain save/load/remove
  passed using a fresh disposable server identity.
- Real Chrome smoke passed browser playback, pause/resume, seek, next/previous,
  natural advancement, reload and grant revocation; bulk import/preparation;
  playlist repeat occurrences, reorder/remove/rename/delete/conflict and committed
  response recovery; artwork, keyboard/mobile, roles and WebAuthn.

## Actual macOS app journeys

Used a disposable trusted localhost TLS Collector profile and the rebuilt native
Phono app. Enrolled through Phono's certificate/fingerprint consent UI, approved
using Collector's existing browser-workflow HTTP API, completed enrollment and
restored the saved connection/collection after restart. Trust was initially
unchecked and enrollment disabled. This is not evidence of driving the approval
page in a browser UI.

Visited Songs, Albums and Playlists. A synthetic preserved CUE/WAV disc appeared
as two fallback songs only after recording its structural observation. Initially
unprepared rows remained visible. Prepared two four-second-total FLAC tracks via
Collector's real preparation pipeline, then displayed Play/Enqueue controls and
artwork. No mocked preparation success was substituted.

Created a playlist in Phono, appended the same song twice, renamed it, and observed
separate occurrence controls. A concurrent server rename caused a native remove
conflict; the UI retained the proposal, refreshed entries and required explicit
Reapply. Reapplication removed one occurrence against the new revision.

Played the album and a personal two-track queue through Kira. Observed advancing
position and natural completion with the queue retained. The user confirmed
hearing the tone. Audible gapless boundary quality remains unqualified: hearing
output does not establish absence of a click/pause. The final layout fix splits
transport and seek/volume/repeat controls into separate rows; all controls were
visible in the rebuilt app. Native pause/resume, seek, repeat-one beyond the track
duration, stop and persistent playback across local-library navigation passed.
Revoking the actual disposable native credential during repeated playback stopped
output, cleared the queue and removed collection/edit capabilities with a visible
authentication error. The disposable connection was then removed in Phono.

The user's existing local catalog reported an older schema at startup. It and
existing provider credentials were preserved; Collector browsing operates
independently. The final disposable app bundle uses the existing
PHONO_SKIP_KEYRING test setting only for unrelated local identification providers.
Collector credential access continues through the actual Keychain adapter.

## Open acceptance

- Focused real-output listening across boundaries; native repeat-all/shuffle,
  next/previous and narrow-window keyboard qualification.
- Full native multi-server switching; Viewer capability changes and collection
  grant revocation while playing through the native app.
- Native committed-response loss plus restart recovery end-to-end (journal restart
  and server/browser receipt recovery have separate automated evidence).
- Malformed/truncated range bodies, validator changes and sustained network stalls
  through a real reader/playback fixture beyond validator/cancellation unit cases.
- Second physical Mac against a trusted LAN endpoint: prepared playback, Curator
  edits, denied Viewer edits, grant/device revocation and applicable LA3 criteria.

No second physical Mac was supplied. PJ1.4 and physical LA3 acceptance stay open.
No offline downloads, preparation in Phono, synchronized listening, discovery,
private-playlist model or unrelated Trash Amp native-client completion is claimed.

## Account/connection usability repair — 2026-10-07

User direction: ordinary web account access should carry into Phono, with extra
trust steps only for a concrete certificate/security need. The source review and
remaining account/recovery work are in
[the usability review](../account-connection-usability-review.md).

Implemented system-root HTTPS enrollment/reconnect, origin-only credential use
for that trust mode, optional native certificate fields, optional server name,
real client-surface `/pair` approval and approval-code prefill. Recent same-account
authentication and one-time native issuance remain application policy. Removed
the retired `reset-password` command and repaired CLI recovery/setup guidance;
local recovery no longer parses unrelated configuration.

Passed: architecture/link checks; cargo fmt check; cargo test --workspace;
cargo clippy --workspace --all-targets -- -D warnings; web type check/build;
server rebuild; public exposure matrix including `/pair`; client policy test for
saved system-HTTPS credentials with no local certificate and forbidden origin,
port, HTTP and URL-credential requests. The full Chrome 153.0.8010.37 smoke suite
passed, including browser approval, real FLAC playback, bulk intake and lists.
Initial sandboxed Chrome launch failed; rerun outside the filesystem sandbox
with a temporary profile passed. No product TLS bypass was introduced.

The focused final `node web/tests/pair-device.mjs` journey passed signed-out
approval destination, code prefill, account display, invalid-code retry without
sign-out, switching accounts while retaining destination, approval, native `/me`
and rejection of repeated completion. Its localhost HTTP profile requires no
browser certificate bypass and qualifies browser UI/issuance, not native HTTPS.
The approval success screenshot was inspected. Phono GUI build/check and affected
lib/GUI clippy passed; focused Collector facade tests passed (3 passed, 1 Keychain
hardware test explicitly ignored).

Not claimed: actual native GUI enrollment against a system-trusted remote HTTPS
endpoint, new Keychain prompt qualification or second physical Mac. Existing
native TLS fixture tests cover pinned transport, not system-root deployment.
Remaining account lookup/password-change and connection lifecycle work is still
planned. PJ1/LA3 hardware acceptance remains open.

## Complete account/recovery/connection repair — 2026-10-07

This entry supersedes the preceding entry's planned account lookup/password-change
and connection lifecycle gaps. The [accepted child ledger](../account-usability-delivery.md)
records the three implemented increments and their storage/backup path.

Implemented native enrollment without username, explicit browser preview/approval,
secret-authenticated typed status/cancel, automatic two-second polling (ten seconds
on temporary network failure), terminal stop, independent cancellation fencing,
Keychain attempts and recoverable issued-credential finalization. Saved connections,
artwork, playback sources and durable edits use server/account identity. Duplicate
approval selects saved access; explicit reauthorization replaces it. Legacy entries
verify `/me` before conversion and retain an existing working account credential;
cleanup is restartable. Disconnect/Forget and bounded owner PEM/pairing import are
separate from server revocation and system-root HTTPS.

Implemented six security tasks, password confirmation/revision/receipt command,
separate other-session/app revocation and the accepted stronger code-recovery policy.
New password-change receipts use deterministic per-command Argon2 input binding,
not a fast password fingerprint. Code recovery guides replacement-code saving;
local recovery additionally revokes passkeys. Stopped account lookup/recovery uses
an existing-state opener/exclusive lock, exact username or UUID, supported private
schemas 5–19, and no configuration/archive access. Schema 19 transactionally copies
actual account-bound enrollment rows before enabling unbound requests.

Implemented durable account creation followed by independently retryable permitted
collection grants; current/default labels refresh immediately. `setup` is the sole
initialization command. Removed the unused legacy Store password-reset helper too.
Historical validation records remain intact. Existing tests expecting private schema
18 now expect 19 because the actual enrollment format changed. The historical
Wave 2 test now distinguishes generating codes (apps retained) from consuming a
code (apps revoked), matching the user's explicit recovery-policy decision.

Observed evidence:

- Actual macOS Keychain credential save/load/remove passed using fresh test-only
  server/account IDs, without replacing user entries.
- Phono service tests passed same-server two-account storage, Keychain save failure,
  credential-written/settings-failed restart recovery, lost issuance without false
  connection success, forgetting only one account, and retained uncertain edits.
  Generation-cancelled reads settle promptly; unverified legacy rows stay inactive.
  Cancellation survives Keychain deletion failure across restart using a nonsecret
  marker; successful deletion remains sufficient when the marker cannot be written.
- Store tests passed concurrent unbound approval with one account winner, secret
  gating before cancellation/expiry, one-time issuance, password receipt replay,
  password access retention, selective other-access revocation, code recovery/fresh
  session/passkey retention, stronger local recovery and preserved account authority.
  The actual schema-18 bound-enrollment conversion preserves credentials and rows.
- CLI regression passed live private locking, nonexistent roots, retired `init`
  rejection, mutually exclusive selectors, damaged configuration/archive and
  existing-state username recovery. Unsupported private state fails closed.
- `web/tests/pair-device.mjs` passed both single-user and multi-user browser sign-in,
  preview, bad-code correction, switching to a different approving account, native
  `/me` without a browser cookie, and repeated-completion rejection.
- `web/tests/account-usability.mjs` passed password change retaining native access,
  account creation followed by a failed grant and successful retry, refreshed default
  login/badges, two separate account approvals, scoped app revocation, recovery login,
  no-code guidance and the replacement-code panel. Its native authority checks use
  bearer-only requests so browser cookies cannot hide a revoked app.
- Full browser smoke passed session/exposure/inventory/role/artwork/WebAuthn/mobile
  journeys, real FLAC playback, bulk intake and collection-list recovery. Security
  task navigation wraps so all six tasks remain visible at narrow modal widths.

Final check results: Collector architecture/link/fixture checks, Rust formatting,
full workspace tests, workspace all-target clippy with warnings denied, web
TypeScript/build and generated API contract check passed. The embedded server was
rebuilt after asset changes. Both single/multi-user pairing and the account browser
journey passed; the full Chrome smoke passed. A final 390px account-panel check
passed all six task buttons without horizontal overflow, keyboard Tab navigation,
password mismatch feedback and post-recovery replacement-code navigation. Its
screenshot was inspected.

Phono full workspace tests passed, followed by final affected-library tests
(41 passed, one hardware test ignored in that run), lib/GUI all-target clippy with
warnings denied, format/diff checks, and both GUI/CLI builds. The separately invoked
actual macOS Keychain acceptance test passed. Earlier failed assertions were fixed:
private-schema expectations advance to 19; native credential checks use native
authentication, not browser sessions; the old code-consumption retention assertion
was changed to the explicitly accepted revocation policy. Tests now distinguish
browser cookies from bearer-only native authority.

Qualification still open: the CUA native-app inventory reported the Mac locked and
unable to unlock automatically. A user unlock was requested; no native GUI result
is claimed for this increment. Actual URL-only native browser opening/completion,
multiple-server/account switching during audible playback, the full GUI failure
matrix and a system-root-trusted HTTPS deployment remain unqualified here. The
second physical Mac remains open as requested. Localhost HTTP browser tests and
pinned TLS fixtures do not establish that system-root deployment evidence.

## 2026-10-07 — Username recovery and discoverable storage paths

Recovery CLI now requires only the sign-in `--username`. `accounts` prints names
and enabled/admin/default status; both commands use configured account storage by
default and accept `--state PATH` for damaged-configuration recovery. `paths`
reports absolute configuration, account-storage and archive locations with
existence status, without opening storage or taking its lock. Removed
`--private-state` and `--user-id` flags have no compatibility aliases. Browser
recovery guidance and current documentation use the simplified commands.

Passed: stopped-account CLI regression (configured recovery, damaged configuration
override, nonexistent storage, live lock rejection, path reporting during a live
lock, damaged SQLite rejection, removed flags); architecture checks; fmt;
workspace tests; all-target workspace clippy with warnings denied; web check/build;
server rebuild after embedded assets; diff whitespace checks. Manual executable
help and `paths` output confirmed the new interface. Chrome 153 full smoke passed,
including playback, bulk library, collection lists, WebAuthn and mobile journeys.
Chrome required an unsandboxed retry after the sandboxed launch aborted.
Existing native GUI/system-trusted HTTPS and second-Mac acceptance remain open.

## 2026-10-07 — Local HTTPS trust and web certificate download

The live same-computer server was reachable at HTTPS localhost:9843. A read-only
probe using `collector-client::system_https_client` (the native transport) failed
with an invalid peer certificate / UnknownIssuer. Using the actual local public
PEM with `Pairing::lan_client` returned 200 OK; curl with explicit `--cacert` also
validated the listener. junk-collector.local did not resolve. The probes issued
no credentials and did not alter OS trust or the running user profile; temporary
example sources were removed.

Added Administration → Server settings → HTTPS certificate: authenticated public
PEM download, existing PEM-file fingerprint, local trusted-channel/macOS SSL and
Phono-only import/approval guidance. Its Blob lifetime follows the panel, and read
errors remain retryable. Startup/doctor reports the local HTTPS address separately
from advertised DNS names, public file/fingerprint and instructions. `paths`
includes the public certificate. Phono's certificate help and URL example now
match; transport connection/timeout errors are actionable using typed reqwest
failure predicates, without changing security policy or inferring it from text.

Passed: `test:certificate` real-server Chrome acceptance (anonymous pairing denial,
PEM/file equality, fingerprint/startup equality, no private key, actual download,
mobile help/read retry, explicitly certificate-validated HTTPS, private loopback
browser approval and one-time issuance); focused LAN/native TLS test (normal HTTPS
rejects untrusted self-signed identity, explicit pairing validates it) and local
account CLI tests; architecture/fmt; complete workspace tests; all-target clippy
with warnings denied; web TypeScript/build and API contract check; server rebuild;
full Chrome 153 browser smoke including actual playback, imports, metadata, bulk
preparation, lists, WebAuthn and mobile. Phono library tests passed (41, one existing
ignored), affected library/GUI clippy passed, GUI/CLI rebuilt and fmt/diff passed.

The certificate help was visually inspected at 390px:
`/tmp/collector-certificate-help.png`. macOS trust steps follow
[Apple's official instructions](https://support.apple.com/guide/keychain-access/change-the-trust-settings-of-a-certificate-kyca11871/mac).
No certificate replacement or OS trust installation was performed. The new native
help was built but not driven through the native GUI in this follow-up; interactive
system-root enrollment/Keychain finalization and second-physical-Mac acceptance
remain open in the existing delivery ledger.

User verification — 2026-10-07: the user confirmed that local Phono-to-Collector
connection works and requested committing both projects. This establishes local
connection success; the trust mode and full native acceptance matrix were not
specified. Remaining failure/account-separation cases and second-physical-Mac
acceptance stay open.
