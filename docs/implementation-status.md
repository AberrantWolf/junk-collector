# Implementation status

This repository is an executable architecture/server foundation, not the complete first release described in the accepted plan. Capability reporting must stay honest as remaining modules land.

Use the [handoff backlog](handoff-backlog.md) for ordered, scoped implementation tickets and the [release requirements](release-requirements.md) for the repository-local target specification. The gap list below is a summary, not a substitute for ticket acceptance criteria.

## Implemented

- Six-crate Rust workspace with a typed collection graph, compiled-module registry, application authorization boundary and a schema-2 third-module contract test.
- Foundation types now include collection-owned Work/Edition records, exclusive Item identity links, ordered specimen Components with optional data-bearing Carriers, collection-level Capture associations, typed asset subjects, language/currency/coordinate validation, canonical digests, and graph/representation validation.
- Archive schema 2 and a migration-only durable checkpoint preserve schema-1 records unchanged while emitting typed Work, Edition, Item, and Capture revisions, deterministic Components, explicit legacy payload locators, and rebuildable Works/Editions/Captures/Components projections. A committed synthetic baseline fixture covers two revisions, duplicate copies, two discs, legacy case/booklet carriers, files/assets, evidence, manual metadata, and lexically ordered unknown JSON.
- Lossless raw-JSON envelopes preserve unavailable module/provider bodies. Compiled modules have namespaced capabilities, exact schema validation, checked adjacent migrations with explicit source-preserving transformation policy, host-owned representation/job ports, and safe presentation contributions. HTTP item reads add post-redaction presentation; unavailable data displays generically and is not executable.
- A04.1 collection/item/Evidence reads now cross a Principal-free mechanism port and application-owned authorization. Explicit response DTOs expose effective permissions, structurally omit viewer-private inventory/metadata/evidence/raw-module fields, and never expose legacy Capture archive locators. Server-administrator status still grants no collection access. The deprecated Wave 1 command adapter preserves protected fields and delegates bounded edits.
- A04.2 moves item/entity commands, media/assets/activity, imports/jobs, administration, account authentication, and passkey persistence behind application-owned policy ports. Item HTTP writes use a deny-unknown bounded command; Store adapters receive identities/effects rather than Principal. Opaque readers preserve range streaming without exposing filesystem paths. The architecture check rejects direct HTTP `state.store` calls.
- A04.3 adds private SQLite schema-3 command receipts and mandatory application command contexts. User/API-major/operation-scoped idempotency keys bind to typed-request digests; completed successes replay across restart, concurrent claims fail distinctly, changed payloads conflict, and failed validation/authorization/revision attempts do not consume keys. HTTP and React transport keys; authentication ceremonies, logout, and offset-governed chunks remain explicit exceptions. A04.5 closes the effect/receipt crash window using atomic private receipts or prepared filesystem outcomes, with process-termination replay coverage across all mutation families.
- A04.4 composes runtime APIs from annotated route groups and checks generated OpenAPI 3.1 plus TypeScript artifacts for drift. React consumes generated response/command types, every request body publishes its enforced limit, and no Swagger/schema route is exposed. A deprecated Wave 1 item envelope may edit only after its server-owned fields match canonical state exactly. A04 is complete; P01 owns crash coordination and archive schema 3.
- P01.1 defines archive-schema-3 canonical Item, revisioned Asset, standalone Evidence, tombstone, legacy-locator, and transaction-envelope records. A committed synthetic Wave 1 schema-2 archive/private-v2 fixture preserves originals, artwork, revisions, raw JSON, accounts, credentials, grants, activity, and jobs; opening it leaves every pre-existing archive byte unchanged and projects the newer standalone Capture.
- P01.2 adds a mechanism-only durable publication journal with synced monotonic phase records, safe archive-relative paths, artifact hashes, actual-file recovery classification, retained unknown staging, and fail-closed handling of corrupt known transactions. Private state advances incrementally to schema v4 for coordinated recovery records. Named in-process fault points are covered with real temporary archives.
- P01.3 additively migrates schema-2 archives to deterministic schema-3 sidecars/checkpoints. Canonical Items omit embedded Capture/Asset snapshots; standalone Evidence and revisioned collection Assets preserve IDs/content and legacy payload hashes without moving old bytes. Capture archive locators have moved out of the portable core into adapter records/projections. Catalog projection schema v3 rebuilds these rows and keeps newer standalone Capture revisions authoritative. Current mutation writers become journaled schema-3 publishers in P01.4/P01.5.
- P01.4 adopts the durable journal for normal Item, Work, Edition, Capture, standalone Evidence, and Asset-metadata publication. Restart aborts pre-commit metadata and reindexes committed revisions before cleaning their journals. Evidence IDs are immutable/content-checked, reverification appends bodies once, and Item history carries only Evidence references.
- P01.5 journals import payload bytes and the Capture revision under a stable upload transaction. Controlled faults and actual killed subprocesses at every phase recover to one complete Capture/Representation/Evidence result or a retained retry, with upload scratch preserved.
- P01.6 coordinates collection archive creation with its intended private owner grant. Startup holds the writer lock through private migration/recovery, archive migration, and projection rebuild; deterministic command-bound Collection IDs make interrupted creation safely retryable. Archive-only restoration still creates no grants. P01's target-Linux/filesystem qualification remains an external review gate, not a claimed power-loss certification.
- S02 implements separate revisioned account, browser-session, passkey, recovery, and audit models in private SQLite schema v5. Exact-Origin enforcement, five-minute recent authentication, scoped deterministic rate limits, ten-code 160-bit recovery batches, atomic password replacement/session revocation, passkey counter CAS, last-enabled-administrator/owner guards, default reassignment, and stopped-server explicit-UserId recovery are covered with real SQLite and deterministic concurrency tests.
- S03 advances private state to v6 with expiring targeted enrollment and separately hashed/revocable native credentials. Same-user recent browser approval, cancellation/expiry/replay protection, one-time issuance, header-independent native authentication, and native-device inventory/revocation are implemented. `collector-client` owns an injectable secure-storage port, refuses redirects/cross-origin credential delivery, validates enrolled server identity/certificate material, and completes the LAN TLS integration through real enrollment.
- U02 provides revisioned account/profile/default-user and collection-grant APIs plus generated-client administration, owner-grant, profile, recovery-code, session, passkey, and paired-device controls. UI affordances use server-computed `EffectivePermissions`; direct service/HTTP authorization remains authoritative. Disabled accounts are preserved rather than deleted.
- Private SQLite advances through explicit migrations and stores a durable default UserId selected once from legacy first-row behavior. First-account creation sets it transactionally; later accounts do not change it.
- The S01 threat/recovery contract and D01 pinned reuse/fixture map are complete design gates. S02/S03 now implement the credential/recovery/enrollment portions; authenticated TLS rotation remains S04, and D02-D05 runtime work remains outstanding.
- Separate private operational database; collection roles, private field filtering, revisioned accounts/grants, password/passkey recovery, browser-session inventory, and revocable paired native credentials.
- Separate public/private HTTP routers and direct LAN TLS. Public curation/admin independently configured; disabled APIs and management bundles are absent.
- WebAuthn passkey registration/login backend and browser integration when an HTTPS authentication origin is configured. Registration and passwordless login pass a Chromium virtual-authenticator end-to-end test; real password-manager/hardware acceptance remains a release gate.
- Web collection browsing/search with artwork alongside game/CD rows, explicit placeholders and identity states, inventory creation/editing, wishlist/owned-copy entry, original cover retention and bounded thumbnails.
- Resumable package upload, destination verification, stable retry identity, original downloads with byte ranges, integrity rechecks and persisted job records.
- Client trust construction, route selection/hysteresis and cache eviction policy. A TLS integration test enrolls through same-user browser approval, persists a native credential through the storage port, proves authenticated ranged downloads over loopback LAN without any public route, and rejects an unrelated certificate or credential-bearing cross-origin route.
- Configured server inbox inventory/import API using the same source-retaining staging and evidence path; background upload errors persist as failed jobs.
- P02.1/P02.2 advance private state to schema v8 and implement typed durable jobs
  plus a local executable scheduler. Transactional FIFO claiming enforces a
  two-job global bound and one active job per user; 30-second leases,
  five-second heartbeats, claim-token fencing, restart takeover, and bounded
  progress persistence are permanent-test covered. Two server worker loops run
  integrity verification as the first consumer. Its deterministic Evidence
  identity makes recovery after archive publication idempotent, and the existing
  verification HTTP action now returns the actual queued Job identity.
