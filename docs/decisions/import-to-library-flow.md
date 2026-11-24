# From import to finished entry: one flow

Status: proposed, not yet reviewed by the owner.

This proposes how bringing things into the library *should* work, from picking
files to having a correctly named album or game with artwork. It is not a list
of small fixes to the current Imports and Metadata screens. It is based on the
current web code (`library-imports.tsx`, `guided-ingest.tsx`,
`library-metadata.tsx`, `metadata-review.tsx`, `library-activity.tsx`) and on
how the server decides what "needs attention".

## What goes wrong today

### 1. "Needs attention" means two different things

The Imports screen and the Metadata screen each count problems, but they count
different things and never show each other's results.

| Screen | Counts as needing attention |
| --- | --- |
| Imports (per batch) | A package the server refused, a failed import step, a failed automatic metadata lookup, or failed audio preparation |
| Metadata (per entry) | No name, no artwork, more than one possible match, incomplete online info, no match, a failed lookup, your manual name differing from the match, or a possible album group |

So an import batch can say "0 need attention" while every album in it has an
unconfirmed match. And when a batch does say "3 need attention" because
automatic lookup failed, you can only fix that on the Metadata screen, which
doesn't know which import the entry came from.

### 2. The thing you care about is the album, but the screens are built around batches and jobs

You think "did *Harbour Lights* come in right?". The Imports screen answers "batch
from 14:02, 40 packages, 38 originals saved, 2 need attention", and you have to
open the batch and scan 40 rows to find the 2. The **Review import** dialog then
shows the import's internal steps (copying, analysis, publishing), file
analysis notes and technical IDs. Most of what it reports can only be fixed
somewhere else.

### 3. The same work shows up in too many places

An import can be started from the **Add / import** dialog in the Library or from
the **Imports** page. These are two different importers with different options.
Once it's running, the same import appears in **Imports**, in **Activity**
("Import batches") and in **Work & history**. Fixing the result happens in
**Metadata**, or in the entry editor's **Advanced** tab, which has its own
"Manual import review" and "Processing history". Nothing says which of these is
the place to look.

### 4. Metadata rows don't say what's wrong

The server already stores, for every entry, which problems it has (the
`metadata_health` table keeps one yes/no flag per reason). But the list rows
(`MetadataCareEntry`) don't include those flags. A row only says "Unconfirmed
names" or "Unconfirmed match". Missing artwork, no match, failed lookup and
name conflicts show nothing. The only way to see the reason is to filter by it
or open the entry.

### 5. Nothing can be marked "this is fine"

If an album truly has no cover, or you deliberately renamed it, it stays in
"Needs attention" forever. A to-do list that can never be emptied stops being
trusted, and real problems get lost among permanent ones.

### 6. Paging you have to remember to do

- Import batches: 50 at a time, then **More imports**.
- Packages inside a batch: 32 at a time, then **More packages**. The page also
  reloads every page you've opened, one after another, every 2 seconds.
- Before starting an import, packages are reviewed 32 at a time with
  **Previous / Next packages**.
- Metadata entries: 50 at a time, then **More entries**.

When you're checking whether an import finished, a problem on row 33 is
invisible unless you think to click.

### 7. The entry fix panel puts tools before answers

The Metadata panel opens with a search tool, then title and subtitle mode
switches, then the list of matches, then artwork. The "Needs attention" box is a
plain bullet list with no link to the part of the panel that fixes each item.

## The proposed model

### The album or game is the unit, not the batch

Every incoming thing becomes one **entry** with one status, computed by the
server from *both* the import side and the metadata side. Batches still exist,
but as a filter ("show what came from this folder"), not as the main list.

### One status per entry, in plain words

Status moves forward through these stages:

1. **Waiting for files** – the upload stopped; reselect the folder to continue.
2. **Saving originals** – copying and checking the files.
3. **Identifying** – looking up names, tracks and artwork.
4. One of:
   - **Added** – nothing left to do.
   - **Needs you** – with the specific reasons.
   - **Couldn't import** – with the reason, and whether the originals were
     saved anyway.

Two facts are always shown separately, because the project already treats them
as separate claims: **are the original files safely saved?** and **do we know
what this is?** A small "originals saved" mark next to every row means a
naming problem never looks like data loss, and data loss never hides behind a
naming problem.

