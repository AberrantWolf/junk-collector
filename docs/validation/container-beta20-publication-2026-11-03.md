# Beta.20 release and application-host update — 2026-11-03 JST

> Deployment identifiers are anonymized; dates, source identities, outcomes and qualification limits are unchanged. Example addresses are not deployment instructions.


Beta.20 is published and installed on application-host. The user authorized qualification failure
repair, publication and installation after the required gates passed. This record
retains the unsuccessful attempts and the pre-existing application-host recovery incident; final
qualification, publication and live acceptance are recorded below.

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

The third source run [41601248769](https://github.com/AberrantWolf/junk-collector/actions/runs/41601248769)
passed workspace tests, strict Clippy and the initial browser journeys, then
failed the local-playback fixture's publication polling on a typed `catalog_busy`
503. Both native runtime gates and AMD64 release-image smoke had passed in
[41601266319](https://github.com/AberrantWolf/junk-collector/actions/runs/41601266319).
The fixture reader now retries only GET + 503 + `catalog_busy`, for at most five
seconds; mutation and other failures remain fatal. A one-shot injected contention
response exercises that path before the actual QUIC playback/seek/revocation
journey. A fresh exact-source run remains required.

The fourth source run [41604077547](https://github.com/AberrantWolf/junk-collector/actions/runs/41604077547)
passed the repaired local-playback and digital-matching journeys, then hit a
five-second Play-button assertion immediately after WAV-to-FLAC preparation.
The worker reported available, and the failure capture already showed the ready
FLAC. Track availability polls at up to five-second intervals, so the assertion
was racing the UI's refresh cadence. The bounded UI wait now allows two refresh
cycles plus request time (15 seconds), and the journey also requires actual
playback from the still-open editor before closing and testing reload playback.
Both native gates again passed; this incomplete attempt is not release evidence.

## Final exact-source qualification and publication

Signed source/tag `253e37e6c15d243fdadee6d928d48ebfe8b332aa` is published as
[beta.20](https://github.com/AberrantWolf/junk-collector/releases/tag/v0.1.0-beta.20).
[Checks 41607777949](https://github.com/AberrantWolf/junk-collector/actions/runs/41607777949)
passes full source coverage: **634 Rust tests, zero failures, five existing ignores**,
strict workspace/all-target Clippy, architecture/format, web types/build/models,
tooling/release/mail fixtures and all **30 browser journeys**. Workspace tests took
332.12 seconds; the browser suite took 537.13 seconds. The focused repaired local
playback, digital matching and full purchase journeys also passed locally.

[Validation 41607798070](https://github.com/AberrantWolf/junk-collector/actions/runs/41607798070)
passes both native builds/runtime gates, AMD64 release-image browser acceptance and
sealing. Initialize/configuration/group access, partial-upload recreation, SIGTERM,
explicit FLAC, backup/restore, inbox cleanup, media isolation and Caddy fixture TLS
all pass. The earlier successful source run at `f2677c6` is supporting evidence;
it did not replace final exact-source qualification.

[Publication 41610647347](https://github.com/AberrantWolf/junk-collector/actions/runs/41610647347)
reused validation artifact `13218013166`, attempt 1, without rebuilding. Tag signature,
release asset SHA-256 checks, anonymous pulls and independent verification of both
platform manifests/source/version labels pass. `preview` matches the new index;
stable `latest` remains absent/unchanged. Redundant tag-triggered Checks
`41610639902` was cancelled after the identical main source had full passing
coverage; it is not additional passing evidence.

```text
index:       sha256:36b95f5a7c731d9cff5d5fb1c1664819d520875c748d5e56ad4206d6c72572d5
linux/amd64: sha256:5edec03ab193d7d3fe8873f88e9418b659ae9f5c8433a81fe9ab4f4e5b143d7f
linux/arm64: sha256:5bbf513cdcc1016e29262fcff67102b519ea754cd04d796db3e834c56a63a928
```

application-host anonymously pulled the immutable index and verified AMD64/source/version before
downtime. Frozen source archive SHA-256 matched on Mac and application-host:
`59a7787376782ac9cd6e7b268407054b0cbfd486d005f41edfa59f3203dda54c`.
No local release executable or deployment image was rebuilt.

## application-host startup incident and lossless recovery

With zero unfinished Jobs, all five stack services stopped and actual PIDs reached
zero. application-host/private-state and whole NAS-parent read-only snapshots were sealed before
activation. The first beta.20 start refused a publication-artifact digest mismatch.
It correctly failed closed, before opening the catalog/private databases.

The **pre-update NAS snapshot already contained the same conflict**. A cancelled
fingerprinting Job (`24c9b0c1-40db-5c33-978c-cb712e47dffb`) had left a `bytes_staged`
publication targeting an observation revision subsequently committed by a different
transaction (`a5ec9371-2ccb-557f-b44f-2adad5437f2f`). Both staged and published records
matched their own manifests. Their only content difference was
`observation.created_at`; the cancelled staging did not own the committed target.
The conflicting published observation was valid and was not rewritten; raw
originals were untouched.

All services were stopped again, and full failed-state snapshots retained. After
checking the terminal cancelled Job, both manifests/digests and distinct transaction
ownership, the cancelled journal and entire staging directory were moved together
to the NAS parent's private recovery directory:

`/volume1/junk-collector/recovery-operations/abandoned-publications/24c9b0c1-40db-5c33-978c-cb712e47dffb`

Its receipt records ownership, the published digest, the timestamp difference and
snapshot references; the retained journal records both staged artifact digests. The published observation's digest remained
`40890ac3e219389a785f4ba662bf4b421a78a09798edae4857cdce6aa0aac4e5`.
No journal, staged evidence, published record, raw original or private Job was deleted.
This was an explicitly inspected legacy-state recovery, not automatic dismissal of
unknown or malformed journals. General automatic conflict reclassification is not
claimed. The integrity guard remains intact.

A fresh stopped-writer pair was sealed **after** the repair and before restart:

- application-host: the private application-state snapshot.
- NAS: the private archive-parent snapshot.

The original `before-beta20-20261103-253e37e` pair and both
`failed-beta20-20261103-253e37e` diagnostic snapshots remain retained. The original
pair contains the legacy restart blocker; use its documented recovery receipt
rather than treating it as a clean startup point. The repaired pair retains current
environment/authority-key ownership and references the complete saved stack,
systemd, mount and prior-image metadata. Recovery requires both matching members
with every writer stopped. These are same-device recovery points, not a claim of
new independent full-media backups.

## Live acceptance

Acceptance passed at **2026-11-03 18:34:24 JST**. The executable reports
`junk-collector 0.1.0-beta.20`; core/media use the qualified immutable image and core
is healthy. All five services are running with zero automatic restarts after the
manual recovery restart. Public and LAN HTTPS return 200 with trusted TLS and
unchanged server identity. Seven served assets match
frozen source, including Metadata/preparation and import bundles.

Counts remain **123 Items, one user, two grants and 5,199 Jobs**, with zero unfinished
work. Both credential-bearing profiles, connection revision 2/collection/cursor/
paused state, configuration/TLS and authority key are unchanged. Receipt suggestions
and accepted-purchase Items remain zero. Bridge reports fresh `synced` status and
the same certificate pin. UID/groups, mounts, isolation, commands, restart policies
and topology are preserved. Only core/media were recreated; Bridge/Caddy/tunnel
container identities and all seven unrelated services are retained.

No production matching retry, mailbox login, credential replacement or mutation
acceptance fixture was issued. Functional playback/import acceptance used isolated
fixtures. Installed Trash Amp remains unchanged. Hardware/phone/cellular acceptance
is outside this release's claims. Private operation evidence is retained on application-host in
`/home/operator/collector-beta20-253e37e` and locally in `/tmp/collector-beta20`.
