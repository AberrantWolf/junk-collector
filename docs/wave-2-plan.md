# Junk Collector Wave 2 execution plan

Status: approved for execution on 2026-09-09. Child tickets remain planned until their own acceptance criteria and review checkpoints pass. Wave 1 is commit `e4cb6c5`.

## Readiness assessment

The Wave 1 baseline passed `cargo test --workspace --locked` (45 tests), `python3 scripts/check_architecture.py`, and `npm --prefix web run check`. Those checks establish a stable starting point, not Wave 2 completeness.

| Contract | Ready foundation | Wave 2 discrepancy |
|---|---|---|
| Typed graph | Work, Edition, Item, Component, Carrier, standalone Capture identities and graph validation exist. | Persisted Item still duplicates Captures and Assets as compatibility snapshots. P01 must remove them from canonical item persistence without letting snapshots overwrite standalone Captures. |
| Module boundary | Namespaced capabilities, schema migration, raw JSON preservation, host readers/jobs, and generic presentation exist. | Production dispatch, authorization, command DTOs, and idempotency are absent. Unknown-module raw values cross presentation without a role-specific DTO firewall. |
| Application boundary | CollectionService owns some collection authorization and redaction. | Store still accepts Principal and owns policy for uploads, files, assets, activity, jobs, accounts, and grants. HTTP handlers directly enforce administrator/import/job policy. |
| Commands | Item, Work, Edition, Capture, upload, and cover operations use some expected revisions. | There is no persisted command identity, account/grant revision, uniform conflict taxonomy, or concurrent replay protection. Frontend writes contain broadly unrestricted stored records. |
| Archive publication | Writer lock, staged revision directories, fsync, atomic rename, startup reindex, and migration checkpoints exist. | Payloads and assets can publish before their referencing revision; entity revisions, projections, jobs, and grants are not coordinated by a durable journal. Collection creation can strand an ungranted collection. |
| Evidence/assets | Evidence and artwork survive the Wave 1 fixture and reindex. | Evidence is copied into every item revision, duplicate IDs use `INSERT OR IGNORE`, Evidence requires an Item and one Representation, and Assets lack collection identity/revisions. |
| Private state | Private SQLite v2 preserves users, password hashes, grants, sessions, passkeys, settings, activity, and jobs. | Sessions and devices are conflated; enabled/profile revision, recovery lifecycle, recent-auth proof, security audit, native credential, and last-admin/owner protection are absent. |
| Browser/native security | Configured origins are compared when supplied; cookies are Secure/HttpOnly/SameSite; passkey ceremonies are bounded and single-use. | Missing Origin is accepted without a cookie, passkey counters use a lost-update-prone read/write, rate limiting is global only, and `x-collector-client: native` can select bearer issuance without enrollment. |
| Regression coverage | Canonical Capture precedence, migration recovery/tamper refusal, identity ownership, projection cleanup, capability dispatch, and language parsing have permanent tests. | There are no process-kill phase tests or coverage for evidence-ID conflicts, grant-removal races, command-key mismatch/concurrency, Origin variants, recovery replay, freshness boundaries, passkey CAS races, or enrollment races. |

Small prerequisite repairs are in scope where they directly resolve a Wave 2 contract: role-safe unknown-module presentation, standalone Evidence for unattached Captures, session/device separation, the narrow enrollment bootstrap exception to the Origin rule, and incremental SQLite migrations. Durable general jobs, discovery, TLS rotation, backup/export, physical-inventory UI, and general sensitive-asset UI remain outside Wave 2.

## Architectural decisions

### Ownership and interfaces

- `collector-core` retains portable identities and records. Add typed SessionId, PasskeyId, EnrollmentId, and NativeCredentialId, but no SQLite, HTTP, cookie, filesystem, GUI, or device mechanisms.
- `collector-app` is the sole policy boundary. Split Catalog into narrow collection/inventory, media, import, access, account, authentication, idempotency, and publication ports. Store adapters receive validated effect requests and audit identities, never Principal for authorization decisions.
- `collector-store` owns SQLite, archive paths, hashing/copying, journals, credential hashing, and atomic compare-and-swap mechanisms. It does not decide whether a role or authentication event permits an operation.
- `collector-server` owns principal extraction, cookie/bearer transport, exact-Origin enforcement, peer rate-limit keys, body limits, OpenAPI wiring, and public/private router composition.
- `collector-client` owns a replaceable native credential-storage interface and enrollment flow. Tests use an isolated temporary adapter.
- React uses generated API types and effective-permission booleans. It does not calculate role precedence, modify stored records directly, or decide grant/security policy.

