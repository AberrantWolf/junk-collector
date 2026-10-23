# HL-07D container publication acceptance — 2026-10-20

## Source and workflow

The signed `v0.1.0-alpha.2` tag identifies
`0f0e0f52fedc239ecdb180436583e69b7ce13a75`.
The [GitHub prerelease](https://github.com/AberrantWolf/junk-collector/releases/tag/v0.1.0-alpha.2)
and [release workflow](https://github.com/AberrantWolf/junk-collector/actions/runs/39354637937)
were started under the user's explicit publication assignment.
Architecture/link checks, container-tools/release policy tests, Rust formatting,
workspace tests, warning-denied all-target Clippy, web checks/build, committed asset
comparison and the release server build passed on GitHub. The native container
build and shipped proxy/inbox install/update/full-backup/restore smoke passed.
The earlier [preparation record](container-release-preparation-2026-10-20.md) retains
its dated, pre-publication state.

The [initial emulated run](https://github.com/AberrantWolf/junk-collector/actions/runs/39344034491) reached Collector's final crate but spent more
than 40 minutes there (over 95 minutes in the container job). It was cancelled
before version/channel/bundle acceptance, rather than repeatedly waiting on QEMU
release optimization. The existing workflow now builds/smokes each architecture
on its native GitHub runner and assembles digest-addressed outputs. No tag is moved.
An explicit manual retry selects the already published tag, fetches its release
status and freezes its source commit before source checks/builds. Empty-input
manual dispatch remains nonpublishing. The tiny two-platform export fixture passed
actual digest-preserving OCI copy/platform/source-label verification after merge;
Actionlint passed. The fixture qualifies merge handling; actual public-image results follow below.

**Delivered:** public version/preview images, both-platform anonymous pulls, digest-pinned
installation assets and isolated actual-host Portainer update/fresh-install/recovery
acceptance all passed. The source tag remains unchanged. Qualification tooling is
signed in `e40211d`; the native release workflow is signed in `0af4517`.

## Isolated Portainer acceptance mechanism

`scripts/check_container.py --portainer-image …` reuses the existing real application
journey. `scripts/portainer_qualification.py` supplies only the service lifecycle:
a disposable Portainer CE 2.45.0 with its own generated database volume, temporary
admin account, loopback listener and Docker-socket environment. It never opens the
operator Portainer database or registry credentials. Fixture project names include
a random suffix; cleanup targets only that generated stack/container/volume/root.

The stack is rendered from the shipped Compose definition and inbox override,
retaining UID/GID, read-only root, capabilities, host networking, bind roots and
supplemental inbox group. One-shot initialization/setup/backup/restore still uses
the same CLI image as the selected server. Public images are pulled through the
actual Portainer Docker gateway without registry credentials. A Collector-only
stack exercises Portainer 2.45's `RepullImageAndRedeploy`. With
`--portainer-keep-companion`, an unchanged, networkless `sleep` control container
must retain its container ID and start time: the adapter pulls Collector's image
and updates its service with whole-stack forced redeployment disabled. This
qualifies service targeting, not a simulated tunnel. Both modes wait for the
asynchronous deployment result and require a replaced Collector with the selected
immutable image ID.
See [Portainer's API explanation](https://docs.portainer.io/api/examples) and
[the versioned update implementation](https://github.com/portainer/portainer/blob/2.45.0/api/http/handler/stacks/stack_update.go).

A preflight with the retained `grouped-fix-20261019` and `hl05a-qualification` local
images passed, including retention of the companion through update and restore.
This establishes the test adapter, not public alpha.2 acceptance.
Self-review caught that Portainer stack stop removes containers; maintenance now
stops only Collector via its container API and checks its retained clean exit.
The ordinary Compose path also passed after this shared test refactor.

The journey preserves an already committed package and a separate partial upload
through a distinct-image update, retaining original login session/account,
ServerId, exact config and TLS certificate. A complete old-image CLI backup is
validated before update. It compares every original byte, resumes the recorded
upload offset, checks one publication, group-readable inbox import, explicit FLAC
preparation and ranged media, then validates/restores a full backup into empty
roots while the source server is stopped. The unknown companion file is retained.
Unavailable optional metadata remains a visible failure without blocking originals
or explicit audio preparation.

## Published image and bundle results

The native release workflow completed successfully, including full source gates,
actual native AMD64 and ARM64 proxy/inbox initialize/setup/upload/recreation/FLAC/
backup/restore smoke, digest-only platform publication, version-index assembly,
anonymous Skopeo all-layer copy/config checks and credential-empty Docker pulls
for both architectures. Version and `preview` select the same index; `latest`
was not created. No browser visibility mutation was needed: anonymous checks passed.

| Coordinate | Verified value |
| --- | --- |
| Version | `ghcr.io/aberrantwolf/junk-collector:v0.1.0-alpha.2` |
| Index | `sha256:f86f46d2c8870f8d647fc9225ce2cfe7c4319521a29c89897b419abb77629433` |
| AMD64 manifest | `sha256:fa5854547379d9a1111732aa3104e8a889456ec160d3d0b62a779fcadcf02f93` |
| ARM64 manifest | `sha256:66927f654e9896ef8bf35a509b659dc0364f945ce4906a3ed904bb64d7ad166b` |
| Source revision | `0f0e0f52fedc239ecdb180436583e69b7ce13a75` |
| NAS AMD64 local image ID | `sha256:fd117ce25e514954c829c88c70f63dd8b9dee961444806978544e1b631dc50c9` |

The public `junk-collector-container.tar.gz`, `release-image.json` and `SHA256SUMS`
were downloaded both through GitHub CLI and independently over anonymous HTTPS;
bytes and SHA-256 checksums matched. The extracted bundle includes its complete
Compose/inbox/config/recovery files and the same image record. Its `.env.example`
pins the index above; shipped Compose/inbox/proxy files match the tested source.
Independent anonymous registry GETs checked actual index bytes and digest headers
for version/preview, and a 404 for `latest`.

## Actual published-image NAS and browser acceptance

On the Synology DS1522+ (Btrfs, Docker 24.0.2, Compose 2.20.1, Python 3.8.15),
two disposable Portainer CE 2.45.0 installations completed with exit status 0:

1. **Version-tag update:** `grouped-fix-20261019` → public alpha.2, using a separate
   pre-update full backup and old CLI. Portainer anonymously pulled the public
   image; the changed Collector was replaced while the unchanged companion kept
   its container ID/start timestamp, including through restore. Saved originals,
   a separate pending upload's exact offset, original session/account, ServerId,
   config and TLS survived. Original byte comparisons, one publication, group-readable
   inbox intake, explicit FLAC and ranges passed, followed by validated full backup
   and empty-root restore. Clean stops took 0.22/0.23/0.21 seconds.
2. **Fresh digest installation and deliberate re-pull/redeploy:** the exact recorded
   digest was anonymously pulled through a fresh Portainer environment, initialized
   in fresh roots, then recreated through `RepullImageAndRedeploy=true` in a
   Collector-only stack. Original/session/config/TLS/upload/FLAC/full restore checks
   passed; clean stops took 0.23/0.28/0.28 seconds. The replaced container's image
   had to match the selected immutable local image ID.

For case 1, actual remote Chrome 153.0.8010.37 passed both after update and after
restore: login, album/track playback with advancing time, seek, exact original
SHA-256 downloads, retained playback after closing detail, no page errors and
390×844 mobile overflow checks. Desktop/mobile screenshots were inspected. The
harness independently validated the generated PEM, hostname and ServerId before
using a test-only TLS browser context; OS certificate trust was not changed.
Optional metadata ended in a visible failure; originals and explicit FLAC remained
usable. No provider result or artwork was invented.

The Portainer lifecycle is exercised through its real HTTP API/Docker gateway;
this is not a claim that the Portainer GUI buttons were clicked. Reproduce from
the source tree on the isolated NAS with the retained starting image:

```sh
python3 -B scripts/check_container.py ghcr.io/aberrantwolf/junk-collector:v0.1.0-alpha.2 \
  --update-from junk-collector:grouped-fix-20261019 \
  --data-parent YOUR_TEST_PARENT --port-base 29841 --proxy --inbox \
  --portainer-image portainer/portainer-ce:2.45.0 --portainer-keep-companion \
  --browser-host YOUR_NAS_HOST --private-ip YOUR_NAS_IPV4 \
  --browser-handoff /tmp/collector-browser.json
```

Run `node web/tests/container-host.mjs FILE` on the browser host for each private
handoff, then write its `.done` marker only after success. The second installation
uses the recorded digest with no `--update-from`, companion or browser flags.
The updated [Portainer runbook](../../deploy/container/README.md#portainer-managed-stacks-and-ssh-maintenance)
explains Collector-only stop and service-targeted/shared-stack versus forced redeploy.

## Native ARM64 runtime

The native GitHub ARM64 runner reported `aarch64` and completed the whole container
recovery smoke above. Separately, a native `linux/aarch64` Docker Desktop engine
anonymously pulled the recorded digest with an empty credentials directory, checked
ARM64/version/revision/source labels, initialized a fresh named volume, ran setup,
doctor and both encoders, served as UID 10001 with read-only root/capability drops,
validated HTTPS identity using the actual PEM and hostname, and exited cleanly on
SIGTERM. Its generated container/volume were removed. This does not qualify every
ARM NAS or Docker Desktop host-network deployment.

## Scope and preservation

The live Collector, operator Portainer and both tunnel containers were inspected,
not redeployed. Their IDs, image IDs and start timestamps are recorded privately
for before/after comparison. The final comparison matched all four containers
exactly (IDs, image IDs, start timestamps and running state). Generated NAS fixture
containers, Portainer volumes and data roots were absent after cleanup; remote/local
private browser handoffs were deleted. Test installations use temporary NAS home-directory
roots on Btrfs, separate ports and generated credentials. No production account,
archive, configuration or backup policy is changed.

The named old image is `grouped-fix-20261019`, not the original alpha.1 source tag.
No direct alpha.1 upgrade, system-browser trust, second-device native, public-ingress
security or power-loss claim follows from this acceptance. Independent media
backups remain optional under the accepted scope; a complete consistent local
recovery point and matching old image precede updates.
