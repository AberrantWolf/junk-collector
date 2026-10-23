# HL-08 AccurateRip contribution investigation — 2026-10-18

## Outcome

The bounded research assignment is complete. The
[source-linked investigation and operator workflow](../accuraterip-contribution-investigation.md)
recommend the physical disc and EAC or dBpoweramp on Windows, with AccurateRip
drive calibration and secure extraction before explicitly authorized submission
through that ripper. A Mac-only setup or missing physical disc/calibrated drive
is an operational blocker. Direct Collector submission additionally lacks an
established supported provider contract and typed physical-rip provenance.

This is a documented recommendation, not execution/acceptance of a contribution.
Submission support remains unimplemented. No external messages, submissions,
new disc lookups, audio scans, package changes or provider installations occurred.

## Evidence reviewed

- AccurateRip's public software/access pages and Illustrate contact route.
- Maintainer Spoon's 13 April 2025 identification of EAC/dBpoweramp as submitting
  clients, 12 July 2025 Mac limitation, and earlier calibration explanation.
- Current official dBpoweramp ripping/setup/advanced help and PerfectTUNES
  guidance about own-submission confidence; CTDB's separate contribution scope.
- Collector's provider adapter, music evidence types, Capture/source fingerprints,
  acquisition ownership and existing web fetch/check commands.
- The dated SQ Relax Vol. 2 lookup report, retaining its exact-TOC/404 limits.

The investigation links each external claim to its primary source, separates
documented configuration requirements from recommended local retention policy,
and distinguishes a database comparison offset from measured drive calibration.
The July 2025 Mac post was available as indexed publisher text; direct fetch
returned 403. Some guessed/obsolete documentation URLs were unavailable; the report
cites the working official help pages instead. No private submission specification
or runtime behavior was inferred from a missing page.

## Validation and limits

Passed:

- `python3 scripts/check_architecture.py` (architecture, documentation links and
  fixture manifests).
- `git diff --check`.

Self-review checked the sources against the recommendation, including the Windows
restriction, CTDB/metadata separation, calibration versus comparison offsets,
documented requirements versus local policy, and acceptance/self-match limits.

No production source or embedded assets changed, so Rust/TypeScript/browser suites
were not rerun for this prose-only investigation. No independent owned-disc,
native-client, Windows/drive, provider acceptance or deployment qualification is
claimed. Provider payload fields, contributor deduplication and publication
thresholds remain unestablished. An eventual implementation would need a separate
assignment after a supported provider contract and evidence policy are settled.
