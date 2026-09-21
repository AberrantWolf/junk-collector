# UX review — 2026-09-20

The largest usability problems are navigation and state continuity. Routine tasks expose much of the archival model at once, while several transitions lose context or display information belonging to an earlier selection.

This is an audit of the current working tree, including its uncommitted implementation. No application code was changed. Findings describe implemented behavior; they are not claims that the entire release or native clients have been evaluated.

## Evidence and scope

- Read the browser application, curation, guided intake, package attachment, physical inventory, artwork, metadata, music preparation/player, job ledger, administration, security, and relevant backing services and documentation.
- Exercised a real temporary server/profile in Chrome 153.0.8010.37 at 1440×1000 and 390×844. Created two collections, an inventory entry, recovery codes, a catalog snapshot, and 20 additional fixture entries for navigation checks. Temporary server/profile were removed afterward.
- Compared served `app.js`, `curation.js`, and `app.css` with current `web/dist`; all matched.
- Used controlled response delays and synthetic evidence labels to reproduce the item-switch race. That test proves incorrect attribution in the UI, not corruption of stored evidence.
- Tested an incomplete CUE through the real import preview. No original user collection or media was modified.
- Did not rerun the full Rust/browser regression suite, encode/play a complete album in this audit, qualify another physical device, or perform a complete screen-reader audit. Source-confirmed findings below are distinguished from browser reproductions.

[Browser observations](browser-observations.json) · [Mobile selection screenshot](mobile-selected.png) · [Collection-switch screenshot](collection-switch.png) · [Editor screenshot](editor-desktop.png)

## Prioritized findings

P1: incorrect information or a major obstacle to a primary task. P2: material friction, confusing behavior, or a missing recovery path.

### UX-01 · P1 · Evidence can belong to a different item

**Browser reproduced.** Open item A, then item B before A's evidence request returns. If A returns last, B's title remains visible while A's evidence replaces B's. I reproduced “FIRST ITEM EVIDENCE” under “Album 00.”

`Detail` is reused across selections and its evidence effect has no abort, generation check, or cleanup. This is especially damaging in a product whose purpose includes preserving trustworthy evidence.

**Change:** cancel obsolete reads and bind results to the selected item. Distinguish loading, failure, and truly empty evidence; currently the empty message also appears while loading or after a failed fetch. Keep integrity, structure, and catalog/audio verification labels distinct.

**Acceptance:** deliberately complete A's request after B's; only B's evidence may appear. A failed read must not imply that no evidence exists.

Source: `web/src/app.tsx:125–129`.

### UX-02 · P1 · Opening an item appears to do nothing on mobile

**Browser reproduced.** With 21 entries at 390×844, tapping the first row left focus on that row and the scroll position at zero. The details began at y=2522, below the complete list. No visible transition announced where the item had opened.

The layout becomes a single column below 1150px, so this also affects tablets and narrower desktop windows. More entries push the detail farther away.

**Change:** use a dedicated detail view or an accessible sheet on narrow screens. Provide a clear return to the previous list position. On wide screens, keep the detail visible when opening a row far down the list.

**Acceptance:** opening any visible row immediately presents its details, moves focus appropriately, and returning restores the list position.

Source: `web/src/app.css` responsive `.browser:has(.detail)` rules; `web/src/app.tsx:102–103`.

### UX-03 · P2 · “Manage collections” does not open collection management

**Browser reproduced.** With an item selected, the button opens “Edit entry.” With no selection, it opens “Add or import.” Collection access is hidden until there is an item or the user chooses “Enter inventory manually,” then appears at the bottom.

An owner trying to grant access to an empty collection is led through an unrelated inventory form. The same navigation label changes meaning with an otherwise incidental selection.

**Change:** give collection settings/access a dedicated entry point bound to the current collection. Keep item editing and intake separate.

**Acceptance:** an empty collection's owner can reach its grants directly without opening an item or a creation form.

Source: `web/src/app.tsx:75–84,97`; `web/src/curation.tsx:58`.

### UX-04 · P2 · Changing collections leaves the old item open

**Browser reproduced.** After opening “A test pressing” in the music collection, selecting the empty game collection changed the heading and empty state but retained the music item's detail and edit button.

