# Beta.4 image publication — 2026-10-22

**Published, verified and ready to pull using `preview`.** No NAS deployment or
restart was performed. The user explicitly authorized beta.4 publication and
promotion of the preview channel after validation.

## Frozen source and workflows

- Signed and locally verified ED25519 tag: `v0.1.0-beta.4`.
- Source: `50a3b7655c4c19fe0dbdabfe5b8e911217c6a8c3`.
- [Successful exact-candidate validation](https://github.com/AberrantWolf/junk-collector/actions/runs/39775646307).
- [Successful publication](https://github.com/AberrantWolf/junk-collector/actions/runs/39779085287).
- [Prerelease and installation assets](https://github.com/AberrantWolf/junk-collector/releases/tag/v0.1.0-beta.4).

Both runs passed source checks and both native architecture/container smoke jobs.
Publication verified anonymous pulls, promoted the verified index to preview and
attached the installation bundle, image record and checksums. This release used
the existing rebuild-on-publication workflow; the user authorized fixing that
redundancy after release completion. The in-flight release was left unchanged.

## Independently verified image

Portainer should retain:

```text
ghcr.io/aberrantwolf/junk-collector:preview
```

Preview and `v0.1.0-beta.4` both resolve to:

```text
sha256:2b9e4a051ec097ee77345aaae582a12916ff5cb99f203b83fc5381d2439d959d
```

| Platform | Manifest digest |
| --- | --- |
| linux/amd64 | `sha256:1e5efabbf9de960790b397c83601d9c866b3a28e2d98c88c0fe867727717a3a4` |
| linux/arm64 | `sha256:d2a503051c1804926068a4eb95882c941192324cdcc276eefb82d8995005e77b` |

An independent anonymous GHCR read verified both tag/index digests, Linux platform
manifests and their version/revision/source configuration labels. Additional OCI
attestation descriptors are distinct from Linux runtime platforms. Downloaded
release assets match SHA256SUMS. The bundle's `.env.example` pins the verified
immutable reference, and its image record matches the published release-image.json.
No local image execution or NAS update was performed by these follow-up checks.

## Update and qualification limits

Keep the preview tag in Portainer and update with image re-pull enabled, following
[the release notes](../releases/v0.1.0-beta.4.md). Retain the existing mounts,
configuration and credentials and a consistent recovery point. Private TCP/UDP
9845 must be reachable. The web app is included in the image; **Trash Amp still
requires its matching new client build**. Do not enable public media fallback as
an old-client workaround. New-build NAS/work-Mac deployment and managed-network
qualification remain separate from publication readiness.
