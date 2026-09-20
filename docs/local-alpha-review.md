# Local-alpha review

## LA1 submitted implementation — 2026-09-16

LA1 is implemented locally and submitted for integrated review. This record is not
an independent review or permission to begin LA2.

### Implemented boundary

- `collector-app` owns typed plan/accept/history/lifecycle contracts and a focused
  guided-ingest coordinator. It composes existing provisional Import, durable Job,
  processing, publication, metadata-resolution and Asset effects.
- `collector-store` private schema 13 adds only guided intent and restartable cleanup
  state. Portable sources, Captures, Representations, observations, Assets and
  authority records retain their existing archive ownership.
- `collector-server` resolves configured direct inbox selections, freezes/rechecks
  their inventory, exposes generated `/api/v1/ingests` routes, and runs paginated
  startup and periodic reconciliation alongside the existing durable workers.
- The React collection action plans browser or inbox packages, uploads/resumes exact
  browser files, renders durable progress/attempts/history, and submits lifecycle
  decisions. It does not schedule provider or publication steps.

### Authority and failure behavior to review

- A filename-derived title is the initial inventory baseline, not a title override.
  Later automatic candidates may display; a later manual inventory edit survives
  refresh and retry.
- CAA bytes publish as a normal cover Asset and bind to the immutable automatic
  metadata candidate. Guided enrichment does not set manual artwork authority.
- Missing analysis support, provider outage or no configured provider does not roll
  back source preservation. The history exposes a partial/failed optional outcome.
- Retry reuses deterministic Job/output identities. Cancellation retains bytes until
  explicit discard. Discard is forbidden once publication begins/commits and uses a
  durable cleanup record for restart reconciliation.
- Published correction changes only the Capture association through its normal
  revision contract; originals and evidence are retained.

### Submitted regression evidence

Focused Rust tests cover receipt replay and changed result ownership, stable output
identities, intent-to-job restart recovery, successful MusicBrainz/CAA automatic
enrichment, unavailable-provider/manual-title authority, retry, cancellation,
discard byte reclamation, cross-user denial and published association correction.
HTTP coverage resolves and stages a configured inbox, replays acceptance, completes
the server-owned pipeline and hides history from another user. The real Chrome
smoke adds a browser package without a pre-created Item, closes the dialog, reopens
durable history, waits for preservation, opens detail and observes the new entry.

### Deliberately remaining

P04 search/filter, terminal dismiss/archive/restore, selected/bulk cleanup and its
full mixed-race completion review remain open. U05 still requires its complete
operation matrix and application-wide interface audit. LA2 preparation controls,
native clients, a new online game provider and P05 are untouched. Live
MusicBrainz/CAA, user-owned dumps, target deployment, other browsers and real-dump
format qualification were not run; deterministic providers and synthetic package
fixtures do not satisfy LA3.

### Review checklist

1. Trace plan-token and receipt binding before mutable inbox/provider resolution;
   verify changed inputs cannot reuse acceptance and all stable identities replay.
2. Inspect recovery at intent, queued/running Job, publication commit and cleanup
   boundaries; look for duplicate effects or authorization bypass after grant loss.
3. Challenge readiness on missing/ambiguous anchors and ensure preservation remains
   available without describing structural analysis as catalog verification.
4. Race manual title/artwork edits with provider completion and retry; verify no
   automatic result becomes manual authority or silently mutates candidate history.
5. Exercise stale revisions, cross-user/collection reads, cancellation before and
   after commit, discard with active work, and association correction retention.
6. Re-run generated-contract, workspace, embedded-asset and temporary-profile
   browser gates. Keep unrun external qualification explicit.

## Independent review corrections — 2026-09-16

The integrated review found and corrected receipt/preflight ordering, first-page-only
recovery, missing periodic inbox dispatch, post-publication polling, ambiguous
terminal workflow state, initial metadata-authority crash recovery, replay against
only the latest observation list, duplicated provider mapping, cancellation/retry
re-admission, premature cleanup receipts, upload/cleanup races, and stale association
responses. Readiness now belongs to modules, shared result DTOs replace duplicated
JSON field probing, and failed structural analysis does not block preservation.

