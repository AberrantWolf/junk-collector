# Implementation status

Pre-release evolution follows [the accepted compatibility and contract policy](decisions/pre-release-evolution.md). Earlier checkpoint compatibility is not a release requirement; data preservation and recovery remain required.

Wave 3 review follow-up (2026-09-11; validated review-fix commit): interrupted pre-commit cancellations now
resolve the Job and Import together at startup. Recovery of a committed Capture
finishes publication despite later Item edits, and does not reserve payload space
again. Exact final upload-chunk retries remain read-only after staging completes.
The browser can reselect and resume an interrupted package using its existing
Import identity; it checks retained prefixes before appending, which retransfers
those prefixes for comparison but never rewrites them. Saved reviews can be edited
again after a failed publication, and transient restore failures retain the saved
Import identity. These changes add permanent recovery and browser regression tests.

This repository is an executable architecture/server foundation, not the complete first release described in the accepted plan. Capability reporting must stay honest as remaining modules land.

Use the [handoff backlog](handoff-backlog.md) for ordered, scoped implementation tickets and the [release requirements](release-requirements.md) for the repository-local target specification. The gap list below is a summary, not a substitute for ticket acceptance criteria.

## Active delivery target — 2026-09-16

D06.3 is committed as `3a6143b`. The accepted [local-alpha plan](local-alpha-plan.md)
now takes precedence over the remaining wave order. LA1 guided ingestion and
automatic enrichment is reviewed and corrected locally. LA2 album preparation and
usable browsing/playback is reviewed and corrected locally (2026-09-17).
LA3 real local installation qualification is in progress: its installation/restore
runbook, owned-corpus journey, live MusicBrainz/CAA enrichment and actual
stopped-copy/restore drill pass on the selected Mac. The intended second LAN device
remains outstanding, so LA3 is only partially qualified. P05.1 remains paused.
The full first release, native clients and public deployment remain later targets.

## Implemented

- Web album follow-up (2026-09-20): archived cover choices have authenticated
  thumbnail previews. Cover Art Archive options also show lazy provider previews
  before acquisition, with unavailable/retry states and no automatic archival.
  A curator-only, bounded server adapter fetches 250px thumbnails; editor close controls no longer float across cleared
  sections. Preparation updates refresh the parent Item/Capture display and
  transient polling errors retry. Album preparation saves independently of
  inventory; its error is shown beside the preparation controls. Music selection
  uses recorded source fingerprints for browsing/admission rather than hashing
  the whole disc on every poll and per album track. Background conversion retains
  actual-byte validation before encoding and publication.

- LA3 is partially qualified on macOS 26.6.2 arm64. Five isolated owned-package
  copies cover flat and nested layouts, mixed-mode game data, Redumper/BIN+CUE,
  existing FLAC sidecars and damaged input. Preservation, retained structure/raw
  hash observations, manual title authority, provider outage, interrupted staging,
  two-track FLAC preparation/real playback and local Owner/Viewer denial/revocation
  were exercised. The actual stopped-server copy into separate empty roots matched
  complete archive/state manifests before open, retained config/TLS/server identity,
  accounts and a Viewer grant, resumed four unfinished Jobs and played restored
  media. The source corpus remained unchanged and original/evidence/metadata record
  digests matched. Explicit CUE/TOC selection now survives analysis and preparation;
  shared game input omits the music-only field; the open import panel tracks the
  published association until edited. Live MusicBrainz identified `HOME` by `B’z`,
  CAA published a real cover, manual title authority survived, and the restored
  browser rendered cover/text and played the existing FLAC. The journey fixed a
  narrowly pinned Internet Archive CDN redirect, numeric CAA image IDs and retried
  catalog-completion receipt projection. A cached 10,001-row full-release Redump
  PlayStation view now parses and indexes from immutable local bytes. Explicit
  raw-2352 row coverage retains provider CUE observations outside the matching
  domain; the owned four-track disc produced a unique complete SHA-1 match for
  `Brave Fencer Musashi (USA)` and supplied automatic descriptive text without
  conflating it with preservation identity. A 2,661-row Headerless No-Intro NES DAT
  is also cached for the implemented iNES-payload domain, but no owned NES dump was
  supplied. DAT-o-MATIC refused the later N64 request as an unknown filter, so N64,
  SNES and PC Engine caches remain absent and automatic retries are disabled. The
  intended second physical LAN browser remains unavailable; LA3 is not accepted.

