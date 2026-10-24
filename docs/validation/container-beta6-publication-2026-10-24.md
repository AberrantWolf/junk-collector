# Beta.6 publication — 2026-10-24

**Published and independently verified on `preview`.** The user authorized
committing pending code, pushing, fixing build failures and publishing once checks
passed. NAS deployment and physical iPhone acceptance of the album attention UI
remain outstanding; no live installation was changed.

## Source and validation

- Album playback attention and version preparation: `fc18a61`.
- Final source: `c5587138c90f5d460b371cb9e489c8eb28aedb7a`, including the CI fixture
  repair. The signed ED25519 commit and `v0.1.0-beta.6` tag verify locally.
- [Standalone checks](https://github.com/AberrantWolf/junk-collector/actions/runs/40099200496)
  pass, including workspace tests, Clippy, web checks/build and release compilation.
- [Exact-source container validation](https://github.com/AberrantWolf/junk-collector/actions/runs/40099202292)
  passes all repository checks, both native AMD64/ARM64 builds and actual exported
  runtime proxy/inbox/recovery/media smoke jobs, then merged OCI verification.
- Local workspace tests: **506 passed, 0 failed, 5 ignored**. Architecture/docs,
  Rust formatting, workspace Clippy with warnings denied, web type check/build,
  generated API consistency and release-tool checks pass. Embedded server/media
  binaries were rebuilt after web assets. Real Chrome 156.0.8078.47 browse,
  digital purchases and full smoke journeys pass. The new encoder fixtures pass
  all eight provider unit tests and affected Clippy after the CI repair.
- Initial [validation 40098766682](https://github.com/AberrantWolf/junk-collector/actions/runs/40098766682)
  failed when Linux rejected a just-written fake encoder with ETXTBSY. Checked-in
  executable fixtures remove that write/exec race while retaining process failure,
  cancellation and I/O-worker termination assertions. Fresh final-source validation
  passed; the failed run was not used for publication.

## Publication and independent verification

[Publication 40100943050](https://github.com/AberrantWolf/junk-collector/actions/runs/40100943050)
passed, consuming validation run **40099202292**, attempt **1**, artifact
**12563170405**. Checks/build jobs were skipped by the publication design; it copied
the validated images without recompilation. The retained validation archive SHA-256
is `b1790e6bc40e1900c23aca833310d58d29811f2085c3213d8f4512242180279e`.

[Prerelease and installation assets](https://github.com/AberrantWolf/junk-collector/releases/tag/v0.1.0-beta.6)
include the installation bundle, `release-image.json` and `SHA256SUMS`. Independently
downloaded bundle/image-record checksums match. Anonymous registry reads confirm
both platform manifests and version/source labels match the frozen source and
release image record. `preview` and the version tag resolve to:

```text
ghcr.io/aberrantwolf/junk-collector@sha256:1deeff5f0b997b83a39a401a82bf763c6ffc6388acefa1a66ec6e9e014c6c2e6
```

| Platform | Manifest digest |
| --- | --- |
| linux/amd64 | `sha256:c1362c117c0c31c3ab368eaae75d9369f804b54f0c83c2a18a3c3741a3b8c619` |
| linux/arm64 | `sha256:788624fb26d9b3dc8185b10b88670bcaaf1ee12c9e1909f0237dd7309c4d3232` |

The beta.5 version digest is unchanged. Stable `latest` remains absent (404), as
before this prerelease. [Release notes](../releases/v0.1.0-beta.6.md) describe update
and recovery requirements. This is publication acceptance, not NAS deployment or
physical iPhone acceptance of the new warning/filter.