Browser coverage now imports a CD package with enrichment enabled, closes/reopens
history and waits for provider metadata and artwork. The first expanded run exposed
a mismatched disc-ID fixture (correctly rejected) and a close-before-upload-complete
test race; both test issues were corrected. Existing playback checks remain included.

Current limitations are explicit in implementation notes: name-level preflight,
atomic multi-package admission, no guided split/anchor editor, and unknown historical
attempt start times. Broader P04/U05, real dumps/live providers and LA3 remain open.
Final gate results are recorded in validation.md after execution.

### Review disposition

LA1 is accepted for the bounded local-alpha journey, with the documented intake and
external qualification limits above. Both HTTPS and local CLI-setup Chrome runs
passed; the workspace run passed 285 tests with the owned-disc oracle left ignored.
Final focused regressions cover progress projection and generic provider-Job
cancellation. Architecture, contracts, formatting, Clippy, web build/check and
embedded server build passed. The next assignment is LA2; no commit or LA2 work was
performed. This is a useful backup checkpoint before that delivery.

## LA2 submitted implementation — 2026-09-16

LA2 is implemented locally and submitted for integrated review. This section records
the implementation boundary and evidence; it is not an independent acceptance or
permission to begin LA3.

### Implemented boundary

- `collector-app` owns album/source selection, the 256-member admission bound,
  frozen batch/member identities, reuse of active/published work, aggregate outcomes,
  retry policy and cancellation ownership. The batch is an existing durable Job
  kind whose fan-out uses the existing per-track derivative effect and publication.
- `collector-store` persists the frozen batch through existing Job tables, recovers
  it through normal leases/startup, and cancels only owned child Jobs that are not
  referenced by another live batch. Guided-import preparation intent stores only
  the batch reference/error alongside its existing private intent record.
- `collector-server` exposes generated plan, submit/list/detail, cancel and retry
  routes. The same service is invoked after guided publication when its optional
  checkbox was accepted; no second queue, encoder or provider pipeline exists.
- React defaults all unambiguous associated discs, requires explicit alternative
  source choices, shows durable batch/member progress and does not hold a page-wide
  busy state for accepted work. The viewer retains its authorized browser player,
  codec/download fallback and revocation cleanup. Effective cover art appears beside
  album preparation/player text.

### Authority and failure behavior to review

- A batch freezes selected preservation sources, FLAC profile, exact single-track
  recipes and the probed encoder version. It never mutates a master or metadata/
  artwork authority. Optional provider, artwork and verification outcomes are not
  preparation eligibility.
- Repeated/overlapping requests from the same actor converge on deterministic child
  Job/output identities. Cancelling one overlapping batch cannot cancel child work
  still referenced by another batch or a pre-existing direct request. Generic Job
  cancellation is fenced while a live batch owns the child.
- Failed/cancelled children stay attributable per track and can be retried; valid
  published Representations are counted from the Capture and reused without another
  encode. A completed child without its Representation is reported as failure.
- Missing encoders and unreadable layouts remain explicit blockers/ineligible
  outcomes; preservation and original reads remain usable. Permission loss is
  rechecked by both batch admission and every child execution/publication.

### Submitted regression and browser evidence

Focused Rust tests cover two associated discs, ambiguous alternative-source planning,
missing encoder readiness, overlapping batches, stable shared child IDs, shared-safe
cancellation, deliberate one-track validation failure, partial outcome, retry,
restart recovery, guided import preparation and already-valid output reuse. Existing
derivative tests continue to cover permission revocation, source mutation,
cancellation and publication crash recovery. HTTP contract coverage includes all five
new routes and the generated OpenAPI/TypeScript shapes.