- P02.3 separates incomplete upload staging from executable queue state and moves
  browser upload publication plus configured-inbox copying/publication onto the
  same durable workers. Queued and bounded-unit running cancellation retain source
  material; a publication gate makes cancellation pre-commit or stably too late
  after `RevisionCommitted`, with committed recovery completed exactly once. Failed
  retries reuse the Job, Capture, and Representation identities, increment attempt,
  retain source bytes, and preserve prior failure details. The process-local job
  semaphore has been removed.
- P02.4 replaces the legacy job-list HTTP shape with cursor `JobView` pages and
  submitter-scoped detail, cancellation, and retry routes. Views provide bounded
  progress, attempt, safe diagnostics, timestamps, and actionable flags while
  excluding stored requests, prior-failure bodies, worker/lease/claim data, and
  other users' jobs. Mutations require idempotency keys. P02 is locally complete;
  target-Linux filesystem/process behavior and independent review remain external.
- U03.1 makes specimen lifecycle state server-owned. The bounded Item-save
  command omits specimens, while its Wave 1 adapter accepts only an exact
  canonical snapshot. Focused application commands and curator-only HTTP routes
  create/update specimens and ordered components, remove unreferenced components,
  start and explicitly return loans, and record disposal without deletion.
  Nested identities are deterministic per command; returned loans, captures,
  assets, evidence references, and payloads survive ordinary Item revisions.
  The previous broad-save “Add an owned physical copy” shortcut is removed.
