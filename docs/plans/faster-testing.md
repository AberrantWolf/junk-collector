# Faster testing plan

This plan makes web behaviour cheaper to test. It does not change what is tested.
When it is done:

- The rules a screen uses to decide what to show, and which request to send, live in
  plain TypeScript modules. Plain Node tests check them in well under a second, with
  no browser and no server.
- Browser journeys still start a real server and open the real screen. They check
  that the screen is wired to those rules and to the server, and that it looks
  right. They no longer have to walk every branch of a rule to cover it.
- A journey can start from data prepared earlier by the same server binary, rather
  than building it through the UI every time.
- One scenario in a long journey can run alone.

Each phase below can land on its own. The order at the end puts the cheapest,
biggest wins first.

## Status

Done:

- Phase 0: `web/tests/support/load-source.mjs`, `web/tests/unit/*.test.mjs` on
  `node:test`, and `npm --prefix web run test:unit` in CI (it also runs
  `harness-check.mjs`). First tests for `browse-location.ts` (split out of
  `browse-navigation.ts`), `import-file-order.ts`, `review-reasons.ts` and
  `playback-preference.ts`. `intake-drafts.ts` is only IndexedDB plumbing and has
  no rule worth a unit test.
- Phase 1 items 1 to 6: `album-copies.ts`, `entry-removal.ts`,
  `review-list.ts` (now also the list requests, Reason filter, polling, tab
  labels, import states, "Next entry" and "all matching" ticks),
  `metadata-candidates.ts`, `intake-plan.ts` and `check-results.ts`, with unit
  tests. The "is this a digital purchase" test now has one home
  (`isDigitalPackage`). Not moved: `lookupFeedback` (spread across several
  commands) and `discFromPath`, which waits on the disc-folder decision below.
  Searching the match list now keeps the match in use pinned above the
  results, without counting it as one (it used to disappear when it didn't fit
  the search). `digital-purchases.mjs` keeps one copy
  switch and "Use every copy"; its copy-count and format-choice assertions moved to
  the unit tests. The removal notice wording stays in the two screens.
- Phase 3 registry without snapshots: `journeys.json` entries can carry
  environment settings, so `auto-grouping`, `grouping-ambiguity` and
  `ingest-progress` are named journeys. The first two run in the default suite.
- Phase 4 ports and concurrency: every journey picks free ports, and
  `run-suite.mjs` runs `COLLECTOR_TEST_JOBS` at a time (default half the cores),
  with timing and audio journeys in a one-at-a-time group afterwards.
