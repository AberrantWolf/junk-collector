# Usability and lifecycle fixes

Prepared 2026-09-21 at the user's request, combining the [18 UX findings](reviews/ux-2026-09-20/review.md) with related unfinished milestone work. **FX-01–FX-03 are implemented and locally accepted (2026-09-21). FX-04 local acceptance passed 2026-09-25. FX-05 Remove/Restore and clean reimport are implemented and locally accepted (2026-09-26). FX-06 is locally accepted (2026-09-26), and FX-07 local acceptance passed 2026-09-27.** This document defines a coherent fix set; it does not declare LA3 accepted or silently replace its outstanding second-device qualification.

## Intended outcome

A user can find and open an item, understand its current state, import or repair it, inspect useful results, and remove mistakes without accumulating duplicate scans or losing preserved material. Work survives navigation/restart; unrelated actions remain available. Common tasks do not require understanding Jobs, observation IDs, file indices or metadata-authority records.

Implement complete user workflows across application, storage, API and UI. Do not split each delivery into serial backend/frontend handoffs. Preserve existing working ingestion, preparation and publication services.

## What to pull forward from the milestones

| Existing work | Actual remaining scope relevant here | Destination |
|---|---|---|
| [U05.1](interface-readiness-plan.md) | Full operation readiness matrix beyond LA1: valid anchors/defaults, eligible combinations, prerequisite reasons, current source/configuration state | FX-02, FX-04 |
| U05.2 | Correct every implemented surface's confusing states, drafts, feedback, permissions and unnecessary blocking | FX-01 through FX-06 |
| U05.3 | End-to-end negative cases, roles, keyboard/mobile, reload/reconnect, actual evidence for every audited action | FX-07 |
| [P04.4](import-management-plan.md) | Search/filter import and attempt history; meaningful package/item names; dismissed history; stable pagination | FX-03 |
| P04.5 | Terminal dismiss/archive/restore, selected/bulk cleanup, cleanup-failure recovery; archiving mistaken published entries | FX-03, FX-05 |
| P04.6 | Full mixed-eligibility, cleanup/race/restart/authorization/browser completion matrix | FX-07, with tests in FX-03/05 |
| [Background-work plan](background-work-plan.md), broader U05 requirement | Submission versus execution state, independent queueing across analysis/providers/imports/admin, durable progress outside the originating dialog | FX-02, FX-03, FX-04 |
| U01/U02/S02 follow-up | These milestones were locally completed; the audit exposes remaining browser usability gaps in collection access, recovery-code redemption and recent authentication | FX-06; reuse existing security policy, do not reimplement it |
| U03/U04/D06 follow-up | These are implemented foundations: simplify copy/artwork/metadata editing and preserve their authority/privacy guarantees | FX-04/05 regression coverage |
| D07.4–D07.5/LA2 | Already reviewed/corrected: reuse lessons and mechanisms for equivalent active work, immutable outputs and durable shared ownership | FX-02/05; retain album/playback regressions |
| P01/P02/P03 | Already implemented publication, durable jobs, cancellation, staging and receipts supply the mechanism for new removal/reset commands | FX-02/03/05; targeted extensions only |

The new item-removal/clean-reimport and validation-result consolidation requirements are genuine added scope, not claims that an old completed milestone already supplies them. Native C03 browsing principles inform artwork, pagination and keyboard behavior; no native ticket is closed by web fixes.

## Delivery ledger and dependency order

