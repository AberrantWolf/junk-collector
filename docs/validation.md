# Validation recorded for the foundation

# Wave 3 D02.1 drift/provenance gate — 2026-09-10

- Re-read the four sibling checkouts at full object IDs and compared their branch,
  upstream divergence, and exact dirty paths with D01. No drift was found and no
  sibling file was modified.
- Confirmed the Phono manifest's exact junk-libs revision exists in the local object
  store. Confirmed committed/generated ARver TOC/CUE, CRC, dBAR and offset vectors,
  Retro's generated archive fixture, and `/opt/homebrew/bin/chdman` are available.
- Failure/qualification probes: Retro's MIT attribution is still the literal
  `[year] [fullname]` placeholder; the Redumper test still requires
  `JUNK_LIBS_REDUMPER_AUDIO_PREFIX` plus a live AccurateRip request; no private dump
  was supplied and no CHD vector was executed.
- Check: `python3 scripts/check_architecture.py`.
- **Implemented:** the reviewable D02.1 documentation/provenance gate only.
  **Not implemented:** D02.2-D02.5 shared APIs, algorithm extraction, consumer
  migrations, and Collector integration. **Externally unqualified:** private
  Redumper/live-provider results and future CHD vectors; Retro source redistribution
  is blocked until attribution is resolved.

## Wave 3 P04.3 browser/security and root review — 2026-09-10

- The expanded Chromium flow retries a real stale-publication Job, observes the same
  identity at attempt 2 while queued, cancels it through “Cancel job,” reaches durable
  `cancelled`, and verifies focus returns to “Refresh jobs.” P02.4 Rust tests retain
  the complete cancellation/retry boundary and idempotency matrix.
- A second Import selects the exact Reference copy / Disc 1 / carrier chain. A real
  concurrent Item edit makes review stale; the server preserves the submitted review,
  the UI fetches current Import/Item revisions, keeps all three selectors, and requires
  an explicit second “Review import” to apply against the current revision.
- The reviewed Import handle survives full page reload in browser storage and is
  re-authorized/refetched when curation reopens. Simulated offline refresh produces a
  visible alert; restoring the network fires immediate Import/Job refresh and clears
  it. Polling does not own or replace local review choices.
- After switching to a separately authenticated viewer, direct reads of the owner's
  saved Import and Job return 404 and direct Import creation in the viewer-only
  collection returns 403. The built-in analyzer remains visibly limited by D04/D05;
  no analyzer success or verification badge is rendered.
- Radio-group keyboard navigation, named forms/regions, one live progress status,
  post-action focus restoration, `prefers-reduced-motion`, and 390px no-overflow
  assertions pass. `import-mobile.png` and `jobs-mobile.png` are captured for visual
  critique; the accession docket remains a single archival ledger rather than a card
  dashboard.
- Checks: architecture/documentation links, generated API drift, TypeScript check/
  build, rebuilt embedded-assets server, and both normal HTTPS and `LOCAL_SETUP=1`
  Chromium suites pass. The 138 Rust tests remain unchanged from P03.4. P04 root
  acceptance review passes locally.
- **Not implemented:** C06 event streaming, D04/D05 analyzers, document attachments,
  and offline artwork. **Externally unqualified:** independent accessibility/security
  review and non-Chromium browser behavior.

## Wave 3 P04.2 durable Job controls — 2026-09-10

- The curation dialog's “Background work” region consumes the submitter-scoped
  cursor page and separates queued/running/cancel-requested Jobs from terminal
  history. Rows expose only `JobView` progress, attempt, safe error/limitation, and
  server-computed `cancellable`/`retryable` actions.
- The polling loop runs every two seconds only while the document is visible and a
  Job is active, backs off to 30 seconds when idle, and refreshes immediately after
  visibility restoration, browser reconnect, Import job submission/completion, or
  cancel/retry. It never writes Import form state; reconnect refreshes the durable
  Import while retaining local identity/association choices.
- Chromium observes completed Import analysis/publication history, creates an
  associated Import whose Item revision becomes stale before publication, waits for
  its real failed Job, selects “Retry job,” and observes attempt 2 on that same Job.
  P02.4's permanent Rust HTTP tests continue to cover real cancel/retry authorization,
  idempotency, and another user's Job identity.
- Checks: TypeScript check/build, rebuilt embedded-assets server, and normal HTTPS
  Chromium smoke pass. The 138 Rust tests remain unchanged from P03.4.
- **Not implemented:** P04.3's explicit offline/reconnect, running cancellation,
  stale-review, unauthorized Import/Job, focus restoration, live-announcement,
  reduced-motion, and local-listener review matrix. C06 event streaming remains
  unavailable. **Externally unqualified:** independent accessibility review.

## Wave 3 P04.1 provisional Import workspace — 2026-09-10

- The Chromium workflow opens the real curation dialog, selects the configured
  `Optical shelf`/`reference-disc` inbox without exposing its temporary root, then
  switches to browser files and stages `browser-disc.bin` through the modern Import
  endpoint. The file inventory reaches its exact byte count and SHA-256 state.
- “Review import” runs the owning music capability as a durable Job. The workbench
  presents its persisted D04/D05 limitation and no successful-looking candidate or
  verified badge. The user explicitly leaves the Capture unattached, saves an
  unidentified review, and “Publish capture” reaches the real published state.
- Forms/regions have accessible names, progress is a native progress element, and
  background phase text is an `aria-live` status. `web/test-results/import-mobile.png`
  records the published docket at 390px; visual review corrected an initially
  unloaded chunk stylesheet and then found a legible single-column inventory with
  no horizontal overflow, consistent with the archival workbench.
- Checks: TypeScript check/build, rebuilt embedded-assets server, and normal HTTPS
  Chromium smoke pass. The existing 138 Rust tests are unchanged from P03.4.
- **Not implemented:** P04.2 active/history Job controls, cancel/retry controls and
  idle/reconnect polling; P04.3 offline/conflict/authorization/accessibility matrix;
  C06 event streaming. **Externally unqualified:** independent visual/accessibility
  review.

## Wave 3 P03.4 publication, compatibility, and root review — 2026-09-10

