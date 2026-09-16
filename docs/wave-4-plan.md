# Wave 4 implementation handoff

Status: **W4.2 reviewed; CT.1–CT.4 and D04.1–D04.3 reviewed and corrected; D05.1–D05.2 reviewed and corrected locally; D05.3 reviewed and corrected locally; D07.1–D07.2 reviewed and corrected locally; D07.3 reviewed and corrected locally; bounded browser-player child reviewed and corrected locally; D06.1 reviewed and corrected locally; D06.2 is reviewed and corrected locally**. Baseline: `c5c3ecf` (Wave 3 plus recovery
and upload-resume review fixes), 2026-09-11. This plan covers D04, D05, D06,
D07, P05, C01, S04, and C02. It supplements, rather than replaces,
[release requirements](release-requirements.md), [architecture](architecture.md),
and [the handoff backlog](handoff-backlog.md).

The implementation steps below deliberately settle scope and ownership. They are
not permission to mark format correctness, security, or unavailable external
fixtures as passed by inference. A less capable implementing model can execute
one child at a time. A reviewer must inspect the named boundary checkpoints;
the implementing model must not certify its own missing oracle or invent a
protocol when a specified contract cannot be implemented.

## Inserted prerequisite: collection ownership

The user approved planning homogeneous, module-owned collections after W4.2.
Implement [CT.1–CT.4](collection-types-plan.md) with its review checkpoints before
D04.1. It supersedes domain-type placement and universal metadata-field decisions
below where explicitly stated. W4.2 remains reviewed. CT.1–CT.3 are reviewed and
corrected. CT.4's authenticated descriptors, availability-aware permissions,
owner-driven forms and unavailable-owner fallback are reviewed and corrected locally. D04.1’s bounded extraction/qualification checkpoint is now reviewed and corrected.

Downstream contracts now consume collection ownership rather than reintroducing a
kind switch: D04/D05 publish results through their registered module owner and
validator; D06 writes module-owned metadata authority; D07 retains explicitly
SharedMedia audio/conversion use across collection types. C01 transports descriptors,
binding and safe fallback DTOs, while C03/offline caches retain binding and artwork.
R01 import must explicitly assign homogeneous archives and report mixed archives for
inspection; it must not recreate a mixed writable collection.

## Execution instructions

1. Read `AGENTS.md`, architecture, implementation status, release requirements,
   this plan's fixed decisions, and the next child. Inspect `git status` in each
   affected repository. Preserve all unrelated edits and user configuration.
2. Start with W4.0. Then follow the serial order below. Do not begin several roots
   at once or delegate unless the user explicitly requests it. Do not change
   sibling repositories merely because their existing checks have unrelated failures.
3. Implement only the current child and necessary regression fixes. Add a failing
   behavioral test first for the state machines, normalization, source binding,
   authorization, retries, and recovery cases listed below. Do not manufacture
   tests that copy the implementation's arithmetic as their expected answer.
4. Follow [pre-release evolution](decisions/pre-release-evolution.md): change APIs and
   schemas with their callers when useful; preserve actual user data through explicit
   conversion or fail-before-write diagnostics. Use typed commands,
   `CommandContext`, application authorization, and existing publication/job
   mechanisms. Do not create a second importer, scheduler, or trust policy.
5. Run the child's checks, then inspect the diff against its stop conditions.
   Update this ledger, implementation status, validation, and the root backlog
   entry with actual evidence and remaining limitations. A build is not acceptance.
6. Report a bounded child ready for review. Make commits only when authorized;
   record actual SHAs after committing. Shared-library extraction and Collector
   integration are separate changes. Never invent a commit or reuse a historical
   test result as evidence for a changed sibling.
7. If blocked, record the exact missing input or incompatible contract and the
   last passing state. Work on another explicitly independent child only if it
   does not bypass that dependency. Do not replace real behavior with a mock in
   production. Ordinary implementation choices and tests need no extra approval.

### Serial implementation order and ledger

Rows without a completion/review result remain **not started**. Dependencies not listed here are already
implemented in Waves 1–3. “Review” means a technical checkpoint, not a request
for permission to run ordinary tools. Root review includes all its child tests.

| Order | Child | New dependencies | Deliverable | Checkpoint |
|---|---|---|---|---|
| 1 | W4.0 | none | Baseline, fixture and dependency lock-in | Reviewed/corrected 2026-09-11; proceed to W4.1 |
| 2 | W4.1 | W4.0 | Package readers, typed results and source binding | Reviewed/corrected 2026-09-11; proceed to W4.2 |
| 3 | W4.2 | W4.1 | Durable observations, snapshots and job integration | Reviewed/corrected 2026-09-11; proceed to D04.1 |
| 4 | D04.1 | W4.2 | Qualified game primitive extraction | Reviewed/corrected 2026-09-13; sibling gates passed |
| 5 | D04.2 | D04.1 | DAT snapshots and complete-match policy | Reviewed/corrected 2026-09-13; vectors pass |
| 6 | D04.3 | D04.2 | Game jobs, curation and supported-operation matrix | Reviewed/corrected 2026-09-13; proceed to D05.1 |
| 7 | D05.1 | W4.2 | Exact CD PCM/layout and bounded checksum readers | Reviewed/corrected 2026-09-14; proceed to D05.2 |
| 8 | D05.2 | D05.1 | AccurateRip acquisition and result policy | Reviewed/corrected 2026-09-14; proceed to D05.3 |
| 9 | D05.3 | D05.2 | CD jobs, curation and fixture qualification | Reviewed/corrected 2026-09-15; proceed to D07.1 |
| 10 | D07.1 | D05.1, W4.2 | Immutable conversion recipes and exact encoders | Reviewed/corrected 2026-09-15; proceed to D07.2 |
| 11 | D07.2 | D07.1 | Recoverable derivative publication | Reviewed/corrected 2026-09-15; proceed to D07.3 |
| 12 | D07.3 | D07.2 | Availability, ranged access and web controls | Reviewed/corrected 2026-09-15; bounded browser-player child reviewed and corrected locally |
| 13 | D06.1 | D04.3, D05.3 | Durable metadata authority and resolver | Reviewed/corrected 2026-09-15; proceed to D06.2 |
| 14 | D06.2 | D06.1 | MusicBrainz, CAA and DAT-backed game metadata | Reviewed/corrected locally 2026-09-15 |
| 15 | D06.3 | D06.2 | Metadata review and artwork UI | Reviewed and corrected 2026-09-16; D06 root review complete |
| 16 | P05.1 | D06.3, D07.3 | Portable export and change inventory | Review |
| 17 | P05.2 | P05.1 | Consistent full backup and restore | Review |
| 18 | P05.3 | P05.2 | CLI/admin integration and recovery drills | P05 root review |
| 19 | C01.1 | W4.0 | Persistent connections and platform secret storage | Review |
| 20 | C01.2 | C01.1 | Authenticated route registration and typed queries | Review |
| 21 | C01.3 | C01.2, D07.3 | Persistent transport integration tests/harness | C01 root review |
| 22 | S04.1 | C01.3 | Versioned rotation state and transition contracts | Security review |
| 23 | S04.2 | S04.1 | Server rotation, grace endpoint and recovery | Security review |
| 24 | S04.3 | S04.2 | Client trust updates and offline recovery | S04 root review |
| 25 | C02.1 | S04.3 | Candidate discovery and authenticated probes | Review |
| 26 | C02.2 | C02.1 | Health selection and ranged transfer failover | Review |
| 27 | C02.3 | C02.2 | LAN/tailnet/public network matrix | C02 root review |
| 28 | W4.R | all roots | Integrated acceptance trace and handoff | Final review |

W4.0 evidence: Collector was inspected at `c5c3ecfb2ada2624e780ac128b290e4c37f1d468`.
The sibling/API drift inventory, exact dependency/tool selections, migration
reservations, fixture provenance and external-oracle instructions are recorded in
[`decisions/reuse-map.md`](decisions/reuse-map.md) and [`fixtures/wave4`](../fixtures/wave4/README.md).
No production capability, schema, sibling checkout, or support claim changed in
this documentation-only child. The following local baseline gates passed on
2026-09-11: `python3 scripts/check_architecture.py`; `cargo fmt --all -- --check`;
`cargo test --workspace --locked` (142 tests); `cargo clippy --workspace
--all-targets -- -D warnings`; `npm --prefix web run check`; `npm --prefix web run
check:api`; `npm --prefix web run build`; and the fixture TOML/digest reproduction
check (8 manifests, 22 vector/composite digests). Review made this check permanent
as `python3 scripts/check_wave4_fixtures.py`, also invoked by the architecture
checker, and clarified negative TLS/metadata identity fixtures. No web/server asset changed, so browser smoke and
a server rebuild were not run for this documentation-only child. This checkpoint
is reviewed and cleared for W4.1.

W4.1 evidence: portable processing/metadata values now include scoped package-file,
observation, snapshot and recipe identities; deterministic versioned source
fingerprints; typed game/CD outcomes and unit-safe CD coordinates; conversion
recipes; and metadata authority. Module hosts expose collection/source-bound
inventories and opaque per-file readers, with a one-file compatibility adapter and
bounded logical relative-name resolution. Application contracts freeze processing
inputs/output identity/time and define snapshot/provider/execution ports without
implementing their mechanisms. Game and music modules were split into focused files,
and a third test module exercises a two-file package. Contract tests cover access
denial, missing/ambiguous files, unsafe inventories, raw unknown round trips,
support-state distinctions, source changes, the golden fingerprint encoding and
frozen job values. No archive/private schema, provider request, analyzer, API, or
runtime support claim was added. At the W4.1 checkpoint, W4.2 had not started.
Review corrections additionally enforce canonical recipe digests and typed conversion
settings, text-only single-authority metadata overrides, package path collisions,
fingerprint/inventory binding and valid audio success evidence. CapabilityId is an
alias of the validated OperationId; the generated API schema remains unchanged.
Acquisition jobs no longer require a fictional file source, snapshots stream their
bytes, and observation queries are bounded.
The W4.1 local gates pass: architecture/link/fixture checks; formatting; locked
workspace tests; warning-free full clippy; TypeScript and OpenAPI drift checks; web
build; and a fresh `collector-server` build. Browser smoke is not applicable because
the child adds no web asset, route, or browser behavior.