| Delivery | User-visible result | Dependencies | State |
|---|---|---|---|
| FX-01 — Reliable browsing and navigation | Open the correct item immediately, return to results, and find entries in a useful order | Existing read services | Implemented; local acceptance passed 2026-09-21 |
| FX-02 — One scan, one useful result | Valid defaults, one equivalent active computation, result-first completed state, compact repeat verification history | Existing processing/jobs; durable equivalence contract recorded in architecture | Implemented; local acceptance passed 2026-09-21 |
| FX-03 — Work and history management | Identify, resume, filter, dismiss, restore and clean selected work; show meaningful analysis targets without losing useful results | FX-02 active-work/result semantics; existing guided history | Implemented and locally accepted 2026-09-21 |
| FX-04 — Focused editing and consistent intake | Focused views, separate import history/detail navigation, and consistent new/existing-entry intake | FX-02 readiness; FX-03 progress/history destinations | Implemented; local acceptance passed 2026-09-25 |
| FX-05 — Remove, restore and reimport cleanly | Repair or remove a messy item without duplicates, resurrected old work or lost originals | FX-02/03/04 and publication/recovery contract review | Implemented and locally accepted 2026-09-26 |
| FX-06 — Usable access and account recovery | Direct collection management, browser recovery and in-place reauthentication | Existing U02/S02 services; independent of FX-02–05 | Locally accepted 2026-09-26 |
| FX-07 — Whole-interface completion review | All implemented actions meet the readiness/lifecycle standard with recorded evidence | FX-01–06 | Locally accepted 2026-09-27; independent release qualifications remain separate |

Recommended sequence: FX-01 → FX-02 → FX-03 → FX-04 → FX-05 → FX-06 → FX-07. FX-06 can be brought forward after FX-01 because it has no scan/cleanup dependency. Each implementation delivery includes its own real UI acceptance; FX-07 finds cross-workflow omissions rather than postponing testing.

## FX-01 — Reliable browsing and navigation

**Covers:** UX-01/02/04/10/11, failed-artwork labeling, relevant U05 browse/accessibility criteria.

- Fence obsolete evidence/detail requests so an earlier item cannot replace the selected item's data. Distinguish loading, failed, absent, unavailable and stale results.
- Use an immediately visible detail route/view on narrow screens, with focus transfer, a back action and list-position restoration. Keep desktop detail accessible when selecting a row far down the list.
- Encode collection, search, sorting and item context in URLs; support reload, browser Back/Forward, and authorized deep links. Changing collection/search must resolve out-of-scope selections consistently.
- Add stable server-paginated sorting by title and recently added, with applicable artist/platform sorting and a bounded initial filter set: wishlist, files attached, playable, needs attention. Define each filter against authoritative state; do not fetch the whole library or infer readiness from strings.
- Keep artwork beside text; unavailable previews get retryable failure states rather than “No art.” Preserve missing-module historical reads and private-field redaction.

**Owners:** application read queries/DTOs, store projections, app navigation and detail UI. Index only the queries introduced here; this does not require P06's startup rewrite.

**Acceptance:** delayed A/B evidence responses; 390px and desktop selection with multiple pages; deep-link reload and Back restoration; sort/filter pagination without duplicates or dropped rows; unauthorized/missing item; a generated larger-library fixture proving bounded reads and no payload hashing. This is not R02's formal 100k benchmark.

## FX-02 — One scan, one useful result

**Covers:** UX-16/17/18 scan portions, U05.1 operation matrix and application-wide nonblocking work.

The application owns a typed operation-state/readiness response. Exact names should be chosen against the implementation; clients need at least the semantics below.

| State | Main presentation/action |
|---|---|
| Not checked, eligible | Explain scope and offer the check |
| Missing/ambiguous prerequisites | Explain requirements; allow valid alternative work, including preserving unanalyzed bytes |
| Queued/running/cancelling equivalent work | Progress/current work; no duplicate start; authorized cancellation where applicable |
| Completed for current inputs | Outcome, coverage and last actual check time; View result first; secondary Check again |
| Failed/interrupted | Safe diagnostic and resume/retry where eligible |
| Inputs materially changed | Explain changed source/settings/reference/version and offer the new check |