- `unattached_import_publication_commits_preallocated_capture_payload_and_evidence_once`
  stages and reviews a real package, runs its claimed publication Job, and verifies
  the exact preallocated Capture/Representation IDs, destination digest/bytes, and a
  standalone schema-3 integrity Evidence record with no Item. Reopen observes one
  durable Capture and a `published` Import.
- `queued_import_publication_cancellation_retains_scratch_without_publishing` covers
  the queued boundary. `import_cancellation_is_too_late_after_capture_revision_commit_and_finishes_once`
  pauses the real journal at `RevisionCommitted`, proves cancellation returns the
  stable too-late error, resumes, and observes one Capture and one Evidence record.
  `published_import_reconciles_a_crash_before_job_completion` then leaves the Import
  published with its Job still claimed, reopens, and proves startup takeover completes
  that same Job without another Capture. Startup reconciliation recognizes this
  deterministic publication transaction.
- `import_publication_returns_to_review_when_the_associated_item_changes` mutates the
  selected Item after review but before worker execution. The Job fails safely, no
  Capture publishes, and the complete selection/association remains in
  `awaiting_review`. Zero-byte declarations receive the canonical empty SHA-256 and
  need no fake chunk; a conflicting declared digest is rejected.
- `legacy_upload_routes_delegate_to_imports_and_use_a_distinct_publication_job`
  exercises the old create/chunk/commit/read surface. Its compatibility handle maps
  to an Import, no same-ID Job exists, commit returns a distinct publication Job, and
  the canonical workflow publishes the attached Capture. The administrator inbox
  route likewise creates an Import plus staging Job and now requires review rather
  than bypassing the workflow. Pre-v9 in-flight Upload rows retain their old adapter.
- The HTTP convergence test publishes the modern browser Import end to end. All new
  routes remain curation-listener composed, submitter/collection scoped, and public
  DTOs contain no scratch, inbox-root, archive path, worker, or claim-token data.
- Full local gate: architecture/link/fixture checks, Rust formatting, all 138 locked
  workspace tests, warning-free all-target clippy, generated API drift, TypeScript
  check/build, and rebuilt embedded-assets server pass. P03 root acceptance review
  passes locally.
- **Not implemented:** P04 import/job UI, D04/D05 built-in byte analyzers, C06 event
  streaming, and document attachments. Legacy `/uploads` and administrator inbox
  endpoints remain compatibility adapters until an API-major removal. **Externally
  unqualified:** independent P03 review and target-Linux/filesystem termination
  behavior.

## Wave 3 P03.3 optional analysis and review — 2026-09-10

- `optional_import_analysis_persists_candidates_or_honest_unavailability` registers
  an available test module and runs its namespaced `import-analysis` operation through
  a claimed durable Job. The operation receives only preallocated identities and the
  digested inventory, and its raw candidate observation retains unknown fields/order/
  number spelling. The same scenario runs the built-in music capability and reaches
  `awaiting_review` with no candidate and the explicit D04/D05 limitation.
- `import_review_preserves_selection_on_stale_item_and_supports_unattached_review`
  updates an Item concurrently, observes a conflict, then reads back the user's
  unidentified choice, association, and old expected revision at a new Import revision.
  It also completes an unattached manual review and records the no-analyzer limitation.
  Store validation rejects absent candidate keys; application validation rejects
  cross-collection or broken physical chains.
- The HTTP convergence test now submits the built-in analysis Job, observes its honest
  limitation, and reviews the Import unidentified without any “verified” response.
  Analysis/review routes are also in the curation listener-composition matrix.
- Full local gate: architecture/link/fixture checks, Rust formatting, all 131 locked
  workspace tests, warning-free all-target clippy, generated API drift, TypeScript
  check/build, and rebuilt embedded-assets server pass.
- **Not implemented:** P03.4 publication, commit-boundary reconciliation, and legacy
  upload/inbox delegation; P04 UI; D04/D05 byte-reading built-in analyzers.
  **Externally unqualified:** independent review of module output fixtures.

## Wave 3 P03.2 source convergence — 2026-09-10

- `browser_and_inbox_sources_converge_on_the_same_durable_staging_state` stages
  identical nested package bytes through browser chunks and a real claimed inbox
  Job. Both finish as the same file inventory, offsets, SHA-256 digests, and
  `analyzing` phase; only the private Job request contains the resolved source path.
- `browser_import_resumes_from_durable_offsets_after_restart` proves create-command
  replay returns the same preallocated identities, partial offsets survive Store
  restart, gap writes conflict without mutation, and the resumed final chunk checks
  its declared digest. Exact already-accepted chunks are harmless replays.
- `changed_inbox_input_fails_the_staging_job_and_retains_resumable_import` changes
  a declared file before claim and observes a failed Job with the Import retained in
  `staging`. Per-entry canonical/symlink/type/size checks, source rehash, metadata
  comparison, and final reinventory cover mutation during copy; existing permanent
  inbox tests continue to cover symlinks and malicious paths.
- `provisional_import_routes_converge_sources_and_hide_inbox_paths_and_other_users`
  exercises the browser create/chunk/read routes and collection inbox listing/stage
  route. It confirms direct directories only, no configured root in either response,
  another user's Import ID is not found, and the queued inbox Job reaches the same
  digested state. Routes follow curation listener composition.
- Full local gate: architecture/link/fixture checks, Rust formatting, all 129 locked
  workspace tests, warning-free all-target clippy, generated API drift, TypeScript
  check/build, and rebuilt embedded-assets server pass.
- **Not implemented:** P03.3 optional analyzer dispatch, candidate/review commands,
  and stale-review preservation; P03.4 publication and upload/inbox compatibility
  delegation; P04 UI. **Externally unqualified:** independent source-security review.

## Wave 3 P03.1 durable provisional Import model — 2026-09-10

- Private schema v9 creates typed `imports` columns for ownership, collection,
  safe source summary, stable Capture/Representation identities, optional physical
  chain and expected Item revision, analysis Job, phase, revisions, and timestamps.
  Declared files, candidates, limitations, and review JSON have explicit storage
  bounds. Schema inspection confirms there is no inbox-root, source/scratch path,
  or archive-path column; local recovery accepts the new private schema.