### API and command contract

- Keep `/api/v1` and current read route names where practical. Use explicit DTOs, including ItemSummary, ItemDetail, EvidenceSummary, AccountView, GrantView, EffectivePermissions, and structured security/enrollment views.
- Viewer DTOs omit acquisition, private notes, loans, manual/provider metadata, curator-only assets, detailed evidence payloads, and unknown-module raw values. Omission is structural, not a null-value convention.
- Mutations use bounded command-specific inputs. Item commands never carry Capture representations, file locators, Assets, Evidence, revisions, or other server-owned fields.
- Every mutable resource uses an expected revision; revision zero means create. Grants and account profiles gain revisions.
- Stable error codes distinguish revision conflict, idempotency conflict, command in progress, recent authentication required, last administrator, last owner, disabled account, expired enrollment, and revoked credential.
- Application mutations require `Idempotency-Key`, except authentication ceremonies, offset-based upload chunks, logout, and one-time secret issuance governed by separate state machines. Keys are scoped by user, API major, and operation and are 1–128 safe ASCII characters.
- A private receipt stores operation, key, typed-request digest, state, and a serialized non-secret result indefinitely for the first release. Same key/digest replays the first success; changed payload conflicts; an in-flight duplicate returns the completed result or command-in-progress. Failed validation/authorization does not consume a key.
- Digests cover the validated typed command, path identities, and operation name, preserving RawJson lexical content where it is evidence.
- A deprecated Wave 1 item-save adapter loads canonical server-owned fields, refuses differences, and delegates only editable fields. It cannot publish Capture or Asset snapshots.
- Generate checked-in `api/openapi.v1.json` from annotated Rust routes/DTOs and `web/src/generated/api.ts` with openapi-typescript. A regeneration check must fail on drift. Swagger UI is not exposed on public ingress.

### Persistence and recovery

- Reserve private SQLite migrations in order: v3 command receipts; v4 publication recovery; v5 accounts/sessions/passkeys/recovery/audit/grant revisions; v6 native devices/enrollments. Catalog projection advances from v2 to v3 under P01.
- Archive schema 3 has canonical Items without embedded Capture/Asset copies, top-level revisioned Assets, standalone append-only Evidence, tombstones, and revision envelopes with evidence references and SHA-256 digests.
- Evidence gains collection_id, optional item_id, and one or more exact representation_ids. Preserve existing IDs, claims, timestamps, details, and digests. Repeated legacy IDs must be byte-equivalent after conversion or migration fails.
- Remove legacy_payload_locator from the portable Capture domain. Legacy and new locations stay in archive adapter envelopes/projections.
- Existing Assets remain readable through migration locators; new Assets use collection-level `assets/<asset-id>/payload` and immutable metadata revisions.
- Schema-1 originals, schema-2 sidecars, IDs, raw module/provider envelopes, manual overrides, loans, and payloads remain unchanged. Schema-3 additions are verified. The latest standalone Capture revision always wins over an embedded Wave 1 copy.
- Publication phases are Prepared, BytesStaged, PayloadPublished, RevisionCommitted, ProjectionApplied, and Completed. Immutable revision-directory rename remains the normal commit point. Recovery inspects actual files and digests because a phase record may lag the filesystem.
- Portable active journals contain transaction/entity IDs, expected revisions, safe relative paths, hashes, and phases—never private users, grants, credentials, or secrets. Completed journals are synced then removed; transaction IDs remain in committed manifests.
- Collection creation first writes a private recovery row containing the intended owner grant. Startup completes the grant when the archive commit exists or aborts the row if it does not. Archive-only restoration creates no grants.
- Store open acquires archive/private locks, migrates databases, reconciles known transactions, migrates the archive, repairs projections, and only then exposes writes. Corrupt known transactions fail startup. Unknown staging is retained and reported.
- Fault injection uses an injected no-op production interface. In-process tests fail at named boundaries. Subprocess tests wait for explicit phase markers, terminate the child, and reopen the same temporary archive/databases without timing sleeps.

