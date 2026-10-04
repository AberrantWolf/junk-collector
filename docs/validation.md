# Validation recorded for the foundation

## Collection lists and playlists — 2026-10-03

[Collection-list validation](validation/collection-lists.md) records portable ordered
lists, role boundaries, recovery, source resolution and real playlist playback.
CL1 is locally accepted: repository gates and the real Chrome playlist/shuffle,
mobile and keyboard journeys passed. The [delivery ledger](collection-lists-plan.md)
records the remaining scope and next handoff.

## Catalog startup and write performance — 2026-10-03

[P06 validation](validation/p06-indexing.md) records checkpoint reuse, indexed
write validation, generated scale measurements and recovery/authorization tests.
It supersedes the unconditional metadata-reindex limitation in the dated records
below; R02 hardware qualification and P05 backup/export UI remain outstanding.

## Prepared-audio startup and album display — 2026-09-28

Startup reindexes derivative records without opening or hashing their FLAC
payloads. The focused HTTP regression checked that an intact prepared track
survives restart, a same-length changed track permits lightweight HEAD but
returns `prepared_media_invalid` on GET, and a missing track returns the same
typed error. Explicit full reindex still rejects changed derivative bytes. The
browser smoke suite passed disc-position ordering with reversed source input,
the specific player error after a failed prepared-media request, successful
retry, actual FLAC playback, and access revocation that releases the media
element. The suite's first browser launch was denied by the sandbox; a later
run found and fixed retry and revocation races, and the final Chrome 153 run
passed. `cargo test --workspace`, strict workspace Clippy, architecture and
format checks, TypeScript check, web build, server rebuild and `git diff --check`
passed. This establishes no whole-collection payload check at startup; the
remaining full metadata reindex is still tracked as P06.

## Album names and import defaults — 2026-09-28

Guided imports now create one owned, present copy for a new Item; the archive
replay test checks this and confirms attachment to an existing Item preserves its
inventory. MusicBrainz observations retain medium-specific titles, with provider
and application tests covering two discs, wrong disc IDs and incomplete layouts.
Current album-wide lookup is exposed in Music; old observations need a fresh
provider response, and ambiguous releases require curator selection. These tests
do not claim a live MusicBrainz track-list qualification or automatic naming when
no usable provider match exists.

`cargo test --workspace` passed before the final focused matching refactor;
the guided-import replay test, strict workspace Clippy, architecture check,
TypeScript check, web build and diff check passed. Chrome 153 browser smoke passed
the Music lookup entry, metadata workflow, WebAuthn and real FLAC playback. The
FX-05 browser journey passed retained reuse, replacement upload resume, mobile
review and historical playback. The first sandboxed smoke attempt failed at
Chrome launch; the same test passed outside the sandbox with a temporary profile.

## FX-07 whole-interface local acceptance — 2026-09-27

The [action, growing-list and combined-journey register](fx07-interface-audit.md)
records local U05.2/U05.3 and P04.6 acceptance for implemented web actions.
No production archive or profile was used. Independent accessibility/security,
second-device, physical hardware and deployment qualifications are separate.

The account-administration query now checks administrator authorization and
search/page bounds in the application, returns username-ordered keyset pages
from SQLite, and uses an exact single-row account read. Personal sessions,
passkeys and paired devices now have creation-time/ID keyset pages. Their
focused security tasks retain profile drafts. The administration panel has
focused Collections, Game catalogs, Accounts and Server settings tasks.
Account search is scope-bound, paged, and has loading/empty/error/retry states.
Catalog rows are hidden immediately when the selected collection changes; a
completing catalog Job refreshes only its admitting collection. Metadata
candidate choices render 25 at a time with local search inside the Store's
4,096-candidate admission bound.

Collection navigation and selectors now use an authorized 50-row keyset query
with name search and role/module filters. The exact selected collection is read
with its grant check, so a direct URL or a selection outside the current search
page remains usable. Curation, access, catalog and work filters share one
searchable chooser. The former unbounded collection-list HTTP endpoint was
removed. A Store regression crosses role/module-filtered pages and denies an
exact read without a grant; `test:fx06` creates 55 additional collections,
crosses the second page and opens curation for a later result.
The reviewed debounce race is fixed in both the shared chooser and sidebar:
rows and cursors belong to their search scope, while same-query refresh retains
navigation context. `test:fx06` changes search before the old More button can
run and then crosses the next page. Game catalogs now shows collection lookup
errors and Retry before selection; configured-source errors have their own Retry.
`test:smoke` injects the initial collection lookup failure and recovers.

`cargo test --workspace` passed, including 179 archive tests and the new
multi-page account search/direct-read and personal-security regressions. The
latter crosses equal-timestamp session, passkey and paired-device pages and
checks malformed cursors. Strict workspace Clippy,
architecture/link checks, formatting, generated API comparison, TypeScript,
web build and `git diff --check` passed. The Rust server was rebuilt after
embedded web assets changed. `test:queue` and `test:context` passed.

Temporary real-server Chrome 153.0.8010.37 journeys passed: `test:fx06`
(account and session page crossing, search, focused security tasks, collection
access, recent authentication, recovery, revocation), `test:fx04` (3,000 Items
and 3,000 imports), `test:work`
(53 imports, mixed eligibility and cleanup), `test:fx05` (clean reimport,
removal/restore and actual historical playback), `test:scans` (64 MiB scan,
equivalent and independent work, invalid submission without a Job),
`test:browse` (paging, keyboard and read recovery), and `test:fx04-review`
(draft preservation and section stability). Final rebuilt `test:smoke` passed
collection-scoped catalog rows, actual FLAC playback and grant revocation,
setup, role denial, WebAuthn and mobile layout. Its offline provider fixture
also returned an explicit no-match response after three ambiguous candidates;
the completion message reported no match and the earlier alternatives stayed
available. The metadata completion contract now carries the refresh outcome.

The final rebuilt `test:work` also passed a three-import case with two
completed analyses and a failed analysis attempted twice; the UI showed all
package rows, separate analysis work and the previous failure. It injected a
real staging-directory permission failure, retried cleanup, preserved the
measured 2-byte result, then restarted with a second pending cleanup and
verified recovery. The Store regression now repeats completed cleanup and
checks that its reclaimed amount cannot return to zero. `test:scans` passed
ambiguous and no-match DAT verification in the real UI using a frozen offline
catalog and published source bytes. The rebuilt `test:fx05` and
`test:fx04-review` journeys passed after the collection fixes.

The Music panel links its 50-batch limit to the selected entry’s Work &
history. Metadata candidate rendering is bounded at 4,096 versions per Item;
page its response before increasing that limit. This does not claim the
separate independent audits, LA3 second-device or physical deployment gates.

## Work-context hardening — 2026-09-26

Checkpoint `5d1fc1a` preserves the accepted FX-05/FX-06 work before this delivery.
The architecture contract and bounded adoption list are in
[work-context hardening](work-context-hardening.md).

`cargo test --workspace` passed, including 175 archive tests, 29 HTTP tests,
nine store unit tests, LAN TLS and the recovery regression suites. The existing
private owned-CD oracle remains ignored without its user-owned dump/oracle inputs.
`derived_asset_publication_requires_its_live_source_and_exact_item` also passed
independently before the final workspace run. It demonstrates that a matching generation cannot authorize cancelled/missing
sources or a different item. The snapshot regression rejects wrong token, wrong
collection, wrong attempt and cancellation even when the caller's callback says
not cancelled. Snapshot preservation fixtures now acquire actual job claims; this
changes their invalid fixture setup, not the expected preservation behavior.

`test:context` passed stale success/error/cleanup, A-to-B-to-A scope replacement,
detachment/disposal, independent lanes, independent authentication waiters,
last-waiter cancellation, obsolete prompts, account mismatch, immutable request
body/key retry and explicit session review. `test:queue` passed. TypeScript,
API contract comparison, architecture/link checks, formatting and web build passed.
No DTO or persistent schema changed. The optional `X-Collector-Actor` transport
precondition rejects account mismatch before dispatch; its HTTP regression also
proves it cannot supply authentication and cannot revoke a session when mismatched.

Temporary real-server browser journeys passed clean-reimport recovery and retained
playback (FX-05), collection access scope switching and cancelled/new authentication
prompts (FX-06), recovery/revocation and the general smoke suite. FX-06 now explicitly
checks that session-specific retry requires reviewing the refreshed session list,
keeps an unsaved profile draft, and leaves `/me` unauthorized after the confirmed
current-session revocation. Authentication prompts and response delays are injected
for deterministic race coverage; actual login/logout/recovery and writes use the
real server. The final embedded server rebuild, repeated FX-05/FX-06 journeys and
full smoke suite passed. Smoke includes actual FLAC playback and grant revocation,
virtual WebAuthn registration/login, unavailable-module restart and mobile layouts.
Strict workspace Clippy passed with warnings denied. This delivery is locally
accepted; FX-07 and external device/deployment qualifications remain separate.


## FX-06 review fixes — 2026-09-26

The three review findings are corrected using the existing grant, authentication
and logout contracts. Scope-tagged grant results cannot supply controls for a
newly selected collection. Password/passkey prompts have per-prompt identities and
cancellable requests, so late completion cannot settle another prompt. Revoke
Current also logs out the replacement session that reauthentication may issue.

TypeScript, web build, architecture/link checks, and `git diff --check` passed.
The Rust server was rebuilt with the updated embedded assets. The extended real
browser `test:fx06` passed collection switching while the next grant response is
held, cancelled in-flight login followed by a newer prompt, preserved drafts,
successful retry, and current-session revocation after authentication (including
an explicit `/me` 401 check and reload). The journey also retained its invalid/
consumed recovery-code and other-session revocation checks. Typed recent-auth
responses and delayed responses were injected for deterministic UI race coverage;
login, mutations, logout and recovery use the temporary real server.
The full browser smoke suite also passed, including virtual WebAuthn registration/
login, login/expiry, access denial, actual FLAC playback and grant revocation,
management bundles and mobile layouts. Formatting passed. No Rust service or API
contract changed in this review follow-up; earlier workspace results remain below.

## Guided-import association feedback follow-up — 2026-09-26

The shared guided-import editor now derives pending changes from the saved Item
association and the selected destination, normalizing an unattached destination to
null. Unchanged destinations explain that there is nothing to save; only changed
destinations offer Save association. Loading is distinct from unattached, and an
in-flight destination refresh cannot replace a newer user selection. The saved
association notice no longer names the unsaved destination as already attached.
This uses the existing chooser, command and server permission contract.

TypeScript, web build, architecture/link checks and `git diff --check` pass.
The existing smoke journey was extended to cover unchanged destination, an unsaved
detach followed by reselecting the original entry, and the saved state after moving
to another entry. Browser verification of this follow-up is pending: two server
rebuild attempts stalled in rustc without output or CPU activity and were stopped
(after approximately five minutes and two and a half minutes). No compiler error
was reported. Generated web assets are updated, but the embedded Rust server still
needs a successful rebuild before this UI change is served. Earlier FX-05 acceptance
below predates this follow-up and is not evidence for its new browser assertions.

## FX-06 collection access and account recovery — 2026-09-26

FX-06 adds a collection-scoped access panel independent of Item selection and
content eligibility, server-bounded searchable grant pages, and an exact grant
revision lookup for updates outside the displayed page. The signed-out browser
redeems recovery codes into the server-issued session. One-time codes can be copied
or downloaded; closing their display asks the user to confirm they saved them.
The shared request client handles the typed recent-authentication error, keeps the
original command body and idempotency key, offers password or passkey, and retries
once after success. Cancellation leaves the form draft available. No recovery code,
password, or passkey assertion is stored in a URL or browser storage.

A temporary Chrome 153.0.8010.37 server/profile journey passes empty-collection
access, cancelled and successful reauthentication with the grant draft, recovery
into an authenticated session, and consumed-code denial. The extended run also
checks invalid-code feedback and revocation of an old browser session. The browser
forces one typed freshness response to exercise the UI without waiting five minutes;
the real Store test advances the clock to the exact freshness boundary. Existing
Store concurrency tests prove only one recovery-code redemption succeeds. The
focused grant query test crosses three pages and checks case-insensitive search.
The full smoke journey exercises virtual WebAuthn, role denial, public/private
listener composition, unavailable-module read-only access, and actual FLAC playback
with grant revocation. Physical passkey and second-device LAN qualification remain
external and are not claimed here. FX-07 still owns the whole-interface completion
review and account-list scale audit.

Required gate results: architecture/link checker, formatting, `cargo test
--workspace`, strict workspace Clippy, TypeScript check, generated API contract
check, web build, rebuilt Rust server, FX-06 browser journey, and full browser
smoke pass. The new grant pagination regression also passes. No production user
archive or profile was used.

## FX-05 clean reimport and lifecycle review — 2026-09-26

The incomplete Remove/Restore implementation was reviewed and extended into the
complete same-Item clean-reimport workflow. The temporary-server
`npm --prefix web run test:fx05` journey passes retained-original reuse,
reviewed preservation defaults, replacement-upload failure, close/reopen/resume,
unchanged inventory identity/fields, real WAV playback across replacement and
availability polling, mobile review, and removal/restoration with resumed playback.
The final mobile review was visually inspected. `test:browse` also passed removal,
reload in Removed entries, restoration, artwork, navigation and keyboard behavior
in Chrome 153.0.8010.37. No production archive or user server was used.

All nine `clean_reimport` archive regressions pass. They cover validated retained
bytes, corruption and repair/retry, default metadata/artwork preservation, explicit
authority resets, failed replacement retaining the current capture, later manual
edits, independent scan ownership, obsolete Job retry rejection, stale acquisition
completion, writer-fenced artwork, command replay, restart/reindex, and six fault
points each for admission and replacement publication. A separate caught-panic
case interrupts immediately after Capture commit, starts a second reimport, and
proves the abandoned replacement cannot reappear. Expected panic output in that
case is deliberate fault injection, not a failing test.

The architecture/link checker, formatting, TypeScript check, generated API contract
comparison, web build, strict workspace Clippy, and all-target compilation passed.
The server was rebuilt after the web assets and final lifecycle changes. The smoke
fixture now verifies its temporary FLAC wrapper before server startup, and browser
readiness has a bounded cold-start allowance; production encoder timeouts were
not relaxed. Earlier runs hit host executable-startup timeouts, recorded separately
from feature failures.

The full browser smoke suite passes, including actual FLAC playback and grant
revocation, automatic disc-ID provider enrichment, private/public role checks,
virtual WebAuthn, restart with an unavailable module, and mobile layouts. Its
retired-placeholder assertion now requires a curator-readable tombstone and
ordinary-browse exclusion instead of 404, matching Restore's contract. The
provider-title assertion waits for actual metadata before reloading. The full
28-test HTTP suite passes. The wider archive run caught a missing-job recovery
case with a retained generation row; job recreation now preserves that row,
and the existing crash-window regression passes again.

Final local acceptance passed: `cargo test --workspace` (including all 172 archive,
28 HTTP, nine store-unit and five recovery-regression tests), strict workspace
Clippy, architecture/link checks, formatting, TypeScript, API contract comparison,
web build, queue tests, and the browse, FX-05 and full smoke browser journeys.
The canonical Item-record test now checks that captures/assets are absent from the
top-level record while allowing the intentional portable historical Capture IDs.
The existing user-owned CD oracle test remains ignored without
`JUNK_COLLECTOR_W4_CD_DUMP_ROOT` and its private `oracle.toml`; this acceptance does
not claim that external qualification or the outstanding LA3 second-device gate.
FX-05 is locally accepted on 2026-09-26; FX-06 acceptance is recorded above.

## FX-05 Remove/Restore slice — 2026-09-25 (FX-05 not accepted)

The archive-backed tombstone and removed-items browse are implemented and locally
exercised. `removed_item_is_recoverable_and_keeps_inventory_across_reindex`
passes with inventory and artwork preservation, a queued Job, pending guided
Import, viewer denial for removed Item/evidence/activity/artwork reads, reindex
and restart. The crash-boundary
`removed_item_finishes_an_already_committed_import_receipt` test verifies
that a Capture committed before removal still finishes its Import receipt and
Evidence reference after restart. The real `web/tests/browse.mjs` journey
passed Remove, refresh in the Removed entries view, and Restore with the same
Item identity and artwork. Removed file rows do not offer downloads or queue
actions until restoration.

The architecture/link checker, formatting, generated API contract comparison,
web type check/build and workspace Clippy passed after these corrections. The
final `cargo test --workspace` passed, including all 163 Store archive tests and
28 server HTTP tests. The server was rebuilt with the final web assets, and the
real Chrome `web/tests/browse.mjs` journey passed again.
Clean reimport and its generation/replacement/reuse acceptance remain open.

## FX-04 review corrections — 2026-09-25

Existing-destination guided admission now rechecks the Item revision in the
Store transaction that commits the Import and intent. Replaying the original
receipt after a later Item edit returns the accepted import; a changed request
with the same key remains an idempotency conflict. The editor adopts inventory
title, subtitle, type fields and Wishlist when publication changes its Item
identity, while later updates to that same Item retain the current section and
draft. Work & history rereads every loaded page, removes successfully mutated
rows until refreshed, and drops entries that no longer match the server filter.

The targeted archive tests and temporary-server `test:fx04-review` and `test:work`
browser journeys cover these regressions. The repository gates below were rerun
after the corrections.

## FX-04 focused editing and intake — 2026-09-24

Local acceptance completed 2026-09-25.

Implementation joins new and existing-entry package intake in the guided
application coordinator. The accepted destination/revision and analysis policy
are durable; preservation-only intake publishes originals without analysis or
provider work. A timestamp-named CD fixture checks that automatic MusicBrainz
requests use the layout-derived disc identifier and contain no filename or
placeholder title. Existing destinations retain their other Captures and inventory;
manual metadata authority survives provider refresh. The selected Work & history
detail is a separate authorized URL-backed view, while current-import progress
reads one import. Destination search and inbox packages use server-side bounded
pages, and the [growing-list audit](fx04-scale-audit.md) records other surfaces.

The temporary-server `test:fx04` browser journey passed with 3,000 Items and
3,000 imports. It found and attached an Item beyond the former 1,000-entry cap,
distinguished two same-title editions, loaded a later destination page, opened a
late import with no catalog read, restored its filtered list with browser Back,
returned to detail with Forward, reloaded the detail URL, and checked mobile layout.
The full `test:smoke` browser journey passed after the FX-04 changes, including
real FLAC playback, new and existing intake, automatic disc-ID lookup without a
repeat request on browser reload, reassociation and the read-only access entry
action. The Rust HTTP test covers
inbox label loading without packages, 50-result search pages, later-page cursors and denied
cross-collection access. Focused archive tests cover existing-item admission,
reassociation, preservation-only processing and automatic timestamp-package
lookup. `test:work`, `test:browse` and `test:scans` all passed after the final
navigation change. The browse fixture now reloads after seeding through the API
to observe those out-of-band writes.

`python3 scripts/check_architecture.py`, `cargo fmt --all -- --check`,
`cargo test --workspace -- --test-threads=4` (including 161 archive tests and
doctests; one owned-corpus oracle intentionally ignored),
`cargo clippy --workspace --all-targets -- -D warnings`,
`npm --prefix web run check`, `npm --prefix web run build`, and
`npm --prefix web run check:api` passed. The Rust server was rebuilt after embedded
web assets changed. All browser runs use temporary server roots, profiles and
fixtures; the user's archive was untouched. This is local validation and does not
qualify LA3's second LAN device or general large-scale indexing.


## Persistent bottom music player — 2026-09-22

Implementation and local acceptance are complete. This is the
user-requested addition after FX-03, not completion of the remaining FX-04 scope.

- One application-owned media element persists across album navigation, closing
  details, catalog refreshes and import completion. Album views enqueue prepared
  tracks/albums; original audio files use the same dock. Preparation and download
  controls remain in the album view.
- The browser-local queue is scoped by user and origin, limited to 500 entries,
  and restores its selected track paused. Immutable item/representation/file
  coordinates bind playback. Adding/reordering/removing other tracks does not
  reload the current source; removing the current track releases buffered audio.
- `npm --prefix web run test:queue` passes for user isolation, stable selection
  through reordering, removal, signed-out mutation denial, malformed stored
  coordinates, queue bounds and disabled storage. No server queue or personal
  activity writes are added.
- The updated real-browser smoke journeys passed in Chrome 153.0.8010.37, including
  actual FLAC playback, pause/resume/seek, next/previous, natural advance, paused
  reload, closing details without stopping playback, cross-album appends and
  queue edits during playback, mobile layout, import completion, session expiry
  and grant revocation. Buffered audio stops when access is lost even away from
  the album. The final rebuilt-server run also passed same-browser user isolation and current-track removal, with guards against obsolete play promises and late media events.

The first workspace run encountered a provider-fixture request read returning
WouldBlock. The fixture now explicitly puts accepted sockets into blocking mode
instead of relying on inheritance from its nonblocking listener; the two-second
read/write bounds remain unchanged. Production networking is unchanged.

