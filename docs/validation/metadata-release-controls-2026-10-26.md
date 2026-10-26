# Metadata release controls — 2026-10-26

User-requested local presentation follow-up to beta.8. No push, publication or
NAS update is included. Concurrent metadata-attention text/CSS changes are retained.

The resolved display candidate, rather than proposed/best-fit labels, determines
the active cue and first visible match. Explicit and automatic selections have
clear labels, a stronger outline and a “Currently using” banner. Manual field
sources receive independent badges in the effective metadata summary; the provider
release name remains visible when a manual display title differs. Manual-only
metadata has a distinct summary and no active release card.

Disc ID submission now appears alongside the active MusicBrainz release. The UI
uses the existing authorized coverage endpoint and server-produced TOC eligibility;
source/item/release/revision changes fence old results. It suppresses mismatched
release responses and registered IDs, deduplicates identical TOCs, and labels
multiple distinct eligible discs. Transient errors are visible and retryable.
Opening the link still requires user confirmation on MusicBrainz. The old hidden
disc-details action is removed.

## Validation

Passed architecture/document links, format, full Rust workspace tests, all-target
Clippy with warnings denied, TypeScript, generated API checks, web build and
rebuilding the Rust server with served assets.

Final focused Chrome acceptance passed explicit, automatic and manual-only states,
manual field badges, preserved drafts, first-position active automatic matching
even after provider ordering changes, and mobile overflow. It no longer relies on
the old small “Automatic best fit” label to establish which release is in use.

Full Chrome smoke passed the actual release-ID lookup/selection workflow,
submission from the Metadata candidate, correct recorded TOC/release arguments,
external-link navigation, mismatched-release suppression, registered-ID hiding,
removal of the old disc-details action, and existing playback/import/list/role
journeys. The other context occupied the default fixture port, so this ran from a
temporary copy of the same script with only ports 19841–19845 remapped to
19731–19735. The copy was removed after completion; no other process was stopped.
Chrome required execution outside the filesystem sandbox.

The mobile active-release, manual-only summary and submission screenshots were
visually inspected. Concurrent attention/copy edits remain intact. There was no
commit, push, publication, deployment or live MusicBrainz submission for this work.