- U03.2 provides the backed copy-level lifecycle editor: condition, ownership,
  possession, acquisition, private notes, ordered components/carriers, lending,
  explicit return with retained history, and disposal. Lifecycle drafts remain
  in React component state across stale Item revisions; the conflict panel names
  both revisions and offers reload/discard or manual reapply. The layout retains
  the existing archival-workbench visual language and is keyboard/mobile tested.
- U03.3 verifies viewer detail and browse DTOs structurally omit acquisition,
  private notes, contacts, and loan history while curator DTOs retain them.
  Focused lifecycle routes are composed only on curation-enabled listeners. Two
  independent copies, returned history, disposal, Capture/Asset relationships,
  and stored artwork bytes survive reindex/restart. U03 is locally complete;
  independent review remains external.
- U04.1 adds standalone, collection-scoped Asset creation for PNG/JPEG/WebP
  Covers, Photos, and Scans attached to Work, Edition, Item, Specimen, Component,
  or Capture subjects. One bounded image pipeline preserves originals and emits
  PNG thumbnails. Visibility/alt-text updates and tombstones create independent
  schema-3 Asset revisions without changing Item revisions; subjects remain
  immutable, payloads/history survive tombstoning, and reindex retains standalone
  records. Documents are rejected as explicitly unavailable rather than accepted
  as a placeholder.
- U04.2 exposes revisioned `PresentedAsset` metadata and collection-scoped
  original/thumbnail resources through application-owned authorization. Viewer
  DTOs omit visibility and stored filenames/digests, while curator-only metadata
  and both byte representations are denied before storage is opened. The old Item
  Asset URL is a compatibility adapter over the same policy. Edition-linked Items
  inherit Work artwork, and a role-aware server `primary_artwork` prefers visible
  module selection before deterministic newest-Cover fallback. Asset mutations
  remain curation-listener composed.
- U04.3 replaces the one-purpose cover shortcut with a backed supporting-image
  workbench. Curators choose the attachment subject, Cover/Photo/Scan role,
  visibility, alternative text, and image; receive a local preview; inspect
  lazy-loaded stored thumbnails; revise metadata; and archive with explicit
  retained-history guidance. Browse/detail consume server-selected artwork and
  visibly separate missing artwork from identity uncertainty. Keyboard flow,
  focus restoration, reduced motion, viewer filtering, and responsive layout are
  browser tested. U04 is locally complete; document uploads, offline artwork, and
  independent review remain outstanding.
- P03.1 advances private state to schema v9 and persists provisional Import
  workflows under an `ImportId` distinct from Jobs and command transactions.
  Records retain a safe source summary, bounded file inventory with offsets and
  digests, preallocated Capture/Representation IDs, optional physical association
  and expected Item revision, lossless candidate observations, limitations, review
  decision, analysis Job link, and phase. Optimistic updates preserve immutable
  source/identity fields across restart, and the model permits a fully staged,
  reviewed, unidentified and unattached published state. No private filesystem
  locator is part of the model.
- P03.2 gives browser and configured-inbox sources one resumable staging path.
  Browser chunks enforce durable offsets, bounded exact replay, and declared digest
  checks. Inbox staging executes as a P02 Job, resumes from persisted offsets, and
  rechecks source entries/inventory for symlinks, special files, path escape, case
  collisions, size, metadata, and content changes. Curators see configured labels
  and direct package names only; private roots remain solely in the bounded worker
  request and are absent from `ImportView` and `JobView`. Both sources converge on
  the same digested `analyzing` state after staging.
- P03.3 submits optional Import analysis as a retryable/cancellable durable Job
  through the A03 module capability registry. Valid outputs persist lossless bounded
  candidates and limitations; unavailable built-in game/music analyzers proceed to
  review with an explicit D04/D05 limitation and no fabricated candidate. Focused
  review commands select a candidate or remain unidentified, validate an existing
  Item/Specimen/Component/Carrier chain or leave the Capture unattached, and retain
  the user's selection in `awaiting_review` when the expected Item revision is stale.
  Manual review records that no analyzer was selected.