TypeScript, web build, embedded-server rebuild, API comparison, architecture and
formatting and strict workspace Clippy checks passed. The corrected provider
fixture passed both in isolation and with its complete eight-test suite. `cargo test --workspace -- --test-threads=4` passed: 313 tests, zero failures,
one intentionally ignored owned-corpus oracle. All
browser fixtures use temporary archives/profiles. The user's running server and
archive remain untouched; the rebuilt binary needs a restart to serve the new UI.

## FX-03 work and history — 2026-09-21

Implementation and local acceptance are complete. FX-02 was
committed as `3493cd8` before this delivery. FX-03 and the persistent-player follow-up are included in the 2026-09-24 checkpoint.

- Requester-scoped server queries filter by collection, item/unattached state,
  package/file name, lifecycle status and dates before bounded keyset pagination.
  Import/preparation roots group their executions and previous failed attempts.
  The browser retains older loaded pages while refreshing current status.
- Dismiss/restore changes private history visibility. Discard and cancel-and-clear
  record durable intentions, fence stale upload/review/admission/retry/publication,
  and settle through existing cancellation and staging-cleanup services. Published
  originals and successful evidence remain. Independently owned work is protected.
- Focused store coverage checks advancing-clock pages, filename-only search,
  publication grouping, exact receipt replay, cross-user denial, mixed eligibility,
  dismissed retry fences, restart cleanup and measured reclaimed bytes. An injected
  staging cleanup failure stays visible and succeeds after repair and retry.
  Existing shared-owner and last-owner lifecycle regressions also pass.
- The dedicated real-browser `test:work` journey creates 28 imports, loads older
  pages, refreshes without dropping them, searches, reviews mixed-eligibility
  selections, dismisses/restores history, clears a partially uploaded package,
  observes actual reclaimed bytes, reloads and resumes an older unattached upload.
  It checks keyboard focus, mobile layout, transient status-read recovery and the
  exact embedded assets served by the temporary server.
- Generated tracks remain grouped by meaningful music-track labels with playback
  and preparation. Module readiness suppresses ineligible per-track CD-operation
  forms while preserving original-package blockers and historical observations.

Initial full-suite runs exposed two schema-version assertions, now updated to 15
while retaining their data-preservation checks. A process-interruption checkpoint
wait and a local provider HTTP fixture timed out under concurrent load; the isolated
process-interruption rerun passed, and the provider fixture passed in the reduced-
concurrency full-suite rerun. The smoke test also needed to await the lazily loaded
history dialog before clearing its item filter and wait for the reopened processing
panel before inspecting its reduced-motion style. No production readiness checks or
test timeouts were weakened.

Architecture/documentation/fixture checks, formatting, generated API comparison,
TypeScript, web build, embedded-server rebuild and strict workspace Clippy passed.
The standard HTTPS browser smoke passed in Chrome 153.0.8010.37, including actual
FLAC start/pause/resume/seek, next/previous, natural ended advance, clean reload,
grant revocation, WebAuthn and mobile checks. The final dedicated FX-03 browser rerun also passed with exact served-asset
comparison. The FX-02 scan browser journey passed again, including the real 64 MiB
scan, concurrent admission, reopen, repeat consolidation, unrelated editing and
invalid-input/no-job checks. `cargo test --workspace -- --test-threads=4` passed:
313 tests, zero failures, one intentionally ignored owned-corpus oracle. All 157
storage/archive tests passed together, including the process-interruption test.
All browser and cleanup fixtures use temporary archives/profiles; the user's running
server and archive were not reset. FX-04–FX-07 and the LA3 second-device/external
qualification remain open; this does not close the broader U05/P04 completion gate.

## FX-02 scan lifecycle — 2026-09-21

Local acceptance passed. Implementation and final browser recovery checks are complete. The prior
alpha/FX-01 checkpoint is commit `20c4ff0`.

- Four concurrent commands with independent receipt keys converge on one active
  computation. Cancelling work still occupies its slot; generic retry joins an
  already active replacement. Readiness/admission work from recorded metadata even
  after a temporary fixture's payload is removed; actual execution fails safely.
- An advancing-clock recheck preserves one substantive result and adds a compact
  occurrence. Receipt replay does not advance the date. The occurrence stores its
  result reference, actual check time and acquisition/rebinding provenance, not
  another copy of the common inputs or result payload. Archive-only reconstruction
  preserves occurrences and rejects contradictory references.
- Separate AccurateRip snapshot IDs with identical bytes and parsed observations
  share a result while preserving acquisition provenance. A wider evaluated offset
  range remains a distinct result, as do no-data/changed-reference outcomes. Existing
  independent v2-only/partial-coverage module vectors remain in the workspace suite.
- A manual request admitted after guided work shares the computation. Parent
  cancellation preserves its source revision; manual retry and expired-lease
  recovery after restart still complete without resuming the cancelled parent.
  Cancelled album preparation retains its original retry/admission fence.
- Module planning selects a sole eligible CUE/TOC even when it is not file zero,
  rejects ambiguous/non-anchor/missing-companion selections and bounds settings.
  HTTP rejects known-invalid processing inputs before job creation.
- `npm --prefix web run test:scans` uses a real temporary server and a 64 MiB
  preserved package: concurrent admission, close/reopen, reload, completed View
  result, intentional recheck consolidation, editable unrelated controls and
  invalid-input/no-job behavior and transient status-read retry. No mocked successful processing service is used.
- Both standard HTTPS and `LOCAL_SETUP=1` smoke journeys passed in Chrome
  153.0.8010.37, including actual FLAC playback, detached-player cleanup, reload,
  role checks, access revocation, imports and provider work.

The initial workspace run exposed route-count/fixture updates and a cancellation
fence regression. The route inventory now includes scan planning, generic paging
fixtures no longer impersonate source-less integrity jobs, and only a durable
independent analysis owner can bypass a cancelled parent's workflow fence. The
focused shared-analysis and cancelled-album tests passed after correction.

Check record: `cargo test --workspace` passed (312 tests, zero failures; the
explicit owned-corpus oracle remains ignored). The final focused lifecycle and
readiness suites, strict workspace Clippy, formatting, architecture/link checks,
generated API comparison, TypeScript, web build and embedded-server rebuild passed.
The final standalone browser scan journey passed on the rebuilt server, including
the transient-status recovery assertion and byte-for-byte served-asset checks.

FX-03
history cleanup and FX-05 remove/reimport remain separate deliveries. Broader
U05/P04 and external LA3 qualification remain open.

## FX-01 browsing/navigation — 2026-09-21

Local acceptance passed. This delivery adds a typed, bounded browse query and
catalog 7 projection, URL/history navigation, responsive detail focus, fenced
asynchronous reads and explicit retry states. It does not change scan admission,
retention, item removal or reimport policy.

- Store regression fixture: 105 entries, 17-entry keyset pages, unique coverage,
  effective-title override, artist search, wishlist filtering, creation order
  after edits/reindex, requester-private attention and denied collection access.
- A committed WAV fixture remains browseable after its payload is removed from
  the temporary archive: filters read recorded metadata rather than hashing or
  opening media. Prepared-track filtering is exercised through real derivative
  publication in the existing archive integration test.
- `npm --prefix web run test:browse` uses a temporary real server and 62 entries.
  It covers second-page selection, list scroll/focus restoration, Back/Forward,
  reload, inventory/artwork edits, collection/search changes, sort/filter controls,
  a single cursor request for Load more (no refetch of earlier pages),
  390px keyboard navigation, delayed evidence, artwork/evidence/list retry,
  missing-item reads and denied-detail recovery. Controlled transport faults are
  used for error/race cases, not successful-looking substitutes for app services.
- Reviewed [mobile](reviews/ux-2026-09-20/fx01/mobile-detail.png) and
  [desktop](reviews/ux-2026-09-20/fx01/desktop-detail.png) captures retain artwork beside text. Same-item refresh
  keeps details visible; failed authorized reads clear details. The test checks
  that served assets exactly match the built bundle.

Check record: architecture/link checks, formatting, strict workspace Clippy,
TypeScript, generated API comparison, web build and embedded-server rebuild passed.
The workspace run reported 303 passed, one expected-schema assertion failure and
one explicitly ignored owned-corpus oracle. The catalog assertion was corrected
from 6 to 7 (the archive-byte preservation assertions remain unchanged) and its
focused rerun passed. A second overlapping workspace attempt hit the existing
five-second encoder-probe timeout; the complete provider unit suite subsequently
passed alone (7 tests). No timeout threshold or production encoder code changed.
The final FX-01 Chrome 153.0.8010.37 journey passed. Existing smoke tests now
explicitly clear an obsolete search and expect selection to survive reload;
the missing-module notice assertion is scoped to the notice instead of matching
both it and immediately restored rows. Workspace doctests, HTTPS smoke, LOCAL_SETUP=1 smoke and the final
incremental-pagination browser run passed. Both smoke modes exercised actual
FLAC start, pause/resume, seek, track changes, natural ended advance, clean reload
and grant revocation. The final browser run also checks the rebuilt served assets
byte-for-byte and bounds next-frame keyboard focus restoration to one second.

Remaining external gates are unchanged: physical second-device LA3 qualification,
independent accessibility/security, 100k scale and other deployment qualification.
Live keyset pages are stable for unchanged data, not a frozen snapshot during
concurrent edits. FX-02–FX-07 and the broader U05/P04 completion gates remain open.

## D04 root review corrections — 2026-09-13

See [review findings and next checkpoint](wave-4-review.md). Review fixes cover
exact snapshot length/SHA-256 checks, strict shared DAT parsing, incomplete-checksum
outcomes, cancellation during reads/scans, missing-module historical reads, and
curation/admin usability. No archive schema change was added during review.

- `cargo test --workspace --locked`: 216 tests passed; focused module tests were
  rerun after the final incomplete-checksum regression and passed.
- Architecture checker, formatting, strict workspace Clippy, TypeScript check,
  generated API check, web build, and embedded server rebuild passed.
- Shared `junk-libs-dat` tests: 3 passed; strict Clippy passed. Affected Retro Junk
  DAT/catalog tests: 104 passed including doctests. The shared parser changes are
  checkpointed in Junk Libs commit `6350026e95dd0715ff6cdfddffede91d1f666c1c`.
  Collector still consumes the sibling through a local path dependency.
- Normal and `LOCAL_SETUP=1` Chromium smoke passed against rebuilt servers, including newly added
  real synthetic DAT import/activation through administration, plus the existing
  inventory, artwork/search, exposure, authorization and WebAuthn checks.

Large DAT corpus/index performance and owned real-dump qualification remain
external. Parsing regenerates a bounded in-memory row view per job, not a persistent
indexed lookup. No scale qualification is claimed.


## D04.2–D04.3 local implementation — 2026-09-13

- Added `collector-providers` at the reserved infrastructure boundary. Its local
  DAT adapter accepts only configured source names, parses bounded Logiqx XML or
  ClrMamePro text through `junk-libs-dat`, retains exact source bytes and SHA-256,
  and regenerates bounded typed catalog rows from the archived snapshot. Empty,
  malformed, oversized, cancelled, missing and changed inputs fail without a
  synthetic successful snapshot.
- Game matching requires the declared system, normalized byte-domain name/version,
  expected lengths and every declared checksum for every ordered file or track.
  Results distinguish no match, one complete match, multiple complete matches,
  partial identification and inconclusive input. Matched units retain the actual
  SHA-1, MD5 or CRC32 algorithm; CRC-only candidates are explicitly described as
  non-cryptographic identity.
- Private schema 11 stores only the active collection/provider snapshot selection.
  Exact DAT bytes and parsed observations are archive-schema-4 Snapshot records;
  the rebuildable catalog index is regenerated from those bytes. Activation uses
  the existing transactional command receipt path and does not rewrite snapshots.
- Registered raw/profile hash, iNES structure, raw-2352 CUE-track and DAT comparison
  operations run through the existing frozen Job executor. Import analysis records
  exact hashes and optionally uses the active snapshot; committed Capture operations
  freeze their Representation source fingerprint and SnapshotId. The web surfaces
  expose configured-source names to administrators, explicit byte domains/systems
  to curators, and current/historical result language without a verified badge.
- The synthetic end-to-end test imports and activates a DAT, analyzes and publishes
  a game package, commits an exact catalog-match observation, imports a changed DAT,
  activates it, and rebuilds the catalog. The original snapshot bytes, digest,
  SnapshotId and exact-match history remain readable while the new snapshot stays
  active. This also exposed and fixed attached Import publication so the schema-3
  Item revision references newly published standalone integrity Evidence.
- Focused provider, module, store and HTTP tests cover equivalent XML/text bodies,
  duplicate and CRC-only rows, conflicting checksums, wrong system/domain, missing
  checksums, reordered/modified files, cancellation/outage/size failures, listener
  composition, role-aware snapshot access, refresh and reindex.
- Passed: `python3 scripts/check_architecture.py`; `cargo fmt --all -- --check`;
  214 tests under `cargo test --workspace --locked`; warning-free
  `cargo clippy --workspace --all-targets --locked -- -D warnings`; TypeScript and
  generated-contract checks; web build; rebuilt embedded `collector-server`; and
  the Chromium smoke suite covering session, management bundles, inventory,
  role-aware artwork, typed collections, search, denial, WebAuthn and mobile layout.
- **External/unavailable:** real copyrighted dumps and large independent DAT corpora,
  real mixed-mode CUEs, target Linux/NAS behavior, and runtime CHD operation exposure
  remain external. The pinned synthetic CHD primitive is not advertised by the D04.3
  runtime. D05 has not started.

## D04.1 repository checkpoint

The reviewed shared extraction is committed in Junk Libs at `cd385d8`; the
associated Retro Junk consumer migration is committed at `f4af7b0`. This Collector
checkpoint contains the matching integration, fixtures and handoff. These hashes
identify the tested sibling state; Cargo still uses local path dependencies, not
distributable immutable pins. Unrelated Retro sync scripts were excluded.

## D04.1 review corrections — 2026-09-13

- N64 reads are bounded to the initial stream length and reject aligned early EOF.
  The range hasher has one shared implementation rather than duplicate loops.
- Qualified CUE hashing preflights BINARY container type, unique logical FILE names,
  exactly one INDEX 01, valid track numbers, ordered indexes and membership in each
  track's source-byte span. Non-binary containers, alias/repeated FILE sources and
  invalid layouts fail before opening payload readers. Generic CUE parser behavior
  is unchanged; these are the qualified hash adapter's fail-closed rules.
- Collector rejects overlapping or overflowing track spans while permitting the
  valid exact raw hash of an empty file. Permanent tests cover these cases.
- The CHD oracle is explicitly ignored in ordinary test runs. Run
  `CHDMAN=/opt/homebrew/bin/chdman cargo test -p junk-libs-game-formats --features chd --test chd_oracle -- --ignored`
  in Junk Libs to qualify it. An explicit missing-tool or wrong-version invocation
  now fails instead of passing silently. The pinned 0.285 synthetic oracle passes.
- Passed: 204 Collector workspace tests, 10 shared game-format vectors, the explicit
  CHD oracle, and 431 affected Retro package tests. Default/all-feature shared vectors,
  all-feature shared-workspace Clippy, affected Retro Clippy and Collector gates pass. No browser rerun was
  needed because D04.1 changes no HTTP routes or frontend behavior; TypeScript,
  generated API drift, web build and embedded-server rebuild were checked.
- Shared-library and Collector changes remain uncommitted in their own repositories.
  Unrelated Retro sync-script edits were preserved. The new `chd` feature gates the
  hash API, but the existing shared disc dependency still includes its decoder;
  no decoder-free build or real mixed-mode-dump qualification is claimed.
- D04.2 then D04.3 may proceed under the updated handoff. Do not map CHD internal
  frame positions into CUE source-byte offsets or infer complete DAT verification
  from normalized hashes alone. Real dumps, platform gates and immutable dependency
  pins remain outstanding.

# D04.1 qualified reader-based game primitives — 2026-09-13

- Added the reader-only `junk-libs-game-formats` crate. It owns versioned
  normalization domains plus exact raw, trainer-free iNES/NES2 payload, canonical
  big-endian N64, explicit SNES/PCE payload, ordered raw-2352 CUE-track, and
  feature-gated CHD-track hashing. It has no filesystem paths, provider policy,
  database, GUI, or Retro orchestration dependency.
- Collector's game module maps shared hashes and CUE source spans into typed
  observations, validates exact domain/version/digest sets, resolves CUE members
  only through the frozen `PackageInventory`, and keeps header structure separate
  from any future catalog match. No production capability was registered, no Job
  executor or persistence path changed, and D04.2 DAT work did not start.
- Retro Junk's actual NES, SNES, N64, PCE, CUE, and CHD hash consumers now call
  the shared primitives. N64 hashing uses the whole-stream hook so arbitrary
  three-byte reader chunks cannot split a normalization word. Existing cooked-CUE
  behavior remains an explicitly unqualified Retro fallback rather than being
  mislabeled as the qualified raw-2352 contract.
