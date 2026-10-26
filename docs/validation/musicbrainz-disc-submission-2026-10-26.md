# MusicBrainz track names and Disc ID submission — 2026-10-26

## MusicBrainz track mapping and Disc ID submission — 2026-10-26

The existing application disc resolver treats an unregistered local Disc ID as
missing provider evidence. An effective single-medium release can supply track
names when every recorded track position matches; multi-medium releases require
an unambiguous registered ID or an explicit source/release/medium assignment.
A positive ID match to another medium still conflicts with an assignment. Manual
track titles retain authority. This is descriptive metadata, not catalog or audio
verification.

Authorized album coverage derives a bounded MusicBrainz TOC argument from the
current source-fingerprint-bound recorded layout, using the same module mapping
and shared TOC validation as Disc ID calculation. It requires a complete audio
layout and agreement with its recorded ID. No media payloads are read or hashed.
The argument is absent without an effective release/layout or when the recorded
ID is already listed anywhere on that release. No persistent record or new job is
needed. The browser adapter builds a MusicBrainz attach link with this TOC and the
effective release ID. Users review the disc and confirm the edit on MusicBrainz;
Collector stores no MusicBrainz credentials and does not claim submission success.
Refreshing release metadata updates button eligibility from the current observation
of the same release, even when a manual choice retains older immutable text.
Existing browse/collection
access applies to coverage; the link itself grants no new Collector access.

## Validation

Focused resolver and TOC regressions pass, including missing/registered IDs,
multidisc ambiguity, positive conflicting assignments, mismatched tracks and an
inconsistent recorded TOC/ID. The store regression uses preserved CUE originals,
selects a release without a registered ID, applies track names and preserves a
manual title through reindex; unauthorized coverage is denied. A changed provider
version registers the ID without replacing the selected immutable metadata or
manual title; suppression survives reindex and remains available to Viewers.
The shared published identifier vector independently checks TOC offsets and
lead-out; skipped track numbers are rejected.

The Chrome journey uses the real local server, preserved CUE/BIN import and
MusicBrainz acquisition/publication workflow with an offline provider fixture.
It enters a release ID, explicitly selects the release, observes both track names,
checks the exact submission TOC/release arguments and follows the link into a new
tab. Only that external destination is intercepted; no live edit is submitted.
It checks mobile layout and refreshes changed provider data to suppress the link.
The original smoke metadata choice is restored before independent playback checks.

Passed: `python3 scripts/check_architecture.py`, `cargo fmt --all -- --check`,
`cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`,
`npm --prefix web run check`, `npm --prefix web run check:api`,
`npm --prefix web run build`, and rebuilding `collector-server` with served assets.
Focused store acceptance was rerun after adding refreshed-registration handling;
the independent published TOC/identifier vector also passes. Existing ignored
owned-corpus/scale cases remain unrun.

Final `npm --prefix web run test:smoke` passed in Google Chrome 156.0.8078.47,
including the new lookup/submission/refresh journey and existing real playback,
seek/queue/list/import/provider/permission/WebAuthn/mobile journeys. Chrome needed
execution outside the filesystem sandbox. The mobile submission screenshot was
visually inspected; the button and explanatory text fit the viewport.

No release, deployment or live MusicBrainz edit is included. Registration status
reflects recorded provider data; refresh the release after submitting an edit.


## Subsequent authorized publication and deployment — 2026-10-26 JST

The user then authorized release and deployment. Beta.8 is published and running
on the NAS after exact-source checks and both native container smokes. Retained
configuration/identity, a complete stopped-server recovery point, public HTTPS
and served-asset checks, and 18 unchanged unrelated containers are verified. See
[the publication/deployment record](container-beta8-publication-2026-10-26.md).
This supersedes the earlier source-only release/deployment limitation. No live
MusicBrainz edit or new physical-device listening acceptance is claimed.