The screen now presents two conflicting collection contexts, making it easy to edit the previous selection unintentionally. Searching has the same underlying selection-continuity problem.

**Change:** close a detail that falls outside the new browsing scope, or explicitly present a persistent item workspace with its collection identity. Apply a consistent rule to collection changes and searches.

Source: `web/src/app.tsx:36,97,100–103`.

### UX-05 · P2 · Recovery codes have no browser recovery flow

**Browser and source confirmed.** Account & security generates codes, but the signed-out screen offers only password and passkey sign-in. There is no “Use a recovery code” action, despite an implemented `/auth/recovery` endpoint. The README instead directs local recovery to a stopped-server command.

**Change:** add a signed-out recovery form accepting the account, recovery code, and replacement password. Explain the effect on sessions and return the user to sign-in. Add a convenient way to copy/download newly generated codes and warn before dismissing the only display of an unsaved batch.

**Acceptance:** a user holding a code can recover using the browser without developer tools or server shell access.

Source: `web/src/app.tsx:118–122`; `web/src/security.tsx:17,21–27`; `crates/collector-server/src/lib.rs:896`.

### UX-06 · P2 · Security operations request reauthentication without providing it

**Source confirmed; the five-minute timeout was not waited out in this audit.** Sensitive commands require authentication within five minutes. The server returns “Reauthenticate before changing security or access settings,” but the UI handles this as an ordinary error. It offers no in-place password/passkey prompt or continuation.

The practical workaround is to close the panel, sign out, sign in, navigate back, and reconstruct the attempted change.

**Change:** centrally handle `recent_authentication_required`, retain the draft, reauthenticate, and return to the pending action. Do not automatically repeat a destructive change without making its pending state clear.

Source: `crates/collector-app/src/lib.rs:918`; `crates/collector-server/src/lib.rs:332`; `web/src/security.tsx:13`; `web/src/administration.tsx:25`; `web/src/curation.tsx:36–39`.

### UX-07 · P2 · A basic edit opens a very long workbench

**Browser measured.** A music entry with no captures or physical copies produced 4671px of editor content inside a 900px viewport. Music preparation and metadata review precede the ordinary title/artist fields. Inventory, artwork, package attachment, jobs, and access management follow in the same scroll container.

There are multiple save boundaries and two title-editing surfaces: “Displayed title”/“Save title override” and “Title”/“Save inventory.” Users must understand metadata authority before they can confidently make a simple correction.

**Change:** create task-oriented sections such as Details, Copies, Files, Music, Artwork, and Advanced evidence. Open the relevant section from the initiating action. Give ordinary descriptive editing one clear primary form, with provenance and automatic/manual authority available progressively.

**Acceptance:** changing a title or attaching a file is immediately discoverable without scrolling through unrelated workflows; every section clearly identifies what its save button commits.

Source: `web/src/curation.tsx:58`; `web/src/metadata-review.tsx:102`; `web/src/music-tracks.tsx:197–209`.

### UX-08 · P2 · Import warnings are hidden behind optional details

**Browser reproduced.** Previewing `missing.cue` without its referenced BIN showed “1 dump packages found” and an enabled “Start import.” The actual warning, “The CUE has no companion data file,” was inside a closed “Package details and import options” disclosure. The preparation checkbox was disabled, with the explanation also inside that disclosure.

Preserving incomplete input may be intentional, but the decision should be explicit rather than discovered after import.

**Change:** show blocking and limiting conditions above the primary action. Summarize how many packages can be preserved, analyzed, enriched, and prepared. When preservation remains allowed, make “Preserve without analysis” an understandable choice. Keep routine package inventories collapsed.

Source: `web/src/guided-ingest.tsx:124–130`.

### UX-09 · P2 · Switching intake modes silently discards the preview

**Browser reproduced.** Select files, preview an import, choose “Enter inventory manually,” then return to “Import dumps.” The selection and plan disappear without warning. The conditional render unmounts `GuidedIngest`; changing collection also replaces it through its key.

**Change:** retain the draft at the parent level, or ask before discarding it. Apply draft protection to mode/collection changes as well as closing the dialog. Guided intake currently marks active uploads but does not mark a prepared, unsubmitted selection as an unsaved draft.