- iNES/NES2 length and trainer rules were checked against the
  [NESdev NES 2.0 format reference](https://www.nesdev.org/wiki/NES_2.0); SNES
  explicit whole-bank/copier-header handling was checked against the
  [SNESdev ROM file-format reference](https://snes.nesdev.org/wiki/ROM_file_formats).
  PCE's explicit bank layout is cross-checked with the migrated Retro consumer;
  real PCE/SNES dumps remain external qualification.
- The pinned `/opt/homebrew/bin/chdman` 0.285 oracle creates a two-track CHD from
  three raw-2352 frames of `0x11` followed by five frames of `0x22`. Shared decode
  returns every unpadded track and matches independent SHA-256 values
  `e75a34b718be8826bd01312d55980c3c6d898e346ab4bf2fda123791737fa12f` and
  `e8d9b3b43740b26ce576e9b8922fa5264c3ccc2a73585a97c985b1b05ca687fb`.
- Explicit unavailable/unqualified cases are trainer-bearing NES, NES2 misc ROMs,
  malformed/trailing/truncated iNES lengths, unaligned or unknown N64 input,
  ambiguous SNES/PCE layouts, cooked/missing/truncated CUE members, unsupported
  CHD units, other consoles/containers, real mixed-mode dumps, and other chdman
  versions. Inputs are never rewritten; failures do not become catalog
  verification, and preservation remains available.
- Independent and cross-consumer tests cover the published raw digest vector,
  exact iNES and NES2 exponent sizes, hostile N64 chunk boundaries, explicit
  SNES/PCE headers, combined and split CUE sources, stored versus synthetic
  pregap, changed later tracks, malformed/incomplete layouts, and the pinned CHD
  round trip. `cargo test -p junk-libs-game-formats` and the feature-enabled CHD
  oracle pass; affected Retro suites pass 147 + 237 + 5 tests. Full repository
  gates pass: architecture/link and all Wave 4 fixture checks; formatting; locked
  workspace tests; warning-free full Collector Clippy; TypeScript and OpenAPI
  drift checks; web build; and a fresh `collector-server` build. Junk Libs passes
  strict all-feature workspace Clippy and the complete all-feature game-format
  suite. The four affected Retro packages pass strict all-target Clippy; the
  broader Retro workspace still has unrelated pre-existing strict-lint failures
  in backend/import code and was not rewritten as part of this extraction.
- The shared and Retro repositories remain separate working-tree changes and the
  path dependency is not a distributable immutable pin. Real private dumps,
  target-platform qualification, published dependency pins, and independent
  review were external at the implementation checkpoint. Local review is recorded above.

# CT.2 persistence and legacy assignment checkpoint — 2026-09-11

- Added durable collection binding to runtime collections, presented DTOs, typed
  creation receipts/recovery and generated contracts. New creation requires a
  registered module; retries preserve the same type/grant and another type under the
  same key conflicts.
- Legacy manifests with no binding are classified without rewrite and are readable
  but content-read-only. Local administrator inspection and explicit assignment use
  application-owned policy over mechanism-only store facts; names, files and provider
  identities are never used to infer ownership.
- Assignment checks all current projected items including tombstones, pending
  imports and pre-commit jobs with known owner identity, current metadata authority,
  and preserved historical override entries. Mixed and unknown collections remain
  unassigned with actionable explanations; empty assignment still requires the
  operator to name the module.
- Assignment publishes a validated provenance-bearing `collection_metadata` revision
  with the existing schema-3 journal and then updates the derived projection. Startup
  recovery/reindex selects it by durable collection identity. Original creation and
  item revision bytes and private grants remain unchanged; corrupt or ambiguous
  sidecars fail closed.
- Synthetic regressions cover game/music/third-module creation, type-bound
  idempotency, every collection-assignment publication fault point, archive-only
  rebuild without grants, directory rename, empty/single/mixed/unknown legacy cases,
  rejected/repeated assignment, unchanged source bytes, and corruption atomicity.
- Passed: `python3 scripts/check_architecture.py`; `cargo fmt --all -- --check`;
  `cargo test --workspace --locked` (187 tests); `cargo clippy --workspace
  --all-targets --locked -- -D warnings`; `npm --prefix web run check`;
  `npm --prefix web run check:api`; `npm --prefix web run build`; and
  `cargo build -p collector-server --locked`.
- Browser smoke was not run because the browser type-selection workflow belongs to
  CT.4 and is intentionally not implemented at this CT.2 review stop. No sibling
  checkout, real user archive/configuration, commit, graph split, coercion, CT.3 or
  CT.4 implementation was touched. CT.2 is awaiting technical review before CT.3.

# CT.1 review corrections — 2026-09-11

- Common metadata deserialization preserves unrecognized historical field entries
  in an explicit bounded raw list. Artist/platform language and lexical values
  survive without reintroducing domain-specific fields into core or applying them
  under an inferred owner. Invalid known title/subtitle entries still fail.
- Added a real archive-only reopen/reindex regression for prior artist/platform
  authority records. Original record and manifest bytes remain unchanged; repeated
  serialization/reindex does not duplicate preserved fields.
- Catalog ambiguity requires at least two distinct provider/record identities;
  repeated candidates and duplicate matched-file IDs are rejected. Permanent tests
  cover invalid cases and a valid two-candidate result.
- These fix both failing review probes. CT.2 must explicitly handle any preserved
  domain authority during owner-aware conversion; no collection assignment is
  performed by this correction.


- Final checks passed: all 181 locked workspace tests, all-target Clippy with
  warnings denied, architecture/link/fixture checks, formatting, diff checks,
  TypeScript and generated API checks, web build and rebuilt server. No browser
  behavior changed; browser smoke was not repeated.
- CT.1 is reviewed and corrected; CT.2 is next. No commit or user-data conversion
  was performed.

# CT.1 ownership and module contract checkpoint — 2026-09-11

- Added validated core binding wire contracts for Assigned module ownership and the
  legacy read/recovery state. Runtime `Collection`, creation, persistence and DTOs
  remain unchanged until CT.2; homogeneous collections are not claimed.
- Built-in and test module descriptors now validate safe collection labels, bounded
  scalar/read-only field descriptions, stable keys and explicit domain/shared-media
  operation scope. The test-only stamp module still registers without a core kind.
- Owner-mediated typed patches preserve omission, distinguish explicit null, retain
  untouched structured/unknown raw values (including lexical numeric form), and
  reject unknown keys, wrong scalar types, wrong owners and schema changes.
- Game result/catalog/normalization contracts moved from core to `modules::games`.
  AccurateRip outcome/confidence/checksum and CD result contracts moved to
  `modules::music`; signed offsets, integer CD/PCM coordinates and shared track
  layout remain in core. Exact serialized result fixtures, source fingerprint and
  recipe digest goldens are unchanged.
- Common metadata authority now contains only title/subtitle plus artwork/candidate
  authority and an optional owner-bound lossless domain envelope. Game platform and
  music artist overrides are typed and owner-validated. Existing authority JSON
  omits the absent new field, preserving its prior wire body.
- Registry dispatch invokes a required module result validator for every successful
  operation before the result can reach application publication. The regression
  suite rejects a success-shaped malformed third-module result; missing handlers
  remain explicitly unavailable rather than successful no-ops.
- Passed: `python3 scripts/check_architecture.py`; `cargo fmt --all -- --check`;
  `cargo test --workspace --locked` (178 tests); `cargo clippy --workspace
  --all-targets --locked -- -D warnings`; `npm --prefix web run check`;
  `npm --prefix web run check:api`; `npm --prefix web run build`; and
  `cargo build -p collector-server --locked`.
- Browser smoke was not run because CT.1 changes no route, DTO, embedded web asset
  or runtime user workflow. No sibling checkout, user archive/configuration, commit,
  or CT.2 behavior was touched. CT.1 is stopped for technical review.

# Wave 4 W4.2 review corrections — 2026-09-11

- Enforced exact authorized package scope on the production module host; fixture
  operations now assert denial for both another collection and another source.
- Fixed SQL cursor pagination beyond the first window, including tied timestamps,
  and reject unknown/out-of-scope cursors. Evidence currency uses server-owned
  committed file records, not a caller-provided fingerprint; browsing does not
  hash payloads. This is source currency, not a new verification result.
- Reject stale provisional digests before Capture commit and verify the explicit
  rebinding against published file digests. Portable observations require a
  Representation source; package file IDs use explicit versioned encoding instead
  of Debug formatting. Package readers reject paths escaping their storage root.
- Validate archive record identities against their envelopes. Base and processing
  projections rebuild in one transaction; a malformed processing record cannot
  leave a partially replaced catalog. Added a real archive corruption regression.
- Spool bounded provider streams outside the archive writer lock, recheck grants
  during acquisition publication, and retain journal/hash validation at commit.
  Recipe publication rejects immutable identity conflicts and missing sources.
  Processing command receipts now bind the Capture path identity as well as input.
- The contracts validate generic result envelopes; operation-specific game/CD
  semantic validators still belong to their future domain handlers. No unavailable
  analyzer/provider qualification is claimed by this review.
- Updated remaining wave plans and release/architecture/backlog guidance with the
  accepted pre-release evolution policy: coordinated breaking changes, explicit
  preservation of actual data, immediate producer/consumer contract reviews, and
  no speculative production placeholders or checkpoint compatibility mandate.
- Two full runs exposed transient lock retention in different existing restart
  tests, while the isolated recovery case passed. Store ownership now explicitly
  unlocks at the last shared handle's drop rather than relying on descriptor close.
  A deterministic duplicate-descriptor regression verifies release while preserving
  exclusion until the final owner drops. This also covers early-open failures.


- Final gates passed: all 173 locked workspace tests, final lock regression,
  architecture/link/fixture checks, formatting, diff checks, warning-free all-target
  Clippy, TypeScript, generated API drift, web build and subsequent server build.
  Browser smoke was not repeated: no browser UI behavior changed. Existing HTTP
  tests include the curation operation route exposure check.
- At the W4.2 checkpoint, D04.1 was next. D04.1 has since implemented its bounded
  primitives and remains uncommitted pending review.

# Wave 4 W4.2 implementation evidence — 2026-09-11

- Added archive schema 4 processing observation, streamed snapshot, metadata
  authority, and conversion-recipe records with catalog schema 5 projections and
  mixed v3/v4 reindex. Existing archive bytes and unknown record directories remain
  untouched; archive-only rebuild restores the portable processing history.
- Added private schema 10 frozen Job inputs and provisional Import observations.
  Existing user databases migrate additively in place; no reset or automatic data
  deletion is performed. Because these are pre-release schemas, no permanent
  reverse-compatibility machinery was added solely for earlier Wave checkpoints.
- Added Import- and Representation-scoped package readers. Service-entry and
  pre-publication checks bind inventories, fingerprints, typed results, metadata,
  recipes, and snapshot metadata. Snapshot bodies are streamed and checked against
  their recorded byte length and SHA-256 before commit.
- Import publication checks the ordered provisional/committed file mapping and
  rebinds observations to the committed Representation fingerprint while retaining
  the provisional Import fingerprint and mapping as provenance. Legacy schema-3
  evidence remains visibly limited rather than being promoted to current proof.
- TDD coverage exercises v3/v4 rebuild and unknown preservation, raw streamed
  snapshot recovery, corrupt snapshot rejection, cancellation followed by restart,
  frozen acquisition recovery without a duplicate provider call, registered
  operation retry/idempotence, revocation before and after commit, archive-only
  Import recovery, invalid result/metadata/recipe rejection, and changed-input
  current-versus-historical summaries with authorization denial.
- Added the authenticated curation operation-enqueue API; generated OpenAPI and
  TypeScript artifacts were regenerated. There is no production provider network
  adapter, game/CD analyzer, metadata search workflow, derivative encoder, or new
  browser UI in W4.2. D04.1 had not started at this W4.2 checkpoint.
- Final checks passed: `python3 scripts/check_architecture.py`; `cargo fmt --all
  -- --check`; `git diff --check`; `cargo test --workspace --locked` (170 tests);
  `cargo clippy --workspace --all-targets --locked -- -D warnings`; `npm --prefix
  web run check`; `npm --prefix web run check:api`; `npm --prefix web run build`;
  and `cargo build -p collector-server --locked`. Browser smoke was not run because
  W4.2 adds no browser UI behavior; the server was rebuilt after generated API/web
  artifacts changed.

# Wave 4 W4.1 review corrections — 2026-09-11

- Tightened package collision checks, fingerprint deserialization/inventory binding,
  typed metadata authority, canonical conversion recipe digests and audio success
  invariants. Recipe SHA-256 is checked against an independently calculated Python
  vector; changed settings and duplicate tracks are rejected.
- Unified stored operation and registry capability identities while retaining the
  existing OpenAPI schema name. The compatibility reader rejects a host returning
  the wrong Representation scope. Structural values retain numeric/Boolean types.
- Separated frozen acquisition inputs from file processing, introduced bounded
  streaming catalog/snapshot readers, and bounded observation pagination.
- Added regression coverage for the above contracts. The initial three targeted
  regressions failed before the fixes. No production provider or storage adapter
  is claimed; W4.2 must enforce validation and authorization at its boundaries.
- Final review gates passed: architecture/link/fixture checks, formatting, all 161
  locked workspace tests, warning-free workspace/all-target clippy, TypeScript
  check, generated API drift check, web build and subsequent server build. Focused
  suites include 13 core, 8 module and 3 app processing contract tests. Browser
  smoke was not repeated because no routes or web behavior changed.
- W4.1 is reviewed and cleared for W4.2; changes remain uncommitted.

# Wave 4 W4.1 initial implementation evidence — 2026-09-11

- Added portable scoped package inventories/file identities, versioned deterministic
  source fingerprints, processing observations/support outcomes, snapshot/recipe
  identities, typed game/CD results with distinct sector/frame/offset units,
  conversion recipes and metadata authority. Lossless module/raw JSON remains the
  extension envelope.
- Replaced module byte access with collection- and Import/Representation-bound
  inventory/per-file handles. Tests deny wrong collection/source/file access,
  reject missing and ambiguous one-file access, and resolve nested logical member
  references without permitting package escape or exposing adapter paths.
- Added source-bound processing, provider and snapshot application ports plus a
  frozen execution input carrying source/snapshot identity, output identity and
  timestamp. No provider network mechanism, analyzer, persistent row, route or
  support qualification was introduced.
- Split the built-in game/music implementations into focused files without changing
  their outputs. A schema-2 third module operates over a two-file package without a
  core enum or built-in-module special case.
- Focused contract tests pass: 6 core, 6 module, and 2 application tests. The core
  golden vector fixes the exact source-fingerprint bytes and SHA-256; changing only
  a second track changes the identity; unknown lexical JSON round-trips unchanged.
- Passed: `python3 scripts/check_architecture.py`; `cargo fmt --all -- --check`;
  `cargo test --workspace --locked` (151 tests); `cargo clippy --workspace
  --all-targets -- -D warnings`; `npm --prefix web run check`; `npm --prefix web
  run check:api`; `npm --prefix web run build`; and `cargo build -p
  collector-server --locked`. Browser smoke is not applicable because W4.1 changes
  no web asset, route, or browser behavior.
- Initial implementation stopped for review; see corrections above. W4.2 is not started.

# Wave 4 W4.0 baseline and qualification lock-in — 2026-09-11

- Re-read Collector architecture/status/release/backlog and every sibling
  `AGENTS.md` before inspection. Collector remained at `c5c3ecf`; Junk Libs,
  Retro, Phono and Trash Amp were inspected at the exact revisions and dirty states
  recorded in the dated reuse map, without changing a sibling checkout.
- Verified the D02/D03 public API boundary against source. The existing whole-disc
  `DiscTrackSamples`/prefix allocation and missing generic preservation PCM host
  contract remain explicit D05.1 work rather than being treated as complete.
- Added eight per-root Wave 4 fixture manifests. A temporary checker parsed all
  manifests and reproduced 22 inline/formula/composite source vector digests from
  the declared bytes. Private dump, Keychain, security, LAN/mDNS and tailnet gates
  remain external and unclaimed.
- Passed: `python3 scripts/check_architecture.py`; `cargo fmt --all -- --check`;
  `cargo test --workspace --locked` (142 tests); `cargo clippy --workspace
  --all-targets -- -D warnings`; `npm --prefix web run check`; `npm --prefix web
  run check:api`; and `npm --prefix web run build`.
- Browser smoke and a Rust server rebuild were not run because W4.0 changes only
  documentation and fixture manifests. No runtime Wave 4 capability or support
  qualification is claimed. W4.1 is the next child after review.

## W4.0 review corrections — 2026-09-11

- Independently confirmed all four sibling HEAD/dirty states, selected Collector/
  Junk Libs lockfile versions, FLAC 1.5.0, chdman 0.285, and the published TOC/ID
  constants against the existing disc-ID test. No sibling files were changed.
- Added a permanent bounded, non-evaluating fixture recipe checker and wired it
  into the architecture check. All eight manifests and 22 byte/composite digests
  reproduce, including declared SHA-1/CRC32 values. Temporary corrupted copies
  prove rejection of bad digest, length, unknown recipe and missing root manifest.
- Corrected D06's search-score/identity ambiguity with separate identity inputs;
  marked S04's deliberately incomplete transition as a negative parser fixture.
  Clarified planned versus implemented architecture and seed versus future fixtures.
- Passed `python3 scripts/check_wave4_fixtures.py`,
  `python3 scripts/check_architecture.py` and `git diff --check`. This review did
  not rerun the unchanged Rust/browser suites; the baseline run above remains the
  implementing agent's recorded evidence. W4.0 is cleared; W4.1 is the next child.

# Wave 3 integration review — 2026-09-10

- The permanent root-by-root acceptance trace is
  [the Wave 3 integration review](wave-3-review.md). Collector's architecture and
  API contract checks, literal workspace fmt, all 139 Rust tests, warning-free
  full clippy, TypeScript check/build, and rebuilt embedded server pass.
- Normal HTTPS and `LOCAL_SETUP=1` private Chromium suites pass. Focused final
  probes pass for three SIGKILL publication gaps, six cancellation boundaries,
  reindex/restart, durable-claim takeover, Import listener/security composition,
  Asset role filtering, and Job scoping/secret redaction.
- Junk Libs whole-workspace formatting, all-feature tests, and strict clippy pass;
  `91dd63d` is the formatting-only cleanup of previously detected unrelated
  workspace drift. Phono full tests/clippy pass. Retro full tests and focused
  changed-package clippy pass; its known unrelated whole-workspace warnings remain
  recorded rather than claimed green. Trash Amp fmt, all 42 executable tests, and
  strict clippy pass with one hardware-oriented test ignored.
- Sibling state was preserved: Retro's untracked sync scripts, Phono's untracked
  `AGENTS.md`, and Trash Amp's user-owned core/presentation/docs/browse/lock hunks
  remain outside Wave 3 commits.
- **Implemented:** P02, U03, U04, P03, P04, D02, D03, and the local W3.R review.
  **Not implemented:** D04/D05, D06, D07, C06, document uploads, runtime plugins,
  distributed scheduling, and Wave 4. **Externally unqualified:** target Linux/NAS,
  abrupt power loss, private fixtures/codecs, live providers/drives, hardware
  playback, published shared pins, and independent security/accessibility review.

# Wave 3 D03.5 cross-repository review — 2026-09-10

- Junk Libs reran `cargo test --workspace --all-features` and
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`. The
  dependency review found no path, HTTP, product-ID, GUI, or CPAL use in production
  audio/playback sources; the only filesystem matches are CUE test-fixture writes.
  The default `junk-libs-audio` tree was narrowed to `thiserror` only in `4c7e98e`.
- Trash Amp reran all 42 workspace tests and full strict clippy in D03.4. Its
  adapter lockfile followed the dependency cleanup in `cb95c57`; the two user-owned
  lock hunks and all other user changes remain unstaged.
- Collector explicit-package fmt passes. `cargo test --workspace` exits zero with
  139 tests passing, and `cargo clippy --workspace --all-targets -- -D warnings`
  exits zero. Architecture and diff checks pass.
- The literal whole-workspace fmt issue found here was resolved during W3.R by the
  formatting-only Junk Libs commit `91dd63d`; no behavior or dependency changed.
- **Implemented:** the D03 headless contracts, qualified WAV/CUE/redumper decode,
  exact frame seeking, bounded resampling, gain/clipping, queue policy, and local
  consumer migrations. **Not implemented:** shared compressed codecs, richer DSP,
  events, or device policy. **Externally unqualified:** published Git pins,
  physical CPAL/loopback, private real dumps/codecs, target platforms, and
  independent review.

# Wave 3 D03.4 consumer migration — 2026-09-10

- Junk Libs `c3c133b` adds validated queue restoration so a device adapter can
  reconstruct current audible state from atomics without putting locks in the
  callback. Its queue tests/clippy pass.
- Trash Amp `3f5b400` changes only the clean `amp-audio` crate, root dependency
  declaration, owned `Cargo.lock` hunks, and a new dedicated validation document.
  User browsing/UI/docs changes and their two lock hunks remain unstaged. WAV,
  CUE/redumper, resampling, and transport commands use shared services; existing
  compressed decoding and CPAL remain product adapters.
- Trash Amp passed all 42 workspace tests (one live MusicBrainz test ignored) and
  `cargo clippy --workspace --all-targets -- -D warnings`. Its focused audio crate
  format/test/clippy gates pass. Whole-workspace fmt still traverses and reports
  recorded unrelated Junk Libs formatting drift outside the changed shared crates.
- Collector's new test proves an authorized media resource retains length,
  extension, media type, read, and seek through `ByteSource` while exposing no
  filesystem path. Focused collector-app test/clippy and architecture checks pass.
- **Implemented:** local consumer migration for the qualified shared services.
  **Not implemented:** shared compressed codecs or D03.5 final review. **Externally
  unqualified:** published Git pins, physical CPAL/loopback, real dumps, private
  codec vectors, target platforms, and independent review.

# Wave 3 D03.3 resampling, queue, and DSP — 2026-09-10

- TDD first failed on absent resampling/DSP and queue APIs. The resampling vector
  uses a generated stereo impulse at 24 kHz, streams it through a bounded 2x sinc
  service, checks channel symmetry, exact 400-frame output, contiguous chunk
  coordinates, and impulse timing within one output frame. It also rejects a
  discontinuous source chunk.
- The initial delay-compensation assumption moved the impulse to frame 73 instead
  of 200; the vector caught it. The corrected service retains timeline alignment
  and truncates only to the mathematically rounded final length.
- Gain and hard-clipping vectors check exact samples, peak/count reporting, and
  reject NaN/zero-invalid parameters. Queue vectors cover replace, seek, stale
  generation, next/previous, truncate, failed-command atomicity, exhaustion, and
  append-resume behavior with opaque string/u64 keys.
- Commit: Junk Libs `8f139fd`. Passed focused package tests/clippy and complete
  `cargo test --workspace --all-features` plus
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- **Implemented:** bounded resampling, gain/clipping primitives, deterministic
  queue transitions, and generation-safe integer-frame seek policy. **Not
  implemented:** Trash Amp/Collector consumption, compressed codecs, output-device
  policy, or broader effects. **Externally unqualified:** hardware playback and
  independent review.

# Wave 3 D03.2 decode and exact seeking — 2026-09-10

- TDD first failed on the absent `WavDecoder`. Generated PCM-WAV vectors now
  assert format discovery, bounded reads, exact frame 17 and EOF seeking, sample
  values, and explicit rejection of non-WAV hints, zero-sized reads, and
  out-of-range positions.
- Feature-gated CUE/redumper tests assert a two-track boundary and an exact seek
  within track two, plus in-memory raw playback with retained per-block quality.
  Shared adapters accept opened readers; they expose no path constructor.
- Commit: Junk Libs `28317d9`. Passed focused audio tests and clippy, followed by
  `cargo test --workspace --all-features` and
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- **Implemented:** qualified PCM-WAV decode and exact integer-frame seeking plus
  opened-reader CUE/redumper playback adapters. **Not implemented:** compressed
  codecs, resampling, queue transitions, DSP, or either consumer migration.
  **Externally unqualified:** compressed/private audio vectors, physical media,
  hardware playback, and independent review.

# Wave 3 D03.1 headless audio contracts — 2026-09-10

- TDD first failed because the two new crates exported none of the requested
  contracts. Green tests read and seek an opaque in-memory source, retain bounded
  media hints, reject zero PCM dimensions/incomplete interleaved frames, preserve
  integer chunk starts, carry a consumer-defined key without interpreting it, and
  reject track-local positions beyond the declared length.
- Commit: Junk Libs `714b49e`. Passed `cargo test --workspace --all-features`,
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`, focused
  package fmt, and `git diff --check`.
- The Trash Amp read-only overlap audit found `amp-audio` clean. User-owned changes
  remain in `amp-core`, desktop source/Slint, README/architecture/validation, and
  `Cargo.lock`; none was modified by this child.
- **Implemented:** byte-source, PCM-shape, frame-coordinate, opaque-key, and
  generation contracts. **Not implemented:** D03.2 decoding/exact seek, D03.3
  resampling/queue/DSP, D03.4 consumer migration, or D03.5 review. No dormant
  service method was added. **Externally unqualified:** independent review and all
  codec/device/hardware behavior.

# Wave 3 D02.5 neutral DAT parser and root review — 2026-09-10

- TDD first failed on the absent `junk_libs_dat::parse_dat` API. The shared
  parser now auto-detects Logiqx XML versus ClrMamePro and accepts only a
  `BufRead`; it contains no path, download, cache, database, matcher, platform,
  or application orchestration.
- Permanent shared probes compare equivalent XML/text records, XML entity
  decoding, lowercase checksum normalization, game-level serial propagation,
  and malformed sizes. Retro's 59 DAT tests add complete Redump/LibRetro record,
  multi-ROM, quoted-name, optional field, matcher, naming, and raw-loader-boundary
  regression coverage through the shared parser.
- Commits: Junk Libs `3643b9a`; Retro Junk `d36bc9a`. Retro's pre-existing
  untracked sync documentation/scripts were not staged.
- Passed: Junk Libs `cargo test --workspace --all-features` and
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`; Retro
  `cargo test --workspace`, `cargo test -p retro-junk-dat`, focused package fmt,
  `cargo clippy -p retro-junk-dat --all-targets -- -D warnings`; Collector
  `python3 scripts/check_architecture.py`.
- The full Retro clippy probe still reports pre-existing warnings in untouched
  `retro-junk-backend` and `retro-junk-import`; none originate in the migrated DAT
  package. Cross-path all-workspace fmt still sees unrelated Junk Libs document
  formatting drift recorded under D02.2.
- **Implemented:** neutral DAT parser, real Retro consumer migration, and D02 root
  review. **Not implemented:** Collector DAT/catalog use (D04), music integration
  (D05), or any provider/downloader in Junk Libs. **Externally unqualified:**
  published Git pins, independent corpus review, and private/live-disc fixtures.

# Wave 3 D02.4 shared AccurateRip extraction — 2026-09-10

- TDD first failed because `junk-libs-accuraterip` exposed none of the required
  CRC, dBAR, or offset-search APIs. The green crate operates only on packed integer
  CD PCM and in-memory dBAR bytes and has no filesystem, HTTP, catalog, GUI, or
  audio-device dependency.
- The shared 42-test suite covers CRC v1/v2 overflow behavior, inclusive 2940-sample
  skip boundaries, single-track double skips, exact sample counts, stacked/truncated
  dBAR records, request-header mismatch, primary-v1/v2/both matching, zero-primary
  checksums, frame-450 non-verification, no-data/mismatch/ambiguous outcomes, and
  cross-track offset search at -2939, -6, 0, +6, and +2939 samples.
- Commits: Junk Libs `da85b92`; Phono Junk `f459c51`. Phono's pre-existing
  untracked `AGENTS.md` was not staged.
- Passed: Junk Libs `cargo test --workspace --all-features` and
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`; Phono
  `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`,
  focused formatting, and `git diff --check`.
- **Implemented:** shared CRC/dBAR/matching/offset algorithms and the real Phono
  migration; HTTP/rate limiting remains correctly product-owned. **Not implemented:**
  D02.5 neutral DAT parsing and Collector/D05 integration. **Externally unqualified:**
  the environment-supplied Redumper fixture plus live AccurateRip, physical-drive
  behavior, publishing/pinning the shared commits, and independent review.

# Wave 3 D02.3 shared disc identity extraction — 2026-09-10

- TDD first failed on the absent `junk_libs_disc_id::calculate_disc_ids` API.
  The green implementation validates track ranges, offset count/order, and lead-out
  before calculating MusicBrainz, FreeDB/CDDB, and AccurateRip disc IDs.
- Independent permanent vectors cover ARver's three-track TOC, MusicBrainz's
  published example, and libdiscid's 22-track case. Phono's existing one-, three-,
  four-, six-, and 22-track vectors and CUE-to-TOC integration remain byte-identical.
- Commits: Junk Libs `92d3bf2`; Phono Junk `1a6e19c`. Phono's pre-existing
  untracked `AGENTS.md` was not staged.
- Passed: Junk Libs `cargo test --workspace --all-features` and
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`; Phono
  `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`,
  focused changed-package fmt, and `git diff --check`.
- Existing qualification: all-workspace Phono fmt traverses unrelated, pre-existing
  Junk Libs document-crate formatting drift through sibling path dependencies.
  The locally committed Junk Libs SHA is not published, so Phono intentionally uses
  sibling paths and documents the required release-time Git pin rather than naming
  an unresolvable revision.
- **Implemented:** checked neutral TOC and disc-ID algorithms plus the real Phono
  consumer migration. **Not implemented:** D02.4 AccurateRip CRC/dBAR/offset
  extraction, D02.5 neutral DAT parsing, and Collector integration.
  **Externally unqualified:** live optical-drive/private-disc results, publishing
  and pinning the shared commit, and independent review.

# Wave 3 D02.2 generic format/hash consolidation — 2026-09-10

- TDD red probes first failed for the absent shared CUE-span and CHD-track visitor
  APIs. `junk-libs-core` now passes the published `123456789` CRC32/MD5/SHA-1
  vector across chunk boundaries.
- Shared reader fixtures cover an in-file `INDEX 00`, synthetic `PREGAP` and
  `POSTGAP`, combined and multi-file CUEs, 2352-byte raw and 2048-byte cooked
  frames, duplicate/unaligned/traversal rejection, and every true frame in a
  two-track CHD while excluding four-frame alignment padding and 96-byte
  subchannel tails. The CHD was generated with `chdman` 0.285 and compared with
  both source tracks.
- Retro's consumer regression proves altering only the non-primary track preserves
  the primary digest but changes the complete ordered set. Its old `MultiHasher`,
  hash/error/reader types, and ordered-CHD hasher were removed; the complete CUE/
  CHD path consumes Junk Libs while platform/catalog policy stays in Retro.
- Commits: Junk Libs `dadc40d`; Retro Junk `5f2bc45`. Retro's pre-existing
  untracked sync documentation/scripts were not staged.
- Passed: Junk Libs `cargo test --workspace --all-features` and
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`; focused
  package fmt; Retro `cargo test --workspace`, focused package fmt, and
  `cargo clippy -p retro-junk-core -p retro-junk-disc -p retro-junk-lib
  --all-targets -- -D warnings`.
- Existing gate findings: all-workspace formatting traverses unrelated pre-existing
  junk-libs document-crate drift; all-workspace Retro clippy reports pre-existing
  warnings in untouched backend/import code. Neither was modified to manufacture a
  green child.
- **Implemented:** reusable generic hashing plus complete CUE/CHD byte-reader
  contracts and Retro consumer migration. **Not implemented:** D02.3 disc identity,
  D02.4 AccurateRip, D02.5 neutral DAT parsing, and Collector integration.
  **Externally unqualified:** private real-dump coverage; synthetic CHD/CUE results
  are not a real-disc preservation qualification.

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
- **Resolved by D02/D03 locally:** the reviewed CUE/CHD, disc identity,
  AccurateRip, DAT, audio-reader, resampling, and queue seams now use narrow Junk
  Libs crates in their migrated consumers. Publishing the shared commits and
  replacing review-workspace sibling paths with immutable pins remain external
  release work; Trash Amp's unrelated dirty changes remain user-owned.
- **Open, assigned to D02/D04:** Retro Junk's MIT file retains placeholder
  attribution. Its Track-1/largest-track hash helpers are identification only and
  must not be promoted to complete verification.
- **Open, assigned to D04:** Retro Junk's NES DAT normalization skips the 16-byte
  iNES header but does not separately skip a trainer advertised in that header;
  trainer-bearing images remain unqualified.
- **Resolved by D02.4 locally:** Phono's offset-search behavior and documentation
  now consume the shared implementation. Live/private qualification must cite the
  code and independent vectors; the owned-fixture gate remains external.
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

## Wave 3 review fixes — 2026-09-11

- Added permanent archive regressions for cancellation interrupted before worker
  acknowledgment, final-chunk replay after staging and publication, conflicting
  replay rejection, and recovery after Capture commit plus a later Item edit.
  Assertions cover retained scratch, terminal Import/Job state, exactly one
  Evidence record, and preservation of the later inventory change.
- Expanded real Chromium smoke: interrupt both attempts of an upload chunk,
  reload the browser, reject a different same-name/same-size source without
  appending bytes, resume the original Import, and lose the final response after
  the server accepts the bytes. Saved physical review choices also survive
  reopening the review form after refresh.
- Passed: architecture/document/fixture checks, formatting, workspace Rust tests,
  strict all-target Clippy, TypeScript checks, generated API drift checks, web
  build, rebuilt server, and normal HTTPS/WebAuthn plus LOCAL_SETUP=1 smoke.
- Browser runs used temporary server state and isolated Chrome profiles with
  approved sandbox escalation. The first new interruption test let the client's
  automatic retry succeed; the test now interrupts both attempts. No user archive
  or running installation was opened or migrated.
- Resume compares retained prefixes using bounded read-only chunk replay. This
  avoids mixing changed source files but retransfers the retained bytes for
  comparison. Target-Linux/NAS/power-loss and other external gates remain open.

## CT.2 review corrections — 2026-09-11

- Assignment validates complete active metadata authority through module validators,
  not just owner IDs. Invalid typed payloads/schema envelopes block assignment;
  preserved historical raw overrides remain inactive and unchanged.
- Pending-work inspection distinguishes shared-media capabilities from domain
  operations, blocks unknown active jobs, and uses explicitly selected import
  candidates. Cancelled jobs cease blocking. Same-owner retries require the original
  expected revision. Policy is isolated in the application's collection-types module;
  storage supplies facts and journaled publication.
- Restored browser collection creation with an explicit registry-driven type picker.
  Updated the former mixed-owner smoke fixture to assert rejection and no published
  item. Unavailable-owner browser fallback now needs a preserved archive fixture,
  explicitly assigned to CT.4; the old successful-looking coverage claim was removed.
- Passed: 190 locked workspace tests, strict all-target Clippy, formatting,
  architecture/document/fixture checks, generated API drift check, TypeScript,
  web build and embedded-assets server rebuild. Normal HTTPS/virtual-WebAuthn and
  LOCAL_SETUP=1 browser smoke both pass with temporary server state/profiles.
  Chrome required approved sandbox escalation; the sandbox-only launch aborted.
  Initial browser attempts exposed an exact-label locator mismatch and the obsolete
  mixed-owner fixture; both were corrected before the successful runs.
- CLI inspection/assignment require exclusive stopped-server access. README includes
  the commands and eligibility limitations. At this CT.2 checkpoint, CT.3 still
  owned complete application/job
  write gating; CT.4 owns descriptor-driven forms and unavailable-owner browser
  coverage. No user archive was changed, sibling repository modified, or commit made.

## CT.3 local implementation — 2026-09-11

- Centralized application content-write policy covers focused and Wave 1 Item,
  specimen/component/loan, Work/Edition/Capture, Asset, legacy upload, provisional
  Import, integrity, acquisition and processing boundaries. It requires Curate,
  Assigned ownership, an installed owner and supported Item schema. Provisional
  Import inspection was deliberately left as an authorized read.
- Import analysis and selected candidates are owner-bound at enqueue, execution,
  review and publication. Registry-declared SharedMedia processing remains allowed
  without changing collection identity. Frozen processing/acquisition inputs bind
  the Assigned owner; queued work rechecks binding, module, scope and version.
- Missing-owner queued work fails durably. Existing snapshots/observations and
  revision-committed archive effects reconcile before availability/grant checks.
  An acquisition crash-after-commit regression now completes with an empty module
  registry and does not reacquire provider bytes.
- Legacy and unavailable-owner content remains readable. Tests prove activity and
  grant management remain independent, existing Artwork bytes remain authorized,
  and new Item/inventory/Import mutations are denied. Wrong-owner/unsupported-schema
  Item input and cross-owner analysis are rejected before persistence. Wishlist and
  unidentified/no-Capture inventory remain valid.
- Passed: architecture/document/fixture checks; 193 locked workspace Rust tests;
  warning-free locked all-target Clippy; Rust formatting; generated API drift and
  TypeScript checks; web build. The embedded server rebuild is recorded after the
  final documentation pass. No browser smoke was rerun because CT.3 changes no web
  workflow; existing HTTP tests cover listener composition, viewer and
  cross-collection denials. No user archive, sibling checkout or commit was touched.
- Metadata-authority and recipe publication still have mechanism-only W4.2 ports,
  not production application entry points. CT.3 did not add placeholder APIs; the
  real D06/D07 producers must use the centralized policy. CT.4 remains unimplemented.

## CT.3 review corrections — 2026-09-11

- Moved shared collection write eligibility into the application collection-types
  module and removed a duplicate grant lookup. Ordinary and compatibility Item
  edits validate the stored schema as well as submitted data, preventing an edit
  from overwriting unsupported preserved payloads. A regression retains raw numeric,
  structured and unknown values unchanged after rejected downgrade attempts.
- Long import workers now pass an application-policy guard to the shared Capture
  commit mechanism. It runs after payload staging, immediately before a new revision
  commits, and is skipped during completion of an existing commit. Denial aborts
  provisional copies/journal only; original upload sources remain retryable. Both
  legacy and provisional import paths have denial/retry regression coverage.
- Integrity jobs recheck eligibility during progress and before publication.
  Revision-committed recovery finishes without hashing again. The existing crash
  regression now changes payload bytes after evidence commit and reopens with no
  module registry, proving recovery preserves that historical evidence.
- Legacy upload cancellation, like provisional import/job cancellation, remains
  available when the owner module disappears; further content writes stay denied.
- Passed: 197 locked workspace tests, warning-free all-target Clippy, formatting,
  architecture/document/fixture checks, generated API drift and TypeScript checks,
  web build and rebuilt embedded-assets server. The initial new denial/retry tests
  found an uncleared provisional journal; cleanup was corrected before these passes.
- CT.4 remains responsible for authenticated descriptors, availability-aware UI
  permissions, owner-driven forms and valid unavailable-owner browser fixtures.
  Policy rechecks do not claim an atomic transaction with concurrent grant changes.
  No user archive or sibling checkout was modified; no commit was made.

The CT.3 normal HTTPS/WebAuthn browser flow passes. Its first run caught a smoke
assertion coupled to the former storage error prose; the test now checks the stable
structured error code while retaining rejection/no-publication assertions.

The private LOCAL_SETUP=1 browser flow also passes. Both runs used isolated Chrome
profiles and temporary server/archive state with approved sandbox escalation.

## CT.4 local implementation — 2026-09-13

- Added an authenticated application query and `/api/v1/collection-types` read API
  for the active registry's validated descriptors. The unauthenticated feature probe
  no longer contains module identities/capabilities. OpenAPI and the generated
  TypeScript surface changed together.
- Collection and Item responses now report application-computed content permissions
  and availability. Curate-role flags are no longer presented as executable when a
  binding is legacy-unassigned, its owner is missing, or an Item schema is unsupported.
  Access management remains independently authorized. Legacy responses include the
  exact stopped-server `inspect-collection-type` and `assign-collection-type` commands.
- The web creation picker uses authenticated descriptors. Item creation inherits the
  collection owner and sends a bounded owner-mediated patch. Generic text/integer
  controls preserve numeric types, empty edited values send explicit null, omitted
  values remain untouched, and structured fields are visibly read-only. Switching a
  new Item's collection clears incompatible unsaved domain fields. Browse uses module
  labels and safe presentation summaries without built-in module-ID branches.
- A test-only postage-stamp module declares required integer/text fields plus a
  structured read-only field and its own validator. HTTP coverage proves descriptor
  authentication, typed creation/edit, malformed edit rejection, browse and reindex.
  Registration regressions reject unsafe labels, invalid keys, excessive field counts,
  duplicate capabilities and foreign capability ownership.
- Normal HTTPS/virtual-WebAuthn and `LOCAL_SETUP=1` Chromium smoke both pass. They
  create separate game/music collections, exercise distinct generic fields and draft
  clearing, preserve read-only tracks, reject a wrong-owner API write, and rebuild a
  temporary homogeneous archive assigned to an absent module. The fallback remains
  browsable with Artwork/media paths and no content edit action. This replaces the
  invalid mixed-owner fixture without changing user data.
- Passed: 198 locked workspace tests; strict locked all-target Clippy; Rust formatting;
  architecture/document and Wave 4 fixture checks (8 manifests, 22 vector/composite
  digests); generated API drift; TypeScript; web build; and rebuilt Rust server with
  embedded assets. Browser iteration found and corrected a two-collection mobile
  intrinsic-width regression and stale assumptions about collection order and the
  schema-3 fixture envelope before the successful runs.
- No external format oracle, real authenticator, trusted remote-browser HTTPS, target
  platform qualification, sibling checkout, user archive or commit is claimed. CT.4
  is stopped for final review; D04.1 has not begun.

## CT.4 review corrections — 2026-09-13

- Domain-edit receipts now bind the original patch command under a distinct operation,
  not a resolved full payload derived from mutable current state. A permanent test
  retries an earlier patch after a later edit and checks the original response,
  unchanged latest data, and rejection of a different patch reusing the same key.
- Collection-type descriptors and permission presentation now live with collection
  policy. Registration bounds capability counts and unavailable-reason text as well
  as fields/labels. Failed-registration tests verify no partial registry pollution.
- The generic web form honors Item and Collection content permissions even when the
  owner descriptor is installed. Account/access administration remains available
  without descriptors. New entries prefer the browsed writable collection; changing
  destination explicitly reports cleared type-specific draft fields. Integer edits
  reject unsafe numeric conversions. Rows retain module artist/platform summaries
  alongside edition text instead of hiding them when a subtitle exists.
- Browser fixtures check numeric third-module patch submission (without pretending
  the fixture is production persistence), installed-owner/unsupported-item disabled
  controls, descriptor-independent administration and artist text in artwork rows.
  Actual third-module creation/validation/reindex remains covered through real HTTP
  and application services. Unavailable-owner preservation uses temporary archive
  reconstruction rather than a forbidden mixed-collection write.
- Passed: 200 locked workspace tests, strict all-target Clippy, formatting,
  architecture/document/fixture checks, API drift checking, TypeScript/build and
  embedded server rebuild. No user archive, sibling repository or commit was changed.
- CT.1–CT.4 are reviewed locally; D04.1 resumes under its reader-only extraction,
  affected-consumer and independent-vector qualification gates. Runtime plugins,
  offline clients and platform/real-dump release qualifications remain unimplemented
  or external as previously recorded.

Both final CT.4 browser runs pass: normal HTTPS/virtual-WebAuthn and private
LOCAL_SETUP=1, including the artist-summary assertion. They used isolated Chrome
profiles and temporary servers with approved sandbox escalation.

## D05.1 local implementation — 2026-09-14

- Collector's music module now reads authorized frozen BIN/CUE and current
  raw-redumper packages through opaque package readers. Tests cover exact signed
  `i16` stereo samples, cross-file seeks, typed INDEX 00/01 and source spans,
  stored versus synthetic gaps, signed lead-in, published ARver disc IDs, strict
  structural parsing, missing/truncated sidecars, and mixed-mode refusal. The
  result contract records the exact evaluated offset interval, selected offset,
  and unavailable leading/trailing physical frames.
- `junk-libs-accuraterip` has one production bounded reader implementation. Its
  vector API delegates to that path; literal independent CUETools/ARver-equation
  vectors cover shifts -6 and +6, cross-track borrowing, i16 packing and both CRC
  versions. A generated 80-minute source confirms read requests remain at most
  16,384 frames and cancellation returns an error. The declared job working
  budget is 64 MiB. No missing samples are padded.
- `junk-libs-disc` retains signed raw-redumper coordinates and adds exact
  sample-addressable wrappers. Phono's real AccurateRip caller no longer builds
  whole-disc PCM vectors. Trash Amp's existing redumper metadata consumer now
  performs checked signed-to-TOC conversion. No D05.2 provider, HTTP, dBAR
  acquisition, route, job publication, schema or web workflow was added.
- Collector gates pass: `python3 scripts/check_architecture.py`;
  `python3 scripts/check_wave4_fixtures.py` (8 manifests, 24 vector/composite
  digests); `cargo fmt --all -- --check`; `cargo test --workspace --locked`;
  `cargo clippy --workspace --all-targets --locked -- -D warnings`;
  `npm --prefix web run check`; `npm --prefix web run check:api`;
  `npm --prefix web run build`; and `cargo build -p collector-server --locked`.
  The full tests retain the W4.2 snapshot binding/cancellation, strict provider
  parsing, and unavailable-module historical-read regressions.
- Junk Libs passes format, full all-feature workspace tests, and strict all-target
  clippy. Phono passes format, full workspace tests, and strict all-target clippy.
  Trash Amp passes the same gates after its narrow signed-coordinate migration;
  all of its pre-existing unrelated working-tree edits remain present. Retro Junk
  passes format and full workspace tests without any file change, but strict
  all-target clippy fails on 37 existing library / 45 test-target Rust 1.95 lint
  findings (`must_use_candidate`, `too_many_lines`, `implicit_hasher`, and other
  unrelated debt). Those failures were not changed or claimed as passing.
- The network-gated ARver/AccurateRip tests and environment-gated owned redumper
  dump tests were not run. `JUNK_COLLECTOR_W4_CD_DUMP_ROOT` was not supplied, so
  real-dump and mixed-mode qualification remains external. D05.1 is stopped at
  review; D05.2 and D05.3 remain unimplemented. No commit was made.

## D05.1 review corrections — 2026-09-14

The completed implementation was D05.1, despite the review request naming D05.2.
D05.2 remains unimplemented and is the next handoff. See the
[review](wave-4-review.md#d051-review--2026-09-14) for corrected findings.

- Shared verification records its v1 interval separately from evaluated v2
  candidates; a v2-only nonzero known match without an anchor is inconclusive,
  not mismatched. Literal independent +6 vectors cover this regression.
- A 451-sector generated disc covers the frame-450 boundary; optional probes
  do not read past physical PCM. Entry cancellation is checked even with no data.
- Shared source PCM decoding handles explicit timeline origins, bounded read
  buffers and cross-source seeks. Collector only maps authorized package handles.
- CUE source aliases, out-of-span indexes and four-channel flags fail closed;
  synthetic gaps remain structural results without exact PCM/disc-ID claims.
- `cargo test --workspace --locked`: 221 passed. The final focused module run
  passed 28 tests after adding synthetic-gap/four-channel regression assertions.
- Shared workspace all-feature tests: 239 passed; strict all-feature/all-target
  Clippy passed. Phono workspace: 365 tests passed and strict Clippy passed.
- Collector architecture/fixture checks, formatting, strict workspace Clippy,
  TypeScript, generated API checks, web build and embedded server rebuild passed.
  No web behavior changed, so browser smoke was not repeated for this review.
- Review changes touch Collector, Junk Libs and Phono. The implementing agent's
  Trash Amp coordinate conversion and its unrelated user work remain untouched.
  Its previously recorded Retro lint failures remain external to this review;
  no claim is made that those unrelated warnings were repaired.

No owned real dump or live AccurateRip oracle was available. Synthetic vectors
qualify the narrow reader/arithmetic behaviors, not real-disc/mastering coverage.
No new runtime capability or network provider is exposed.

D05.1 sibling checkpoints:
- Junk Libs: `3c5213706d207ac11405e675e10af7ad4fd18ed6`.
- Phono Junk: `8e212f4a2e261ed963ab560dea78085b4e796693`.
- Trash Amp: `6972ef989006d17d41899f9a5f05f7ae5a43d9ae` (only the signed-coordinate metadata conversion).

Collector consumes these shared libraries through local path dependencies.
Unrelated Phono and Trash Amp working-tree files were excluded from the checkpoint.

## D05.2 local implementation — 2026-09-14

- Added the production AccurateRip provider mechanism in `collector-providers`,
  using the existing small-response `ProviderPort`. Production is pinned to the
  AccurateRip HTTPS origin; local tests can inject only an uncredentialed loopback
  HTTP origin. Requests use a shared one-per-second limiter, a 15-second maximum
  timeout, manual same-origin redirects capped at three, at most three transient
  attempts, cancellable Retry-After delays, and an 8-MiB limit enforced during
  reads. No provider/network code entered the music module.
- Exact 200 dBAR and 404 no-data bodies are retained with SHA-256 and a typed query
  observation. Malformed/truncated, empty-success, wrong-identity and oversized
  responses return unavailable rather than an audio mismatch. Offline replay reads
  the complete archived Snapshot body and verifies recorded length/digest before
  parsing; same-length mutation, truncation, trailing bytes and cancellation fail.
- Music-owned interpretation records algorithm version, database query/response
  identity, exact input digests and per-track computed ARv1/ARv2 checksum/confidence,
  inclusive v1 interval, explicit v2 candidate offsets and unavailable edge frames.
  A shared one-track match cannot promote the disc. Every expected track plus one
  unambiguous selected offset is required for whole-disc verification. Database
  mismatch requires complete v2 coverage and no unavailable physical search edge;
  incomplete v2, edge-limited, tied-offset and frame-450-only cases stay inconclusive.
- TDD failure was recorded before the new contracts existed. Focused policy tests
  now pass 6 cases and local HTTP/frozen-response tests pass 5 cases. The negative
  fixtures specifically include one-of-two matched tracks, candidate-only v2,
  complete-v2 with unavailable edges, tied offsets, frame-450-only support, wrong
  dBAR IDs, changed/truncated/trailing snapshot bytes, oversize, timeout, outage,
  retry cancellation, cached replay and a later response with a distinct digest.
- Passed: `python3 scripts/check_architecture.py`; `cargo fmt --all -- --check`;
  `cargo test --workspace --locked` (232 tests); `cargo clippy --workspace
  --all-targets --locked -- -D warnings`; `npm --prefix web run check`;
  `npm --prefix web run check:api`; `npm --prefix web run build`; and
  `cargo build -p collector-server --locked`. No web asset changed, so browser
  smoke was not applicable.
- D05.2 stops at review. No provider was composed into the server, no music
  capability/job/route/UI was registered, and no observation publication or D05.3
  workflow was added. No sibling file or user archive changed. Live AccurateRip,
  owned real-dump, real mixed-mode, and external mastering/search qualification
  were not available and are not claimed. No commit was made.

## D05.2 review validation — 2026-09-14

Review fixes preserve per-track computation offsets, check match records against
frozen dBAR entries, reject inconsistent result envelopes, rate-limit redirect
requests, and support IPv6 loopback fixtures. The policy suite now has nine cases,
including an actual shared offset search with independently calculated constant-PCM
checksums; the provider suite has six local HTTP/replay cases.

Passed architecture/document links, Wave 4 fixture recipes, formatting, the
workspace test suite, the final focused policy suite, warning-free workspace/all-target
Clippy, TypeScript checking, generated
API contract comparison, web build and rebuilt server binary. No web workflow
changed, so browser smoke was not repeated. Live-provider, owned-disc and full D05
workflow qualification remain outstanding. No sibling file or user archive was
modified and no commit was made. See the D05.2 review and D05.3 handoff constraints.

## D05.3 local implementation — 2026-09-14

- Registered music Import analysis, CD structure and AccurateRip verification with
  the compiled Music owner. All execution uses the existing authorized frozen jobs,
  application cancellation checkpoints, package handles, acquisition executor and
  archive-v4 observation/snapshot publisher. No scheduler, PCM decoder, checksum
  algorithm, migration, conversion or FLAC path was added.
- The server composes one AccurateRip provider instance for requests and checked
  replay. Small bounded responses are adapted into the existing streamed snapshot
  publication path. The parser reads the complete archived payload and rejects
  length/digest changes before emitting neutral dBAR entries. The music operation
  cross-checks the frozen query against package-derived disc IDs and publishes the
  snapshot digest, input SHA-256 values, per-track computation offsets and explicit
  ARv1/ARv2 coverage from the real shared bounded verifier.
- `music_cd_workflow_publishes_shared_accuraterip_results_and_survives_reindex`
  stages/analyzes/reviews/publishes generated BIN/CUE and signed-coordinate redumper
  packages. It exercises whole-disc shared verification, exact archived-byte
  corruption rejection and retry, provider outage and same-Job retry, queued
  cancellation, checked no-data, distinct refresh snapshots, historical evidence,
  unsupported redumper verification that leaves preservation intact, normal reindex
  and archive-only projection rebuild.
- The curation UI exposes clear structure/fetch/verify actions, package and anchor
  selection, immutable snapshot labels, a bounded frame-offset control, durable Job
  progress/errors, per-track outcomes/computation coordinates/version coverage and
  limitations. It remains in the Item dialog beside existing art/text tools. The
  smoke fixture now stages and publishes actual synthetic BIN/CUE bytes and drives
  the structure, local dBAR acquisition and verification controls.
- Added `scripts/run_d05_real_dump_oracle.py`, an ignored private-root test, and an
  example manifest requiring independently recorded layout digest, result/offset and
  per-track ARv1/ARv2 outputs. With `JUNK_COLLECTOR_W4_CD_DUMP_ROOT` unset, the runner
  returned its documented unavailable exit status. No private path or bytes were
  printed or copied.
- Focused provider tests (12 DAT/AccurateRip tests) passed after composing the
  blocking adapter safely across async server startup/shutdown. The focused archive
  workflow test passed. Architecture/document/fixture checks, formatting, all 237
  non-ignored workspace tests, warning-free all-target Clippy, TypeScript checking,
  generated API comparison, web build and the post-asset server rebuild passed; the
  private real-dump oracle remains intentionally ignored without its opt-in input.
  The first browser run found and led to the provider lifecycle correction. Two
  sandboxed launches then aborted with `SIGABRT`/`EPERM`; the allowed run exposed
  stale fixture assumptions and an immediate post-publication Item refresh gap.
  After correcting both, `npm --prefix web run test:smoke` passed in an isolated
  headless Chrome profile, including actual BIN/CUE staging/publication, structure,
  local dBAR acquisition, shared verification, artwork retention and offline error
  recovery through the rendered workflow.

**External/unavailable:** no owned dump, completed private oracle manifest, live
AccurateRip qualification, real mixed-mode disc, alternate mastering/offset oracle,
physical media, target Linux/NAS or external browser matrix was available. Synthetic
success is not claimed as those gates. D05.3 stops for D05 root review. D07 is the
recommended next root under the music-first proposal after acceptance; it was not
started.

Manual D05 check: create/open a Music CDs collection and Item with artwork; in
Capture accession select all files for one copied BIN/CUE package, stage it, choose
Review import, keep it unidentified if appropriate, and publish. Reopen the Item's
curation dialog. Under Music CD analysis choose the two-file Representation, select
Inspect CD structure, select BIN/CUE and the `.cue` anchor, then run it. Confirm the
track INDEX/PCM coordinates and limitations. Choose Fetch AccurateRip response and
watch Background work; an outage may fail/retry but the Capture and structure must
remain. After a response is archived, choose Verify audio with AccurateRip, select
that immutable snapshot and an explicit offset bound, then run it. Inspect the disc
outcome plus every track's computed offset, ARv1 range, explicit ARv2 offsets and
limitations. Fetch again to confirm both old and new snapshot-bound results remain,
restart/reindex, and confirm history and artwork still appear. Repeat structure with
a copied current redumper package; unsupported coordinates/layouts must say so and
must not remove or relabel the preserved master.

## D05.3 review validation — 2026-09-15

The reviewed integration uses one acquisition registry, a bounded-response adapter,
and music-owned shared catalog DTOs. Layout-to-query policy is module-owned. CUE
reads are capped during consumption; the private oracle hashes payloads in 64-KiB
chunks. The UI exposes checksum/confidence and physical edge coverage, including
nested structure observations. AGENTS.md records the architectural lesson.

Passed: architecture/document links, Wave 4 fixture recipes, formatting, workspace
checks/tests, all-target warning-free Clippy, TypeScript, generated API comparison,
web build and rebuilt server. The extended focused CD workflow passed with both
supported and unsupported raw-redumper packages (68.86 seconds); viewer acquisition
denial is covered. Both normal and LOCAL_SETUP=1 browser smoke passed after updating the expected
sample-unit label and adding confidence/edge-detail assertions.

The default selected Xcode toolchain refused linking because its license had not
been accepted. These review checks used the already-installed Command Line Tools
via DEVELOPER_DIR=/Library/Developer/CommandLineTools, without accepting a license
or changing system configuration. Sandboxed Chrome aborted with SIGABRT/EPERM;
browser runs used approved execution with isolated temporary server/browser state.
The private real-dump oracle, live AccurateRip, Linux/NAS and physical-device
qualification remain unrun. No sibling file or user archive was changed; no commit
was made. D07.1 is next, with optional explicit FLAC preparation.

## D07.1 local implementation — 2026-09-15

D07.1 is implemented locally and stopped for review. Before implementation, the
Wave 4 plan recorded the producer/consumer contract: the music module owns one
exact seekable integer PCM timeline and ordered track ranges; application code
maps readable presets to immutable recipes and owns future authorization/jobs; the
provider adapter owns optional processes and receives only an authorized reader
plus an already-opened scratch handle. D07.2 will own durable publication/recovery,
and D07.3 will own authorized availability/media descriptors. No placeholder route,
registered conversion Job, representation publication, web playback or metadata
provider was added.

- `ConversionProfile::Flac` seals level 5, `flac`, its exact probed version and
  unchanged signed little-endian 16-bit stereo 44,100-Hz input/output. Opus accepts
  only 96/128/192 kbps, 20-ms frames, `opusenc`, the same CD-DA input and an
  explicit 48-kHz stereo decoded format. Source fingerprint and selected ordered
  ranges remain in the canonical recipe digest.
- The D05 CUE reader now has a conversion view which starts at the first stored
  audio byte. Stored HTOA is prepended to track 1; later stored INDEX 00 audio stays
  on the preceding track through the existing INDEX-01-to-next-INDEX-01 span.
  Exact ranges are nonempty, contiguous and cover the timeline once. Synthetic
  gaps, mixed mode, four-channel flags, missing ranges and ambiguous package
  members continue to fail before conversion. Redumper retains its validated
  signed first-track-to-lead-out range.
- The process adapter passes arguments directly, uses 16,384-frame PCM chunks,
  caps diagnostics at 64 KiB, caps encoded output from the checked source size
  (with an 8-GiB absolute ceiling), caps Ogg packets at 1 MiB, and uses an explicit
  six-hour process ceiling. Cancellation kills/reaps the child; every post-start
  failure clears only the supplied scratch handle. No package name, selected
  archive root or shell command reaches a child.
- Local `/opt/homebrew/bin/flac` reported 1.5.0. The generated four-frame extreme/
  impulse vector encoded with the manifest's exact arguments to SHA-256
  `edb1aed9e9366afa9a5734a62cca23b40701a4d698633084ab54b394ce677588`.
  Claxon 0.4.3 independently decoded the result and compared every channel sample,
  exact frame length and format against a separately seeked D05-style integer
  reader. A nonzero selected range, internal short reads and EOF are exercised.
- `opusenc` is absent. The profile is therefore unavailable rather than simulated.
  A synthetic independently constructed Ogg Opus vector checks stereo OpusHead,
  the fixed 20-ms packet duration, 48-kHz source-duration conversion, 312-frame
  pre-skip and 320-frame end padding. No encoded Opus golden is claimed.
- Focused tests also cover unsupported recipe values, wrong encoder identity or
  version, absent tools, out-of-range selection, direct process failure,
  cancellation and partial-scratch cleanup. AccurateRip input/result is absent
  from the recipe/conversion API, so no-data, mismatch or outage cannot gate a
  supported conversion; no conversion result is described as AccurateRip evidence.

Passed exactly: `python3 scripts/check_architecture.py`; `cargo fmt --all --
--check`; `cargo clippy --workspace --all-targets -- -D warnings`; and
`DEVELOPER_DIR=/Library/Developer/CommandLineTools
SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk cargo test --workspace`
(245 passed, 0 failed, 1 ignored private D05 oracle). The Command Line Tools
environment avoids the default Xcode installation's unaccepted-license linker
failure without changing system configuration. `npm --prefix web run check` and
`npm --prefix web run build` also passed; no web asset changed, so smoke was not
applicable. The installed FLAC version and golden digest were independently
reproduced from the manifest input with `flac --version` and the exact direct
encoder arguments.

**External/unavailable:** `opusenc` and therefore a real Opus golden; an owned CD/
redumper package; alternate mastering/offset and external FLAC oracles; real
mixed-mode/unsupported-layout corpus checks; target Linux/NAS process behavior;
and later publication, recovery, HTTP range, browser decoding/playback and device
qualification were not available or are outside D07.1. Masters and existing
archives were not modified. No sibling checkout was changed and no commit was made.

## D07.1 review — 2026-09-15

Extracted integer FLAC comparison and strict generated Ogg Opus inspection to
`junk-libs-audio`'s optional `derivative-validation` feature. Collector retains
process mechanics, recipe policy and cancellation mapping. Source-reader overrun,
validation cancellation and final hashing checks fail closed; failed output workers
terminate the isolated process group and discard scratch. Encoder identity now
retains bounded full version reports, including reported library versions.

The independently generated shared `silence-588.opus` fixture uses FFmpeg 8.1.2 /
libopus and contains 960 coded frames, final granule 952, pre-skip 312, playable
frames 640 and end padding 8. This replaces the malformed synthetic container and
incorrect 320-frame padding expectation. CRC, EOF, stream continuity/EOS and
malformed-header checks are bounded. It is an Ogg/timing oracle, not an opusenc
golden or a claim of lossy PCM equality. See RFC 7845 and the links in the review.

Focused conversion tests passed, including real FLAC 1.5.0 PCM equality, a disk
write failure with a still-running encoder, and cancellation during the second
(source validation) pass. Junk Libs audio all-feature tests and all-target Clippy
passed. Trash Amp `amp-core` and `amp-audio` tests passed with `--locked`; its
unrelated working-tree changes were preserved. The extraction changes only the
Junk Libs audio manifest, module export, validation module and generated fixtures.

No real opusenc, owned dump, Linux/NAS or playback qualification is claimed. No
publication/API/UI was added, so browser smoke is not applicable. Builds use
DEVELOPER_DIR=/Library/Developer/CommandLineTools to avoid the selected Xcode
installation's license issue without altering system configuration. No commit
was made; the shared extraction must be checkpointed separately from Collector.

Collector review gates also passed: full workspace tests, warning-free all-target
Clippy, formatting, architecture/document links, fixture reproduction, TypeScript
checking, generated API comparison, web build and rebuilt server. The final
focused conversion suite passed after the version-report/explicit-argument changes.

## D07.2 — explicit derivative publication (2026-09-15; review pending)

Implemented and tested locally, not a completed D07 root review. No commits or
agent delegation were used. Existing D07.1 and unrelated local changes were
retained; this child did not edit sibling Junk Libs.

Workflow: import a synthetic two-track BIN/CUE through the real provisional
Import service, preserve HTOA on track 1 and later stored INDEX 00 on the preceding
track, explicitly request each track, independently validate real FLAC, publish
separate immutable recipes/outputs and validation Evidence, associate/edit an
existing Item, read the stored files and reindex. No AccurateRip result is a
prerequisite. Jobs use existing receipts, leases, cancellation and Capture
revision publication. Real encoding and validation happen outside the writer lock.

Executed checks:

- `COLLECTOR_TEST_FLAC=1 cargo test --workspace --locked`: passed, 257 tests;
  one existing ignored test. This includes the real FLAC publication opt-in and
  existing independent decoder/process-validation regressions.
- `COLLECTOR_TEST_FLAC=1 cargo test -p collector-store --test archive derivative_
  --no-fail-fast`: passed, 11 harness cases (including the subprocess helper).
- `cargo test -p collector-store --test archive derivative_ --locked`: passed,
  11 harness cases after the recovery/missing-module and validation refinements.
  Without `COLLECTOR_TEST_FLAC`, the real-tool opt-in body is intentionally unrun;
  explicit unavailable/missing/version-changed adapter paths are tested.
- `cargo test -p collector-app validation_envelope_distinguishes_opus --locked`:
  passed. A structural Opus envelope cannot claim lossless PCM equality or use
  the CD input-frame count as its decoded 48-kHz duration.
- `cargo test -p collector-store --test archive
  derivative_publication_failures_restart --locked`: passed after the final
  recovery correction. A committed archive recovers even when a returned error
  left the private job failed with its claim token cleared.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `python3 scripts/check_architecture.py`, `cargo fmt --all -- --check`, and
  `git diff --check`: passed.
- `npm --prefix web run check`, `npm --prefix web run check:api`, and
  `npm --prefix web run build`: passed. OpenAPI/TypeScript contracts are unchanged.
- `cargo build -p collector-server`: passed after the web build.

Fault coverage includes two simultaneous encoders for one recipe converging on
one Capture output; request replay; separate track identities; changed source
records and actual source bytes; changed encoder reports with frozen retry;
failed validation; corrupted scratch; queued and post-encode cancellation;
revoked authorization; failure and cancellation at Prepared, BytesStaged,
PayloadPublished, RevisionCommitted, ProjectionApplied and Completed; actual
child-process termination at those same phases; restart after commit without a
tool, grant or installed music module; preserved Item edits; and corrupt published
bytes rejected by reindex/reopen. The publication tests use clearly labeled fake
encoder bytes for deterministic faults; they do not qualify codecs. The enabled
real FLAC test uses `flac 1.5.0` plus shared independent integer PCM validation.

The initial link attempt hit the selected Xcode installation's unaccepted license.
The installed Command Line Tools successfully ran focused tests. After the user
accepted Xcode's agreement and installed updates, subsequent checks used the
default toolchain successfully; no license or global toolchain setting was changed
by this task.

Limits: `opusenc` is absent, so real opusenc conversion/publication remains unrun.
Existing FFmpeg/libopus fixtures qualify structural/duration inspection only, not
lossy PCM equality. Owned-disc, real mastering/redumper conversion, Linux/NAS,
large-library and live-provider qualification remain unrun. No new routes or web
controls were added, so D07.3 browser playback and derivative-specific transport
qualification remain pending; browser smoke was not rerun for this service/storage
child. No automatic conversion on import or media GET is implemented. Stop for
D07.2 review; the plan's ready-to-send prompt requests review, not D07.3 implementation.

## D07.2 review validation — 2026-09-15

Reviewed and corrected locally; no commit made. The new regression checks a
retained Capture with contradictory format, derivative role or profile despite
unchanged payload/validation digests. Completed-job recovery and reindex both
reject each case without invoking the encoder again. Rebuild, recovery and
concurrent convergence now share the retained-record consistency check.

Passed on the default macOS toolchain:

- `COLLECTOR_TEST_FLAC=1 cargo test -p collector-store --test archive derivative_ --locked`
  — 12 tests, including the subprocess helper, real FLAC, publication fault/kill
  matrices, cancellation, concurrency and the new contradictory-record cases.
- `COLLECTOR_TEST_FLAC=1 cargo test --workspace --locked` — 259 passed, one ignored
  external real-dump oracle. The existing AccurateRip workflow test completed
  successfully after exceeding the test runner's 60-second notice threshold.
- `python3 scripts/check_architecture.py`, `python3 scripts/check_wave4_fixtures.py`,
  `cargo fmt --all -- --check` and `git diff --check`.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`.
- `npm --prefix web run check`, `npm --prefix web run check:api`,
  `npm --prefix web run build`, then `cargo build -p collector-server --locked`.

No route or UI behavior changed, so browser smoke was not rerun. No sibling code
was changed in this review; D07.1's shared-library extraction remains uncommitted.
Real opusenc, owned-disc, NAS/Linux and scale qualification remain unrun. This
review does not claim new browser playback or catalog verification acceptance.

## D07.1–D07.2 checkpoint — 2026-09-15

User authorized the reviewed checkpoint. Shared audio validation is committed
separately in Junk Libs as `d6d7d18` (Extract shared FLAC PCM and Ogg Opus
derivative validation). Collector includes the reviewed recipes, optional encoder
adapter, recoverable publication and D07.3 handoff. Earlier no-commit notes above
record the state at review time.

## D07.3 local implementation — 2026-09-15

D07.3 is implemented locally and stopped for D07 root review. No commit or agent
delegation was used. The application contract exposes ordered, viewer-safe
track/profile state and immutable published media facts; private Job information,
raw Evidence and archive locators remain internal. Server-owned optional encoder
configuration supplies probed complete version reports to the reviewed frozen Job
path. Preparation stays explicit and curation-only, one output per track. Missing
tools do not affect preservation or original/published reads, and import/media GET
never initiates conversion.

Passed before the final full-gate rerun:

- `COLLECTOR_TEST_FLAC=1 cargo test -p collector-server --test http -- --nocapture`
  — 26 HTTP tests. The D07.3 case uses real `flac 1.5.0`, completes a frozen
  preparation Job, independently validates output against source PCM, reloads the
  published row after restart without the music module/tool, and covers viewer,
  curator, cross-collection, exposure, missing-tool and revoked-grant behavior.
- The same HTTP case checks strong digest ETags and exact full, partial, suffix,
  open-ended, unsatisfiable, stale If-Range and weak If-Range results for original
  and derivative bytes, including actual response lengths.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`,
  `npm --prefix web run check`, `npm --prefix web run check:api`,
  `npm --prefix web run build`, and `cargo build -p collector-server --locked`.
- OpenAPI was regenerated and reports 81 paths; TypeScript contracts and embedded
  curation assets were rebuilt.

Both `npm --prefix web run test:smoke` attempts, including the retry against a
rebuilt embedded-assets server, stopped before application navigation:
Google Chrome aborted during Playwright launch (`SIGABRT`; cleanup then reported
`kill EPERM`). The retry did start the test server and confirm its FLAC 1.5.0
probe, but Google Chrome 153.0.8010.37 never opened a page. The actual web
preparation workflow is therefore unqualified in this environment rather than
reported as passed.

Real `opusenc`, owned-disc/mastering/redumper, Linux/NAS/deployment and scale
qualification remain unrun. D07.3 does not claim actual browser playback, seeking,
gapless behavior or track transitions; those belong to the next bounded browser
player child even when its media URL can be downloaded successfully.

Final repository gates passed:

- `python3 scripts/check_architecture.py` and
  `python3 scripts/check_wave4_fixtures.py` (8 manifests, 24 vector/composite
  digests; semantic oracles remain separate).
- `cargo fmt --all -- --check` and
  `cargo clippy --workspace --all-targets --locked -- -D warnings`.
- `COLLECTOR_TEST_FLAC=1 cargo test --workspace --locked`; all executable tests
  passed. The existing private owned-disc oracle was the sole ignored test. The
  existing AccurateRip workflow exceeded the harness's 60-second notice and then
  passed.
- `npm --prefix web run check`, `npm --prefix web run check:api`, and
  `npm --prefix web run build`, followed by
  `cargo build -p collector-server --locked`.
- `git diff --check` after documentation and generated-asset updates.

## D07.3 review validation — 2026-09-15

Reviewed and corrected locally; no commit or sibling edit. The HTTP regression
now replays an accepted preparation request with the encoder removed, returning
the original Job, and rejects a changed profile under the same command key. It
checks typed preparation permissions for viewers, curators, missing tools and
retained output without its module. Existing real-FLAC, restart, range and access
checks remain enabled.

Passed:

- `COLLECTOR_TEST_FLAC=1 cargo test -p collector-server --test http music_derivative_http --locked`.
- `COLLECTOR_TEST_FLAC=1 cargo test --workspace --locked`: 260 passed, one ignored
  external owned-disc oracle; no failures.
- `python3 scripts/check_architecture.py`, `python3 scripts/check_wave4_fixtures.py`,
  `cargo fmt --all -- --check`, and `git diff --check`.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`.
- `npm --prefix web run check`, `npm --prefix web run check:api`,
  `npm --prefix web run build`, and `cargo build -p collector-server --locked`.
- Both `npm --prefix web run test:smoke` and
  `LOCAL_SETUP=1 npm --prefix web run test:smoke` passed with isolated temporary
  server/profile data in Google Chrome 153.0.8010.37. Chrome was launched outside
  the filesystem sandbox; this resolves the earlier sandbox launch limitation.
  Both runs exercised actual FLAC preparation and download, ranged reads, styles
  present in served CSS, artwork retention, and visible failure feedback. The UI
  failure test injects a 503 response; subsequent preparation uses real FLAC.

The generated contracts include `can_prepare`; clients no longer parse reason
text as a capability. The new polling effects abort obsolete reads and clear
obsolete timers; reopened availability follows already-preparing jobs. The server
binary was rebuilt after web assets and the final CLI guidance update.

This qualifies preparation/download behavior only. Actual media-element playback,
seeking and track transitions remain the next browser-player child's acceptance.
Real opusenc, owned-disc, Linux/NAS/deployment and scale tests remain unrun.

## Bounded browser-player child implementation — 2026-09-15

Implemented locally and stopped for review; no commit, agent delegation, backend
player, queue protocol, route or storage change was added by this child. The client
bundle mounts a small HTML media-element player beside the existing Item artwork
and text. It consumes D07.3's ordered availability rows and same-origin media URLs
directly, groups queues by Capture and source Representation, and selects one
decodable representation per track with FLAC preferred. Profile variants remain
download/preparation choices rather than duplicate queue tracks. The curation
bundle retains explicit durable preparation and typed `can_prepare` behavior.

Actual browser qualification passed on macOS 26.6.2 in Google Chrome
153.0.8010.37. `HTMLMediaElement.canPlayType("audio/flac")` returned `probably`,
and Chrome actually decoded the generated files: the element reached a playing
state and its media time advanced. The two-track fixture uses real `flac 1.5.0`
outputs prepared as two separate durable curator Jobs. A separate Viewer account
then exercised stable pause, resumed time progression, an approximate 1.5-second
seek, next/previous changes to the expected immutable Representation IDs, and a
natural `ended` event after seeking near the real track end that began track 2.
Reload reconstructed a stopped Track 01 queue, closing details removed the media
element, and current grant removal cleared playback state and showed an explicit
access error. This is browser media-element qualification, not physical audio-
device output, gapless playback or sample-accurate seeking.

Both real browser modes passed outside the filesystem sandbox:

- `npm --prefix web run test:smoke` — generated and prepared two real FLAC tracks,
  retained the D05 structure/AccurateRip and D07.3 curation workflow, then exercised
  viewer playback, mobile layout, refresh, close cleanup and revocation. The viewer
  did not load `curation.js` and had no preparation action.
- `LOCAL_SETUP=1 npm --prefix web run test:smoke` — repeated the same playback
  assertions from the public client-only origin with curation and administration
  exposure disabled, after signing in through the private setup origin.

Repository gates passed after the implementation:

- `python3 scripts/check_architecture.py` and
  `python3 scripts/check_wave4_fixtures.py` (8 manifests, 24 vector/composite
  digests; semantic oracles remain separate).
- `cargo fmt --all -- --check` and
  `cargo clippy --workspace --all-targets --locked -- -D warnings`.
- `COLLECTOR_TEST_FLAC=1 cargo test --workspace --locked` — 260 passed, with the
  existing user-owned disc oracle as the sole ignored test. The AccurateRip
  workflow emitted its expected over-60-second notice and passed.
- `npm --prefix web run check`, `npm --prefix web run check:api`,
  `npm --prefix web run build`, `cargo build -p collector-server --locked`, and
  `git diff --check`. The final server rebuild embeds the player stylesheet and
  client bundle.

Chrome's positive FLAC result does not qualify other browsers. `opusenc` is not
installed, so real Opus encoding/playback remains unrun. Owned-disc, physical
audio output, gapless/sample-accurate playback, Linux/NAS/deployment, live-provider
and scale qualification remain unrun. D06, C01 and native C04 were not started.

## Bounded browser-player review validation — 2026-09-15

Reviewed and corrected locally; no commit or sibling changes. Passed:

- `npm --prefix web run check`, `npm --prefix web run check:api`,
  `npm --prefix web run build`, followed by `cargo build -p collector-server --locked`.
- `python3 scripts/check_architecture.py`, `python3 scripts/check_wave4_fixtures.py`,
  `cargo fmt --all -- --check`, and `git diff --check`.
- `COLLECTOR_TEST_FLAC=1 cargo test --workspace --locked`: 260 passed, one ignored
  external owned-disc oracle; the existing long AccurateRip workflow passed.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`.
- `npm --prefix web run test:smoke` and
  `LOCAL_SETUP=1 npm --prefix web run test:smoke`, outside the filesystem sandbox
  with isolated temporary servers and Chrome profiles.

Google Chrome 153.0.8010.37 reported `audio/flac=probably` and actually decoded
real FLAC files with advancing media time. Both browser modes passed viewer
play/pause/resume, approximate seek, immutable next/previous selection, natural
ended advance, clean reload and client-only playback without curation.js. New
regressions passed delayed obsolete play-promise rejection, closing during playback
with a retained detached-element paused/source assertion, session-cookie loss
while active, reauthentication and grant revocation. Existing preparation, error,
range, artwork and mobile checks remain intact.

The local-setup run additionally exercised the final added unsupported-codec
simulation (disabled Play and usable FLAC downloads), then restored the browser's
real codec support. It also played retained FLAC after changing the archived owner
to an unavailable module. These browser-controller simulations do not substitute
for the separately observed real decoding/playback checks.

Physical audio-device output, other browsers, real Opus, gapless/sample-accurate
playback, owned-disc, Linux/NAS/deployment and scale qualification remain unrun.

## 2026-09-15 — Album import and dialog usability corrections

Corrected misleading import phase labels and removed the publication callback
from analysis completion. Review submission now says Save review. Music preparation
appears near the editor top; empty/loading states explain the next action. Track
availability refreshes on Capture revisions independently of the Item revision.
Dialogs guard drafts on close/Escape, contain and restore focus, and keep close
controls visible when scrolling. Administration documents the current TOML-only
FLAC/Ogg Opus configuration; automatic metadata lookup and browser-based encoder
settings writes remain unimplemented.

Passed: architecture checker, Rust formatting, workspace tests (one ignored
external oracle), workspace/all-target Clippy with warnings denied, TypeScript
check, web build, rebuilt embedded Rust server, and git diff whitespace check.
The normal Chrome smoke suite passed outside the sandbox using its temporary
server/profile, including real FLAC playback, immediate post-publication track
refresh, rejecting and accepting draft discard, top-dialog-only Escape dismissal,
and the scrolled close-button bounds. The playback reload assertion now waits
for the actual player state rather than interpreting the new loading status as
a completed availability response. The private-local variant was not rerun for
these corrections. The initial sandboxed Chrome launch was unavailable.

## D06.1 durable metadata authority validation — 2026-09-15

Implemented locally and ready for review; no commit or sibling-repository change.
The core/application/store path now provides bounded typed title/subtitle candidate
observations, stable provider-record/version identity, explicit authority changes,
evidence-first deterministic resolution and a materialized display. Existing
inventory titles seed manual authority. A refresh appends lossless observations in
the same schema-4 authority revision as the recomputed display without changing
manual authority. Catalog projection schema 6 rebuilds candidate history by reading
every authority revision while retaining the newest authority/display record.

Permanent tests cover field-override/selection/automatic precedence, complete
identity before language, deterministic provider priority and identity tie-breaking,
Japanese strong identity over an English name match, tentative equal-rank results,
selection through disappearance/outage, stale revision conflict, same-key replay,
changed-input conflict, current authorization after revocation, restart/reindex,
revision-commit interruption recovery, and lexical raw/unknown field preservation.
Existing module tests continue to validate owner-bound typed game platform and
music artist overrides.

Passed after implementation:

- `python3 scripts/check_architecture.py` and
  `python3 scripts/check_wave4_fixtures.py` (8 manifests, 24 vector/composite
  digests; semantic oracles remain separate).
- `cargo fmt --all -- --check` and
  `cargo clippy --workspace --all-targets -- -D warnings`.
- `COLLECTOR_TEST_FLAC=1 cargo test --workspace --locked`: 264 passed, with the
  existing user-owned CD dump oracle as the sole ignored test. The long
  AccurateRip workflow emitted its expected notice and passed.
- `npm --prefix web run check`, `npm --prefix web run check:api`,
  `npm --prefix web run build`, `cargo build -p collector-server --locked`, and
  `git diff --check`.

D06.1 adds no HTTP route, web asset, live provider, or configured network access,
so browser smoke and live-provider checks are not applicable to this child. D06.2
MusicBrainz/CAA and DAT-backed metadata/artwork adapters and D06.3 review UI remain
unimplemented. Provider correctness/rate behavior, remote artwork, owned-disc,
other-browser, Linux/NAS/deployment and scale qualification remain unrun or in
their recorded later gates.

## D06.1 review validation — 2026-09-15

The metadata authority review corrected mutable candidate/observation identities,
stale display after inventory edits, hidden ambiguity and missing publication
checks; details are in `wave-4-review.md`.

- `COLLECTOR_TEST_FLAC=1 cargo test --workspace --locked`: 266 passed, 1 ignored,
  zero failures. Focused final metadata runs also passed: 3 application resolver
  tests and 5 archive tests, including forged-display rejection and inventory CAS.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: passed.
- `cargo fmt --all -- --check`, `git diff --check`,
  `python3 scripts/check_architecture.py` and
  `python3 scripts/check_wave4_fixtures.py`: passed.
- `npm --prefix web run check`, `npm --prefix web run check:api` and
  `npm --prefix web run build`: passed. Rebuilt the embedded-assets binary with
  `cargo build -p collector-server --locked`.

No HTTP route or frontend changed; browser smoke was not rerun for this backend
child. Live MusicBrainz/CAA, real Opus encoding, owned-disc and deployment checks
remain unrun/external. D06.1 has no new user-facing metadata panel; D06.2 providers
and D06.3 review UI remain separate unimplemented children. No commit was made.

## D06.2 metadata providers and artwork validation — 2026-09-15

Implemented locally and stopped for review; no commit, agent delegation, HTTP route,
web metadata-review surface, backend migration, sibling-repository change or
additional provider was added. The application composes one MusicBrainz adapter
and limiter per server plus one Cover Art Archive adapter through the existing
streaming acquisition and catalog-parser registries. DAT-backed descriptive
metadata consumes the frozen D04 snapshot and item-associated match result. Both
paths delegate publication to the existing revisioned metadata refresh effect.

The deterministic loopback provider suite covers generated provider-shaped ambiguous and reordered
MusicBrainz bodies; immutable versions when either exact or normalized content
changes; fixed provider priority; positive and negative cache hits/expiry; explicit
refresh through the limiter; timeout bounded to three attempts; successful 429
Retry-After and cancellation during its delay; malformed and oversized bodies;
foreign, credential-bearing, private/link-local and excessive redirects; pinned
CAA listing/image redirects; meaningful User-Agent delivery; and absence of
ownership, loan, acquisition, private-note and filesystem data in requests.

The archive integration executes actual local MusicBrainz and CAA adapters through
durable Jobs, freezes exact response/image bytes, publishes a valid CAA image through
the existing original-plus-thumbnail Item Asset effect, and only then links it to
the matching metadata candidate. It verifies exact lexical `1e+02` raw history,
fresh observation identity distinct from Job/snapshot/publication identities,
manual inventory text and explicit artwork precedence, stable digest candidate
versions, failed image decoding without displacement, and archive reindex. The D04
workflow now also exercises exact-snapshot DAT metadata refresh, provider provenance,
manual authority and reindex. Existing unavailable-provider Job coverage continues
to prove preservation is not dependent on optional provider composition.

Passed after implementation:

- `python3 scripts/check_architecture.py` (including fixture-manifest validation).
- `cargo fmt --all -- --check`.
- `cargo clippy --workspace --all-targets -- -D warnings`.
- `COLLECTOR_TEST_FLAC=1 cargo test --workspace` with `/opt/homebrew/bin/flac`:
  274 passed, one ignored user-owned CD-dump oracle, zero failures. The existing
  long music-CD/AccurateRip workflow emitted its expected over-60-second notice
  and passed.
- `npm --prefix web run check` and `npm --prefix web run build`.
- Focused provider, application resolver, MusicBrainz/CAA Asset workflow and DAT
  refresh/reindex tests, followed by `git diff --check` after documentation updates.

Live MusicBrainz/CAA requests, target-Linux/NAS/deployment, browser metadata review,
owned-disc, real Opus and scale qualification were not run. Caches are process-local;
their durable exact-byte record is the existing archived acquisition snapshot.
MusicBrainz and DAT identity observations are descriptive candidate evidence, not
audio verification. D06.3 remains unimplemented pending D06.2 review.

## D06.2 review validation — 2026-09-15

Reviewed and corrected acquisition receipt binding, candidate version/current-history
semantics, connection-time DNS validation, cache limits, rate admission/cooldowns,
and malformed/negative response handling. See `wave-4-review.md` for findings.

- `COLLECTOR_TEST_FLAC=1 cargo test --workspace --locked`: 277 passed, 1 ignored,
  zero failures. Final focused application metadata and archive metadata tests
  also passed (4 and 7 respectively), including the non-JSON 404 workflow.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: passed.
- `python3 scripts/check_architecture.py`, `python3 scripts/check_wave4_fixtures.py`,
  `cargo fmt --all -- --check` and `git diff --check`: passed.
- `npm --prefix web run check`, `npm --prefix web run check:api`,
  `npm --prefix web run build`, and `cargo build -p collector-server --locked`: passed.

The review rechecked official MusicBrainz/CAA documentation. No live provider,
metadata browser workflow, owned-disc, Opus or external deployment qualification
was performed. Browser smoke was not rerun: no metadata routes or frontend changed.
The initial rerun exposed a hanging fixture on assertion failure; bounded teardown
and blocking accepted sockets fixed its failure reporting. The final runs above
passed. D06.3's ready-to-send implementation prompt is current. No commit made.

## D06.3 local implementation validation — 2026-09-16

The new curation-only metadata contract test covers collection-role denial, disabled
public-route absence, optimistic revision conflict, explicit candidate selection,
safe current/history DTOs, raw provider-body filtering, typed unavailable-provider
state, viewer omission of inventory baselines, role-visible effective artwork and
effective-title search. Existing D06.1/D06.2 archive suites continue to cover outage
preservation, current-versus-historical refresh, immutable versions, manual authority,
failed image publication, restart and reindex.

The browser smoke passed in Google Chrome 153.0.8010.37 with local MusicBrainz/CAA
fixtures and actual published PNG bytes. It exercised provider guidance and defaults,
durable Job polling/reopen, three retained alternatives, Japanese explicit selection,
field override/clear, a preserved conflict draft, manual artwork set/clear, CAA
listing/image/publication/re-refresh, effective search, viewer redaction, keyboard
navigation and a no-overflow 390×844 layout. The existing FLAC playback workflow also
passed with FLAC 1.5.0.

Final gates:

- `python3 scripts/check_architecture.py`: passed, including documentation links and
  Wave 4 fixture manifests.
- `cargo fmt --all -- --check`: passed.
- `COLLECTOR_TEST_FLAC=1 cargo test --workspace --locked`: 278 passed, one ignored
  user-owned CD-dump oracle, zero failures.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `npm --prefix web run check`, `npm --prefix web run check:api` and
  `npm --prefix web run build`: passed.
- `cargo build -p collector-server`: passed after rebuilding changed embedded assets.
- `npm --prefix web run test:smoke`: passed with the deterministic provider fixtures,
  published artwork, virtual WebAuthn and real FLAC browser playback.

Live MusicBrainz/CAA, other browsers, target deployment, owned-disc, Opus and scale
qualification were not run. D06.3 is implemented locally and awaits the D06 root
review; P05 has not started. No commit was made.

## D06 root review validation — 2026-09-16

Reviewed D06.3 and corrected application ownership of acquisition completion,
receipt replay order, detached/persisted client completion, draft dismissal,
stable sibling panel keys, lookup eligibility and effective search projection.
The expanded HTTP regression checks unchanged effective provider search after an
unrelated inventory update, later manual inventory title precedence, and both
cases after reindex. Completion routes reject viewers and remain absent from a
public listener with curation disabled.

Passed:

- Architecture/documentation and Wave 4 fixture checks; Rust formatting and diff
  whitespace checks.
- `COLLECTOR_TEST_FLAC=1 cargo test --workspace --locked`: 278 passed, one ignored
  user-owned dump oracle. The expanded metadata HTTP regression passed again after
  the final search cases were added.
- Workspace/all-target Clippy with `--locked -- -D warnings`.
- Web type check, generated API check, production asset build and rebuilt embedded
  Rust server.
- Standard browser smoke in Google Chrome 153.0.8010.37: held an acquired-result
  completion request, closed the panel, checked unchanged authority and reopened
  to finish; exercised explicit cover publication/selection, metadata history,
  conflict drafts, search, viewer filtering, keyboard order and 390px layout.
  Actual FLAC 1.5.0 playback, seek, queue advance and grant revocation passed.

The local-setup/private-management smoke also passed on the final rebuilt binary,
including the metadata workflow, wrong-origin guidance, repaired origin, preserved
TLS/account state, client-only routing, private management and actual FLAC playback.
Initial validation caught the new route-count assertion, a legitimate draft-discard
confirmation in the test, and duplicate sibling React keys; these were corrected,
and the standard browser workflow passed afterward. Screenshots were inspected for
mobile layout. Live providers, other browsers, owned-disc, Opus, deployment and
scale qualification remain unrun. D06 root review is complete; P05.1 is next.
No commit made.

## Local-alpha planning validation — 2026-09-16

Documentation-only scheduling/contract change after D06.3 commit `3a6143b`.
Added the accepted local-alpha delivery plan, illustrative typed contracts,
coordinator pseudocode, and two PlantUML source diagrams. Updated active handoffs,
AGENTS.md, architecture/status/release scope and the older detailed plans together.
LA1 is next; P05.1 is paused. No alpha feature is claimed implemented.

`python3 scripts/check_architecture.py` passed (including documentation links and
fixture checks); `git diff --check` passed. Inspected the diagrams as source;
PlantUML is not installed and diagram rendering was not run. No runtime source or
assets changed, so Rust/browser suites were not rerun. Actual dump corpus, live
providers, deployment and backup restore remain LA3 qualification work.

## LA1 local implementation validation — 2026-09-16

LA1 guided ingestion and automatic enrichment is implemented locally and submitted
for integrated review. Focused store regressions cover intent replay and changed
input, stable Item/Capture/Representation identities, intent-to-Job restart
reconciliation, automatic MusicBrainz/CAA title and artwork, DAT-backed metadata,
manual-title authority during provider failure/retry, cancellation with retained
bytes, actual unpublished discard reclamation, cross-user denial, too-late discard
and published Capture association correction. The HTTP regression additionally
exercises configured-inbox planning and staging, traversal rejection, replay,
server-owned completion and private history authorization.

The standard temporary-profile browser journey passed in Google Chrome
153.0.8010.37. It uploaded a package from the collection page without a pre-created
Item, previewed the destination, accepted the intent, closed the curator, reopened
server-side history, observed `published · succeeded`, inspected the retained
Capture identity and found the new collection entry. Existing WebAuthn, role,
management, artwork and real FLAC 1.5.0 playback/revocation coverage passed in the
same run.

Final gates:

- `python3 scripts/check_architecture.py` and `cargo fmt --all -- --check`: passed.
- `cargo test --workspace`: 278 passed, one ignored user-owned CD-dump oracle,
  zero failures. The expected long music-CD/AccurateRip test passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `npm --prefix web run generate:api`, `npm --prefix web run check`,
  `npm --prefix web run check:api`, `npm --prefix web run build` and
  `cargo build -p collector-server`: passed; the Rust server was rebuilt after the
  embedded web assets changed.
- `npm --prefix web run test:smoke`: passed with temporary accounts, archive,
  local deterministic providers and virtual WebAuthn.
- `git diff --check`: passed after the final implementation and documentation edits.

Live MusicBrainz/CAA, user-owned game/CD dumps, real Opus, other browsers, target
deployment, backup restore and scale qualification were not run. Deterministic
provider and synthetic package fixtures do not satisfy LA3 real-input qualification.
The broader P04 bulk/history cleanup and application-wide U05 audit remain planned;
LA2, P05, native clients and a new online game provider were not started. No commit
was made.


## LA1 independent review — 2026-09-16

Review fixes cover durable receipt ordering, paginated periodic recovery, terminal
workflow state, shared provider mapping and full-history replay, initial-authority
recovery, transactional cancellation/retry admission, cleanup receipts/progress,
module-owned readiness and the guided UI lifecycle. See local-alpha-review.md.

- `COLLECTOR_TEST_FLAC=1 cargo test --workspace --locked`: 285 passed, one ignored
  user-owned CD dump/oracle gate. All workspace doc-tests passed.
- After the final progress-label correction, `cargo test -p collector-store --test
  archive guided`: all six focused ingestion regressions passed. Includes malformed
  CUE analysis preserving originals, post-publication cancellation retaining the
  Capture, stopped-import admission denial and provider observation replay.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`, architecture
  checker, formatting, TypeScript, generated API check and `git diff --check`: passed.
- Web assets rebuilt and the embedded server rebuilt. FLAC 1.5.0 was available.
- `npm --prefix web run test:smoke` and `LOCAL_SETUP=1 npm --prefix web run
  test:smoke`: passed using Google Chrome 153.0.8010.37. Both include guided
  browser CD intake with automatic metadata/artwork, close/reopen history,
  shared artwork after reindex, actual playback, revocation and existing UI checks.
- Final generic provider-Job cancellation correction: all six guided regressions
  passed again; Clippy and the embedded server build were rerun.

Earlier review runs exposed and corrected cleanup completion SQL typing and stale
progress projections. The expanded browser fixture required an actual queried disc
ID and an additional shared-artwork row; it waits for upload completion before
closing. A stalled older held-request test was stopped and its fixture hold bounded.
These failed/interrupted intermediate runs are not counted as acceptance.

Live MusicBrainz/CAA, user-owned dumps, target hosting, the LA3 backup drill and
non-Chrome browsers remain unqualified. PlantUML sources were updated; no renderer
was available. No sibling extraction was performed, and no user archive was touched.

## LA2 local implementation validation — 2026-09-16

LA2 album preparation and browser listening is implemented locally and submitted
for integrated review. Focused permanent regressions cover application-owned
ambiguous-source planning; two associated discs; missing encoder readiness; stable
overlapping batch/member and child Job identities; shared-work-safe cancellation;
deliberate one-track validation failure with another track published; partial state;
retry; Store close/reopen recovery; import-time preparation; and reuse of an already
valid output without another encoder call. Existing derivative suites cover source
mutation, permission revocation, queued/running cancellation, publication fault
recovery, recipe/tool-version changes and independent FLAC validation.

Generated HTTP coverage contains plan, submit/list/detail, cancel and retry album
routes, with curation-listener composition and checked OpenAPI/TypeScript output.
The standard temporary-profile browser journey passed in Google Chrome
153.0.8010.37 using FLAC 1.5.0 behind a three-second test-only delay wrapper. It
submitted two overlapping Prepare album batches before completion, restored
server-owned batch/member progress, kept refresh and inventory-title controls
enabled, and played the first published track in a second browser page while another
member was still queued/running. It then observed `2/2 ready`, received two real FLAC
links and exercised actual pause/resume, seek, next/previous, natural-ended advance,
clean reload, session expiry and grant revocation. Guided import exposed the optional FLAC preparation
choice and its durable step. Artwork remained alongside album/player text. The
same run retained deterministic provider, role, management, mobile-layout and
virtual-WebAuthn coverage.

Final integrated implementation gates:

- `python3 scripts/check_architecture.py`: passed.
- `cargo fmt --all -- --check`: passed.
- `cargo test --workspace`: 289 passed, one ignored user-owned CD dump/oracle,
  zero failures; all doc-tests passed. The long music-CD/AccurateRip test passed.
- Focused LA2 tests passed again while iterating, including the album overlap/
  partial/retry/restart case, guided preparation/reuse case, two-disc/missing-tool
  case and ambiguity unit case.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `npm --prefix web run check:api`, `npm --prefix web run check` and
  `npm --prefix web run build`: passed.
- `cargo build -p collector-server`: passed after the final embedded asset build.
- `npm --prefix web run test:smoke`: passed after rerunning outside the filesystem
  sandbox so temporary-profile Chrome could launch. An initial sandboxed launch was
  aborted by Chrome before a page opened and is not counted as product evidence.
- `git diff --check`: passed before documentation finalization and is rerun with the
  final documentation checks.

The wrapper changes only timing: the real configured FLAC binary still encoded and
decoded every accepted output. The durable Store overlap regression separately
supplies deterministic evidence for stable shared identities, cancellation ownership
and restart recovery. The broader U05 application-wide busy-state
audit remains open. User-owned dumps, live MusicBrainz/CAA, Opus, non-Chrome
browsers, target deployment, scale, native clients and the stopped-server LA3
backup/restore drill were not run. No commit was made.

## LA2 independent review validation — 2026-09-17

Reviewed and corrected on Darwin arm64 with Rust 1.95.0, Node 24.3.0,
Google Chrome 153.0.8010.37 and FLAC 1.5.0.

Passed on the reviewed implementation:

- Architecture/dependency, documentation-link and fixture checks:
  python3 scripts/check_architecture.py.
- Formatting: cargo fmt --all -- --check.
- Full workspace: COLLECTOR_TEST_FLAC=1 cargo test --workspace --locked:
  293 passed, one user-owned CD oracle test intentionally ignored because its
  JUNK_COLLECTOR_W4_CD_DUMP_ROOT/oracle.toml inputs were not supplied.
- Clippy: cargo clippy --workspace --all-targets --locked -- -D warnings.
- Web type checking, generated OpenAPI/TypeScript agreement and embedded build:
  npm --prefix web run check; npm --prefix web run check:api;
  npm --prefix web run build; cargo build -p collector-server --locked.
- Both complete temporary-profile Chrome journeys:
  npm --prefix web run test:smoke and
  LOCAL_SETUP=1 npm --prefix web run test:smoke.
- git diff --check.

New regressions exercise requests separated by clock advances, later direct-track
ownership, last shared request cancellation, release of cancelled direct requests,
atomic parent-claim admission, failed-admission outcomes and retry, preserved retry
diagnostics, cross-curator privacy and role loss, bounded admission and cancellation
capacity release, known-carrier alternative sources, guided cancellation with a
missing historical intent pointer, and rejection of cancelled-import retry.

Chrome used genuine encoded and independently validated FLAC, with deliberate
conversion delays. It submitted a track command and overlapping album batches,
preserved a deliberately unchecked disc across refresh, saved inventory during
preparation, played a completed track while another was pending, and verified both
downloads, play/pause/seek/next/previous/ended, reload and revocation. Guided intake
preserved the master, applied deterministic provider metadata/artwork and completed
optional preparation. The local setup variant additionally checked repaired
origins, retained TLS/account identity and private management routing.

The review found and fixed a stale browse-row graph when reopening curation.
Harness corrections added bounded waits, used the actual Save inventory label,
waited for independently polled media links, applied the unsupported-schema fixture
to both list and detail reads, drained held metadata interception before the next
bounded read, and delayed conversion rather than encoder version probes. An attempted change to explicit derivative-command identity was reverted
after the archive revalidation regression correctly rejected it; existing
revalidation behavior remains covered and passing.

No user-owned corpus, live MusicBrainz/CAA, second physical LAN device, target Linux
or NAS installation, stopped-server restore drill, Opus encoder, other browser,
scale benchmark or broad P04/U05 audit was qualified here. LA3 owns the bounded
local installation/corpus/backup qualification; later full-release criteria remain
open. These results do not authorize wholesale adoption of an existing archive.

## LA3 procedure and disposable restore rehearsal — 2026-09-17

LA3 remains in progress. The operator procedure is recorded in
`docs/local-alpha-installation.md`; README recovery guidance links to it. It covers
the complete archive, complete private-state root (SQLite, TLS and pending scratch),
configuration, quiescence through an exclusive Store open, stopped-copy manifests,
empty isolated restore roots, projection rebuild, account/role checks and work
resumption. It explicitly excludes portable export/change bundles and concurrent
use of the copied server identity.

The focused regression
`stopped_server_copy_rehearsal_preserves_archive_accounts_and_pending_scratch`
passed. Its disposable profile contains a published synthetic original and integrity
evidence, Owner and Viewer accounts with an explicit grant, and a browser Import
stopped after two of four bytes. After closing every Store handle, it copies the
complete archive and state into previously absent restore roots. Every file digest
matches before open. The restored Store retains the same ServerId, both working
passwords, Viewer role, original bytes and evidence records; the same Import resumes
at byte two and reaches its checked four-byte digest. This is procedure coverage,
not an owned-dump or live installation claim.

A second CLI rehearsal created a real disposable `collector.toml`, account and TLS
identity under `/private/tmp/junk-collector-la3-rehearsal.h44Cun`. With no server
running, complete relative archive/state roots and config were copied through a
backup directory into separate restore roots and compared recursively. The restored
profile passed `doctor` and `reindex`. Before and after restore, `pairing` reported
ServerId `01a0ad56-06d7-7fc2-9d7e-d941c4e5bf98` and certificate SHA-256
`9098064e9512d52d1541c5fd4d7ead8ad714e33643608e8d115ebe8e7408d392`.
The disposable directory was removed after these values and comparisons were
recorded.

Environment: macOS 26.6.2 arm64 (Darwin 25.6.0), Rust/Cargo 1.95.0, Node 24.3.0,
npm 11.4.2, Google Chrome 153.0.8010.37 and FLAC 1.5.0. Focused results:

- `cargo fmt --all -- --check`: passed before the rehearsal.
- `cargo test -p collector-store --test archive
  stopped_server_copy_rehearsal_preserves_archive_accounts_and_pending_scratch
  -- --exact`: one passed, zero failed.
- `cargo build -p collector-server --locked`: passed.
- Disposable source/backup/restore `diff -rq`: no differences.
- Restored CLI `doctor`, `reindex` and identity comparison: passed.
- `npm --prefix web run test:smoke`: passed outside the filesystem sandbox with
  the temporary HTTPS profile and real FLAC 1.5.0. Chrome completed deterministic
  provider metadata/artwork, guided import, album preparation, actual playback,
  Viewer denial and revocation. The initial sandboxed Chrome process aborted before
  opening a page and is not counted as product evidence.

The intended server/storage roots, owned game/CD corpus and exact redumper/BIN+CUE
variants, real provider configuration/responses, trusted second physical LAN device,
and isolated actual restore destination were not supplied. The real browser journey,
provider-outage/manual-authority cases and actual stopped-server drill therefore
remain required. LA3 is not accepted and the complete repository gate is deferred
until those external acceptance inputs are available.

## LA3 owned-corpus and actual restore qualification — 2026-09-17

LA3 remains partially qualified, not accepted. The exact roots, package digests,
identities and repeatable recovery steps are recorded in
`docs/local-alpha-installation.md`.

Environment: macOS 26.6.2 build 25G83 arm64 (Darwin 25.6.0), Rust/Cargo 1.95.0,
Node 24.3.0, npm 11.4.2 and FLAC 1.5.0. The owned-corpus browser journey used Codex
26.623.81905 with bundled Chromium framework 149.0.7827.197. Repository smoke used
Google Chrome 153.0.8010.37.

Owned-corpus results:

- Five explicit package directories were copied from `/Users/scott/dumps` into
  `/private/tmp/junk-collector-la3-owned.YFRhnq/inbox`. Final `diff -rq` checks for
  all five source/copy pairs returned no differences. The package set comprised 17
  nested mixed-mode game files (1,090,422,259 bytes), a flat 13-file two-track CD
  (334,449,135 bytes), 40 files including 16 existing FLAC sidecars
  (2,455,327,621 bytes), and damaged 8-file/6-file packages (510,365,466 and
  950,108,972 bytes). Direct-child selection and nested relative-path retention
  passed; recursive package-root discovery beneath `bad`/`likely-valid` remains a
  documented limitation.
- The corrected two-track Import produced a retained Redumper layout, two tracks
  and MusicBrainz disc identity `_QyyxioVRyBqTsJQKgyelNOp7rE-`; this is not catalog
  or audio verification. FLAC 1.5.0 produced independently validated 33,799,412-byte
  and 27,623,823-byte derivatives. The actual in-app browser played and paused the
  first result, the Viewer played to 31 seconds, and the restored server played to
  ten seconds. The 16 supplied FLAC sidecars remained in a 40-file
  `preservation_master` and were not presented as Collector derivatives.
- Manual title `LA3 Manual Album Title` survived restart and restore. The initial
  provider-outage run showed `MusicBrainz is not configured; originals remain
  preserved` and published no fabricated cover. With
  `JunkCollector/0.1.0 (https://github.com/AberrantWolf/junk-collector)` configured,
  a live refresh then identified MusicBrainz release
  `99c5a9f0-2070-4738-b4db-05e60395f627` as `HOME` by `B’z` and published real CAA
  cover Asset `d23de098-6923-5811-a842-22dcb7ce02db`. The manual title remained the
  displayed authority, while the subtitle and cover came from the current provider
  candidate. The restored in-app browser rendered cover plus text, loaded both
  prepared FLACs, entered `Now playing`, advanced past three seconds and paused at
  7.45 seconds. This is provider identity/artwork evidence, not catalog or audio
  verification. The unsupported owned package preserved its originals and reported
  `redumper mixed-mode or four-channel CD audio`.
- Game staging Job `009048ea-49bd-58d6-b14c-b7f6e31ff7c5` was interrupted after
  retaining 961,553,973 of 1,090,422,259 bytes, then resumed under the same identity
  and attempt without another Capture. After correcting the shared-envelope defect,
  fresh browser Import `37dafd9b-f1b5-5592-b591-e632ace8f366` preserved all 17
  nested files and published raw-hash observation
  `28fcd52a-9df7-5fb4-b5f2-00e56f5a1984`. It explicitly reported no active DAT;
  no catalog identity or verification is claimed.
- Local Owner/Viewer access passed collection filtering, Viewer FLAC playback and
  HTTP 403 denied self-promotion. After restore, grant revocation returned an empty
  collection list and HTTP 403 media; active browser-session revocation returned
  the browser to login and a revoked API session returned HTTP 401. LAN TLS was
  checked locally with the copied CA certificate and without disabling validation.
  The intended second physical device remains unrun.

Journey corrections and focused checks:

- Explicit CUE/Redumper-TOC anchor selection is signed, persisted and retained for
  analysis/preparation. `redumper_package_can_select_toc_without_splitting_its_cue_companion`,
  `guided_ingest_persists_selected_redumper_anchor_from_signed_plan` and the updated
  guided preparation regression passed in the full workspace.
- A music-only anchor extension is no longer emitted as `null` to strict unrelated
  module inputs. `guided_game_analysis_does_not_receive_the_music_anchor_extension`
  passed focused and in the full workspace.
- The open import panel now follows the server's published association until the
  curator edits the selector. The repaired real browser showed `game mixed nested`
  selected instead of the stale pre-publication `Leave capture unattached` value.
- The live CAA path exposed three owning-layer defects. Production redirects now
  narrowly admit observed `dn<digits>.ca.archive.org` HTTPS storage alongside the
  existing archive origins; numeric and string image IDs normalize to the same
  retained string type; and guided reconciliation reads a retried catalog Job's
  nested completion receipt. Focused checks
  `production_destinations_include_archive_storage_but_reject_private_addresses`,
  `cover_art_listing_and_image_are_bounded_pinned_and_request_no_private_item_data`
  and the retry-expanded
  `guided_automatic_enrichment_applies_provider_title_and_front_art_without_manual_authority`
  passed. Live metadata Job `317f6345-5f1c-57f0-a8e1-0476faf1ac61` completed on
  attempt 1; listing Job `3d46548a-0dc4-530a-8519-783b47370e87` retained two failures
  and completed on attempt 3; image Job `079c4db4-d8d5-5414-8d7f-ecb50e89a6fd`
  completed on attempt 1. The JPEG was 560,318 bytes with SHA-256
  `72cbbd8777f858e99de2892e5dced32c0fa983ec6918f5e048228d065f6eab6b`.

Actual stopped-server drill:

- Source roots were `/private/tmp/junk-collector-la3-owned.YFRhnq/archive` and
  `state`; backup was `backup`; restore roots were
  `restore-parent/archive` and `restore-parent/state`. Exclusive `pairing` opens
  proved quiescence. Complete source, backup and pre-open restore manifests matched:
  archive 88 files / manifest digest
  `c49c3a0bf87ba8cc805e37c32fc33a5f4c4c7e73cd1594797ff8b3a03f1edf79`;
  state 102 files / manifest digest
  `15f41813f286e0bcf45986c90dbec0edf9f71c7eeaa12ee26009ed5ee81caeb4`.
- Source and restore reported the same ServerId
  `01a0adaf-1621-7152-9357-3a6387f04992` and certificate SHA-256
  `eb73cbb34df45936e5593643ae42713f4fcdd9f679b996bce6f509f10d980111`.
  Representative original, observation and metadata-authority record digests
  matched exactly. Both accounts, the Viewer grant and four unfinished Jobs were
  present before restored startup. Restored `doctor`, `reindex`, resumed publication,
  both logins and FLAC playback passed. Original and restored server identities were
  never concurrent.
- After resumed work, the corrected game journey and access revocations, every Job
  was terminal. Clean stop and exclusive reopen passed. At that pre-provider
  checkpoint, manifests contained 218 archive and 120 state files, with digests
  `d2e01ccfd72b5289f001397aee7589addba2d587820c55639b4b2a5a02035c91` and
  `00350bbd8b175222616ae11c759b075279996e30e899922cadc1e6c6f1dd6f01`.
- After live enrichment and playback, all Jobs were again terminal. All three
  listeners were closed, and exclusive `pairing` returned the same ServerId and
  certificate digest. Final manifests contain 235 archive and 120 state files,
  with digests
  `cf07586ba170f7c5a441f136e136cdb156544e29eeb52cea5db2f49cc3429216` and
  `f31f3d0645b581dcf7554abc787357d7864672b34a5ed86099ab35ddf368b125`.

Final repository gates on the corrected checkout:

- `python3 scripts/check_architecture.py`: passed.
- `cargo fmt --all -- --check`: passed.
- `COLLECTOR_TEST_FLAC=1 cargo test --workspace`: 297 passed, zero failed, one
  ignored owned-oracle test whose separate `oracle.toml` input was not configured;
  all doc-tests passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `npm --prefix web run check`, `npm --prefix web run check:api` and
  `npm --prefix web run build`: passed.
- `cargo build -p collector-server`: passed after the final embedded web build.
- `npm --prefix web run test:smoke`: passed outside the filesystem sandbox, including
  actual Chrome/FLAC playback and revocation. An initial sandboxed Chrome launch
  aborted before a page opened and is not product evidence.
- `LOCAL_SETUP=1 npm --prefix web run test:smoke`: passed outside the sandbox,
  including retained TLS/account identity and private-management routing.
- `git diff --check`: passed after the final documentation update.

Missing acceptance evidence is limited but blocking: no intended second physical
LAN browser with trusted HTTPS was available. Its cover/text browsing, real FLAC
playback, denied curation, grant revocation and active-session revocation remain
unqualified. A configured DAT-backed game identity was also absent at that
checkpoint; the 2026-09-19 continuation below supersedes that limitation. Live
MusicBrainz/CAA cover/text passes. This drill does not claim P05 portable
export/change bundles. No commit was made.

## LA3 owned local-DAT qualification and bounded cache — 2026-09-19

LA3 remains partially qualified and unaccepted. The source corpus at
`/Users/scott/dumps` was not read for mutation or changed. Provider artifacts were
stored only under
`/private/tmp/junk-collector-la3-owned.YFRhnq/catalogs`, and only the restored
qualification identity ran during catalog work.

Cache and parser results:

- The Redump PlayStation ZIP/XML were retained byte-for-byte with SHA-256
  `7b2883f4398f9d0e3018649a0ef6e9701b281b30d9f11fedf7eab0a15301781e`
  and `c698671d4fb304ce636352c245323b1f787d216c8f629f9aac1b328537241bb2`.
  A separate manifest-recorded full-release view excluded 975 explicitly tagged
  Alpha/Beta/Demo/Kiosk/Preview/Proto/Sample/Taikenban rows and retained 10,001.
  The 11,993,736-byte derived DAT has SHA-256
  `f3d24d5d394c6f248c6c479b892c67b382a315bdf39220ad4fdb14f3d8befabc`.
- Collector archived and activated those exact derived bytes as snapshot
  `7d61cc8d-eb58-53e7-ac00-f4f17cde7c92` for `sony-playstation` in
  `org.junk.game.cd.raw-2352-track` v1. The first real comparison, observation
  `a8f93fb8-968d-5118-8b44-d828399f37b6`, was partial: all four tracks matched
  `Brave Fencer Musashi (USA)`, but the provider row also carried a CUE checksum.
  No complete-match claim was made at that point.
- The provider adapter now preserves every non-domain source file in
  `excluded_files`; raw-2352 comparison includes only nonempty `.bin` files with
  lengths divisible by 2352 and fails closed when a row has no qualified track.
  Reindex reparsed the same immutable snapshot. The repeated real browser journey
  produced unique complete-match observation
  `1667ceb8-11ac-55a7-96a2-bae41a97ce2b`, provider record 920, with four SHA-1
  matched units for `Brave Fencer Musashi (USA)`. The old partial observation
  remains historical. Applying its title candidate changed descriptive metadata
  only. This verifies the four analyzed catalog tracks, not the CUE text,
  subchannel data, sidecars or whole preservation package.
- DAT-o-MATIC generated a Headerless NES `20260918-104030` DAT after its own queue
  delay using full-release, licensed and machine-life-span selections. It contains
  2,661 rows and 1,107,207 bytes; ZIP SHA-256 is
  `0b5861775b54940916affd77bd9566192bf8886fb869980df575b3fd1d0bbe78`
  and DAT SHA-256 is
  `4b7961f4737120e10cdd925b63a9ff1a84b73b078cd4088627787f8674a260fd`.
  Collector parsed/indexed all rows as snapshot
  `9e3645f9-e600-5ccc-8e30-030932e023d7` in the
  `org.junk.game.nes.ines-payload` v1 domain. No owned NES dump was supplied, so no
  match is claimed. The earlier Nintendo 3DS inspection file was not copied because
  that platform is unsupported.
- The later N64 form submission returned DAT-o-MATIC error 1260, `Unknown filter`,
  followed by a provider network refusal. Requests stopped immediately after the
  refusal was identified; no circumvention or alternate address was used. N64,
  SNES and PC Engine remain uncached. The script now replays only controls present
  in the exact system form and aborts the whole batch when refusal markers appear.

`scripts/cache_game_dats.py` is a manual allowlisted acquisition aid, not a server
refresh implementation. It performs no parallel requests, spaces ordinary requests
by a jittered 10–18 seconds, uses multi-minute generation waits, honors bounded
429/503/`Retry-After` backoff, caps HTML and ZIP responses, writes through `.part`,
reuses complete cache entries, rejects repository-local output, and enforces both a
cache-root process lock and a persistent cross-run request timestamp. Source Code,
Unofficial and unsupported systems cannot be selected through its manifest.

Checks and measured journey results on the corrected checkout:

- Offline saved-form tests passed for dynamic Prepare/Download controls, exact
  Headerless/full-release/licensed NES selection, Redump 975/10,976 filtering and
  immediate provider-refusal detection.
- `cargo test -p collector-providers --test dat`: eight passed, including explicit
  CUE retention and no-qualified-track fail-closed coverage.
- The two focused catalog module tests passed.
- `python3 scripts/check_architecture.py` and `cargo fmt --all -- --check` passed.
- `cargo test --workspace`: 299 passed, zero failed, one explicitly ignored owned
  oracle test; all doc-tests passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `npm --prefix web run check` and `npm --prefix web run build` passed.
- `cargo build -p collector-server` passed after the embedded web rebuild.
- `git diff --check` passed before the validation documentation append.
- The actual in-app browser showed the Redump snapshot, the honest historical
  partial result, the new exact result and automatic title. It also showed the NES
  snapshot's 2,661 parsed rows and exact digest. The restored server was stopped;
  ports 21841–21843 were closed at handoff.

The intended second physical LAN browser with trusted HTTPS remains absent, so its
Viewer playback, denied curation, grant revocation and active-session revocation are
still the blocking LA3 acceptance evidence. The DAT-o-MATIC refusal and incomplete
additional cache coverage remain documented limitations. No commit was made.


## Independent LA3 review — 2026-09-20

Reviewed the LA3 module anchor selection, application reconciliation, provider
repairs, catalog coverage, downloader and qualification records against the prior
LA2 review. Corrected a false-completeness path: malformed BINs or unknown payload
entries beside valid tracks could previously be excluded from catalog coverage.
The module now owns the narrow CUE exclusion rule; provider normalization and
matching both enforce it. Regressions cover malformed/empty/unknown tracks and
forged exclusions in otherwise matching rows.

The manual cache helper now requires explicit catalog selection, rejects new
DAT-o-MATIC requests before network I/O, validates form values and records submitted
controls rather than unconditional filter claims. It checks downloaded system/name
and version, bounds ZIP expansion, verifies existing cache digests and publishes new
entries from private staging. Existing snapshots cannot be force-overwritten;
refreshes use a new cache root. HTML refusal pages at download and HTTP-error stages
are checked too. Nine offline Python regressions pass, including a no-network
paused-provider check, failed publication, wrong-system download, ZIP bounds and
altered cache detection.

Independent checks on the final Rust source:

- `COLLECTOR_TEST_FLAC=1 cargo test --workspace`: 301 passed, zero failed, one
  ignored separately configured owned-oracle test; doc-tests passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- Architecture/documentation checks, Rust formatting, web type check/build,
  OpenAPI/TypeScript contract comparison and post-web server rebuild: passed.
- Existing Redump and NES cache entries passed fresh length/SHA-256 verification.
  Independent read-only XML inspection of the retained Redump catalog found 10,001
  rows, 46,552 sector-aligned BINs and 10,001 CUEs, all consistent with the tightened
  domain policy. This is catalog structure coverage, not a new owned-disc match.

No DAT-o-MATIC or other live provider was contacted. The previous manual
owned-dump/provider/restore runs remain dated evidence; they were not independently
repeated here. Second-device trusted HTTPS and Viewer access/playback/revocation
remain outstanding, so LA3 is still not accepted. No commit was made.

Both Chrome journeys passed on the rebuilt server with FLAC 1.5.0 and Google Chrome
153.0.8010.37: `npm --prefix web run test:smoke` and the sequential `LOCAL_SETUP=1`
variant. These use isolated temporary profiles. Actual playback, pause/resume,
seeking, track transitions, artwork rows, session expiry, grant revocation and
private-management routing passed. Browser execution required the existing
outside-sandbox launch permission. Final architecture/link checks and
`git diff --check` passed. Review logs are `/tmp/la3-review-final-workspace.log`,
`/tmp/la3-review-clippy.log`, `/tmp/la3-review-api.log`, `/tmp/la3-review-build.log`,
`/tmp/la3-review-smoke.log` and `/tmp/la3-review-local-smoke.log`.

## Web album follow-up — 2026-09-20

Read-only inspection of the local installation found album command receipts taking
707 and 722 seconds between creation and completion before their batches appeared.
Those requests subsequently produced durable track Jobs/outputs. The single FLAC
initially present predated the album button; it was not evidence that these newer
requests had already completed. The live server was not restarted during its work.

Music source selection was hashing the entire package on availability polls and
again per track during batch admission. It now resolves observations using the
committed source fingerprint. Actual source validation remains in the background
worker before encoding and publication. A regression admits an album with altered
payload bytes, then verifies that every track fails before encoder invocation and
no output is published. The existing cancellation/source-change/scratch-corruption
regression and eight album workflow tests pass.

Archived image choices now render authorized lazy thumbnails with labels and an
explicit unavailable-preview fallback. Remote CAA images still require the existing
acquisition action before an archived preview is available; listing a provider's
options does not silently download all originals. Preparation refreshes parent
Capture/Item displays, retries transient polling failures, and displays errors near
the preparation controls. Sticky close buttons no longer float across cleared
sections. Browser regression checks assert compact spacing after the track list;
the user's exact transient blank rendering was not independently reproduced.

The Chrome HTTPS smoke passed with actual FLAC playback, loaded archived previews,
a simulated transient availability failure, album preparation without an inventory
save, immediate sidebar refresh, reload persistence, and layout measurements. Web
type/build, generated contract comparison, architecture/links, Rust formatting,
Clippy across all targets, and post-assets server build passed. Logs are under
`/tmp/collector-web-fixes-*.log`. Live post-restart latency has not been measured;
second-device LA3 qualification remains outstanding.

Final checks: `COLLECTOR_TEST_FLAC=1 cargo test --workspace` passed 302 tests,
zero failed, one separately configured owned-oracle test ignored; doc-tests passed.
The sequential `LOCAL_SETUP=1 npm --prefix web run test:smoke` also passed,
including the same real playback and regression checks over localhost HTTP.


### Cover Art Archive chooser correction — 2026-09-20

The user's screenshot identified the provider-option cards, rather than the
already-archived image selector. Those cards now lazy-load 250px previews through
a new curation-only endpoint. The service verifies membership in the retained
listing before provider I/O. The existing CAA adapter supplies bounded transient
bytes, retaining the same redirect/DNS rules; four concurrent reads, disconnect
cancellation, a deadline, 1 MiB body limit and raster MIME restrictions apply.
Neither previews nor retrying them archive originals. Failed previews expose a
labelled Retry action, and explicit selection remains the acquisition command.
The generated API gains one endpoint (98 to 99 paths) and listing options carry
Snapshot identity; no archive schema or migration changes are needed.

Eight provider tests and all 28 HTTP tests passed, including preview endpoint
selection, oversized/non-image responses, cancellation during reading, redirect
rejection, and public/private exposure with spoofed proxy headers. Provider fixture
teardown now stops unused listeners and bounds socket reads/writes, so a setup
assertion cannot leave a test hanging. The local-HTTP Chrome smoke passed with
actual loaded thumbnails before acquisition, no Asset creation during preview,
unknown-image rejection, failed-image retry, Viewer denial, and the prior album
preparation/playback checks. These use local fixture providers, not live CAA or
DAT-o-Matic. Web type/build and regenerated contract comparison passed, and the
server binary was rebuilt. Logs: `/tmp/collector-caa-*.log`.
Final workspace Clippy (`--all-targets -- -D warnings`), formatting,
architecture/link and diff checks also passed.

## Folder intake, upload feedback and playback selection — 2026-09-20

The module tests cover nested CUE folders, multiple same-directory redumper dumps,
overlapping filename stems, uppercase TOCs, preserved sidecars, ambiguous CUE
ownership and missing companions. The application test exercises authorization,
disjoint discovery and unsafe/duplicate logical paths. HTTP coverage includes the
new discovery route and all public/private exposure combinations with spoofed
proxy headers. The exposure fixture supplies a valid Origin so it measures router
composition rather than the independent CSRF rejection.

Chrome 153.0.8010.37 passed the HTTPS/WebAuthn smoke with real two-package folder
selection, explicit unassigned files, no manual inventory form, automatic
publication, reopening durable history, and uninterrupted audio playback during
another import. Both upload interfaces show live progress; retained-prefix checking,
wrong-source rejection, interrupted transfer resume and lost-acknowledgement retry
passed. The earlier local-HTTP smoke passed the upload/progress regressions. A later
local run stopped because the configured FLAC probe timed out at startup; the final
HTTPS run qualified actual FLAC playback and the completed folder journey.

Web type/build, generated API comparison, architecture/link checks, formatting,
diff checks and workspace Clippy passed. The embedded server binary was rebuilt.
The folder screenshot exposed oversized checkbox layout; checkbox labels were
aligned afterward, and small transfers now display KiB rather than rounded zero
MiB. Logs: `/tmp/collector-folder-*.log`, `/tmp/collector-upload-smoke.log`.

Read-only inspection of the local OT0 archive found separate disc captures and
original packages, with prepared tracks retained on both. No real archive data was
changed by this work. Association changes retain the original inventory entry;
background notifications now refresh data without selecting another item and
unmounting its player.

Final Rust validation: the workspace run passed the client, core, module and provider
suites; after correcting the new exposure test's Origin, the server/store rerun
passed all 196 tests, including 28 HTTP and 152 archive tests. The long AccurateRip
workflow completed successfully. No test remains failing.

## Retire reassigned automatic placeholders — 2026-09-20

The ten guided-ingest archive tests passed. The association regression now checks
portable tombstones through reindex, recovery of the association/cleanup gap,
retention of wishlists and manual titles/authority, preservation of another Capture,
and writer-locked rejection of a stale item snapshot after an independent Capture
or authority change. Workspace Clippy, web type/build, formatting and architecture
checks passed. Cleanup uses existing archive publication and schema; it never
removes payloads. Original user data was not edited during validation.

The UI refreshes destination choices after reassociation and disables correction
until the choices load. The browser regression waits for the selected source option
before reading its ID, avoiding an empty select value during asynchronous loading.

Final local-HTTP Chrome smoke passed: reassociation returned a tombstone-backed
404 for the retired source, excluded it from browse results, kept both Captures on
the destination, and left unrelated real audio playing. The existing reindex and
Viewer checks passed afterward. Log: `/tmp/collector-retirement-smoke-verified.log`.
The rebuilt server includes the final chooser and retired-selection handling.

## Metadata candidate replacement and inspection — 2026-09-28

`python3 scripts/check_architecture.py`, `cargo fmt --all -- --check`,
`cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`,
`npm --prefix web run check`, `npm --prefix web run build`, and
`npm --prefix web run check:api` passed. The focused archive regression passed
after the final semantic JSON-equivalence adjustment. The Chrome smoke passed
with dataset inspection, two-way comparison, no-match replacement, unmatched
choice retention, re-open persistence and mobile overflow checks. The full
workspace suite passed with 179 archive tests; the private CD dump oracle remains
intentionally ignored without user-owned inputs. Raw provider observations remain
archived, while the browser receives only validated typed dataset details.

## Album-disc coverage and compact processing history — 2026-09-28

`python3 scripts/check_architecture.py`, `cargo fmt --all -- --check`,
`cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`,
`npm --prefix web run check`, `npm --prefix web run build`, and
`npm --prefix web run check:api` passed. The generated OpenAPI and TypeScript
contracts include the viewer-safe album-disc coverage route; the HTTP regression
checks its response and module-independent track availability. A focused resolver
test covers two release media, missing IDs, manual assignment, conflicts and
incomplete positions. The private real-CD oracle remains ignored without its
user-owned inputs.

The rebuilt server passed the full Chrome smoke: the imported two-track CD layout
appeared in album coverage before an explicit structure rerun, missing release
disc-ID linkage remained visible, technical evidence stayed collapsed until
opened, and prepared audio survived unavailable-module fallback. The separate
64 MiB processing browser regression also passed with repeat consolidation and
DAT ambiguity/no-match flows after the Advanced evidence disclosure change.
These fixture checks do not establish that the user's specific physical discs or
their audio match a selected release; that requires the recorded disc IDs and
per-source AccurateRip outcomes shown in the new checklist.

## Bulk import, metadata repair and music preparation — 2026-10-03

BL1–BL3 local acceptance and its limits are recorded in
[bulk library validation](validation/bulk-library.md), including the shared metadata
pipeline, durable intake, confirmed atomic grouping/undo, frozen FLAC batches,
shared cancellation, real browser playback and the generated 2,400-member run.
The [delivery ledger](bulk-library-plan.md) and handoff/status summaries are updated
together. Trusted second-device and wider deployment/scale qualification remain
separate; blocked DAT downloads remain disabled.

## Collection-list review repairs — 2026-10-04

The fixes and current regression/gate results are recorded in the dated follow-up
in [collection-list validation](validation/collection-lists.md). Chooser selection,
obsolete pagination, uncertain command receipts, shared dialog behavior and complete
disc expansion are repaired without changing the outstanding LA3 device gate.
