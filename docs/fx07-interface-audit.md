# FX-07 interface audit — 2026-09-27

This is the completed local coverage register for the whole-interface review.
Each named browser run used a temporary archive and profile. Independent
accessibility, security, hardware and deployment qualifications remain separate.

## Action review

| Surface and owner | Default and prerequisites | Invalid/pending/result and recovery | Evidence |
|---|---|---|---|
| Setup, sign-in, recovery (`collector-server`, `app.tsx`) | CLI setup creates the first account; browser starts with password sign-in. Recovery requires an unused code and replacement password. | Typed auth errors retain the form; a used code fails. A new session replaces old sessions. | `test:smoke`, `test:fx06`, Store recovery regressions |
| Collection browse, search, removed view (`collector-app`, `app.tsx`) | Authorized collections have 50-row server search/pages and exact selection; entries use separate server-filtered 50-row pages. | Loading, empty, failure and access loss are distinct; search/filters and retry preserve navigation. A changed search hides the prior page/cursor immediately; same-query refresh retains current navigation context. Removed entries are explicit. | `test:fx06` crosses collection pages and the debounce race; `test:browse`, `test:fx04`, `test:fx05` |
| Collection creation and server settings (`collector-app`, `administration.tsx`) | Collection type is required; settings and configured catalog sources are read from the server. Collections, catalogs, accounts and server settings are focused tasks. | Missing source/collection prevents catalog submission; collection and source lookup errors expose their own Retry controls even before a collection is selected. Existing snapshots remain available when a source is absent. | `test:smoke` injects initial catalog collection lookup failure and retries |
| Account administration (`collector-app`, `collector-store`, `administration.tsx`) | Authorized administrator; username search and 50-row keyset pages, ordered by username. | Invalid page sizes are rejected; stale account revisions fail; loading, empty, error/retry and later-page navigation are explicit. | New Store paging regression and `test:fx06` |
| Collection grants and account security (`collector-app`, `collection-access.tsx`, `security.tsx`) | Owner-scoped grant search; explicit role and exact username. Profile/recovery, browser sessions and passkeys/devices are focused tasks. | Recent authentication resumes only the original account's command; cancellation retains drafts. Revocation and code use report outcomes. Personal histories have 50-row pages. | `test:fx06`, `test:context`, Store auth/paging regressions |
| Item inventory, copies, loans (`collector-app`, `curation.tsx`, `inventory-editor.tsx`) | Type-specific fields and existing Item revision; physical copy and loan fields are task-specific. | Invalid associations and stale revisions fail without replacing history; drafts remain in the editor. | `test:smoke`, `test:fx04-review`, Store focused-inventory regressions |
| Artwork and metadata (`collector-app`, `asset-editor.tsx`, `metadata-review.tsx`) | Manual information retains authority; automatic candidates remain tentative and distinct from verification evidence. | Provider unavailability, no match, ambiguity and failed artwork are visible; explicit retry and manual choices remain. The completion response carries the actual refresh outcome, so no match cannot masquerade as a candidate result. | `test:smoke` offline no-match and ambiguous provider fixtures, `test:fx04-review`, metadata Store regressions |
| Guided intake and manual import review (`collector-app`, `guided-ingest.tsx`, `import-workspace.tsx`) | Server plan proposes eligible packages and analysis; preservation-only remains available. Existing destination uses server search. | Missing companions and tool availability appear before admission; upload resumes and durable progress survives panel closure. | `test:fx04`, `test:fx04-review`, `test:smoke` |
| Processing and evidence (`collector-app`, `processing-workspace.tsx`) | Eligible operation/anchor and frozen inputs come from typed plans. | Known invalid inputs create no Job; repeated equivalent scans show results and actual rechecks. Ambiguous and no-match DAT results are distinct and do not claim verification. | `test:scans` uses an offline DAT and real bytes; Store processing regressions |
| Work, history and cleanup (`collector-app`, `work-history.tsx`) | Authorized scoped search with 25-row pages; explicit selected targets for bulk actions. | Active, failed, cancelled, dismissed and cleanup-pending states remain distinct; retry/restore and actual reclaimed bytes are shown. Repeated cleanup preserves the first measured result. | `test:work` covers three imports, failed-twice analysis, actual cleanup fault/retry/restart; `test:fx04` and Store cleanup regressions |
| Remove, restore and clean reimport (`collector-app`, `clean-reimport.tsx`) | Retains originals and manual authority by default; reset choices are reviewed. | Replacement failure leaves old Capture usable; obsolete work is fenced; upload resume and retained-copy retry recover. | `test:fx05`, Store lifecycle/fault regressions |
| Album preparation and playback (`collector-app`, `music-tracks.tsx`, `music-player.tsx`) | Eligible source/profile and encoder availability; batch prepares separate tracks. | Independent work remains queueable; playback checks access and releases detached media; errors and retries are visible. | `test:smoke`, `test:fx05`, `test:queue` |

