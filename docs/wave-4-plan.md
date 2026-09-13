# Wave 4 implementation handoff

Status: **W4.2 reviewed; CT.1–CT.4 reviewed and corrected; D04.1 reviewed and corrected; D04.2–D04.3 are next**. Baseline: `c5c3ecf` (Wave 3 plus recovery
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
| 5 | D04.2 | D04.1 | DAT snapshots and complete-match policy | Routine plus vectors |
| 6 | D04.3 | D04.2 | Game jobs, curation and supported-operation matrix | D04 root review |
| 7 | D05.1 | W4.2 | Exact CD PCM/layout and bounded checksum readers | Review; sibling gates |
| 8 | D05.2 | D05.1 | AccurateRip acquisition and result policy | Review |
| 9 | D05.3 | D05.2 | CD jobs, curation and fixture qualification | D05 root review |
| 10 | D06.1 | D04.3, D05.3 | Durable metadata authority and resolver | Review |
| 11 | D06.2 | D06.1 | MusicBrainz, CAA and DAT-backed game metadata | Routine plus provider tests |
| 12 | D06.3 | D06.2 | Metadata review and artwork UI | D06 root review |
| 13 | D07.1 | D05.1, W4.2 | Immutable conversion recipes and exact encoders | Review; sibling gates if changed |
| 14 | D07.2 | D07.1 | Recoverable derivative publication | Review |
| 15 | D07.3 | D07.2 | Availability, ranged access and web controls | D07 root review |
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
is recorded in [validation](validation.md). D04.1 is now implemented locally and
awaiting review; D04.2 has not started.

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

### D05.2 — AccurateRip provider and result interpretation

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

### D05.3 — CD workflow and root acceptance

Wire structural and AccurateRip operations into the module registry, jobs and web.
Show separate integrity, structure and audio-catalog evidence, including per-track
status and offset units. An unavailable provider must not stop an unidentified
CD from being archived or turned into a derivative. Run a BIN/CUE and a synthetic
redumper package through staging, analysis, publication, verification and reindex.
Add an opt-in real-dump oracle runner with documented inputs and reference outputs.
If no owned dump is available, record the real-layout gate as externally unqualified
and do not advertise that layout as independently validated.

### D06.1 — Durable metadata authority

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

### D06.2 — Providers and artwork

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

MusicBrainz's request-rate/User-Agent policy is documented in the
[official API guide](https://musicbrainz.org/doc/MusicBrainz_API). Recheck provider
contracts at implementation time and record drift instead of inventing endpoints.

### D06.3 — Metadata review UI

Add a curation panel for alternatives, provenance, explicit selection, per-field
overrides and clear-override actions. Preserve drafts through job polling/conflicts;
allow reopening a saved decision after refresh. Display effective text/artwork in
browse/search/detail rows with tentative versus definitive identity distinct from
artwork absence. Test actual provider-job results, outage/refresh without lost edits,
foreign-language labels, viewer redaction, keyboard navigation and 390px layout.
Do not render raw provider HTML or include unreviewed URLs as executable content.

### D07.1 — Recipes and exact conversion

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

Expose recipe/output availability and enqueue commands through services; add web
buttons for FLAC default or an explicit smaller Opus copy. Return the immutable
representation/file identity, content length, media type and digest, with a strong
ETag derived from actual immutable content. Implement Range/If-Range correctly
for originals and derivatives before C02 relies on them. Verify viewer download
permissions, current grant revocation, disabled curation APIs, byte ranges and
FLAC source equality through HTTP. Native offline management remains C05.

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

## Ready-to-send implementing-agent prompt

```text
Implement D04.2 and D04.3 next, in that order. D04.1 and CT.1–CT.4 are reviewed
and corrected. Complete D04.2 matching/vector gates before integrating D04.3.
Stop for architectural conflicts or missing required qualification; stop for review
after D04.3. Do not proceed into D05.
Preserve the existing working-tree changes. Read decisions/pre-release-evolution.md
alongside the required architecture/status/release documents. Inspect the immediate
reader/result contract, qualified primitives, independent vectors, and migrated
callers together. Do not add compatibility wrappers solely for a wave checkpoint
or scaffold unimplemented production APIs. Preserve real data and sibling consumers.
Read AGENTS.md and the required architecture/status/release documents first.
Follow the fixed decisions, dependencies, tests and stop conditions exactly.
Use TDD for the listed failure/state/algorithm cases and real adapters for production.
Do not implement another child to bypass a blocked dependency, silently broaden
support, reset user state, or claim unavailable external qualification as passed.
Preserve unrelated edits and separate shared-library extractions from Collector
integration. Update the ledger, handoff backlog, implementation status and validation
with actual results. Stop at the child's review checkpoint with a concise diff/test/
limitation report. Do not commit until authorized. Do not spawn other agents.
```