- Select a unique eligible CUE/TOC automatically; never default to file zero or offer arbitrary package files as anchors. Resolve companion requirements, module capability, snapshot, structure result, encoder/provider configuration, and role through the owning services. Revalidate at submission.
- Establish canonical active-work identity from authorized scope, committed source fingerprint, operation/version, relevant settings and frozen reference input. Atomically converge equivalent requests even with different command keys or from different tabs; keep durable request ownership and submitter-private views.
- Reuse existing job mechanisms and D07's shared-ownership lessons. Cancelling one owner must not cancel another's required work. Cancelling work remains active until settled; do not start its duplicate in the gap.
- Distinguish request receipt, active computation, substantive result content and occurrence identities. These are different concepts, even if implemented with a small number of records.
- Store one substantive equivalent result and lightweight actual-check occurrences (first/last dates, count and optional expanded dates). Equivalence includes source, algorithm/version, normalized inputs, actual coordinates/coverage, reference content and full outcome/limitations. Exclude incidental timestamps/request IDs. Preserve distinct provenance when content is shared.
- Changed bytes, reference data, algorithms or partial coverage remain distinguishable. A common “match” label is not equivalence. Cached reuse must not update the last actual verification date. New provider acquisition dates are not new byte-verification dates.
- Group pre-existing duplicate observations through rebuildable projections; retain their original immutable records. Evolve new storage explicitly with recovery/conversion rules for actual data, without a checkpoint-compatibility wrapper chain.
- Return promptly after durable admission. Poll/reopen current work without holding a whole-dialog busy flag. Browse, edit unrelated records, play existing tracks and submit different work while a scan runs. Browsing/admission uses recorded fingerprints, not disc-sized reads.

**Owners:** modules define valid operation inputs and evidence semantics; application owns readiness, equivalence, ownership and commands; store owns atomic admission/result publication/projections; HTTP/React consume generated contracts. Update architecture before introducing the durable result/occurrence contract.

**Acceptance:** concurrent different-key equivalent requests run once; reopened/restarted UI shows that work; later direct ownership survives a parent's cancellation; repeat actual scan with advancing clock produces one result plus another occurrence; cache reuse does not advance dates; v2-only/partial/changed-reference cases stay distinct; known-invalid API/UI submissions create no job; unrelated work remains usable during a deliberately slow real operation.

## FX-03 — Work and history management

**Covers:** UX-12 and operational-history portions of UX-16, the remaining P04.4/P04.5 scope, and the user-reported repeated derivative analysis forms (2026-09-21).

- Add an accessible Work & history destination, with item-specific views and links from progress notices. Show package/item/collection names, scope, submitted time, stage, outcome and safe diagnostics.
- Group import staging, analysis, publication, enrichment and preparation under the requested workflow. Expand attempts only when useful. Show current successful scan results separately from failed execution attempts.
- Keep analysis centered on meaningful source packages. Do not render a CD-inspection form for each generated FLAC/Opus track when the operation cannot apply. Use module-owned eligibility and representation context; retain actionable blocked states for incomplete original packages and authorized historical results even when new operations are unavailable. Group generated tracks under playable files with track number/title or filename, format and useful status instead of identical internal-role cards. Preserve files, playback and preparation behavior.
- Implement server-side search/filter by collection/item or unattached state, package/file name, dates and lifecycle status; bounded pagination must retain loaded pages across polling. Reopen older work without local-storage dependence.
- Provide terminal-history dismiss/restore and an archived view. Support selected/bulk cleanup with an exact target set, eligibility, byte impact and per-target outcomes; a partial batch must not look wholly successful.
- Distinguish **Dismiss history**, **Discard unpublished import**, and **Cancel and clear unfinished work**. Reclaim eligible staging through durable cleanup after cancellation settles. Show actual reclaimed/pending bytes and retryable cleanup failures. Keep originals and successful evidence.
- Fence stale upload/review/retry/publication after discard. A committed publication wins over staging discard and remains recoverable. Keep minimal recovery/audit state without restoring dismissed clutter on restart.
- Retain existing submitter scope and current collection authorization. Shared work can expose safe progress appropriate to the actor without leaking another requester's job details.

**Owners:** existing P02/P03/P04 application lifecycle services and store adapters; one UI over typed workflow projections. No new scheduler or C06 event prerequisite. Reuse FX-02 readiness and existing track availability/preparation services for the derivative presentation fix; it does not depend on FX-04’s editor restructuring. Deliver working behavior across required layers with no dummy services or placeholder actions.

**Acceptance:** the original P04 three-import/two-failed-attempt case; search/reopen after reload; stable older pages while polling; mixed-eligibility bulk cleanup; cleanup failure/retry/restart with measured bytes; last-owner cancellation and commit races; cross-user/collection denial; keyboard focus and mobile controls. Include a real multi-track music item: one appropriate disc-analysis target, no repeated ineligible derivative forms, identifiable grouped generated tracks, retained historical evidence, actionable incomplete-original readiness, and working playback/preparation after reload.

