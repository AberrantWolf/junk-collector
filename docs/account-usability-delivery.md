# Account, recovery and Phono connection delivery

**Scope revised 2026-10-09:** Phono Junk is no longer a target. Its remaining PJ1
and Phono-specific GUI/system-root/second-Mac acceptance are retired, not passed.
This ledger preserves implemented shared software, data guarantees and dated
evidence. Relevant web/Trash Amp qualification lives only in
[remaining work](remaining-work.md). Further Phono work is not a target.

Accepted 2026-10-07. This ledger supersedes the remaining software repair scope in
[the usability review](account-connection-usability-review.md). It does not expand
PJ1 into discovery, certificate rotation, email invitations or offline media.

| Increment | Delivered behavior | Acceptance |
|---|---|---|
| Native connections | Address-only enrollment, browser preview and enabled-account approval, secret-authenticated status/cancel, automatic polling/completion, persisted Keychain attempts, recoverable finalization, account-keyed connections/artwork/playback/commands, duplicate selection, explicit reauthorization, local forget confirmation, bounded owner trust import | Automated service/transport tests and actual Keychain round trip; user verified local connection on 2026-10-07; full native GUI and system-root HTTPS deployment qualification remain open |
| Account security/recovery | Six security tasks, browser-only revision/receipt-fenced password change, separate revoke-other-access, recovery-code revocation/fresh browser session, stronger local recovery, bounded stopped account lookup and username-only recovery with configured storage defaults and optional `--state` override | Store concurrency, replay, preservation and stopped-state tests; browser UI journeys recorded in validation |
| Administration/onboarding | Current/default account labels, immediate default login refresh, durable creation followed by independent collection grant/retry, setup-only initialization, local and HTTPS address/DNS guidance | Browser grant failure/retry and default refresh; CLI existing-state/config-independent regression |

## Preservation and storage

Collector private SQLite schema **19** converts the enrollment table in one
transaction so new requests may initially have no account. Existing bound rows,
credential hashes, grants, journals and authority are copied without changing
identity. Stopped account lookup/recovery supports existing private schemas 5–19;
it neither creates missing state nor loads configuration or the archive. Unsupported
private formats fail closed. Local recovery retains enabled/admin/default status
and grants, including when the account is disabled.

Before upgrading, stop the server and copy the complete archive and private-state
root as described in [full backup](../README.md#full-backup). Restore the complete
backup with its matching executable if an upgrade fails; do not downgrade just the
SQLite version marker or replace the private root. Lookup/recovery never rebuilds
accounts from archive contents.

Phono saves credential secrets under `ServerId/UserId` in the existing macOS
Keychain service. Existing server-keyed entries are verified through `/me` and the
account profile, then the new credential and connection settings are committed
before deletion of the old entry. A durable cleanup marker retries deletion after
restart. Offline legacy rows stay visibly unverified. Pending edits never supply
an account identity for conversion.

`connections.json` stores account identity, display name, endpoint, collection,
active pair and legacy cleanup markers. A separate nonsecret
`cancelled-enrollment.json` marker prevents cancelled requests resuming if Keychain
deletion fails; successful Keychain deletion also remains sufficient if that marker
cannot be written. `pending-edits.json` keeps exact existing
server/account/collection command coordinates. The pending enrollment, completion
secret and any issued credential awaiting settings finalization reside only in
Keychain. A lost issuance response requires new browser authorization; a persisted
issued credential resumes local finalization. Closing an attempt stops polling;
an unreachable cancellation expires, while an already issued partial save remains
recoverable. Existing working connections survive unsuccessful enrollment.

Back up Phono settings and the command journal together with the normal Keychain
backup. Without Keychain access, reauthorize the same account; do not discard its
journal or infer its identity from edits. Disconnect preserves access; Forget
removes only the selected local credential/settings. Server revocation is a separate
Account & security → Apps action.

## CLI usability follow-up — 2026-10-07

Recovery uses `recover-account --username NAME`; `accounts` lists sign-in names
and status without exposing internal UUIDs. Both default to configured account
storage. `paths` shows absolute configuration/account-storage/archive locations
and existence, including while serving. `--state PATH` bypasses damaged
configuration; retired `--private-state` and `--user-id` flags have no aliases.
The existing-state locking and revocation guarantees are unchanged.

Validation for this follow-up: configured/default and explicit-state recovery,
path reporting with live storage locks, missing/damaged storage and retired-option
regressions passed. All Collector gates, server rebuild and Chrome 153 full smoke
passed; see the dated validation record above.

## Local HTTPS certificate usability follow-up — 2026-10-07

The reported local enrollment failure was reproduced: HTTPS localhost:9843 is
reachable but uses the setup-generated self-signed certificate, which default
trust rejected. junk-collector.local also failed to resolve. Reading the local
public PEM and explicitly validating HTTPS against it succeeded. Existing
certificates, credentials, grants, settings and the running user server were not
replaced or restarted.

Administration → Server settings → HTTPS certificate downloads only public PEM
through the authenticated pairing read, displays its PEM-byte fingerprint and
guides macOS SSL trust or Phono-only explicit trust. Phono help explains that its
own certificate import does not establish browser trust; same-computer approval
can use the private loopback `/pair` page. Remote approval still requires trusted
HTTPS. Startup/doctor prints a local client HTTPS address separately from
advertised names, along with public file location/fingerprint/help. `paths` lists
the public certificate. No OS trust change or validation bypass is automatic.
The fingerprint remains SHA-256 of PEM bytes, not a certificate viewer’s binary
encoding; no identity format or credential conversion changed.

The real-server certificate browser acceptance passed: signed-in download,
unauthenticated pairing denial, byte equality with generated certificate, SHA-256
equality with trusted startup output, absence of private key, mobile help and
read-failure retry, explicitly validated HTTPS, local approval and one-time
credential issuance. Native GUI/CLI rebuilt and library tests/lint passed.
Remaining full gates and their results are recorded in the dated validation log.

Final follow-up gates passed: complete Collector workspace tests, all-target
clippy, architecture/fmt/diff, web check/build/API contract, server rebuild and
Chrome 153 full smoke. Focused native TLS rejected untrusted self-signed identity
and accepted explicit pairing. The live native transport independently reproduced
UnknownIssuer and accepted the actual local public PEM. Phono affected tests,
lint and both executable builds passed. The dated validation record distinguishes
these checks from pending interactive OS-root/native GUI/second-device acceptance.
