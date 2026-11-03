# Documentation authority and maintenance

## Read by purpose

| Purpose | Maintained entry point |
| --- | --- |
| Installation, import and listening | [Getting started](getting-started.md), [container guide](../deploy/container/README.md) |
| Settings, access and recovery | [Configuration](configuration.md), [operations](server-operations.md) |
| Current boundaries and detailed contracts | [Architecture](architecture.md) and its subsystem references |
| Implemented capabilities and limits | [Implementation status](implementation-status.md) |
| Open assignments and conditional work | [Remaining work](remaining-work.md), the only execution queue |
| Active product/security decisions | [Scope](decisions/home-lab-scope.md), [security](decisions/security.md), [evolution](decisions/pre-release-evolution.md) |
| Validation and release qualification | [Evidence index](validation.md), [testing](testing.md) |
| Review criteria for implementation | [Development guidance](development-review-guidance.md) |

## Authority and history

Current guides describe supported behavior for readers with different installations.
A source implementation, a published artifact and acceptance on a particular platform
are separate facts. State which is established. Never infer publication from a version
bump or installation from a successful build.

Existing dated validation, release notes, investigations and reviews remain evidence
for their stated sources and scope. Older plans are references, not assignments; an
old missing-feature claim cannot override current status. Retain valid constraints in
their owning current contract. Remove obsolete proposals/instructions and use pinned
Git citations when historical context is useful. Do not create snapshot copies of old
documents: version control owns superseded prose.

User guides use generic host roles, placeholder paths and reserved example domains.
Never include a particular operator's machine names, SSH aliases, local account names
or deployment endpoints. Existing evidence may anonymize those identifiers while
retaining dates, source/artifact identity, hardware/software qualifications, failures
and limits. Label anonymized identifiers as illustrative; do not imply a rerun.

## Updating a subsystem

1. Read its current contract and relevant source/tests. Record the review source for
   material behavioral corrections, rather than refreshing dates without verification.
2. Update that contract when behavior changes. Update the capability matrix only for
   availability/limits, and the queue only for unresolved work. Link the same result
   instead of copying delivery paragraphs into every file.
3. Record actual validation in one dated result or the existing operation record.
   Preserve failed attempts, omitted checks and exact artifact provenance. Remove
   completed work from the queue without deleting unrelated conditional requirements.
4. Review links, anchors, commands, configuration defaults, supported-format matrices
   and public release labels. Mechanical facts should point to their source of truth;
   avoid repeating dependency pins or database versions in unrelated guides.
5. Review the whole diff with [change-based validation](testing.md). The architecture
   checker covers README, AGENTS and docs file links; also inspect deployment/API/fixture
   links and fragment anchors. A successful link check is not proof of semantic currency.

Choose one subsystem per periodic semantic review. Check its docs against the affected
code and open work; do not requalify every historical environment or copy an old test
count into a new current-status summary.

## LLM ingestion

Use these entry points and their maintained subsystem contracts as the default corpus.
Treat `docs/validation/`, `docs/releases/`, reviews and older plans/research as evidence,
retrieved only when the task needs them. Existing filenames are not sufficient to infer
currency: a delivered plan may contain a valid contract alongside obsolete chronology.

Attach document purpose, authority/status and source/date to every retrieved chunk.
A warning at the start of a long document may not be retrieved with a later section.
Distinguish accepted policy, current implementation, conditional design and historical
observation. Prefer the current owning contract over old planning claims; retain source
citations when using historical evidence. Keep one maintained corpus rather than a
second LLM-only prose copy. No automated ingestion pipeline is implemented by this guide.