### Problems come in three levels

Not every reason is equally urgent. Putting them in one list is part of why the
list feels endless.

| Level | Reasons | Shown by default |
| --- | --- | --- |
| **Blocked** | Package refused, upload stopped, an import step failed | Yes, at the top |
| **Identity** | No match, several possible matches, lookup failed, unconfirmed names, your name differs from the match, possible album group | Yes |
| **Polish** | No artwork, incomplete online info | Only when you pick the "Polish" filter |

### "This is fine" is a real choice that's remembered

Each Identity or Polish reason can be accepted as it is: "no cover exists",
"my name is correct", "these are separate albums". This is saved as one of your
manual choices, so a later automatic refresh keeps it, like manual names do now.
It is tied to what the entry looked like when you accepted it: if a new disc is
added or the match changes, the reason comes back. Blocked reasons can't be
accepted, only fixed or discarded.

## Proposed screens

### Sidebar

```
Organize
  Add to library        (the one importer)
  Review            12  (count of entries that need you)
  Purchase inbox
  Prepare audio
  Activity              (technical job history only)
```

**Imports** and **Metadata** go away as separate pages. **Review** replaces both.
The Library's **Add / import** button opens **Add to library**, not a second
importer in a dialog.

### Review: the queue

```
Review                                   Collection: Shelf & stereo ▾
┌──────────────────────────────────────────────────────────────────┐
│ Needs you 12 │ In progress 23 │ Added recently 140 │ Polish 31   │
└──────────────────────────────────────────────────────────────────┘
From: [All imports ▾]   Reason: [Any ▾]   Search: [            ]

■ BLOCKED
 ▢ [art] Live at the Hall (disc 2)    ⚠ Upload stopped     [Resume upload]
 ▢ [art] mystery_dump_07           ✕ Couldn't import · originals saved
                                     Not a recognised disc image [Open]
■ IDENTITY
 ▢ [art] Harbour Lights                ? 3 possible releases [Choose]
 ▢ [art] Untitled (track01.flac…)  ? No match found      [Search]
 ▢ [art] Night Songs / Night Songs… ⧉ Possibly one album  [Compare]
```

- The tabs are counts from the server, so you see at a glance whether anything
  is still running, without opening batches.
- **In progress** updates by itself, has one progress bar per running import
  and per entry, and drops entries into **Added** or **Needs you** as they
  finish. It never needs paging because it only holds what's moving.
- **Added recently** is the reassurance list: what came in cleanly, with links
  into the Library.
- Rows show the reason and one main action, so most fixes are a single click.
- Selecting several rows allows bulk actions: retry lookup, accept as is, group
  into one album.
- Long lists load more rows as you scroll, with "Showing 50 of 312" at the top.
  Nothing is hidden behind a button you might not notice.

### Review: the fix panel

Clicking a row opens one panel that handles *every* kind of problem for that
entry, whether it came from the import or from metadata.

```
Harbour Lights                                    originals saved ✓
Example Band · 1 disc · from "Music drop" import

What needs you
 1. Three releases match these 17 tracks.        ← jumps to section
 2. No cover yet.                                ← jumps to section

Which release is this?
 ┌ 1969 UK LP ──────────┐ ┌ 1987 CD ───────────┐ ┌ 2019 remaster ─┐
 │ 17 tracks ✓ lengths  │ │ 17 tracks ✓ lengths│ │ 17 tracks ~    │
 │ matches your files   │ │ disc ID matches ✓  │ │ 2 lengths off  │
 │ [Use this]           │ │ [Use this]         │ │ [Use this]     │
 └──────────────────────┘ └────────────────────┘ └────────────────┘
 Not here? [Search again]

Cover   [choices from the chosen release]  [Upload]  [No cover exists]

Name    Harbour Lights (from the release)  [Change]

▸ Import details (steps, files, retry, move to another entry)

[Skip]                                   [Accept and go to next ›]
```

- The panel starts with what's wrong, numbered, each linking to its section.
- Choosing a match is the main job, so matches are compared side by side
  *against your files* (track count, track lengths, disc ID), not as a long
  list of provider records. The raw field comparison moves under a details
  toggle.
