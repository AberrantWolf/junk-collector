# Collection lists and playlists

**Current direction (2026-10-09):** [remaining work](remaining-work.md)
is the only remaining-work queue. Older assignment/order and gap statements below
are historical. Proposed contracts require reassessment under
[the home-lab scope/retention policy](decisions/home-lab-scope.md); their presence
does not require implementing speculative machinery or permanent lookup history.
Phono Junk is no longer a target. Implemented contracts describe current behavior,
not a claim that cleanup or redesign has already happened.

User assignment: add lists within each collection; playable music lists contain
individual tracks, allow whole albums or discs to be appended, preserve order,
and support playback, rearrangement and shuffle play.

## Delivery CL1

- Collection-shared, portable lists; Viewers browse and play, Curators/Owners edit
  through the curation surface. Music collections contain track occurrences;
  other collections contain Item occurrences. Repeats remain independently movable.
- Create, rename, delete, append, remove, move before another occurrence and move
  to end. Each command binds a receipt to exact submitted inputs and the expected
  list revision. Deletion tombstones the list without deleting collection content.
- Disc/album additions compare their full recorded audio-track scope with available
  source coordinates and block partial expansion with a recovery action. Individual
  available tracks remain addable; complete retained derivatives can still expand
  the recorded disc when original bytes are unavailable.
- Album additions expand supported recorded audio tracks by numeric medium then
  track position. Disc additions select a particular original and append only
  its tracks. Unknown multi-disc placement, missing listed discs and alternative
  originals block whole-album addition with a useful next action; adding available
  discs individually remains possible. No arbitrary source/discovery order.
- The stable source Representation/Capture/track position survives FLAC preparation.
  Current descriptive information and prepared output availability resolve on read.
  Capture links follow confirmed same-collection merges and guarded undo. Removed,
  superseded or unavailable sources remain explicit list entries.
- Ordinary journal/rename archive publication and immutable list revisions; catalog
  v12/checkpoint format 5 rebuild projections and recover receipt outcomes. No new
  private list database or media byte copies. Grant/revision/source currency checks
  are fenced through list publication. Archive copies carry lists, never access grants.
- At most 5,000 entries per list, name at most 256 bytes, list/entry pages at most
  100 rows. Entry cursors bind to the current list revision. Mutations never silently
  truncate. Browser queues now hold at most 5,000 tracks; oversized playlist starts
  leave the existing queue intact.

## UX locations

**Lists** is a full addressable client page, beside Library. Select the collection
in the existing sidebar, create or open a list, then use its persistent action bar
for Play playlist, Shuffle play and settings. Each artwork/text row shows album,
disc, track and typed availability, with entry/preparation links. Keyboard buttons
move up/down, move to end, remove, or play a track. More entries loads bounded pages.
Mobile keeps the same readable rows and wraps actions. Browser Back/Forward and
workspace navigation retain the selected list; reload restores its URL.

Library entry details have **Add entry to list** for ordinary collections and
**Add album / disc / track to list** for music. One focused chooser supports paged
list search and creating a list, and confirms the actual destination after append.
The selected destination stays visible outside search results; obsolete pages are
cancelled through the common operation-scope helper. Unconfirmed create/append
commands keep their original inputs and receipt keys through reload and panel
reopen, scoped to the signed-in user and source. Retry resolves the original command
before another addition; confirmed repeats use fresh commands. The chooser uses
the shared dialog dismissal, focus and scroll behavior.
Opening the saved list is a direct action. Concurrent-edit errors retain the page
and offer explicit reload. Controls stay busy until the new revision/order loads.

Play and Shuffle play freeze a complete bounded set of list pages at one revision
before replacing the personal queue. The UI reports excluded unavailable or
browser-unsupported tracks; they stay in the saved list. Shuffle affects only the
playback queue. The existing shell player owns actual media, survives navigation,
checks current access and releases buffered playback on revocation. Reload restores
a paused queue and never repeats autoplay. Future video support needs a real module
capability and target resolver; this delivery does not pretend video playback exists.

## Acceptance ledger

CL1 is locally accepted on 2026-10-03. Repository gates, archive interruption and
recovery tests, and the real Chrome playlist/shuffle, mobile and keyboard journeys
passed. The 2026-10-04 review repairs and repeated gate results are appended to
[validation](validation/collection-lists.md); the original acceptance record is
retained as dated history.
No commit is authorized by this new implementation request. Wait for the user to
be at the computer before triggering a signing prompt on a later commit request.