W4.2 evidence: archive schema 4 publishes and reindexes immutable processing
observations, streamed provider/catalog snapshots, metadata authority, and
conversion recipes; private schema 10 persists frozen Job input and provisional
Import observations. Store-backed Import and Representation readers bind opened
files to a validated frozen inventory. Registered operation ownership drives
executor dispatch, while application services enforce validation and grants.
Import publication explicitly rebinds provisional observations to the committed
Representation after checking ordered file identities, lengths, and digests, and
retains the provisional fingerprint and mapping as provenance. Authorized summaries
distinguish current from historical results without deleting history. Recovery,
cancellation/restart, retry/idempotence, revocation timing, streamed snapshot
integrity, archive-only rebuild, unknown-record preservation, and changed-input
currency are covered by tests. No game/CD analyzer or production network provider
was added by W4.2. D04.1 subsequently added only its bounded reader/result
primitives and no registered runtime operation. Final gates and limitations are recorded in
[`validation.md`](validation.md). Compatibility-only work is limited to reading and
reindexing existing archive schema 3 and exposing its provenance limitation; no
permanent adapter was added merely for a prior Wave checkpoint.

C01.1–C01.2 can be assigned independently after W4.0, but the default is the
serial order above. C01 precedes S04; C02 follows both. Do not introduce a circular
“rotation required for initial pairing” dependency. P05 follows domain records
so export tests cover the new portable data. D07 depends on exact CD reading,
not on a successful AccurateRip match or an online provider.

The [music-first delivery assessment](music-first-milestones.md) proposes moving
D07 and C01 forward and bringing C07 ahead of the full desktop/offline wave. It
now advances D07 ahead of D06 in this ledger. The later C01/C07 acceleration
remains a proposal to settle before assigning those roots.

## Contract checkpoints for remaining work

Follow [pre-release evolution](decisions/pre-release-evolution.md) before each root:
settle its immediate producer/consumer contract, then implement complete workflows.
Do not scaffold speculative production endpoints or success-shaped placeholders.
D04/D05 must review operation-specific result validation and authorized snapshot
access before connecting analyzers. C01 establishes shared read DTO ownership before
native clients consume it. Later discoveries may revise these interfaces together.

W4.2 review corrected host scope enforcement, cursor pagination, source currency,
pre-commit evidence rebinding, archive envelope identity checks, atomic reindex,
provider spooling/grant rechecks, recipe replay and command path binding. It also
fixed final-owner lock release exposed by parallel recovery tests. Final validation
is recorded in [validation](validation.md). D04.1 is reviewed and corrected;
D04.2 and D04.3 are reviewed and corrected; see [the review](wave-4-review.md).

## Fixed architecture and scope decisions

### Files and ownership

Existing entry points are `crates/collector-core/src/lib.rs`,
`crates/collector-modules/src/lib.rs`, `crates/collector-app/src/lib.rs`,
`crates/collector-store/src/{lib,files,archive_v3,publication,jobs,provisional_imports}.rs`,
`crates/collector-server/src/{lib,main}.rs`, and
`crates/collector-client/src/lib.rs`. Their existing tests are the regression base.
New files named below are planned destinations, not files claimed to exist today.

- Core: add focused `processing.rs` and `metadata.rs` for portable typed values.
  Re-export only deliberate public contracts from `lib.rs`.
- Modules: split existing built-in game/music implementations into `games.rs` and
  `music.rs` as they gain behavior. Keep registry/host contracts separate. Modules
  use reader-based algorithms, never HTTP clients, SQLite, process spawning or paths.
- Application: add `processing.rs`, `metadata.rs`, `derivatives.rs`, `backup.rs`.
  Services own authorization, supported operations, candidate ranking, and command
  semantics. Avoid growing the existing `run_next_job` match with an entire
  implementation per kind; delegate executors to their owning service.
- Store: add `processing.rs`, `metadata.rs`, `derivatives.rs`, `backup.rs` for
  mechanism ports; reuse publication, receipt and job code. Source payload access
  stays here. New tables are projections or private state, never a second archive.
- Providers: add a `collector-providers` infrastructure crate for bounded HTTP,
  catalog parsing adapters, recorded-response tests and optional encoder-process
  adapters. It may depend on core, app contracts and narrow junk-libs crates;
  never on Store, server or GUI. Server composition injects its ports into app.
  Register this dependency direction in the architecture checker before adding it.
- API contracts: add `collector-api-types` in C01.2 for the shared client-facing
  read DTOs moved from application code. It depends on core plus serialization/
  schema libraries only; app, client and server may depend on it. It owns no
  services or authorization. Move callers together; compatibility re-exports are
  optional temporary conveniences. Regenerate OpenAPI/TypeScript with changed contracts.
- Client: add `connections.rs`, `transport.rs`, `trust.rs`, `discovery.rs`,
  `transfer.rs`, plus feature-gated platform storage. No Store dependency.
- Web: add focused processing/metadata/derivative panels to curation rather than
  embedding provider logic in React. They use generated API contracts and the Job ledger.

The architecture checker must cover new server modules as well as `lib.rs` and
`passkeys.rs`; moving forbidden calls into a new file must not evade it.

### Portable records and versioning

W4.1 specifies these types before persistence or APIs:

| Contract | Required meaning |
|---|---|
| `PackageFileId` | Typed identity scoped to a Representation or provisional Import; not a filesystem path |
| `SourceFingerprint` | Ordered file identities, lengths and raw SHA-256 values, plus layout/profile digest when applicable |
| `ObservationId`, `SnapshotId`, `RecipeId` | Distinct durable types; neither a path nor an unvalidated provider string |
| `ProcessingObservation` | Operation/version, exact source fingerprint, raw source observations, typed result and limitations |
| `OperationSupport` | Supported, unavailable dependency, unsupported layout, or externally unqualified; include reason |
| `games::CatalogMatchResult` | Game-module no match, unique complete match, ambiguous complete matches, or partial identification; not a Boolean |
| `music::AudioVerificationResult` | Music-module per-disc/per-track status, signed shared sample-frame offset, checksum version and confidence |
| `ConversionRecipe` | Versioned codec settings, exact input/track layout, encoder version, output sample format and digest |
| `MetadataAuthority` | Automatic selection, explicit candidate selection, common title/subtitle overrides, optional owner-bound typed domain overrides and retained unknown fields |

Use enums/newtypes for owned semantics. Module-specific payloads and raw provider
bodies remain lossless envelopes. IDs native to an external provider may be opaque
strings inside validated provider-specific types; they are not universal domain IDs.

W4.2 currently writes envelope v4 and reads earlier archives. Those are implemented
formats, not permanently supported wave baselines. Follow the pre-release policy
when changing them: preserve actual archive data, unknown envelopes and evidence
meaning, but do not add historical readers solely for checkpoint compatibility.
Current catalog/private versions are recorded in implementation status. Inspect
actual applied state before modifying a schema; coordinate real conversions and
rebuild projections instead of allocating a migration for every ticket. Earlier
W4.0 reservations are advisory, not an architectural constraint.

Raw DAT files, relevant provider responses, observation history, metadata authority,
and conversion recipes needed to explain collection records are **portable archive
data**. Account credentials, provider secrets, route trust, jobs and scratch are
**private state**. Cache indexes may be rebuilt; a cache eviction must not erase a
provider observation already cited by an archived result. Snapshot files referenced
by committed evidence must survive backup/reindex/provider outages.

### Processing and publication invariants

- Extend the module host with bounded package inventory and per-file readers.
  `representation(id)` currently selects a stream and is insufficient for BIN/CUE
  and redumper packages. Preserve it as a compatible single-file adapter; return
  an explicit ambiguity error for a multi-file package. No host method accepts an
  arbitrary source path. Resolve CUE relative names against the declared inventory.
- Provisional analysis may read staged bytes only through an app-authorized,
  Import-scoped reader. Its observations remain provisional until publication
  carries them into portable history. Do not claim staged analysis is committed
  verification evidence. Re-run/rebind checks if source fingerprints change.
- Compute identity from actual bytes, not extension/name or previously stored hashes
  alone. Store full raw-file digests independently of normalized catalog digests.
- Each processing Job freezes operation/version, source fingerprint, selected
  existing snapshot, candidate/recipe selection and stable output IDs. Acquisition
  jobs first freeze the provider query; when a response arrives, archive its bytes
  and bind its SnapshotId in a durable checkpoint before analysis/publication.
  Retry may repeat acquisition only before that checkpoint; afterward it uses the
  recorded response. A request for a newer catalog/recipe/response is a new Job.
- Before publishing, recheck the source identities and grant eligibility. If access
  was revoked before commit, stop with a safe failure; after an archive commit,
  complete internal reconciliation even if the user no longer has access. Future
  reads still enforce current authorization.
- Network acquisition and long analysis happen outside the archive writer lock.
  Commit through the existing fence, journal and projection transaction. A crash
  after the archive commit finishes the same result; do not run a newer provider
  query or reopen a pre-commit revision decision while recovering.
- Observations/Evidence are append-only. “Current” is a read-model comparison of
  their exact inputs to the current source, not an update to old successful rows.
  No provider outage deletes history or clears manual metadata.
- Job limit stays two globally/one per user. Stream I/O in bounded chunks and check
  cancellation between chunks/provider pages. Codec/parser resource limits must
  be explicit; never load an entire CD plus checksum prefix arrays by default.

### HTTP and user-visible surfaces

Use annotated Axum routes and generated contracts. Concrete route families:

| Surface | Routes/behavior |
|---|---|
| Client | Authorized read-only processing summaries, derivative availability, metadata display and media; connection/trust metadata through explicitly enrolled routes |
| Curation | `POST /captures/{id}/operations`, `POST /items/{id}/metadata/refresh`, `PUT /items/{id}/metadata/authority`, `POST /captures/{id}/derivatives`; all return real Jobs or revisioned command results |
| Administration | Catalog source/snapshot configuration, provider settings, export jobs/status and TLS rotation lifecycle; disabled public administration removes backing APIs too |
| CLI | Catalog load, stopped full backup, restore validation/restore, archive-only access reassignment, rotation recovery, and native connection diagnostic harness |

Use existing collection authorization and submitter-private Job views. Operation
submission takes a registered capability and typed versioned input; the server
must not accept arbitrary executable names or filesystem locators. Keep raw provider
observations curator-only. Viewer summaries expose useful outcomes/limitations,
not acquisition, private notes, local filenames/paths, secrets or provider credentials.

