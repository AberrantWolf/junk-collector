# GitHub Metadata review repairs — 2026-11-03

Issues #8 and #10 were valid on source `2d18c6f`. `unmatched` includes the retained
automatic identity even when a manual selection supersedes it. It describes result
membership, not acceptance or provider failure. Presentation now derives the cue
from the effective display identity and presents other retained candidates neutrally.
No authority, candidate history or provider evidence is changed.

Both lookup methods share panel styling, spacing and primary actions. Their short
introductions identify MusicBrainz IDs/text and AcoustID fingerprints respectively.
Readiness/configuration failures and mapping conflicts remain visible.

Implementation review removed a second acceptance fallback: only the effective
identity can receive an accepted cue. The regression includes an old automatic
candidate alongside a different selected release. Review checks no warning/in-use
style is applied to that older result.

Selected checks: architecture/link checker, TypeScript, asset build, web models,
server/media debug build and smoke, fx04-review and digital-matching browser
journeys. All pass. Mobile screenshot inspected at 390 px; no horizontal overflow.
The digital journey exercises both search methods with real files and offline
provider responses. No production deployment or external provider calls.

The later broad suite exposed an integration assertion waiting for the old lookup
introductory sentence. The panel rendered correctly. That incidental copy assertion
was removed; the following keyboard/help/metadata controls remain the acceptance
checks. Broad browser revalidation follows the background-contention record.
The next integration assertion expected the removed warning class on a retained
selected candidate. It now checks the meaningful guarantee: the selected Japanese
release remains the sole effective candidate after a no-match search.