- `schema_nine_persists_durable_imports_without_private_path_columns` creates a
  real record, closes the store, reopens it, and compares the complete serialized
  workflow. `durable_import_updates_preserve_identity_and_raw_review_observations`
  covers optimistic revision conflict, immutable source/preallocated identities,
  analysis Job association, phase movement, timestamping, and lossless unknown/raw
  candidate JSON.
- `import_validation_rejects_unsafe_or_incoherent_staging_state` probes path-shaped
  source summaries, case-colliding names, offsets beyond declared length, selection
  of an absent candidate, and publication before all bytes have valid digests.
  `durable_import_allows_published_unidentified_unattached_capture_identity` proves
  the persisted workflow does not require a fabricated Item or Specimen.
- Full local gate: architecture/link/fixture checks, Rust formatting, all 124
  locked workspace tests, and warning-free all-target clippy pass.
- **Not implemented:** P03.2 browser/inbox staging convergence and restart resume;
  P03.3 analyzer dispatch and review commands; P03.4 journal publication and legacy
  adapters; public Import DTO/routes; P04 UI. **Externally unqualified:** none for
  this local persistence child; independent P03 review follows its root checkpoint.

## Wave 3 U04.3 supporting image UI and root review — 2026-09-10

- The curation workbench exposes only implemented PNG/JPEG/WebP Cover, Photo,
  and Scan choices. Its labeled attachment selector covers the Item, linked
  Work/Edition when identified, copies, ordered parts, and Captures. A selected
  file receives a local preview before the U04.2 command executes; persisted rows
  use lazy canonical thumbnails and show attachment, visibility, alt text, and
  Asset revision.
- Chromium creates a shared Cover, observes the real stored thumbnail, edits its
  alt text and visibility to curator-only at revision 2, creates then archives a
  Photo after confirmation, and verifies focus returns to the file selector. The
  later viewer session loads neither the private thumbnail nor its metadata and
  sees the explicit “No artwork attached” state independently of “Unidentified.”
- `web/test-results/assets-mobile.png` captures the retained curator-only Cover
  row at 390px. Visual review found the image ledger legible, compact, and
  consistent with the existing archival workbench; controls remain in keyboard
  order and the page reports no horizontal overflow. All imagery is lazy-loaded,
  and the existing reduced-motion media rule covers the new surface.
- Full local gate: architecture/link/fixture checks, Rust formatting, all 120
  locked workspace tests, warning-free all-target clippy, generated API drift,
  TypeScript checking, web build, rebuilt embedded-assets server, and both normal
  HTTPS and `LOCAL_SETUP=1` private-listener Chromium suites pass. U04 root
  acceptance review passes locally.
- **Not implemented:** document attachments and C05 offline artwork. **Externally
  unqualified:** independent U04 authorization/accessibility review.

## Wave 3 U04.2 Asset DTO and authorization — 2026-09-10

- `asset_routes_filter_metadata_and_bytes_before_role_aware_artwork_selection`
  drives the real HTTP API. A viewer receives shared `PresentedAsset` metadata
  without visibility, filename, length, or digest fields and can range-read its
  original and thumbnail through both canonical and legacy paths. After a real
  metadata revision makes it curator-only, metadata/original/thumbnail and the
  legacy Item URL all deny the viewer; an unrelated user is denied and a wrong
  collection path is not found. Archival then hides every route.
- The same test attaches a Cover to a Work and proves an Edition-linked Item
  inherits it as role-aware `primary_artwork`. The U04.1 fixture registers a real
  test module whose visible selected Photo wins ahead of fallback Covers. Fallback
  ordering uses newest Asset update time and AssetId as a deterministic tie-break.
- `asset_mutations_follow_curation_listener_composition` proves create,
  metadata-update, and archive methods are 404-absent when public curation is off
  and reach authentication only when it is enabled. Generated OpenAPI/TypeScript
  contain only the working routes, including the explicit 20 MiB create limit.
- Full local gate: architecture/link/fixture checks, Rust formatting, all 120
  locked workspace tests, warning-free all-target clippy, generated API drift,
  TypeScript checking, web build, and rebuilt embedded-assets server pass.
- **Not implemented:** U04.3 attachment/metadata/archive controls, keyboard/lazy
  loading/responsive review, and distinct no-artwork versus uncertain-identity UI
  review. Document uploads remain explicitly unavailable. **Externally
  unqualified:** independent authorization review.

## Wave 3 U04.1 standalone asset commands — 2026-09-10

- `standalone_assets_cover_all_subjects_and_keep_independent_history` creates
  real PNG, JPEG, and WebP Covers/Photos/Scans across Work, Edition, Item,
  Specimen, Component, and Capture subjects. It verifies original digests/bytes,
  decodable PNG thumbnails bounded to 192×192, deterministic replay, immutable
  subjects, independent visibility/alt-text revision, three retained metadata
  revisions after tombstoning, unchanged Item revision, retained payload bytes,
  and standalone-row survival through reindex.
- `unsupported_or_invalid_asset_inputs_publish_nothing` probes explicit document
  unavailability, corrupt image bytes, a byte over the 20 MiB limit, and a subject
  outside the selected collection. No Asset revision or image scratch is created.
  Existing phase-fault coverage continues to exercise real Asset payload recovery
  through the compatibility cover consumer.
- Full local gate: architecture/link/fixture checks, Rust formatting, all 118
  locked workspace tests, warning-free all-target clippy, generated API drift,
  TypeScript checking, web build, and rebuilt embedded-assets server pass.
- **Not implemented:** U04.2 collection-scoped `PresentedAsset` metadata/original/
  thumbnail routes, visibility authorization on those new routes, and server-side
  `primary_artwork`; U04.3 attachment controls and artwork-row review. Document
  uploads remain explicitly unavailable. **Externally unqualified:** independent
  review.

## Wave 3 U03.3 physical lifecycle privacy/reindex review — 2026-09-10

- The read-model firewall fixture now includes a returned loan with a unique
  contact marker. Authenticated viewer responses for both `GET /items/{id}` and
  cursor browse contain no acquisition, private-note, contact, or loan-history
  keys or marker text; the curator response retains the private note and contact.
- `physical_lifecycle_routes_follow_curation_listener_composition` probes all
  seven focused route patterns/eight methods. They are 404-absent on public
  ingress when public curation is disabled and reach authentication only when
  explicitly enabled; spoofed proxy headers remain irrelevant.
