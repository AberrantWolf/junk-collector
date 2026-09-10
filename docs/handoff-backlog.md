# Handoff backlog

Baseline: Wave 1 commit `e4cb6c5`, 2026-09-09. **Wave 2 is complete, including review corrections and receipt/publication coordination; external qualification gates remain open.** Ticket status is recorded individually below; existing mechanisms alone are not evidence that a ticket is complete. The approved child sequence and TDD/review contract are in [the Wave 2 execution plan](wave-2-plan.md).

## How to use this backlog

Read `AGENTS.md`, [architecture](architecture.md), [implementation status](implementation-status.md), and the ticket before editing. Start from the current checkout and inspect applicable sibling instructions before any extraction. Preserve local configuration, credentials and user archives; use temporary fixture servers for tests. Never reset the user's state to make a test pass.

Assign one ticket at a time. Dependencies must be completed or their required contract explicitly available before dependent implementation starts. **Routine** tickets suit a less capable model after dependencies. **Review** tickets require experienced architecture/security/domain review of the design and final diff; this is a technical review checkpoint, not an instruction to ask the user for routine tool permission. **Gate** tickets produce a concrete design or fixture inventory first; their dependent tickets must not guess the unresolved contract. Only ask the user about unresolved product choices or unavailable required inputs, after inspecting the repository and sibling code.

Each ticket lists its owning layers, entry files, scope, acceptance and limits. New files/crates are allowed when justified by those boundaries. A ticket that remains too large after inspection must be split into numbered child tickets here, with independent acceptance tests, before a successor is asked to implement it wholesale. Do not silently absorb adjacent tickets. Record status, resulting commits, exact checks and remaining limitations under the ticket on completion; update implementation-status.md and validation.md as appropriate.

### Suggested order

| Wave | Tickets | Purpose |
|---|---|---|
| 1 | A01, U01, A02, A03, S01, D01 | Resolve foundational contracts; improve existing onboarding independently |
| 2 | A04, P01, S02, S03, U02 | Service/API, preservation and access foundations |
| 3 | P02, U03, U04, P03, P04, D02, D03 | Reliable curation, imports and reusable algorithms |
| 4 | D04, D05, D06, D07, P05, S04, C01, C02 | Verification, enrichment, derivatives, migration and native trust |
| 5 | C03, C04, C05, C06, C07, P06 | Client integration, offline use, Trash Amp and scale |
| 6 | R01, R02, R03 | Deployment, performance and release evidence |

Within a wave follow the explicit dependencies below, not table position. Independent tracks may proceed separately if deliberately assigned. After Wave 1 review fixes are validated, the next implementation contracts are **A04** and **P01**; use their explicit dependencies. Do not give a small model the entire table as one coding task.

### Completion rules and checks

A successful build alone does not satisfy an acceptance condition. Test real failure paths, migrations and independent reference data where specified. Use fixtures rather than personal dumps/accounts; do not commit copyrighted dump payloads or secrets. Record oracle provenance and a reproducible way to supply private fixtures locally. Missing hardware, reference dumps, provider access or deployment environments means the corresponding acceptance remains outstanding, not passed.

Apply the checks in AGENTS.md to affected work: architecture check, Rust formatting/tests/clippy, TypeScript checking and web build. Rebuild the server after embedded web changes. Run the existing browser smoke for relevant UI/auth changes, including `LOCAL_SETUP=1 npm --prefix web run test:smoke` for local onboarding. Extend meaningful integration coverage rather than adding tests that merely restate labels or implementation details. Sibling extractions require affected sibling checks. Documentation-only work needs link/dependency/coverage checks, not a Rust rebuild.

## Architecture and transport

### A01 — Inventory contracts and settle foundation discrepancies

**Gate · Dependencies:** none. **Owner:** foundation/application/module design. **Start:** core, app and modules `src/lib.rs`; store `src/lib.rs` and `files.rs`; architecture.md.

**Status (2026-09-09): complete; included in the Wave 1 delivery.** [Foundation contracts](decisions/foundation-contracts.md) records the accepted entity graph, value/default-profile rules, baseline-v1 compatibility, append-only evidence and durable transaction decisions, policy audit, examples and ticket mapping. No persisted schema or runtime capability was changed. Architecture/link checks and the full repository validation listed below pass.

Produce `docs/decisions/foundation-contracts.md` mapping every accepted entity/capability to existing types and missing behavior. Settle persistence/relationship rules for work and edition entities (currently mostly references), component versus carrier, capture identity, typed language/currency/coordinates, and the initial default-profile identity (currently inferred from the first account). Audit service policy currently located in Store/HTTP. Document migration compatibility from the baseline fixture archive.

**Acceptance:** concrete entity relationships, invariants and command ownership; examples for duplicate copies, multi-disc inventory and unattached captures; a decision on append-only evidence records versus accumulated revision snapshots and on durable transaction records versus startup replay. Explicitly distinguish an approved target from an existing shortcut. Each gap maps to A02/A03/A04/P01 or a new bounded ticket.

**Limit:** design and focused contract examples, not a broad refactor. Do not call the accepted requirements fulfilled just because an ID or enum exists.

### A02 — Complete foundation entities and migrations

**Review · Dependencies:** A01. **Owner:** core types, app validation, store migration. **Start:** core `Item`, `Specimen`, `Carrier`, `Capture`, `Money`, `LocalizedText`.

**Status (2026-09-09): complete; included in the Wave 1 delivery.** Architecture and final-diff review corrections are implemented and regression-tested. The child changes below are complete. The migration-only checkpoint and embedded legacy evidence/payload locators are explicitly transitional; P01 remains owner of the general transaction and append-only evidence protocol. Validation and discrepancies are recorded in validation.md.

#### A02.1 — Core model and validation

**Status: complete.** Adds the reviewed identities, Work/Edition records, exclusive Item identity links, ordered Components/Carriers, Capture associations, typed asset subjects and validated language/currency/coordinate/digest values. Collection graph and representation validation reject duplicate/broken ownership, invalid physical carrier states, invalid values, active-loan conflicts, and local cycles. Lossless raw-JSON envelopes preserve unavailable module/provider lexical bodies.

#### A02.2 — Non-destructive archive migration

**Status: complete with the documented P01 transition.** Schema-1 records remain byte-identical. The restartable schema-2 migration writes typed sidecars/records, deterministic Components, first-appearance Capture revisions, legacy provenance/locators, and completes last. Reindex validates and replaces expanded projections. A committed synthetic schema-1 fixture—not the ignored local installation—exercises migration/restart/repeat, duplicate copies, two discs, legacy case/booklet carriers, payload downloads, manual/evidence data, uppercase digest provenance, and raw JSON.

#### A02.3 — Durable default profile

**Status: complete.** Private/catalog SQLite versions advance transactionally. Legacy first-row behavior is persisted once as `default_user_id`; first-account creation sets it in the same transaction, later creation cannot change it, and a missing/deleted target reports an actionable configuration error.

Implement the reviewed entity/value contracts and durable default-profile selection; add versioned archive/private-state migration and reference validation. Retain durable IDs across renames and preserve the current initialized account.

**Acceptance:** baseline archive loads/migrates without lost IDs or fields; invalid/dangling relationships fail clearly; duplicate specimens, multi-disc editions, no-file inventory and unidentified captures round-trip. Unknown module/provider payloads survive migrations. Multi-user toggling does not silently select another profile.

