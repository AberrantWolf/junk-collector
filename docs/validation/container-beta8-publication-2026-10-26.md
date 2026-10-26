# Beta.8 publication and NAS update — 2026-10-26 JST

The user authorized committing the MusicBrainz fixes, publishing a new release
and deploying it to the NAS. Beta.8 is published on `preview` and installed on
`fatchocobo`. Unrelated raw-CD helper and beta.6 documentation edits remain
uncommitted and were excluded from this release.

## Source and publication

- Signed source commit: `252ee2bc9ed32fc65bfa03a195811eb5a6408dc2`.
  The commit and signed `v0.1.0-beta.8` tag have locally verified ED25519 signatures.
- [Standalone checks](https://github.com/AberrantWolf/junk-collector/actions/runs/40254253153)
  and [exact-source validation](https://github.com/AberrantWolf/junk-collector/actions/runs/40254283704)
  passed. This includes workspace tests/Clippy, architecture/format/web/served
  asset checks, release executables and native AMD64/ARM64 OCI builds with actual
  proxy/import/recovery/media runtime smokes.
- [Publication](https://github.com/AberrantWolf/junk-collector/actions/runs/40256285448)
  passed by copying those validated exports without rebuilding. Validation run
  `40254283704`, attempt 1, artifact `12622024919`, artifact digest
  `sha256:012fe2e56f6f3d4ac206a57f041a3ba6ea356255314407906c885a3b930e7bb2`,
  archive SHA-256 `20a4b726cd320ca0222191dc50949e6caa78075b562b3e806d93d94c1a768af6`.
- [Release and installation assets](https://github.com/AberrantWolf/junk-collector/releases/tag/v0.1.0-beta.8)
  have independently verified checksums. Anonymous registry reads confirm both
  manifests and version/source labels match `release-image.json`.

Published immutable reference:

```text
ghcr.io/aberrantwolf/junk-collector@sha256:fc28083fa71e7f6afc0e0129b2236422913a32a006f2799db6bf6c2061f9ee64
```

| Platform | Manifest digest |
| --- | --- |
| linux/amd64 | `sha256:78e651f313f0d9e4c1ed29e1e8343be8be0e0a3699ee20a13972a2cdcea2ed5d` |
| linux/arm64 | `sha256:8c95683f0329b60eee28cb45ca320ccda6d8128a5d1b906a323b5ba83f53b51e` |

`preview` matches beta.8. The beta.7 digest is unchanged; stable `latest` remains
absent (404), as before publication. The official current-stable Rust manifest
still selects the pinned Rust 1.99.1; no shared-library/toolchain change was needed.

## Recovery and deployment

The NAS pulled the immutable published image before stopping services. Old and
candidate read-only doctor output matched. Expanded Compose models differed only
in the core/media/one-shot media initializer image references. Only core and media
were recreated with `--no-deps --pull never`; no initializer or setup was rerun.

Both old processes stopped cleanly before the complete read-only Btrfs snapshot:

```text
/volume1/collector-recovery/before-beta8-20261026T015432Z
```

Adjacent private recovery metadata and a retained old-image tag preserve rollback
coordinates. The retained Portainer definition was updated atomically. Interruption
measured **4.67 seconds**. Both services report beta.8's image, with zero restarts.
The installed AMD64 image ID is
`sha256:24f78e9ed01d54612162e8dc64fef23f44fe7ad1da43f63803e2cc6bbdcff82c`.
Config/TLS digests, ServerId, environment, commands, mounts and Docker host
configuration are retained, including media network isolation. All **18** other
running containers retain their IDs, images and start times, including the HTTPS
proxy and tunnels. No original media was copied or hashed for this update.

## Public HTTPS verification and limits

Certificate-validated checks passed after deployment:

- Control page: HTTP 200. Served `library-preparation.js`, `curation.js` and
  `app.css` match the frozen source, including the submission control and styles.
  Their SHA-256 values are respectively
  `53e04554f4dde2fc9ed58ab4adb684b6b016cd28497463f875f69686e32c20de`,
  `304883ff3d6e774859400f4978e1e44ef17a3f714b881d8797e015d037151548`,
  `03204940a791ad691bfdc3395acfc5a41d1b717bcba8c8bb2ffd60159abe3d35`.
- Control and direct media identities agree with the retained ServerId; media's
  real Unix-socket authorization connection reaches the updated core.
- Media administration route: HTTP 404; `/read` without a grant: HTTP 401.

Private operator details remain in the NAS `collector-beta8-update-20261026`
directory. Source Chrome acceptance covers release-ID lookup, track names,
recorded TOC/release link arguments, mobile layout, external-link navigation and
button suppression after refreshed registration; see
[feature acceptance](musicbrainz-disc-submission-2026-10-26.md). No live MusicBrainz
edit, authenticated production listening session or new physical iPhone acceptance
is claimed. Refresh the browser to load the deployed UI; refresh MusicBrainz
metadata after submitting a Disc ID.
