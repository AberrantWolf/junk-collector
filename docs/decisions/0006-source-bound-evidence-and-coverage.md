# ADR 0006: Source-bound evidence and coverage

Status: Accepted. Formalized 2026-11-03 from existing documented decisions and
implemented constraints; this is not a claim that implementation or qualification
first occurred on that date. Source reviewed: `eaff933`.

## Context

Integrity, structure, descriptive identity and reference matching establish different claims. Reusing one result as a broader claim obscures uncertainty.

## Decision

- Integrity hashing, structural inspection, provisional metadata, MusicBrainz identity, game catalog matching and AccurateRip must retain distinct meanings.
- Evidence must bind actual source/reference inputs, algorithm/version, settings, computation coordinates and coverage. Consumed reference bytes must be validated against their recorded length and digest before new results are attributed to them.
- Partial, ambiguous or missing coverage must remain explicit. One matching file or track must not establish a complete package/disc result; bounded candidate discovery must not be described as exhaustive verification.
- Processing malformed structural input or inconsistent result envelopes must fail closed rather than silently inventing lengths, padding missing content or skipping required entries. Separately authorized preservation with an honestly reported analysis failure must remain available.
- Cached reuse must not be represented as a newly performed integrity or reference check.

## Consequences and scope

This makes claims auditable and limits automatic conclusions. It does not freeze supported algorithms or require retaining every equivalent provider response indefinitely.

## Supporting requirements and evidence

- [Source fingerprint and processing records](../../crates/collector-core/src/processing.rs)
- [Frozen-input/source-bound contract tests](../../crates/collector-app/tests/processing_contracts.rs)
- [Supported game claim boundaries](../game-identification.md)
- [Audio claim boundaries](../audio-preparation.md)

Implementation and tests are supporting evidence, not a fresh qualification run.
Detailed subsystem requirements and existing dated validation limits remain applicable.

## Reconsideration

Revisit when adding a claim type or algorithm; define its input domain, independent evidence and coverage before publication.
