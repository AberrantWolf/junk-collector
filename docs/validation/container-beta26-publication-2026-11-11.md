# Beta.26 publication and deployment — 2026-11-11 JST

[Beta.26](https://github.com/AberrantWolf/junk-collector/releases/tag/v0.1.0-beta.26)
is a signed prerelease at `24cbdc9a655203cc0f2b2b7c3a7ceb1dc59b9495`.
It includes [album import review](album-import-review-2026-11-11.md) and
the [repository exposure changes](repository-exposure-2026-11-11.md).

[Exact-source qualification](https://github.com/AberrantWolf/junk-collector/actions/runs/43037362301)
passed the source checks, native AMD64 and ARM64 runtime checks, and the AMD64
release-image browser check. The album purchase journey verified format
selection, disc splits, keyboard reordering, and stable ordering for equal
track numbers. The local workspace test run recorded 649 passes and five
ignored cases. The local HTTPS playback journey passed with shared and separate
origins after explicitly selecting both fixture tracks.

The [initial publication attempt](https://github.com/AberrantWolf/junk-collector/actions/runs/43040605942)
uploaded the image. The anonymous pull verifier then hit a local Docker storage
conflict when it pulled the same index reference for both architectures.
The [publication retry](https://github.com/AberrantWolf/junk-collector/actions/runs/43041390328)
verified the public index and pulled each child digest anonymously. It reused
qualified artifact `13864363366` from attempt 1 and promoted the preview channel.
The downloaded image record and install bundle matched the published checksums.

| Published object | SHA-256 |
| --- | --- |
| Multi-platform index | `ab5d5c1b86c07c909269b972ef7c91b33b063d91c930235081c27d7e41c96af4` |
| Linux AMD64 manifest | `8b43f12f8fb677613e45b4b0e22c5f2f69132e4903bc4eb6a87dff866e15063d` |
| Linux ARM64 manifest | `57da846b9e5b52eec2dfe3b19f9346855aad921ccbfa15377b634d9a8e548333` |

## Deployment

The split-host installation upgraded from beta.25 with the qualified AMD64 image
by index digest. All five writers stopped before the application-data and
archive-parent read-only snapshots were sealed together. Stopped database
integrity checks passed. The rollout updated the collector and media image pins.
The recovery record retained both snapshot identities and the prior configuration
and image identity.

Post-start checks verified image identities, mounts, TLS and media identity,
retained application state, and stable companion containers. Bridge synchronized,
and a new receipt scan passed for the enabled connection. Public and LAN HTTPS
served the expected assets and denied anonymous access to protected routes. The
anonymous page omitted the alternate playback origins from its Content Security
Policy. The separate HTTPS media listener returned the expected server identity.
A signed-in Firefox reload restored the saved Music import review. Opening an
existing package showed the album file table with playback selection, disc and
track controls, and up/down buttons. The check left the saved choices intact.

See the [installed-stack runbook](../../deploy/container/beta-update.md).
