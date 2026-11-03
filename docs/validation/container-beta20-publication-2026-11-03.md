# Beta.20 release and application-host update — 2026-11-03 JST

> Deployment identifiers are anonymized; dates, source identities, outcomes and qualification limits are unchanged. Example addresses are not deployment instructions.

The user authorized a new release, monitoring/repair of qualification failures,
and application-host installation after every required gate passes. This is the current operation
record; candidate preparation does not assert publication or installation success.

## Candidate scope

The candidate advances the workspace to `0.1.0-beta.20`. It includes incremental
digital matching and independent review choices (`2d18c6f`), Metadata candidate and
panel repairs (`655ffb8`), background catalog-contention recovery (`2b15b29`), and
the 75-lead error-boundary review (`cf0d3bb`). The preceding source delivery records
retain local validation and limitations. Raw originals, accepted sources, manual
authority and unknown module records are preserved.

Qualification uses beta.19 as the validated baseline with conservative `infra`
coverage, both native image/runtime gates and AMD64 release-image browser acceptance.
The existing Container release workflow owns builds, sealed exports, publication,
anonymous pull checks and preview promotion. Publication must reuse those exact
validated bytes. Stable `latest` must remain unchanged. No extra standalone release
build is required for this container-only delivery.

application-host installation will first inspect the live installation and pending work, then
retain a matched stopped-writer application-host/private-state and NAS/archive recovery pair.
Only the qualified core/media image may be updated. Existing identity, configuration,
credentials, Bridge/Caddy/tunnel topology, unrelated services and meaningful history
remain protected. Actual-host acceptance and recovery paths will be recorded after
execution; no production work is cancelled merely to make an update convenient.

## First qualification failure and repair

Candidate `f2677c6` validation run [41599166464](https://github.com/AberrantWolf/junk-collector/actions/runs/41599166464)
built both native images, then failed both runtime gates at the same obsolete
optional-metadata assertion. With providers unconfigured, the repaired application
reports a failed workflow coordination step rather than attributing an unstarted
Job to metadata. The smoke gate now accepts that truthful projection only with a
visible diagnostic, failed overall outcome, no metadata Job and no remaining
background work. Configured providers must still acquire and settle a metadata
Job. Originals, explicit FLAC preparation and range playback remain required.
This failed attempt is not publication or deployment evidence; a new exact-source
qualification is required.

The second attempt, [41600690208](https://github.com/AberrantWolf/junk-collector/actions/runs/41600690208)
at `87eb68b`, passed the repaired metadata gate on both architectures, then found
a second obsolete cleanup error-text assertion. The real read-only mount refused
cleanup with a `ReadOnlyFilesystem` diagnostic and zero removed bytes. The drill
now requires a diagnostic without freezing its wording; source-byte preservation,
archive downloads and subsequent read/write cleanup remain required.
Earlier source Checks [41598695106](https://github.com/AberrantWolf/junk-collector/actions/runs/41598695106)
also found a browser-test polling race: a temporary typed catalog-busy response
was dereferenced as metadata. Saved-artwork polling now uses the existing
status-aware metadata reader, which retries only that bounded contention and
rejects other HTTP errors. Saved cover selection/download checks remain intact.