## FX-04 — Focused editing and consistent intake

**User requirement 2026-09-22 — separate views at collection scale:** Import history
and the selected import's details must be separate navigable views. Do not append
current-import details below the history list or put the full list inside the
current-import panel. FX-04 implements this additional navigation requirement;
FX-03's earlier acceptance did not include it. Apply the same principle
to other overloaded views when the data or task warrants a distinct destination.

**Observed selector defects (2026-09-22, corrected in FX-04):** `guided-ingest.tsx`
loads collection entries in pages of 100 when opening an import, stops at 1,000,
and puts those general entries into the attachment dropdown. They are not detected
matches; entries beyond the cutoff are unavailable there. The manual-import
history destination picker searches up to 50 entries but exposes no next page.
The audit must address both collection preloading and silent truncation.

**User addition 2026-09-22:** A persistent bottom music player with a browser-local,
per-user queue is implemented and locally accepted ahead of the rest of FX-04. Album info views add
tracks/albums, while shell-owned playback survives navigation and processing
refreshes. No server queue/activity persistence is added. The remaining editing and
intake scope below is unchanged.

**Covers:** UX-07/08/09/13/14, metadata/artwork/cost/lookup polish, broader U05 interface audit.

- Give Work & history its own bounded, searchable list view for finding/selecting work and bulk actions. Opening an import replaces that list with a focused detail view containing only that import's progress, results, relevant attempts and available actions. Intake/current-import progress must not embed global history. Provide an explicit Back to history action and browser Back/Forward; preserve list filters, search, loaded pages, selection and scroll without mounting the list beside/below the detail view. Support direct links and reload of a selected import through its authorized server identity.
- Review view boundaries across the workbench, imports, processing/evidence, preparation history, metadata/artwork and administration. Separate browsing a set, inspecting one record and performing a task when they represent meaningfully different user intents. Tabs or collapsed sections within an ever-growing panel are not sufficient when a separate view is warranted. Keep local context and next actions visible without requiring users to scroll past unrelated records or forms; avoid splitting simple cohesive tasks unnecessarily.
- Replace the existing-entry attachment dropdown with an explicit destination chooser. Keep evidence-backed suggested matches separate from **Search existing entries**; label the basis and uncertainty of each suggestion. General collection results must never look like detected matches, and lack of matching evidence must be stated honestly. Do not invent a matching service or infer identity from a title alone. Searching remains available when there are no suggestions.
- The destination chooser searches the authorized collection on the server with bounded pages, visible loading/error/empty/more-results states and cancellation of obsolete queries. Show artwork, title, artist/subtitle and useful edition/disc context to distinguish similarly named albums. Keep the selected destination visible independently of the result page, fetch/check its current revision before applying, and clearly distinguish attaching, leaving unattached and creating an entry. Opening import details must not preload the collection just to populate a closed chooser. Neither a giant dropdown nor a silently truncated first 50/100/1,000 records is acceptable.
- Audit every selector/list backed by a growing user dataset, including existing-entry association, manual-import destinations, inbox packages, import/job/preparation histories, catalog snapshots and account/access lists. Record expected cardinality, query/page bounds, whether results are complete, and the way a user finds a record outside the first page. Use searchable paged choosers or separate browse views as appropriate; reserve ordinary dropdowns for genuinely small fixed choices. Route access-specific corrections through FX-06, and retain the full scale audit in FX-07. A client-side search over a preloaded collection does not meet this requirement.
- Replace the single long workbench with navigable task sections: Details, Copies, Files/import, Music, Artwork, and advanced Evidence. Open the section implied by the initiating action. Collection grants move to FX-06's collection settings.
- Make ordinary title/subtitle editing one clear user task; reveal automatic/manual provenance and history progressively. Keep per-section save boundaries explicit and place success/failure feedback beside the action.
- Share the intake journey for a new entry and an existing entry: destination → package/source → visible readiness/options → submit → durable progress/result. Use the same application coordinator; preserve meaningful advanced association/review controls.
- Make provider metadata lookup automatic once supported CD analysis has produced usable disc identity, across both new-entry and existing-entry/manual intake. Use the actual parsed CD layout/TOC and derived provider disc identifier through the existing application-owned acquisition pipeline. Never automatically search a dump filename, folder name, timestamp or a display title inherited from them (for example `260919_134632_disk7`). Such text is a source label, not album identity. Existing guided ingestion already uses the derived MusicBrainz disc ID; extend/reconcile that behavior across the remaining paths rather than adding a second frontend lookup pipeline.
- Present automatic lookup as durable progress and results, not an ordinary task requiring the user to press Find metadata. Admit work when its analysis prerequisites become ready, with duplicate prevention, restart recovery and cancellation through existing services; opening or refreshing a view must not trigger repeated acquisition. Missing/unsupported disc information, no match, ambiguity and provider/configuration failure must remain distinct and visible. Do not silently fall back to filename-based text search or block original preservation. Keep intentional retry/refresh and advanced user-entered text/release-ID lookup available as recovery tools, clearly distinct from automatic disc-based lookup. Preserve manual metadata/artwork authority and provider observations; provider identity is not catalog/checksum verification.
- Surface missing companions and important limitations above the main action. Explicitly offer preservation without analysis when allowed. Clearly distinguish originals preserved, metadata found and audio ready.
- Preserve drafts across mode/section changes; collection changes reconcile incompatible fields with explicit notice/confirmation. Include file previews and artwork-only selections in draft protection. Do not make “close progress” mean cancellation.
- Make automatic artwork selection save consistently; fix custom lookup types and stale values. Use currency-aware amounts rather than minor-unit input. Show role-aware next steps instead of pointing viewers to hidden controls.
- Inspect every remaining global busy state, including catalog/provider acquisition. Guard conflicting submissions only; durable execution must not block unrelated editing/navigation.
- Keep encoder/provider configuration limitations honest: actionable configuration guidance and readiness are included; browser editing of server executable paths is not added by this delivery.

