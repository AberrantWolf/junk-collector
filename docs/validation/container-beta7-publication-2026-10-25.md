# Beta.7 publication and NAS update — 2026-10-25 JST

The user authorized committing/pushing the album repair and security updates and
deploying them. Beta.7 is published on `preview` and installed on `fatchocobo`.

## Source and publication

- Security commits: `e0ef5c8` and `55ad45d`; album repair and beta.7 preparation:
  `5cc417b376f8a5481401853e1b130f3de3862197`. The final commit and annotated
  `v0.1.0-beta.7` tag have locally verified ED25519 signatures.
- [Standalone checks](https://github.com/AberrantWolf/junk-collector/actions/runs/40198330813)
  and [exact-source validation](https://github.com/AberrantWolf/junk-collector/actions/runs/40198347759)
  passed. Validation includes full workspace tests/Clippy, web checks and served
  asset consistency, release builds, native AMD64/ARM64 OCI builds and actual
  proxy/import/recovery/media runtime smoke tests.
- [Publication](https://github.com/AberrantWolf/junk-collector/actions/runs/40200566594)
  passed by copying those validated images, without rebuilding. Validation run
  `40198347759`, attempt 1, artifact `12601238602`, archive SHA-256
  `c2d3f0e6dcd111123f3f91ee9209ebcab31acebe794c53686b89cf0d4e9d62c3`.
- [Release and installation assets](https://github.com/AberrantWolf/junk-collector/releases/tag/v0.1.0-beta.7)
  have independently verified checksums. Anonymous registry reads confirm both
  platform manifests and version/source labels agree with `release-image.json`.

Published immutable reference:

```text
ghcr.io/aberrantwolf/junk-collector@sha256:30d2881fdd5470250d8556f4bc2bb796bdeb6071ef87cfda7fc7e83307a7bcc4
```

| Platform | Manifest digest |
| --- | --- |
| linux/amd64 | `sha256:9f49616e70138dfe38ceb4916785b9dcb61710c7852ec7eb47cb292cf7f19efa` |
| linux/arm64 | `sha256:1bcdf81dc5c2bcfd4b1e7d8df3bc5eff0be054a8533819c31d753d10b18d2af8` |

`preview` matches beta.7. The beta.6 version digest is unchanged; stable `latest`
remains absent (404), as before publication.

## Recovery and retained installation

Before replacing services, both old processes stopped cleanly. A complete
read-only Btrfs recovery snapshot was taken at
`/volume1/collector-recovery/before-beta7-20261025T140956Z` (UTC timestamp), with
private adjacent recovery metadata and the exact previous image retained.
The new AMD64 image ID is
`sha256:53bd79c9ea6014c15d944abb5648036fc41c02e5ee0b38d3b3ea529dcc4b7fd8`.

Read-only doctor output matched. Expanded Compose models differed only in the
core/media/one-shot media initializer image references. The retained Portainer
stack was updated atomically; only core and media were recreated, with
`--no-deps --pull never`. No initializer/setup was rerun. Interruption measured
**4.15 seconds**. Config/TLS digests, ServerId, environment, commands, mounts and
Docker host configuration are retained, including media's network isolation.
Both new services run with zero restarts. All **18** other running containers
retain their IDs, images and start times, including Caddy and the tunnels.

New metadata revisions use resolution policy 4. Rollback requires the complete
pre-update snapshot with its matching old image, not an image-only downgrade;
see [release/update notes](../releases/v0.1.0-beta.7.md).

## Actual duplicate cleanup

Read-only archive metadata inspection distinguishes live inventory from retained
removed/grouped-source history. Before update, 11 live music entries had 44 active
candidate identities, including **17 redundant versions**. All 11 now use policy
4, with **27 active choices and zero equivalent version duplicates**. There are
still 33 retained metadata entry histories. All **131** pre-existing metadata
revision files have unchanged SHA-256 values; 11 new metadata revisions bring
the total to 142. Verification read metadata JSON only, not audio payloads.
The source regressions additionally cover incremental disc fit, explicit manual
selection, semantic differences, reindex/idempotence, queued-work deferral and
curator permission fencing; see [delivery acceptance](edition-fit-2026-10-25.md).

## Production HTTPS verification and limits

Normal certificate-validated requests passed:

- Control page: HTTP 200. The served metadata workspace bundle matches the frozen
  source: SHA-256 `71032a3d1781f3e1d43cd67085495f6bcb83a7e898240457f7ed3b308d8b7f0d`.
- Control and direct media identity agree with the retained ServerId. The media
  handler exercises its real Unix-socket authority connection to the new core.
- Media administration route: HTTP 404; `/read` without a grant: HTTP 401.

Private deployment details/logs remain in the NAS operator's
`collector-beta7-update-20261025` directory. The source's real Chrome metadata
workspace and complete smoke/playback journeys passed before publication; no new
physical iPhone or authenticated production listening acceptance is claimed.
Unrelated pending raw-CD helper and beta.6 documentation changes were not included
in these commits.
