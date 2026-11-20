# Product requirements

What Junk Collector is for and the requirements the owner has set. For what is
implemented now, see [implementation status](implementation-status.md); for open
work, see [remaining work](remaining-work.md).

## Scope

A clean, useful self-hosted tool for a home lab: preserving and cataloguing
physical video games and music CDs, plus digital music purchases. It keeps
original data, verification results, descriptive metadata and physical
inventory — including inventory without files and captures not yet identified.
Price tracking is secondary. Security should be proportional to a home server
that may be reachable from the Internet.

The server targets Linux (native binary or container) and embeds the web UI for
listening, curation and administration. Trash Amp is a separate lightweight music
player that connects to Collector.

Out of scope for now: replication between servers, cross-collection
deduplication, runtime plugins, video, emulators, ripping hardware, DRM'd
purchases, synchronized listening, PriceCharting exchange and an iOS app.

## Preservation

- Keep raw originals byte-for-byte: ISOs, cartridge images, redumper output,
  BIN/CUE packages, FLAC/MP3/WAV purchases. Never rewrite them.
- Keep physical collection facts, manual metadata choices, accepted
  metadata/artwork sources, meaningful evidence and unknown module data. Lookup
  history that adds nothing does not need to be kept.
- Simplifying or compacting stored data needs an explicit, tested conversion —
  never silent deletion.
- Recovery after an interrupted import or a rebuilt catalog must not lose
  originals, manual overrides, loans or evidence.
- Practical local recovery is required: a stopped-server full backup/restore
  (including media) and a portable full export/import. Automatic, cloud or
  off-site backups are not required; a backup-age reminder is optional.

## Import

- Import from configured inbox folders or resumable browser uploads. Validate
  paths, stage, hash, propose identity, let the user review, verify the
  destination bytes, publish, then run checks and enrichment in the background.
- Importing a raw dump creates the entry with one owned physical copy by
  default. Manual entries can exist without an owned copy, or be wishlist-only.
- Source files are kept by default; removing them from the inbox is an explicit
  action.
- Missing optional tools or providers must not prevent basic preservation.
- Import history lists each import and attempt with actionable outcomes. Users can
  reopen older imports, discard unpublished staging (showing what space was
  reclaimed), dismiss/restore finished entries, and fix a wrong association
  without losing originals or evidence.
- Items can be removed and restored, with clear rules about what is kept. An
  existing item can be cleanly reimported without duplicate entries. Users can
  clean up failed, cancelled and unfinished work.

## Identification and metadata

- Games: upload standard DAT files or ZIPs, review and activate them; existing
  and future imports are identified automatically. Identity never comes from
  file names, suffixes or sizes, and malformed input never falls back to a guess.
  Headerless images need a complete unambiguous match; CRC-only matches are
  weaker evidence.
- Music: show proper track names when a matching MusicBrainz release has them,
  with an obvious lookup action for older albums. Gathering names never requires
  reimporting.
- Pick a useful tentative match when ambiguous, keep the alternatives, mark
  uncertainty visibly, and keep manual corrections across refreshes and provider
  outages. Identity strength outranks language; the account's metadata language
  (English by default) picks between variants, and original text is kept.
- Multi-disc albums show a per-disc checklist: which originals are attached,
  recorded versus release-listed disc IDs, missing discs, uncertain assignments,
  and how to complete structure and audio checks. Detailed processing history is
  available but doesn't dominate the album view.
- Integrity, structure, DAT matches, AccurateRip and conversion checks are
  separate kinds of evidence. MusicBrainz identity never proves audio
  correctness.
- Scans: equivalent scans can't run twice at once; completed results are the
  primary action, and repeat results are consolidated with recheck dates/counts.
  A cached result is not a fresh verification.

## Browsing and playback

- Browsing and search show cover art beside each game or album row, including
  offline views, with platform/edition/artist details, accessible labels and
  consistent placeholders. Missing artwork and uncertain identity look different.
- Library data loads in layers — connection and login, album count, title pages,
  artwork, then tracks on demand — each request fast on a healthy install.
  Background work shows as pending without blocking already-published data.
- Playback streams the requested bytes without first verifying the whole file.
  A failed read belongs to that stream; the client decides what to do (Trash Amp
  skips to the next track).
- Preparing audio is explicit: a whole album (including selected associated
  discs) can be prepared as separate FLAC tracks; Opus preparation on import is
  opt-in. Other work keeps running while preparation runs.
- Lists belong to one collection. Music lists hold individual tracks; adding an
  album or disc appends its tracks in order. Repeats and custom order are kept;
  lists can be played or shuffled. Unavailable members and concurrent edits are
  shown, never silently dropped.
- Trash Amp supports online and offline listening; offline copies can't be
  recalled once downloaded.

## Interface

Every screen offers useful defaults, only valid choices, prerequisite guidance up
front, and clear outcomes and recovery steps. This applies to new screens too.

## Networking and security

- Client, curation and administration are separate pages and APIs. Public client
  access is on; public curation and administration are off by default. Roles
  always apply.
- Public/private is decided by listener, never by source IP or forwarded headers.
  A public tunnel only reaches the public listener.
- Native clients trust the server's own certificate after authenticated pairing;
  there is no general invalid-certificate bypass. Browser trust needs a properly
  trusted hostname (local DNS, local CA or Tailscale HTTPS).
- Argon2id passwords, standard WebAuthn passkeys, revocable device credentials,
  recovery codes and local administrator recovery. Single-user mode hides account
  selection but is not anonymous access.

## Distribution

Docker images for amd64 and arm64 are built automatically and can be pulled
anonymously, so users install and update without compiling. `latest` never points
at a prerelease. Updating a Portainer stack keeps existing configuration,
accounts, originals and pending work.