Source: `web/src/curation.tsx:56,58`; `web/src/guided-ingest.tsx:18–27,110`.

### UX-10 · P2 · Browsing has no useful address or history

**Source confirmed; collection/item navigation stayed at `/` in the browser.** Collection, search, selected item, and management state exist only in React state. Reload loses the browsing context; an item cannot be bookmarked or shared by its current page URL; browser Back cannot serve as “back to my results.”

**Change:** introduce collection/item URLs and search parameters, with browser history and list-position restoration. Keep authorization on every read as today.

Source: `web/src/app.tsx:34–37,97–103`.

### UX-11 · P2 · The library is ordered by opaque IDs

**Browser and source confirmed.** Sequentially named fixture albums appeared in an apparently arbitrary order. The store sorts browsing and search by item ID, and the UI provides no sort or filters beyond collection and free text.

This becomes awkward once users have more than a small shelf: neither alphabetic scanning nor “find the thing I just imported” is reliable. Wishlist status is displayed but cannot be used as a filter.

**Change:** offer title, artist/platform, and recently added sorting with stable pagination. Prioritize useful filters such as wishlist, files attached, playable, and needs attention according to implemented capabilities.

Source: `crates/collector-store/src/lib.rs:2645–2650`; `web/src/app.tsx:100–102`.

### UX-12 · P2 · Background jobs lack enough context to act confidently

**Source confirmed; unrelated catalog work also appeared in the test item's editor.** `JobLedger` requests `/jobs` without an item filter, but rows show job kind, phase, state, percentage, and attempt—not their item, collection, or package. Generic cancel/retry buttons sit inside an item editor even when the job is unrelated.

Loading older jobs appends them, but the next scheduled refresh replaces the list with the first page. Browsing history can therefore undo itself.

**Change:** give background work its own accessible overview; show the affected item/package, submission time, meaningful progress, and a link to the result. Scope the editor's job list to that item. Merge refreshed statuses without discarding loaded pages.

Source: `web/src/job-ledger.tsx:17–26,41–56`; `web/src/curation.tsx:58`.

### UX-13 · P2 · Success and failure messages can be far from the action

**Source/layout confirmed.** Administration renders feedback after collections, encoder instructions, catalogs, accounts, and exposure information. Creating a collection near the top can produce its result at the bottom. Curation does the inverse: inventory/image/access feedback goes near the top even when the action is far below it.

ARIA live regions help announcements, but sighted users can still be left wondering whether a click worked. Some broad success copy also suggests a next step unrelated to the action just completed.

**Change:** put feedback beside the initiating form, retain errors until addressed, and provide a visible destination after creation, such as “Open collection.” Reserve a global banner for global failures.

Source: `web/src/administration.tsx:29,36–42`; `web/src/curation.tsx:45,52–58`.

### UX-14 · P2 · Adding files means two different workflows

**Source confirmed; both entry surfaces inspected.** New-item intake offers guided discovery, automatic enrichment, and optional FLAC preparation. “Edit / attach files” instead presents “Capture accession,” then Stage package → Review import → Save review → Publish capture; metadata remains an explicit later operation.

The backend distinction is legitimate, but the UI makes attaching a second disc substantially harder than importing a first disc. Returning users must learn both the task vocabulary and different completion expectations.

**Change:** expose a shared user-facing intake sequence with a clear destination: new entry or existing entry. Keep detailed association/review controls under advanced options where necessary. Show a single understandable result: originals preserved, metadata status, and playable tracks status. Preserve the existing service boundaries rather than introducing another browser-owned processing pipeline.

Source: `web/src/guided-ingest.tsx:110–145`; `web/src/import-workspace.tsx:128–159`; `web/src/curation.tsx:53`.

## Accepted follow-up requirements — 2026-09-21

The user explicitly requested manual item removal, clean reimport, cleanup of unfinished/failed/duplicate processing, consolidated identical validation results, completed-result-first actions, and prevention of duplicate active scans. These are required product behavior, not implemented capabilities. This follow-up is based on source inspection; no additional browser reproduction or implementation is claimed.

### UX-15 · P2 · Provide manual removal and a clean-reimport workflow

There is no general item-removal action in the current browser or application service. The existing `retire_item_if_unchanged` mechanism only removes an unused automatic placeholder after capture reassociation. Physical-copy disposal and image archival do not remove an item.

