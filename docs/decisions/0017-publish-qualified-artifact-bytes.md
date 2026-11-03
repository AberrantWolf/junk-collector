# ADR 0017: Publish the artifact bytes that passed qualification

Status: Accepted. Formalized 2026-11-03 from existing documented decisions and
implemented constraints; this is not a claim that implementation or qualification
first occurred on that date. Source reviewed: `eaff933`.

## Context

Rebuilding at publication would separate deployed bytes from the artifacts that passed validation. Source identity alone does not establish artifact identity.

## Decision

- Each deployment target must be built in release mode, tested as its actual deliverable and published from those same qualified artifact bytes. Embedded assets must match the serving executable.
- Validation evidence must bind exact source, required check scope/outcomes and artifact identities. Publication must verify provenance and digests before promotion.
- Missing, expired, ambiguous, corrupt or failed validation must stop publication; it must not trigger an implicit unqualified rebuild. Explicit fresh validation may produce replacement qualified artifacts.
- Rehearsal, source preparation and local tests must not be reported as registry publication or deployment acceptance. Prerelease promotion must not advance the stable channel.
- Applicable unchanged evidence may be reused only under the maintained validation policy. Cache hits must not count as qualification evidence, and container-only delivery must not require discarded standalone builds.

## Consequences and scope

This separates qualification from promotion and supports reproducible rollback. CI vendor, artifact retention periods and cache keys remain mechanisms rather than permanent architecture.

## Supporting requirements and evidence

- [Publication/provenance contract](../container-release-pipeline.md)
- [Artifact identity checks](../../scripts/container_release.py)
- [Validation consumer](../../scripts/container_validation.py)
- [Artifact/validation regression checks](../../scripts/check_container_validation.py)

Implementation and tests are supporting evidence, not a fresh qualification run.
Detailed subsystem requirements and existing dated validation limits remain applicable.

## Reconsideration

Revisit for a new artifact type or publication platform, preserving the chain from validated source and bytes to promoted deliverable.