- The two-copy restart fixture now asserts the disposed first copy retains its
  returned loan, while the separately updated borrowed second copy retains its
  label and possession with no loan history. Reindex/reopen also retains the
  Capture/Asset relationships and readable original artwork established before
  disposal.
- Full local gate: architecture/link/fixture checks, Rust formatting, all 116
  locked workspace tests, warning-free all-target clippy, generated API drift,
  TypeScript checking, web build, and rebuilt server pass. Normal HTTPS and
  `LOCAL_SETUP=1` private-listener Chromium smoke suites both pass.
- **Externally unqualified:** independent lifecycle/privacy review. **Not
  implemented by design:** specimen deletion, trading, or disposal-driven
  Evidence/payload deletion.

## Wave 3 U03.2 physical lifecycle editor — 2026-09-10

- The Chromium smoke scenario creates a copy with condition, acquisition, cost,
  source, and private notes through the focused API; creates Disc 2 before Disc 1
  and observes canonical ordering; starts and returns a loan; and confirms the
  returned loan remains visible after disposal.
- The scenario opens an edit draft, publishes a concurrent broad Item revision,
  receives a real 409, and proves the typed draft remains unchanged. The conflict
  panel displays the form's revision and current server revision; “Keep draft”
  rebases without discarding input and the subsequent focused command persists it.
- Forms and copy/part/loan rows have named landmarks and controls. The browser
  checks tab order from “Add physical copy” to “Edit copy,” captures
  `web/test-results/inventory-mobile.png` at 390px, and finds no horizontal
  overflow. Visual inspection found the copy ledger dense but legible, with
  ordered parts, private notes, and lifecycle actions clearly separated in the
  existing archival-workbench style; no decorative dashboard treatment was added.
- `npm --prefix web run check`, `npm --prefix web run build`, the rebuilt embedded
  `collector-server`, and `npm --prefix web run test:smoke` pass.
- **Not implemented:** U03.3's complete viewer structural-absence matrix,
  independent-copy restart/reindex review, and browser security review. Specimen
  deletion remains intentionally absent.

## Wave 3 U03.1 focused physical lifecycle commands — 2026-09-10

- TDD coverage in `focused_inventory_commands_preserve_history_and_are_idempotent`
  proves command-bound specimen/loan identity, idempotent replay, two independent
  copies on a wishlist item, explicit loan return, retained loan history,
  disposal, broad-save history preservation, Wave 1 tamper rejection, reindex,
  and restart. It also publishes a Capture and real artwork payload before
  disposal, then proves both relationships and the original bytes remain readable.
- `component_order_and_references_are_protected_by_focused_commands` exercises
  out-of-order multi-disc creation, deterministic ordering, component update,
  successful unreferenced removal, and refusal to remove a component referenced
  by either a Capture or an Asset. Existing projection-removal tests now use the
  focused command rather than the retired broad-save bypass.
- `focused_inventory_routes_require_curator_role_and_revisioned_idempotent_commands`
  proves direct viewer denial, required idempotency keys, stable replay, and the
  role-filtered `PresentedItem` response. Contract coverage asserts all seven
  lifecycle paths are annotated. Generated OpenAPI and TypeScript omit specimens
  from `SaveItemCommand` and include only working focused routes.
- Full local gate: `python3 scripts/check_architecture.py`,
  `cargo fmt --all -- --check`, `cargo test --workspace --locked` (115 tests),
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `npm --prefix web run check`, `npm --prefix web run check:api`,
  `npm --prefix web run build`, and the rebuilt `collector-server` pass.
  The isolated Chromium smoke suite also passes after removal of the obsolete
  broad-save add-copy control.
- **Not implemented:** U03.2 copy-level lifecycle UI and its draft/conflict
  behavior; U03.3's complete privacy/browser/restart matrix. The obsolete
  broad-save add-copy shortcut is removed, so no final-looking control advertises
  missing editor behavior. Physical specimen deletion remains intentionally
  absent.

## Wave 3 P02.1 durable job repository — 2026-09-10

- A test-first schema assertion failed against the Wave 2 two-column JSON job
  table, then passed after private schema v8 added typed ownership, target,
  state/phase, progress, attempt, timestamp, diagnostic, request/result, lease,
  claim-token, and cancellation fields.
- The committed private-v2 fixture migrates its older Job shape, which has no
  Item identity, without changing its Job ID or losing completed progress. A
  current upload produces a typed queued row and remains readable through the
  existing service.
- `python3 scripts/check_architecture.py`, `cargo fmt --all -- --check`,
  `cargo test --workspace --locked` (96 tests), and
  `cargo clippy --workspace --all-targets -- -D warnings` pass.
- **Not implemented:** durable claims/leases, scheduler execution, restart
  reconciliation, cancellation/retry commands, and the new job API remain
  P02.2-P02.4. No UI or capability advertises them.

## Wave 3 P02.2 durable claim and worker state machine — 2026-09-10

- TDD coverage uses real schema-v8 SQLite rows to prove FIFO claiming, the
  storage-enforced global limit of two, one active job per user, 30-second lease
  expiry, immediate takeover from a prior server instance, and claim-token
  fencing. A stale worker cannot heartbeat or complete a reclaimed Job.
- Phase changes persist immediately. Same-phase progress is monotonic, bounded by
  total units, and persisted no more than once per second. The server starts two
  blocking-safe worker loops, each with a five-second heartbeat; async HTTP
  workers do not execute integrity reads or hashing.
- Integrity verification is the first executable scheduler consumer. Its HTTP
  action returns `202` with the durable Job identity. A recovery test publishes
  the Evidence, simulates loss before Job completion, reclaims from a new server
  instance, and proves the deterministic Evidence and Item revision are not
  published twice.
- Review found and corrected two issues before the child was marked implemented:
  concurrency was initially bounded only by worker composition rather than in
  the claim transaction, and recovered integrity work initially generated fresh
  Evidence IDs. Both have permanent regressions.
