# 0006: Source-bound evidence and coverage

Status: proposed, not yet reviewed by the owner.

## Context

Integrity hashes, structural checks, descriptive identity and reference matching
support different claims. Using one as if it were another hides uncertainty.

## Decision

- Integrity hashing, structural inspection, provisional metadata, MusicBrainz
  identity, game catalog matches and AccurateRip are distinct kinds of result.
- Evidence records the actual source and reference inputs, algorithm and version,
  settings, coordinates and coverage. Reference bytes are checked against their
  recorded length and digest before new results are attributed to them.
- Partial, ambiguous or missing coverage stays explicit. One matching track is not
  a whole-disc result; bounded candidate discovery is not exhaustive verification.
- Malformed input fails closed: no invented lengths, padding or skipped entries.
  The file can still be preserved with the analysis failure reported.
- A cached result is never presented as a fresh check.

See: [processing records](../../crates/collector-core/src/processing.rs),
[game identification](../game-identification.md),
[audio support](../audio-preparation.md).