### Security and administration state machines

- A principal includes credential kind and authentication instant. Browser sessions and native credentials have separate tables and types. Migrated sessions remain browser sessions, keep their old identifier as SessionId, and are non-fresh. Old bearer tokens never become native credentials.
- Recent authentication lasts five minutes from password, passkey, or recovery authentication; ordinary requests do not refresh it.
- A recovery replacement returns ten independently random 160-bit codes. Store only SHA-256 hashes, batch ID, timestamps, consumption state, and redacted audit metadata.
- Consuming a recovery code atomically installs the supplied new password, invalidates the batch, revokes browser sessions, and issues one fresh browser session. Native credentials and grants are unchanged.
- Passkey verification occurs outside SQLite; completion transactionally compares the verified stored record, writes the counter update, and issues the session. Stale compare-and-swap fails closed.
- Account removal is disable-only. Users gain display name, enabled state, and revision. Disable revokes sessions, native credentials, pending enrollments, and recovery batches while preserving identities, password/passkey records, grants, and history. Re-enable does not restore revoked credentials.
- Demoting/disabling the last enabled administrator and removing/demoting a collection's last enabled owner fail. Disabling the durable default user requires explicit default reassignment.
- Local recovery is a stopped-server command with explicit UserId/private-state path. It takes the private lock, resets password, revokes all credentials/passkeys/recovery, records an audit event, and preserves grants and administrator status.
- Password login, passkey challenge creation, and enrollment use independent account/direct-peer/global token buckets with injected monotonic clocks. Forwarded headers never select the peer key.
- Browser login, passkey ceremonies, recovery, and cookie-authenticated mutation require exact normalized Origin. Missing, null, malformed, sibling-host, and alternate-port origins fail.
- Native enrollment start/complete are narrow unauthenticated bootstrap protocol endpoints. They may omit Origin, grant no authority before browser approval, are independently rate-limited, and completion authenticates with a high-entropy enrollment secret—not a caller-selected header.
- Enrollment targets the username supplied by the client, or the durable default in single-user mode. Only that same recently authenticated browser user may approve.
- Enrollment phases are PendingApproval, Approved, Issued, Cancelled, and Expired; expiry is ten minutes. Start returns an ID, high-entropy completion secret, short user code, and generic verification URL without query secrets. Approval requires manual code entry.
- Completion atomically changes Approved to Issued, creates a 30-day native credential, stores only its hash, and returns plaintext once with authenticated Pairing data. A lost response requires revoking the stranded device and restarting enrollment.
- Native authentication succeeds only through the native-credential table. Remove `x-collector-client`; revoked and expired credentials fail immediately.

## Dependency-ordered child tickets

Each child is bounded. Its root remains incomplete until the listed review checkpoint passes.

### A04 — Application/API foundation

1. **A04.1 — DTO firewall and authorized read services** (depends on A01–A03). Split read ports, add explicit role-safe read models/effective permissions, move collection/item/evidence query authorization behind application services, and make unknown raw module values curator-only. Touch collector-app, store read adapters, server read handlers, and app/HTTP tests. Accept when an administrator without a grant cannot browse; viewer bytes contain no private fields; curator/owner fields remain available; read adapters receive no Principal; browsing/artwork/unavailable-module behavior remains usable.
2. **A04.2 — Command, media, import, account, and administration service ports** (depends on A04.1). Move saves, readers, uploads, cover requests, activity, jobs, reindex, collection/user creation, and grants behind application services. Handlers retain transport concerns only. Accept when handlers invoke no policy-bearing Store methods, service and HTTP denial agree, and item inputs cannot supply files/Evidence/Assets/Capture representations.

   Implementation split after handler audit: A04.2a bounded entity commands and item-input firewall; A04.2b media/assets/activity; A04.2c uploads/inbox/jobs/administration; A04.2d authentication/account/passkey mechanisms. These are sequencing units, not new scope. The A04.2 architecture checkpoint occurs only after all four pass.