The temporary-profile browser smoke passed in Google Chrome 153.0.8010.37 with FLAC
1.5.0 behind a three-second test-only delay wrapper. It submitted two overlapping
album batches before completion, kept refresh and inventory-title controls enabled,
opened a second browser page and played the first published track while another
member remained queued/running, then observed server-owned `2/2 ready` progress and
two real FLAC media links. It also exercised actual pause/resume, seek, next/previous,
natural-ended advance, clean reload, session expiry and grant revocation. Guided
import exposed the optional preparation checkbox and durable preparation step. The
same run retained artwork/text rows, generated assets, provider fixtures and virtual
WebAuthn coverage.

### Deliberately remaining

The delay wrapper only controls timing; every output still comes from and is decoded
by the real configured FLAC 1.5.0 binary. Deterministic Store regressions separately
prove overlapping-work cancellation ownership and restart behavior. The broader U05
audit still owns other application-wide blanket locks. P04 bulk/search/archive cleanup remains open. Opus,
non-Chrome browsers, user-owned dumps, live providers, target deployment, scale and
the stopped-server backup/restore drill remain unqualified LA3/full-release work.

### Review checklist

1. Trace immutable batch scope and receipt binding before mutable encoder/source
   resolution; try repeated, overlapping and alternative-source submissions.
2. Interrupt before/during fan-out and child execution/publication, then restart;
   verify no accepted member disappears and no valid output re-encodes.
3. Race batch/direct/guided ownership and cancellation. Confirm only owned unshared
   work stops and that generic Job controls cannot bypass this policy.
4. Force missing tools, capacity limits, one-member failure and grant loss; inspect
   aggregate/member states, retry attempts and master preservation.
5. Exercise close/reload, another queued request, unrelated editing/browsing and real
   playback while work is active. Verify artwork, codec fallback and revocation.
6. Re-run architecture, contracts, Rust/web gates, embedded build and temporary-
   profile Chrome. Keep all external qualification limits explicit.

## LA2 independent review corrections — 2026-09-17

The review found cancellation ownership frozen at planning time, an unfenced
check-then-enqueue race, and exact-input conflicts when identical requests were
made at different times. Album policy now lives in collector-app/album_preparation.rs;
conversion and availability remain in derivatives.rs. Private indexed member
references and direct-request pins are committed with admission. Cancellation
consults current retained requests, including a direct request made later, and a
child can only be admitted under a live, uncancelled parent claim. Explicit direct
cancellation releases its pin. Repeated immutable outputs retain the first Job's
creation time while all other frozen inputs must still agree.

Failed admission no longer leaves absent children permanently queued. Batch views
include admission state/errors, current role-based actions and track outcomes.
Retry retains attempt diagnostics, respects cancelled guided imports and does not
re-encode published outputs. Guided recovery finds an already-admitted deterministic
batch before rebuilding its plan, checks the Capture association before new
automatic preparation, and shares the same outcome projection as explicit batches.
The guided history can retry failed preparation as its own step. Admission commits
the guided intent-to-batch link atomically; cancellation also finds batches through
their durable import request, covering an older interrupted coordinator with a
missing pointer. A regression verifies that cancellation stops its children and
that retry cannot resurrect the cancelled import.

Scope now treats multiple captures of one known physical carrier as alternatives.
Another curator's active preparation does not expose their private Job identity or
break admission: each actor retains an authorized request, while publication
converges on the immutable representation. Admission is bounded to 256 members per
batch and 32 outstanding batches per user; retries use the same capacity guard.
History prioritizes unfinished batches over recent terminal history, including more
than fifty cancelled requests that still refer to a shared active child.

The browser fences detached mutation continuations, retains idempotency keys after
uncertain submissions, preserves deliberately unchecked discs during refresh and
shows admission failures. Availability polling also discovers work begun in another
session. The real browser regression now tests an independent track submission,
overlapping album submissions, a saved inventory mutation during preparation,
preserved checkbox selection and actual playback while another track remains active.