- Phase 4 pacing: see the corrected section below.
- Not in the original plan: building `collector-server` now rebuilds `web/dist`
  when `web/src` changed (or refuses, with the command to run, when it can't), so
  a server can't be built with old screens. The container build copies `web/src`
  for the same check, so an image can't ship old screens either.

- Not in the original plan: Rust tests. `collector-store/tests/archive.rs` (344
  tests) was about 290 s of `cargo test --workspace`, mostly waiting on disk
  flushes on macOS. `scripts/test-tmpdir.sh` puts scratch data in memory, and
  debug builds now optimize third-party crates. Together: about 70 s.

Left: Phase 1 item 7; Phase 2 snapshots; the Phase 3 splits that need
snapshots; replacing refresh-timer waits.

## Where the time goes today

Measured on one developer machine: Apple M4 with 10 cores, a debug server build,
the default Chromium, and `npm --prefix web test` running journeys one after
another. The timings came from one run, with temporary per-line timestamps added
to the journeys (removed afterwards). Expect a few seconds of variation per
journey.

The whole suite took **12 min 26 s**.

Each journey's own time, as `run-suite.mjs` reports it:

| Journey | Seconds | Journey | Seconds |
| --- | ---: | --- | ---: |
| `integration` (`smoke.mjs`) | 151.2 until it failed (see findings) | `game-catalog` | 11.5 |
| `opus-preparation` | 92.1 | `game-audio` | 10.4 |
| `purchases` (`digital-purchases.mjs`) | 86.6 | `navigation` | 10.1 |
| `intake-review` | 86.2 | `fx06` | 9.6 |
| `digital-matching` | 47.5 | `smoke` (`quick-smoke.mjs`) | 7.3 |
| `work` | 41.9 | `remote-media` | 7.0 |
| `browse` | 31.1 | `storage` | 6.7 |
| `fx05` | 22.3 | `local-playback` | 6.5 |
| `fx04-review` | 20.2 | `accounts` | 6.3 |
| `cd-library` | 19.3 | `documents` | 5.4 |
| `scans` | 19.2 | `backup-reminder` | 4.5 |
| `email-purchases` | 16.9 | 7 others | under 3 each |
| `fx04` | 14.7 | | |

Inside the slowest journeys, the time splits like this:

| Journey | Server ready | Browser, sign-in, collections | Data setup and waiting for jobs | Steps under test |
| --- | ---: | ---: | --- | --- |
| `digital-purchases.mjs` (86.6 s) | 0.5 s | 2.2 s | About 48 s waiting for intakes and preparations to finish. The four longest waits were 13.0, 9.0, 9.0 and 7.0 s. | The rest is UI steps. The copy choice, format choice and delete scenarios start at 77.2 s and take 9.4 s. |
| `intake-review.mjs` (86.2 s) | 0.5 s | 1.8 s | Inbox imports, provider settlement and two server restarts, mixed in with the steps | Inbox chooser 2.0 s, intake review 31.5 s, cleanup and progress 50.3 s |
| `intake-review.mjs` with `AUTO_GROUPING_ONLY` | 0.5 s | 1.6 s | 4 disc imports 0.7 s, intake completion 2.3 s, then 11.0 s waiting for automatic grouping | It failed at 16.6 s (see [findings](#findings-from-planning)). Removal and deletion come after the undo and a restart. |
| `browse.mjs` (31.1 s) | 0.6 s | 2.0 s | 5.6 s creating 62 entries through the API, then 2.2 s waiting for the background library index | About 21 s |
| `fx05.mjs` (22.3 s) | 0.5 s | 2.7 s, including seeding one entry through the API | 1.5 s import | About 17.6 s, including a deliberate 5.5 s wait across the player's access recheck |
| `cd-library.mjs` (19.3 s) | 0.5 s | 1.7 s | 1.0 s API import, then 12.5 s for a library batch over 105 entries | About 3 s |

What this shows:

- **Starting a server and a browser is cheap.** Creating the server's storage,
  serving, launching the browser and signing in take 2 to 3 s per journey. That
  is about 1.5 min of the suite's 12.5 min. Prepared data saves little on its own
  for short journeys.
- **Most of the time goes to the server's background jobs.** These are imports,
  analysis, provider lookups and grouping. Some of it is pacing rather than
  work. The MusicBrainz client always waits at least 1 s between requests
  (`MUSICBRAINZ_MINIMUM_INTERVAL` in `crates/collector-providers/src/metadata.rs`),
  even when a journey points it at a local stand-in server. Only one job per
  provider runs at once.
- **Some of the time goes to the screen's own refresh timers.** Nine waits in
  `digital-purchases.mjs` last 1.8–1.9 s, which matches a 2 s list refresh.
  Several sections there also wait for 5 s track-availability refreshes.
- **Journeys run strictly one after another.** About 18 journeys hard-code
  their listener ports (for example `fx05.mjs` uses 20001–20003), so they can't
  run side by side today.
- **Rerunning is what costs the most.** A change to the copy choice rule is
  checked only after `digital-purchases.mjs` has run for 77 s. A change to
  removal is checked only through the grouping journey, which no suite runs. A
  flaky wait anywhere earlier in a journey hides the step being changed.

## Target layers

| Layer | Runs | Checks | Typical time |
| --- | --- | --- | --- |
| Rust tests (`cargo test`, mainly `crates/collector-store/tests/archive.rs`) | Real application services and store, no HTTP | Policy, storage, recovery and evidence rules | no server process or browser |
| Web unit tests (new `web/tests/unit/`) | Plain Node, sources bundled by esbuild | Screen decision rules: which choices are offered, labels, request bodies, retry and ordering rules | under 5 s for the whole layer |
| Scenario journeys (split from today's long journeys) | Real server started from prepared data, real browser | One user task end to end, plus the real screen at desktop and mobile widths | 10–40 s each |
| Release journeys (kept whole) | Real server built from scratch, real browser, real Chrome where needed | The full flows listed under [what stays end to end](#what-stays-end-to-end) | 5–150 s each, as today |

The rule for where a check belongs: if it can fail without a browser, it goes in
a unit test. A journey asserts that the screen used the rule (one or two cases
that only a browser can show), not every branch of it.

## Phase 0: a home for fast web tests

Three plain Node tests exist today (`web/tests/presentation-labels.mjs`,
`music-queue.mjs`, `work-context.mjs`). Each one bundles a source file with esbuild,
imports the bundle and asserts on it. They are listed in `journeys.json` like
browser journeys, the default `npm test` skips them, and CI did not run them
until this plan's first commit added `npm --prefix web run test:models`.

Steps:

1. Add `web/tests/support/load-source.mjs`, which bundles one `web/src/*.ts` file
   for Node and returns its exports. It is the shared form of the `load()` helper
   in `work-context.mjs`. It asks esbuild for the list of files that went into the
   bundle and fails if that list includes `react`, `src/runtime.ts`,
   `@collector/runtime` or any `.tsx` file. That keeps the modules under test free
   of the DOM and the network, so a test can't quietly start depending on a
   browser.
2. Move the three tests to `web/tests/unit/*.test.mjs` using Node's built-in test
   runner (`node:test`). This gives named test cases, `--test-name-pattern` to run
   one case, `--watch`, and files run in parallel. Keep their assertions as they
   are.
3. Add `npm --prefix web run test:unit` (`node --test tests/unit/`). Replace the
   `test:models` step in CI with it, list it in [testing](../testing.md), and
   remove `queue`, `context` and `labels` from `journeys.json`.
4. Add unit tests for the existing `.ts` modules that have none, starting with the
   ones that change most: `browse-navigation.ts` (12 commits), `import-file-order.ts`,
   `intake-drafts.ts`, `review-reasons.ts` and `playback-preference.ts`.

Time saved: none directly. This is the base the other phases build on.
Risk: low. The existing assertions move unchanged.

## Phase 1: move screen decision rules out of components

Each candidate below decides what a screen offers or sends, needs no DOM, and is
written inline in a component today. The ranking weighs how often the file changes
(commits touching it), how much browser time its only coverage costs, and how much
branching it has. Each move is one change: create the module, make the component
call it with no change in behaviour, and add unit tests. The component keeps its
state, effects and requests; only the decisions move.

Commit counts are from `git log --name-only -- web/src`.

### 1. Album copies and formats (`music-tracks.tsx`, 30 commits)

Today `albumCopies`, `formatChoices`, `currentCopy`, `preferredAvailable` and the
override building inside `chooseAlbumCopy` and `chooseAlbumFormat` all live in the
component. The two choose functions each build the same `track_overrides` list
with copied code. Only `digital-purchases.mjs` covers them, from about line 690 of
a 768-line journey, so checking one rule means running every section before it.

Target `web/src/album-copies.ts`:

```ts
type TrackRow = Pick<Availability, 'source_representation_id' | 'track_position' |
  'disc_position' | 'display_position' | 'profile' | 'album_selection' |
  'audio_format' | 'source_label' | 'state'>;
type TrackOverride = { source_representation_id: string; track_position: number;
  title: string | null; disc_position: number | null; display_position: number | null;
  album_selected: boolean | null };
type SelectionChange = Map<string, { positions: Set<number>; selected: boolean | null }>;

export function albumCopies(rows: TrackRow[], sourceLabel: (id: string) => string): AlbumCopy[];
export function currentCopy(copies: AlbumCopy[]): string;              // '' when none or several are used
export function copySelectionChanges(copies: AlbumCopy[], key: string): SelectionChange;
export function formatChoices(rows: TrackRow[]): FormatChoice[];
export function formatSelectionChanges(rows: TrackRow[], source: string, codec: string): SelectionChange;
export function mergeTrackOverrides(existing: unknown[], changes: SelectionChange): TrackOverride[];
export function preferredAvailable(group: TrackRow[], canPlay: (mediaType: string) => boolean): TrackRow | undefined;
export function groupDiscs(rows: TrackRow[], discOrder: (key: string) => number): DiscGroup[];
```

`canPlay` replaces the `document.createElement('audio').canPlayType` call, so the
rule runs in Node. The component keeps reading the current review and tracks and
sending the PUT.

Unit tests check:

- A CD rip set plus one purchase gives two copies, labelled with the disc count and
  the purchase.
- Two purchases holding disc 1 and disc 2 form one copy, so there is no choice.
  Two purchases that both hold disc 1 form two copies.
- Choosing a copy marks every track of the other copies as left out. It keeps each
  track's title, disc and display position overrides. It leaves overrides for
  other sources untouched.
- The empty key brings back only copies that were left out whole.
- An override with nothing left in it is dropped rather than saved empty.
- A format choice is offered only for an original the album uses, and only when
  that original holds more than one codec. "Automatic" clears `album_selected` but
  keeps the other per-track edits.
- `preferredAvailable` prefers an original, then FLAC, then anything playable, and
  skips formats the player can't decode.

Journey left: `digital-purchases.mjs` keeps one copy switch and one format switch
that play back, and checks the screen. Its other copy and format assertions move
to unit tests.

### 2. Removing and deleting entries (`remove-entry.tsx`, new, and `library-metadata.tsx`)

`RemoveEntryDialog.confirm` is a small state machine. Which requests it sends
depends on the choice (hide, delete files, delete everything), on whether the
entry was already removed, and on whether this dialog removed it before a failed
deletion. The button label, which choices are disabled, the error wording after a
partial failure, the copy-record checkbox in `DeleteFilesDialog`, and the notice
`library-metadata.tsx` shows afterwards are all inline. The only coverage is the
end of `automatic-album-grouping-journey.mjs` (which no suite runs today; see
[findings](#findings-from-planning)) and the end of `digital-purchases.mjs`.

Target `web/src/entry-removal.ts`:

```ts
export type RemovalChoice = 'hide' | 'files' | 'everything';
export type RemovalStep = 'remove' | 'purge' | 'purge-files';
export function initialRemovalChoice(item: Item): RemovalChoice;
export function removalChoiceDisabled(item: Item, choice: RemovalChoice, removedHere: boolean): boolean;
export function removalSteps(choice: RemovalChoice, state: { alreadyRemoved: boolean; removedHere: boolean }): RemovalStep[];
export function removalButtonLabel(choice: RemovalChoice, alreadyRemoved: boolean, busy: boolean): string;
export function removalFailureMessage(message: string, removedByThisDialog: boolean): string;
export function originalBytes(item: Item, captures?: string[]): number;
export function copyRecordChoice(item: Item, captureId: string): { copy?: Specimen; shared: boolean };
export function removalNotice(title: string, result: { removed?: Item; kept?: boolean; freed?: number }): { text: string; removed?: Item; closePanel: boolean } | undefined;
export async function withConflictRetry<T>(request: () => Promise<T>, refresh: () => Promise<void>,
  options?: { attempts?: number; wait?: (ms: number) => Promise<void> }): Promise<T>;
```

Unit tests check:

- An already removed entry offers only "delete everything" and sends only the
  purge.
- "Hide" sends only the remove.
- "Delete everything" sends remove then purge. A retry after a failed purge skips
  the remove.
- "Delete files" is disabled with no captures, or once this dialog has removed the
  entry.
- The failure message says the entry can still be restored only when this dialog
  removed it.
- `withConflictRetry` retries only 409 responses, refreshes between attempts and
  stops after the limit. The test passes an instant `wait` so it doesn't sleep.
- The copy-record checkbox is offered only when no other original shares the copy.

Journey left: one Review → remove → Library → delete permanently pass, as a
scenario (Phase 3).

### 3. Review lists and the fix panel (`library-metadata.tsx`, 18 commits)

The inline rules include:

- The query string each tab sends.
- Which reasons the Reason filter offers per tab.
- How fast a list polls (2 s while rows are still updating, otherwise 10 s).
- The tab labels with their counts.
- Which reasons "Accept as is" may accept.
- When "Group into one album" is enabled.
- Which entry "Next entry" opens.
- `ingestState`.
- The checkbox state in "all matching except" mode.
- When "Possibly the same album" opens on its own.

Target `web/src/review-list.ts`. It already exists with `groupingAllowed`,
`proposesGrouping` and `maxGroupedAlbums`, tested by `web/tests/review-list.mjs`;
the rest moves in beside them:

```ts
export function reviewTab(param: string): ReviewTab;
export function entryListPath(scope: { collection: string; search: string; reason: string;
  system: string; tab: ReviewTab }, cursor: string | null): string;
export function ingestListPath(scope: { collection: string; intake: string },
  status: 'active' | 'attention' | 'added', cursor: string | null): string;
export function reasonFilterOptions(tab: ReviewTab): ReviewReason[];
export function entryPollDelay(rows: Row[]): number;
export function reviewTabLabel(tab: ReviewTab, counts?: Counts): string;
export function ingestStateLabel(summary: Ingest): string;
export function groupingAllowed(selection: Row[]): boolean;   // exists: music only, 2–32 entries, one collection
export function nextEntry(rows: Row[], itemId: string): Row | undefined;
export function rowChecked(row: Row, selection: { all: boolean; exclusions: string[]; chosen: Row[] }): boolean;
export function bulkSelection(selection: {...}, query?: MetadataCareQuery): BulkSelection;
export function proposesGrouping(reviewReasons?: ReviewReason[], rowReasons?: ReviewReason[]): boolean;   // exists
```

`acceptable` already lives in `review-reasons.ts`, which gets its first tests in
Phase 0.

Unit tests check:

- Each tab sends the filters it claims: "Polish" asks for polish level only, and
  "Needs you" asks for `needs_attention=true`.
- The Reason filter never offers a reason the tab can't show.
- Lookup-failed, needs-main-file, no-catalog and grouping-proposal reasons are
  never auto-accepted.
- Grouping is allowed only for 2 to 32 music entries in one collection (already
  tested).
- "Next entry" at the end of the list is empty.

### 4. Metadata candidates and status (`metadata-review.tsx`, 26 commits)

A chain of inline expressions decides which album matches are listed, which are
hidden behind "show others", what the search keeps, which match is pinned on top
and in what order. Other inline rules choose:

- The status line: imported tags in use, provider match in use, manual metadata
  in use, or no match in use.
- Which background lookup the panel reattaches to after a reload.
- The feedback shown when a lookup finishes.

The only coverage is `smoke.mjs` (the 600 s integration journey) and
`digital-album-matching.mjs`. None of the status lines is asserted anywhere.

Target `web/src/metadata-candidates.ts`:

```ts
export function visibleCandidates(review: Review, options: { module: string;
  showOthers: boolean; search: string; limit: number }): { shown: Candidate[]; hidden: number; pinned?: string };
export function metadataSourceLabel(review: Review, module: string): string;
export function resumableLookup(lookups: Lookup[], dismissed: Set<string>): Lookup | undefined;
export function lookupFeedback(lookup: Lookup, review: Review): string | undefined;
```

Unit tests check:

- Selected and automatically accepted matches always show.
- Searching never hides the pinned match.
- A tentative identity is never labelled as one in use.
- A dismissed failed lookup is not reattached.

### 5. Import review batching and intent (`library-imports.tsx`, 27 commits)

The inline rules include:

- How packages are split into `/intakes/{id}/members` requests (at most 64
  packages, 10,000 files or 12 MiB each).
- Where a resumed review continues.
- The import intent built from the "preserve only", collection type and audio
  choices.
- Which packages count as needing a decision.
- The folder-layout guess (`albumRoot`).
- The "is this a digital purchase" test. Three copies of it exist, in
  `library-imports.tsx` (twice) and `processing-workspace.tsx`.

Only `intake-review.mjs` (600 s budget) and `opus-preparation.mjs` cover these.

Target `web/src/intake-plan.ts`:

```ts
export function intakeBatches<T extends { files: { bytes: number }[] }>(packages: T[], resumeFrom: number,
  limits?: { packages: number; files: number; bytes: number }): Array<{ start: number; end: number }>;
export function intakeIntent(choice: { preserveOnly: boolean; music: boolean; audio: AudioChoice }): IngestIntent;
export function packageDecisions(packages: Package[], plans: Plans): { needsDecision: string[]; ready: string[] };
export function albumRoot(paths: string[]): string | undefined;
export function isDigitalPackage(names: string[]): boolean;
export function discFromPath(path: string): number | undefined;
```

Unit tests check:

- Batch boundaries at exactly 64 packages, 10,000 files and 12 MiB.
- One package larger than a limit still goes alone, rather than looping forever.
- A resume continues at the saved count.
- Each intent combination produces the documented request.

`discFromPath` needs a decision first (see [findings](#findings-from-planning)).

### 6. Check result wording (`processing-workspace.tsx`, 15 commits)

`resultLabel` turns an integrity, structure or catalog result into the sentence a
curator reads, such as "Exact catalog match (CRC32 only; not cryptographic
identity)" or "Partial identification only". `AGENTS.md` requires these claims to
stay separate and never claim more coverage than was computed. No test reads
them. Move `resultLabel`, and the rule for which operations are available, to
`web/src/check-results.ts`. Its unit tests assert properties, not wording:

- A partial or ambiguous result never gets the same label as a full one.
- A CRC32-only match never reads as cryptographic identity.
- An unknown result kind falls back to neutral text.

This follows the pattern of `presentation-labels.mjs`.

### 7. Smaller moves

Each of these is already a standalone function in a `.tsx` file, or close to one.

| Rule | From | To | Unit tests check |
| --- | --- | --- | --- |
| `tentativeAssignments`, `matchingProgress`, apply eligibility | `digital-album-matching.tsx` | `album-matching.ts` | filename, disc folder, ±2 s duration and title matching; apply only with every track mapped |
| Library load mode (full, next page, quiet refresh) and which rows to re-read | `app.tsx` (59 commits) | `library-loading.ts` | the three modes; re-read at most 20 rows and stop when nothing is pending |
| `pathDisc`, `orderedAudio`, `tree()` | `import-file-tree.tsx` | `import-file-tree-model.ts` | deep paths fold, sort order, manual order groups |
| Money conversion, `specimenDraft` | `inventory-editor.tsx` | `inventory-drafts.ts` | currency decimal places, safe integer limit, invalid currency code gives a readable error, disposed copies keep their state |
| Selection and request body | `library-preparation.tsx` | `preparation-selection.ts` | "all matching except" never sends excluded entries |
| Paging boundary and per-entry outcome lines | `work-history.tsx` | `work-history-model.ts` | stops at the boundary entry; "not eligible" outcomes |

Time saved by Phase 1: checking a changed rule drops from rerunning a
76–86 s journey (and everything before the step) to a unit run under 1 s. This is
the largest saving for day-to-day work, though it barely changes the full suite's
time.

Risks:

- **Behaviour drift during the move.** Move code verbatim first, then tidy it in
  a separate commit. Run the journey that covers the component once after each
  move.
- **Testing wording instead of behaviour.** `docs/testing.md` forbids copy-only
  tests. Unit tests assert decisions: which choice, which request, whether two
  states stay distinguishable. They assert labels only where the label is the
  claim, as with check results.
- **Modules that slowly start importing `runtime`.** The bundle check in
  `load-source.mjs` fails the test when that happens.

## Phase 2: start journeys from prepared data

### Options considered

**Commit a saved archive and state folder.** This is the fastest to start, but
it doesn't fit this project:

- Before a stable release, storage formats change together with the code
  (`AGENTS.md`). Each change would need the fixture converted, or the server
  would refuse it. A silent reset is not allowed.
- The fixture would also carry a TLS key and an account database.

The repo already commits old-format data on purpose for upgrade tests
(`crates/collector-store/tests/fixtures/schema-1`). That is the right use for
committed fixtures: proving a conversion. It is the wrong use for journey
setup.

**API-level seeding.** A Node script signs in over HTTP and creates entries and
imports through the public API, with no browser:

- `PUT /items/{id}` makes entries without files.
- `POST /ingests` plus the file uploads makes real imports.
- It then polls until the jobs finish.

`fx05.mjs`, `browse.mjs` and `cd-library.mjs` already do this from inside the
page with `page.evaluate(fetch…)`. It runs the real application code, so the
data is honest. States that only background jobs can reach still cost job time,
but no browser time. Examples are provider matches, automatic grouping and saved
formats.

**A debug-only seeding endpoint.** This would write records straight to the
store. It is the fastest, but it bypasses application services. It could create
states the real code can never reach, which `AGENTS.md` calls "a mock that looks
like a working feature". It also puts store access in an HTTP handler, which
`scripts/check_architecture.py` rejects. The existing debug-only hooks
(`JUNK_COLLECTOR_TEST_*_ORIGIN`, compiled only with `debug_assertions`) only
point providers at local stand-in servers. Keep it that way.

### Recommended: scenario snapshots built by the binary under test

1. Add `web/tests/support/scenarios.mjs`. It defines named scenarios as recipes.
   Each recipe is a Node function that seeds data through the HTTP API, as
   above, and waits for jobs to settle. Examples:
   - `music-shelf-signed-in`: one account and one music collection.
   - `two-purchases-disc-one`
   - `four-discs-separated`: the state after automatic grouping and undo.
   - `review-grouping-proposals`
2. `prepareScenario(name)` does the following:
   - Runs the recipe against a fresh server.
   - Stops the server cleanly.
   - Copies the archive and state folders to
     `target/test-scenarios/<name>-<key>/`.
   - The key is a SHA-256 hash of three things: the server binary, the recipe's
     source, and the fixture files it reads.
   - A later run with the same key reuses the copy.
3. `startFromScenario(name)` copies the snapshot into a new temporary folder and
   writes a config with fresh ports. It then starts `serve` and returns the
   origin and the test password. The copy is a copy-on-write clone where the
   filesystem supports it: `cp -c` on APFS, `cp --reflink=auto` on Linux.
   Otherwise it is a normal copy.
4. The snapshot is always written by the same binary that will read it. So a
   storage format change never meets an old snapshot: the binary hash changes,
   and the snapshot is rebuilt through the real API. There is no conversion to
   maintain and nothing to reset silently. Old snapshot folders are deleted by
   key, only under `target/test-scenarios/`.
5. In CI, the snapshots build once per job, before the journeys run. Locally they
   persist between runs until the binary changes. A server rebuild invalidates
   them, so the local saving mainly comes from rerunning journeys and scenarios
   (often only web assets changed). Rebuilding a snapshot costs about what the
   setup costs today.

Spike first, before converting any journey. Start a server on a copied archive
and state folder, and confirm three things:

- The archive identity check passes. The identity is a file inside the archive,
  compared by content, so it should.
- The catalog checkpoint is reused, not rebuilt. If copying changes the file
  details the checkpoint compares, the first start rebuilds the catalog. Measure
  that cost, and preserve modification times if it matters.
- No pending upload or job refers to the old path. Snapshot only settled states:
  the recipe waits for every job to finish.

If the copy fails any check, fall back to the supported `backup` and `restore`
commands, at the cost of their full validation pass.

Time saved by Phase 2: a scenario starting from a snapshot pays about 3 s
(copy, serve, browser, sign-in) instead of its setup. For the scenarios in Phase 3,
that removes 10–75 s of imports and job waits before the first step under test.
The full suite saves little until journeys are split, because each journey's
setup is mostly the thing it tests.

Risks:

- **A snapshot hides a setup bug.** The import and grouping paths that build
  snapshots must still be covered end to end. The release journeys keep doing
  that from an empty server.
- **Disk use in `target/`.** Snapshots are small (seconds of audio, a few
  entries). The key includes the binary hash, so stale ones are easy to prune.
- **Order dependence.** Every scenario starts from a fresh copy, so scenarios
  can't depend on each other's leftovers.

## Phase 3: run one scenario alone

Today a long journey is one script, and each section depends on the state the
previous one left behind. For example, "Review removes an entry and the Library
deletes it" can only run after:

- importing four discs;
- waiting for automatic grouping (up to 90 s);
- undoing the grouping;
- restarting the server.

All of that sits inside the `intake-review.mjs` run that only `AUTO_GROUPING_ONLY`
selects.

Steps:

1. Split long journeys into scenario functions that take `{page, origin,
   server}`, following the pattern `intake-review.mjs` already uses for
   `intakeReviewJourney`, `inboxCleanupJourney` and
   `automaticAlbumGroupingJourney`. Each scenario names the snapshot it starts
   from.
2. Add `web/tests/scenarios.json`, mapping scenario names to a module, an
   exported function and a snapshot. Extend `run-suite.mjs` to accept a scenario
   name as well as a journey name: `npm --prefix web test -- review-removes-entry`.
3. `run-suite.mjs` runs each scenario in its own process with its own server, as
   journeys do now. A scenario is selected by name, so there are no hidden
   environment flags. The `AUTO_GROUPING_ONLY`, `GROUPING_AMBIGUITY` and
   `INGEST_PROGRESS_ONLY` variants become named scenarios, so the suite and CI
   can see them.
4. A journey becomes a list of scenarios plus the setup it still owns. Run whole,
   it can keep sharing one server between its scenarios for speed. Run alone, a
   scenario starts from its snapshot.

First splits, by measured cost and how often they are rerun:

| Scenario | Snapshot | Today it starts after |
| --- | --- | --- |
| `review-removes-entry` (Review → remove → Library → delete permanently) | `four-discs-separated` | four disc imports, automatic grouping, undo, restart |
| `album-copy-choice` (choose a copy, then a format, and play) | `two-purchases-disc-one` | about 690 lines of `digital-purchases.mjs` |
| `combined-album-delete` (delete one copy, then everything) | `combined-two-copies` | the same, plus the copy choice |
| `grouping-ambiguity` | `four-discs-ambiguous` | four disc imports, with a different provider answer |
| `replace-originals-while-playing` (fx05) | `music-shelf-with-entry` | sign-in, collection creation, one import |

Time saved by Phase 3: `review-removes-entry` drops from the whole
grouping run (imports, about 11 s of grouping, undo, restart: an estimated 30 s or
more, before the failure described in the findings) to about 10 s.
`album-copy-choice` drops from about 86 s to about 12 s.

Risk: a scenario passes alone but the full flow breaks between steps. The release
journeys still run every step in one go.

## Phase 4: shorter waits and journeys side by side

These changes cut the time of the full suite, not only of a rerun.

### Provider pacing against local stand-ins

Correction: this was already done. With a test origin set, debug builds give
MusicBrainz a 1 ms interval and AccurateRip none (`with_test_origin` in
`crates/collector-providers`). The waits come from the server's recovery loop
instead: automatic album grouping and metadata upkeep run on a 5 s cycle
(`start_job_workers` in `crates/collector-server/src/lib.rs`), and grouping needs
more than one pass. Debug builds now read
`JUNK_COLLECTOR_TEST_RECOVERY_INTERVAL_MS` (50–5000) for that cycle.
`intake-review.mjs` sets 500 ms; every other journey keeps the real 5 s, so the
real cadence stays covered. It changes only how soon a pass runs.

### Run journeys side by side

1. Replace fixed listener ports with `fixturePort()` in the 18 or so journeys that
   hard-code them. The list comes from
   `grep -l '127.0.0.1:[0-9]\{4,5\}' web/tests/*.mjs`. Some journeys bake the
   origin into passkey or certificate expectations; those stay serial until
   checked.
2. Give `runJourneys` a concurrency limit (`COLLECTOR_TEST_JOBS`, default half the
   CPU cores). Each journey already has its own process, server, data folder and
   browser.
3. Keep the journeys that measure timing or play real audio
   (`opus-preparation`, `local-playback`, `fx04` scale, `remote-media`) in a
   serial group, so contention can't make them flaky.

With most of the suite's time spent waiting rather than computing, four journeys
at a time should bring 12.5 min down to roughly 4–5 min. `integration` (at
least 151 s) and the three journeys of about 90 s set the floor until they are
split in Phase 3.

Risk: timing-sensitive waits that pass alone and fail under load. Run the
parallel suite several times before making it the CI default. Anything that
fails only in parallel gets fixed or moved to the serial group, not retried.

### Waiting on the screen's refresh timers

Journeys wait for 2 s list refreshes and 5 s availability refreshes. Where a
journey only needs fresh data and isn't testing the refresh itself, it should
reload or reopen the view instead of waiting for the timer. Leave the waits that
test refresh behaviour, and the deliberate wait in `fx05.mjs` that crosses the
player's access recheck. Don't fake the clock (Playwright's `page.clock`) against
a real server. That would make the screen's timing differ from what users get.

## What stays end to end

`AGENTS.md` says to check the real screen. These still run whole, from an empty
server, through the real UI, before every release and in CI:

- `integration` (`smoke.mjs`): first sign-in, collection creation, import,
  playback, curation and administration across the app.
- `purchases` (`digital-purchases.mjs`), with its later sections trimmed once
  Phases 1 and 3 cover them. Purchased-file intake, resumable upload and playback
  of the result stay.
- `intake-review` and the automatic album grouping run: real inbox import,
  provider settlement, grouping, undo and restart persistence. This is the path
  that builds the grouping snapshots, so it must also run from scratch.
- `opus-preparation` and `local-playback`: real encoders and real audio output.
- `fx05`: replacing originals while a track keeps playing.
- `cd-library`: a CD import with AccurateRip and a library batch.
- `browse` and `navigation`: back and forward, scroll restore and paging.
- Mobile-width checks already inside these journeys (no horizontal overflow,
  screenshots).
- `smoke`, `pair`, `certificate`, `numeric-ip`, `storage`: sign-in, trust and
  listener behaviour, which depend on the real server and browser.

Everything else moves, one rule at a time, to unit tests (Phase 1) or to
single-scenario journeys (Phase 3). A journey assertion that only re-checks a
rule now covered by a unit test is deleted once the unit test lands. The one
assertion that shows the screen uses the rule stays.

## Order and expected savings

Each step can land alone, and later steps don't need all of the earlier ones.

| Order | Step | Effort | Expected saving | Main risk |
| --- | --- | --- | --- | --- |
| 1 | Phase 0: `load-source.mjs`, `node:test`, `test:unit` in CI | half a day | none directly; the base for the rest | low |
| 2 | Phase 1 items 1 and 2 (album copies, removal) | 1–2 days | checking these rules: 77–86 s journey → under 1 s | changed behaviour in the move; mitigated by moving code verbatim |
| 3 | Phase 3 registry with no snapshots: the hidden `AUTO_GROUPING_ONLY`, `GROUPING_AMBIGUITY` and `INGEST_PROGRESS_ONLY` runs become named scenarios | half a day | no time saved, but these runs become visible to the suite and CI | the grouping run fails today and must be fixed first |
| 4 | Phase 4 ports and concurrency | 1 day | full suite 12.5 min → about 4–5 min | contention flakiness |
| 5 | Phase 1 items 3–6 | 2–3 days | same kind of saving for Review, metadata, import review and check results | as step 2 |
| 6 | Phase 4 provider pacing hook | half a day, after counting requests | 10–25 s per intake or grouping run (to confirm) | must stay debug-only and tied to a test origin |
| 7 | Phase 2 spike: start a server on copied folders | half a day | decides between copying folders and `backup`/`restore` | catalog rebuild on start |
| 8 | Phase 2 snapshots and Phase 3 splits, one scenario at a time | 1 day for the harness, then about an hour per scenario | 10–75 s per scenario rerun | setup paths losing end-to-end coverage; the release journeys keep it |
| 9 | Phase 1 item 7 and trimming journey assertions that unit tests now cover | ongoing | a few seconds per journey | deleting the assertion that proves the wiring |

Steps 1–4 give most of the benefit for day-to-day work (fast rule checks and a
suite under 5 minutes) without any server change.

## Findings from planning

These turned up while measuring. They are bugs or gaps in today's tests or
code. Those marked fixed are fixed; the rest are left for a decision or a
separate change.

- **Fixed: CI never ran the plain Node model tests.** The default `npm test`
  skips `queue`, `context` and `labels`, and CI ran only `npm test`. CI now also
  runs `npm --prefix web run test:models`.
- **Three journey variants run nowhere.** `AUTO_GROUPING_ONLY`,
  `GROUPING_AMBIGUITY` and `INGEST_PROGRESS_ONLY` in `intake-review.mjs` are set
  by no suite and no CI step. That includes the only browser check of "Review
  removes an entry and the Library deletes it". Step 3 above makes them named
  scenarios.
- **Fixed: automatic grouping lost its undo moments after finishing.**
  Grouping kept the first disc's copy of the release, then the metadata
  maintenance pass re-picked among equal copies and saved a new revision, so
  undo reported "Metadata changed after grouping; automatic undo is
  unavailable." Grouping now saves the same choice the maintenance pass would
  make. The grouping run must still pass before step 3 adds it to CI.
- **Fixed: `integration` failed on macOS at a playback wait.** Live FLAC
  streamed from FFmpeg left its length unknown, and Chrome then dropped the last,
  shorter block of audio, so a 0.133 s test track never played past 0.1 s. Live
  FLAC made from PCM now records its exact length.
- **Three disc-folder patterns disagree.**
  - `digital-album-matching.tsx` accepts `Disc.2` and at most two digits.
  - `import-file-tree.tsx` (`pathDisc`) rejects `.` and takes any number.
  - `library-imports.tsx` (`albumRoot`) rejects `.`.
  - So one folder name can get a disc number in album matching but not in the
    import tree.
  - Picking one rule is a product decision. Phase 1 item 5 then gives it one home
    (`discFromPath`) and a unit test.
- **Fixed: "Possibly the same album" opened on its own only when the entry was
  in the loaded list.** It now decides from the entry's own review reasons.
- **Fixed: the "Group into one album" button allowed more than 32 entries.** It
  now applies the same limit as the grouping panel and says why when it can't.
- **Fixed: one disc of a grouped album could fail its artwork step.** Discs of
  one album resolve metadata in parallel jobs that save to the same entry. When
  one saved between another's revision read and its save, the second failed with
  "revision conflict" and the import showed as failed. The job's provider save
  now reads the entry again and retries a few times. A faster server made this
  happen in most `intake-review` runs.
- **Fixed: `digital-matching` assumed track 1 is looked up first.** Each track's
  AcoustID lookup starts when its fingerprint is ready, and lookups run one at a
  time, so either track can go first. The stand-in held track 2, which blocked
  the only slot until it timed out whenever track 2 went first. It now holds
  whichever lookup arrives second. (This timeout first looked like optimized
  builds computing different fingerprints. They don't: debug, optimized and
  release builds give bit-identical fingerprints for the fixtures, however the
  audio is chunked.)
- **Fragile, but not a bug today:** `specimenDraft` in `inventory-editor.tsx`
  turns a disposed copy's ownership into `unknown`. A save would lose the
  disposal record. This can't happen now, because disposed copies have no "Edit
  copy" button. The Phase 1 item 7 unit test should pin that a disposed copy
  never becomes a draft with a changed state.
