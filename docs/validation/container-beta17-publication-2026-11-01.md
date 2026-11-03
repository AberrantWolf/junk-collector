# Beta.17 publication and application-host update — 2026-11-01 JST

> Deployment identifiers are anonymized; dates, source identities, outcomes and qualification limits are unchanged. Example addresses are not deployment instructions.

[Beta.17](https://github.com/AberrantWolf/junk-collector/releases/tag/v0.1.0-beta.17)
is published on `preview` and installed on application-host. It includes the three purchased-
audio compatibility fixes, preserved-identification retry and the previously
local inbox selector/review popup. Originals remain byte-identical. See
[release notes](../releases/v0.1.0-beta.17.md),
[audio behavior/review](audio-compatibility-2026-11-01.md) and
[selector acceptance](import-source-selectors-2026-11-01.md).

## Clean source review and exact-source qualification

Review covers PCM extension/boundary checks, APE lengths/flags/header agreement,
metadata precedence, MPEG payload/seek bounds, cancellation and the narrow
upstream decoder backport. Recovery tests were strengthened for a private
companion before the audio file and manual album authority through retry/reindex.
All three strengthened tests pass, including real native FLAC preparation. The
second complete review found no remaining actionable gaps within the supported
scope. The 12 inconsistent-length WAVs and MP3 gapless timing limits remain
explicit; they are not successful fallback cases. No inbox files became fixtures.

Shared junk-libs `438f195449622da2a2b9082c36cb06f66098813c` is merged/pushed to main
and its review branch. Collector source/tag
`fe7b95c966ff75e8bb78e529fe94617e057ded69` is signed and pushed to main and
`v0.1.0-beta.17`; both signatures verify. No candidate change followed qualification.
This later documentation commit records completed operations without moving the tag.

- [Checks 41304322360](https://github.com/AberrantWolf/junk-collector/actions/runs/41304322360),
  attempt 1: beta level, `infra` classification, full coverage, unchanged committed
  source. Architecture/docs/fixtures, format, tooling/container/release/mail checks,
  web types/build/models, strict workspace Clippy and all **30 browser journeys**
  pass. Linux Rust totals: **601 passed, five existing ignores, zero failed**.
  Receipt base is `e64bbab8b6b2f21d2fad26a32fa3a8dce71d9e88`; full coverage includes
  all functional changes since beta.16. Source artifact **13083288090**, digest
  `sha256:032be75f26b86365582f10ba03cc452eb099779a16e8c73e665ac449415e4378`.
- [Container validation 41304328387](https://github.com/AberrantWolf/junk-collector/actions/runs/41304328387),
  attempt 1: native AMD64/ARM64 runtime, proxy, inbox, cleanup, recovery and
  isolated-media checks pass, plus browser acceptance against the actual AMD64
  release image. The source job reused the trusted same-SHA full receipt rather
  than compiling again. Artifact **13083197323**, `validated-container-1`, digest
  `sha256:bb5db41149722a9f3a760c3b1e9751a269ec72e1b4f713e9de99da8d8c5e6390`;
  sealed OCI archive digest
  `sha256:f83587c90ddf72111459d96aa68f9488de060d6d69a8bfd3e9f64fbf2e5fac7c`.
- [Publication 41306116801](https://github.com/AberrantWolf/junk-collector/actions/runs/41306116801)
  consumes those sealed bytes with build/source jobs skipped. Anonymous registry
  inspection matches both platform manifests/configs and source/version labels.
  `preview` matches beta.17; stable `latest` remains absent. Release bundle and
  image-record checksums pass. Duplicate branch and tag-push check runs
  **41304322474** and **41306115305** were cancelled; required qualification and
  publication succeeded.

## Published and installed identity

```text
image: ghcr.io/aberrantwolf/junk-collector:v0.1.0-beta.17
index: sha256:2fd55806cf98e2664e31852702df894e8d3510f3ad0cf04c3eb4dec650f98ba6
linux/amd64: sha256:45acb9472baa320147544954fcaf85b316d3885064dc5cd10cfaead3162115cf
linux/arm64: sha256:5406ecd2155e5a48eda93acc9e0eeb49bc77f16da4ef90982595b700204f33e1
```

application-host pulled anonymously by immutable index before downtime. Image architecture,
version/source labels and RepoDigest match the release record. Core/media pin
those same immutable bytes. No local deployment build occurred.

## Complete stopped-writer recovery pair

No Collector Jobs were unfinished. The guarded service stopped all five stack
services; all actual PIDs reached zero and all five exit codes were zero.
After writeback completed, both read-only snapshots succeeded without excluded
nested subvolumes:

- application-host: `/var/backups/collector-recovery-pairs/before-beta17-20261101-fe7b95c/application-data`.
- NAS: `/volume1/collector-recovery/before-beta17-20261101-fe7b95c`.
- NAS snapshot UUID: `32bf3679-e677-2e46-a8a7-932aab4a18be`.

The matching pair was sealed before activation. application-host covers all local private
state/WAL, imports/uploads/scratch, config/TLS, Bridge home/cache and profiles;
NAS covers the complete archive parent subvolume. Snapshot database quick checks
pass. Private recovery metadata retains Compose/environment/systemd/fstab/helper
files, prior image identities, numeric ownership of the media authority service
key and cutoff/acceptance facts. Rollback requires both matching members with all
writers stopped; an image-only or database-only rollback is insufficient.

## Live acceptance and preservation

Acceptance completed at **2026-11-01 21:25:03 JST**. The executable reports
`junk-collector 0.1.0-beta.17`. Core is healthy, core/media run the qualified image,
and all five stack containers have zero restarts. Public and LAN HTTPS health
return 200 with normal certificate verification and unchanged ServerId
`01a15429-856a-7763-b558-3c158b1604a0`. Served `app.js`, `app.css`, `curation.js`,
`library-imports.js` and `purchase-inbox.js` match the frozen release source.

Counts remain **123 Items, one user, two grants and 3,076 Jobs**, with zero
unfinished work. Config/TLS, media authority key, mounts, UID/groups, isolation,
restart policy, commands and topology are retained. Only core/media were recreated.
Bridge, Caddy and tunnel container/image identities are retained, and all **seven
unrelated containers** retain running state and start times.

Both credential-bearing profile files are identical. The managed connection keeps
its profile, collection, revision/cursor and enabled state; receipt suggestions and
accepted-purchase Items remain zero. Bridge pin is unchanged and its reporter is
fresh, reporting `synced`; that report is not an exhaustive mailbox audit. No
mailbox LOGIN, credential replacement, rescan or scanning-setting change was issued.

Production acceptance used read-only runtime/state/asset checks. Functional
failure/retry, original download hashes, real playback, automatic FLAC and artwork
acceptance ran in disposable archives with mounted inbox inputs. No synthetic
production accounts/Items were created and no failed production import was
silently retried. Failed identification can now be explicitly retried from
**Review import → Retry this step**. Installed Trash Amp is unchanged.

Private operation evidence is retained in `/home/operator/collector-beta17-fe7b95c`
on application-host and `/tmp/collector-beta17` locally.
