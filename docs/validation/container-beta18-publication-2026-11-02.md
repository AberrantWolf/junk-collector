# Beta.18 qualification and application-host update — 2026-11-02 JST

> Deployment identifiers are anonymized; dates, source identities, outcomes and qualification limits are unchanged. Example addresses are not deployment instructions.

The user authorized GitHub failure repair, a clean beta release, application-host update and
subsequent cleanup of obsolete application-host experiments/versions. Beta.17 is the release
baseline; the entire candidate selects workspace tests/Clippy, tooling/contracts,
web models and all 30 browser journeys plus both native release-image gates.

GitHub run 41417303165 failed because the broad playback test synchronously
counted track rows while the new lazy track request was still pending. The test
now waits for exactly one disc and two logical tracks before checking their
representation IDs and exercising actual playback, seek and revocation.

The separately recorded grouped-intake race is repaired by establishing all
reviewed physical copies before freezing the first child revision. Store selection
returns bounded same-destination manifest pages, including unadmitted members;
application code owns physical/digital policy and deterministic copy identity.
Manual edits retain the existing strict publication fence. Deterministic cases
exercise distinct copies, staggered staging, failed retry and reindex.

## Exact-source qualification and publication

Signed source/tag `4fecd6ff747f4b9af5a71894e577d3a1000481db` is published as
[beta.18](https://github.com/AberrantWolf/junk-collector/releases/tag/v0.1.0-beta.18).
Main [Checks 41437672072](https://github.com/AberrantWolf/junk-collector/actions/runs/41437672072)
passes all selected gates: **609 Rust tests, zero failures, five existing ignores**,
strict workspace/all-target Clippy, architecture/format, web types/build/models,
tooling/release policy/mail fixtures and **all 30 browser journeys**. The playback
and grouped-intake journeys pass together. Local focused grouped tests, paged
manifest regression, strict Clippy and actual Chrome integration also pass.

[Validation 41437671234](https://github.com/AberrantWolf/junk-collector/actions/runs/41437671234)
passes both native image builds/runtime checks and AMD64 release-image browser
acceptance. Initialize/configuration/group access, partial-upload recreation,
SIGTERM, FLAC, backup/restore, inbox cleanup, media isolation and Caddy fixture TLS
pass. Optional metadata is unavailable in the disposable image fixture; originals
and explicit FLAC remain usable. No standalone release executable was rebuilt.

[Publication 41440258145](https://github.com/AberrantWolf/junk-collector/actions/runs/41440258145)
promotes the sealed exports without rebuilding. Signatures and release asset
SHA-256 checks pass. Anonymous verification confirms both platform manifests and
source/version labels. `preview` matches the new index; stable `latest` is unchanged.
The redundant tag-triggered Checks run was cancelled because the successful main
run already qualifies the identical source; it is not additional passing evidence.

```text
index:       sha256:1b033a28a303bb4e200215ecaf4e2f0bd62c94aa5745c0dcebb287d1bd4bb58f
linux/amd64: sha256:85eef4360c8869205d89559917e0c0cc8fdb3e2b81143492e15a14c29b353ed3
linux/arm64: sha256:0f4ee46e0500f202d8bbf746842ce591036b5822df44858c4896f7d11d849e75
```

## application-host recovery point and live acceptance

application-host anonymously pulled the immutable index before downtime and checked its exact
source/version/architecture. The source archive SHA-256 matches on Mac and application-host:
`51a852b652cbb799d140b65013b54c2c943609b89c3783890384cb0cd9691fed`.
No local image build occurred. With no unfinished Jobs, all five stack services
stopped and all actual PIDs reached zero. Full read-only snapshots and database
quick checks succeeded before sealing the matching pair:

- application-host: `/var/backups/collector-recovery-pairs/before-beta18-20261102-4fecd6f/application-data`.
- NAS: `/volume1/collector-recovery/before-beta18-20261102-4fecd6f`.
- NAS snapshot UUID: `f16268e0-71ce-c04c-bc6c-d1060eb61d43`.

application-host covers all local private state/WAL, imports/scratch, configuration/TLS, Bridge
home/cache and profiles; NAS covers the entire archive parent subvolume. Neither
has excluded nested subvolumes. Private recovery metadata retains old stack/env/
systemd/fstab, image identities and the authority service key with numeric ownership.
Rollback requires both matching members with all writers stopped.

Acceptance completed at **2026-11-02 20:55:20 JST**. The executable reports
`junk-collector 0.1.0-beta.18`; core/media use the qualified immutable image and all
five stack containers have zero restarts. Core is healthy. Public and LAN HTTPS
return 200 with trusted TLS and unchanged ServerId
`01a15429-856a-7763-b558-3c158b1604a0`. Seven served assets match frozen source:
`app.js`, `app.css`, `curation.js`, `library-imports.js`, `purchase-inbox.js`,
`library-metadata.js` and `library-preparation.js`.

Counts remain **123 Items, one user, two grants and 3,507 Jobs**, with zero
unfinished work. Config/TLS, authority key, mounts, UID/groups, isolation, restart
policy, commands and topology are retained. Only core/media were recreated.
Bridge/Caddy/tunnel identities and all seven unrelated running services are retained.
Both credential-bearing profiles are identical; the connection retains revision 2,
its collection/cursor and paused state. Receipt suggestions/accepted-purchase Items
remain zero. Bridge reports fresh `synced` status with the same certificate pin.
No mailbox login, credential replacement, rescan or production retry was issued.
Functional mutation/playback acceptance used disposable fixtures, not production.
Installed Trash Amp and its retained development worktree are unchanged.

## Cleanup after acceptance

Removed the three stopped `junk-collector-qualification` containers, their two
named volumes and the unused local fixture tree. Removed obsolete committed-source
checkouts, extracted source directories and source tar bundles from prior beta/mail
experiments. One Collector image unreferenced by containers or any retained recovery
metadata was removed. All retained recovery snapshots, required old images, private
operation evidence, production data and unrelated services are preserved.

Docker reports **7.154 GB + 10.58 GB** reclaimed from unused build cache (about
17.7 GB). No general volume prune or collection-history deletion was performed.
Post-cleanup container identities/unrelated service start times and public/LAN
HTTPS/assets pass again. Current deployment configuration remains under its existing
`/opt/junk-collector-beta14` path because the live service still uses those files.
Private operation evidence remains in `/home/operator/collector-beta18-4fecd6f` on application-host
and `/tmp/collector-beta18` locally. Hardware/phone/cellular acceptance remains
outside this release's claims.