3. **A04.3 — Optimistic revisions and durable idempotency** (depends on A04.2). Add private v3 receipts, command contexts, conflict taxonomy, and idempotent create/update semantics to current mutations. Accept when retries replay IDs, changed payload reuse conflicts, concurrent duplicates cause one effect, stale revisions publish nothing, and receipts survive restart without secrets.
4. **A04.4 — Generated OpenAPI/TypeScript contracts and Wave 1 compatibility** (depends on A04.3). Generate/check OpenAPI and TypeScript, migrate React from handwritten types, and add the bounded Wave 1 save adapter. Accept byte-identical regeneration, schema/limit coverage for composed routes, drift detection, web build, and both smoke variants.

Architecture review follows A04.2; contract/security review follows A04.4. P01 and S02 begin only after both pass.

### P01 — Durable archive publication

5. **P01.1 — Wave 1 format fixture and archive-schema-3 records** (depends on A04.4). Commit a synthetic e4cb6c5 archive plus private-v2 SQL fixture; define schema-3 revision/evidence/asset/tombstone records and legacy locators. Cover multiple revisions, newer standalone Capture, upload/artwork, duplicate Evidence, manual metadata, loans, raw JSON, and existing private entities without personal data. Conflicting identity/digest input must fail.
6. **P01.2 — Durable journal and recovery kernel** (depends on P01.1). Add journal primitives, phase transitions, safe paths, injected failures, startup reconciliation/reports, and private-v4 recovery rows. Sync phases/parents; make replay idempotent; block corrupt known transactions; retain/report unknown staging.
7. **P01.3 — Compatible schema-2-to-3 migration and projection rebuild** (depends on P01.2). Add sidecars/checkpoints, extract Evidence/Assets, strip compatibility copies, move legacy locators to the adapter, and advance projection v3. Preserve bytes, IDs, raw data, overrides, loans, claims, artwork, accounts, and grants; reject conflicting Evidence; prefer standalone Capture; create no grants from archive alone.
8. **P01.4 — Transactional entity, Evidence, Asset, and projection publication** (depends on P01.3). Journal entity/Asset/tombstone saves and Evidence; replace `INSERT OR IGNORE` with strict identity/content checks. Interruption before commit yields no accepted revision; after commit repairs projection; reverification appends Evidence; stale writes publish nothing; targeted/full recovery agree.
9. **P01.5 — Transactional payload/import publication with process termination** (depends on P01.4). Journal payload, Capture, Evidence, job marker, and projection while keeping resumable scratch separate. Inject and actually terminate at every phase. Never reference partial bytes; reread hashes; reuse identities on identical retry; block conflicting orphan bytes.
10. **P01.6 — Collection/owner-grant recovery and startup write barrier** (depends on P01.5). Coordinate archive collection creation with private intended-owner recovery and order startup behind reconciliation. Every termination point yields no collection or exactly one owned collection in an intact install; archive-only recovery has no grants; no writes occur before reconciliation.

A storage specialist reviews schema 3 and the journal before P01.3. An independent final review reruns termination tests on Linux.

### S02 — Credential lifecycle and recovery

11. **S02.1 — Private auth model, clock, lock, and v5 migration** (depends on P01.6). Add account/session/passkey/recovery/audit models, injected clocks, private lock, profile/grant revisions, and incremental migration. Preserve private state; legacy sessions become non-fresh browser sessions; no row becomes a native credential; future schemas fail closed.
12. **S02.2 — Exact-Origin, recent authentication, and scoped rate limits** (depends on S02.1). Add authenticated transport types, password/passkey reauthentication, and deterministic account/peer/global buckets. Enforce the exact five-minute boundary, non-refreshing activity, Origin matrix, direct-peer keys, and generic deterministic exhaustion errors.
13. **S02.3 — Recovery codes, password replacement, and session lifecycle** (depends on S02.2). Add one-time batches, replacement, consume-with-new-password, session listing/revocation, and redacted audit. Return exactly ten 160-bit codes once; one concurrent winner; revoke old sessions/password; allow fresh new password/session; deny disabled accounts.
14. **S02.4 — Passkey inventory, revocation, and counter CAS** (depends on S02.3). Add labels/revisions, list/revoke/reauth ceremonies, and transactional counter/session issue. Concurrent advanced counters issue at most one session; zero-counter authenticators retain library semantics; revoked credentials fail; sensitive operations require recent auth.
15. **S02.5 — Last-admin/owner guards and local recovery** (depends on S02.4). Add transactional disable/admin/default and grant removal/demotion guards; replace the shortcut reset with explicit stopped-server recovery and a clear old-CLI compatibility error. Concurrent authority changes retain an enabled admin/owner; local recovery requires the lock, revokes credential kinds, audits, and preserves grants/admin status.

