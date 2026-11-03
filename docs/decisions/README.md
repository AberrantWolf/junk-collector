# Architecture decision records

The numbered ADRs state accepted architectural rules using **must**, **shall** and
**must not**. They formalize the high-confidence choices authorized on 2026-11-03;
they do not invent an earlier acceptance date or establish that a capability has
been implemented, released or qualified on every platform.

## Accepted ADRs

| ADR | Decision |
| --- | --- |
| [0001](0001-application-policy-and-mechanism-boundaries.md) | Application policy and mechanism boundaries |
| [0002](0002-independent-collection-identities.md) | Independent collection and physical identities |
| [0003](0003-portable-and-private-authority.md) | Portable collection authority and private operational state |
| [0004](0004-single-writer-publication-and-recovery.md) | Single-writer publication and recovery |
| [0005](0005-lossless-module-data-and-write-eligibility.md) | Lossless module data and write eligibility |
| [0006](0006-source-bound-evidence-and-coverage.md) | Source-bound evidence and coverage |
| [0007](0007-manual-metadata-authority.md) | Manual metadata authority over provider observations |
| [0008](0008-meaningful-provenance-and-bounded-lookup-history.md) | Meaningful provenance and bounded lookup history |
| [0009](0009-exposure-composition-and-independent-authorization.md) | Exposure composition and independent authorization |
| [0010](0010-trust-before-credentials-and-native-enrollment.md) | Trust before credentials and native enrollment |
| [0011](0011-input-bound-command-receipts.md) | Input-bound command receipts and effects |
| [0012](0012-durable-workflow-ownership-and-fencing.md) | Durable workflow ownership and fencing |
| [0013](0013-bounded-projections-and-client-refresh-state.md) | Bounded projections and client-owned refresh state |
| [0014](0014-playback-delivery-separate-from-integrity.md) | Playback delivery separate from integrity verification |
| [0015](0015-isolated-direct-media-delivery.md) | Isolated direct media delivery |
| [0016](0016-coordinated-pre-release-evolution.md) | Coordinated pre-release evolution with data preservation |
| [0017](0017-publish-qualified-artifact-bytes.md) | Publish the artifact bytes that passed qualification |

The existing [ordinary music-CD capture tradeoff](cd-audio-capture-tradeoff.md) is
also accepted. Its specialized Q-subchannel policy and original acceptance date
remain in place; it is not a general relaxation of structural or audio checks.

## Requirements, evidence and authority

ADRs own cross-cutting architectural commitments. Subsystem contracts own detailed
rules, supported formats and operating procedures; they remain applicable and are
not replaced merely because an ADR summarizes their boundary. Existing dated
validation and release records retain their sources, outcomes and limitations.
[Implementation status](../implementation-status.md) owns availability; the
[open-work queue](../remaining-work.md) remains the only assignment queue.

Supporting code and test links establish why a rule is treated as intentional.
They are not new test results. Alternatives described as excluded by a rule are
not evidence that a historical alternatives evaluation occurred. Retain documented
rationale and identify missing rationale rather than reconstructing it as fact.

The older foundation/security/scope/evolution documents remain linked detail and
rationale. If a conflict is found, reconcile the owning decision and contract
explicitly; do not silently discard a valid preservation, security or recovery rule.

## Proposals and future changes

[Lower-confidence proposals](proposals.md) are discussion material, not accepted
rules or new implementation assignments. Tunable values and a tested current
mechanism do not automatically become permanent architectural commitments.

An ADR change must explain context, normative decision, consequences, scope,
supporting requirements/evidence and reconsideration triggers. Materially replacing
a decision must explicitly identify the affected ADRs and update their references.
Git owns superseded prose; do not create dated snapshot copies. Accepted rules must
remain discoverable in live documentation until explicitly revised or superseded.

See [documentation maintenance](../index.md) for generalization, evidence preservation
and LLM ingestion. Retrieved ADR chunks must retain the ADR identity/status and
whether they contain a decision, rationale, proposal or implementation observation.
