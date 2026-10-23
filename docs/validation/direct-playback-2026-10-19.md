# HL-07B direct album and track Play — 2026-10-19

The web now offers Play album, per-import Play original and individual-track Play
beside album/track text, including preparation views. Explicit Play replaces the
personal browser queue and starts immediately. Track Play uses one track; Play
original uses the included album tracks from one explicitly chosen import. Add to
queue remains a separate continuation action. Format downloads and explicit
preparation stay available.

## Boundaries and review

Album/original playback reuses the existing authorized `client-playback` snapshot,
which shares application-owned scope, numeric disc/track ordering, original-choice
and completeness policy with native playback/list expansion. The presentation
refreshes authorized availability and chooses a browser-decodable existing format
for every returned track before atomically replacing the queue. Any unavailable
track rejects the complete selection and keeps the previous queue. No preparation,
provider workflow or new playback policy is composed in the frontend.

Each source is identified beside its tracks. Ambiguous CD imports require an
explicit original choice; unresolved album positions remain application blockers.
Alternate digital encodings retain existing include/exclude review; individual
original tracks can play while the album choice is unresolved. Playback does not
start encoding. The persistent player continues enforcing media authorization,
queue identity, seeking and revocation.

The panel owns cancellable playback reads. Closing/changing the item or starting
a replacement request fences every continuation before queue mutation. It shares
queue-track construction with Add to queue and exports the existing start operation
through the common runtime for separately served management bundles.

## Acceptance

Chrome 153.0.8010.37, temporary local server/profile, test-only self-signed HTTPS:

- `npm --prefix web run test:purchases`: actual FLAC/MP3 album and individual-track
  Play, immediate playback, queue replacement, next-track behavior, paused reload,
  playlists, source-specific manual edits, and normal import/privacy/offline-provider
  behavior. An actual album response is withheld, its panel closed/reopened and a
  new track played; releasing the old response cannot replace the new queue.
- Alternate encodings are imported through the real UI. Album Play refuses the
  unresolved choice without changing the queue; the selected individual MP3 plays.
  Explicit include/exclude review then permits one-track FLAC album Play.
- Browser smoke: actual prepared two-disc album Play follows server numeric order;
  seek, next, individual-track Play and explicit original Play use the persistent
  player. Unprepared album audio keeps the prior queue, with disabled track Play
  and visible recovery guidance. A Viewer starts Play while its request is held;
  actual grant revocation denies the request before release, leaving the queue
  unchanged and stopping authorized media through the existing player.

The final expanded smoke run passed, including the real denied-Play request.
Mobile screenshots were visually reviewed without overflow. Temporary digital
acceptance artifacts: `collector-purchases-qAkSkH` (final rebuilt assets).

## Checks

TypeScript check, production build, rebuilt embedded Rust server, queue regression
and the full expanded browser smoke passed. Workspace tests and workspace
Clippy with warnings denied also passed, as did architecture/link checks, Rust
formatting, API consistency (unchanged contract) and whitespace review. No native client, live NAS archive or provider
was modified; broader hardware/LAN/fidelity qualification remains HL-04/HL-06/HL-08.
