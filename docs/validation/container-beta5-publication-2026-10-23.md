# Beta.5 publication and NAS update — 2026-10-23

**Published, verified on `preview`, and installed on the permanent NAS.** The
operator authorized merge/publication and the release-image update. A dated
[2026-10-24 follow-up](browser-media-session-2026-10-23.md#physical-iphone-follow-up--2026-10-24)
records operator acceptance of physical iPhone lock-screen metadata.

## Frozen source and publication

- [PR #4](https://github.com/AberrantWolf/junk-collector/pull/4) merged after both
  PR/branch checks passed. Source: `0ef69628734d4a1617f9dc07af37302cf61e7735`.
- Signed ED25519 tag `v0.1.0-beta.5` was locally verified before push.
- [Successful exact-source validation](https://github.com/AberrantWolf/junk-collector/actions/runs/39966962589)
  passed repository checks and both native AMD64/ARM64 container proxy, inbox,
  recovery and isolated media/Caddy smoke jobs.
- [Successful publication](https://github.com/AberrantWolf/junk-collector/actions/runs/39971292059)
  consumed those validated OCI bytes. Its checks/build jobs were skipped; image
  publication did not recompile or rebuild. Anonymous both-platform pulls preceded
  `preview` promotion. Stable `latest` was not advanced.
- [Prerelease and installation assets](https://github.com/AberrantWolf/junk-collector/releases/tag/v0.1.0-beta.5).
  The release image record retains validation run/attempt, artifact ID/digest and
  archive SHA-256. Consumed attempt: 1; artifact: `12508399756`; archive SHA-256:
  `13e22683358334f248d96a2aafc6ada88ebfedf732d423c5b19b7e961f918548`.

## Independently verified public image

Version tag and `preview` both resolve to:

```text
ghcr.io/aberrantwolf/junk-collector@sha256:510e60d1f9521978fea5b61ab45adf4744e9b77d643098b5f9198e281b1f355a
```

| Platform | Manifest digest |
| --- | --- |
| linux/amd64 | `sha256:ea9bc88b888039baef46d514606f8ae66a0b0e68ee0d674401a2966ecf94372f` |
| linux/arm64 | `sha256:290e619c1fd73323131e840a6d1a4e49033e6b23c0c0a2d196d56a927e908ad8` |

An independent anonymous GHCR request checked index/manifest/config bytes and
their digests, both Linux platforms, version/revision/source labels and equality
with the release record. Downloaded assets match `SHA256SUMS`; bundled image
record and pinned `.env.example` match the verified reference. The media overlay,
Caddyfile, initializer and operator instructions are included.

## Permanent NAS update

- Pulled the published immutable image before maintenance; its AMD64 runtime
  config ID matches the independently checked manifest:
  `sha256:4965931a4ab99d48b3b285455228d23b809a54111eb75df6dccb226861064965`.
- Read-only doctor and before/candidate comparison passed. The retained Portainer
  definition changes only the image references for core, media and the one-shot
  media initializer. Accounts, configuration, mounts, groups, socket volumes,
  listeners and DNS/OpenWrt/Caddy settings are retained.
- Stopped media and core, observed clean core exit 0 and captured a complete
  read-only Btrfs recovery snapshot before replacement. The old exact source
  image and private rollback metadata remain available. No full-media copy/hash
  or storage conversion was required; private/catalog schemas remain 21/15.
- Replaced only core and media. Observed interruption: **4.87 seconds**. Both run
  the same verified image and report healthy startup with zero restarts. Core
  reports `junk-collector 0.1.0-beta.5`. All **18** other running containers retain
  their IDs/images/start times, including Caddy and both Cloudflare connectors.
- Core ServerId, configuration checksum and retained core TLS identity match the
  pre-update installation. The real media Unix-socket → core authority identity
  check passes. Production media remains network-disabled/read-only with CPU
  affinity `3` and 256 MiB memory; Caddy retains its 128 MiB bound. DSM's unsupported
  process-count cap remains explicitly unqualified.
- The actual served web bundle matches the accepted source SHA-256
  `da2ccf8756e03bde7a2068c1c31cd03404d3339c2ae34b5211ce960ca9750ebc`.
  A normal certificate-validated request through an explicitly selected public
  Cloudflare edge returns those same bytes (`private, no-store`, cache BYPASS).
- Direct media HTTPS passes normal CA/hostname validation and identity HTTP 200;
  unrelated administration API is 404 and missing playback grant is 401.

## Device evidence and limits

The earlier source candidate has operator-confirmed iPhone Safari cellular audio,
seek, next/previous and an audible track change while locked; see the
[original NAS/device report](direct-https-media-nas-2026-10-23.md). New
[Now Playing browser tests](browser-media-session-2026-10-23.md) pass title,
artist, album, artwork, controls, natural transitions and immediate cleanup in
real Chrome, including playlist presentation. Full local workspace/Clippy/web
checks, generated API, embedded-server rebuild and browser smoke also passed.

On 2026-10-24 the operator answered “It works!” after being asked to reload Safari
and check lock-screen metadata, optionally trying OS buttons, on the released
build. This accepts the physical metadata workflow without separately itemizing
fields or controls. A subsequent report accepts approximately
[three hours of background cellular listening without playback issues](direct-https-media-nas-2026-10-23.md#extended-cellular-background-follow-up--2026-10-24).
Lease expiry during a single long track, network transitions, later certificate
renewal and deployment rollback remain unqualified. Caddy's separate DMZ/egress policy
and the NAS kernel's process-count limitation are unchanged. Trash Amp still
needs its matching shared-client build; it was not changed by this release.
