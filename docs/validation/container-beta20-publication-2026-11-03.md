# Beta.20 release and Fi update — 2026-11-03 JST

The user authorized a new release, monitoring/repair of qualification failures,
and Fi installation after every required gate passes. This is the current operation
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

Fi installation will first inspect the live installation and pending work, then
retain a matched stopped-writer Fi/private-state and NAS/archive recovery pair.
Only the qualified core/media image may be updated. Existing identity, configuration,
credentials, Bridge/Caddy/tunnel topology, unrelated services and meaningful history
remain protected. Actual-host acceptance and recovery paths will be recorded after
execution; no production work is cancelled merely to make an update convenient.