**Owners:** existing curation components and application intake/metadata services. Consolidate UI orchestration through current services; do not duplicate parsing/provider/publication policy.

**Acceptance:** with at least 3,000 catalog entries and 3,000 import-history entries, find and open old and current imports without rendering the whole history or scrolling past it. The selected-import view must contain no global history list. Verify bounded loading, direct-link/reload, browser Back/Forward, restored list filters/selection/pages/scroll, bulk actions from the list, and independent polling of the selected import on keyboard and mobile. Find and attach to an entry beyond the old 1,000-entry cutoff; distinguish same-title albums using artist/edition/artwork; exercise genuine suggestions separately from general search, no suggestions, no search results, later pages, rapid query changes, stale selection/revision, revoked access and keyboard/mobile selection. Verify that opening a current import neither downloads nor mounts the full catalog, and that no result cap silently makes the intended destination unreachable. Also correct a title, add a copy/loan and image, choose automatic/manual cover, attach a second disc to an existing album, and import a new item through real UI. Import a timestamp-named redumper package such as `260919_134632_disk7` and assert that automatic provider requests use the derived disc ID, never the filename or placeholder title, without an explicit lookup click. Exercise both new and existing entries, delayed analysis completion, reopen/restart, no duplicate requests after UI refresh, missing disc identity, no match, ambiguous releases, provider failure and retained manual overrides. Cover missing companions, ambiguous anchors, unavailable tools/provider, role denial, lost connection, draft mode switches, immediate refresh, foreign text, keyboard/mobile, and independent actions during accepted work.

## FX-05 — Remove, restore and reimport cleanly

**Covers:** UX-15 and the mistaken-published-item portion of P04.5; new user-requested lifecycle scope.

