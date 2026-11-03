# ADR 0007: Manual metadata authority over provider observations

Status: Accepted. Formalized 2026-11-03 from existing documented decisions and
implemented constraints; this is not a claim that implementation or qualification
first occurred on that date. Source reviewed: `eaff933`.

## Context

Provider observations and automatic ranking change over time. Deliberate collection curation must survive refresh, retries and projection rebuilds.

## Decision

- Explicit candidate selection, field overrides and artwork choice must remain separate from provider observations and automatic selection.
- Provider refresh and automatic adoption must not overwrite manual metadata, digital-source or artwork authority.
- Resolution and publication must bind to the inventory/source and authority revisions they resolve. Stale computation must not silently replace later manual edits.
- Stable candidate/observation identities must not silently acquire different content. Current automatic candidates must remain distinct from selectable retained history.
- Source-only and artwork-only commands must preserve unrelated authority. Clearing or resetting authority must be an explicit validated action.

## Consequences and scope

Automatic enrichment remains useful without taking ownership of curator choices. Ranking rules and UI presentation are maintained in subsystem contracts, not frozen by this ADR.

## Supporting requirements and evidence

- [Authority model](../../crates/collector-core/src/metadata.rs)
- [Resolution precedence tests](../../crates/collector-app/tests/metadata.rs)
- [Authority update policy](../../crates/collector-app/src/metadata.rs)
- [Digital matching and review evidence](../validation/digital-matching-and-review-2026-11-03.md)

Implementation and tests are supporting evidence, not a fresh qualification run.
Detailed subsystem requirements and existing dated validation limits remain applicable.

## Reconsideration

Revisit for new authority types or explicit reset workflows, testing refresh, stale inputs and reindex conflicts.
