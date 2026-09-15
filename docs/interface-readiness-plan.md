# U05 — Guided interfaces and readiness review

User requirement, 2026-09-15. Planned, not implemented. Required alongside
[P04.4–P04.6](import-management-plan.md) before declaring the web UI complete.
Applies to every implemented web surface and to acceptance of future web, native
and CLI interfaces. It does not require implementing all future clients now or
change the current D06.3 handoff.

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
| U05.1 — Readiness contracts and analysis defaults | Planned | A03/A04/P03/D05: module input requirements, typed preflight/eligible inputs, complete analysis workflow |
| U05.2 — Audit and correct all implemented interfaces | Planned | U05.1 plus each surface's owning services: enumerate every screen/action, implement findings through their owners |
| U05.3 — Interface completion review | Planned | U05.2/P04.6: actual user journeys, negative cases and audited completion evidence |

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

## Ready-to-send assignment when scheduled

Implement U05.1 from docs/interface-readiness-plan.md. Read AGENTS.md,
docs/architecture.md, docs/release-requirements.md and the relevant A03/A04/P03/D05
contracts. Establish module-owned input requirements and application-owned typed
readiness; implement eligible analysis choices, valid defaults and up-front
missing-input guidance through the real UI and API. Cover CUE/redumper, misplaced
anchors, missing companions, ambiguity and stale selections. Preserve imports
without analysis. Update this ledger, current status, validation and the next
assignment; do not claim U05.2/U05.3 complete from this analysis-specific child.
