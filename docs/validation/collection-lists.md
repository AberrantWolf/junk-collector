# Collection lists validation — 2026-10-03

Delivery contract: [collection lists](../collection-lists-plan.md).

## Focused evidence

- Portable list validation checks occurrence identities, repeated targets, kind
  mismatch, name bounds and the exact 5,000-entry boundary.
- Store integration tests exercise Curator writes/Viewer reads and denied writes,
  cross-collection denial, frozen source revision conflicts, command receipt replay,
  duplicate append occurrences, rename/move/remove/delete, revision-bound cursors,
  restart and explicit reindex. A removed original leaves a visible unavailable
  occurrence without reading/hashing audio during list browsing.
- Six ordinary archive publication fault boundaries recover list revisions and
  command outcomes with exactly one intended creation.
- Generic game lists contain Items. Unknown multi-disc placement blocks whole-album
  append; individual disc/track choices work, and nonexistent track positions fail.
- Lists follow merged Capture ownership and guarded undo; missing modules and removed
  Items retain explicit unavailable occurrences.
- HTTP tests establish `/lists` and authorized list reads on the client listener,
  absent mutation methods on a client-only listener, and Viewer mutation denial even
  on the private listener.
- Queue tests cover per-user persistence/isolation, stable reordering, reload without
  autoplay, bounded replacement, signed-out behavior and disabled browser storage.

The complete workspace run exposed a catalog-version guard still comparing with
v11, which could recreate a missing table and then reuse an incomplete v12
projection. The opener and migration now share one v12 constant; damage regression
cases include the list table and its checkpoint trigger. The old fixture's expected
projection version advances to 12 because lists add a real projection, while its
archived fixture bytes and private v18 data remain unchanged. OpenAPI route coverage
likewise advances from 132 to 134 paths for the actual new list APIs.

## Browser and gates

CL1 is locally accepted. The first integrated browser run correctly blocked an album
with an unlinked disc: its provider fixture changed release contents as requests
arrived. The offline fixture now advertises its complete two-medium release with
stable computed disc IDs before either import. A subsequent journey found a quick
consecutive-edit race; mutation controls now remain busy until the refreshed list
revision and order are loaded. Final browser verification passed that sequence.

Final repository results:

- `python3 scripts/check_architecture.py` and `cargo fmt --all -- --check` passed.
- `cargo test --workspace` passed every suite: archive integration reported 204
  passed, 0 failed and 2 ignored in 171.33 seconds; HTTP integration reported 30
  passed. The focused checkpoint-damage and actual-data migration regressions passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `npm --prefix web run check`, `build`, `test:queue` and `check:api` passed.
  Generated OpenAPI and TypeScript match the 134 annotated API paths.
- `cargo build -p collector-server` rebuilt the server with current embedded web
  assets. `git diff --check` passed.
- `npm --prefix web run test:smoke` passed on Google Chrome 153.0.8010.37 using
  temporary local profiles and a virtual WebAuthn authenticator. It exercised
  album/disc/track append, repeated occurrences, keyboard reorder, consecutive
  edits, removal, rename/delete, conflicting edits and reload, mobile layout,
  actual playlist and shuffle playback, and paused reload. The complete existing
  import/group/preparation, playback, role, artwork and WebAuthn journeys also passed.

Desktop and mobile list screenshots were visually inspected. The final small
refinement displays the selected collection beneath the Lists heading; TypeScript,
web build and embedded-server rebuild passed after it. No commit was made.

No live provider behavior changed or was requalified for this delivery. Tests use
isolated local profiles and offline responses; actual stored collection data,
credentials, grants and server configuration are untouched. Chrome fixture playback
is not qualification of other browsers, video, NAS, multi-device synchronized
playback or general external-file repair.

## Review repairs — 2026-10-04

The independent review reproduced four defects: filtering hid the submitted
destination, delayed pagination contaminated a new query, unavailable originals
shrunk disc additions to their prepared subset, and reloading after an uncertain
append discarded its receipt and could duplicate the occurrence. The chooser also
duplicated existing dialog behavior.

Repairs keep the selected destination visibly pinned through searches, use the
existing operation scopes for reads/commands, and use the shared dialog dismissal
hook. Unconfirmed create/append inputs and receipt keys remain frozen through
reload and panel reopen in user/source-scoped session storage. Definite command
rejections allow a revised submission; uncertain outcomes replay the same receipt.
Reload also reads a selected destination outside the current page/search, so a
conflict does not trap a paginated selection at its stale revision.

Disc/album addition now compares the full recorded audio-track positions with
availability coordinates. Incomplete additions fail explicitly without changing
the list; individually available tracks remain addable. Complete retained
derivatives can still expand a recorded disc when its original bytes are missing.
This reuses the existing fingerprint-bound recorded layout resolver and reads no
audio payload for the new currency check.

Permanent regression coverage:

- `collection_lists_block_partial_disc_expansion_when_originals_are_missing` tests
  disc and album additions with zero, one and both tracks prepared, then restoration
  of the original. The six focused collection-list integration tests pass.
- `npm --prefix web run test:lists` exercises the actual chooser in Chrome with
  controlled responses: visible selection, superseded pagination, modal focus/scroll
  and Escape behavior, uncertain append recovery through page reload and panel
  reopen, uncertain creation recovery, intentional repeats, conflict recovery
  for a selected list outside the current search, and a destination change during
  a delayed exact refresh.
- The real-server smoke journey now loses a response after a committed append and
  verifies that reload/retry replays the identical command and creates one occurrence.

Integrated results after the repairs:

- Architecture/link/fixture checks, Rust formatting and `git diff --check` passed.
- `cargo test --workspace` passed all suites. Archive integration reported 205
  passed, 0 failed and 2 ignored in 178.08 seconds; HTTP integration reported 30
  passed. The ignored private-oracle checks retain their existing input requirements.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- Web TypeScript, build, API-contract, queue and new chooser checks passed.
- `cargo build -p collector-server` rebuilt the final embedded assets.
- The final complete `npm --prefix web run test:smoke` passed on Google Chrome
  153.0.8010.37 with temporary profiles and the virtual WebAuthn authenticator,
  including the real committed-append response-loss/replay check and existing
  bulk intake, metadata/grouping, actual FLAC playback, roles, revocation and mobile
  workflows. The final delayed-refresh guard also passed its isolated chooser test.

Status, architecture, delivery and handoff ledgers reflect these repairs. Original
dated acceptance evidence above remains historical; no live provider, owned-corpus
or second-device requalification is claimed by these fixture repairs. No commit was
made.
