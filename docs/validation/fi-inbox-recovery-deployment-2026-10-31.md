# Fi inbox recovery deployment — 2026-10-31 JST

The user authorized deploying the reviewed inbox connector changes to Fi while
preserving the existing Bridge connection and receipt history. Signed source
`0bec4afed811ad91e26d8ff16f3329f8ab59f042` was explicitly approved for GitHub
`origin/main` and pushed. This is a Fi-only exact-source update; no release tag,
registry release, or channel change was published.

## Exact installed artifact

The Git archive SHA-256 is
`70d61909c917d3ed1a3af85ce1b2c9dfebac1c5fdc2861bbba25947b629912f6`.
Fi verified that archive and built the deployment image once with the source
revision label. The existing Dockerfile built optimized Collector/media executables;
no standalone release build or Bridge image rebuild was performed.

- Collector and media: `sha256:f97ed0c5e4039eb9e2d7c7d65165b4a844592d91cfb3540747320d3a9833a5eb`.
- Local image tag: `junk-collector:inbox-0bec4af`.
- Bridge retained: `sha256:add5534fa0dcb507bbc9b6ea9040071abf037fdbe968645ff46a8b0a2975b5b1`.

The retained environment pins the new image for core/media. Compose, systemd,
Bridge configuration/image/home, proxy/tunnel settings, listeners and mount sources
remain unchanged. The reviewed host mail-helper guidance was updated. Source/build,
qualification logs and operation scripts are retained privately under
`/home/operator/collector-inbox-0bec4af` on Fi.

## Qualification before activation

The [source validation and independent clean-context review](inbox-recovery-2026-10-31.md)
were reused with fresh qualification of the actual release image:

- Native initialize/configuration/group access, old-to-new partial-upload upgrade,
  clean SIGTERM, FLAC, full disposable backup/restore, read-only inbox cleanup refusal
  and explicit writable cleanup all passed. Optional metadata was unavailable in
  the disposable fixture; originals and explicit FLAC remained usable.
- Shipped media isolation and Caddy fixture TLS/Unix-upstream/routing/log checks passed.
  These tests mounted disposable data only.
- The old production image created a synthetic Bridge profile through its actual
  web UI, then accepted one receipt and dismissed another. The candidate started
  over that same fixture state. Exact profile bytes, paused connection, revision,
  collection/cursor, both suggestions/dismissal and the accepted purchase were retained.
- The candidate's served `app.js`, `app.css` and `purchase-inbox.js` matched the
  committed `0bec4af` bytes. Real browser actions passed folder edit/save, pinned
  credential revalidation, reload, restore, accepted-purchase reopen/correction/
  reassignment, and 390px mobile overflow checks. Screenshots were inspected.

Only one candidate image was built, and these same tested bytes were installed.
The copied fixture harness disabled its local-playback listener to avoid the live
port. Initial mail-fixture attempts had an unsupported HTTP passkey-origin setting
and a health probe in the helper's isolated network namespace; only the disposable
fixture/invocation was corrected. A later asset comparison detected concurrent local
workspace edits; switching the expected bytes to the frozen commit confirmed the
candidate was correct. Failed fixture attempts were not counted as acceptance.
All disposable fixture containers/data and the loopback SSH tunnel were removed.

## Complete stopped-writer recovery pair

There were no unfinished Collector Jobs. The guarded systemd unit stopped all five
stack services. Core, media, Bridge and Caddy exited 0; tunnel shutdown was accepted
under its established normal-stop policy. All container PIDs were zero before sync
and snapshots. Both read-only snapshots succeeded before the pair was sealed:

- Fi: `/var/backups/collector-fi-pairs/before-inbox-recovery-20261031-0bec4af/fi-data`.
- archive-host.example.com: `/volume1/collector-recovery/before-inbox-recovery-20261031-0bec4af`.
- NAS snapshot UUID: `ac167c0c-9644-4e4c-83f6-7af96ead7943`.

The Fi snapshot covers all local private state/WAL, profiles, Bridge vault/cache,
configuration, TLS, imports and scratch. The NAS point covers the entire archive's
parent subvolume. Snapshot database quick checks passed; neither source had excluded
nested subvolumes. The private Fi pair directory also retains the old Compose/
environment/systemd/fstab/helper files, service authority key, image identities,
cutoff/acceptance facts and the complete-pair record. Recovery must use both members
of this pair; a single database or archive-only rollback is insufficient.

## Live acceptance and preservation

The new core/media image is running, with the core healthy. Verification completed
at **2026-10-31 23:37:44 JST**. The retained server identity, TLS/configuration bytes,
media authority key, mount sources, UID/groups/isolation/restart policy and proxy/
tunnel/Bridge container identities match the prior installation. All seven unrelated
containers retained their running state and original start times.

At cutoff there was **one managed Bridge profile and one saved connection, paused**.
The legacy profile file was also retained. Both credential-bearing profile files
remain byte-for-byte identical with private permissions. The connection's profile
ID, collection, enabled flag, revision and cursor remain unchanged. No credential
rotation, revalidation, rescan or scan enablement was performed on production.

There were **zero receipt suggestions and zero Items with accepted purchase orders**
at cutoff; those counts remain zero. Nothing was cleared or recreated. Existing
catalog/security/Job counts remain **79 Items, one user, two grants, 2,585 Jobs**.
The complete private state/history is retained by the snapshot and live update.

Bridge resumed its existing sync and reported **92.0%** at acceptance; its public
certificate fingerprint matches the pre-update value and saved profile pin without
a mailbox LOGIN. Public HTTPS health returns 200, all changed served asset bytes
match the committed source, and LAN connections to ports 1143/1025 remain refused.
No real message or credential was printed or used by the development fixtures.
Actual personal receipt calibration remains an operator workflow after enabling
scanning; this update preserves its existing paused state.

Concurrent WAV implementation work that appeared after `0bec4af` was committed
was preserved in the local workspace and is outside this deployment artifact.
