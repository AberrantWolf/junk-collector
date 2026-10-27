# Beta.10 publication and NAS update — 2026-10-27 JST

The user authorized releasing all current changes and deploying them to the NAS.
[Beta.10](https://github.com/AberrantWolf/junk-collector/releases/tag/v0.1.0-beta.10)
is published on `preview` and installed on `fatchocobo`. It includes one-click and
automatic-ingest AccurateRip checks, collection CD defaults, live worker settings,
original provider artwork, Library import progress and the standalone capture/
naming helper follow-ups. See [release notes](../releases/v0.1.0-beta.10.md).

## Source and qualification

- Signed source and signed `v0.1.0-beta.10` tag:
  `c1cc1a4e61c055c48877b00449abb6e30943872a`. ED25519 signatures verify locally.
- [Accepted exact-source validation](https://github.com/AberrantWolf/junk-collector/actions/runs/40509921666)
  passes architecture/documentation links, formatting, testing/container/release
  tooling, workspace tests, workspace/all-target Clippy with warnings denied,
  web type checking/build/models and all **27** configured browser journeys.
  Its receipt covers all 88 changed paths since beta.9, reports `coverage=full`,
  `level=beta`, and confirms unchanged committed source. Workspace tests took
  313.7s; browser journeys took 456.8s.
- Both native AMD64/ARM64 release-image runtime/media/proxy/inbox smokes and AMD64
  browser acceptance pass before sealing. This is beta qualification, not a new
  RC/stable or optional 3,000-entry scale/physical-disc qualification.
- The standalone release identity utility was built once; final offline helper
  fixtures pass (12 naming tests and 10 capture/eject tests). The unchanged utility
  was reused after script repairs. No standalone server release binary was built
  or deployed alongside the container.

Qualification found and repaired two issues. The artwork integration assertion
read focus immediately before the editor's scheduled animation frame; it now
waits for actual focus and still fails if focus never returns. Local actual-Chrome
integration passes, including the later ingest/playback/list journeys. Rapid CD
rechecks could resolve an older verified observation because the history index
orders whole seconds and deterministic IDs. Application policy now chooses the
full immutable timestamp across matching-source observation pages. A deterministic
regression uses two timestamps one nanosecond apart, reversed ID order, two pages
and a newer unrelated fingerprint; the real ingest/restart/recheck workflow passes.
No archive/index format or original bytes are changed by this repair.

The final source's separate push run
[40509921372](https://github.com/AberrantWolf/junk-collector/actions/runs/40509921372)
stopped on `Address already in use` in the disposable numeric-origin listener
fixture. The accepted container workflow then ran its own complete source suite,
including that fixture, successfully on the same source. Earlier failed or
superseded candidates were not published. The redundant main-push source run was
cancelled only after complete exact-source qualification; required validation and
publication jobs were retained.

## Publication and independent verification

[Publication 40513206226](https://github.com/AberrantWolf/junk-collector/actions/runs/40513206226)
passes by copying accepted OCI exports without recompilation. Its check/build jobs
are intentionally skipped. It consumed validation run **40509921666**, attempt
**1**, artifact **12736853755**, ZIP digest
`sha256:ff4720ff4f35ad394c898bc258f1806c7b7a55807da464c6868b59e0fd007f3c`, and archive SHA-256
`sha256:620e6335ef6c79159e872f1c96edafcfdeb2f1198f5c78ca389e5a795f18a60e`.

Downloaded release assets pass `SHA256SUMS`. Published version/source/platform
digests agree with the separately downloaded sealed candidate image record.
Anonymous GHCR manifest/config reads verify both architectures and their source,
version and repository labels. `preview` matches beta.10; stable `latest` remains
absent (404), matching the prepublication baseline.

```text
ghcr.io/aberrantwolf/junk-collector@sha256:34b62992ef7f6de57e1e5fee00727f400900985c14758e6f2c4cca34c635813c
```

| Platform | Published manifest digest |
| --- | --- |
| linux/amd64 | `sha256:6beaf4fbdf3ff86fd9b98ee0e3b2faa5d4c03936880f6c690d0f4bf189e79029` |
| linux/arm64 | `sha256:958ba2261f5fb2c0bfd238aec4858dd18fdc8469d8dddef5b4c8f9d823b6cf28` |

## Recovery and actual NAS update

The immutable published image was pulled before stopping services. Old/candidate
doctor configuration agrees; its only expected text change explains that saved
Administration worker settings override config defaults. All worker counts and
other output agree. Expanded Compose models differ only in core/media/one-shot
initializer image references. Neither setup nor the initializer was rerun.

Both old processes exited cleanly before the complete read-only Btrfs snapshot
(timestamp UTC):

```text
/volume1/collector-recovery/before-beta10-20261027T133907Z
```

The exact old image and private adjacent recovery metadata are retained. This
captures archive, complete private state, configuration and TLS together. Only
core and media were recreated with `--no-deps --pull never`, after atomically
updating the retained Portainer definition. Interruption measured
**4.25 seconds**. Both services report beta.10 with zero restarts.
Installed AMD64 config/image ID matches the anonymous registry config:
`sha256:9b44a9534ec67bf5532be3044606e6e2bddbc79c816861c9a14ce86a3166b6e4`.

Config/TLS digests, ServerId, environment, commands, mounts and host configuration
are retained, including media isolation and the existing LAN playback bind. All
**18** unrelated running containers retain IDs, images and start times. No original
media was copied or hashed for deployment. No live-library batch was started;
new ingests use their accepted library defaults. The operator capture helper is
source tooling, not an additional NAS service or installed Trash Amp update.

## Public HTTPS acceptance and limits

Certificate-validated control HTML returns 200. Control and media identities match
the retained ServerId; media identity traverses the real Unix-socket authorization
connection. Media administration returns 404 and `/read` without a grant returns
401. All nine changed served assets match the exact signed source:

| Asset | SHA-256 |
| --- | --- |
| `library-preparation.js` | `785a6bf84fc9fd18baa6050776c0ec2f4cc3df41aba07f054c979839569d1e08` |
| `app.css` | `3b71cc5dd944c043c1cc4fd2e6c320755cce3e388d2cafededa965e207b243e4` |
| `curation.js` | `15666c42c047ddb4d597e76d3829ab5f359c380c7ac5fb6d0831ed7d1bd27ff0` |
| `library-metadata.js` | `42183f46895cdd9d6fa4571971a085e11d837315204c0b358222a2cef46e6e60` |
| `app.js` | `4919fa6a9cc05685c4173cfc5fee553ee83cf7f04191c2add37c4941d3d92fe7` |
| `administration.js` | `538fe9015b1bd1eef2fd32e6a72c7b45b4046eb8fc5c157e14304174d7fa1574` |
| `library-imports.js` | `fa237791f71bb7c2d06414bbc808629abfceab741fc2aeb6c5981540a0a8d8cc` |
| `library-activity.js` | `86c44856ceaf25424301caeb45b40c107ad12938bd31fb57dc1ad9b8b2903399` |
| `work-history.js` | `dc59fa1043747ac3f73680cf795f3e59bf8b0a054927ee42e525f10ff521c4d5` |

Authenticated production listening, a new physical iPhone session, physical CD
capture/ejection and a live NAS acquisition of the previously oversized cover were
not separately exercised. Source/browser and native release-image acceptance
remain distinct from these unrun live cases. No automatic artwork refresh, library
batch, MusicBrainz edit or Trash Amp installation is claimed. Private operation
receipts remain in `collector-beta10-update-20261027` on the NAS.