**Required behavior:**

- Offer **Remove item** from the item's actions. Remove it from ordinary browsing/search, with a clear explanation of retained originals/history and a restore path. This is a user-facing removal, not merely disposal of a physical copy. Permanent payload deletion would be a separate explicit operation.
- Offer **Reimport cleanly** for an existing item. It should start a fresh import workflow and clean operational view without producing another visible duplicate item. Allow reuse of retained originals or selection of a replacement package; do not require uploading an unchanged disc again solely to reset processing.
- Preview what will be replaced or reset: active attempts, previous workflow history, automatic metadata, and prepared media. Preserve physical inventory, manual metadata and artwork choices by default; offer explicit reset choices for user-edited metadata when the user wants a fully fresh interpretation.
- Settle/cancel obsolete work before allowing it to publish into the reset item. A late result from an old workflow must not repopulate its current state. Shared jobs/assets must retain other owners' work and references.
- Keep the old usable state until replacement publication succeeds. Interrupted or failed reimport must remain resumable and must not strand or destroy the preserved originals. Already committed publication must finish recovery safely.

**Acceptance:** remove and restore an item with files; cleanly reimport an item containing multiple failed, duplicate and unfinished attempts; interrupt/restart mid-reset; prove late old completions cannot become current. The normal UI ends with one intended item and one current workflow.

Sources: `crates/collector-app/src/guided_ingest.rs:718`; `crates/collector-store/src/lib.rs:2709`; `web/src/curation.tsx:58`.

### UX-16 · P2 · Consolidate identical validation results and clean operational history

Current processing history renders every observation separately. Processing observation identity is derived from a submission-specific Job ID, so separate commands can create separate entries even for equivalent work.

**Required behavior:**

- Store/display one substantive result set for equivalent checks; retain lightweight occurrences such as first checked, last checked, repeat count, and optional individual check dates. Reuse result content rather than writing another full equivalent result body for each occurrence.
- Define equivalence from the committed source fingerprint, operation/algorithm version, normalized settings, actual computation coordinates/coverage, reference snapshot content, and complete normalized outcome/limitations. Timestamp and request IDs alone must not make a new substantive result. Distinct snapshot IDs with identical validated content may share content identity while retaining each acquisition's provenance.
- Matching status text alone is insufficient: partial versus complete coverage, different algorithms, different offsets, changed bytes or reference data must remain distinguishable. If materially different checks arrive at the same conclusion, they can share a compact UI group while preserving those distinctions.
- A newly requested check that actually rereads/recomputes and confirms the same result adds a verification occurrence. Merely showing/reusing a cached result must not advance the last-verified date.
- Offer **Clear failed/cancelled history** and **Cancel and clear unfinished work** with accurate counts and scope. Clearing history removes operational clutter, not raw masters or independently useful successful evidence. Cancel unfinished work durably before cleanup; reclaim eligible scratch only after work settles. Retain compact diagnostics/provenance where needed for recovery, without forcing users to see every attempt in the normal history.
- Existing duplicated observations should be consolidated in the read projection without rewriting immutable historical evidence. New checks should use result-content reuse plus lightweight occurrences.

**Acceptance:** repeat a completed check with advancing timestamps and get one result row plus updated occurrences; rerun against changed inputs/coverage and retain distinct evidence; clear failed/unfinished clutter, restart, and verify it does not return or delete successful results.

Sources: `crates/collector-app/src/lib.rs:4259`; `web/src/processing-workspace.tsx:116`; `web/src/job-ledger.tsx:41–56`.

### UX-17 · P2 · Completed scans should lead to results, not another scan

The current panel continues to offer **Run operation** after completion. Its availability is based on capability/configuration and local `busy`, not whether an equivalent current result already exists.

**Required behavior:** show the current result, its scope, and its last actual check time first. Make **View result** the obvious action. Put intentional **Check again** behind a secondary/advanced action. If the source, settings, algorithm or reference data changed, explain what changed and why a new check is useful. “Completed” must not be presented as “verified”: an inconclusive/no-match check also completed, but its outcome remains explicit.

**Acceptance:** reopen an item with a current completed scan; the primary action opens its result. Changing material inputs offers a new check with a reason. Intentional repeat checks use UX-16's consolidated history.