- Name and artwork follow from the chosen match. Changing them is a secondary
  action, not two "Automatic / Manual override" mode switches.
- Import problems (stopped upload, failed step, wrong destination) live in the
  same panel under **Import details**, so the old **Review import** dialog is no
  longer needed.
- **Accept and go to next** moves through the queue, so working through 12
  problems is 12 decisions in a row, not 12 trips back to a list.
- The same panel opens from a Library entry's **Fix metadata** action, so
  there is one place to fix an entry no matter where you start.

### Add to library: one importer

```
1 Choose            2 Check                     3 Start
[Folder] [Files]    40 albums found             Automatic names & art  ☑
[Server inbox]      ✓ 36 ready                  Audio format: FLAC ▾
                    ⚠ 3 need a decision  ▸      [Start import]
                    ✕ 1 skipped (no audio) ▸
```

- One importer, used everywhere. It has the options from both of today's
  importers.
- **Check** shows a summary, and opens only the packages that need a decision
  (choose which files to play, confirm a guessed album group, label physical
  copies). The other 36 stay collapsed in one table without paging.
- The server already proposes album groups (`automatic_album_grouping`), so
  the check step suggests "these 2 folders look like discs of one album"
  instead of asking you to type matching "album group" keys into text boxes.
- After **Start**, the page goes straight to **Review → In progress**, filtered
  to this import, and the import continues on the server if you leave (except
  for browser uploads, which still need the tab open; the page says so
  clearly and only while an upload is actually running).

### Activity

Activity keeps the job history, worker state and bulk-operation logs for
diagnosis. It stops listing import batches as a third place to check imports.

## Games, music and future collection types

The flow above is shared: the same stages, the same three levels, the same
Review queue and fix panel. What changes per collection type is what
"identifying" means, which problems exist, how serious each one is, and what the
fix panel shows. The collection type (its module) should supply those, so a
books or video collection later adds its own reasons and panel sections without
another round of "album" wording leaking into it.

### How games differ

| | Music | Games |
| --- | --- | --- |
| What "identifying" does | Online lookup (MusicBrainz), then cover art | Checks the files against your saved game catalogs (DAT files); no online lookup |
| Main identity question | Which release is this? | Which game, system, region and version is this, and do all checksums match? |
| Evidence shown when choosing | Track count, track lengths, disc ID against your files | Checksum coverage (all checksums vs CRC only), system, region, version against your files |
| Automatic artwork | Yes, from the matched release | None. No game artwork source exists |
| Import decisions | Album groups, copy labels, which files to play, audio format | Main file (for example which CUE sheet), copy details (cartridge, box, manual) |
| Grouping | Discs of one album | Discs of one game (multi-disc games). Not supported today |

### Game problems and their level

| Reason | Level | Notes |
| --- | --- | --- |
| No main file, or several CUE sheets | Blocked | Needs a choice before analysis can run |
| Several catalog entries match | Identity | Choose one; show region and version side by side |
| Only a weaker match (CRC only, or partial) | Identity | Choose it or accept as is; never applied automatically |
| Format recognised but not supported (CHD, UNIF, compressed) | Identity | Originals are saved; name it manually and accept |
| No catalog loaded for this system | Setup | Not a problem with the game. See below |
| No cover | Polish | Upload a scan or photo of your copy |