- LA2 adds application-owned whole-album FLAC scope over the existing derivative
  pipeline. A bounded frozen batch Job records selected associated-disc sources and
  stable members, fans out existing per-track Jobs, converges active/identical work,
  reports aggregate and per-track partial outcomes, retries unfinished members and
  cancels only owned unshared children. Prepare album and the optional guided-import
  checkbox use the same service. Generated plan/batch/cancel/retry contracts and the
  curation UI restore progress across reopen/restart without globally disabling
  independent controls; effective artwork remains beside album/player text. Focused
  coverage includes multidisc scope, source ambiguity, missing tools, overlap,
  partial failure, shared cancellation, retry, restart, permission revocation and
  valid-output reuse. Chrome 153.0.8010.37 with FLAC 1.5.0 completed the two-track
  album workflow and real player/revocation checks. LA2 review and both browser
  configurations passed; broader U05/P04 and LA3 external qualification remain open.

- LA1 adds collection-page guided intake without a pre-created Item for configured
  direct inbox packages and resumable browser uploads. Typed planning previews
  package boundaries, unique CUE/TOC anchors, blockers, destinations and provider
  availability. Accepted intent owns stable Item/Capture/Representation identities;
  existing Jobs continue analysis, publication, DAT/MusicBrainz metadata and CAA
  artwork without a browser-owned chain. Server-side bounded history reopens after
  reload, exposes steps/attempts and permissions, supports retry/cancel, safely
  discards unpublished staging, and corrects a published Capture association.
  Filename titles and provider artwork stay automatic rather than manual authority;
  provider failure never rolls back originals. Private schema 13 persists intent
  and cleanup state. Focused P04/U05 journey coverage is implemented, not their
  broader completion gates.

- 2026-09-15 workflow corrections to P04/U01/D07: upload, analysis, review and
  publication have distinct labels and next steps; analysis no longer announces
  publication. Music preparation appears at the top of the editor, and absent
  tracks show guidance. Track availability follows Capture revisions, including
  publication without an Item revision change. Management dialogs keep their close button visible,
  handle Escape with draft-discard confirmation, contain keyboard focus and
  restore focus on close. Administration explains encoder setup; browser-based
  encoder settings writes and browser-triggered automatic MusicBrainz/artwork lookup remain absent.

- D06.3 is reviewed and corrected locally. Curation-only generated
  contracts expose typed metadata prerequisites, provider availability, permissions,
  safe candidate history and typed catalog-job results without raw provider bodies.
  The archival-workbench review panel provides explicit MusicBrainz/CAA lookup,
  durable job status/cancel/retry/reopen behavior, field and artwork authority,
  selectable history, provenance and clear actions. Effective metadata/artwork now
  drives browse, search and detail, including inventory edits and reindex. Application
  services complete acquired results; preserved artwork needs explicit cover selection.
  Pending completion inputs survive reopen, and detached requests are cancelled.
  Viewer reads omit inventory baselines,
  private assets and observations. Deterministic browser fixtures exercised real
  provider jobs and published image bytes through Google Chrome at desktop and
  390px. Live providers and deployment qualification remain unrun.

- D06.2 is reviewed and corrected locally. Optional configured
  MusicBrainz and Cover Art Archive adapters use the existing durable acquisition
  registry with pinned production origins, bounded/cancellable retry and redirect
  handling, a shared one-request/second MusicBrainz limiter, and bounded positive/
  negative caches. Exact responses remain snapshots; immutable digest-versioned
  MusicBrainz candidates and exact D04-result-bound DAT candidates enter the
  existing revisioned metadata refresh effect. CAA images enter the existing
  authorized Item Asset pipeline before candidates may reference them. Manual
  titles, inventory corrections, selected history and explicit artwork remain
  authoritative. Deterministic local fixtures cover provider failures and reindex;
  LA3 adds one live MusicBrainz/CAA disc-and-cover result, while deployment remains
  unqualified.

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
- The S01 threat/recovery contract and D01 pinned reuse/fixture map are complete design gates. S02/S03 now implement the credential/recovery/enrollment portions; authenticated TLS rotation remains S04. D02/D03 and D04.1's reader-only game primitives are implemented locally. D04.2–D04.3 now add the DAT adapter, snapshot/match policy and game workflow locally; D04 is reviewed locally. D05.1 is reviewed and corrected locally: authorized frozen BIN/CUE and current raw-redumper packages produce strict signed layouts, exact `i16` stereo 44.1-kHz PCM, published disc IDs where the layout permits them, and bounded cancellation-aware AccurateRip checksum/offset inputs. Mixed mode, synthetic gaps and unsupported/ambiguous sidecars cannot claim whole-disc audio. D05.2 is reviewed and corrected locally: the provider adapter freezes bounded exact dBAR/no-data observations for checked offline replay, and music-owned policy retains explicit per-track v1/v2 coverage while requiring every expected track and a justified unique offset for whole-disc verification. D05.3 is reviewed and corrected locally: music Import analysis, committed structure jobs, shared-verifier AccurateRip jobs, archived snapshots, history and web curation use the existing registry/job/publication paths. Synthetic workflow tests cover outage, retry, cancellation, exact snapshot-byte rejection, no-data, refresh and reindex. D07.1 is reviewed and corrected locally; D07.2 is reviewed and corrected locally; D07.3 is reviewed and corrected locally. Immutable fixed-profile recipes consume music-owned exact track ranges; the optional bounded process adapter validates FLAC by independently decoding every integer PCM frame and inspects Opus delay/padding; single-track outputs publish through the Capture journal and appear through a viewer-safe availability/read contract with strong ETag and Range behavior. Server-owned optional encoder configuration and curation-only explicit preparation leave import and original reads independent of tools. The bounded browser-player child is implemented locally over that shared DTO: album track selection is grouped by Capture/source and prefers a decodable FLAC; the 2026-09-22 shell-owned player keeps a local per-user cross-album queue with play/pause/seek/next/previous/ended transitions and honest codec/access failures. Google Chrome 153.0.8010.37 actually decoded and played generated FLAC; the child is reviewed and corrected locally. `opusenc`, live-provider, owned real-dump, gapless/sample-accurate playback and external mastering/deployment qualification remain unavailable/external.
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
- P04.1 adds a curation accession workspace over the executable Import API. It can
  select browser files or safe configured-inbox package names, stage and inspect the
  complete inventory with live progress, display persisted analyzer limitations and
  candidate alternatives without a verification claim, record an existing physical
  chain or no association, and publish the reviewed Capture. The responsive keyboard-
  named workflow is browser-tested at desktop and 390px.
