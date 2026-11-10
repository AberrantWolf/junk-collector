# Beta.16 publication and application-host update — 2026-11-01 JST

> Deployment identifiers are anonymized; dates, source identities, outcomes and qualification limits are unchanged. Example addresses are not deployment instructions.

[Beta.16](https://github.com/AberrantWolf/junk-collector/releases/tag/v0.1.0-beta.16)
is published on `preview` and installed on application-host. It delivers parent album naming and
companion retention for exact case-insensitive FLAC/MP3/WAV format folders, both
disc/wrapper orders and disjoint albums/sections/alternate encodings. Shared parent
companions have one deterministic owner. Existing archived imports are unchanged.
See [release notes](../releases/v0.1.0-beta.16.md) and
[source behavior/acceptance](format-folder-discovery-2026-11-01.md).

## Exact-source qualification and publication

Signed source/tag `a6bc202e4abd746a3bad55c441d04c42fdba2935` is pushed to `main` and
`v0.1.0-beta.16`; both signatures verify locally. No candidate change followed
qualification. The later documentation commit records these completed operations.

- [Checks 41273075119](https://github.com/AberrantWolf/junk-collector/actions/runs/41273075119),
  attempt 1: full source coverage at beta level, `infra` classification, unchanged
  committed source. Architecture/docs/fixtures, formatting, tooling/container/
  release/mail checks, web types/build/models, workspace tests, strict all-target
  Clippy and all **30 browser journeys** passed. Linux Rust totals: **597 passed,
  five existing qualification tests ignored, zero failed**. Its full receipt uses
  base `27f84088e533b7567e83f83569a984a6449c4c41`; the release requested beta.15
  (`6c0f42a5c616a839e8845ad88d78ece3cbbd01a2`) as baseline and reused
  full coverage through the maintained trusted-receipt checker. Receipt artifact
  **13070358630**, digest `sha256:c3efa399b537251c2db36addefa30add1157124791becba8367ff68f5963705f`.
- [Container validation 41273078930](https://github.com/AberrantWolf/junk-collector/actions/runs/41273078930),
  **attempt 2**, passed both native AMD64/ARM64 runtime/proxy/inbox/cleanup/recovery
  and isolated-media checks, plus browser acceptance against the actual AMD64
  release image. Artifact **13070437434**, `validated-container-2`,
  digest `sha256:9727f3d4924f1a07afc9e3f14b5d9c09567ad093b876837f159213d57e24c4ec`; sealed OCI archive digest
  `sha256:cdc5e4b5822a4a6614dd895d03b645dbd12fddc2104237e9052a1278e0f8a4b4`.
- Attempt 1 failed AMD64 browser acceptance after Chromium reported
  `net::ERR_NETWORK_CHANGED` on the administration settings request, leaving the
  collection form disabled. Runtime checks had passed. The attempt was cancelled
  and **all** container validation jobs rerun against unchanged source; no failed
  or mixed-attempt image was published. Attempt 2's browser check passed.
- [Publication 41274503926](https://github.com/AberrantWolf/junk-collector/actions/runs/41274503926)
  consumed the sealed bytes, with build/source jobs skipped. Anonymous registry
  manifest/config inspection independently matches both platforms and source/
  version labels. `preview` matches beta.16; stable `latest` remains absent.
  Downloaded installation bundle and image-record checksums pass. The redundant
  same-source tag-push check **41274501021** was cancelled after required full
  qualification; qualification and publication runs succeeded.

## Published and installed identity

```text
image: ghcr.io/aberrantwolf/junk-collector:v0.1.0-beta.16
index: sha256:baa877df6c5d9b99b0e4aa3b144ec5b3abc8a5615a425a9814a727708b7aa080
linux/amd64: sha256:5b10648cab9c255ee8efeae2efed3f51775caedc227c77926a00db91734d7a5c
linux/arm64: sha256:3d86bf6653dbd3c4f0ba687460a7607822a1f3f7c9c73961b1894613c8914fe0
```

application-host pulled anonymously by immutable index before downtime. Docker's image ID on
this containerd-backed host is that index digest. Architecture/version/source
labels and RepoDigest match the release record. Existing core/media environment
references now pin that immutable image. No local deployment build occurred.

## Complete stopped-writer recovery point

No Collector Jobs were unfinished. The guarded service stopped all five stack
services; actual PIDs reached zero and clean exit policy passed. After writeback
completed, both read-only snapshots succeeded with no excluded nested subvolumes:

- application-host: `/var/backups/collector-recovery-pairs/before-beta16-20261101-a6bc202/application-data`.
- NAS: `/volume1/collector-recovery/before-beta16-20261101-a6bc202`.
- NAS snapshot UUID: `297873f8-6180-e34c-9793-d42d0e24c2d5`.

The pair was sealed before activation. application-host covers all local state/WAL, imports,
uploads/scratch, configuration/TLS, Bridge home/cache and profiles. The NAS point
covers the full archive parent subvolume. Snapshot database quick checks pass.
The private pair directory also retains Compose/environment/systemd/fstab/helper
definitions, numeric ownership of the media authority service key, prior image
identities and cutoff/acceptance facts. Recovery requires both matching members
and all writers stopped; an image-only or database-only rollback is insufficient.

## Live acceptance and preservation

Acceptance completed at **2026-11-01 13:52:00 JST**. The actual executable reports
`junk-collector 0.1.0-beta.16`; both core/media run the qualified image with zero
restarts, and core is healthy. Public and LAN HTTPS health return 200 with normal
certificate verification and unchanged ServerId
`01a15429-856a-7763-b558-3c158b1604a0`. Served `app.js`, `app.css`, `curation.js`,
`library-imports.js` and `purchase-inbox.js` match frozen release-source bytes.

Config/TLS and media authority key hashes, mounts, UID/groups, isolation, restart
policy, commands and topology are unchanged. Existing Compose/systemd installation
paths are retained. Only core/media were recreated. Bridge, Caddy and tunnel
container/image identities are retained; all **seven unrelated containers** keep
their running state and start times.

Counts remain **79 Items, one user, two grants and 2,585 Jobs**, with zero unfinished
work. Both credential-bearing profile files are identical. The managed connection
retains its profile, collection, revision/cursor and paused state. Receipt suggestions
and accepted-purchase Items remain zero; nothing was cleared or recreated. The
Bridge certificate pin is unchanged and its reporter is fresh. Its initial phase is
explicitly unknown, not a claim of complete synchronization. No mailbox LOGIN,
credential replacement, rescan or enabling of scanning was performed.

The existing exposure/media-access policy remains unchanged. Production acceptance
used read-only runtime/state/asset checks; no synthetic production Items/accounts
or mail were created. Functional authenticated browser/audio and upgrade/recovery
acceptance ran against disposable fixtures. Private operation evidence remains in
`/home/operator/collector-beta16-a6bc202` on application-host and `/tmp/collector-beta16` locally.
