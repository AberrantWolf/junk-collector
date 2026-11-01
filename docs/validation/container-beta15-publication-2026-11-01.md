# Beta.15 publication and Fi update — 2026-11-01 JST

[Beta.15](https://github.com/AberrantWolf/junk-collector/releases/tag/v0.1.0-beta.15)
is published on `preview` and installed on Fi. It includes the complete Collector
change since beta.14: private email purchase intake and Bridge setup/status/recovery,
WAV digital originals/preparation/tags/artwork/matching, retained-source analysis,
metadata-aware FLAC downloads and intervening fixes. See
[release notes](../releases/v0.1.0-beta.15.md).

## Exact-source qualification and publication

- Signed candidate commits `a22748f` and `6c0f42a` are pushed to `main`.
  Signed tag `v0.1.0-beta.15` resolves to
  `6c0f42a5c616a839e8845ad88d78ece3cbbd01a2`; commit/tag signatures verify locally.
- [Checks 41267636169](https://github.com/AberrantWolf/junk-collector/actions/runs/41267636169),
  attempt 1, qualifies the complete candidate against beta.14
  (`e470ef501312fad8c260934919210c884286e761`), with full coverage at beta level,
  `infra` classification and unchanged committed source. Architecture/docs/fixtures,
  formatting, tooling/container/release/mail tests, web types/build/models,
  workspace tests, all-target warning-denied Clippy and all **30 browser journeys**
  pass. Rust totals: **592 passed, 5 explicitly ignored, 0 failed**. Receipt artifact
  **13067998367**, SHA-256
  `a2e59650ee2f5fd9a71e5ca5849f4a798aed7c4f2632669b38497553724f6bf9`,
  was independently recovered with the maintained trusted-receipt checker.
- [Container validation 41267638173](https://github.com/AberrantWolf/junk-collector/actions/runs/41267638173),
  attempt 1, reuses that receipt and builds one final release image per architecture.
  Native AMD64/ARM64 runtime/proxy/inbox/cleanup/recovery and isolated-media checks
  pass; AMD64 browser acceptance passes against the actual release image. The
  sealed `validated-container-1` artifact is **13068068528**, ZIP digest
  `sha256:a94ea2a5b7e804b6ec07659d94809b94207eb247132f02088b333b233e49b858`.
  Sealed OCI archive SHA-256:
  `9a31b2431df946cd07f486d63935adb76586951c1e2accce7ce42dbe8b498650`.
- [Publication 41269247985](https://github.com/AberrantWolf/junk-collector/actions/runs/41269247985)
  succeeds by consuming those sealed bytes, with build/source-check jobs skipped.
  Anonymous version pulls and both-platform labels are verified by the workflow.
  Independent anonymous registry manifest/config reads also match the release
  record; `preview` matches the new index and stable `latest` remains absent.
  Downloaded release bundle/image-record checksums pass.

Release review caught an installation-bundle gap before publication: the new mail
override, Bridge build context and provisioning helper were omitted. The first
candidate's runs **41267239558/41267247279** were cancelled, and `6c0f42a` repaired
packaging with meaningful byte/link assertions. Its downloaded published bundle
contains the exact mail override, Dockerfile/entrypoint/status reporter/public
verification key/policy and local provisioning helper. Bridge remains an optional
self-managed Linux AMD64 image, separate from the multi-platform Collector image.
No superseded candidate was published. The redundant same-source tag-push check
**41269245956** was cancelled after full source/container qualification; required
qualification and publication runs succeeded. Local `gh` log-cache permission
failure was resolved with an isolated temporary cache, without affecting CI results.

## Published and installed identity

```text
image: ghcr.io/aberrantwolf/junk-collector:v0.1.0-beta.15
index: sha256:bfcf99db4647ecde8bf62f8fe2be1565778e74f5ec26f9f2d9f766c300473d3e
linux/amd64: sha256:a1ec2ded538cbd9f030f9228f5549c5bc390b4913e62213348800ecac610431d
linux/arm64: sha256:feaec549a954414a12c12dfca089db219c130bef9fbebe2365d314528af61122
amd64 config: sha256:bc08595bb29acab8a0771b38b368a9e9cd1cc72914a8c0a9db2db1106db1cd23
```

Fi pulled anonymously by immutable index before downtime. Its containerd-backed
Docker image ID is the index digest, rather than the platform config digest.
Architecture/version/source labels and the pulled RepoDigest match the published
record. Both retained core/media environment references pin that immutable image;
no local deployment build occurred. Bridge remains
`sha256:add5534fa0dcb507bbc9b6ea9040071abf037fdbe968645ff46a8b0a2975b5b1`.

## Complete stopped-writer recovery point

There were no unfinished Collector Jobs. The guarded systemd unit stopped all five
stack services, their PIDs reached zero, clean exit policy passed, and filesystem
writeback completed before snapshots. Neither source had excluded nested subvolumes.
Both read-only members succeeded before the pair was sealed:

- Fi: `/var/backups/collector-fi-pairs/before-beta15-20261101-6c0f42a/fi-data`.
- NAS: `/volume1/collector-recovery/before-beta15-20261101-6c0f42a`.
- NAS snapshot UUID: `a3fac94d-5229-0743-b717-4fddda1ad14b`.

Fi covers complete local private state/WAL, imports/scratch, configuration/TLS,
Bridge home/cache and profiles. The NAS point covers the full archive parent
subvolume. Snapshot database quick checks pass. The private pair directory also
retains Compose/environment/systemd/fstab/helper definitions, the media authority
service key with numeric ownership, prior image identities and cutoff/acceptance
facts. Recovery requires both members of this matching pair with all writers
stopped; an image-only or single-database rollback is insufficient.

## Live acceptance and preserved state

Acceptance completed at **2026-11-01 12:30:10 JST**. Core is healthy, both core/media
run the qualified image with zero restarts, and the actual core executable reports
`junk-collector 0.1.0-beta.15`. Public and LAN HTTPS health return 200 with normal
certificate verification. Both routes retain ServerId
`01a15429-856a-7763-b558-3c158b1604a0`; served `app.js`, `app.css`, `curation.js`,
`library-imports.js` and `purchase-inbox.js` match frozen release-commit bytes.

Config/TLS and media-authority key hashes, mount sources, user/groups/isolation,
restart policy, commands and topology are unchanged. Existing Compose/systemd
installation paths are retained. Only core/media were recreated; Bridge, Caddy and
tunnel container/image identities remain intact. All seven unrelated containers
retain their original running state and start times.

Catalog/security/Job counts remain **79 Items, one user, two grants and 2,585 Jobs**,
with no unfinished work. Both private credential-bearing profile files are identical;
the one managed connection retains its profile, collection, revision/cursor and
paused state. Receipt suggestions and accepted-purchase Items were zero at cutoff
and remain zero; no history was cleared or recreated. The Bridge certificate pin
is unchanged and the reporter is fresh; its initial post-restart phase is explicitly
unknown, rather than a claimed completed synchronization. No mailbox LOGIN,
credential revalidation/rotation, rescan or scan enablement was performed.

The existing `remote_media_allowed = false` policy, listeners and HTTPS origins
remain unchanged. Production acceptance used read-only runtime/state/asset checks;
no synthetic production Item/account or mail was created. Functional authenticated
browser/audio and upgrade/recovery acceptance ran against disposable fixtures.
Physical audio output, live provider recognition and personal receipt calibration
retain their previously documented limits. Private source/operation logs remain
under `/home/link/collector-beta15-6c0f42a` on Fi; local evidence is under
`/tmp/collector-beta15`.
