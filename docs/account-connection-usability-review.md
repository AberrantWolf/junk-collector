# Account, recovery and server connection usability review

**Current direction (2026-10-09):** [remaining work](remaining-work.md)
is the only remaining-work queue. Older assignment/order and gap statements below
are historical. Proposed contracts require reassessment under
[the home-lab scope/retention policy](decisions/home-lab-scope.md); their presence
does not require implementing speculative machinery or permanent lookup history.
Phono Junk is no longer a target. Implemented contracts describe current behavior,
not a claim that cleanup or redesign has already happened.

2026-10-07. Source review of Collector and sibling Phono; this is not a completed
native/browser usability qualification. The user requested this reassessment and
removal of retired commands. Existing uncommitted PJ1 work is retained.

## Findings

| Priority | Current behavior and source | User consequence |
|---|---|---|
| P1 | Collector returns `/pair` from enrollment start, but its router registers no `/pair` page and the browser app has no enrollment approval UI (`collector-server/src/lib.rs`, `web/src/app.tsx`). | Phono's “Open Collector to approve this device” link has no working destination. API enrollment tests do not qualify this browser journey. |
| P1 | Phono asks for name, HTTPS address, username, PEM text, fingerprint and trust confirmation together (`phono-junk-gui/src/views/collector.rs::connection_dialog`). | Users must translate exported pairing JSON into several fields; a disabled button gives no field-specific explanation. Trust checking is necessary, but the manual transcription is avoidable. |
| P1 | Local recovery requires an exact private-state path and account UUID (`collector-server/src/main.rs`, `collector-store/src/auth.rs::local_recover`); there is no stopped-server account selector. | A locked-out user cannot discover the required identity through the normal account UI. Recovery must work without opening a damaged archive or starting the server. |
| P2 | Native approval is completed manually; the GUI retains only code and URL, although enrollment supplies an expiry (`collector-client/src/lib.rs`, Phono worker/view). | No visible expiry/countdown, waiting state or guided restart. Closing the dialog does not express cancellation of the pending request. |
| P2 | Phono persists one credential per ServerId and enrollment replaces the saved server for that identity (`phono-junk-lib/src/collector.rs`). | Multiple servers work in the model; multiple accounts on the same server do not. Account switching must be explicit rather than appearing to add another server. |
| P2 | Disconnect retains credentials; Forget removes the local Keychain entry and saved server, with no confirmation or server-side revocation (`CollectorService::disconnect/remove`). | Users cannot tell whether they stopped playback, signed out or revoked device access. Forget can remove the route back to account-scoped uncertain playlist commands, which remain journaled. |
| P2 | Native credentials last 30 days; reconnection retries the stored credential. | Expiry/revocation needs a named “Authorize again” action that retains server trust, rather than asking users to re-enter certificates. |
| P2 | Browser Security provides profile, recovery codes, sessions and credential revocation, but no password-change form; passkey registration is a separate sidebar action (`security.tsx`, `app.tsx`). | Credential management is scattered and ordinary password rotation has no obvious journey. Recovery should not substitute for password change. |
| P2 | Admin account creation and collection grants are separate panels (`administration.tsx`, `collection-access.tsx`). | A newly created member may sign in to an empty library with no explanation. Server administrator is independent of collection access and should be explained at creation. |
| P2 | Single-user sign-in hides the default username. Account rows do not identify the default, though “Make default” is available. | Users cannot readily verify which account is active/default before changing configuration or approving a device. |
| P2 | Local recovery revokes sessions, native credentials, enrollments, codes and passkeys; code recovery revokes sessions and code batches (`collector-app::security::credential_effects`). | Recovery paths have materially different consequences. Local CLI output previously mentioned only sessions/passkeys; browser copy does not explain the whole code batch invalidation. |
| P3 | `setup` and `init` both initialize; `reset-password` was an advertised command that only rejected execution. | More entry points and a stale recovery recommendation complicate onboarding. `init` is still functional, not a retired command. |

## Intended complete journeys

1. **Set up Collector:** one documented `setup` path, reveal exact management and
   client addresses, sign in, save recovery codes, create a collection. Preserve
   scripted stdin setup. Consolidate `init` only with its actual callers updated.
2. **Add server in Phono:** enter the normal HTTPS web address and authorize in
   the browser with the same account. No certificate transcription for system-
   trusted HTTPS. Owner-provided trust is an advanced option only when needed;
   a future pairing-file import can simplify that exceptional path.
   Importing a file alone does not establish trust; never fetch an unauthenticated
   fingerprint and silently trust it. Explain certificate hostname mismatches.
