# ADR 0014: Playback delivery separate from integrity verification

Status: Accepted. Formalized 2026-11-03 from existing documented decisions and
implemented constraints; this is not a claim that implementation or qualification
first occurred on that date. Source reviewed: `eaff933`.

## Context

Whole-file verification before each playback read or poll makes interactive operations proportional to media size. A recorded digest is not a new integrity scan.

## Decision

- Playback must consume requested byte ranges without a whole-file integrity-verification preflight. HEAD and ordinary availability reads must not hash entire media payloads.
- Delivery must retain range, identity, authorization and read/length/EOF checks. Recorded digests must not be represented as fresh byte validation.
- Import/preparation source validation, explicit integrity checks and completed-download validation must retain their owning verification policies.
- Clients must expose media failures and apply their recovery policy; a failed stream must not be reported as a successful integrity or catalog check.
- This separation must apply consistently to core HTTP, private media, direct media and supported native offline playback.

## Consequences and scope

This deliberately accepts that playback need not detect same-length corruption before consumption. Verification remains explicit; actual stream failures stay visible. Transport buffer sizes and decoder choices are outside this decision.

## Supporting requirements and evidence

- [Playback read contract](../layered-library-refresh.md)
- [Core HTTP media response](../../crates/collector-server/src/lib.rs)
- [Counted-reader range regression](../../crates/collector-server/src/local_playback.rs)
- [Isolated media reads](../../crates/collector-media/src/lib.rs)

Implementation and tests are supporting evidence, not a fresh qualification run.
Detailed subsystem requirements and existing dated validation limits remain applicable.

## Reconsideration

Revisit for an explicit end-to-end verified-playback capability, including its latency, cancellation, caching and user-facing claim semantics.