The web can actually run verification, review metadata and request/download music
derivatives by the end of this wave. Native transport is testable through a small
CLI harness. The Slint app, native playback UI, durable offline library, activity
sync, events, iOS and Trash Amp server provider remain Wave 5/follow-on scope.
One-way Retro Junk archive migration remains R01; P05 is backup/restore of
Collector archives, not a substitute for that migration.

## Child instructions

### W4.0 — Baseline and qualification lock-in

**Edit:** this ledger, `docs/decisions/reuse-map.md`, fixture manifests and the
backlog only. Read sibling AGENTS files before inspecting/extracting their code.

1. Record Collector HEAD and the HEAD/dirty status of Junk Libs, Retro, Phono and
   Trash Amp. Verify D02/D03 APIs still match this plan. Do not overwrite the old
   dated reuse inventory; add a new dated section with discrepancies.
2. Enumerate every advertised game/CD operation with input domain, output domain,
   exact API, feature dependencies, fixture/oracle and license provenance. Keep
   Retro's license placeholder and unpublished shared pins visible until resolved.
3. Establish per-root fixture directories under `fixtures/wave4/`; store recipes,
   tiny legal vectors, provenance, expected results and digests. No private dumps.
   Record environment-variable instructions for externally supplied real dumps.
4. Read actual API fields/source layouts; record final migration numbers and the
   exact catalog/encoder library/tool versions to use. No floating Git dependency.
5. Update architecture for the planned provider and API-contract crates and the boundaries above.
   Do not implement an extractor/provider under a documentation-only child.

**Pass:** fixture recipes reproduce declared bytes/results; baseline Collector
checks pass or unrelated failures are explicitly recorded. **Stop:** API drift,
missing provenance or dependency restrictions that change a selected extraction
need review before that extraction. Missing private oracles do not block synthetic
implementation but do block claiming those formats fully qualified.

### W4.1 — Typed processing and package-reader contracts

**Edit:** core/module/app contracts; architecture checks and contract tests.

1. Add the portable types above and typed game/CD result payloads. Represent CD
   sectors and PCM frames with separate types; offsets are signed stereo sample
   frames, not bytes, sectors, floating-point seconds or per-channel samples.
2. Add package inventory/per-file host methods with length and safe display name;
   the actual handle is opaque. Inventory references cannot escape their authorized
   Import/Representation. No filesystem-layout string crosses into domain code.
3. Define source-bound processing queries/commands and provider/snapshot ports.
   Providers return raw bytes plus bounded parsed observations; domain policy
   interprets them. Define job execution context with progress/cancellation and
   frozen source/snapshot identities.
4. Split module implementations into focused files without changing current output.
   Exercise a test-only third module over a multi-file package to prove the new
   contracts do not special-case only games and music.

**Tests first:** wrong collection/file/Import denied; malformed inventory, missing
file and ambiguous single-reader rejected; unsupported operation distinct from
failure; unknown module/raw data round-trips; changing one track changes the source
fingerprint even if the selected identification track stays identical.
Fingerprint encoding is versioned and deterministic: preserve declared file order,
serialize typed fields in a fixed order with sorted map keys, prohibit floating-point
coordinates, and hash the exact encoded bytes with SHA-256. Golden encoded-byte
fixtures protect cross-version identity. Do not hash Debug strings, filesystem paths,
or unordered maps. Freeze timestamps/output IDs before committing Job results.
**Stop:** no fake analyzer support, provider network calls or new persistent rows yet.

### W4.2 — Durable records and processing executors

**Edit:** store processing/projection/publication adapters, app processing service,
server composition, archive fixtures and APIs.

1. Add durable record reading/writing and reindex. Preserve existing evidence and
   unknown envelopes; provide conversion only for actual affected data. Persist
   Job input/output references and reuse job claims and fenced publication.
   Mixed historical schema support is not a separate acceptance requirement.
2. Implement Import-scoped and Representation-scoped package readers, snapshot
   retention and immutable observations. Bind all opened files to a frozen inventory.
3. Add executor dispatch through registered operation ownership. Persist frozen
   output IDs before writing bytes. Move adapters behind injected ports; no Store
   calls from HTTP or network clients from modules.
4. Carry provisional observations into committed archive history during Import
   publication. Recovery must finish these records as well as Capture/Evidence.
   Import and Representation are distinct fingerprint scopes: explicitly rebind
   published observations to the committed Representation, verify the ordered file
   identities/lengths/digests, and retain the provisional fingerprint and mapping
   as provenance. Test archive-only rebuild without private Import rows. Never
   copy an Import fingerprint and label it current Representation evidence.
5. Add current/historical evidence projection and authorized summary DTOs. Legacy
   provenance limitations must remain visible rather than promoted to current proof.

**Contract integration:** validate frozen inputs and inventories at service entry,
validate typed results/metadata/recipes/snapshot metadata before publication, and
hash streamed snapshot bytes against their recorded length/digest. Acquisition
jobs use `FrozenJobInput::Acquisition`; large catalogs use streaming ports, while
buffered responses are limited to 8 MiB. Observation queries are bounded pages.
Use the canonical OperationId/CapabilityId identity rather than parallel strings.

**Tests first:** current-format rebuild equivalence and any supplied conversion; raw response survives cache deletion;
restart before/after every new commit point; cancellation requested then restart;
post-commit Item edit; revoked grant before versus after commit; retries create one
result; a changed input invalidates currency without deleting historical success.
**Review:** transaction ownership, source binding, secret filtering and third-module
extensibility. Domain work starts only after this checkpoint passes.

### D04.1 — Qualified reader-based game primitives

**Status (2026-09-13): reviewed and corrected locally; proceed to D04.2–D04.3.**

**Read:** Retro Nintendo NES/SNES/N64, NEC PCE, core analyzer contracts and DAT
normalization; Junk Libs core/disc/DAT. Retro core currently contains path-bearing
options, so do not make it a dependency of `collector-modules`.

1. Extract the required reader-only normalization/structural algorithms into narrow
   Junk Libs format crates or a feature-gated `junk-libs-game-formats` crate. Keep
   platform naming/catalog policy in the game module and filesystem adapters in
   infrastructure. Migrate affected Retro consumers in a separate change.
2. Initial enabled matrix: raw exact-byte hashing; trainer-free iNES/NES2 payload
   hashing with independently validated header/length rules; N64 z64/v64/n64
   normalization; SNES and PCE header handling for explicitly qualified layouts;
   ordered BIN/CUE raw-2352 track hashes. Add CHD raw-CD only after the existing
   version-pinned generated chdman oracle passes with the optional feature enabled.
3. Every other inspected operation receives a named unsupported/unqualified row,
   not a catch-all “game support” badge. Trainer-bearing NES, ambiguous copier
   headers, unsupported CHD unit sizes and incomplete discs must not get a complete
   catalog verification result. Preserve them and report the limitation.
4. Add normalized hash domain/version to results. A normalization never rewrites
   a master. Header-only structural information is separate from a catalog match.

**Tests first:** independently expected normalized hashes; chunk-boundary/endian
cases; trainer and malformed/truncated lengths; altered second disc track; stored
versus synthetic pregap. Cross-check old and new consumers plus independent vectors,
not only old/new equality. Run affected Junk Libs/Retro gates and record limitations.
**Stop:** no wholesale Retro orchestration dependency, no copied parallel algorithm
in Collector, no qualification inferred from filename or one selected track.

D04.1 review handoff: preserve strict BINARY CUE source and index/span checks,
N64 length bounds, and ordered non-overlapping source-byte ranges. Invoke the
external CHD oracle explicitly with `CHDMAN=/path/to/chdman ... -- --ignored`;
ordinary all-feature tests report it ignored. Its required version is 0.285, and
an explicit missing/wrong-version run must fail. The `chd` feature gates the shared
hash entry point; `junk-libs-disc` still brings its existing decoder dependency
transitively, so this is not a decoder-free build claim.

CUE `source_byte_offset` is a byte coordinate in an uncompressed source file. Do
not reuse that field for CHD padded frames or compressed file offsets; a future
Collector CHD adapter needs an explicit source/coordinate mapping. Do not label
raw bytes, header structure, or a normalized hash as catalog verification. Real
dumps, target platforms and immutable dependency pins remain external gates.

### D04.2 — DAT snapshots and matching

**Status (2026-09-13): reviewed and corrected locally with D04.3.**

**Edit:** provider catalog adapter, game module matching, app/admin snapshot commands.

1. Load administrator-selected local DAT bytes or an explicitly configured source
   through the provider adapter. Use `junk-libs-dat`; store exact source bytes,
   digest, declared system/catalog/version and retrieval metadata as a snapshot.
   Parsing cannot overwrite the active snapshot until the replacement is complete.
2. Build a rebuildable index; catalog matching freezes a SnapshotId. Use platform,
   normalized byte domain, expected lengths and all declared ordered tracks/files.
   If the catalog lacks enough checksums/domain information, return inconclusive.
3. Distinguish zero, one and multiple complete matches. Partial/track-one matches
   contribute identity candidates only. CRC alone must not be described as strong
   cryptographic identity; retain which algorithm actually matched.
4. Expose explicit snapshot import/activation/status; source paths remain admin
   configuration only. Missing DAT/tool leaves preservation and structural analysis usable.

**Tests:** equivalent XML/text fixtures; duplicate candidates; wrong platform;
missing/reordered/modified tracks; old snapshot after refresh; malformed/oversized
DAT; parser cancellation and source outage. Snapshot bytes survive backup/reindex.

### D04.3 — Game workflow and root acceptance

**Status (2026-09-13): reviewed and corrected locally; D05.1 is the next bounded implementation checkpoint.**

Wire game structural and DAT operations into Import analysis and committed Capture
jobs, then web curation. Show operation availability, snapshot/version, exact versus
partial match and historical evidence. A tentative title is useful but never a
verified badge. Populate every row of the operation matrix with an actual test or
an explicit unavailable/external gate. Re-run listener/role tests and browser
artwork-plus-text/search tests. Publish one synthetic game package end-to-end,
verify it, change the active snapshot, and demonstrate preserved old evidence.
D04 closes locally only when every enabled operation has its independent vector.

### D05.1 — Exact CD reading and bounded AccurateRip input

**Read:** Junk Libs disc/disc-id/accuraterip/audio; Phono offset, dBAR and ARver
fixtures; existing opened-reader CUE/redumper adapters. **Edit shared algorithms
separately** from Collector consumers.

