# Beta.14 publication and NAS update — 2026-10-30 JST

[Beta.14](https://github.com/AberrantWolf/junk-collector/releases/tag/v0.1.0-beta.14)
is published on `preview` and installed on FatChocobo. It delivers reviewed digital
album matching and archive admission/interruption safeguards. The NAS retains its
existing local Btrfs archive/private-state layout. This is not Fi migration approval.
See [release notes](../releases/v0.1.0-beta.14.md).

## Exact-source release evidence

- Signed commit and signed tag resolve to
  `e470ef501312fad8c260934919210c884286e761`; both signatures verify locally.
- [Checks 41002882082](https://github.com/AberrantWolf/junk-collector/actions/runs/41002882082),
  attempt 1, passes the complete 93-path change since beta.13: architecture/docs,
  formatting, release/testing/container tools, workspace tests, warning-denied
  all-target Clippy, web checks/build/models and all **29** browser journeys.
  Rust results total **558 passed, 5 ignored, 0 failed**. Workspace tests took
  **403.84 s**, Clippy **22.58 s**, browsers **548.21 s**. The committed-source
  receipt reports full coverage and unchanged source. Artifact **12955191747**:
  `sha256:913e877ab9a736c17852967de25973b817715201a59943dcab93b313398244e4`.
- [Container validation 41002882164](https://github.com/AberrantWolf/junk-collector/actions/runs/41002882164),
  **attempt 2**, reuses that exact-source receipt and passes native AMD64/ARM64
  runtime/proxy/inbox/cleanup/recovery and isolated-media acceptance, plus AMD64
  browser acceptance against the release image. All jobs and sealing succeed.
  Artifact **12955087643**, `validated-container-2`, ZIP digest
  `sha256:4cc300fa08383e00e6329465c4f72fdc002d08ec07bd95817cb7d955456aab86`.
  Its locally downloaded 107,008,000-byte OCI archive matches sealed SHA-256
  `d7b759da1cf24e08d96983b061e71dbf8af0f1d9df6be16d1b2452fd614ea9c4`.
- [Publication 41005192492](https://github.com/AberrantWolf/junk-collector/actions/runs/41005192492)
  succeeds, consuming those validated bytes with source/build jobs skipped.
  Anonymous registry inspection independently matches source/version labels,
  both platforms and `preview`. Release bundle/image-record checksums pass, and
  the downloaded bundle contains all new remote deployment files.
- Official current-stable Rust remains 1.99.1, manifest dated 2026-10-15, matching
  repository/CI/container pins; no toolchain update was needed.

Qualification found and fixed two delivery gaps before publication: the bundle
omitted remote deployment files, and the cleanup probe used the old broad `/data`
mount instead of the CLI's `/inbox` mount. Checks were corrected without relaxing
storage or cleanup assertions. Superseded candidates were not published. Attempt 1
at the final source missed newly completed same-source evidence and selected
redundant checks; the whole validation was deliberately cancelled and rerun with
the now-available verified receipt. Attempt 2 alone qualifies the release; artifacts
from attempts were not combined. Receipt-visibility handling is a tooling follow-up.

## Published and installed identity

```text
image: ghcr.io/aberrantwolf/junk-collector
index: sha256:f750ce8124b8fd30d1054383ad32451f4e80009f108ac93de85518dd7c8ff066
linux/amd64: sha256:957d07d411fc35065a88ec22911f071bdbc4ea4dbdee6ca5191506146a8564b5
linux/arm64: sha256:d8ff40f48aba1d3aa4a80fad5899af9a2ef78174ec5fb1c5ce4d2c606eedb557
installed amd64 image ID: sha256:85a4a096e106c56599e8fd39a9c17c492c3e7ba48dda5a3c2394094568fe0abd
```

Core, media and the retained one-shot media initializer reference this immutable
index. Only core/media were recreated (`--no-deps --pull never`). `latest` was not
promoted. The expanded retained Portainer definition differs only at those three
image references; configuration was not replaced with new sample templates.

## Complete stopped-server recovery point and enrollment

Read-only inspection reconfirmed `/volume1/junk-collector` is one Btrfs subvolume,
with no nested subvolumes and all authoritative config, archive and complete state
inside it. The approximately **76 GB `state/imports`** is retained in full. The
separate external inbox retains its existing mount and permissions; no cleanup
or new processing batch was requested or started.

The new image was pulled before downtime. Both beta.13 services exited **0**, with
no unfinished jobs. Their manually stopped `unless-stopped` policy kept them
stopped during offline operations. The retained beta.13 image and full restricted
stack/runtime metadata accompany the complete read-only snapshot:

```text
/volume1/collector-recovery/before-beta14-20261030T045400Z
/volume1/collector-recovery/before-beta14-20261030T045400Z-metadata
```

The first extra SQLite check could not initialize SQLite shared memory on the
read-only snapshot. With both services still stopped and before enrollment, only
the snapshot's SQLite databases and WAL files were copied into isolated writable
tmpfs scratch. `PRAGMA quick_check` passes for both databases. The snapshot itself
remains unchanged. Config, imports and archive collection roots were also checked.
There was no full-media copy, scan/hash or destructive production storage test.

The qualified new CLI then ran explicit offline `enroll-archive` as UID/GID
10001:10001 plus GID 65538 against the confirmed retained roots. Archive identity
and private-state binding match and are recorded in the restricted operation
receipts. No setup or replacement archive was initialized. The old snapshot is a
complete **beta.13** recovery point, intentionally predating enrollment.

The measured stopped-checkpoint-to-identity-readiness interval was **105.7 s**,
including the scratch-verification adjustment. It excludes the preceding clean
stop duration. Beta.14 core/media then started with the preserved ServerId,
config and TLS hashes. Both have zero restarts.

Rollback uses the retained old image and this entire matching snapshot, with all
writers stopped and current later writes preserved separately before selecting a
rollback cutoff. Do not merely downgrade the image against newer data. The
[snapshot runbook](../../deploy/container/snapshot-recovery.md) applies to this
single-NAS layout. The [paired procedure](../../deploy/container/remote-archive.md)
is mandatory after the split; restoring a NAS archive without matching Fi state
would not recover the installation.

## Current-NAS acceptance and limits

- Full runtime command/environment/security configuration and mounts are retained.
  Docker's inherited empty DNS fields serialize as `[]` before and `null` after;
  these were compared as equivalent empty lists. No resolver setting changed.
- UID/GID 10001:10001 and core supplemental GID 65538 remain. Core retains archive
  and inbox write access. Media retains read-only archive/authority mounts, its
  isolated network namespace and no private state/config/inbox access.
- All **18 other containers**, including Caddy/tunnels, retain IDs, images and
  start times. NAS exports/permissions, router and tunnels were not changed.
  Local playback retains `192.168.1.106:9845`; public hostnames remain stable.
- Public HTTPS uses normal system trust. HTML and **12 assets** pass; their bytes
  match the newly running core. Protected control returns 401, media control 404
  and grantless media reads 401. Public media identity traverses the authority
  socket and matches core's retained ServerId.
- All **79 desired Library summary variants** are ready, with no pending variants.
  The **1 account, 1 grant and 2,585 jobs** match the stopped snapshot. There are no
  unfinished jobs after restart. This adds no active-job interruption claim.
- Private operation scripts/receipts are retained at
  `/volume1/homes/rynban/collector-beta14-update-20261030`, including the failed
  read-only snapshot check, explicit resume and final verification. No authenticated
  production browser/playback session was exercised in this update; browser and
  media functional acceptance ran against isolated qualified source/images.

Fi migration still requires the exact approval criteria in the remote runbook:
actual supported NFS write/durability/outage and reconnect qualification, measured
local SSD capacity including all imports/scratch/recovery needs, effective
single-writer and boot fences, correct Fi listener addresses and a demonstrated
complete paired restore/rollback with pending work. Read-only NFSv3 access probes
from the migration assessment do not qualify the required NFSv4.1/4.2 contract.