- P04.2 adds the submitter's durable active/history Job ledger with bounded progress,
  attempts, safe diagnostics, and server-computed cancel/retry affordances. Visible
  active work polls at two seconds, idle state at 30 seconds, and reconnect/action/
  visibility changes refresh immediately. Retry keeps the same Job identity and
  updates its attempt; Import review inputs remain owned outside the polling ledger.
- P04.3 persists the current Item-scoped Import handle across page refresh, refreshes
  durable Import/Job state after reconnect, and rebases stale review submissions while
  retaining the user's identity and physical-chain choices for explicit reapply. The
  real browser matrix covers retry/cancel, cross-user and collection authorization,
  unavailable analyzer disclosure, keyboard/focus/live status, reduced motion, mobile
  layout, and normal/private listener composition. P04 is locally complete.
- D02.1 revalidates the D01 extraction boundary at exact sibling commits and dirty
  paths. It confirms the old Phono junk-libs pin is locally available, the published
  and generated disc vectors plus `chdman` are present, and the owned-dump/live
  oracle is still external. This is an implemented documentation gate only: no D02
  shared algorithm, consumer migration, or catalog verification capability exists
  yet, and Retro-owned source cannot be redistributed while its MIT attribution is
  a placeholder.
- D02.2 makes `junk-libs-core` the single implementation of generic streaming
  multi-hashing and its byte-reader/result contracts. `junk-libs-disc` now computes
  complete stored CUE file-byte spans and visits each true CHD 2352-byte
  main-channel track frame without padding or subchannel bytes. Retro's complete
  ordered-track hashing consumes those contracts, so changing a non-primary track
  changes the complete result while platform normalization and catalog matching
  remain product policy. Disc identity, AccurateRip, DAT extraction, Collector
  integration, and real-dump qualification remain required under D02.3-D02.5.
- D02.3 adds `junk-libs-disc-id` as the single checked implementation of the
  neutral audio-CD TOC and MusicBrainz, FreeDB/CDDB, and AccurateRip disc IDs.
  Phono re-exports the shared TOC and maps its results into provider-facing fields
  through compatibility wrappers; published ARver, MusicBrainz, and libdiscid
  vectors remain byte-identical. AccurateRip CRC/dBAR/offset behavior, neutral DAT
  parsing, Collector integration, live-drive qualification, and a published Git
  dependency pin remain required under D02.4-D02.5.
- D02.4 adds `junk-libs-accuraterip` as the single headless implementation of
  CRC v1/v2 over packed integer CD PCM, boundary skips, dBAR parsing/request
  validation, primary-checksum comparison, frame-450 evidence, and bounded
  sample-offset search across track boundaries. Phono's compatibility modules
  consume it while CUE/CHD construction, HTTP/rate limiting, evidence storage,
  and product errors remain local. DAT extraction and Collector music integration
  remain required under D02.5/D05; private/live/hardware checks remain external.
- D02.5 adds `junk-libs-dat` as the neutral `BufRead` implementation for Logiqx
  XML and ClrMamePro syntax, entity decoding, checksum normalization, and optional
  record observations. Retro now supplies only file/error adapters at that seam;
  its downloading, cache, index, matching, platform normalization, and naming
  policy remain product-owned. D02 is locally complete. Collector game/music
  integration remains D04/D05, and publishing/pinning shared SHAs plus private/
  live-disc qualification remain external.