**Setup gaps are one notice, not many rows.** If 14 PlayStation games are waiting
because no PlayStation catalog is loaded, Review shows one notice ("14 games are
waiting for a PlayStation catalog → Game catalogs") instead of 14 problem rows.
When the catalog is activated, those games are checked again automatically,
which the server already does.

### Places where games are treated like albums today

- **No cover is a permanent problem for every game.** The server flags any entry
  without artwork, whatever its type. Games have no automatic artwork source, so
  every game without an uploaded cover sits in "Needs attention" with no
  automatic fix.
- **Game identification results aren't reasons.** Unrecognised, ambiguous and
  CRC-only results show inside the Metadata panel, but the list's reasons are
  all music-shaped. A game with no catalog match usually shows up as "Missing
  name", which doesn't say why.
- **Music wording in shared screens:** Metadata rows say "No artist or edition
  details" for games. The reason filter offers "Possible album groups" in game
  collections. Activity says "N albums" for every batch.
- **Library filters:** "Playable audio" and "Album playback needs attention" are
  offered in game collections.
- **The entry detail view** has a tailored layout for albums ("Album details &
  files", a management menu, track list). Games get the plain leftover layout.
  A game deserves its own: system, region, catalog check result and coverage,
  copy completeness (cartridge, box, manual), and a soundtrack section when
  the disc has CD audio.

### The player bar

**Today:** the player bar shows when the selected collection or entry can be
listened to, when the "Playable audio" filter is on, or when the play queue has
any tracks. The queue is saved in the browser, so once you've queued anything
the bar stays on every page, including game pages, even when nothing is
playing. Game discs with CD audio also count as "can be listened to", so opening
such a game shows the bar too.

**Proposed:** a general media slot in the layout, which a player fills only
while it has something to show.

- The layout renders one `<MediaDock/>`. Nothing else in the layout knows about
  music.
- A player registers with the dock when it has something to show: music today,
  later video or an emulator. Each provides a compact view for the dock and an
  optional expanded view. Only one is active at a time, and starting one pauses
  the other.
- The dock shows only while something is playing, or was paused recently in this
  tab. Browsing music no longer forces it open; pressing Play does.
- When a saved queue exists but nothing is playing, a small "Resume queue (12)"
  button sits in the header instead of a full-width bar.
- The music player code loads only when first needed, so a games-only session
  never downloads it.
- A game with CD audio gets a **Soundtrack** section in its detail view. Playing
  it uses the same dock, and the dock disappears again when playback stops.
- An emulator would be a full-screen view rather than a dock. The dock would
  only show "Running: *game*" with a way back to it.

## What the server would need

None of this changes how files or originals are stored.

1. **One status per entry, with reasons.** A summary the server builds from the
   import records (whether originals were saved, failed steps, upload stopped)
   and the existing metadata flags, plus failed audio preparation. Imports that
   failed before an entry existed need a row too. List rows include the reasons
   and their level. This is read from the existing databases, without reading
   file contents, so it fits the rule that browsing never hashes or reads full
   files.
2. **Counts per tab and per reason** for a collection, and optionally for one
   import, so the tabs and sidebar badge are cheap to show.
3. **Accepted reasons**, saved as manual choices and tied to the current state of
   the entry, so they come back if that state changes.
4. **A "what changed since" read** for the queue, so the page doesn't reload
   every visible page every 2 seconds.
5. **Reasons come from the collection type.** Each module lists its reasons
   with a label, a level and the fix-panel section that resolves them, instead
   of the web code keeping one hard-coded, music-shaped list. Game
   identification results (ambiguous, CRC-only, unsupported format, no catalog
   for this system) become reasons. Artwork counts as a problem only for
   collection types that say so.
6. **Setup gaps are grouped**: a count of entries waiting on the same missing
   catalog, per system, so Review can show one notice.

## Suggested order

1. Server: reasons in list rows, per-entry status, counts. Show reason labels
   on today's Metadata rows straight away. This is the cheapest big win.
2. Server and UI: "accept as is".
3. The **Review** page and fix panel, built from today's Metadata panel pieces.
   Retire the **Review import** dialog and the batch-member list.
4. One importer: merge the **Add / import** dialog's importer into
   **Add to library**, with the summary-first check step and suggested album
   groups.
5. Remove import batches from **Activity**.
6. Media dock: move the player behind `<MediaDock/>`, show it only while
   something plays, and stop showing music filters in game collections. This
   doesn't depend on the other steps and can happen any time.
7. Game detail layout, and game reasons from the module.

## Open questions for the owner

- Should **No artwork** count as Polish (hidden by default) for every type, or
  as Identity for music only? For games it should not be a problem at all
  unless you opt in.
- Should multi-disc games be grouped like multi-disc albums, using the same
  grouping step with game wording?
- Should an accepted reason be stored with the collection (it travels with an
  exported archive, like manual names) or as private state on this server?
- Should games and music share one Review queue when you're viewing all
  collections, or should Review always be per collection?
- Is **Purchase inbox** another source in **Add to library**, or does it stay its
  own page and only send its results into **Review**?