**Reviewed immediate contract (2026-09-14, implementation checkpoint):**

- `junk-libs-disc` owns the exact seekable signed-16-bit, stereo, 44,100-Hz PCM
  reader and strict CUE/redumper layout mechanisms; `junk-libs-accuraterip` owns
  bounded checksum/offset arithmetic over that reader. Collector maps an already
  authorized frozen package inventory into those mechanisms and maps their output
  into the music module's typed result. It does not expose paths or own a second
  parser/algorithm.
- D05.1 creates no durable record, private state, provider snapshot, route, or
  registered runtime capability. A later existing processing job may publish the
  validated result at the established observation commit point; cancellation is
  cooperative through the existing host checkpoint and cannot yield a success.
- Authorization, role/exposure filtering, retry identity, source currency and
  historical reads remain unchanged in the application processing service. The
  reader consumes only the exact frozen package scope; malformed, truncated,
  synthetic-gap, mixed-mode and missing-sidecar inputs fail distinctly before a
  whole-disc audio result can be claimed.
- The complete local example is an authorized multi-file BIN/CUE package producing
  typed layout and bounded checksum inputs from independent fixed vectors. The
  denied/interrupted examples are wrong-scope package access and cancellation
  during a long generated reader; neither produces a publishable success. D05.2
  remains the owner of dBAR acquisition and network/result policy.

1. Build a typed disc layout from declared BIN/CUE and redumper sidecars through
   package readers. Validate INDEX 00/01, track order, file spans, signed lead-in
   coordinates, stored versus synthetic gaps and data/audio classification.
2. Preserve exact signed 16-bit stereo 44,100-Hz PCM for CD verification and
   derivatives. Existing f32 playback chunks are not the preservation interface.
   Add a narrow integer PCM reader contract; playback can convert downstream.
3. Existing `DiscTrackSamples` uses whole-track vectors and prefix arrays. Add a
   bounded reader-based checksum/offset path in Junk Libs, with the vector API kept
   as a compatibility/reference adapter. Use seekable readers and bounded boundary
   windows; do not allocate all tracks plus per-sample prefix arrays in Collector.
   Set a 64-MiB working-buffer budget per verification job, excluding the bounded
   provider body. Verify the bound with a generated long-disc reader.
4. Use the existing offset convention and default search interval [-2939, 2939]
   stereo sample frames. Record the exact interval, selected offset and unavailable
   boundary samples. Never pad missing physical samples and then assert exactness.
5. Multi-disc editions produce one disc result per Capture/TOC. Initial mixed-mode
   behavior must explicitly identify data tracks and decline whole-disc audio
   success for unsupported layouts. No reconstruction success is implemented here.

**Tests first:** published disc IDs; independent ARver/CUETools-derived checksum
and offset vectors; both signs and cross-track boundaries; extreme i16 values;
INDEX/gap variants; short/truncated/missing sidecars; bounded memory and cancellation.
Vector-versus-streaming equivalence is additional evidence, not the sole oracle.
**Review:** exact coordinates and bounded arithmetic before integrating the network.

**Local implementation evidence (2026-09-14; reviewed and corrected):** Collector now
maps authorized frozen BIN/CUE and current raw-redumper packages to typed signed
sector/index/gap/source-span layouts, published disc-ID primitives and exact seekable
little-endian `i16` stereo PCM. Structural CUE directives are strict; malformed,
truncated, missing, synthetic-gap and mixed-mode inputs fail before whole-disc audio
success. Negative redumper lead-in coordinates remain signed. No operation, route,
provider, network acquisition or durable publication was added.

`junk-libs-disc` owns the exact frame reader and signed redumper coordinates.
`junk-libs-accuraterip` now searches the exact physical interval through a bounded
reader, rolling ARv1/frame-450 windows and evaluating ARv2 only on the evidence
frontier. Its maximum read request is 16,384 frames and its stated per-job working
budget is 64 MiB. The old vector entry point delegates to this implementation and a
test-only prior arithmetic reference remains available for regression work. Literal
independent positive/negative CUETools/ARver-equation vectors cover cross-track
borrowing and both CRC versions; generated long-disc cancellation returns an error.

The real Phono verification caller was migrated from whole-disc vectors to the
bounded reader. Trash Amp received only the required checked signed-redumper-to-TOC
conversion. Junk Libs, Phono and Trash Amp format/test/clippy gates pass. Retro's
format and full workspace tests pass; its all-target strict clippy gate fails on
existing Rust 1.95 lint debt unrelated to this change, so no Retro files were
changed. The external owned redumper dump and live/network ARver gates were not run
and remain unqualified. The subsequent D05.2 checkpoint is recorded below.

### D05.2 — AccurateRip provider and result interpretation