S02.2–S02.5 require independent security review and secret scanning of logs, errors, and fixtures.

### S03 — Browser-mediated native enrollment

16. **S03.1 — Enrollment/native-credential state and v6 migration** (depends on S02.5). Implement targeted enrollment, hashed completion secrets, user codes, expiry/cancel, device/credential state, one-time issuance, and clocks; document the bootstrap Origin exception. Wrong user/secret, expiry, cancel, replay, and concurrent completion fail; exactly one device/hash is created and no plaintext persists.
17. **S03.2 — Public browser approval and native authentication transport** (depends on S03.1). Add start/approve/complete/cancel on client ingress, recent-auth approval, credential extraction/revocation/expiry, and remove the header shortcut. Approval remains reachable with public management off; browser Origin/user binding holds; only verified bootstrap/native requests omit Origin.
18. **S03.3 — Client credential adapter and enrolled TLS integration** (depends on S03.2). Add client enrollment and secure-storage ports; update LAN tests to enroll, store in a temporary adapter, authenticate, and consume authenticated Pairing. Reject unrelated identity, revoked/expired credentials, and credential-bearing redirects.

S03 requires independent protocol review and attack-test replay.

### U02 — Account, profile, and grant administration

19. **U02.1 — Account/profile APIs and administration UI** (depends on S02.5 and A04.4). Add server-admin account list/create/edit/enable/disable/default reassignment, self profile edit, and generated-client UI. Use revisions, recent auth, and last/default guards; administrator status alone exposes no collection content.
20. **U02.2 — Collection grant APIs and owner-facing UI** (depends on U02.1). Add collection grant list/add/update/remove, owner UI, and exact-username target lookup; retain a documented old `/admin/grants` adapter. Only owners manage grants; disabled users receive none; removal is immediate; last-owner races fail; administrators need grants.
21. **U02.3 — Effective permissions, single-user behavior, and security controls** (depends on U02.2 and S03.3). Drive UI from EffectivePermissions and integrate passkey/recovery/session/device screens while keeping account selection hidden in single-user mode. Direct APIs remain authoritative; codes display once; refresh reflects revocation/removal.
22. **W2.R — Wave 2 integration and independent review** (depends on P01.6, S03.3, U02.3). Reconcile architecture/status/validation/security/API migration docs and run cross-layer attack, migration, restart, and browser scenarios. Every root criterion links to evidence; shortcuts are not described as architecture; external gates remain explicit; no personal state is opened.

## Selective TDD contract

