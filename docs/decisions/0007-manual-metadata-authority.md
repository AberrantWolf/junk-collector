# 0007: Manual metadata authority

Status: proposed, not yet reviewed by the owner.

## Context

Provider data and automatic ranking change over time. A curator's deliberate
choices must survive refreshes, retries and rebuilds.

## Decision

- Explicit candidate selection, field overrides and artwork choice are stored
  separately from provider observations and automatic selection.
- Provider refresh and automatic adoption never overwrite manual metadata, digital
  source or artwork choices.
- Resolution is bound to the source and authority revisions it read, so a stale
  computation can't replace a later manual edit.
- Candidate and observation IDs never silently change content. Current automatic
  candidates stay distinct from retained history.
- Source-only and artwork-only commands leave other authority alone. Clearing an
  override is an explicit action.

See: [authority model](../../crates/collector-core/src/metadata.rs),
[update policy](../../crates/collector-app/src/metadata.rs).