- Generated OpenAPI and TypeScript reflect the real asynchronous verification
  response. `python3 scripts/check_architecture.py`, API drift checking,
  `cargo fmt --all -- --check`, `cargo test --workspace --locked` (102 tests),
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `npm --prefix web run check`, `npm --prefix web run build`, and the subsequent
  embedded-assets server rebuild pass.
- **Not implemented:** P02.3 cancellation/retry and publication commit-boundary
  policy; upload/inbox publication still uses the legacy request-triggered path
  and process-local semaphore. P02.4 cursor `JobView`, detail, cancellation, and
  retry routes are absent. Event streaming remains C06.

## Wave 3 P02.3 cancellation, retry, and publication reconciliation — 2026-09-10

- `upload_source_staging_is_not_claimable_until_commit_is_enqueued` proves an
  incomplete or fully staged source is not executable until the commit command
  durably changes its phase. The HTTP commit action only enqueues; worker threads
  perform archive hashing, copying, and publication.
- `configured_inbox_staging_runs_through_the_durable_worker` proves inbox copying
  and publication use the same claim/fence path and preserve the configured source.
  Source locators remain private job requests and are absent from public DTOs. The
  process-local server semaphore and ad-hoc inbox task are removed.
- `queued_and_running_upload_cancellation_retain_source_material`,
  `integrity_job_observes_cancellation_between_files`, and
  `cancellation_at_payload_boundary_aborts_before_revision_commit` cover immediate
  queued cancellation, bounded-unit checks, and the final payload/revision race.
  Pre-commit payloads/journals are aborted while private source chunks remain.
- `cancellation_after_revision_commit_has_a_stable_too_late_result` proves the
  stable conflict after the durable commit point. Startup reconciliation marks
  committed upload/integrity work before routes open, and the worker completes the
  committed result instead of accepting cancellation or duplicating Evidence.
- `failed_upload_retry_reuses_job_and_publication_identities` injects a real
  publication failure, then proves retry keeps Job/Capture/Representation IDs,
  source bytes, and failure history while incrementing attempt to two.
- Full local gate: architecture/link/fixture checks, Rust formatting, 109 locked
  workspace tests, warning-free all-target clippy, TypeScript check/build, and the
  rebuilt embedded-assets server pass.
- **Not implemented:** P02.4 cursor `JobView`, job detail, and generic job
  cancel/retry routes remain absent. `GET /jobs` is still the legacy bounded vector;
  the existing upload cancellation route is only a compatibility adapter. Event
  streaming remains C06. Target-Linux process/filesystem behavior remains
  externally unqualified.

## Wave 3 P02.4 job API and root review — 2026-09-10

- `job_views_are_cursor_paged_and_omit_execution_secrets` covers stable descending
  cursor pages, page bounds, submitter filtering, safe progress, and serialization
  that contains no stored request, claim token, or worker identity.
- `failed_integrity_job_retries_with_the_same_identity_and_attempt_history` proves
  the generic retry path reuses the real Job, increments attempt, retains private
  failure history, and completes the same executable integrity consumer.
- `job_routes_are_submitter_scoped_idempotent_and_secret_free` exercises list,
  detail, and cancellation through HTTP. Another user's administrator receives
  404 rather than a bypass, and an identical cancellation key replays the same
  safe result. Missing/mismatched idempotency behavior remains covered by the
  general mutation contract test.
- Generated OpenAPI and TypeScript now include cursor `JobView`, detail, cancel,
  and retry routes. No view contains private request/result history, lease, worker,
  or claim fields.
- P02 root review reran architecture/link/fixture checks, Rust formatting, all 112
  locked workspace tests, warning-free all-target clippy, TypeScript checking,
  deterministic API drift, web build, and the embedded-assets server rebuild.
  Chromium initially failed to launch/terminate under the filesystem sandbox with
  `EPERM`; the same isolated temporary-profile smoke suite passed with approved
  sandbox escalation.
- **Externally unqualified:** target-Linux process termination/filesystem behavior
  and independent scheduler review. **Not implemented:** C06 event streaming and
  distributed scheduling. These are not represented as P02 capabilities.

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

## A04.1 read-boundary validation — 2026-09-09

The first HTTP test failed against the Wave 1 behavior because a viewer response
contained the exact unknown-module RawJson envelope twice (domain payload and
generic presentation), and serialized private metadata/specimen field names as
empty or null values. The implementation replaces flattened stored Items with
explicit application response DTOs and removes the legacy public core-record read
shortcut.

- `viewer_item_dto_omits_private_fields_and_admin_without_grant_is_denied` uses a
  real temporary archive plus private/catalog SQLite databases. It saves unknown
  lexical module data, acquisition/private notes/provider metadata, commits a real
  upload and integrity Evidence, then checks serialized viewer bytes, curator
  fields/effective permissions, Evidence detail filtering, archive-locator absence,
  and direct denial of an ungranted server administrator.
- Existing unavailable-module owner presentation, collection pagination/search,
  role redaction, migration, Capture precedence, upload, artwork, and LAN TLS tests
  remain passing. The workspace now has 46 Rust tests.
- `python3 scripts/check_architecture.py`, `cargo fmt --all -- --check`,
  `cargo test --workspace --locked`, and
  `cargo clippy --workspace --all-targets --locked -- -D warnings` pass.
- `npm --prefix web run check`, `npm --prefix web run build`, and
  `cargo build -p collector-server --locked` pass.
- `npm --prefix web run test:smoke` passes with a temporary server/profile. Its
  first sandboxed Chrome launch aborted before application assertions; the allowed
  external browser rerun passed. No local user archive/configuration was opened.

A04.3 idempotency and A04.4 generated contracts remain unimplemented. A04.1 does
not promote the combined Wave 1 Catalog adapter or handwritten web types into
permanent architecture.

## A04.2 application-service validation — 2026-09-09

- The first bounded-command HTTP test failed with 422 because the Wave 1 handler
  only accepted a complete stored Item. The new command succeeds for editable
  fields and rejects Capture, Asset, Evidence, representation, legacy locator,
  metadata, revision, and timestamp fields at deserialization.
- Entity integration tests now cross SaveItem/Work/Edition/Capture commands. They
  prove revisions and migration provenance remain server-owned, Capture
  reassociation retains payloads, and locator/representation input is impossible.
