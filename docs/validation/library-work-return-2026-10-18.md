# HL-08 Library return after nested Work & history — 2026-10-18

## Delivery and boundary

The Library return defect recorded in the 2026-10-09 AccurateRip shortcut journey
is repaired. The browse history distance previously counted item selections but
copied that distance unchanged when opening Work & history and selecting work.
After closing the modal, “Back to results” could therefore land on an older work
URL and reopen the modal instead of returning to the unselected list.

The shell and Work & history now share a small URL-push helper that includes each
nested entry in the existing browse return distance. Closing work still replaces
its current URL, and intentional browser traversal still restores work URLs.
Direct links without a recorded results entry retain the replace-on-return
fallback. No server, API, storage, provider or sibling-client contracts changed.

## Browser regression

`npm --prefix web run test:browse` uses real Chrome, a temporary server/archive and
62 catalog entries plus a real unfinished Import. The new journey opens work
through the entry editor and checks:

- Browser Back from work detail to history, closing the modal, and same-document
  Forward reopening the selected work after the modal has unmounted.
- Intentional browser Back to work detail after navigating to another document.
- Reopening work, using Back to history and opening the Import again before close.
- Explicit Back to results clears both item and work selection, leaves no modal,
  restores both loaded pages, and restores the original row's scroll and focus.
- Direct item links can return after nested work without leaving their collection.

The regression failed against the previous embedded assets at the explicit return:
the URL retained item/work selection and Work & history reopened. That failure is
recorded in `/private/tmp/hl08-browse-before.log` for this local run. Existing
pagination, edit/artwork, filter/sort, mobile keyboard, delayed-read and recovery
coverage remains in the same journey.

## Validation

Passed against the rebuilt embedded-assets server with Google Chrome
153.0.8010.37:

- `npm --prefix web run test:browse` (including the new regression).
- `npm --prefix web run test:work` (paging, retry, cleanup and restart recovery).
- `npm --prefix web run test:navigation` (desktop/mobile, scope and browser Back).
- `npm --prefix web run test:smoke` (including actual FLAC playback, WebAuthn,
  provider concurrency, bulk imports and collection lists).
- `python3 scripts/check_architecture.py`.
- `cargo fmt --all -- --check`.
- `cargo test --workspace` (all enabled tests passed).
- `cargo clippy --workspace --all-targets -- -D warnings`.
- `npm --prefix web run check` and `npm --prefix web run build`.
- `cargo build -p collector-server` after the web build; the browse journey also
  checks served application/curation assets against the built files.
- `git diff --check`.

Existing manual/measurement-only ignored tests remain unrun; this delivery does
not claim owned-disc, external-provider or deployment qualification.

Local run logs: `/private/tmp/hl08-browse-after.log`,
`/private/tmp/hl08-work-browser.log`, `/private/tmp/hl08-navigation-browser.log`,
`/private/tmp/hl08-browser-smoke.log`, `/private/tmp/hl08-workspace-tests.log`,
`/private/tmp/hl08-clippy.log` and `/private/tmp/hl08-server-build.log`.

This is local software acceptance. HL-02 LAN/native listening and HL-04 actual
Linux/NAS/public-ingress qualification remain open. No live collection was changed.

## Self-review follow-up — 2026-10-18

Reviewed both URL push sites, browse return-distance updates, work URL replacement,
popstate reopening, direct-link fallback and reload restoration. No further
production-code defect was identified. The original Back/Forward regression kept
the modal mounted, so it did not exercise the shell reopening a work detail during
same-document traversal. The regression now closes and unmounts the modal before
Forward, and checks the expected Import title after both same-document restoration
and browser Back from another document.

The strengthened full browse journey passed in Chrome 153.0.8010.37 against the
existing rebuilt server; log: `/private/tmp/hl08-self-review-browse.log`.
`npm --prefix web run check`, `python3 scripts/check_architecture.py` and
`git diff --check` also passed again. This follow-up changes only the regression
and records; production sources and embedded assets are unchanged from the fully
validated delivery above.
