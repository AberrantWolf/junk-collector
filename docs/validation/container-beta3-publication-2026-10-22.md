# Beta.3 image publication — 2026-10-22

**Published and ready to pull. Actual beta.3 NAS/Portainer updating is unqualified.**
No running NAS container, live archive, configuration or deployment was changed by
this publication task. Concurrent home-network setup is separate work.

## Frozen source and build

- Signed tag: `v0.1.0-beta.3`, locally verified ED25519 Git signature.
- Source: `d80013530ff8147f8a534dba819eeaaaa813be1e`.
- [Successful Container release](https://github.com/AberrantWolf/junk-collector/actions/runs/39699939301).
- [GitHub prerelease and installation assets](https://github.com/AberrantWolf/junk-collector/releases/tag/v0.1.0-beta.3).

Exact-source checks passed architecture, container tooling/release policy, web
check/build consistency, formatting, workspace tests, strict all-target Clippy and
optimized server build. The local optimized executable also reported beta.3;
local full source/browser acceptance is recorded in the
[grouping delivery](automatic-album-grouping-2026-10-22.md).

Both native AMD64 and ARM64 jobs built and ran the real container smoke test with
proxy, inbox and inbox-cleanup checks. The publication job verified anonymous
version pulls for both platforms and matching version/revision/source labels,
promoted the verified index to `preview`, and uploaded the bundle and image record.

## Verified public image

```text
ghcr.io/aberrantwolf/junk-collector@sha256:d954aaaa9ffa5e72266330e460bdbd2cdea4aa58c5d945014653568b64589d21
```

| Platform | Manifest digest |
| --- | --- |
| linux/amd64 | `sha256:017c0098920373d54e0e79396d3e3563c73da0102d26a1654284a3b667e6b869` |
| linux/arm64 | `sha256:28b019e11e509121655fb2bfd75d749135129b35ff025f6badede051125578af` |

The published [release-image.json](https://github.com/AberrantWolf/junk-collector/releases/download/v0.1.0-beta.3/release-image.json)
binds this exact source, beta.3 tag/version, platform digests and successful
anonymous Docker pulls. A separate anonymous registry read after completion
confirmed the version index digest and both platform descriptors, and that
`preview` resolves to the same digest. Stable `latest` remains absent (HTTP 404).

The downloaded bundle and image record match the published `SHA256SUMS`.
The bundle's `.env.example` pins this immutable reference and its bundled image
record exactly matches the separately published record. No archive extraction or
local image execution was needed for those follow-up checks.

## Update and limits

Use the [beta.3 update notes](../releases/v0.1.0-beta.3.md) and existing Portainer
procedure: keep the old digest and configuration, stop Collector and take a
consistent archive/private-state/config recovery point, select the verified
reference and re-pull/update the stack. Do not rerun fresh setup or replace mounts
and credentials. Catalog schema 15 rebuilds from preserved records; automatic
grouping may update eligible existing entries after startup.

The passed beta.2 Synology drill is historical evidence, not beta.3 deployment
acceptance. Actual beta.3 NAS sign-in, library/import activity, playback and
operator recovery checks remain to be performed. No automatic discovery, active
route failover or release-driven LAN configuration is claimed.