Focused Store regressions cover advancing timestamps, later direct requests,
last-owner cancellation, cancelled-parent late admission, failed-parent retry,
cross-curator privacy/role changes and same-carrier alternative sources, in addition
to the worker's partial-failure/restart/output-reuse tests. Final gate results are
recorded in validation.md; this review does not qualify owned dumps, live providers,
target LAN deployment, Opus, other browsers, or the backup/restore drill.


The local-setup browser run additionally exposed a stale-row race after Capture
publication: immediately reopening curation could inherit the old browse graph.
Curation now fetches the authorized Item when opening and fences superseded/opened
panel callbacks. The slow-FLAC fixture delays conversion rather than the version
probe, so load cannot turn a five-second startup probe into a false missing-tool
result. Browser waits are bounded and concurrent test waits attach rejection
handlers immediately; a wrong test selector was corrected without changing product
labels.

The HTTPS metadata-detachment fixture now drains its intercepted request before the
next read and puts a deadline on both operations; this removed an intermittent
harness stall without changing the production metadata workflow.

## LA3 qualification progress — 2026-09-17

LA3 is in progress and not submitted for acceptance. In addition to the disposable
procedure regression, the selected Mac now has an actual owned-corpus and
stopped-server execution record in `docs/local-alpha-installation.md`. Five whole
package copies cover a nested mixed-mode game disc, flat two-track Redumper/BIN+CUE,
16 existing FLAC sidecars and two damaged inputs. Final source/copy comparisons are
unchanged. The browser preserved and browsed originals, retained structure or raw
hash observations, prepared and played two real FLAC tracks, kept a manual title
across restart/restore, exposed the absent-provider outcome, resumed an interrupted
1.09 GB game stage with the same Job/attempt, and exercised a local Viewer through
playback, denied curation, grant revocation and session revocation.

The actual stopped copy covered complete archive/private-state roots, configuration,
TLS and unfinished scratch. Exclusive Store opens proved quiescence; source,
backup and pre-open restore manifests matched exactly. The restored profile kept
the same ServerId/certificate, accounts, Viewer grant, representative
original/evidence/metadata digests and four unfinished Jobs. `doctor`, `reindex`,
resumed publications and restored playback passed. The original and restored
identities were never run concurrently, and the restored identity was stopped and
opened exclusively again after all work settled.

The journey found and corrected three owning-layer defects: CUE and Redumper TOC
companions now have an explicit signed/durable anchor choice reused by preparation;
the music-only anchor extension is absent from game operation inputs; and an open
import panel follows the server's published Capture association until the curator
edits it. Focused regressions and repaired real-browser checks pass. An early import
and preparation failure remain recorded as failed attempts rather than being hidden.

At this checkpoint, these results did not satisfy the live-provider or second-device
gates. A real MusicBrainz/CAA User-Agent/contact and response, a configured game DAT
if catalog identity is required, and the intended second physical LAN browser with
trusted HTTPS remained absent. Provider cover/text refresh and second-device
playback/denial/revocation were unqualified. LA3 remained unaccepted.

## LA3 live-provider continuation — 2026-09-17

The project URL was accepted as the MusicBrainz/CAA User-Agent contact, avoiding a
personal or GitHub noreply address. The restored profile reused the existing
preserved CD Import. MusicBrainz acquired `HOME` by `B’z`; automatic CAA enrichment
published a real JPEG cover, while `LA3 Manual Album Title` remained the manual
display authority. The restored browser showed the cover beside the manual title
and provider subtitle and played the existing FLAC past three seconds before a
verified pause.

The live path found three bounded defects and corrected them in their owning layers:

- the provider adapter's Archive.org allowlist did not include the observed
  `dn<digits>.ca.archive.org` HTTPS storage redirect;
- CAA's numeric image ID was rejected by a string-only response DTO; and
- the guided coordinator read a retried catalog Job's raw result without unwrapping
  the completion receipt that preserves prior-failure history.