- Media, activity, imports/jobs, administration, authentication, and passkey
  persistence cross application services. Store effect ports receive UserId and
  typed identities rather than Principal. A real HTTP/SQLite/archive test compares
  service and direct-HTTP denial for another user's upload and a curator-only
  Asset. Ranged LAN download, artwork, upload retry, inbox, failed-job, recovery,
  passkey, and public/private exposure regressions remain passing.
- Review moved inbox authorization before source-path inventory and added a source
  architecture check rejecting `state.store` in HTTP/passkey implementations.
- `python3 scripts/check_architecture.py`, Rust fmt/workspace tests/clippy with
  `--locked`, web check/build, server rebuild, and normal browser smoke pass. The
  workspace has 47 tests. Browser state, archives, accounts, and TLS files were
  temporary.

A04.3 receipts/idempotency and A04.4 generated contracts remain unimplemented.
S02 still owns recent authentication, rate buckets, passkey CAS, and credential
lifecycle; the A04.2 port move does not claim those shortcuts are permanent.

## A04.3 command-idempotency validation — 2026-09-09

- `same_key_replays_after_restart_and_changed_payload_conflicts` uses a real
  archive and private/catalog SQLite databases, closes and reopens the Store, and
  proves the original result is replayed without a second item revision while a
  changed typed request returns `IdempotencyConflict`.
- `concurrent_receipt_claim_has_one_executor_and_replays_completion` releases two
  threads through a deterministic barrier against the real receipt table. Exactly
  one claims execution, the other receives `CommandInProgress`, and completion is
  subsequently replayable.
- `stale_revision_abandons_receipt_without_publishing` proves a revision conflict
  creates no item revision and does not consume the key; a corrected expected
  revision with that key can then commit.
- `mutation_requires_key_and_http_replay_has_stable_conflict_code` verifies the
  transport rejects a missing key, accepts an identical retry, and emits the
  stable `idempotency_conflict` code for changed payload reuse.
- The browser API supplies keys for command mutations and retries one network
  failure with the same key. Authentication ceremonies/logout and upload chunks
  remain intentionally exempt.

The focused HTTP test and the 51-test Rust workspace pass. A04.4 generated
contracts and its contract/security review remain outstanding; P01 still owns
coordination between receipts and crash-recoverable archive publication.

## A04.4 generated-contract validation — 2026-09-09

- Runtime API routes are composed from `utoipa-axum` annotated route groups.
  `annotated_contract_covers_api_and_is_not_served_as_public_ui` checks all 32
  API paths have responses, every request body declares its enforced byte limit,
  viewer-facing Capture schemas omit archive locators, the item request is the
  reviewed two-shape compatibility union, and no schema endpoint is served.
- `scripts/api_contract.py` emits OpenAPI and TypeScript into temporary paths and
  compares bytes with `api/openapi.v1.json` and
  `web/src/generated/api.ts`. The check passes after a clean regeneration.
- React imports generated response and command types. Type checking initially
  failed because curation fabricated a response-shaped Item; it now constructs a
  bounded `SaveItemCommand` and normalizes only editable fields.
- `wave1_item_envelope_preserves_server_owned_state_and_rejects_changes` proves a
  Wave 1 envelope can edit an ordinary field, while changed deletion/server-owned
  state is rejected and the canonical record remains unchanged.
- Contract/security review confirmed command secrets are not receipted, raw
  private/module fields remain behind application DTO filtering, body limits
  match router layers, and OpenAPI/Swagger are absent from all listeners.

Full A04 checkpoint: architecture check, API drift check, Rust fmt/workspace tests
(53 tests)/clippy, web typecheck/build, server rebuild, normal browser smoke, and
`LOCAL_SETUP=1` browser smoke pass. The first sandboxed Chrome launch aborted
before assertions; the approved external-browser runs passed. Tests used only
temporary archives, databases, browser profiles, accounts, and TLS state.

## P01.1 Wave 1 fixture and schema-3 record validation — 2026-09-09

- `materialize_wave1_fixture` reproducibly upgrades the repository's schema-1
  synthetic fixture through Wave 1 behavior and advances its standalone Capture
  beyond the embedded compatibility copy.
- `committed_wave1_schema2_and_private_v2_fixture_are_upgrade_inputs` copies the
  committed fixture into temporary directories, initializes a real SQLite private
  database from the SQL fixture, opens the Store, and proves every pre-existing
  archive file retains its exact SHA-256 digest. Accounts, grants, session,
  passkey, setting, activity, and job rows survive the incremental private-v3
  migration, while item hydration selects the latest standalone Capture.
- Schema-3 unit tests prove canonical Item serialization cannot contain embedded
  Captures or Assets, lexical unknown-module JSON survives, and standalone
  Evidence permits an unattached Capture while requiring one or more distinct
  Representation IDs.

P01.1 defines compatibility inputs and records only. It does not claim the
P01.2 journal, schema-2-to-3 migration, or transactional publication writer.

## P01.2 journal/recovery-kernel validation — 2026-09-09

- `controlled_fault_at_every_phase_leaves_classifiable_real_files` uses a real
  temporary archive for faults after Prepared, BytesStaged, PayloadPublished,
  RevisionCommitted, ProjectionApplied, and Completed. Reopening classifies each
  state from files and digests without sleeps.
- Journal tests reject traversal/absolute paths, invalid hashes, non-adjacent
  revisions, backwards phases, conflicting transaction reuse, missing committed
  revision bytes, and corrupt known artifacts. Durable completion cleans only its
  known journal/scratch after recording Completed.
- `startup_retains_and_reports_unknown_staging` proves unrecognized scratch bytes
  remain untouched and visible in the startup report.
- `corrupt_known_publication_blocks_store_open` proves Store startup rejects a
  digest-mismatched known transaction before exposing services.
- The full `collector-store` suite passes with 40 tests, using real temporary
  SQLite databases and archive directories. The Wave 1 fixture now migrates
  incrementally through private schema v4 without losing any v2 rows.

P01.2 does not claim entity publication or process-termination durability;
P01.4 and P01.5 own those tests respectively.

## P01.3 compatible archive-migration validation — 2026-09-09

- The committed Wave 1 archive/private-v2 fixture test now verifies deterministic
  schema-3 Item/Evidence/Asset/Capture output and catalog projection schema v3.
  Every pre-existing archive file retains its original SHA-256 digest.