- D03.1 establishes `junk-libs-audio` opaque `Read + Seek + Send` byte sources,
  length/media hints, nonzero sample-rate/channel formats, interleaved `f32` PCM
  chunks, and integer frame coordinates. `junk-libs-playback` establishes generic
  opaque track keys, generations, and validated track-local positions. These are
  executable contracts only: decoding, resampling, queue transitions, DSP, and
  consumer migrations remain required under D03.2-D03.5, and AccurateRip remains
  entirely on integer CD PCM.
- D03.2 adds feature-gated PCM-WAV decoding from those opaque readers with bounded
  output and exact track-local frame seeking. Path-free adapters accept already-
  opened CUE and redumper readers; CUE boundaries remain explicit and redumper
  quality observations are returned with playback PCM. No compressed codec is
  advertised. Resampling, shared queue transitions/DSP, and consumer migrations
  remain required under D03.3-D03.5.
- D03.3 adds bounded streaming sinc resampling with exact final length, qualified
  linear gain and explicit hard clipping, and a generic queue state machine for
  replace/append/truncate/track transitions. Seeks are generation-fenced and use
  integer track-local frames. Consumer adapters and their end-to-end behavior
  remain required under D03.4-D03.5.
- D03.4 migrates Trash Amp's WAV, CUE/redumper, resampling, and transport-policy
  paths to the shared crates while keeping filesystem discovery, compressed-codec
  compatibility, and CPAL output in its adapters. Collector can consume an
  already-authorized representation as the shared opaque byte source without
  exposing an archive path. Shared compressed codec support remains required;
  physical output and private fixtures remain external qualifications.
- D03 is locally complete and reviewed across Junk Libs, Trash Amp, and Collector.
  The headless default audio contract depends only on `thiserror`; decoding, disc
  readers, and resampling are optional mechanisms. Compressed shared decoding,
  richer effects, event delivery, published dependency pins, and hardware/private
  qualification remain required or external as recorded in the Wave 3 ledger.
- W3.R traces every Wave 3 root to executable tests, browser interaction, or an
  explicit external qualification in [the integration review](wave-3-review.md).
  The complete local Collector gate and affected sibling gates pass; no D04/D05,
  provider, derivative, event-streaming, document-upload, plugin, or Wave 4
  capability is counted as implemented.
- Optional private TLS management listener bound to an explicit interface address.
- WebUI onboarding distinguishes client-only and private management surfaces, offers first-collection/first-item next steps, preserves structured API error codes, and separates wrong-origin, expired-session and role-denial guidance without weakening listener or authorization policy.

## Wave 4 plan

[Wave 4 implementation handoff](wave-4-plan.md) defines the next 28 ordered
implementation/review steps from baseline `c5c3ecf`. It covers game/CD processing,
metadata, derivatives, backup/restore and native connection/trust/failover services.
W4.0 is reviewed and corrected. W4.1's portable processing/metadata types,
source-bound package-reader and application port contracts, deterministic source
fingerprints, and split game/music module files are implemented locally, reviewed and corrected. They add no analyzer, provider I/O, persistence, API, or runtime support
claim. The baseline/dirty-state inventory, API drift assessment, operation
matrix, exact dependency/tool selections, migration reservations, generated fixture
recipes and external-oracle instructions are recorded; schema reservations are
advisory under the pre-release policy.
W4.2 durable records and executors are implemented locally, reviewed and corrected.
D04.1–D04.3 and D05.1–D05.2 are reviewed and corrected locally; D05.3 is reviewed and corrected locally; D07.1 is reviewed and corrected locally; D07.2 is reviewed and corrected locally; D07.3 is reviewed and corrected locally. D06.1 durable metadata authority and deterministic resolution are reviewed and corrected locally; D06.2 providers and artwork are reviewed and corrected locally; D06.3 metadata review and effective presentation are reviewed and corrected locally.
Archive schema 4 and catalog schema 8 cover processing observations and compact occurrences,
streamed snapshots, metadata authority, and conversion recipes; private schema 14
persists frozen Job input, provisional Import observations, active catalog
selection, guided-ingest intent and restartable staging cleanup. Import publication
rebinds evidence to its committed Representation with checked file mapping and
retained provenance. Registered dispatch, authorization, restart/cancellation,
idempotent retry, archive-only recovery, and current-versus-historical summaries
have executable coverage. D04.2 adds a configured local-DAT provider using
`junk-libs-dat`, immutable exact-byte snapshots, rebuildable row indexes and
complete/ambiguous/partial/inconclusive game matching. D04.3 registers the
qualified game operations, connects Import and committed-Capture jobs, and exposes
admin/curation status without leaking configured paths. D04.1 adds a
reader-only shared game-format crate for exact raw, trainer-free iNES/NES2, N64,
explicit SNES/PCE, ordered raw-2352 CUE, and pinned-oracle CHD hashing. Collector
maps those outputs into validated module results; D04.3 registers the qualified
runtime subset, while CHD remains an external/unexposed gate. Actual Retro Junk
consumers use the same algorithms. The validation
evidence and remaining limits are recorded in [validation](validation.md).
The native user-testable surface in this wave is a diagnostic CLI; Slint,
offline-library management and the Trash Amp provider remain later tickets.