| Invariant | First failing test and boundary | Real fixture/synchronization | Passing condition |
|---|---|---|---|
| Administrator is not collection access | `administrator_without_grant_cannot_read_collection`, app then HTTP | Temporary private/catalog SQLite and archive | Denied/empty authorized query; no record bytes |
| Viewer filtering | `viewer_item_dto_omits_every_private_field`, DTO serialization | Acquisition, loan, notes, metadata, curator asset, evidence details, raw unknown module | Fields/raw values absent, not null |
| Optimistic revision | `stale_command_publishes_nothing`, app/store | Real archive/SQLite and before/after snapshots | Conflict; no revision, receipt success, Evidence, payload, or projection |
| Idempotency | `same_key_replays_and_changed_payload_conflicts` plus concurrent case | Real receipt DB and barriers before effect commit | One effect; replayed response; changed digest conflicts |
| Archive phases | `fault_at_each_publication_phase_recovers` | Real archive/SQLite and named injected failures | Reopen yields pre-state or exactly one post-state |
| Process termination | `sigkill_at_each_publication_phase_recovers` | Child executable, phase marker pipe, same temporary roots | No sleeps; deterministic complete/abort; originals preserved |
| Projection consistency | `targeted_recovery_equals_full_reindex` | Several entity/Capture/Asset/Evidence revisions | Authorized snapshots are equivalent |
| Schema compatibility | `wave1_schema2_fixture_migrates_without_loss` | Committed e4cb6c5 archive/private-v2 fixture | Original hashes and IDs/history/raw/private state remain |
| Capture authority | `migration_never_projects_embedded_capture_over_newer_standalone` | Embedded N, standalone N+1 | Projection/read uses N+1 |
| Evidence identity | `conflicting_duplicate_evidence_id_blocks_migration` | Same ID, different body | Stop before completion/projection replacement |
| Collection/grant recovery | `collection_create_killed_between_archive_and_grant_recovers` | Private recovery plus journal/barriers | Intact install has one owner; archive-only none |
| Authentication freshness | `recent_auth_expires_at_exact_boundary` | Injected wall clock/session | Allow before five minutes; deny at/after; activity irrelevant |
| Origin | `browser_origin_attack_matrix` | Missing/null/malformed/sibling/alternate-port/spoofed headers | Only exact browser origin; no header-selected native mode |
| Recovery concurrency | `recovery_code_concurrent_consumption_has_one_winner` | Real SQLite, two connections, barrier | One password/session winner; old sessions revoked |
| Recovery replacement | `recovery_batch_replacement_is_atomic` | Old/new batches, injected transaction failure | Exactly one complete usable batch |
| Passkey CAS | `concurrent_passkey_counter_update_fails_stale_assertion` | Synthetic records, two connections, barrier | At most one record update/session |
| Revocation | `revoked_credentials_fail_on_next_request` | Session, passkey, native credential | Immediate failure without cache/expiry wait |
| Last authority | `concurrent_last_authority_changes_cannot_both_commit` | Real SQLite and barriers | At least one enabled admin/owner remains |
| Local recovery | `local_recovery_requires_exclusive_lock_and_preserves_grants` | Lock holder then stopped DB | Live attempt fails; stopped attempt revokes, resets, audits, preserves authority |
| Enrollment terminal states | `enrollment_terminal_states_are_single_use` | Injected clock and real v6 DB | Only Pending→Approved→Issued; terminal replay fails |
| Enrollment binding | `only_requested_user_and_completion_secret_can_finish` | Two users/two secrets | Wrong user approval and wrong secret completion fail |
| Issuance race | `simultaneous_enrollment_completion_issues_once` | Barrier before phase CAS | One plaintext response and one stored hash |
| U02 authorization | `direct_grant_api_matches_affordances` plus smoke | Roles and two collections | UI matches permissions; bypass fails |

Mocks/fakes are limited to pure policy, clocks, and fault hooks. Every persistence or concurrency invariant also has a real temporary SQLite/archive integration test.

## Validation and handoff

After each child, run owning tests before applicable repository gates. A04.4 and W2.R run architecture/contract checks, Rust fmt/test/clippy with `--locked`, web check/build, server rebuild, both smoke modes, the P01 termination matrix, and credential/archive/log secret scanning.

External qualification remains explicit:

- Run P01 process termination on target Linux filesystems. This is not power-loss certification; NAS semantics remain R02 unless Wave 2 reveals incompatible rename/fsync behavior.
- Exercise S02 with a real password manager/platform passkey and hardware authenticator; virtual Chromium is CI coverage only.
- Exercise authentication/enrollment over browser-trusted remote HTTPS; localhost Chrome is insufficient remote-browser certification.
- Validate native secure storage with the eventual macOS client under C01/C03. S03 may finish its port and temporary adapter while recording this gate outstanding.
- Obtain independent storage review after P01 and independent security review after S02/S03. Omitted failure probes become permanent tests before sign-off.

Chosen product decisions: accounts are disabled rather than deleted; enrollment targets the selected/default username and the same user approves; recovery-code use atomically installs a new password. No product choice blocks A04. Remaining uncertainty is external qualification of filesystem crash semantics, real authenticators, trusted remote HTTPS, and platform secure storage.

The required sequence is A04.1–A04.4, P01.1–P01.6, S02.1–S02.5, S03.1–S03.3, U02.1–U02.3, then W2.R. The first implementation-ready child is A04.1.

## Review resolution — 2026-09-10

The approved scope now includes A04.5's explicit command-effect scope and private schema v7 for prepared outcomes and persistent grant revision counters. These close the review's crash-idempotency and grant-revision defects without changing public contracts. Security authority/lifecycle decisions are pure application policies over transactionally stable adapter snapshots. Passkey ceremonies retain their original verified records through CAS. Recognized incomplete journal replacements and Prepared staging are recoverable; committed corruption still blocks startup. See implementation-status.md and validation.md for final evidence and external qualifications.