Source: `web/src/processing-workspace.tsx:108–116`.

### UX-18 · P1 · Prevent equivalent scans while one is underway

The current UI only disables **Run operation** while that mounted panel's local `busy` flag is true. Reopening the editor resets that state. `enqueue_processing_operation` derives the job identity from user and command key; the web transport generates a fresh key for a new command. It does not perform equivalent-active-work admission at this boundary.

**Required behavior:**

- While an equivalent scan is queued, running, or cancelling, expose status/progress and **View progress**, with cancellation where authorized. Do not offer another start action for the same computation.
- Enforce this atomically in the application/store admission path, not only by disabling a browser button. Concurrent tabs, reopened panels, restarts, retries, and automatic workflows must converge on one active computation.
- Derive work identity from source, operation/version, settings and reference input. Maintain durable request ownership and permissions so shared work is not cancelled by removing only one requester. Returning progress must not disclose another user's private job details.
- Determine availability using committed fingerprints and stored work state. Do not hash the full payload just to decide whether to show a button.

**Acceptance:** concurrently submit equivalent scans with different command keys; only one computation runs. Close/reopen, restart, and try from another tab: the UI restores active progress. A cancellation request continues to block a replacement until settled; shared-owner cancellation remains correct.

Sources: `web/src/processing-workspace.tsx:79–105,115`; `web/src/runtime.ts:27–28`; `crates/collector-app/src/lib.rs:4259–4325`.

These additions expand the prioritized review to **18 findings**. Implement UX-18 with UX-16/17 as one complete scan lifecycle improvement, and UX-15 as a removal/reimport delivery with explicit recovery acceptance. They do not mark the active alpha assignment or any release gate complete.

## Smaller but worthwhile improvements

- **Use role-aware guidance.** The player tells viewers to use “Edit / attach files” even when that action is absent. Curators may be pointed to Administration despite not having it. Explain who can perform the action, and give an actionable next step. `web/src/music-tracks.tsx:200–202`.
- **Reduce implementation vocabulary.** “Field authority,” “capture accession,” “immutable snapshot,” “durable batch,” “carrier,” provider IDs, and raw operation names compete with the user's task. Keep exact technical detail available under provenance/advanced disclosures.
- **Make costs human-readable.** “Cost in minor units” exposes storage representation. Accept/display normal currency amounts with a currency-aware conversion; JPY and currencies with fractional units should not require mental arithmetic. `web/src/inventory-editor.tsx`, `SpecimenForm`.
- **Fix artwork selection semantics.** Selecting “Automatic candidate or collection cover” sets an empty local choice and disables “Use selected artwork.” A separate “Clear artwork choice” button is required when a manual choice exists. Make selecting and saving automatic behavior work like selecting and saving a specific cover. Include artwork-only drafts in dismissal protection. `web/src/metadata-review.tsx:94,105`.
- **Distinguish unavailable artwork from no artwork.** A thumbnail request failure falls back to “No art” / “No artwork attached,” which misstates an existing asset's state. Display “Preview unavailable” and allow retry. `web/src/app.tsx:27–30`.
- **Make lookup source selection complete.** Choosing a custom MusicBrainz release/disc option changes `lookupId` but only updates kind/value when the option exists in `review.lookup_choices`. Custom options need a second type selection and can retain a stale value. Set appropriate type/placeholder directly. `web/src/metadata-review.tsx:100`.

## Suggested delivery order

1. Correct state attribution and navigation: UX-01, UX-02, UX-04; add focused delayed-response and responsive-navigation regressions. Prevent duplicate active scans and consolidate scan results/actions through UX-18, UX-16 and UX-17.
2. Repair incomplete workflows: UX-03, UX-05, UX-06, UX-08, UX-09, and item removal/clean reimport in UX-15; verify each through the real UI.
3. Reshape the primary work surfaces: UX-07, UX-10, UX-13, UX-14.
4. Improve larger-library browsing and operational visibility: UX-11, UX-12, then the smaller improvements above.

The existing separation of original preservation from metadata/verification, durable work, cover previews, and explicit preparation status is worth retaining. The redesign should make those capabilities easier to understand and reach while preserving their underlying guarantees.