**D05.1 review constraints (2026-09-14):** The shared verifier's inclusive interval
is the **v1** search range. Its `v2_evaluated_sample_shifts` explicitly lists the
candidate offsets evaluated for v2. A v2-only catalog without a v1/frame-450 anchor
can return `IncompleteSearch`; this is inconclusive, never a mismatch or corrupt
audio. Do not claim exhaustive v2 search from interval endpoints. Either retain
this limitation or add independently tested bounded complete-search machinery.
The shared `Verified` status means at least one full-track match: whole-disc success
requires every expected track, usable database identity, and a justified offset.
Propagate per-version search coverage into Collector's typed result. Preserve the
short-disc frame-450 bound and cancellation checks. Package-source PCM decoding now
lives in `junk-libs-disc::pcm::SourcePcmReader`; do not recreate it in a module.
CUE duplicate source identities, out-of-span indexes and four-channel audio fail
closed. Synthetic gaps remain structural observations with no exact PCM/disc-ID
claim. See [review notes](wave-4-review.md#d051-review--2026-09-14).

1. Adapt Phono's URL/response primitives into the provider crate; do not import
   its network/persistence orchestration into the music module. Store the exact
   bounded dBAR response and a snapshot identity. Cached/recorded responses are
   first-class offline inputs.
2. No database entry => inconclusive/no data. Malformed response or network outage
   => unavailable, not corrupt audio. Present database mismatch only when a usable
   record exists for the exact supported layout and the declared offset search is
   complete. Ambiguous offsets remain inconclusive. Frame-450 evidence alone cannot pass.
3. Store per-track checksum version/confidence, evaluated offsets, edge limitations,
   implementation version and input digests. Do not use MusicBrainz results to
   change the audio-verification outcome.
4. Freeze responses for retries; a provider refresh creates a distinct observation.
   Bound timeout/body/retry behavior using the common provider rules below.

**Tests:** known match, no data, mismatch, tied offsets, malformed/truncated response,
timeout, provider outage with cache, cancellation and replay with a later provider
response. Use local HTTP fixtures; live checks are opt-in qualification only.

**Local implementation evidence (2026-09-14; reviewed and corrected):**
`collector-providers` now owns the pinned-origin dBAR URL and blocking HTTP adapter.
It uses a 15-second maximum timeout, an 8-MiB cap enforced while reading, at most
three transient attempts, a shared one-request-per-second limiter, cancellable
Retry-After delays, no automatic redirects, and manual same-origin redirect checks.
The only injectable HTTP origin accepts an uncredentialed loopback literal for
local fixtures. Exact 200 and 404 bodies become digest-bound observations; 404 is
typed no-data, while malformed, misdirected, oversized and unavailable responses
cannot become audio-mismatch evidence. Archived dBAR replay re-reads the complete
body and checks its recorded length and SHA-256, including trailing-byte rejection.

The music module now interprets the reviewed shared verifier without widening its
claim. Each track records computed ARv1/ARv2 checksums, their computation offset, confidence, the inclusive
v1 interval, explicit v2 candidate offsets, unavailable physical edges and input
digests. Whole-disc `Verified` requires every requested/database track at one
unambiguous selected offset. A partial match cannot verify the disc. Candidate-only v2 search, unavailable
edge coverage, tied offsets and frame-450-only evidence remain inconclusive; a
database mismatch requires complete v1 and v2 coverage over the declared physical
interval. Local HTTP and policy fixtures cover the listed states and refresh/replay.
No music capability, server composition, job workflow, route, UI, observation
publication or D05.3 fixture workflow was added. Live AccurateRip and owned-dump
qualification were not run, and no sibling repository was changed.

### D05.3 — CD workflow and root acceptance

Wire structural and AccurateRip operations into the module registry, jobs and web.
Show separate integrity, structure and audio-catalog evidence, including per-track
status and offset units. An unavailable provider must not stop an unidentified
CD from being archived or turned into a derivative. Run a BIN/CUE and a synthetic
redumper package through staging, analysis, publication, verification and reindex.
Add an opt-in real-dump oracle runner with documented inputs and reference outputs.
If no owned dump is available, record the real-layout gate as externally unqualified
and do not advertise that layout as independently validated.

**Integration constraints from D05.2 review:**

- Adapt the bounded `ProviderPort` response into the existing streaming acquisition
  executor; do not introduce a second registry, scheduler or publication path.
  Compose one shared AccurateRip provider instance so its request limiter covers
  all jobs, including redirects and retries.
- Derive the query from the frozen supported layout and the response digest from
  the verified archived snapshot. Wrap results in the existing source/snapshot-bound
  processing observation. Retain each track's `computed_offset` even when no
  disc-wide offset is selected. Never fabricate no-data for network failures.
- Exercise the actual shared verifier through provider replay, application jobs
  and archive publication, rather than constructing all verification summaries
  by hand. Preserve candidate-only v2 coverage as inconclusive and require all
  expected tracks for whole-disc success.
- Make coordinated contract changes where needed, without checkpoint migrations
  or placeholder successful capabilities. Stop for D05 root review before D06.1.

**Local implementation evidence (2026-09-14; awaiting root review):** The Music
module now owns registered Import analysis, CD structure and AccurateRip operations.
Application policy derives a dBAR query only from a current frozen supported-layout
observation and stores its Representation fingerprint in the acquisition request.
The existing acquisition executor adapts the bounded small-response provider into
the normal streamed snapshot publisher. Server composition shares one provider
instance between acquisition and parsing, including its redirect/retry limiter.
Before module interpretation, the parser consumes all archived bytes and verifies
recorded length and SHA-256; the module cross-checks the query against freshly
derived layout IDs and records the checked snapshot digest plus all frozen input
digests in its result.

The archive integration test stages, analyzes, reviews and publishes synthetic
BIN/CUE and signed-coordinate redumper packages. BIN/CUE then produces structure
and whole-disc shared-verifier output; the test also covers provider outage and
same-Job retry, queued cancellation, same-length archived-byte corruption, restored
retry, checked no-data, distinct refresh snapshots, historical observations and
archive-only reindex. Redumper structure publishes while its deliberately unsupported
disc-ID coordinates yield an explicit unavailable verification observation without
removing the Capture. The React workflow exposes package/layout selection, immutable
snapshot acquisition, bounded offset choice, durable progress, separate results,
per-track computation/coverage and limitations beside the existing artwork tools.
The browser smoke fixture exercises the actual structure/acquisition/verification
controls with a local dBAR service and passes in an isolated headless Chrome profile,
including immediate post-publication Item refresh and retained artwork.

`scripts/run_d05_real_dump_oracle.py` and the private-root manifest contract run
the production package readers and shared bounded verifier only when
`JUNK_COLLECTOR_W4_CD_DUMP_ROOT` is supplied. No owned dump or completed external
reference manifest is available here, so live provider, owned redumper, real
mixed-mode and mastering/offset qualification remain external. No D07 FLAC work
was added. Stop for D05 root review; after acceptance, D07 is the recommended next
root under the music-first proposal, which has not silently replaced the serial ledger.

### D06.1 — Durable metadata authority

**Status (2026-09-15): reviewed and corrected locally; D06.2 is reviewed and corrected
locally.**

1. Add typed common display fields (title and subtitle), owner-bound typed
   artist/platform overrides, language, artwork choice and explicit authority commands. Preserve
   existing string-key overrides/unknown values as legacy envelopes rather than
   dropping or silently coercing them. Existing manually entered inventory titles
   become manual authority; refresh must not replace them merely because no new
   explicit override row existed previously.
2. Persist all candidate alternatives and raw observations in portable history.
   Stable candidate identity is provider plus provider record identity/version,
   not array position. Removing a candidate from a new response does not delete
   a manually selected historical candidate.
3. Resolution order: field override, explicit selected candidate, then automatic
   ranking. Automatic rank is complete identity evidence, identifier/layout strength,
   English-preferred language, provider priority, stable provider ID tie-break.
   Language cannot outrank stronger identity. Keep original-language strings.
4. Expose revisioned authority update/clear commands and a materialized display
   projection. Clearing an override is explicit. Refresh appends observations,
   then computes display; it never writes into the manual authority layer.

**Tests first:** old manual title survives migration/refresh; explicit selection
survives disappearance/provider outage; Japanese strong match beats English name
match; ambiguous matches yield a marked tentative display; stale authority update
conflicts; unknown override fields and raw lexical JSON survive archive-only export.

**Local implementation evidence (2026-09-15; awaiting review):** Core now owns
bounded typed candidate observations, stable provider-record/version identity,
language-bearing title/subtitle values and explicit materialized resolution state.
The application resolver ranks complete identity before identifier/layout strength,
then English preference, provider priority and a stable identity tie-break; equal
policy ranks remain tentative. Typed field overrides, explicit selection, artwork
choice and owner-validated domain overrides use focused revisioned commands.
Existing inventory title/subtitle values seed manual authority on first use, while
legacy string-key fields remain preserved envelopes.

Each refresh writes its lossless observation and recomputed display in one existing
schema-4 MetadataAuthority revision without modifying manual authority. Receipt
claims bind the submitted observation/change before reading the mutable authority
revision, and schema-4 publication prepares command recovery against the actual
record commit. Catalog schema 6 materializes stable candidates; archive-only reindex
scans every authority revision to rebuild their history and projects the newest
authority/display. Tests cover evidence-before-language ranking, deterministic
reordering, ambiguity, manual precedence, selection across outage, stale conflict,
same-key replay, revoked replay authorization, raw lexical/unknown preservation,
reindex and restart. No live provider, HTTP route or web UI was added; D06.2 and
D06.3 remain unimplemented.

**Review corrections (2026-09-15):** Candidate versions and observation identities
are immutable, including during reindex. Contradictory refreshes fail before archive
publication. Inventory baselines preserve later manual edits and provide writer-lock
CAS; effective application reads recompute display, and reindex validates frozen
display inputs. Automatic ambiguity survives unrelated manual choices. Refresh
artwork must belong to an active asset of the item, and deleted items reject writes.
Ambiguous legacy record IDs do not arbitrarily select a provider. No routes or UI
were added. See the dated review/validation records for qualification.

### D06.2 — Providers and artwork

**Status (2026-09-16): reviewed and corrected locally; D06.3 is reviewed and corrected locally.**

Initial providers are MusicBrainz release/disc identity, Cover Art Archive, and
DAT-backed game catalog titles/identifiers from D04. Do not add ScreenScraper,
PriceCharting or another credential-requiring game service by guesswork. The
provider port permits later adapters without changing metadata authority.

1. Query MusicBrainz by known disc/record identifiers, then bounded explicit user
   search where needed. Do not send ownership, loans, acquisition data, paths or
   private notes. Identify the client with a configured meaningful User-Agent.
2. Use one shared MusicBrainz limiter per server, at most one request/second;
   cache successful identity responses for 24 hours and negative lookups for one
   hour. Explicit refresh still respects the limiter. Respect Retry-After; permit
   at most three transient attempts with cancellable delays. No busy retry loops.
3. Default provider request timeout 15 seconds, JSON/dBAR body cap 8 MiB, redirects
   at most three only to allowed HTTPS provider/CDN origins. DAT administration
   uses a separate streaming parser cap of 512 MiB; reject larger input clearly.
   CAA image bytes use the existing asset upload/decode limits, not the JSON cap.
4. Pin production provider origin rules. Test-only local origins must be injected
   through test composition, never an undocumented SSRF-capable production bypass.
   Reject private/link-local targets and revalidate redirected destinations before
   fetching provider content. Provider credentials never follow redirects.
5. Fetch artwork into the existing Asset pipeline; retain originals and bounded
   thumbnails. Automatic artwork selection cannot replace an explicit user choice.
   Preserve attribution/provider IDs and uncertain association separately from missing art.

**Tests:** recorded ambiguous/reordered responses; timeout/429/Retry-After;
redirect/oversize/malformed bodies; private-field request inspection; stale cache;
provider disagreement; failed art download leaves previous art usable.

**Local implementation evidence (2026-09-15; awaiting review):** Module-owned
MusicBrainz and Cover Art Archive query/observation DTOs feed adapters registered
through the existing acquisition and catalog-parser registries. Production origins
are fixed; loopback injection is test-only. MusicBrainz uses one server-composed
limiter, a meaningful configured User-Agent, 15-second timeout, 8-MiB JSON bound,
three-attempt cancellable retry/Retry-After policy, three-hop revalidated redirects,
and 24-hour success/one-hour negative caches. CAA listing JSON uses the same JSON
bound, while PNG/JPEG/WebP image responses use the existing 20-MiB Asset ingress
bound and pinned CAA/archive origins. Provider requests contain only bounded
release/disc/search or release/image identifiers.

Exact acquired bytes remain immutable snapshots. MusicBrainz candidates use fixed
provider priority and versions derived from the exact response digest plus the
normalized record, so changed raw or normalized content cannot reuse an identity.
DAT-backed game metadata consumes the exact D04 snapshot and its item-associated
catalog result through module-owned DTOs rather than a second provider pipeline.
Fresh item/provider-bound metadata observations are distinct from Job, snapshot,
receipt and publication transaction identities. Refresh delegates to the existing
revisioned metadata effect; it does not edit inventory or catalog rows.

CAA images enter the existing authorized Asset creation effect, which retains the
original and bounded thumbnail. A later refresh may reference only an active Asset
of that Item and only for an observed provider record. Explicit artwork choices and
manual/inventory text authority remain dominant. Tests use deterministic loopback
fixtures for ambiguity/reordering, cache expiry and refresh, timeout, 429 and
cancellable Retry-After, malformed/oversized bodies, redirect/credential/private
target rejection, private-field request inspection, provider disagreement, exact
raw lexical preservation, failed image decoding, restart and reindex. Live-provider
and deployment checks were not run; MusicBrainz identity is never represented as
audio verification. No route, metadata review UI, native client, offline cache,
additional provider, backend migration or separate publication pipeline was added.

MusicBrainz's request-rate/User-Agent policy is documented in the
[official API guide](https://musicbrainz.org/doc/MusicBrainz_API). Recheck provider
contracts at implementation time and record drift instead of inventing endpoints.

**Review corrections (2026-09-15):** Acquisition receipts now bind submitted
item/query inputs before freezing timestamps. Candidate versions include ranking
and artwork inputs. Authority records retain current candidate identities separately
from immutable selectable history, so refreshed versions win automatic resolution.
DNS checks run in the actual HTTP resolver under timeout, with proxies disabled;
CAA permits narrowly pinned Archive storage hosts. MusicBrainz admission spans
request dispatch and shared Retry-After cooldowns; cache entries are bounded to
128 keys/32 MiB and cached reads honor caller limits. HTTP status is retained;
malformed successful responses fail, while non-JSON 404 bodies remain exact
snapshots referenced by metadata. English language codes normalize for display.
Regression coverage includes repeat command replay, artwork/version changes,
current versus historical reindex, cached limits, private/mapped addresses,
malformed success, and non-JSON no-match preservation. No metadata HTTP/UI added.

### D06.3 — Metadata review UI

**Status (2026-09-16): reviewed and corrected locally; D06 root review complete.
P05.1 is the next serial child.**

Add a curation panel for alternatives, provenance, explicit selection, per-field
overrides and clear-override actions. Preserve drafts through job polling/conflicts;
allow reopening a saved decision after refresh. Display effective text/artwork in
browse/search/detail rows with tentative versus definitive identity distinct from
artwork absence. Test actual provider-job results, outage/refresh without lost edits,
foreign-language labels, viewer redaction, keyboard navigation and 390px layout.
Do not render raw provider HTML or include unreviewed URLs as executable content.

**Local implementation evidence (2026-09-16; awaiting review):** Application-owned
curation routes expose safe review, acquisition, refresh and artwork commands plus
typed provider availability, lookup choices, permissions and catalog-snapshot Job
results. MusicBrainz/CAA sequencing reuses acquisition Jobs, receipts, Asset
publication and the metadata refresh effect; absent providers return typed guidance
without blocking inventory or existing art. Effective metadata is projected into
browse/search/detail and FTS, with curator-only inventory baselines and role-filtered
artwork. The React panel retains drafts across polling and revision conflicts,
persists active Job identity for reopen, cancels obsolete loads, distinguishes
current candidates from selectable history, and provides explicit field, candidate
and artwork set/clear actions.

The deterministic browser workflow used loopback MusicBrainz/CAA fixtures through
the production provider adapters, reopened an in-flight acquisition, selected
foreign-language history, exercised an authority conflict, acquired and published
real PNG bytes, searched effective text, and checked viewer artwork filtering,
keyboard order and 390px overflow. Chrome 153.0.8010.37 passed. Live MusicBrainz/
CAA, other browsers and deployment remain unrun.

**Root review corrections (2026-09-16):** Acquired-result completion now belongs to
application services, authorizes the caller and Item, and dispatches one existing
receipt/effect. Image publication no longer depends on a second MusicBrainz request
or silently changes authority; users explicitly choose the preserved cover. The
panel aborts detached work, persists completion keys/revisions, retains failed
completion for explicit retry, and protects unsaved drafts on dismissal. Shared
application resolution keeps search consistent through Item changes and reindex.
Lookup defaults distinguish recorded MusicBrainz disc IDs from unrelated IDs;
unsupported Item schemas remain read-only. No live-provider qualification is claimed.

### D07.1 — Recipes and exact conversion

**Status (2026-09-15): reviewed and corrected locally; D07.2 is reviewed and corrected locally.** The
settled contract and implementation below add no publication, route, registered
conversion Job or playback capability at the D07.1 checkpoint. D07.2 status is below.

**User decision (2026-09-14):** Maintaining a parallel server-side FLAC library
is optional. Initially prepare derivatives only on explicit request; importing,
tracking and verifying masters must work without generating FLAC. FLAC-first means
the default conversion format, not automatic conversion of every import. A future
opt-in maintenance policy must use the same jobs and representations. Disabling
automatic preparation must not implicitly delete existing derivatives or masters.
When no playable representation exists, show preparation as an optional action;
do not promise direct raw-dump playback or silently transcode on a media GET.

**Settled producer/consumer contract (2026-09-15, before implementation):** The
music module is the sole owner of the ordered CD-to-track mapping. For supported
CUE packages, the exact PCM timeline starts at the first stored audio byte: stored
HTOA before track 1 is prepended to track 1, and a later stored `INDEX 00` remains
owned by the preceding track through the existing D05 `INDEX 01`-to-next-`INDEX
01` span. Synthetic `PREGAP`/`POSTGAP`, mixed-mode/four-channel layouts, missing
physical ranges and ambiguous package members are not convertible. Supported
redumper audio uses its validated signed first-track-to-lead-out timeline and the
same next-`INDEX 01` ownership. The module returns a bounded seekable integer
`i16` stereo 44,100-Hz reader plus ordered, nonempty track ranges; it does not
spawn tools, authorize users, schedule jobs or publish representations.

Application code owns future explicit-request authorization, frozen Job identity,
source-currency checks and cancellation and calls a mechanism-only conversion
port. The provider adapter receives only the sealed recipe, the already-authorized
PCM reader and an already-opened scratch handle. It directly spawns the configured
binary, streams bounded chunks through pipes, caps diagnostics/output, kills and
reaps on cancellation, and returns typed validation—not a published media identity.
The immutable recipe digest binds the source fingerprint, exact ordered track
ranges, fixed codec settings, encoder name/version and input/output PCM formats.
FLAC means level 5 and unchanged signed little-endian 16-bit stereo 44,100-Hz PCM.
Opus means 96/128/192 kbps, 20-ms frames, stereo music mode, input declared as
44,100-Hz integer PCM and an explicit 48-kHz decoded timeline; pre-skip and final
padding are measured from the Ogg Opus result. A missing or version-mismatched tool
is unavailable, never a successful no-op.

D07.2 persists one recipe/validation result and immutable representation per
source fingerprint plus recipe and track selection, using stable retry identities;
D07.3 will expose authorized ordered descriptors containing track position,
track-local frame count, immutable representation/file identity, media type,
content length/digest, availability and safe display fallback. Those consumers
must use these music-owned ranges and must not reinterpret pregaps or expose archive
paths. These are planned consumers only: D07.1 adds no route, publication, media
descriptor or placeholder endpoint. A supported layout remains convertible when
AccurateRip reports no data, mismatch or outage because catalog verification is
neither an input to this contract nor a conversion authorization gate.

1. Keep existing `ConversionProfile::Flac` / `Opus { bitrate_kbps }` API readable.
   Introduce immutable `RecipeId` behind those user presets; a preset is not enough
   to identify cached output when encoder versions/settings change.
2. Initial profiles: FLAC level 5, source 16-bit stereo 44,100-Hz PCM unchanged;
   Opus 96/128/192 kbps, 20-ms frames, stereo music mode, explicit 48-kHz conversion
   recipe and encoder delay/padding. Arbitrary unsupported values fail validation.
   No gain normalization, lossy intermediate or resampling in the FLAC path.
3. Use optional configured `flac` and `opusenc` tools via a bounded process adapter
   first. Pass arguments directly, never a shell command. Feed exact integer PCM
   or a generated bounded-header WAV through handles/pipes; no external process
   receives a user-selected archive root. Tool paths are admin configuration.
4. Drain bounded diagnostics concurrently; cancellation terminates/reaps the child
   and removes only its known partial scratch. Record tool version/settings.
   Decode FLAC independently and compare every PCM frame plus length/format;
   include nonzero extremes, impulses and cross-track joins.
5. Define track ownership once: retain stored pregap with the track/file layout
   specified by D05; do not silently discard HTOA. Unsupported/ambiguous track
   boundaries cannot produce an advertised exact track derivative. A disc-wide
   exact derivative may be offered with its full layout if supported.

**Tests:** independent integer PCM equality, EOF/seek and track boundaries; known
lossy Opus metadata/duration within codec padding rules; absent tools; process
failure/cancellation and malicious file labels. Golden vectors name tool versions.
Do not invent a full shared encoder framework: mechanism adapters belong outside
core/modules; reusable decode/PCM work belongs in Junk Libs with sibling checks.

### D07.2 — Derivative publication

**Status (2026-09-15): reviewed and corrected locally; proceed to D07.3.**

**Implemented contract:** `CollectionService::prepare_music_derivative` accepts one
source Representation, music package selection, track position, preset and full
configured encoder version report. The application resolves music-owned ranges,
seals a recipe and freezes it in the existing job store. Requests use command
receipts; separate requests have private JobIds but converge on content-derived
RecipeId/RepresentationId identities. One job produces exactly one track file.
No HTTP endpoint, automatic import hook, media-GET conversion or second scheduler
is added. The host composes an optional `AudioConversionPort`; absent/mismatched
tools return unavailable.

Store owns anonymous encoder scratch and known per-attempt publication spools.
Encoding, shared codec validation and scratch digest/length checking happen outside
the archive writer lock. A checked Capture revision appends the representation;
its journal publishes output, schema-4 recipe and schema-3 validation Evidence
before the Capture revision rename. The final cancellation gate and current
application authorization/source-record check precede that rename. Capture edits
are merged from current storage under the writer lock; Item records are not
rewritten. Pre-commit cancellation/failure removes only journal-owned artifacts;
post-commit recovery projects the original records and bytes, independently of
current tools, grants and module registration, including a lagging failed private
job checkpoint. Startup cleans known disposable
attempt spools after journal recovery. Existing migrated master locators remain
intact; new derivatives use canonical Capture payload paths.

The executable workflow imports a synthetic two-track BIN/CUE with HTOA and a
stored later pregap, explicitly prepares separate tracks, edits/associates an Item,
and reindexes. Tests include concurrent encoders, command replay, changed source
and encoder, invalid validation/scratch, queued/encoded/phase cancellation, denied
authority, failures and actual process kills across all six journal phases. Real
FLAC is an explicit enabled test; Opus structural validation remains distinct from
PCM equality, and real opusenc/owned-disc qualification is unrun.

Create one immutable derivative representation per source fingerprint/recipe/track
selection. A retry has the same IDs; a different source or recipe has new IDs.
Stage and validate output, then attach it through the existing Capture revision
publisher. Never replace an existing master or repurpose a published representation.
Persist the recipe and validation evidence before advertising availability; recovery
finishes a committed derivative without reconverting under a newer encoder.

**Tests first:** two concurrent requests for the same recipe converge safely;
request replay; cancellation/restart at every phase; failed independent decode;
source change; Item edit after commit; missing encoder; corrupted output; reindex.
No client response lists partial output. Do not take the writer lock while encoding.

### D07.3 — Availability and media

**Status (2026-09-15): reviewed and corrected locally.**

Expose recipe/output availability and enqueue commands through services; add web
buttons for FLAC default or an explicit smaller Opus copy. Return the immutable
representation/file identity, content length, media type and digest, with a strong
ETag derived from actual immutable content. Implement Range/If-Range correctly
for originals and derivatives before C02 relies on them. Verify viewer download
permissions, current grant revocation, disabled curation APIs, byte ranges and
FLAC source equality through HTTP. Native offline management remains C05.

**Implemented contract:** the application returns ordered per-track/per-profile
rows containing source Representation, exact integer start/count coordinates,
safe stored-title fallback, profile and only preparing/unavailable/available
state. Available rows add immutable Representation/package-file identity, length,
media type and digest. Archive locators, raw Evidence and Job identity/diagnostics
do not cross the viewer DTO. Published recipes reconstruct their rows even if the
music module or current encoder is missing. The server probes optional configured
`flac`/`opusenc` executables and supplies the full report to the existing frozen
preparation command; HTTP inputs cannot select a tool or claim its version.

The client router exposes authorized availability and the existing opaque media
reader. The curation router alone exposes explicit one-track preparation using
normal receipts and Jobs. Public curation configuration determines whether that
command exists; forwarded headers do not. The curation UI defaults visually to
FLAC, offers explicit Opus 96/128/192 choices, explains missing tools, follows Job
progress and refreshes after completion. It adds no automatic import/GET work,
disc concatenation, streaming/transcoding API or browser playback claim.

Actual HTTP coverage includes viewer reads, curator-only preparation,
cross-collection denial, hidden public curation, missing tools, completion/reload,
restart reads without module/tool, real FLAC bytes and source-PCM equality. Strong
digest ETags and exact full, partial, suffix, open-ended, unsatisfiable, stale and
weak If-Range responses are checked for originals and derivatives, including
actual lengths and current grant revocation. The generated OpenAPI/TypeScript
contracts and embedded web assets are rebuilt. Real `opusenc`, owned-disc and
deployment qualification remain unrun. The following bounded browser-player
child now implements real play/pause/seek/track-transition acceptance locally and
is reviewed and corrected locally. It consumes this DTO and media route without changing the
D07.3 service, receipt, encoder or publication contracts.

### P05.1 — Portable export and changes

1. Export to a newly created directory bundle (readable TOML manifest plus files),
   not an arbitrary archive-extraction format in the first implementation. Include
   committed revisions, tombstones, referenced masters/assets/derivatives, recipes,
   snapshots, raw observations and unknown module records. Omit credentials, route
   trust, jobs and unrelated private scratch.
2. Freeze a committed revision inventory under the writer lock; release it while
   copying immutable files, then verify digests and write a completion marker last.
   Retain source data. Reject symlinks/special files/traversal and overlapping
   source/destination roots. A failed export is never reported complete.
3. Exportable changes use a versioned manifest inventory with stable entity IDs,
   revisions and digests. A delta names the complete base-inventory digest and
   includes required dependencies. This is offline export, not live replication,
   conflict merging, or concurrent archive writers.

**Tests:** export during later writes has a coherent frozen cut; partial copy;
unknown module bytes; missing dependency; changed base digest; tombstones/loans/
overrides/evidence survive fresh import and reindex; no private credentials present.

### P05.2 — Full backup and restore

Initial full backup/restore requires a **stopped server**. This is a deliberate
consistency boundary, not a claim of online full backup. Acquire both process locks;
fail with instructions if an active server owns them. Reconcile interrupted archive
transactions before selecting the backup cut. Include the archive plus private DB,
server identity/TLS material, relevant configuration and retained Import/upload
scratch needed by pending jobs. Exclude rebuildable catalog and transient sockets.
Use SQLite's backup API even while stopped; never copy a live WAL database file
alone. Write a manifest with file digests/schema versions and completion marker last.

Restore into new empty destinations, validate every file/schema/reference first,
and leave the original installation untouched. Distinguish:

- Full restoration of the same stopped installation: retain ServerId, credentials
  and trust; never run the old and restored instance simultaneously.
- Independent clone/archive-only restore: create new private state/ServerId/TLS;
  import no sessions/grants. Require explicit local admin bootstrap and collection
  access reassignment. IDs/history in the archive stay stable.

Do not “restore” by opening an arbitrary backup directory with automatic migrations
before validation. Bad manifests, path escapes, missing scratch and unsupported
versions fail without publishing a successful installation. Relocated inbox/tool
configuration requires explicit mapping; never blindly use old host paths.

**Tests:** restore current schema and any explicitly supported conversion fixtures; stop during export/restore;
full recovery of pending Jobs/Imports; archive-only no grants; identity separation;
corrupt/partial backup; live-lock denial; private file permissions; exact unknown
raw data. SQLite's [backup API](https://www.sqlite.org/backup.html) supplies a DB
snapshot, not the cross-file archive consistency boundary; the stopped-server locks do that.

### P05.3 — Operator surfaces and drills

CLI commands: `export`, `backup`, `restore-validate`, `restore`, and explicit
archive-only access reassignment. Resolve exact flags against the current CLI in
this child; require explicit mode/destination, no destructive defaults. App services
own preflight/validation; CLI is an adapter. Admin UI/API offers portable export
Jobs, status and validation reports. For full backup/restore it provides exact
stop-and-run instructions; it must not expose a pretend live-backup button.

Document backup contents, credential sensitivity, stopped-server requirement,
NAS payload/local SQLite layout, scratch inclusion, restoration versus cloning,
and how to verify the result before starting it. Run both full and archive-only
restores in temporary roots with the combined D04–D07 fixture collection. No real
user archive/configuration may be migrated for a test. Re-run P05 after S04 adds
rotation state; pending/active/grace generations must back up consistently too.

### C01.1 — Connection persistence and secrets

1. Persist server/route metadata in a client-local versioned store with atomic
   transactions. Model `ConnectionId` separately from ServerId: the same server
   may be enrolled as different users. Merge transport facts, not credentials,
   grants or personal activity. Device credentials belong to a connection/profile.
2. Implement macOS Keychain storage behind `NativeCredentialStorage`; the existing
   trait keyed only by ServerId must become connection-keyed with callers migrated
   together; retain an adapter only if a real remaining consumer needs it. Persist secret references, never bearer credentials
   in the connection DB/logs/Debug output. No plaintext fallback if Keychain fails.
3. Tests use an injected secure-store fake; add a platform-specific opt-in Keychain
   round trip and cleanup. Linux headless tests need no fake production Keychain.
4. Cover crash between metadata/secret writes: pending enrollment record, secret
   insert, metadata completion, orphan-secret cleanup. Revocation/logout deletes
   the intended secret without removing another account's server connection.

**Tests first:** restart two servers/two profiles; identical local IDs across
servers; locked/missing secret store; partial write; expired credentials; redacted
errors; conversion of existing persisted enrollment data where actually present.

### C01.2 — Route trust and typed requests

1. Persist canonical HTTPS origins and explicit trust mode: enrolled local TLS
   material or Web PKI for an authenticated configured public/tailnet HTTPS origin.
   The existing `EnrolledClient` builds only a local-root client; separate transports
   are required. Neither permits redirects carrying credentials.
2. Public origin registration must be authorized through an already trusted route
   or explicit browser-mediated enrollment at that origin. A stranger's valid
   Web PKI certificate plus copied ServerId is not evidence of the same server.
3. For a discovered local candidate, verify the enrolled TLS identity and hostname
   without credentials first; only then check the authenticated server/profile
   identity and register it. Treat authenticated endpoint manifests as suggestions
   until their TLS checks pass. Restrict the current public
   `add_verified_endpoint(server_id, endpoint)` shortcut behind this verified flow.
4. Add typed browse/item/evidence/media-manifest methods. Move the required existing
   read DTOs into the planned `collector-api-types` crate, migrate callers together
   and keep OpenAPI/TypeScript drift checks. Do not copy a second hand-maintained model
   or give client a dependency on collector-app/Store. Register the contract crate
   in the dependency checker. Validate URLs and paths before attaching credentials; no userinfo/fragments,
   ambiguous origin normalization, arbitrary RequestBuilder or caller-chosen host.
5. Native credentials currently expire after 30 days. Expose an explicit
   re-enrollment-required result on expiry/revocation; do not invent refresh tokens.

**Tests:** real local TLS plus separately trusted public TLS test origin; wrong
ServerId, same ID/wrong key, unauthorized origin, hostname mismatch, redirects,
replayed endpoint manifest, grant revocation and no public route/Internet needed.

### C01.3 — Native diagnostic surface

Add a feature-gated client CLI/example that enrolls, lists connections, browses,
shows redacted route/trust status and downloads an authorized immutable file. Use
real client services, Keychain when running on macOS and explicitly injected test
storage only in tests. Do not ship a command-line bearer-token argument or log
secrets. Demonstrate restart persistence and ranged transfer over encrypted LAN
without a public route. This is the user-testable Wave 4 native surface; Slint remains C03.

### S04.1 — Rotation contract

Use authenticated TLS delivery of versioned trust material, not a new homegrown
signed-message protocol. ServerId remains fixed; trust generation increases.
A transition contains server ID, generation, old/new certificate fingerprints,
new PEM/SANs, activation and grace deadlines. Clients accept it only over an
already trusted authenticated route and reject regressions or a wrong old identity.
Persist both pending and active trust atomically before acknowledging readiness.

Server states: active -> prepared -> activated-with-grace -> retired. Preparation
may be cancelled before activation; activation never rolls back to an earlier
generation. Stage a new cert/key directory and durable operational record before
changing the listener. Startup reconciles the chosen generation before serving.
Keep this state and all private keys out of portable archive exports.

An old enrolled certificate cannot validate a new unannounced certificate. Therefore
provide an explicitly configured old-certificate LAN grace listener during rotation,
or an already-enrolled independent Web-PKI route that can deliver the transition.
Default grace is 30 days, bounded by the old certificate's real expiry. Preflight
must refuse an automatic rotation that would strand all unenrolled-for-next-generation
clients without a configured recovery route; administrator may explicitly choose
re-enrollment recovery instead. Do not silently open a new private management interface.

Offline devices reconnect through old trust during valid grace, fetch the transition,
persist it, then use the new listener. After grace/expiry, explicit re-enrollment
is required if no independently trusted route remains. Browser certificate trust
and passkey origins are unaffected by native trust persistence; document them separately.

**Tests first:** state transition table, generation replay, wrong old fingerprint,
certificate/SAN mismatch, no valid grace route, deadline/expiry with a fake clock,
secret redaction and interruptions between every durable step. Security review must
approve these tests and the route bootstrap before S04.2 changes live listeners.

### S04.2 — Server rotation lifecycle

Implement prepare/status/activate/retire via recently authenticated administration
commands and explicit local recovery CLI. Use versioned TLS directories plus a
single atomic active-generation pointer; never replace cert.pem and key.pem as
independent writes. Integrate Axum TLS reload and the configured grace listener.
The grace listener exposes the client surface, never administration by implication.
Serve transitions through authenticated client routes without disclosing keys.
Bound retained generations; an administrator can explicitly retire old trust early
with a visible device-recovery consequence. Re-run full backup/restore with prepared
and activated/grace states, crash/restart and old/new listener combinations.

### S04.3 — Client rotation lifecycle

Add the state machine to connection persistence. Commit new trust before selecting
new routes; interruption may retain both trusted generations, never neither.
Do not send credentials after an unexpected certificate failure to “see whether
the server ID matches.” Try only already authorized recovery routes. Failures
produce a clear pairing/recovery action. Test online and offline clients, grace
expiry, revoked credentials, stale transitions, concurrent updates, server/client
restarts and full restore. Perform real TLS handshakes; mocked fingerprints alone
cannot close S04.

### C02.1 — Discovery and probes

1. Advertise a versioned `_junk-collector._tcp.local` DNS-SD service on explicitly
   enabled LAN interfaces, with port/protocol hints and non-authoritative ServerId.
   No credentials, collection names or user identities in TXT records. mDNS records
   are candidates only. Tailnet/public hosts remain explicit authenticated endpoints;
   `.local` is not a tailnet routing protocol.
2. Limit candidate queue to 64/server, probes to four concurrent/host-safe timeouts,
   and per-server configured routes to 32. Probe without credentials until the C01
   trust condition passes. Cache TTLs; discard expired discovered hints unless
   separately configured. Re-resolve names safely on network changes.
3. Inject clock, resolver and probe interfaces for deterministic policy tests;
   production uses real discovery/TLS. Respect interface scope for IPv6/link-local
   addresses; do not downgrade HTTPS because a literal IP lacks a certificate SAN.

**Tests:** forged TXT/ServerId, duplicate/expired adverts, private/public origins
with wrong trust, interface changes, IPv4/IPv6 and full tailnet hostnames. No
credentials reach an unverified probe. [RFC 6762](https://www.rfc-editor.org/info/rfc6762/)
defines mDNS's local-link behavior; test actual multicast separately from fake discovery.

### C02.2 — Health and transfer failover

- Prefer authenticated healthy direct LAN. Otherwise use healthy enrolled routes
  by measured latency. Preserve the current same-class route unless another is at
  least 25 ms faster on three consecutive probes. Promote a recovered LAN after
  three successes; fail over immediately on a failed active transfer. Mark routes
  unhealthy after two failed probes, with a 10-second cooldown before retry.
  Refresh health at most every 30 seconds while active, or on network change/action.
- Transfer manifests bind ServerId, RepresentationId, file identity, RecipeId if
  present, length and SHA-256. Persist the last fsynced offset and expected manifest
  in transfer scratch (not a full offline-library database). After a route change,
  use Range plus a strong If-Range validator. Never append a full 200 response to a
  partial file; restart safely in separate scratch or report an identity change.
- Validate 206 Content-Range start/end/total, ETag, content encoding and final digest;
  request identity encoding for byte ranges. Reject changed identity, wrong offset,
  truncated/oversized bodies and redirects. A 416 is not proof of completion: verify
  expected length/digest. Flush progress before publishing the final local file.
- C02 implements transport continuation, not pinning/eviction/offline browsing from C05.

**Tests first:** LAN disconnect mid-file -> public continuation; restored LAN avoids
flapping; two routes return different content under copied IDs; ignored Range;
wrong 206/ETag; lost final response; client restart; expired/revoked credentials;
no Internet with healthy LAN. Use real TLS servers and deterministic disconnect
barriers. Range/validator behavior follows [RFC 9110](https://www.rfc-editor.org/rfc/rfc9110.html).

### C02.3 — Network qualification

Run deterministic loopback multi-route tests in the normal suite. Add opt-in real
LAN, mDNS and tailnet tests with a recorded topology, interface names, TLS/trust
setup, client/server platforms and observed byte route. Disconnect public access
and prove actual authorized downloads stay on LAN. Test public-to-LAN recovery and
old-to-new trust rotation during a partial transfer. No available tailnet/hardware
means that matrix row remains externally unqualified, not a fake passed probe.
Expose redacted diagnostics in the client harness for a human to verify route choice.

## Required verification and final review

For a Collector code child run:

```text
python3 scripts/check_architecture.py
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets -- -D warnings
npm --prefix web run check
npm --prefix web run check:api
npm --prefix web run build
cargo build -p collector-server
```

Run both `npm --prefix web run test:smoke` and
`LOCAL_SETUP=1 npm --prefix web run test:smoke` after relevant UI/transport changes.
Use temporary profiles and explicit Chrome configuration; a sandbox launch failure
is not an application pass. Documentation-only children need document/fixture/link
checks, not an unnecessary server rebuild. Shared extractions require the exact
Junk Libs/Retro/Phono/Trash checks for every changed consumer recorded in the reuse
map. If tools/encoders are optional, run both disabled and enabled feature gates.

### W4.R — Integrated acceptance trace

Create `docs/wave-4-review.md` only when reviewing implemented work; do not prefill
passing claims. Trace each root to exact tests, commands, fixtures and commits.
Run these combined scenarios in addition to child checks:

1. Archive an unidentified game and CD, verify supported operations, obtain useful
   tentative metadata, make manual corrections, refresh during provider failure,
   generate FLAC/Opus, export, rebuild and restore without changing masters/history.
2. Cancel each new Job kind before commit, restart while cancellation awaits its
   worker, retry failures, and interrupt immediately after commit. Verify stable
   output identities, no duplicate Evidence and no pre-commit review after commit.
3. Remove grants before and after processing/derivative commit. Test all public
   exposure combinations and viewer/private-field filtering on every new route.
4. Full backup includes pending Imports and TLS rotation state. Archive-only restore
   gets no access or native trust. An independent clone cannot inherit credentials.
5. Pair through a public origin, learn a trusted LAN route, remove Internet access,
   download over LAN, rotate trust, fail over mid-transfer and verify final bytes.
6. Review dependencies and supported-operation matrix. No strings replacing typed
   contracts, hidden parser/provider orchestration in UI, whole-disc allocations,
   fabricated success, incomplete assets, or undocumented optional-tool requirements.

Mark local implementation separately from external qualification. No numerical
performance claim is made for 100,000 items/one million records in this wave;
P06/R02 own that measurement. Preserve all earlier acceptance tests, especially
Wave 3 final-chunk replay, interrupted cancellation and post-commit recovery cases.

## Completed browser-player implementation prompt (historical)

```text
Implement the bounded web browser-player child after D07.3 only. Read AGENTS.md,
docs/architecture.md,
docs/implementation-status.md, docs/release-requirements.md,
docs/decisions/pre-release-evolution.md, docs/music-first-milestones.md,
docs/wave-4-review.md, D07.1–D07.3 in docs/wave-4-plan.md and the "Small web
playback child after D07.3" section of docs/music-first-milestones.md. D07.1 and
D07.2 are committed checkpoints; D07.3 is reviewed and corrected locally. Preserve those services,
routes and unrelated sibling edits. Do not commit or spawn agents.

Use D07.3's shared authorized ordered availability DTO and same-origin media URL.
Use the typed can_prepare field for preparation eligibility; never inspect error
or display strings to determine capabilities. Preserve the command's receipt over
client inputs, with mutable encoder/source resolution only on first execution.
Mount viewer playback from the client bundle, without importing the curation
bundle or requiring curation exposure. Retained output must remain accessible
when its module is absent. Group the disc queue by Capture and source identity;
do not concatenate different captures or flatten profile variants into duplicate
tracks. Cancel obsolete UI requests/pollers when changing items or closing UI,
resume availability refresh for existing jobs, and keep errors visible.
Ensure styles are actually included in served assets, following web/build.mjs.
Do not add a browser-only track model, backend player, session queue protocol,
streaming/transcoding endpoint, credential-bearing URL or conversion on media GET.
Keep conversion as the existing explicit durable curator command. A viewer can
play already-published permitted output but cannot prepare it. Current revocation,
expired sessions, unavailable formats and playback failures must remain honest.

Add a small in-memory ordered disc queue beside the existing artwork and text:
play/pause, seek, next/previous, current-track state and automatic advance after a
track ends. Prefer an available FLAC representation; if the target browser cannot
decode it, show useful download/open guidance rather than claiming playback.
Preserve track-number/title fallbacks and useful artwork placeholders. Do not add
persistent favorites, cross-device queues, offline cache, sample-accurate/gapless
claims or DSP parity; those remain native/offline milestones.

Exercise the real web workflow with actual generated derivatives and a viewer
account. Prove the media element actually begins playback, pause/resume changes
state, a nontrivial seek succeeds, next/previous selects the correct immutable
track, ended advances to the following track, refresh reconstructs a clean queue,
and revoked access or session expiry produces an honest failure. A click, assigned
URL, successful fetch/download or synthetic DOM event alone is not playback or
transition qualification. Keep the existing D07.3 preparation workflow covered
on the curation surface and rebuild the server after embedded asset changes.

Run repository checks with COLLECTOR_TEST_FLAC=1 where flac is installed. Record
the exact browser/version and codec outcome. Report real opusenc, owned-disc,
gapless/sample-accurate playback and deployment qualification as unrun unless they
were actually exercised. Stop after this bounded browser child; update status,
validation and the next prompt together without beginning D06, C01 or native C04.
```

## Ready-to-send implementation prompt

```text
Implement P05.1 only: portable export and change inventory. Read AGENTS.md,
docs/architecture.md, docs/implementation-status.md, docs/release-requirements.md,
docs/decisions/pre-release-evolution.md, docs/handoff-backlog.md, the Wave 4 shared
prerequisites, the complete P05.1 child and P05 root acceptance criteria.
D06.1–D06.3 and D07.1–D07.3 are reviewed and corrected locally. Preserve unrelated
work. Do not commit, spawn agents, start P05.2/P05.3, or add native/offline features.

First trace existing archive journals, revisions, processing sidecars and export
ports. Keep policy in application services and filesystem/locking/copy mechanisms
in the archive adapter. Freeze a complete committed revision inventory under the
single writer lock, then copy immutable bytes outside it. Export to a new directory
bundle with readable versioned TOML, stable identities, revisions and digests.
Include dependency closure, tombstones, masters/assets/derivatives, recipes,
snapshots, observations, metadata authority and opaque module bytes. Omit accounts,
credentials, trust, jobs and unrelated private state. Verify every copied file and
write the completion marker last. Preserve all sources; reject overlapping roots,
symlinks, special files and traversal. Bound work and cancellation.

Implement versioned change inventory/delta semantics bound to the complete base
inventory digest. This is portable offline transfer, not replication or concurrent
writers. Do not add compatibility wrappers or migrations for checkpoint stability.
Use focused tests first for coherent export during later writes, incomplete copy,
invalid/missing dependencies, changed base, unknown modules, tombstones and private
state omission. Prove fresh archive import and reindex retain loans, manual overrides,
evidence and IDs, with explicit access reassignment. Reuse existing import/recovery
mechanisms; do not silently leave round-trip acceptance for another child.

Run architecture/fixture checks, formatting, workspace tests, clippy and generated
API/web checks as affected. Rebuild embedded assets if changed. Update implementation
status, P05.1 ledger, review notes, validation and ready-to-send review prompt together.
Report incomplete acceptance honestly. Stop for review after P05.1.
```
