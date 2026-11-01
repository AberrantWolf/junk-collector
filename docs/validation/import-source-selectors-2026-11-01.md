# Import source selectors — 2026-11-01

## Delivery

The existing shared browser chooser serves library imports, guided intake and manual
imports. Its presentation uses existing application colors/type, with horizontally
aligned controls/text, full-row labels, selected/focus feedback, long-name wrapping
and a bounded scroll area. No domain, API, authorization or import pipeline changes.

Multiple selection offers **Select all shown**, a native indeterminate checkbox,
shown/selected counts and **Clear selection**. The bulk toggle selects/deselects
loaded rows in the current search without dropping choices outside that search.
Pagination remains explicit: newly loaded rows start unselected; the bulk control
becomes mixed until selected. Clear selection removes hidden choices as well.
Single selection retains radios and does not offer bulk controls.

## Validation

Selected with `python3 scripts/check_changes.py --dry-run --journey intake-review`:
architecture/documentation links, web type check/build, model tests, one embedded-asset
server/media fixture build, short browser smoke and the intake-review journey.
The browser run uses the permitted macOS context outside Codex seatbelt.

The added real-server chooser acceptance uses 52 filesystem packages, crosses the
50-row page boundary, checks keyboard bulk selection and mixed state, filters while
retaining hidden choices, clears those choices and checks the disabled Review action.
Desktop (1440px) and mobile (390px) screenshots inspect long unbroken names, aligned
controls, wrapping and absence of horizontal overflow.

All selected checks passed, including short smoke (7.01s) and intake-review
(96.35s), using Chrome 153.0.8010.12. The existing review, cleanup/restart and
31-album activity/provider-settlement acceptance passed with the new chooser.
Desktop/mobile screenshots were visually inspected. Documentation links and diff
whitespace checks were checked again after the delivery notes were added.

Rust tests/Clippy and the broad browser suite are omitted for this frontend-only change under the maintained
change-selection policy. No release-mode deployment artifact or publication is claimed.

## Follow-up: active import review popup

The package's Review import button previously mounted its details after the entire
member list, leaving the result offscreen on long batches. It now opens an explicitly
named Import review dialog using the shared dialog dismissal/focus mechanism. The
popup shows loading before the selected package's progress arrives, keeps focus
inside, supports Escape and close, and returns focus to its package button. Closing
unmounts the detail reader; the admitted import and batch polling continue normally.
Existing upload/draft/busy dismissal protections remain in the shared mechanism.

The active 31-album real-server journey now opens the first package on desktop,
closes and restores focus, opens another package with the keyboard on mobile,
checks the selected package and visible close control, exercises focus trapping and
Escape, and reopens the first package before checking ongoing batch settlement.
Screenshots cover the popup at desktop/mobile widths and were visually inspected.
The complete selected check set passed again: architecture/documentation links, web
type checking/build, model tests, embedded-asset server build, short smoke (7.11s)
and intake-review (91.52s), using Chrome 153.0.8010.12. The original selector
acceptance and review/recovery checks also passed in this run. Rust tests/Clippy
and the broad browser suite remain omitted for the same frontend-only scope.
No deployment is included.

## Review corrections

The shared dialog focus order now includes native summary controls, so the final
Correct destination disclosure remains reachable with keyboard navigation. Guided
intake exposes a pending association correction as unsaved state to the existing
dialog dismissal guard. Both Close and Escape ask before discarding it; refusing
keeps the selected destination across detail polling. Accepting discard and reopening
shows the saved destination, with no accidental association mutation.

Regression acceptance exercises those behaviors in the real active-batch popup,
including keyboard opening of Correct destination, rejection of both dismissal
paths, polling with a pending correction and discard/reopen. The document journey
also exercises another consumer of the shared dialog helper.

The first regression run raced original publication; the test now waits for actual
correction eligibility. The next run exposed Chrome retaining rectangles for
collapsed disclosure contents. The focus helper now uses native visibility and
nonnegative tab order, including visible summaries while excluding collapsed
contents. Both failures are corrected; assertions were retained.

Final selected checks passed: architecture/documentation links, web type checking
and build, model tests, embedded-asset server/media fixture build, short smoke
(7.23s), intake-review (92.30s) and documents (8.80s). Intake used Chrome
153.0.8010.12; documents used Chrome 156.0.8078.74. Browser runs used the permitted
macOS context. The final selector report is `/tmp/import-review-final-checks.json`.
Rust tests/Clippy and the broad browser suite remain omitted under the maintained
frontend change-selection policy. No deployment or release publication is included.
