# Web navigation redesign — 2026-10-07

**Current direction (2026-10-09):** [remaining work](remaining-work.md)
is the only remaining-work queue. Older assignment/order and gap statements below
are historical. Proposed contracts require reassessment under
[the home-lab scope/retention policy](decisions/home-lab-scope.md); their presence
does not require implementing speculative machinery or permanent lookup history.
Phono Junk is no longer a target. Implemented contracts describe current behavior,
not a claim that cleanup or redesign has already happened.

## Problem and decision

The left rail mixed destinations, a persistent collection filter, management
commands and account actions with nearly identical styling. The Library care
label started midway through the task list; Activity and Work & history opened
the same destination. Long collection lists crowded out management controls,
and mobile presented every section as horizontally scrolling button strips.

Use a destination rail with Browse (Library, Lists), Organize (Imports, Metadata,
Music preparation, Activity), Manage (collection access and server administration) and a
separate account footer. Keep existing application services, permissions,
listener exposure, routes, history, request cancellation and playback ownership.
Move the collection filter to a searchable, paginated disclosure above workspace
content. Its current value is visible even when closed; changing it preserves
workspace routing and clears obsolete selected entries/lists through the existing
navigation controller. Activity has one navigation entry. Remove the misleading Manage collections
command: it opened an entry editor. Add / import remains a contextual workspace
action, and collection creation remains in Administration. Management dialogs
remain commands rather than receiving false destination selection indicators.

## Visual direction

Use a deep blue navigation rail (#20334b), a cool white workspace (#f5f7fa),
white reading surfaces (#ffffff), blue action/selection (#355fd6), dark text
(#24324a) and muted text (#606c7f). Retain the local Avenir Next/system sans stack;
use it for headings as well as controls, with clear weight and scale. Artwork
and collection entries remain the product's visual content. No decorative hero,
new font dependency, or generic card dashboard.

```
Desktop                           Phone
┌──────────────┬────────────────┐  ┌────────────────────────┐
│ Brand        │ Collection ▾   │  │ Brand             Menu │
│ Browse       │                │  ├────────────────────────┤
│  Library     │ Workspace title│  │ Collection ▾           │
│  Lists       │ Search/actions │  │ Workspace title        │
│ Organize     │                │  │ Search/actions         │
│  Imports …   │ Artwork + text │  │ Artwork + text         │
│ Manage       │                │  │                        │
│  Access      │                │  └────────────────────────┘
│ Account      │                │  Menu expands labelled
└──────────────┴────────────────┘  groups in normal flow.
```

All content is left aligned. Group borders and spacing encode different tasks;
the active destination has a marker plus contrast, with aria-current. Descriptions
explain the less obvious task names. Mobile menu and collection disclosure are
keyboard operable, close after selection and avoid hidden horizontal scrolling.
Desktop navigation scrolls independently when necessary; all controls remain
reachable at short viewport heights and with the persistent player present.

## Acceptance

Exercise real navigation, collection search/pagination/errors, retained context
across workspaces, back/forward, permission-dependent controls, keyboard focus,
mobile disclosure/overflow and playback through the existing smoke journeys.
Inspect desktop/mobile screenshots of actual populated collection content. Record
completed checks and limitations here and in implementation status.

## Delivery and validation

Implemented in the existing shell and browse navigation controller. No new
service, private-state format or authentication pipeline. Search and pagination
continue through the existing collection search; no collection IDs are guessed.

Passed: real-server navigation acceptance in Chrome 153, populated desktop and
390px screenshots, scope preservation across Lists/Library and browser Back,
search continuation and query-change fences, keyboard disclosure/Escape/focus,
mobile group separation and reachable footer, short desktop viewport and
restricted-listener visibility. Existing browsing acceptance passed: entry
pagination/history/edit, sorting/filtering, mobile keyboard navigation, delayed
evidence and failure/retry. Architecture checks, fmt, complete workspace tests,
all-target clippy with warnings denied, web TypeScript/build, server rebuild and
whitespace checks passed. Full browser smoke validation is recorded below on
completion.

Screenshots from the temporary real-server fixture:
`/tmp/collector-navigation-desktop.png`, `/tmp/collector-navigation-mobile.png`,
`/tmp/collector-navigation-mobile-menu.png`. The fixture contains actual saved
inventory entries and honest missing-artwork placeholders. It does not represent
qualified audio captures or matching evidence.