- Add **Remove item** and a discoverable removed-items view with Restore. Remove from ordinary browsing/search; explain that preserved bytes/history remain. This is separate from recording physical disposal or dismissing a job.
- Define how removed items interact with captures, shared assets, grants, active imports and publication before coding. Deny new conflicting work and settle/cancel obsolete owned requests through existing lifecycle commands. Rebuild/restart must retain removal state.
- Add **Reimport cleanly** for the same item identity. Use retained originals or a selected replacement package. Present exactly what will reset: operational history/current automatic results and optional metadata/artwork overrides. Preserve manual metadata, physical inventory, loans and selected artwork by default; reset manual choices only when explicitly selected.
- Retained bytes can be reused without browser reupload, but background consumption still validates them; no claim of fresh integrity comes from a stored fingerprint alone.
- Introduce a durable workflow generation or equivalent publication fence so late old work cannot become current after reset. Commit request ownership and child admission under the same applicable claim/transaction boundaries. Shared work survives other owners' requests.
- Keep the last usable item/capture state until replacement succeeds, then publish the new current association atomically/recoverably and archive the old operational view. Failed replacement retains a resumable state; it does not create another visible placeholder item.
- Retain original evidence and provider observations. “Clean” means an understandable fresh working state, not silent loss of historical provenance. Actual staged-byte cleanup follows FX-03, separately from published masters.

**Owners:** application item/import lifecycle, existing archive journal and private coordinator, metadata authority and reference-safe storage mechanisms. This requires an architecture/contract review at the start, not a direct `deleted` field edit from React.

**Acceptance:** remove/restore an item with files/art/copies/loans; reimport a deliberately messy item with failed, duplicate and unfinished work; preserve default manual choices; test explicit reset options; restart at each commit/cancellation boundary; old completion cannot reappear; shared requests survive; failed replacement retains the old playable state; reindex produces the same visible/removed/current history.

## FX-06 — Usable access and account recovery

**Covers:** UX-03/05/06 and U01/U02/S02 browser follow-up.

- Make Manage collections open collection settings/access independently of any selected item, including empty and content-read-only collections. Keep content permission separate from permission to manage grants.
- Add signed-out recovery-code redemption using the existing recovery service and its returned session semantics. Provide safe code copying/downloading and explicit guidance before dismissing the one-time display.
- Handle recent-authentication-required centrally: retain the draft, offer password/passkey authentication, then return to the pending action. Follow the existing ceremony/session contract and clearly expose whether the action still needs submission.
- Keep secret-bearing input in memory, never URLs/local storage/logs. Preserve exact-origin, exposure, rate-limit, last-owner/admin and role rules. Put permission and success/failure feedback at the affected form.

**Acceptance:** recover in the real browser; consumed/invalid code and concurrent replay; advance server clock past freshness; return to preserved grant/security draft; cancel reauthentication; session revocation; empty-collection access; administrator without collection rights; public/private exposure; virtual WebAuthn plus explicitly unclaimed external hardware qualification.

## FX-07 — Whole-interface completion review

**Covers:** U05.3 and P04.6, not merely the existing 18 findings.

Audit dataset cardinality across every list and selector, not just the main album browser. With thousands of records, verify bounded server queries/rendering, explicit completeness/pagination, reachable records beyond the first page, useful disambiguation and honest suggestion-versus-search labels. Include attachment destinations, manual imports, inboxes, preparation/history, metadata choices, snapshots and access management; a search box alone is not proof of scale readiness.

Audit the information architecture as well as individual controls: identify screens that mix collection-wide lists, record details and editing/progress tasks. Verify that meaningful separate views are provided, that return navigation preserves context, and that thousands of records never turn a focused task into a long scrolling panel. Recheck the import list/detail separation delivered in FX-04; pagination alone does not satisfy it.

Maintain a checklist for every implemented action: owner, default, prerequisites, invalid combinations, pending/completed/error state, actual consequence, draft behavior, recovery path, permissions and test evidence. Include setup/login, collections/search, inventory/copies/loans, artwork/metadata, imports/scans/history/cleanup, removal/reimport, music/playback/downloads and administration.

Run combined journeys: fresh setup to import/play; incomplete package to repair; no-match/ambiguous verification; repeated equal scan; concurrent equivalent and independent work; clean reimport with interruption; failed cleanup/retry; item removal/restore; owner/viewer and access revocation; stale editor and session; mobile/keyboard/reduced motion; reload/offline/reconnect. Use actual media playback in the affected music journey, not DOM presence alone.

Run the repository gates at each integrated implementation boundary: architecture, API contract generation/check when contracts change, fmt, workspace tests, clippy, web check/build, rebuilt embedded server, relevant real-browser suites. Record exact results, no broad claims from a green happy path. Documentation-only planning does not require these implementation suites.

