# U05 — Guided interfaces and readiness review

**Provider lookup requirement 2026-09-22 (FX-04):** Automatically acquire CD
metadata when supported dump analysis supplies its actual disc identity. Reuse the
existing guided disc-ID acquisition for other intake paths; do not require a lookup
button or automatically search filenames/timestamp-derived titles. Missing identity,
no match, ambiguity and failure need distinct states; original preservation and
manual metadata authority remain intact. Advanced explicit lookup/retry remains a
recovery path, not the default journey. Verify requests for timestamp-named dumps.

**Historical scale requirement 2026-09-22 (resolved by FX-04/FX-07):** At the time, existing-entry attachment
listed general collection entries, not detected matches, and stopped at
1,000. The requirement was server-side search and explicit paging, with evidence-backed
suggestions clearly separate from general search. Audit all growing record selectors
for collection preloading, silent caps, disambiguation and access to later results;
validate with at least 3,000 catalog entries. The manual-import destination search's
former unpaged 50-result limit was also in scope. Fixed small enum choices may remain dropdowns.

**Historical user requirement 2026-09-22 (resolved by FX-04/FX-07):**
Import history and current-import details must be distinct navigable views, with no
global history list embedded in intake/progress/detail panels. Preserve list context
on return and support direct-link/reload of the selected import. Validate with
thousands of records and bounded loading. More generally, separate meaningful
browse/detail/task views instead of accumulating unrelated content in one panel.
See [the FX-04 scope and acceptance](ux-fix-plan.md#fx-04--focused-editing-and-consistent-intake).

**FX-03 update (2026-09-21):** Work & history, scoped server filtering/pagination,
workflow/attempt grouping, dismissal/restoration, selected cleanup with durable
settlement and byte outcomes, server-backed import reopening, and removal of
ineligible derivative analysis forms are implemented. Local acceptance passed on 2026-09-21; see [validation](validation.md#fx-03-work-and-history--2026-09-21). Broader interface, mistaken-item removal/reimport and release gates
remain open; this does not mark all U05/P04 criteria complete.

**Planning update 2026-09-21:** [FX-01–FX-07](ux-fix-plan.md) groups the remaining
operation matrix, whole-interface corrections/nonblocking behavior and completion
review into integrated fixes with the UX audit. FX-01 completes the browse/navigation
subset with [local acceptance evidence](validation.md#fx-01-browsingnavigation--2026-09-21). U05.1, the remaining U05.2 surfaces and the
whole-interface U05.3 gate remain open. FX-02 implements the processing-scan subset; its [local acceptance passed](validation.md#fx-02-scan-lifecycle--2026-09-21). FX-03 implements history/cleanup and derivative-target presentation with local acceptance passed; FX-04–FX-07 remain proposed.

**Status update 2026-09-27:** FX-04–FX-07 are locally accepted. FX-05 implements
Remove/Restore and clean reimport; exact lifecycle validation is recorded separately.
The implemented web-interface U05 gate passed local acceptance. FX-07 added
bounded server search/paging for administrator accounts, personal security
histories and authorized collection selectors. Administration has
focused tasks and collection-scoped catalog results. The metadata completion
response distinguishes candidates from no match in the real browser.
The [FX-07 interface audit](fx07-interface-audit.md) records current action
coverage and combined journeys. Independent accessibility/security and physical
device/deployment qualifications remain separate.

**Scheduling update 2026-09-16:** [Local alpha](local-alpha-plan.md) is active.
LA1–LA3 integrate the relevant acceptance below across layers. Individual child
assignments at the bottom are reference scope, not separate required handoffs.
Keep unmet broader criteria open; alpha does not certify every web interface.

Historical user requirement, 2026-09-15. The implemented web surface passed
local acceptance through FX-01–FX-07 alongside
[P04.4–P04.6](import-management-plan.md).
Applies to every implemented web surface and to acceptance of future web, native
and CLI interfaces. It does not require implementing all future clients now or
block LA1 on unrelated future interfaces.

## Standard for every action

Background-work requirement (2026-09-15): use the
[application-wide nonblocking rule](background-work-plan.md). U05.2 audits every
busy/disabled state for a real dependency or conflict, recording submission versus
execution state and which independent actions remain available. U05.3 must prove
more work can be queued before earlier jobs finish. D07.4–D07.5 supplies the album/
track workflow and is an additional dependency of U05.3 completion.

- Choose useful, valid defaults from the actual target and available capabilities.
  Auto-select a unique eligible choice. When several materially different choices
  remain, explain them and ask for selection; do not guess using file order.
- Offer only eligible inputs and combinations. An ineligible option may be shown
  disabled with a reason when that aids discovery, but must not be selectable.
  Omit irrelevant controls rather than asking users to configure unused inputs.
- Show known missing prerequisites before submission, naming what is required
  and how to supply it. A known-invalid action must not enqueue a doomed job.
  Explain disabled actions; preserve other useful operations such as saving raw
  files when optional analysis or encoders are unavailable.
- Recompute eligibility on target, operation, profile, source, role or capability
  changes. Clear stale selections; preserve valid user choices. Cancel obsolete
  requests and refresh immediately after mutations, without reopening a window.
- Explain what an action affects, what actually completed, and the next step.
  Distinguish loading, empty, unsupported, unavailable, failed and ambiguous states.
  Provide actionable recovery, cancellation and draft/discard behavior.
- Use progressive disclosure for advanced settings. Make common workflows usable
  without knowing internal IDs, file indices, module names or job architecture.

Module-owned input requirements and application-owned eligibility/authorization
must reach clients through typed contracts shared by all consumers. The browser
must not duplicate format policy or infer eligibility from error messages. Cheap,
bounded preflight identifies known blockers; structural checks remain necessary
and the service revalidates submitted inputs against current/frozen state. A file
extension can identify a candidate, but cannot establish valid content. Successful
preflight is not catalog verification and cannot guarantee provider/tool success.

## Analysis example and acceptance

The current processing form defaults to file index 0 and BIN/CUE while offering
all files. Redumper requires a `.toc` anchor; BIN/CUE requires a `.cue` anchor.
Both require the appropriate supporting package members, not just a sidecar.

- For one eligible redumper TOC, select it and the redumper layout automatically,
  even when `.scram`, `.state` or `.subcode` comes first in the inventory.
- For BIN/CUE, select the eligible CUE and its matching layout. A TOC is not a
  universal prerequisite for all analysis operations.
- Reject selection of non-anchor files for anchor-based operations. If an anchor
  or required companion is absent, identify it before Run and explain how to
  supply a complete package. Preserve the option to store an unanalyzed package.
- Multiple anchors, conflicting layouts, unsupported versions and malformed
  sidecars get explicit outcomes. Auto-selection must not invent disc boundaries
  or silently pick the first candidate.
- Changing operation/package invalidates incompatible selections. Missing tools,
  snapshots, structure results or permissions are explained before dependent work.
- Exercise direct API rejection as well as the UI. Valid defaults do not replace
  server validation, resource limits, cancellation or authorization.

## Child ledger and audit coverage

| Child | State | Scope |
|---|---|---|
| U05.1 — Readiness contracts and analysis defaults | Locally accepted for implemented web actions via LA1, FX-02/04 and FX-07 | Typed anchors/blockers/provider availability, eligible defaults, current source/configuration state and invalid submission without a Job |
| U05.2 — Audit and correct all implemented interfaces | Locally accepted in FX-01–FX-07 | [Action and growing-list register](fx07-interface-audit.md) covers each implemented surface and its owning service |
| U05.3 — Interface completion review | Locally accepted in FX-07 | [Combined journeys](fx07-interface-audit.md#combined-journeys-and-completion-boundary) include negative cases, roles, mobile, reduced motion, reload/reconnect and real playback |

U05.2 must inventory every implemented screen/action, not just these examples:

| Surface | Required review examples |
|---|---|
| Setup, login, account/security, administration | Configured origins, prerequisites, roles, encoder/provider availability, consequences and recovery |
| Collections, browse/search, item editing | Useful defaults, empty/no-results states, unavailable modules, allowed fields, stale revisions |
| Copies, carriers, artwork and associations | Eligible targets, dependent fields, preview, destructive consequences, drafts and conflicts |
| Imports, analysis, verification and job history | Complete input packages, applicable operations, snapshots, explicit findings and next actions, retry/cleanup |
| Preparation, playback and downloads | Source/profile/tool/browser eligibility, preparing versus playable states, immediate refresh, errors and access loss |

Maintain a checklist with screen/action, default, prerequisites, invalid
combinations, loading/empty/error states, consequences, recovery path, owning
ticket and test evidence. Findings must become scoped fixes even when the user
has not encountered them. A defect moved to another ticket still blocks this
completion gate until resolved; future unimplemented capabilities stay labelled
planned and out of the available-action flow.

U05.3 exercises complete journeys with fresh, partial, ambiguous, failed and
unavailable data, owner/curator/viewer roles, desktop/mobile, keyboard-only use,
reload/reconnect and changes while dialogs are open. Test cases must defeat
first-file/default-profile shortcuts and verify that known-invalid submissions
create no job. Record real UI evidence and all remaining gaps; a happy-path smoke
test or completed audit document alone cannot satisfy acceptance.

## Historical child assignment (use the local-alpha handoff instead)

Implement U05.1 from docs/interface-readiness-plan.md. Read AGENTS.md,
docs/architecture.md, docs/release-requirements.md and the relevant A03/A04/P03/D05
contracts. Establish module-owned input requirements and application-owned typed
readiness; implement eligible analysis choices, valid defaults and up-front
missing-input guidance through the real UI and API. Cover CUE/redumper, misplaced
anchors, missing companions, ambiguity and stale selections. Preserve imports
without analysis. Update this ledger, current status, validation and the next
assignment; do not claim U05.2/U05.3 complete from this analysis-specific child.