**Limit:** no provider integration or new inventory UI; split entity migration and private-profile migration into child changes if needed.

### A03 — Complete compiled-module extension contracts

**Review · Dependencies:** A01. **Owner:** modules and application host ports. **Start:** `CollectionModule`, `Registry`, module tests.

**Status (2026-09-09): complete; included in the Wave 1 delivery.** Architecture and final-diff review corrections are implemented and regression-tested. Indexed namespaced capability ownership/state errors, exact schemas, checked pure adjacent migrations with explicit source-preserving transformations, opaque readers, bounded job requests and safe presentation are implemented. The Stamps test module migrates schema 1 to 2, rejects destructive migration, reads bytes, requests a fake job and presents without a core enum edit. Production operation transport remains deliberately absent until A04.

Add schema migrations, capability dispatch and presentation contributions through typed contracts. Modules request host I/O/jobs through interfaces; they do not acquire filesystem/HTTP dependencies. Define unsupported schema/capability behavior and a generic presentation payload.

**Acceptance:** the test-only third module contributes validation, migration, a host-mediated operation and presentation without core enum edits; unavailable modules retain data across import/export/reindex and display generically. Reject undeclared destructive migrations; declared schema transformations must retain exact source provenance.

**Limit:** compiled modules only; no dynamic loader, native ABI or runtime sandbox.

### A04 — Service ports, generated contracts and command semantics

**Review · Dependencies:** A01, A02, A03. **Owner:** app policy; store adapters; HTTP DTOs. **Start:** app `Catalog`/`CollectionService`, server handlers, `web/src/types.ts`.

Move shared command/authorization policy behind application services as identified by A01. Generate versioned OpenAPI and TypeScript from transport contracts, with cursor/revision/error conventions, bounded DTOs and persisted idempotency semantics for mutations. Define replay scope, lifetime and conflict behavior explicitly. Keep transport DTOs separate from unrestricted stored records.

**Acceptance:** generated contracts are reproducible in CI; mismatched payload reuse of an idempotency key fails; retrying a successful create does not create another entity; stale revisions conflict; private fields cannot leak through generated DTOs; malformed and oversized inputs are bounded. Existing clients retain a documented migration path.

**Limit:** do not generate business logic into frontends. Split by service family if necessary, retaining one documented policy owner.