Close U05/P04 criteria only when the full applicable checklist passes. Retain independent accessibility/security and physical-device/hardware/deployment qualifications as open where unavailable. LA3's remaining trusted second device is a separate recorded gate, not something this audit can simulate into completion.

## Coverage and explicit deferrals

All audit findings are assigned: UX-01/02/04/10/11 → FX-01; UX-16/17/18 → FX-02 plus history cleanup in FX-03; UX-12 → FX-03; UX-07/08/09/13/14 → FX-04; UX-15 → FX-05; UX-03/05/06 → FX-06. Six smaller findings are covered by FX-01/04. FX-07 audits additional actions rather than treating this list as exhaustive.

- **P05 export/full backup tooling remains paused/outside this fix set.** Use the existing stopped-server backup/restore runbook for actual-data changes and test temporary roots. Preserve records needed by future export. Full P05.1–P05.3 is useful later, but not a dependency for reversible removal or history cleanup.
- **Permanent deletion of published payloads is excluded.** It needs a separate reference/retention/recovery design; none of these controls should imply disk reclamation for preserved masters.
- **P06 full incremental startup and R02 formal scale/deployment remain deferred.** Add bounded indexes/query tests needed by these screens without absorbing the entire storage/performance milestone.
- **C06 events remain deferred.** Correct polling, pagination and durable reconciliation suffice for this delivery.
- **Native/offline clients, Trash Amp, TLS rotation/discovery, legacy migration, new providers/formats and document attachments remain deferred.** Their absence must remain honest in the UI.
- **Completed D06/D07/LA1/LA2 are reused, not restarted.** Revalidate affected behavior without claiming new external qualification.

## Implementation handoff

FX-01–FX-04 are locally accepted. FX-04 was implemented in the 2026-09-24
checkpoint; see its [validation record](validation.md#fx-04-focused-editing-and-intake--2026-09-24)
and [growing-list audit](fx04-scale-audit.md). FX-05 now includes Remove/Restore,
retained-original and replacement-upload reimport, reviewed authority resets,
recoverable capture switching, generation fences and continued queued playback.
Local acceptance passed on 2026-09-26; see [validation](validation.md) for the exact checked scope and limitations.
Preserve local user data and configuration. FX-06 and FX-07 are locally accepted. The user-requested [work-context hardening](work-context-hardening.md) is locally accepted (2026-09-26). The outstanding LA3 second-device gate remains separate.

At completion of each delivery, update this ledger, implementation status, the mapped U05/P04/background-work criteria, validation evidence and the next ready-to-send prompt together. Retain dated earlier acceptance records and outstanding LA3/external gates. For FX-02 and FX-05, document durable identities, publication fences, cancellation/ownership and restart recovery before changing their subsystem boundaries.

**FX-07 local acceptance, 2026-09-27:** Administrator accounts now use an authorized,
bounded, searchable keyset query, with later-page and direct-record regression
coverage. Personal sessions, passkeys and paired devices have bounded pages;
metadata candidate rendering is progressive within its per-Item limit.
Administration and security have focused tasks; catalog snapshot rows and completion
refreshes follow their selected collection. Collection navigation and selectors
now use an authorized, role/module-filtered server query with 50-row pages,
search and an exact selected-collection read; the unbounded collection-list
HTTP endpoint was removed. Metadata completion now carries its actual refresh
outcome; the real-browser no-match fixture retains earlier ambiguous choices.
The collection search debounce race and hidden catalog lookup recovery reported
in review are corrected. A real-browser three-import journey shows two
successful analyses, two failed attempts, actual cleanup fault/retry and
restart reclamation. Offline DAT fixtures display ambiguous and no-match
verification. The [action, scale and combined-journey audit](fx07-interface-audit.md)
and [validation](validation.md) support local U05.2/U05.3 and P04.6 acceptance.

**Next ready-to-send prompt:** Continue the remaining trusted second-device
[LA3 qualification](local-alpha-plan.md#ready-to-send-la3-prompt). Use the
intended second device and record actual LAN playback, download and trust
behavior. Keep independent accessibility/security and physical deployment
reviews in their release gates. Page metadata candidate responses before
raising the current 4,096-version per-Item admission ceiling.