Focused regressions cover the exact CDN hostname shape and hostile lookalikes,
numeric plus string CAA image IDs, and failed-listing retry through automatic
metadata/artwork publication. The real listing retained both failed attempts before
success, so the repair does not erase diagnostics. MusicBrainz identity and artwork
remain descriptive provider evidence, never catalog or audio verification.

This advances but does not accept LA3. The intended second physical LAN browser is
still unavailable, so browser-trusted HTTPS, Viewer playback, denied curation, grant
revocation and active-session revocation remain unqualified on that device. A game
DAT is still absent if DAT-backed identity is required for adoption.

## LA3 local-DAT continuation — 2026-09-19

The restored installation cached the standard Redump PlayStation ZIP/XML unchanged
and built a separate, manifest-recorded 10,001-row full-release view. Collector
archived and activated the derived bytes for `sony-playstation` in the
`org.junk.game.cd.raw-2352-track` v1 domain. The first real comparison was honestly
partial: all four owned track SHA-1 values matched `Brave Fencer Musashi (USA)`, but
the provider row's CUE checksum made it appear to contain five raw-2352 units.

That defect was fixed at the provider adapter boundary. Raw snapshot bytes remain
immutable, while indexed rows explicitly retain non-domain provider files in
`excluded_files`; only `.bin` files with nonzero lengths divisible by 2352 enter
track matching. A row without any qualified track fails closed. Focused provider
and module regressions passed, and a full reindex of the same snapshot succeeded.
The repeated browser journey produced unique complete-match observation
`1667ceb8-11ac-55a7-96a2-bae41a97ce2b` for four SHA-1 tracks and provider record
920. Historical partial observation `a8f93fb8-968d-5118-8b44-d828399f37b6` remains
visible. Applying its automatic title changed descriptive metadata to
`Brave Fencer Musashi (USA)` while the UI continued to state that metadata does not
upgrade verification evidence. This verifies the analyzed catalog track set; it
does not verify CUE text, subchannel data or every preserved sidecar.

The bounded cache script also obtained a 2,661-row Headerless No-Intro NES DAT with
provider-side full-release/licensed/in-life filters. Collector parsed and indexed
all 2,661 rows as snapshot `9e3645f9-e600-5ccc-8e30-030932e023d7`; no owned NES
dump exists, so no match is claimed. Nintendo 3DS was deliberately excluded as
unsupported. The later N64 form submission triggered DAT-o-MATIC error
1260, `Unknown filter`, after which the provider refused the network. Further
requests stopped. The script now replays only fields present in the exact form,
aborts the entire batch on refusal text, uses a persistent cross-run rate stamp and
exclusive lock, and keeps ordinary requests 10–18 seconds apart with multi-minute
queue backoff. N64, SNES and PC Engine remain uncached; no result is fabricated.

The blocking LA3 acceptance gate remains the absent intended second physical LAN
device. The provider refusal and missing additional DATs are also recorded cache
limitations, not substitutes for that device gate.

## Independent LA3 review — 2026-09-20

Reviewed the LA3 anchor selection, provider repairs, catalog coverage and cache
helper against the existing LA2 baseline. Corrected silent exclusion of malformed
BIN/unknown payload rows; module-owned coverage rules are shared by parsing and
matching. Hardened the cache with expanded-size bounds, digest-checked reuse,
staged publication, preservation of existing snapshots, explicit catalog selection
and offline regressions. New DAT-o-MATIC acquisition is disabled pending review
after the operator restores access. No remote provider was contacted.

The actual owned-dump, playback and restore records remain historical qualification
evidence. Second-device trusted HTTPS, playback and authorization checks still
block LA3 acceptance. See validation.md for this review's independently run gates.

Independent validation passed: 301 Rust tests (one external oracle ignored), nine
offline downloader tests, Clippy, architecture/format/contracts/web checks, the
rebuilt server and both Chrome/FLAC journeys. No new manual owned-corpus match or
second-device result is claimed. This is a suitable uncommitted backup checkpoint.