Planned children (complete in order): [A04.1 DTO firewall/read services](wave-2-plan.md#a04--applicationapi-foundation), A04.2 command/mechanism service ports, A04.3 optimistic revisions/idempotency, and A04.4 generated contracts/Wave 1 compatibility. A04.2 and A04.4 have mandatory architecture and contract/security review checkpoints. All children, including review follow-up A04.5, are complete.

**Root status (2026-09-09): complete; included in Wave 2.** All four children and the architecture/contract/security checkpoints pass. The generated contract, migration note, full repository gates, normal browser smoke, and local-setup browser smoke use only repository fixtures and temporary state.

#### A04.1 — DTO firewall and authorized read services

**Status (2026-09-09): complete; included in Wave 2.** Collection reads use a Principal-free port; CollectionService owns authorization/redaction and emits explicit collection, item, Capture, Specimen, and Evidence response shapes with effective permissions. Viewer JSON omits private fields and unavailable-module raw values structurally. Capture archive locators are absent for every role. A real temporary Store/archive HTTP test covers viewer bytes, curator fields, detailed-Evidence filtering, and an administrator without a grant. Owning tests, architecture/fmt/workspace/clippy gates, web check/build, server rebuild, and browser smoke pass.

#### A04.2 — Application-owned command and mechanism service families

**Status (2026-09-09): complete; included in Wave 2.** A04.2a–d and architecture review pass. Entity writes use bounded commands that cannot express server-owned files/Evidence/Assets/revisions/locators; media is an opaque reader capability; activity, import/job, administration, authentication, and passkey persistence use Principal-free Store ports. Inbox collection authorization precedes filesystem inventory. An architecture check rejects direct Store calls from HTTP/passkey source. Real archive/SQLite and HTTP tests cover command injection, service/HTTP upload ownership, private-asset visibility, role denial, retry/media ranges, and admin denial. Full Rust/web gates and browser smoke pass.

#### A04.3 — Optimistic revisions and durable idempotency

**Status (2026-09-09): complete; included in Wave 2.** Private SQLite schema v3 stores operation-scoped command receipts keyed by user/API major/idempotency key and a typed-request SHA-256 digest. Application mutations require a validated command context; authentication ceremonies, logout, upload chunks, and internal failure bookkeeping retain their explicit state-machine/offset semantics. Successful responses replay across restart, changed requests conflict, concurrent receipt claims have one executor, and validation/authorization/stale-revision failures abandon the claim. HTTP requires `Idempotency-Key` and returns stable `idempotency_conflict`/`command_in_progress` codes. Real SQLite/archive tests cover restart replay, concurrency, and stale publication.

#### A04.4 — Generated OpenAPI/TypeScript contracts and Wave 1 compatibility

**Status (2026-09-09): complete; included in Wave 2.** Runtime API composition now uses `utoipa-axum` route groups, which generate checked [OpenAPI v1](../api/openapi.v1.json) and [TypeScript types](../web/src/generated/api.ts). A temporary regeneration check detects either artifact drifting. The React application imports generated response/command types and no longer constructs response-shaped Items for saves. Every request body records the matching router body limit, and the contract/Swagger UI is not served. The deprecated Wave 1 `{item,expected_revision}` adapter compares canonical server-owned state before delegating editable fields and cannot publish Capture/Asset snapshots. Real HTTP tests cover route/schema completeness, absence of locator response fields, the unavailable public schema route, compatible legacy editing, and owned-field tampering. The contract/security review and all full A04 gates, including both browser smoke modes, pass. **Next child: P01.1.**

#### A04.5 — Receipt/effect crash-window review

**Status (2026-09-10): complete; included in Wave 2.** Explicit command-scoped adapters commit private effect/results in one SQLite transaction or prepare the exact response against an immutable filesystem publication marker. Private schema v7 adds command outcomes; startup settles committed responses before accepting writes, repairs upload bookkeeping, and removes completed artwork scratch. A real subprocess matrix terminates after effects across 19 mutation families and additionally at early archive commit points; every committed command replays the exact response and rejects changed payloads. Prepared partial files and interrupted journal replacements recover without weakening committed-byte checks.

## Onboarding, inventory and management

### U01 — Explain local setup and denied operations in the WebUI

**Routine · Dependencies:** none. **Owner:** presentation and CLI diagnostics. **Start:** server `main.rs`, web `app.tsx`, `administration.tsx`, `curation.tsx`, smoke test.

**Status (2026-09-09): complete; included in the Wave 1 delivery.** The WebUI preserves structured error codes, distinguishes wrong origin, expired/revoked session and role denial, presents honest success/error state, and guides empty/client-only surfaces to the correct next action. Setup/doctor wording and browser limitations are documented. Normal and `LOCAL_SETUP=1` Chromium smoke tests pass with temporary accounts, archives and TLS state; HTTP error-code assertions and all repository checks pass.

Build on existing setup/doctor rather than replacing it. Add clear local-versus-client-surface guidance and contextual next steps when no collection is visible or a management operation is denied. Verify session-expiration handling and avoid leaving a successful-looking form after failure. Keep Add for a new item and Save for an existing item.

**Acceptance:** fresh and repaired local setup reaches collection creation; wrong-origin, expired-session and genuine role denials are distinguishable; public-only entry directs users appropriately without exposing private settings. Existing accounts/configuration are preserved. Document browser-trust limitations; do not claim untested Safari support.

**Limit:** no weakening cookies, origin checks or listener separation; no new grant model. Missing required capability API work becomes a separate dependency.

### U02 — Account, profile and grant administration

**Routine after review of role operations · Dependencies:** A02, A04, S01, S02.5. **Owner:** app authorization, private store, administration UI. **Start:** store `auth.rs`, app `grant`, web administration bundle.

Implement account/profile listing/editing and collection grant assignment/removal, with explicit permitted actions and useful empty states. Expose effective collection permissions for UI affordances through authorized queries. Hide account switching in single-user mode while retaining device security.

**Acceptance:** owner/curator/viewer and server-admin cases are exercised; server administration does not silently bypass collection policy. A viewer cannot curate via direct API. Grant removal takes effect on subsequent requests. Last-administrator/owner removal follows S01's reviewed recovery rules and cannot accidentally strand access.

**Limit:** credential recovery/revocation ceremonies belong to S02/S03; UI hiding is not authorization.

Children: [U02.1 account/profile APIs and UI, U02.2 grant APIs and owner UI, and U02.3 effective permissions/security controls](wave-2-plan.md#u02--account-profile-and-grant-administration). U02.3 additionally depends on S03.3.

**Status (2026-09-10): complete; included in Wave 2.** Revisioned account/profile/default-user APIs and UI preserve disabled identities, enforce recent authentication and surface last-admin/default conflicts. Collection owners can list, add, revise, and remove exact-username grants; disabled users cannot receive new grants, removal applies on the next request, and transactionally protected last-owner races fail. Account security screens list/revoke sessions, passkeys, and native devices and display recovery codes once. All curation/access affordances use `EffectivePermissions`, while direct API tests prove UI hiding is not authorization. **External usability review remains part of W2.R.**

### U03 — Physical inventory lifecycle editing

**Routine · Dependencies:** A02, A04, U02. **Owner:** core invariants/app commands, curation UI. **Start:** specimen/ownership/possession/acquisition/loan types and curation bundle.

Provide focused editors for copies, components/carriers, condition, acquisition, ownership/possession, lending/return and disposal. Show revision conflicts without discarding unsaved user input.

**Acceptance:** two copies of the same edition can be loaned/disposed independently; returning a loan retains history; wishlist-only and multi-disc items work; sale/disposal preserves archive bytes/evidence. Private acquisition/loan details remain filtered for viewers. Round-trip through reindex retains all history.

**Limit:** no trading marketplace or deletion of payloads as a lifecycle side effect.

Planned children (complete in order): U03.1 focused lifecycle commands, U03.2
lifecycle editor, and U03.3 privacy/reindex review. See
[the Wave 3 execution plan](wave-3-plan.md).

**Status (2026-09-10): complete locally; U04.1 is next.** The broad Item-save
command can no longer submit specimen state, and the Wave 1 adapter delegates
only when its specimen snapshot exactly matches canonical storage. Focused,
revisioned, idempotent commands and curator-only routes create/update specimens
and ordered components, remove only unreferenced components, start/return loans,
and dispose specimens without deleting their history or archive relationships.
Specimen, Component, Carrier, and Loan identities derive deterministically from
command identity. Permanent tests cover two copies, wishlist-only creation,
multi-disc ordering, Capture/Asset removal guards, returned-loan history,
disposal, replay, authorization, reindex, and restart. The obsolete broad-save
add-copy UI shortcut was replaced only after the focused APIs worked. The
archival-workbench editor now manages copies, condition, acquisition/private
notes, ordered parts/carriers, lending, explicit return, retained loan history,
and disposal. A real revision race keeps the user's draft and offers either
server reload/discard or manual reapply against the displayed current revision.
Chromium exercises the full lifecycle, keyboard order, and a 390px responsive
layout. U03.3 proves both detail and browse DTOs structurally omit acquisition,
private notes, contacts, and loan history for viewers; all focused mutations
follow curation listener composition; and independent copy state survives
reindex/restart. Both normal and private-local Chromium suites pass. Independent
review remains externally unqualified.

### U04 — Sensitive supporting assets and role-aware artwork

**Routine · Dependencies:** A04, U02. **Owner:** app asset services, store images, web presentation. **Start:** store asset methods, server asset routes, `Artwork` component.

Add photos/scans and visibility controls using existing original/thumbnail storage. Apply visibility to originals, thumbnails and metadata; retain artwork originals and bounded derivatives. Give all asset actions clear labels and appropriate access affordances.

**Acceptance:** private assets and thumbnails are denied through direct URLs and filtered queries; games and CDs show art beside text in search/browse, with lazy loading, keyboard-accessible rows and distinct uncertainty/placeholder states. Oversized/corrupt images fail within resource bounds.

**Limit:** do not duplicate image-processing algorithms in React; offline propagation is C05.

**Status (2026-09-10): complete locally; P03.1 is next.** Standalone,
collection-scoped Asset commands now create PNG/JPEG/WebP Covers, Photos, and
Scans for Work, Edition, Item, Specimen, Component, and Capture subjects. The
single store image pipeline preserves original bytes, emits bounded PNG
thumbnails, journals independent Asset revisions, allows only visibility/alt-text
updates, and tombstones metadata without removing history or payloads. Item
revisions do not change. Reindex retains standalone Work/Edition assets rather
than depending on the legacy Item projection. Permanent tests cover every
subject and image format, replay, update/tombstone history, preserved payloads,
and fail-closed corrupt, oversized, document, and cross-collection input. U04.2
adds revisioned `PresentedAsset` DTOs plus collection-scoped metadata, original,
thumbnail, create, metadata-update, and archive routes. Viewer metadata omits
visibility and stored file/digest details; curator-only assets are rejected before
opening either byte representation. The legacy Item URL delegates through the same
policy. Edition-linked Items inherit Work Assets, and server-selected
`primary_artwork` prefers the first visible module choice before the newest visible
Item/Edition/Work Cover with deterministic ID tie-breaking. Mutation routes remain
absent when curation exposure is disabled. The archival-workbench UI now creates
labeled attachments at Item/copy/part/Capture/linked-identity scope, previews the
selected local image, displays lazy stored thumbnails, edits visibility/alt text,
and archives with retained-history guidance and focus restoration. Browse/detail
use server-selected artwork, while visible “No art” remains distinct from
tentative/unidentified identity. Normal and private-local Chromium suites cover
the real workflow, keyboard controls, viewer filtering, reduced-motion CSS, and a
390px no-overflow visual review. Document uploads and offline propagation remain
explicitly unavailable. Independent review is externally unqualified.

## Security and enrollment

### S01 — Threat model and recovery/authorization contracts

**Gate · Dependencies:** none. **Owner:** application/security design. **Start:** server middleware/passkeys, store auth, client pairing, exposure tests.

**Status (2026-09-09): complete; included in the Wave 1 delivery.** [Security contracts](decisions/security.md) records trust/ingress boundaries, distinct administrator and collection authority, five-minute recent authentication, recovery-code/local-recovery rules, last-admin/owner refusal, passkey CAS, Exact-Origin/native credential rules, initial rate buckets, and attack-test ownership. S02/S03 are now implemented; S04 remains unimplemented.

Write `docs/decisions/security.md` with trust boundaries, principal/role and ingress matrix, enrollment binding, recent-authentication policy, recovery-code lifecycle, last-admin recovery, passkey counter/concurrency handling, session/credential storage and revocation scope. Audit CSRF, rate limits, upload/provider resource abuse, log redaction and proxy behavior against code. List concrete defects separately from hypotheses.

**Acceptance:** define attack-oriented tests for S02/S03/S04 and release review; specify which actions require recent authentication and how local recovery works. A browser-trusted public authentication origin and a paired native device have distinct trust roles. Explicitly address localhost browser limitations without granting all LAN traffic private status.

**Limit:** design/review first; do not invent cryptography or broadly disable existing protection to make onboarding easier.

### S02 — Credential lifecycle and recovery

**Review · Dependencies:** S01, A04. **Owner:** private store/auth services/HTTP, security UI.

Implement the reviewed recent-authentication gates, recovery-code creation/use/replacement, passkey listing/revocation, concurrent counter handling and local administrator recovery. Integrate U02 screens when available; keep service acceptance independent of the UI.

**Acceptance:** codes are hashed, single-use and safe under concurrent replay; stale authentication is rejected for gated actions; passkey and password recovery/revocation semantics match the contract; logs/errors reveal no credentials. Test real password managers/hardware as an explicit external gate, beyond the virtual authenticator.

**Limit:** no change to collection roles as an undocumented recovery side effect.

Planned children: [S02.1 private auth model/migration, S02.2 Origin/freshness/rate limits, S02.3 recovery/session lifecycle, S02.4 passkey CAS/inventory, and S02.5 authority guards/local recovery](wave-2-plan.md#s02--credential-lifecycle-and-recovery).

**Root status (2026-09-10): complete; included in Wave 2; independent security/hardware review pending.** All five children pass deterministic real-SQLite/application/HTTP tests. No hardware password-manager or remote browser-trusted HTTPS qualification is claimed.

#### S02.1 — Private auth model, clock, lock, and v5 migration

**Status (2026-09-10): complete; included in Wave 2.** Core now has distinct Session, Passkey, Enrollment, and NativeCredential IDs; application auth models distinguish credential kind and authentication instant. Store construction accepts an injected wall/monotonic clock and exclusively locks private state independently of the archive. Incremental private schema v5 adds enabled/revisioned account profiles and grants, explicit browser sessions with non-fresh migrated authentication instants, revisioned/labeled passkeys with stable migrated IDs, recovery batches/codes, and redacted security-audit storage. The committed v2 fixture proves every existing row survives and no session becomes a native credential.

#### S02.2–S02.5 — Browser defenses, recovery, credential inventory, and authority guards

**Status (2026-09-10): complete; included in Wave 2.** Exact normalized Origin is mandatory for browser ceremonies/cookie mutations; forwarded addresses never choose rate keys, and injected clocks cover the exact five-minute freshness and account/peer/global bucket boundaries. Ten-code 160-bit recovery batches are hash-only, replacement and concurrent consumption are atomic, successful recovery replaces the password/revokes browser sessions/issues one fresh session, and disabled accounts cannot recover online. Passkey inventory/revocation and counter/session CAS fail stale assertions closed. Concurrent account/grant changes cannot remove the last enabled administrator or owner; disabling the default requires reassignment. Explicit stopped-server local recovery holds the private lock, revokes all credential kinds, audits, and preserves grants/administrator state.

### S03 — Browser-mediated device enrollment

**Review · Dependencies:** S01, S02, A04. **Owner:** auth application services, private device state, HTTP enrollment, client credential adapter.

Implement bounded, expiring, single-use enrollment bound to the requesting device and authenticated user, provisioning server trust and revocable credentials. Specify native secure storage behind a platform adapter and clear cancellation/retry behavior.

**Acceptance:** replay, cross-device completion, wrong-user approval and expired enrollment fail; sign-in remains reachable on public client ingress with management off; a paired client authenticates over an enrolled alternate TLS route. Secrets do not enter URLs/logs or archive exports.

**Limit:** manual enrollment first; discovery is C02, key rotation S04. No unauthenticated identity response counts as trusted pairing.

Children: [S03.1 enrollment persistence/state, S03.2 browser approval/native transport, and S03.3 client credential/TLS integration](wave-2-plan.md#s03--browser-mediated-native-enrollment).

**Status (2026-09-10): complete; included in Wave 2; protocol/platform review pending.** Private schema v6 stores expiring targeted enrollment, hashed completion secrets, terminal phases, revisioned native devices, and credential hashes. Wrong-user approval, wrong-secret/cross-device completion, expiry, cancellation, replay, simultaneous completion, revocation, and plaintext scanning are permanent tests. Client-surface bootstrap endpoints grant no authority before same-user recent browser approval; only native-table lookup selects bearer authentication and `x-collector-client` cannot issue or authenticate credentials. `collector-client` supplies the secure-storage port, validates Pairing/server identity and enrolled certificates, refuses redirects/cross-origin credential delivery, and the LAN TLS integration performs the complete enrollment/persist/authenticated-range/revocation path. A production macOS secure-storage adapter remains C01/C03 external work.

### S04 — Authenticated local TLS identity rotation

**Review · Dependencies:** S03, C01. **Owner:** server private identity lifecycle and client trust persistence.

Implement a reviewed rotation protocol covering authenticated old-to-new identity transition, offline devices, expiry, interruption and explicit recovery enrollment. Keep durable server identity distinct from endpoint names/certificates.

**Acceptance:** valid rotation preserves enrollment; changed unannounced certificates and hostile rotation messages fail closed before credentials; interrupted updates recover consistently; offline-device behavior and browser-trust consequences are documented and tested.

**Limit:** no accept-invalid-certificates fallback or inferred trust from matching server ID alone.

## Archive, imports and jobs

### P01 — Durable archive publication and recovery gaps

**Status (2026-09-10): implemented; included in Wave 2; independent target-Linux storage review pending.** P01.1–P01.6 are complete with real archive/SQLite, deterministic fault, and subprocess-termination coverage. The remaining external qualification is explicitly not a power-loss/NAS certification.

**Review · Dependencies:** A01, A02, A04. **Owner:** store transaction adapter; application transaction ports. **Start:** store publication/reindex and archive tests.

Implement A01's reviewed evidence and transaction design, including collection manifest/grant crash gaps, payload publication, projection commits, revisions/tombstones and pre-write recovery. Preserve baseline archives through a versioned migration.

**Acceptance:** fault injection at staging, durable-record write, payload/manifest publication and projection/grant update recovers deterministically; no accepted write references partial bytes; stale revision conflicts do not publish; process termination/restart preserves overrides/loans/evidence. Unknown data and IDs survive directory renames/reindex. Archive-only recovery requires explicit grants.

**Limit:** one writer, no replication or cross-collection deduplication. A passing ordinary save test is insufficient.

Planned children: [P01.1 Wave 1 fixture/schema 3, P01.2 journal kernel, P01.3 compatible migration, P01.4 entity/Evidence/Asset publication, P01.5 payload publication/termination, and P01.6 collection-grant recovery/write barrier](wave-2-plan.md#p01--durable-archive-publication).

#### P01.1 — Wave 1 format fixture and archive-schema-3 records

**Status (2026-09-09): complete; included in Wave 2.** A reproducible example materializes a synthetic schema-2 archive from Wave 1 behavior, including a standalone Capture newer than its embedded compatibility copy. A paired private-v2 SQL fixture covers accounts, sessions, passkeys, grants, settings, activity, and jobs without personal data. Schema-3 item, Asset, standalone Evidence, tombstone, legacy-locator, and revision-envelope shapes are defined. Real archive/SQLite tests prove upgrade opening does not alter existing archive bytes, private rows survive the incremental v3 migration, and the standalone Capture remains authoritative. **Next child: P01.2.**

#### P01.2 — Durable journal and recovery kernel

**Status (2026-09-09): complete; included in Wave 2.** Archive publication journals validate adjacent revisions, lowercase SHA-256 digests, and strictly normal relative paths. Journal writes and phase replacements fsync their file and parent directory. Startup inspects actual staged/published bytes, classifies lagging journals, retains and reports unknown staging, cleans completed journals, and refuses corrupt known transactions before migrations or writes. Private SQLite advances incrementally to v4 with a publication-recovery table. Named fault points cover every durable phase with real temporary archive files and deterministic injection; subprocess termination is intentionally P01.5. **Next child: P01.3.**

#### P01.3 — Compatible schema-2-to-3 migration and projection rebuild

**Status (2026-09-09): complete; included in Wave 2.** The additive migration writes deterministic schema-3 Item and Capture sidecars, standalone immutable Evidence, revisioned collection-level Asset metadata, legacy payload locators, per-Item checkpoints, and a last completion marker. Canonical Item records exclude Capture/Asset compatibility vectors, and the portable Capture domain no longer contains archive paths. Catalog SQLite advances rebuildably to v3 with standalone Evidence/Asset projections. Opening the committed Wave 1 fixture preserves every pre-existing byte, all private rows, raw envelopes, manual metadata, loans, artwork, evidence, and IDs; the newer standalone Capture remains authoritative. Conflicting duplicate Evidence IDs block completion and projection creation. **Next child: P01.4.**

#### P01.4 — Transactional entity, Evidence, Asset, and projection publication

**Status (2026-09-09): complete; included in Wave 2.** Normal Item, Work, Edition, Capture, standalone Evidence, and Asset revisions now publish as schema-3 envelopes through the durable phase journal. New cover payloads use deterministic command-bound Asset IDs and collection-level payload paths in the same journal; migrated item-local artwork remains reachable through verified legacy locators. Item envelopes reference immutable Evidence by ID/digest rather than copying bodies; compatibility Capture/Asset views are projections only. Evidence projection uses exact-content conflict checks instead of `INSERT OR IGNORE`. Startup aborts pre-commit journals, rebuilds projections for committed revisions, and then durably completes them. Real fault injection at every phase proves pre-command or exactly-one post-command recovery, and targeted projections equal a full reindex. Reverification produces new standalone Evidence bodies and reference-only Item revisions. **Next child: P01.5.**

#### P01.5 — Transactional payload/import publication with process termination

**Status (2026-09-10): complete; included in Wave 2.** Resumable private upload chunks remain separate from archive publication. Commit uses the durable upload/Job identity as its transaction, copies and fsyncs verified bytes into archive-local staging, publishes payloads, commits the standalone Capture revision, repairs its projection, then publishes deterministic Evidence and the Item reference. Pre-Capture recovery removes only hash-verified known payload artifacts; post-Capture recovery preserves and completes the same Capture/Representation/Evidence identities. Real child test processes block on pipe-visible phase markers and are killed at all six phases without timing sleeps; every retry yields one complete capture and retained source chunks. **Next child: P01.6.**

#### P01.6 — Collection/owner-grant recovery and startup write barrier

**Status (2026-09-10): complete; included in Wave 2.** Collection IDs are deterministically bound to the authorized idempotency scope before the effect. A private v4 recovery row containing the intended collection/owner is committed before archive mutation. Startup under the archive lock migrates private state, reconciles collection recovery, resolves prepared command outcomes and releases uncommitted claims, performs archive migrations/reindex, and only then returns a writable Store. A valid committed collection gains exactly its intended owner; absent archive commit aborts the recovery; malformed known state fails closed. Controlled faults and killed child processes cover every collection boundary, and archive-only recovery receives no grants. **P01 is complete pending its independent Linux/filesystem review. Next child: S02.1.**

### P02 — Durable job scheduler and cancellation

**Review · Dependencies:** A04, P01. **Owner:** application job state machine and store execution adapter; HTTP worker composition.

Replace persisted status plus process-local scheduling with durable claims/restart handling, bounded concurrency, progress, cancellation and idempotent retries. Define leases/reconciliation and cancellation commit boundaries before implementation.

**Acceptance:** stop/restart during each job phase neither loses a job nor duplicates publication/evidence; cancellation retains sources and leaves consistent state; failed tools produce actionable limitations; resource limits hold under several users. No blocking parser work runs on async transport workers.

**Limit:** local scheduler only; no distributed queue service.

Planned children (complete in order): P02.1 durable job repository and v8
migration; P02.2 claim/worker state machine; P02.3 cancellation, retry, and
publication reconciliation; P02.4 job API and review. See
[the Wave 3 execution plan](wave-3-plan.md).

**Status (2026-09-10): complete locally; U03.1 is next.** Private schema
v8 stores typed jobs and migrates legacy uploads without identity/history loss.
Transactional FIFO claims enforce two active jobs globally and one per user, with
30-second leases, five-second heartbeats, prior-instance restart takeover, claim
fencing, and bounded progress persistence. Two server worker loops execute the
first real consumers. Integrity verification, browser upload publication, and
configured-inbox staging/publication now use the durable workers; no process-local
job semaphore remains. Cancellation is cooperative before the fenced archive
revision boundary and stably too late afterward. Retry retains source, identities,
attempt count, and prior failure details. Cursor job pages plus submitter-only
detail/cancel/retry routes expose bounded safe views and require idempotency keys
for mutations; stored requests, failure history, worker identities, leases, and
claim tokens never cross the API. An administrator cannot bypass submitter scope.
The local root review passed 112 Rust tests and Chromium smoke. Target-Linux
process/filesystem behavior and independent review remain externally unqualified;
event streaming remains C06 and distributed scheduling remains out of scope.

### P03 — Provisional package review and resumable ingest

**Review · Dependencies:** A04, P01, P02, A03. **Owner:** application import workflow; store staging; HTTP uploads.

Separate package inventory/staging, analyzer candidates, user review/physical association and publication. Allow preservation without an analyzer or provider. Add durable resume/disconnect behavior and explicit expected revisions across review.

**Acceptance:** malicious paths/symlinks, incomplete multi-file packages, corrupt/conflicting chunks and concurrent edits fail safely; retries after browser/server restart resume deterministically. Unidentified capture can publish with no fabricated specimen. Sources remain unchanged and destination bytes are verified. Provider outage cannot block basic preservation.

**Limit:** use existing upload/inbox mechanisms, not a second importer; actual analyzer implementations arrive in D04/D05.

**Status (2026-09-10): complete locally; P04.1 is next.** Private schema
v9 now persists a durable provisional Import separately from its execution Jobs:
safe source summary, bounded declared-file inventory and resumable offsets/digests,
preallocated Capture/Representation identities, optional physical association and
expected Item revision, raw candidate observations, limitations, review decision,
analysis Job link, and workflow phase. Optimistic revision writes preserve immutable
source and identities across restart. Validation rejects private path-shaped source
summaries, unsafe/case-colliding inventories, invalid offsets/digests, absent selected
candidates, and premature publication. A completely staged, reviewed but unidentified
and unattached Import is valid without fabricated catalog or physical identity.
Browser chunks and curator-selected configured inbox packages now converge on one
private scratch representation and the same digested `ImportView`. Inbox work runs
as a cancellable/retryable P02 Job. It rechecks each entry and the final inventory
for symlinks, special files, escape, case collisions, sizes, metadata, and content;
browser offsets and exact replay survive restart. Collection-scoped endpoints expose
only configured labels and direct package names, while submitter scope hides another
user's Import and both Import/Job views omit source roots. Optional module analysis
now runs as a P02 Job through the A03 owner capability and stores bounded raw
alternatives/limitations. Built-in game/music analysis honestly yields no candidate
until D04/D05. Review supports candidate or unidentified identity plus a canonical
existing physical chain or no association; stale Item revision conflict preserves
the submitted selection in `awaiting_review`. Publication is a durable P02 Job that
re-reads source scratch, preserves the preallocated Capture/Representation identities,
and uses the P01 journal for payload and revision commit. It can publish an unattached
Capture plus integrity Evidence without inventing an Item or Specimen. Cancellation
retains scratch before commit and is stably too late after `RevisionCommitted`, where
reconciliation completes exactly once. The legacy `/uploads` and admin inbox routes
delegate to Imports and distinct Jobs; pre-v9 in-flight uploads remain readable through
a bounded fallback, and both compatibility routes are marked for API-major removal.
P03 passed its local root review with 138 tests. Independent review and target-Linux/
filesystem termination behavior remain externally unqualified.

### P04 — Inbox, import-review and job UI

**Routine · Dependencies:** P02, P03, U02. **Owner:** web curation/administration through application APIs.

**Status (2026-09-10): complete locally; D02.1 is next.** The curation
workbench now stages browser files or a configured-inbox package through the one
P03 workflow, displays the complete inventory and durable progress, shows honest
analysis limitations/candidate choices, records Item/copy/part/carrier or unattached
association, and publishes the Capture. The Chromium interaction covers both source
selectors and a full unidentified/unattached browser publication at desktop and
390px. A submitter-scoped active/history ledger now shows bounded progress,
attempts, diagnostics, and real cancel/retry controls. It polls visible active work
every two seconds, backs off to 30 seconds when idle, and refreshes on reconnect or
action without owning or resetting Import review drafts. Chromium retries a real
stale-publication failure and observes attempt 2. The P04.3 matrix now proves
refresh restoration, offline/reconnect recovery, retry and queued cancellation,
stale physical review with choices retained/reapplied, unavailable-tool disclosure,
viewer denial for another user's Job/Import and unauthorized staging, keyboard/focus
order, live progress, reduced motion, and mobile layout in both normal HTTPS and
private local-setup modes. P04 passed its local root review; C06 event streaming and
independent accessibility/security review remain outstanding.

Add configured inbox selection, package inventory/review, provisional association controls, durable job progress/error/cancel/retry views and reconnect behavior. Configuration administration remains private by default; collection imports use the curation surface.

**Acceptance:** browser upload and inbox package reach the same review workflow; retry/cancel remain usable after refresh; missing tools are explained without a false verification badge; users cannot enumerate unauthorized server paths or other users' jobs.

**Limit:** no filesystem paths assembled by the browser, and no UI-owned scheduler.

### P05 — Export, full backup and explicit recovery

**Review · Dependencies:** P01, P02, A04. **Owner:** store export/recovery adapter and app/admin commands.

Implement portable archive export, exportable revisions/tombstones/changes, consistent full backup of private state, restore validation and explicit access reassignment for archive-only recovery. Start with CLI and add administration status/actions through the same services. Document restoration versus independent-server cloning and identity handling.

**Acceptance:** recover into a fresh temporary installation; verify originals, historical evidence, overrides and loans; archive-only recovery has no inferred grants or leaked credentials. Full restore retains intended private state coherently. Unsafe live SQLite copying is rejected/avoided. Unknown module data round-trips.

**Limit:** no replication protocol; no destructive cleanup of the source archive.

### P06 — Incremental catalog startup and indexing

**Review · Dependencies:** P01, P05. **Owner:** store projection/reindex adapter.

Replace unconditional full startup reindex with verified incremental replay/index maintenance, retaining an explicit complete rebuild path. Define checkpoints relative to the archive commit point and handle missing/corrupt projections. Replace whole-collection graph/reference scans on writes with indexed ownership/reference checks, preserving the same invariants and testing write cost at scale.

**Acceptance:** interrupted index work resumes safely; incremental and full rebuild yield equivalent authorized results/history; reindex does not read payload bytes just to browse; malformed checkpoints cannot silently omit committed records. Record startup and query measurements on generated scale fixtures.

**Limit:** do not trade recovery correctness for speed; final hardware performance gate is R02.

## Domain processing and shared audio

### D01 — Reuse inventory and independent fixture plan

**Gate · Dependencies:** none. **Owner:** shared-library/domain integration design.

**Status (2026-09-09): complete; included in the Wave 1 delivery.** [Reuse map](decisions/reuse-map.md) pins each inspected checkout SHA/dirty state and qualifies each hashing, normalization, CUE/CHD, Redump, redumper, disc-ID, AccurateRip, decoding, seek, resampling, queue, DSP and output operation by byte/coordinate domain, dependency, consumer, target and independent fixture. D02-D05 own extraction and outstanding qualification; no sibling files were changed.

Inspect sibling manifests/source/instructions and record exact algorithms, crate APIs, license/provenance, optional dependencies and intended extraction destinations in `docs/decisions/reuse-map.md`. Starting candidates: `../junk-libs/junk-libs-disc`, Retro Junk platform/disc/DAT/catalog crates, and `../trash-amp/crates/amp-audio`/`amp-core`. Locate Phono Junk before assuming its path or API. Inventory supported analyzer operations individually.

**Acceptance:** name affected consumers/check commands and independent fixtures for normalized hashes, disc tracks, AccurateRip offsets and audio PCM. Each operation is supported, unavailable or unqualified with a reason. Document how real-dump oracles can be supplied legally without committing payloads. Assign extraction to D02/D03 and verification to D04/D05.

**Limit:** read-only inventory/design; no wholesale sibling app dependency or guessed provider implementation.

### D02 — Extract format/hash/disc verification primitives

**Status (2026-09-10): D02.1 implemented/reviewed; D02.2 is next.** The
documentation-only drift gate reconfirmed exact sibling SHAs, upstream divergence,
dirty paths, licenses, and fixture/tool availability without changing sibling code.
Retro source redistribution remains blocked by its placeholder MIT attribution;
the Phono owned-dump/live-provider oracle remains externally gated. No shared API
or compiling scaffold was added. D02.2 may now consolidate the generic reader/hash
consumer path while preserving all recorded user changes.

**Review · Dependencies:** D01. **Owner:** independently consumable junk-libs crates and affected sibling consumers.

Execute reviewed narrow extractions of parsers, normalized hashing, disc identity and verification primitives; keep I/O backends optional. Migrate existing consumers in separate changes from Collector integration. Resolve sibling write permissions through the environment when required; do not copy algorithms into Collector to evade a blocked extraction.

**Acceptance:** old/new results match independent vectors; affected Retro/Phono/Junk consumers build and pass relevant tests; Cargo manifests name specific crates; no UI/storage/scanning dependency enters the shared library graph.

**Limit:** no Collector providers or archive migration in the extraction commit.

### D03 — Extract reusable audio services and reader contract

**Review · Dependencies:** D01, A03. **Owner:** shared audio crates; collector application media-reader port; Trash Amp adapters.

Define capability-based authorized media readers and extract decoding, integer-sample seeking, queue/DSP/playback services from app-specific orchestration. Specify PCM format/track-boundary semantics and device adapter ownership; keep filesystem and HTTP readers behind adapters.

**Acceptance:** independent sample/seek vectors and affected Trash Amp checks pass; services run without a GUI; queues/DSP do not depend on Collector storage; format/device features can be disabled. Unauthorized readers cannot be created through a frontend bypass.

**Limit:** no Slint UI, transcoding jobs or Trash Amp server provider yet.

### D04 — Game analyzer and DAT verification integration

**Review · Dependencies:** D02, A03, P02, P03. **Owner:** game module handlers; infrastructure catalog/provider adapters.

Integrate qualified game operations through host services; ingest versioned DAT/catalog snapshots and append source-bound structure/normalized-hash/catalog-match observations. Populate a supported-operation matrix with fixture provenance and limits.

**Acceptance:** independent fixtures cover each advertised operation; modified inputs produce historical rather than current success evidence; missing/multiple catalog matches are distinct; unavailable optional tools leave preservation operational. No whole-archive scanning/persistence code imported from Retro Junk.

**Limit:** unsupported formats remain explicitly unavailable; no emulator or ripping support.

### D05 — Music CD structural and AccurateRip verification

**Review · Dependencies:** D02, A03, P02, P03. **Owner:** music handlers, verification primitives and provider adapters.

Support reviewed raw redumper and BIN/CUE track models, AccurateRip IDs/checksums/offset treatment and source-bound results. Define mixed-mode/multi-disc and unsupported-layout limitations from D01 evidence.

**Acceptance:** track indices/boundaries and offset behavior match independent vectors and locally supplied real-dump oracles; no-match, unavailable and incorrect results differ; provider/catalog versions and exact inputs are retained. MusicBrainz metadata cannot set an audio-verification outcome.

**Limit:** no reconstruction success claims without an independent reconstruction oracle; unavailable cases remain explicit.

### D06 — Metadata resolution, providers and review

**Routine integration with domain review · Dependencies:** A03, A04, P02, P03, U04. **Owner:** application resolver, infrastructure MusicBrainz/CAA and qualified game providers, web review UI.

Persist candidate alternatives/raw responses; rank strong identity before language, prefer English display while retaining original text, and provide explicit manual selection/override controls. Fetch artwork through bounded provider adapters into the asset pipeline. Define provider rate/cache/retry policy in adapters.

**Acceptance:** ambiguous fixtures produce useful tentative titles/art; outages and refreshes preserve manual corrections; provider disagreement retains provenance; foreign-language text displays; artwork uncertainty and absence remain distinct. Private fields never enter external provider requests unnecessarily.

**Limit:** no price exchange; metadata is not verification. Live-provider checks supplement reproducible recorded fixtures.

### D07 — FLAC/Opus derivative jobs

**Review · Dependencies:** D03, D05, P02, A04. **Owner:** audio transforms, application derivative jobs, store representations.

Implement immutable source/profile identity, FLAC default and configurable bounded Opus profiles, conversion provenance, partial-output staging and derivative validation. Expose availability and range readers through services.

**Acceptance:** decoded FLAC matches independent source PCM exactly; Opus settings and lossy status are recorded; seek/track boundaries are correct; failed/cancelled/retried jobs never replace masters or advertise partial output. Missing optional encoders do not prevent preservation or original downloads.

**Limit:** no lossy result presented as archival verification or as lossless PCM validation.

### C01 — Persistent native server connections and trusted transport

**Review · Dependencies:** S03, A04. **Owner:** collector-client services and platform credential adapters. **Start:** client `Pairing`, `Endpoint`, trust/route tests.

Persist multiple server connections, authenticated endpoint configuration and enrolled trust, with secure credential storage. Implement actual authorized queries/ranged readers with no credential-bearing cross-origin redirects. Keep server identity independent of path/hostname.

**Acceptance:** paired alternate endpoints merge only after authenticated identity verification; unrelated server IDs/keys do not merge; restart retains connections; revocation rejects future requests; an enrolled direct connection works with no public route or Internet.

**Limit:** route policy structs alone are not completion; automatic discovery/handover is C02.

### C02 — Discovery, health and real transfer failover

**Review · Dependencies:** C01, S04. **Owner:** client endpoint services; authenticated server discovery/configuration.

Add mDNS candidate discovery, explicit tailnet/public endpoints, authenticated identity checks, reachability/latency probes and LAN preference with hysteresis. Resume ranged transfers across verified routes using immutable representation identity.

**Acceptance:** spoofed discovery never receives credentials; LAN wins when healthy; lost LAN resumes over another enrolled route without mixing versions; restored LAN avoids flapping; stale endpoints and changed keys fail safely. Record actual network tests, including tailnet names rather than assuming .local spans tailnets.

**Limit:** no reliance on public Internet for already-paired LAN use, and no source-IP authorization shortcut.

### C03 — Slint macOS shell and collection browsing

**Routine · Dependencies:** C01, A03, A04. **Owner:** new native presentation crate over client services.

Create the actual desktop app with connection selection, enrollment UI, paginated browsing/search, generic module fallback, item/evidence details and art/text rows. Use bounded lazy image loading and platform credential adapters.

**Acceptance:** two servers remain distinct unless verified as one; game/CD rows and search show art or accessible placeholders; foreign text and keyboard navigation work; unavailable modules remain browsable. Validate an actual macOS build/run, not only shared Rust tests.

**Limit:** no UI-owned provider, database or authorization logic; playback/offline work follows.

### C04 — Native music playback and queue UI

**Routine integration with audio review · Dependencies:** C03, D03, D07. **Owner:** client playback service plus Slint controls.

Connect authorized media readers to shared decoding/queue/DSP, implementing play/pause/seek, track transitions and clear errors. Keep sample positions below presentation boundaries and avoid full-file buffering.

**Acceptance:** tested track transitions/seeks match reference audio; queue changes survive relevant UI lifecycle; network/auth failures are visible; macOS device playback is manually qualified with recorded hardware/settings.

**Limit:** no video/emulation; a browser audio element is not native playback acceptance.

### C05 — Persistent offline downloads and artwork

**Review · Dependencies:** C03, D07, C02. **Owner:** client download/cache services and Slint download controls.

Implement durable resumable downloads, immutable cache keys, FLAC-first/Opus selection, pinned albums, budgets/eviction and downloaded metadata/art/evidence summaries. Handle restart, partial files and source revision changes.

**Acceptance:** stop/restart and route changes resume without corruption; pinned albums survive eviction; equal local IDs from different servers do not collide; disconnected browse/search/playback includes artwork/text and evidence summaries. Offline edits are unavailable. Explain that revocation cannot recall disconnected copies.

**Limit:** no DRM, replica collection editing or cross-server cache deduplication.

### C06 — Personal activity sync and HTTP events

**Review · Dependencies:** A04, P02, C01. **Owner:** application private activity/event services and client reconciliation; presentation consumers.

Specify then implement favorites/playback-event identities, deduplication and conflict rules, authenticated event delivery and polling fallback. Bound cursor retention and reconnect catch-up. Wire into native UI after C03/C04/C05 are available.

**Acceptance:** repeated/offline activity sync converges without duplicate counts or lost favorites under defined conflicts; device clocks do not silently determine security/order; events cannot leak another collection/user; expired cursors recover through polling. Collection edits still require connectivity.

**Limit:** this is personal activity sync, not archive replication. Record conflict semantics before coding.

### C07 — Trash Amp Collector provider

**Routine integration with sibling review · Dependencies:** D03, C01, D07. **Owner:** separate Trash Amp provider adapter using shared client/audio services.

Add server enrollment, music browsing and authorized playback through an adapter while preserving standalone local-player use. Keep Collector inventory/admin flow out of Trash Amp.

**Acceptance:** affected sibling tests pass; local-only use needs no Collector; server music playback works with revocation/error behavior; shared queue/decoding/DSP is used without a forked copy.

**Limit:** product merger or replacement of Trash Amp is not required by this ticket.

## Migration and release validation

### W2.R — Wave 2 integration and independent review

**Review · Dependencies:** P01.6, S03.3, U02.3. **Owner:** cross-layer validation and documentation.

Run the approved Wave 2 migration, attack, restart, browser, secret-scan, and independent-review matrix; reconcile architecture, implementation status, validation, security decisions, and API migration documentation. See [the bounded W2.R contract](wave-2-plan.md#u02--account-profile-and-grant-administration).

**Status (2026-09-10): complete with review corrections; included in Wave 2.** The independent source review found seven issues, now corrected: interrupted staging/journal recovery, disable/re-enable credential resurrection, misplaced recovery-code revocation, stale passkey snapshot rebasing, receipt/effect crash coordination, grant revision reuse, and storage-owned security policy. Permanent tests cover these behaviors, the 19-family process-kill matrix, and the real WebAuthn browser-handler race. Both normal and local-setup Chromium smoke flows pass using temporary servers/profiles outside the sandbox. Target-Linux termination/filesystem and hardware/browser/platform qualifications remain external release gates; local process-kill tests are not power-loss/NAS certification.

### R01 — One-way Retro Junk archive migration

**Review · Dependencies:** D01, A02, A03, P03, P05. **Owner:** infrastructure legacy adapter calling import/application services.

Inspect actual source archive versions and map IDs, metadata, original payloads and historical evidence into the reviewed schema. Provide dry-run inventory, conflict/unmappable-field reporting, checkpoints and idempotent restart. Preserve legacy IDs where compatible; explicitly retain source identity mappings where conversion is required.

**Acceptance:** independent source fixtures migrate twice without duplicates; original files/manifests remain byte-for-byte unchanged; historical evidence retains provenance and is not upgraded to fresh verification; failed records can resume; unknown data and manual choices survive.

**Limit:** one-way only; do not invoke a legacy scanner or directly write Collector tables around transaction services.

### R02 — Recorded scale and deployment qualification

**Review · Dependencies:** P06, D04, D05, D07, C02, S02. **Owner:** release tooling and infrastructure.

Add reproducible dataset/load generators, benchmark instructions and a deployment matrix. Run 100k items/1m file-evidence records with five users, recording host specs, dataset mix, warmup, query types, latency distribution and failures. Validate binary/container, explicit mounts, non-root operation, local SQLite/NAS payloads, public tunnel/private tailnet and unavailable tools.

**Acceptance:** warm paginated browse/search below 500 ms p95 on recorded four-core/16-GB/local-SSD hardware; no cross-user leak under load; actual deployment evidence for each matrix cell. Container mounts must include separate archive/state/scratch semantics. Environment limitations remain visible release blockers.

**Limit:** no fabricated benchmark timings or treating compile-only container validation as execution. Split benchmark and deployment matrix into child tickets for separate operators.

### R03 — First-release acceptance audit

**Review · Dependencies:** every first-release ticket above, including U01 and R01/R02. **Owner:** cross-layer release review.

Reconcile requirements against implementation and recorded evidence, review exposure/auth/upload/private-field failure cases, and execute end-to-end real-client acceptance. Add portable browser smoke coverage to CI where practical and document manual gates separately.

**Acceptance:** all release-requirements.md bullets link to tests or reproducible manual reports; native pairing then Internet-disconnected LAN playback/download succeeds; rotation/discovery/failover, independent game/CD/FLAC oracles and real password-manager/hardware gates have results. No unfinished feature is marked available. Remaining defects receive tickets rather than being hidden in a successful summary.

**Limit:** do not declare release complete while required external-environment gates are unrun.

## Coverage and deferred scope

| Required outcome | Owning tickets |
|---|---|
| Typed entities, clean service boundaries, unknown modules and migrations | A01–A04, P01, P05 |
| Inventory without dumps, copies/discs, loans/disposal and private attachments | A02, U02–U04, P01 |
| Complete packages, source retention, review, retries and durable jobs | P01–P04 |
| Evidence provenance, normalized hashes, AccurateRip and independent oracles | D01, D02, D04, D05, R03 |
| Tentative metadata, alternatives, English preference and manual overrides | D06 |
| Artwork/text rows, search, accessibility, lazy loading, offline art | U04, C03, C05 |
| Readers, integer samples, PCM-validated FLAC, optional Opus, shared Trash Amp audio | D03, D07, C04, C07 |
| Exposure combinations, roles, credential recovery and no proxy bypass | S01–S03, U02, R03 |
| Encrypted direct LAN, identity merge, key rotation, discovery and failover | S03, S04, C01, C02, R03 |
| Persistent offline cache, pinning, budgets and personal activity sync | C05, C06 |
| Archive history, explicit access recovery, backup and one-way legacy import | P01, P05, R01 |
| Generated API contracts, idempotency, limits, events and polling | A04, P02, C06 |
| Performance, native/container/tunnel/tailnet/NAS deployments | P06, R02, R03 |

Follow-on tickets must be planned separately: SwiftUI iOS/UniFFI, PriceCharting exchange after API research, runtime plugin isolation, replication, video, emulators and ripping hardware. Cross-collection byte deduplication is not an initial requirement. Do not invent these as prerequisites for a first-release ticket.

## Copyable assignment

> Implement ticket **[ID]** from `docs/handoff-backlog.md`. Read its dependencies, `AGENTS.md`, `docs/architecture.md`, and `docs/release-requirements.md` first. Inspect current code rather than assuming the baseline descriptions are still exact. Work only within the ticket; preserve local user data and configuration. If a required contract is unresolved, produce the concrete design decision and tests first rather than guessing. Complete acceptance checks, update the ticket/status/validation documentation with results and limitations, and report changed files plus a user-verifiable path. Do not implement adjacent tickets or mark unavailable external validation as passed.

## Wave 1 review follow-up

The review found defects despite a passing original suite. The working tree now includes regression fixes for canonical capture replay, cross-collection updates, incomplete migration writes, nested identity uniqueness, transitive capture associations and capability dispatch. Adjacent fixes cover stale capture views, legacy reassociation reads, file-ownership restrictions, component projection cleanup, explicit schema transformations and language-tag parsing. See validation.md for the current checks. These are corrections to A01/A02/A03/U01 patterns, not claims that later security, transaction or client waves are implemented.