- Canonical schema-3 Item JSON contains neither `captures` nor `assets`, while its
  lossless unknown module envelope retains the original key order and `1e+02`
  number spelling. Existing artwork remains in its item-local path and is reached
  through a hash-checked legacy locator.
- The fixture's standalone Capture revision 3 remains authoritative over embedded
  revisions 1/2 after migration and full reindex. Capture locators no longer exist
  in `collector-core` or API DTOs.
- `conflicting_duplicate_evidence_id_blocks_schema3_migration` changes a repeated
  Wave 1 Evidence body under the same ID, updates its valid source manifest, and
  proves migration stops before the completion marker or catalog projection exists.
- `cargo test -p collector-store --locked` passes 41 tests and the store all-target
  clippy gate passes with warnings denied.

Schema-3 migration is additive; P01.4 still owns journal adoption by entity,
Evidence, Asset, tombstone, and projection publication.

## P01.4 transactional entity-publication validation — 2026-09-09

- `entity_fault_at_each_publication_phase_recovers_to_pre_or_post_state` opens a
  real archive/catalog/private SQLite installation with a one-shot injected fault
  at each journal boundary. Prepared/BytesStaged/PayloadPublished reopen to no
  Item; RevisionCommitted/ProjectionApplied/Completed reopen to exactly revision 1.
  Startup removes only the reconciled known journal without sleeps.
- `targeted_projection_and_full_reindex_are_equivalent` snapshots the projected
  Item/Evidence JSON, fully rebuilds catalog SQLite, and obtains identical results.
- The stale-revision test additionally proves no journal is created by a rejected
  command. Existing Work/Edition/Capture reassociation/restart tests now exercise
  schema-3 journaled writers.
- The upload/reverification regression proves two integrity observations produce
  two standalone Evidence bodies and a latest Item envelope with two ID/digest
  references and no copied Evidence details.
- `project` now compares a reused Evidence ID's complete canonical record and
  fails on differences instead of silently ignoring them.
- The 43-test store suite and all-target clippy with warnings denied pass.

P01.5 still owns payload staging/publication, stable retry identities, and actual
subprocess termination tests.

## P01.5 payload publication and termination validation — 2026-09-10

- `upload_fault_at_each_phase_recovers_with_stable_identities` injects failures at
  all six phases against real archive and SQLite paths, reopens, and retries. Every
  case produces exactly one Capture, Representation, Evidence record, verified
  downloadable payload, completed Job, and retained private source chunk.
- `sigkill_at_each_upload_publication_phase_recovers` launches the integration-test
  executable as a child, waits for an explicit stdout phase marker, terminates it,
  and reopens the same temporary paths. It uses no sleeps or timing assumptions.
- The payload transaction uses the durable upload ID, re-reads copied destination
  hashes, refuses conflicting published bytes, and makes the Capture revision its
  commit artifact. Pre-commit cleanup prunes only empty known publication paths.

These process-kill tests are not a power-loss or filesystem certification. Target
Linux and NAS semantics remain external gates.

## P01.6 collection/grant recovery validation — 2026-09-10

- `collection_and_owner_grant_recover_as_one_retryable_effect` injects each
  collection boundary, reopens, and retries the same idempotency key. Exactly one
  Collection and owner grant remain, the private recovery table is empty, and a
  second installation opened from only the archive has no grants.
- `sigkill_collection_create_recovers_exactly_one_owner_grant` repeats the matrix
  with child processes blocked at deterministic markers and forcibly terminated.
- Startup migration/reconciliation is synchronous under the archive writer lock;
  corrupt known recovery fails Store construction, so no write service can be
  reached early. Stale in-progress command claims are cleared only after exclusive
  startup proves their former executor is gone; typed writers supply retry safety.
- The P01 store suite passes 49 tests, including worker entrypoints, using only
  temporary archives/private/catalog SQLite databases.

P01 requires independent storage review and target-Linux termination execution
before release sign-off.

## S02.1 private-auth migration validation — 2026-09-10

- The committed private-v2 fixture now migrates through v5 and retains both users,
  password hashes, grants, session, passkey JSON, settings, activity, and jobs.
  Accounts receive enabled/profile revision 1, grants revision 1, and passkeys get
  deterministic typed IDs without changing their credential records.
- The legacy session keeps its identifier and expiry but receives
  `authenticated_at=0`/`created_at=0`, so it is a valid browser session that can
  never satisfy recent-authentication checks. Schema v5 creates no native
  credential table or credential from that row.
- `private_lock_prevents_two_servers_using_the_same_auth_state` opens two distinct
  archives against one temporary private directory and proves the second server is
  rejected until the first lock holder closes.
- Future private schemas above v6 still fail closed. All migration checks use real
  SQLite files rather than mocked repositories.

## S02.2–S02.5 credential lifecycle validation — 2026-09-10

- `recent_auth_expires_at_exact_boundary_and_activity_does_not_refresh_it` uses an
  injected clock and proves freshness is accepted before five minutes and rejected
  at the boundary. `browser_origin_attack_matrix` rejects missing, null, malformed,
  sibling-host, alternate-port, and spoofed-header browser requests; only the exact
  configured origin succeeds.
- Deterministic account/direct-peer/global token buckets use injected monotonic time
  and never consult forwarded addresses. Browser ceremony, recovery, and mutation
  classifications come from authenticated transport state rather than a caller
  header.
- `recovery_code_replacement_and_concurrent_consumption_have_one_winner` uses two
  real SQLite connections and a synchronization barrier. It proves ten independent
  160-bit codes are returned once, only hashes persist, replacement is atomic, one
  concurrent consumer wins, the password changes, old browser sessions fail, and
  one fresh browser session is issued.
- `concurrent_passkey_counter_update_fails_stale_assertion_and_revocation_is_immediate`
  verifies outside SQLite, then races the record/revision CAS with session issuance;
  at most one stale assertion can issue a session. Revoked passkeys fail immediately.
- `concurrent_last_authority_changes_cannot_both_commit` races administrator
  demotion and collection-owner removal in real SQLite transactions. At least one
  enabled administrator and owner remains. Account disable revokes sessions/native
  credentials/pending enrollments/recovery batches while preserving identity,
  password/passkey records, grants, and history.
