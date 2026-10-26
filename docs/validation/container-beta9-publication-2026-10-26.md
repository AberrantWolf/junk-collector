# Beta.9 publication and NAS update — 2026-10-26 JST

The user authorized pushing/building a new beta and updating the NAS once it was
packaged. Beta.9 is published on `preview` and installed on `fatchocobo`. The
uncommitted raw-CD helper follow-up and its tests were preserved in the original
checkout and excluded from the isolated release candidate.

## Source and qualification

- Signed source and signed `v0.1.0-beta.9` tag:
  `89d66608e70a5ebfc256e84fe73749e0d1163304`. Both ED25519 signatures verify locally.
- [Full source checks](https://github.com/AberrantWolf/junk-collector/actions/runs/40318188062)
  pass architecture/format, tooling/provenance, workspace tests, warnings-denied
  workspace/all-target Clippy, web type checking/build, model tests and all 26
  named browser journeys. The authenticated receipt reports full coverage and
  unchanged committed source. Workspace tests took 301.54s; browser journeys
  419.09s. The optional 3,000-entry scale run is not claimed.
- [Exact-source candidate validation](https://github.com/AberrantWolf/junk-collector/actions/runs/40318228521)
  passes both native AMD64/ARM64 runtime/media smokes and AMD64 browser acceptance
  against the actual release image. Source qualification was reused, not executed
  twice. All required jobs completed successfully before sealing.
- Official current-stable Rust remained 1.99.1, matching the toolchain/CI/Docker
  pins. No shared-library or toolchain update was needed.

Earlier source candidate 40312662293 failed in remote playback: Media Session
metadata became visible before HTML audio duration was ready after restarting a
stopped source. The fixture now waits for actual playback and finite positive
duration before its existing seeks. Separate-origin, one-origin and expired-grant
recovery cases pass locally in actual Chrome; the final full Linux suite passes.

Ubuntu browser-package mirror downloads delayed two image-validation attempts.
A fresh attempt reached the browser but returned to login with `Failed to fetch`
after reload. The original diagnostics did not establish the underlying network
error; the fixture now pins its exact disposable HTTPS certificate across browser
requests and records failed requests. Three independent local image smokes and
the final native release-image smoke pass. No production TLS/exposure policy was
changed. Failed/cancelled candidates were not used for publication. The duplicate
tag-triggered source run was cancelled only after successful full exact-SHA source
and container qualification; required validation/publication jobs were retained.

## Publication and independent verification

[Publication 40320083277](https://github.com/AberrantWolf/junk-collector/actions/runs/40320083277)
passes by copying the accepted OCI exports without compilation. Its source-check
and build jobs are intentionally skipped. It consumed validation run
**40318228521**, attempt **1**, artifact
**12648407199**, ZIP digest
`sha256:16a149b0e365be4e11d52c8c0a4a8055bc1cc968cdf0b27e1378f5fd34944d6f`, and archive SHA-256
`sha256:f6a9c8c7126b1b982ec6a596c73603c513ff4abc48ea3548cad59c723108c649`.

[Prerelease and installation assets](https://github.com/AberrantWolf/junk-collector/releases/tag/v0.1.0-beta.9)
have independently verified checksums. Anonymous registry reads confirm both
platforms and their version/source labels match the signed source and image record.
`preview` matches beta.9; beta.8 is unchanged and stable `latest` remains absent
(404), as before this prerelease.

```text
ghcr.io/aberrantwolf/junk-collector@sha256:a14a4609f3e9ddbf3a3d210c0bcb9e5df545add24bf95975b45a2198ffef2711
```

| Platform | Manifest digest |
| --- | --- |
| linux/amd64 | `sha256:f9978e5b1b5dc1e874bac52841d31b7357faa0fa6dbd92ef6712b72b640b5b86` |
| linux/arm64 | `sha256:f9fcc48ea3a507b623bf9925166d4f2cf30305603fc13601587427e6965c3396` |

## Recovery and actual NAS update

The published immutable image was pulled before stopping services. Its installed
config digest matches the published AMD64 manifest. Read-only old/candidate doctor
output agrees. Expanded Compose models differ only in the core/media/one-shot
media initializer image references; no initializer or setup was rerun.

Both old processes exited cleanly before the complete read-only Btrfs snapshot
(its timestamp is UTC):

```text
/volume1/collector-recovery/before-beta9-20261026T121130Z
```

The exact old image and private adjacent recovery metadata are retained. Only
core and media were recreated, using `--no-deps --pull never`, after atomically
updating the retained Portainer definition. Interruption measured
**3.94 seconds**. Both services report beta.9 and zero restarts.
Installed AMD64 config/image ID:
`sha256:41034209d475b77884fa2abe09da13e95c59c6a014dd4abd143a8c0d6dd5e934`.

Config/TLS digests, ServerId, environment, commands, mounts and host configuration
are retained, including media network isolation. All **18** other running
containers retain IDs, images and start times, including Caddy and the tunnels.
No original media was copied or hashed for this update.

## Public HTTPS acceptance and limits

Certificate-validated control HTML returns 200. Control and direct-media identities
agree with the retained ServerId; media's identity handler traverses its real
Unix-socket authorization connection. Media administration returns 404 and `/read`
without a grant returns 401. Served assets match the frozen source:

| Asset | SHA-256 |
| --- | --- |
| `library-preparation.js` | `9731daec847756cc50f55c6bd16250410dbd1b8120f87f297b1c43e83121e3f5` |
| `app.css` | `0e4035a715cb47a404ae74b2623d28f27f5f8abbc1dc516bbe2bf2d718f18efb` |
| `curation.js` | `7b9834e0404583a59956a497e2033e795578f571493a6bfbca8904439fee4477` |
| `library-metadata.js` | `be88ebfb7fb3a8791b608001724275a2800f0e331afbc5cbc909804339b47e9e` |
| `app.js` | `1c834e26d2b2adf04952982d476df28650944ee81afb19aa5857ce4fab83beea` |

The source/local real-Chrome playback and seek journeys pass. Authenticated
production listening and a new physical iPhone session were not separately
exercised; the available controlled Chrome session could not reach the production
origin, while the independent certificate-validated HTTPS probes passed. No new
MusicBrainz edit or Trash Amp installation is claimed. Refresh the browser for
the deployed metadata controls and copy. Private operation logs remain in
`collector-beta9-update-20261026` on the NAS.