3. **Approve device:** implement an addressable `/pair` page on client listeners,
   preserve the destination across sign-in, prefill the displayed code, show device
   and signed-in account, require recent same-account authentication and explicit
   approval. Use existing enrollment policy and APIs. Show success back in Phono
   with cancellable bounded completion polling, expiry, retry and restart states.
   Browser certificate trust remains a separate prerequisite.
4. **Manage connections:** show server, endpoint, authorized account, collection
   and connection state together. Offer Reconnect, Authorize again and Forget with
   clear consequences. Preserve pending command journals across reconnect; require
   explicit account selection before resolving them. Explain that local Forget
   does not revoke a device on Collector. Keep one account per server explicit
   initially; multiple simultaneous accounts are a separate contract change.
5. **Recover account:** sign-in offers passkey, one-time code, and clear no-code
   guidance. Add a stopped-server account lookup using only existing private state,
   under the same exclusive lock as recovery; select an exact identity and show
   revocation consequences before accepting a new password. Never create a fresh
   state root on a typo, guess an owner, require archive health, or restore admin
   privileges implicitly. After recovery guide new codes/passkeys and native
   reauthorization; preserve grants, enabled status and admin authority.
6. **Manage accounts:** group profile/password/passkeys/recovery/devices in Account
   & security; label active/default identities. Account creation ends with an
   explicit grant step or clear “no collection access yet” result. Keep collection
   roles separate from server administration. Add password change through an
   application command with recent authentication and deliberate revocation policy.

## Delivery order and acceptance

Deliver the first connected journey across layers: trusted pairing-material input
→ real browser approval → native completion → restart/reconnect → revoke/authorize
again. Do not call PJ1 enrollment usable on the strength of the TLS/API tests alone.
Then deliver lockout recovery and account administration as complete journeys.
Discovery, certificate rotation, account invitations and simultaneous same-server
accounts are excluded from this repair scope.

Required evidence: real Phono GUI and browser from a clean temporary profile;
single- and multi-user approval, wrong account/code, expiry, denial, cancellation,
certificate mismatch, offline/stalled endpoint, Keychain failure, duplicate server,
restart and revoked credential. Test forgotten-server pending edits without losing
or replaying them under another account. Confirm browser-trusted management access
on the intended endpoint, rather than bypassing TLS validation. Recovery tests must
cover live-server lock denial, nonexistent state/account, damaged archive, revoked
credentials and preserved roles. Include keyboard navigation and actionable errors.
Keep the second physical Mac and audible playback acceptance open until exercised.

## DRY and best practices

Reuse `Pairing`, `PendingEnrollment`, native credential storage and the existing
application enrollment/security services. Phono's service/worker owns networking,
Keychain and cancellation; the view renders typed states. Collector's browser owns
approval presentation, never credential issuance policy. Introduce one typed
connection state/error model instead of interpreting display text. Avoid a second
trust store, protocol or frontend workflow. No compatibility aliases for removed
commands; retain actual credentials, configuration, journals and local catalog data.

## Changes made with this review

Removed the retired `reset-password` clap variant and dead dispatch. Corrected
`init`'s recovery recommendation; missing account/TLS guidance points to `setup`.
Local recovery now reports all revoked credential categories and the next steps.
Following the user's clarification, Phono now defaults to system-trusted HTTPS
and keeps manual certificate entry behind an owner-provided certificate option.
Collector serves `/pair` with real approval through the existing API and recent
same-account authentication. Normal HTTPS credentials retain only the enrolled
origin and reconnect with system trust; existing pinned credentials keep their
trust. Local recovery bypasses configuration parsing as well as archive opening.
Account lookup, password change, automatic approval completion, invitation/grant
handoff and simultaneous same-server accounts remain unimplemented. The review
findings describe the starting state; these implemented repairs supersede the
corresponding findings. Hardware/system-certificate qualification remains open.

Validation: required Collector gates and full browser smoke passed. Final focused
browser approval (including prefill and wrong-code retry), Phono build/check/lint
and facade regressions passed. Actual native system-root HTTPS and second-Mac
qualification remain open; see [the dated record](validation/phono-integration.md).


## Implemented complete repair — 2026-10-07

The user's subsequent accepted complete plan supersedes this review's remaining
software gaps. The [delivery ledger](account-usability-delivery.md) records native
connection lifecycle/account separation, security/recovery and administration/setup
as implemented increments. [Validation](validation/phono-integration.md) records
passing repository/browser/Keychain evidence and the explicitly unqualified native
GUI/system-root HTTPS and second-physical-Mac matrix. Earlier findings above describe
the reviewed checkpoint, including the now removed `init` command.