- P03.4 publishes through a distinct durable P02 Job and the P01 journal using the
  Import's preallocated Capture and Representation identities. It re-reads staged
  bytes, publishes integrity Evidence, supports a canonical physical association or
  an unattached Capture without fabricated inventory, and completes committed work
  exactly once after restart. Cancellation retains source scratch before the Capture
  revision commit and returns a stable too-late conflict afterward; stale associated
  Item revisions preserve review state. The legacy browser-upload and administrator
  inbox endpoints delegate to the same Import workflow and distinct execution Jobs,
  with a bounded fallback only for pre-v9 in-flight uploads. P03 is locally complete.
- Optional private TLS management listener bound to an explicit interface address.
- WebUI onboarding distinguishes client-only and private management surfaces, offers first-collection/first-item next steps, preserves structured API error codes, and separates wrong-origin, expired-session and role-denial guidance without weakening listener or authorization policy.

## Required before the planned first release

1. Implement authenticated certificate rotation (S04) and complete the external security qualifications: real password-manager/platform and hardware passkeys, browser-trusted remote HTTPS, native platform secure storage, and independent review.
2. Build the P04 configured inbox/import-review and durable-job UI over the locally
   complete P02/P03 APIs. The current legacy upload and administrator inbox routes
   are compatibility adapters scheduled for API-major removal. Document attachments,
   event streaming, and offline artwork remain unimplemented.
3. Implement analyzer/provider adapters using extracted Retro Junk/Phono Junk logic, DAT/catalog ingestion, MusicBrainz/CAA enrichment and independent real-dump verification oracles. No catalog verification is currently claimed.
4. Add source-bound FLAC and configurable Opus derivative services and validated PCM transforms. Browser preview currently only plays already-playable original formats.
5. Implement one-way Retro Junk archive migration, complete export/backup/recovery commands, recovery UI and scalable incremental reindexing.
6. Deliver an actual Slint macOS app with shared playback, browsing, artwork, pairing, download persistence and personal activity sync. Extract/share audio with Trash Amp and add its server provider. Current client crate is not a desktop application.
7. Add authenticated endpoint discovery, mDNS, automatic health checks and actual transfer failover. Extend the tested enrolled LAN trust path to automatic discovery and full native-client failover.
8. Add event delivery/polling plus their UI/integration tests under P02/C06. A04 application boundaries, command idempotency, generated OpenAPI/TypeScript contracts, body limits, and Wave 1 command migration are complete.
9. Execute the 100k-item/1m-record benchmark on a recorded 4-core/16GB SSD host; validate native/container/tunnel/NAS deployments and load behavior.

## Follow-on releases

SwiftUI iOS through UniFFI; PriceCharting collection exchange; runtime plugins; replication; video/emulation; ripping hardware. These remain explicitly deferred by the accepted plan.

## Local onboarding

`setup` creates or repairs configuration with a hidden, confirmed terminal password prompt, preserves existing accounts/data/TLS identities, and prints the exact private localhost URL and first collection steps. `doctor` checks configuration without opening the archive. The WebUI distinguishes browser-origin failures, recent-authentication requirements, expired/revoked sessions and genuine role denials, and client-only addresses point back to the printed private management address. Login checks that the session cookie was retained. Single-user login resolves the durable default UserId and hides account switching; administrators can explicitly reassign the default through the revisioned account UI. Local HTTP onboarding is Chromium-tested; remote and other unqualified browsers require separately qualified trusted HTTPS. This is CLI guidance, not yet a comprehensive interactive network/proxy wizard.

## Wave 1 review fixes

Capture revision/ownership regressions, incomplete migration recovery, graph identity/ancestry gaps and capability dispatch are corrected with retained regression tests. Capture file/locator changes cannot bypass import services; legacy payloads remain reachable after reassociation. Reindex hydrates canonical captures and clears removed component projections. Schema transformations retain exact source envelopes, and language tags use a syntax parser. P01 transaction/evidence recovery and the S02/S03 security runtime now have permanent restart, concurrency, and attack regressions; their external review/hardware qualifications remain outstanding.

## Wave 2 review fixes

Private schema v7 adds prepared command outcomes and grant revision counters. Scoped effect adapters commit private responses atomically or tie exact archive responses to immutable publication markers. The process-kill matrix covers 19 mutation families and early archive commits; startup reconciles upload bookkeeping and artwork cleanup before returning. Prepared partial files and recognized journal temporary files recover, while committed corruption remains fatal.

Application-owned security snapshots preserve transactional authority guards. Disabling accounts revokes browser/native access and pending enrollments permanently; replacing recovery codes preserves paired devices and pending enrollment. Grant removal never resets its revision history. Passkey ceremonies finalize against their original verified credential snapshot; the virtual-authenticator browser regression rejects stale ceremonies through HTTP.

The review findings and A04.5 are closed. Linux filesystem/power-loss/NAS qualification, real authenticators, trusted remote-browser HTTPS, and native platform secure storage remain explicitly unqualified; these are not claims of the local tests.
