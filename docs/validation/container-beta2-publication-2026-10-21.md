# Beta.2 correction and publication — 2026-10-21

**In progress.** Beta.2 corrects the [beta.1 NAS upgrade failure](container-beta1-publication-2026-10-21.md).
Publication and actual-host recovery are not yet accepted.

## Repair

- Recreate obsolete catalog schemas before rebuilding projections from the archive.
- Detect beta.1's bad schema-14 stamp by checking the music-search column layout.
- Retain the previous catalog for diagnosis. Leave private state and archive records intact.
- Rebuild from metadata without hashing media payloads.

The regression covers both alpha.3's schema 13 and beta.1's bad schema 14.
It preserves old checkpoint/trigger names so missing-object recovery cannot hide the bug.
The test fails on beta.1 code and passes with the repair.
Existing checkpoint recovery/reuse tests also pass.

## Source review

Reviewed the schema check, cache replacement, interrupted-rebuild path and private-state boundary.
Focused checkpoint tests, strict workspace Clippy, web check/build, container tooling,
architecture/docs/fixtures and formatting passed. Full workspace tests are running.
No web assets changed; beta.1's actual Chrome acceptance is reused.

## Pending acceptance

Full source gates, signed source/tag, native images, public pulls, pinned bundle,
DS1522+ alpha.3 upgrade/resume/cleanup/full restore and live-container comparison.