## Growing-list review

| Dataset | Current bound and way to find later records | Audit result |
|---|---|---|
| Items, including removed Items | Authorized server search/keyset pages of 50; direct Item URL. | `test:fx04` crosses 3,000 Items. |
| Existing-Item destinations, including manual import | Server search/pages of 50; selected Item remains visible outside results. | `test:fx04` selects beyond the old 1,000 cap. |
| Work and execution history | Server search/pages of 25; direct work URL and older-page actions. The page query filters grants in SQL without loading all collections. | `test:fx04` crosses 3,000 imports; `test:work` crosses pages with mixed eligibility and distinguishes three imports from their analysis jobs. |
| Inbox packages | Server search/pages of 50 within one configured inbox. | Search and More packages; no browser preload. |
| Catalog snapshots | Server pages of 50 per game collection. | More snapshots control; immutable IDs remain visible. |
| Collection grants | Server username search/pages of 50. | `test:fx06` covers scope switching; Store regression crosses pages. |
| Administrator accounts | Server username/display-name search/pages of 50; exact account reads use a single-row query. | New `test:fx06` and Store regression cross pages. |
| Preparation batches | Most recent 50 per Item; older Jobs in Work & history. | Music labels the bound and links to this entry’s history; Work & history retains its item filter across navigation and reload. |
| Authorized collections in navigation/forms | Role/module-filtered 50-row server search/pages; exact selected collection read. Navigation and curation, access, catalog and work filters share the bounded chooser. | `test:fx06` creates 55 later collections and selects one from the second page. Store regression tests authorization, filters and exact read. |
| Per-account sessions/passkeys/devices | Server pages of 50 by creation time and stable ID; More controls in separate security tasks. | Store regression crosses pages with equal timestamps. `test:fx06` exercises later browser sessions. |
| Per-Item provider candidate versions | Item-bound Store read has a 4,097-record ceiling and merge rejects more than 4,096. The browser renders the selected version plus 25 choices at a time, with search and More. | Bounded rendering and reachability within the admitted per-Item limit. Page the response before increasing that admission ceiling. |

## Combined journeys and completion boundary

| Journey | Local evidence |
|---|---|
| Fresh setup through import and actual playback | `test:smoke` with temporary setup, guided import, FLAC start/pause/seek/advance and revoked access |
| Incomplete package and recovery | `test:work` reopens a partial import after reload; `test:fx04-review` retains draft and destination choices |
| No-match and ambiguous verification | `test:scans` runs both results against a frozen offline DAT and published bytes; `test:smoke` separately covers ambiguous and no-match metadata lookup |
| Repeated and concurrent work | `test:scans` checks equivalent scan convergence, actual recheck count, independent operation eligibility and invalid submission without a Job; Store ownership/restart regressions cover publication races |
| Three imports and failed attempts | `test:work` identifies all three packages, displays separate import and analysis rows, shows two completed analyses and both attempts of the failed one |
| Cleanup, interruption and reimport | `test:work` covers mixed eligibility, real staging removal, filesystem cleanup failure, Retry and restart reclamation; `test:fx05` covers replacement interruption, resume, old playable state, removal and restoration; Store tests fence stale completions |
| Roles, stale state and reconnection | `test:fx06` covers owner actions, session revocation and recovery; `test:smoke` covers viewer denial, WebAuthn, offline/reconnect, mobile and reduced motion; `test:fx04-review` covers stale drafts and editor context |

P04.6 and U05.2/U05.3 are locally accepted for the implemented web surface.
The per-Item metadata candidate ceiling is an explicit bound; a paged candidate
response is required before raising it. Independent accessibility/security review,
trusted second-device playback and physical hardware/deployment qualification
remain open in their respective release gates.