- `local_recovery_requires_exclusive_lock_and_preserves_grants` refuses a running
  server lock, then resets an explicit UserId while stopped, revokes every credential
  kind, writes a redacted audit event, and preserves grants/administrator state.

## S03 browser-mediated enrollment validation — 2026-09-10

- `enrollment_terminal_states_wrong_user_and_issuance_race_are_enforced` uses an
  injected clock and real schema-v6 private SQLite. Wrong-user approval, wrong
  completion secret, cancellation, exact expiry, terminal replay, and simultaneous
  completion fail; one winner receives one plaintext credential and one hash is
  stored. Archive/private scans find no issued plaintext secret.
- `browser_approved_enrollment_issues_and_revokes_native_credential` exercises the
  client-surface start/complete bootstrap, exact-Origin recently-authenticated browser
  approval, header-independent bearer lookup, device inventory, revocation, and
  failure on the next request. `x-collector-client` cannot request bearer issuance.
- `paired_lan_authentication_and_ranged_download_work_without_a_public_route` uses
  the `collector-client` enrollment/storage port over direct TLS, verifies Pairing
  server identity and certificate material, persists the credential in an isolated
  test adapter, performs an authenticated ranged download, and refuses unrelated
  certificates or cross-origin credential delivery. Redirects are disabled.
- The macOS production secure-storage adapter, real password-manager/hardware
  authenticators, and browser-trusted remote HTTPS remain explicit external gates.

## U02 administration and Wave 2 integration validation — 2026-09-10

- `account_revisions_and_last_admin_default_guards_surface_stable_errors` covers
  account optimistic conflict, default reassignment, disabled-account, and
  last-administrator error codes through HTTP.
- `direct_grant_api_matches_owner_policy_and_removal_is_immediate` covers owner,
  curator, viewer, and ungranted administrator behavior across two collections.
  Exact-username grant changes are revisioned; direct bypass attempts fail, removal
  applies to the next request, and the last owner is protected.
- The administration, curation, and account-security React bundles consume generated
  API types. Curation/grant controls derive from `EffectivePermissions`; the API
  remains authoritative. Recovery codes display once and session/passkey/native
  revocation refreshes inventory.
- `cover_payload_faults_recover_with_one_collection_asset` faults every durable
  phase and proves new artwork payloads publish under collection-level Asset identity,
  retry with the same command produces one Asset, legacy item-local reads remain
  compatible, and private staging is removed after recovery.
- The final local gate passed architecture/link/fixture checks, deterministic API
  regeneration, Rust formatting, 86 locked workspace tests, warning-free all-target
  clippy, TypeScript checking/build, and a rebuilt embedded-assets server. The
  Chromium smoke command was attempted but Chrome launch/termination failed with
  sandbox `EPERM`; neither normal nor `LOCAL_SETUP=1` smoke is claimed for this run.
- Target-Linux process termination, real hardware/browser qualifications, native
  platform secure storage, and independent storage/security/protocol reviews remain
  required. Process-kill coverage is not a power-loss or NAS certification.
- Completed command receipts replay after restart, but the process-exit boundary
  after a non-P01 effect commits and before its receipt is finalized has no kill
  test. Startup currently clears that stale claim and a typed retry can conflict;
  A04.5 must coordinate or reconcile this boundary before exact crash replay is
  claimed for every mutation.

## Wave 2 review corrections and final validation — 2026-09-10

The seven independent review findings are corrected in this delivery:

- Prepared partial staging and interrupted `.json.tmp` journal replacements recover;
  committed payload/record corruption still blocks startup. Collection manifests
  publish through a synced temporary file and atomic rename.
- Disable/re-enable never restores native credentials or pending enrollment.
  Recovery-code replacement and password recovery preserve native access and
  pending enrollment as specified. Regression tests use real private SQLite.
- Passkey ceremonies retain their original stored record/revision through verified
  finalization. The Chromium virtual-authenticator test creates two ceremonies,
  completes the newer assertion first, and proves the stale one receives 401.
  This exercises the real browser, library verification, HTTP handler, and CAS.
- Private schema v7 adds persistent grant revision counters and command outcomes.
  Removing/recreating a grant cannot accept its previous revision, and exact
  successful retries are not rejected by later inventory revisions.
- `CommandEffects` scopes explicitly bind mechanism adapters to a receipt. Private
  effects and receipts share a SQLite transaction; archive effects prepare exact
  responses against immutable commit markers in private state. No receipt,
  account identity, or grant is written to portable publication journals.
- `sigkill_receipt_effect_gap_replays_every_mutation_family` terminates subprocesses
  after effects but before application receipt completion across 19 mutation
  families, plus five early archive-commit cases. All 24 cases replay the exact
  response after restart and reject a changed digest. Upload job bookkeeping and
  completed artwork staging are repaired on startup.
- Account/grant authority, credential revocation sets, default-account eligibility,
  and enrollment transitions are application-owned policies over transactionally
  stable adapter snapshots. Storage retains CAS, hashing, and transaction mechanics.

Local onboarding smoke also found that exact-Origin rejection was blocking static
ES modules at an unconfigured address before the UI could explain the login error.
Static resources now load while API/mutation checks and listener exposure remain
intact; a new HTTP test and the local-setup browser flow cover the distinction.
The artwork smoke assertion waits for the asynchronous row thumbnail refresh.

Final checks: architecture/documentation/fixture validation, deterministic OpenAPI
and TypeScript regeneration, Rust formatting, 95 workspace tests (including the added
review regressions), warning-free all-target clippy, TypeScript checking/build,
embedded-assets server rebuild, and both normal and LOCAL_SETUP=1 Chromium smoke
flows. Chrome runs used explicitly authorized sandbox escalation, isolated profiles,
and temporary servers; no user archive/configuration was opened or migrated.

The preceding A04.5 caveat and sandbox-only browser failures are historical and
superseded by this section. Linux termination/filesystem execution, power-loss/NAS
qualification, real password managers/hardware authenticators, trusted remote HTTPS,
and native platform credential storage remain external or later release gates.
Local process termination and virtual authenticators do not certify those surfaces.
