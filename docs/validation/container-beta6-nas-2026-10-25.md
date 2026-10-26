# Beta.6 NAS update — 2026-10-25 JST

The user explicitly requested updating Junk Collector on `fatchocobo`. Core and
the separate media service now run the published beta.6 image. This is a retained
installation update, separate from the [publication acceptance](container-beta6-publication-2026-10-24.md).

## Installed image and recovery

- Immutable published index:
  `ghcr.io/aberrantwolf/junk-collector@sha256:1deeff5f0b997b83a39a401a82bf763c6ffc6388acefa1a66ec6e9e014c6c2e6`.
- Verified AMD64 runtime image ID:
  `sha256:5132c598bce6b25c5b957b8dfbca16eb1a2efc3df9be353e3174ed681abac989`.
  Labels identify version `0.1.0-beta.6` and source
  `c5587138c90f5d460b371cb9e489c8eb28aedb7a`.
- Retained exact beta.5 image:
  `sha256:4965931a4ab99d48b3b285455228d23b809a54111eb75df6dccb226861064965`,
  also tagged `junk-collector:before-beta6-20261024T235154Z`.
- Complete stopped-server read-only Btrfs snapshot:
  `/volume1/collector-recovery/before-beta6-20261024T235154Z`.
  The private adjacent `-metadata` directory retains the original stack,
  deployment settings, exact identities/image IDs and the NAS-clock cutoff.
  The timestamp is UTC; the update occurred on October 25 in Japan.
- The first attempt created a snapshot but failed saving its metadata because
  the helper used a host path inside a differently mounted container directory.
  Beta.5 was immediately restarted without changing its stack or image. After
  correcting the helper, a fresh stopped-server snapshot and metadata succeeded.
  The successful update interruption measured **2.9 seconds**; that excludes the
  earlier attempt's unmeasured interruption. Both old services exited cleanly.

## Retained configuration and runtime checks

Read-only doctor output matched between old and candidate images. Parsed Compose
configuration comparison confirmed that only the core, media and one-shot media
initializer image references changed. The retained Portainer stack was updated
atomically; only core and media were recreated with `--no-deps --pull never`.
No initializer/setup was rerun. The release declares no storage/schema conversion;
no full-media copy or payload hashing was used for this update.

Core reports `junk-collector 0.1.0-beta.6`. Both services use the verified image,
remain running with **zero restarts**, and retain their environment, entrypoints,
commands, mounts and Docker host settings. Bind-mount ordering is compared as an
unordered list. Media retains network mode `none`, read-only archive/root,
CPU affinity `3` and its existing memory limit. Configuration and core TLS
certificate SHA-256 values and ServerId match the baseline. All **18** other
running containers retain their IDs, images and start times, including Caddy,
Portainer and both Cloudflare connectors.

Normal certificate-validated production HTTPS checks pass:

- Control page: HTTP 200. Served `/app.js` matches the beta.6 source build,
  SHA-256 `151132c56f5354465d65c16701d83a7939458d0398e693f6a2b5e3f7e73d05f7`.
- Direct media identity: HTTP 200 and the retained ServerId. This handler checks
  the real media Unix-socket-to-core authority connection.
- Media administration route: HTTP 404; `/read` without a grant: HTTP 401.

Private operation details and service logs remain under the NAS operator's
`collector-beta6-update-20261025` directory. Live authenticated browsing/playback
and physical iPhone acceptance of the new album warning/filter were not exercised
by this deployment check. Existing cellular-listening evidence remains dated to
its earlier build; this update does not broaden it.