## Usability/lifecycle fixes — 2026-09-21

FX-01 in [the fix plan](ux-fix-plan.md) implements URL-backed browsing,
immediately visible responsive details, fenced evidence reads, explicit read-error
recovery, and server-paginated title/recent/artist/platform sorting with wishlist,
files-attached, playable-audio and requester-scoped attention filters. Catalog 7
adds rebuildable browse keys; browsing does not hash payloads. Local acceptance passed on 2026-09-21; see the [validation record](validation.md#fx-01-browsingnavigation--2026-09-21)
for browser evidence and exact Rust check/rerun outcomes.

FX-02 implements module-owned eligible-file planning, typed scan states, atomic
requester-scoped active-work convergence, durable parent/direct ownership, and
compact repeat-result occurrences. Actual checks retain first/last dates and counts;
receipt replay does not advance them. Equivalent reference bytes and parsed inputs
share result content while occurrences preserve snapshot provenance. Reindex retains
legacy records, reconstructs occurrence references and rejects inconsistent links.
The processing panel restores selected settings/current work, leads with View result,
and keeps unrelated editing usable. Manual import analysis resumes polling after
reopening. Admission uses recorded fingerprints; workers check actual bytes with
cancellation. [Local acceptance passed](validation.md#fx-02-scan-lifecycle--2026-09-21) on 2026-09-21.

FX-03 implements Work & history with collection/item/unattached, package/file-name,
date and lifecycle filters, bounded server pages, import/preparation grouping and
execution-attempt detail. Private schema 15 retains dismissal/restoration and durable
clear intents; cancellation and eligible staging cleanup settle before archival.
Per-target review shows eligibility, byte effects and partial outcomes. Pending clear
and dismissed history fence stale admission/retry; shared work remains protected.
Manual and guided imports reopen from server identities, including upload resumption.
Generated track files remain in the named Music track groups; ineligible derivative
CD-operation forms are suppressed using scan readiness while evidence stays readable.
FX-03 [local acceptance passed](validation.md#fx-03-work-and-history--2026-09-21) on 2026-09-21; no external release qualification is claimed.

FX-04 implements focused editing/intake (2026-09-24); FX-05–FX-07 remain
proposed: manual removal/restore/clean reimport, account recovery and the
whole-interface review. See the current FX-04 section below for exact scope.
LA3's second-device qualification and broader release gates remain open.

## Required before the planned first release

- **Album/background-queue gate, added 2026-09-15:**
  [D07.4–D07.5](background-work-plan.md) are reviewed and corrected locally. Whole-album submission, durable per-member state, independent
  enqueueing, retry/restart and shared-safe cancellation now use the existing bounded
  Jobs. U05 must still audit all other interfaces for unnecessary background-work
  blocking before web completion; LA2 does not claim that application-wide gate.

- **Interface readiness gate, added 2026-09-15:** LA1 implements typed readiness,
  unique-anchor defaults and prerequisite guidance for its guided import journey.
  The broader [U05.1–U05.3](interface-readiness-plan.md) operation matrix and
  application-wide audit remain planned before web UI completion.

- **Web UI completion gate, added 2026-09-15:** LA1 implements bounded personal
  import/attempt history, reopen, retry/cancel, staging reclamation and published
  association correction for guided intake. P04 search/filter, bulk cleanup,
  dismiss/archive/restore, cleanup-failure UI and its full completion review remain
  planned. See the [import-management plan](import-management-plan.md).

1. Implement authenticated certificate rotation (S04) and complete the external security qualifications: real password-manager/platform and hardware passkeys, browser-trusted remote HTTPS, native platform secure storage, and independent review.
2. Complete independent accessibility/security review for the locally implemented
   P04 workspace and Job controls. The legacy upload and administrator inbox routes
   remain compatibility adapters scheduled for API-major removal. Document
   attachments, event streaming, and offline artwork remain unimplemented.
3. Complete independent D04-D05 real-dump verification oracles and review the
   locally implemented MusicBrainz/CAA enrichment and DAT metadata integration.
   D02/D03 and D04.1 complete the reusable disc/audio/game primitives locally; no
   Collector catalog verification is currently claimed.
4. Complete the active local-alpha deliveries before returning to P05.1 portable
   export/change inventory. The local deterministic MusicBrainz/CAA workflow
   passes with actual published artwork, and LA3 adds one live provider result;
   other-browser and deployment qualification remain unclaimed.
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

## Collection-type correction checkpoint

[CT.1](collection-types-plan.md#ct1--ownership-and-module-contracts) is implemented,
reviewed and corrected. Game processing
results and music verification policy now belong to their modules; only genuinely
shared CD/PCM coordinates and layout remain in core. Module descriptors declare
bounded scalar/read-only fields and explicit operation scope. Owner-dispatched
typed patches preserve omitted and untouched raw values, distinguish explicit null,
and reject unknown/wrong-typed edits. Successful operation output and typed metadata
domain overrides require owner validators.

[CT.2](collection-types-plan.md#ct2--persistence-and-existing-collections) is
reviewed and corrected locally. New collections persist an
Assigned owner selected from the registered module registry; creation idempotency,
recovery intent, collection DTOs and generated API contracts include that identity.
Pre-CT.2 manifests deserialize as LegacyUnassigned without rewriting original files
and remain readable; graph publication rejects unassigned collections. Local administrator inspection reports exact
current item-domain counts, assignment eligibility, pending work, active authority
conflicts and preserved historical overrides. Explicit homogeneous/empty assignment
uses an optimistic, provenance-bearing collection metadata revision through the
existing journal and is reconstructible from archive alone. Generic graph validation,
normal publication and reindex reject item-domain/assigned-owner mismatches.

Assignment validates full active authority payloads and pending operation scope;
unknown active jobs block assignment, and retries honor the original expected revision.
The CLI requires the stopped server and exclusive store access. A minimal browser
creation picker now sends the explicitly selected module using the existing feature
list.

[CT.3](collection-types-plan.md#ct3--application-policy-and-jobs) is implemented
and reviewed locally. One application policy now gates all implemented
collection-content mutation families on Curate authority, Assigned binding, an
installed owner and supported Item schema. Collection-domain analysis/processing
must belong to that owner; explicitly SharedMedia operations remain usable across
collection types. Durable processing/acquisition jobs freeze the binding and recheck
it, operation ownership/version and module availability at execution/publication.
Long import publication rechecks policy after copying bytes and before Capture
commit, preserving sources on denial. Stored unsupported schemas cannot be replaced
through ordinary edits. Already committed integrity recovery does not rehash payloads.
Already committed recovery completes without modules or grants. Legacy/unavailable
reads, authorized existing media/Artwork, personal activity and access administration
remain available under their independent policies.

[CT.4](collection-types-plan.md#ct4--user-facing-integration-and-completion) is
reviewed and corrected locally. Authenticated clients receive bounded
collection-type descriptors from the application registry; public feature discovery
does not expose that registry. Collection and Item DTOs carry server-computed content
editability and distinct assigned, legacy-unassigned, missing-owner and unsupported-
schema availability. Generic web forms inherit the collection owner, render module-
declared text/integer fields, visibly preserve structured fields as read-only, and
send owner-mediated patches rather than replacement module JSON. Browse labels and
safe summaries are descriptor/presentation driven, with no game/music module-ID UI
switch. A test-only third module passes create/edit/browse/reindex and forged-input
validation through real services, and Chromium covers a valid homogeneous archived
collection whose owning module is absent.
Unavailable module fallback remains readable; runtime plugins, automatic conversion,
legacy graph splitting and type changes are outside this refactor. Historical field
authority remains preserved verbatim and is never activated by assignment.

CT.4 review also fixes stable patch-command receipts, unsupported-schema form
controls and descriptor-independent access administration. Integer edits cannot
silently round beyond the browser's safe range. The generic third-module browser
fixture checks submission types; its production persistence remains covered by
real HTTP/service/reindex tests. D04.1 subsequently implemented and qualified the
bounded shared reader/result primitives; D04.2–D04.3 now integrate their local DAT
and game workflows and have passed local D04 review.

D04 review preserves exact DAT snapshot length/digest checks on every parse, rejects
malformed catalog entries rather than silently omitting them, and checks job
cancellation during package reads and catalog scans. Curator evidence reads remain
available without an installed owner module. The catalog currently uses bounded
per-job parsing and linear comparison; large-corpus/index performance is unqualified.

D05.1 review keeps PCM source concatenation/decoding in the shared disc crate.
The bounded AccurateRip verifier searches v1 across the declared interval and v2
at explicitly recorded candidate offsets. Missing v2 coverage yields incomplete
search rather than mismatch. D05.2 must retain that distinction and require every
expected track for whole-disc success. No music runtime capability is registered yet.

D05.2 retains that distinction in the music-owned result contract. Per-track
evidence records both computed checksum versions/confidence, the inclusive v1
range, explicit v2 offsets, physical edge limitations, provider response identity,
implementation version and exact input digests. Whole-disc success requires every
database track at one unambiguous offset; only a complete non-edge-limited v1/v2
search can report mismatch. The provider crate owns a pinned HTTPS dBAR adapter
with bounded retries/time/body/rate/redirect policy and exact archived-byte replay.
Local HTTP fixtures cover data, no-data, malformed/misdirected/oversized input,
timeouts, cancellation, outage/cache replay and distinct refreshes.

D05.3 registers music Import analysis, CD structure and AccurateRip verification
with the compiled owner and runs them through the existing frozen P02/W4.2 jobs.
The small provider port is adapted into the same streaming snapshot publication
executor; one server-composed provider instance owns the limiter. The provider
parser re-reads the complete archived dBAR payload and checks length/SHA-256 before
returning neutral entries, while the music result retains that snapshot digest,
the frozen Representation fingerprint, every input digest, computation offset and
per-version coverage. Provider failure fails only its retryable acquisition Job;
404/no-data publishes a distinct snapshot and inconclusive result. Curation shows
separate structure and catalog evidence, explicit frame units, progress/history,
limitations and artwork in the same Item workflow. No FLAC preparation or playback
was added. Synthetic workflow qualification is local; the opt-in owned-dump runner
is present but external qualification is unavailable here.

D07.1 keeps `ConversionProfile::Flac` and `Opus { bitrate_kbps }` as readable
presets while sealing source fingerprint, selected music-owned PCM ranges, fixed
settings, tool/version and exact formats into recipe identity. CUE conversion
retains stored HTOA on track 1 and the D05 next-INDEX-01 ownership for later stored
pregaps; synthetic gaps, mixed mode and ambiguous ranges fail closed. The provider
adapter directly spawns configured `flac`/`opusenc`, bounds streams/diagnostics,
kills and reaps cancellation, clears only its supplied scratch handle, and never
receives archive paths or labels. Local `flac` 1.5.0 output matches every source
`i16` frame under independent Claxon decoding. Opus 96/128/192-kbps recipes and
20-ms/48-kHz delay-padding inspection are implemented, but `opusenc` is absent and
therefore unavailable rather than simulated. D07.2 now publishes explicitly requested
single-track derivatives through the existing frozen jobs and Capture journal.
Recipe, validation evidence and output bytes are immutable artifacts of that
Capture commit; failed/cancelled pre-commit publication advertises nothing, and
committed recovery finishes the original output without an encoder or a new access
check. Encoder versions and exact source/track ranges determine output identity.
Concurrent requests converge; current Capture associations and existing Item edits
survive publication. Reindex checks stored derivative bytes against validation
evidence. D07.3 adds an authorized ordered track/profile availability DTO, client
media reads, curation-only preparation, server-owned optional encoder probing,
generated API contracts and web preparation/progress controls. Published output
remains readable after restart without its module or tool. Strong content ETags
and full, partial, suffix, open-ended, unsatisfiable and If-Range behavior are
exercised through HTTP. Import, inventory, verification and media GET still do
not request conversion. The following bounded browser child is now implemented
locally and is reviewed. Its album views use these rows directly for Capture/source-scoped track selection;
the 2026-09-22 shell player owns the local queue and actual viewer FLAC playback
without adding a backend player
or conversion-on-read path.

### LA3 independent review (2026-09-20)

Catalog track coverage now rejects malformed BINs and unknown payloads rather than
silently excluding them; only CUE metadata is excluded, through module-owned policy
shared by the adapter and matcher. The manual cache helper validates existing
digests, bounds expansion and stages complete new entries. New DAT-o-MATIC network
requests are paused in code following error 1260; cached data remains usable.
Second-device qualification remains outstanding.

### Folder intake and upload feedback (2026-09-20)

- Add / import defaults to folder intake, with manual inventory as a separate mode.
  Modules group same-stem redumper packages, CUE directory packages, and supported
  game single-file formats. Nested anchored CUE folders remain separate; multiple
  CUEs in one directory stay together for review. Unassigned files are listed.
- Every accepted browser package uploads in sequence and has a durable entry and
  import record before bytes transfer. Closing interrupts browser transfer, not
  saved entries or subsequent server jobs; selecting the same originals resumes.
- Both upload UIs share actual request-body progress, server acknowledgement,
  retained-prefix checks, abort handling and safe lost-response retry. Server work
  without measurable units uses an indeterminate bar rather than invented percent.
- Background import notifications refresh the collection without selecting another
  entry and unmounting its player. Association details show the current destination;
  correcting an association retires an unused automatic placeholder while retaining independent inventory, manual information and other discs.

- Placeholder retirement is an application-owned, recoverable follow-up to a committed
  guided association change. Settled/cancelled intent plus the current association
  identify cleanup candidates, including gaps from earlier server versions. The
  adapter compares the whole hydrated item and metadata-authority revision under
  the archive writer lock before publishing the existing portable tombstone.
  Other captures, assets, specimens, wishlists, identity/domain edits and manual
  metadata authority prevent automatic retirement. The original guided workflow settles
  before retirement; no payloads are deleted. Browser refresh clears retired
  selections, and explicit reassociation follows the destination only if the moved
  source was selected, preserving unrelated playback.

### Persistent browser music queue — 2026-09-22

The user-requested bottom player is implemented and locally accepted; see
[validation](validation.md#persistent-bottom-music-player--2026-09-22).
One shell-owned audio element survives album navigation, catalog refresh and import
completion. Album views append a track or prepared album; original audio files can
also be queued. The player supports transport/seek, cross-album advance, reorder,
remove and clear. A bounded localStorage queue is scoped to browser origin/user,
restores paused on reload and is never written to the server. Without writable
localStorage, it remains usable for the current session. Access is rechecked before
playback and periodically; loss of access/session releases buffered audio. This
completes a user-requested portion adjacent to FX-04, not the whole FX-04 delivery.

### View separation requirement — 2026-09-22 (historical requirement)

FX-04 now explicitly owns separate import-history and selected-import detail/progress
views, with preserved return context, direct-link/reload and thousands-of-records
acceptance. The combined history/detail panel did not meet this requirement at the
time of writing; FX-04 now implements separate views. FX-07 verifies meaningful
browse/detail/task separation across the interface. See [the fix plan](ux-fix-plan.md).

The 2026-09-22 selector defects are corrected by FX-04's authorized, bounded
server search and pagination. General collection results are labelled as search,
without claiming detected matches. The [growing-list audit](fx04-scale-audit.md)
records remaining access-list work owned by FX-06 and broader FX-07 checks.

### Automatic CD provider lookup requirement — 2026-09-22 (historical requirement)

Guided ingestion already acquires MusicBrainz data automatically with a disc ID
from its CD analysis. The metadata editor still presents an explicit optional
lookup and offers a search built from displayed metadata. FX-04 must make automatic
dump-derived identity lookup consistent across intake paths and remove reliance on
that explicit control for ordinary ingestion. Dump filenames/folders/timestamps,
including placeholder titles inherited from them, must never become automatic text
queries. Missing identity and provider/no-match/ambiguity states must remain honest;
manual metadata authority and original preservation remain protected. This broader
requirement was a planning statement. The primary new/existing intake path now
shares the guided coordinator and its derived-disc-ID acquisition. The advanced
manual review path remains for imports admitted through the older provisional API;
its explicit provider lookup is a recovery action, not automatic acquisition.

### FX-04 focused editing and intake — implemented 2026-09-24, locally accepted 2026-09-25

Review corrections on 2026-09-25 bind existing-destination revision checks to
first admission after receipt replay, initialize editor drafts when a guided
publication adopts a different Item, retain the chosen section on later Item
updates, and refresh every loaded Work & history page. See
[validation](validation.md#fx-04-review-corrections--2026-09-25).

The workbench presents Details, Copies, Files, Music, Artwork and Advanced evidence
as navigable task sections. Inventory title and edition edits are direct; metadata
authority/provenance is progressively disclosed. Files uses guided intake for new
and existing entries, with destination, source, readiness, preservation-only
choice, upload and durable server progress. Existing-Item admission freezes its
revision and preserves its other Captures and independent inventory. Missing
companions and provider/tool availability are visible before submission. The
existing private guided-intent table gains additive `created_item` and `analysis`
fields without resetting older intents; the private schema remains v15. The
timestamp-named music fixture confirms automatic MusicBrainz requests use the
parsed disc identifier. No-match, multiple-candidate, missing-identity and provider
failures have distinct progress text; manual metadata authority survives refresh.

Work & history now uses separate list and focused selected-work views, an
authorized URL identity and Back/Forward/reload. Import detail and current intake
do not preload the catalog or mount the global history. The destination chooser
searches server-side with 50-result pages, selected-entry context and revision
recheck. Catalog snapshots and inbox package selection are paged. The
[scale audit](fx04-scale-audit.md) records all growing surfaces and scope of later
access and history work. A temporary-server browser journey passed with 3,000
entries and 3,000 imports, including selection beyond the former 1,000-entry cap.
