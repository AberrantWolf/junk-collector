# Beta.27 publication and deployment — 2026-11-12 JST

[Beta.27](https://github.com/AberrantWolf/junk-collector/releases/tag/v0.1.0-beta.27)
is a signed prerelease at `e65985374599c922338253c2012504e0155cadd6`.
It includes the [selected-track numbering fix](album-import-review-2026-11-11.md#selected-track-numbering--2026-11-12).

[Qualification](https://github.com/AberrantWolf/junk-collector/actions/runs/43138751971)
reused the [exact-source check receipt](https://github.com/AberrantWolf/junk-collector/actions/runs/43138751854)
against the verified beta.26 baseline. Workspace Rust checks and the smoke and
digital purchase browser journeys passed. The earlier publication verifier
change selected extended runtime checks. AMD64 and ARM64 passed, and the AMD64
release image passed browser smoke. The full browser matrix was outside this
qualification. The run sealed artifact `13909845321` from attempt 1.

[Publication](https://github.com/AberrantWolf/junk-collector/actions/runs/43141084455)
reused that artifact, verified anonymous pulls for both platforms, and promoted
the preview channel. Both downloaded release assets matched their published
checksums.

| Published object | SHA-256 |
| --- | --- |
| Multi-platform index | `7a17591583829db261cde8cbef92a1de96177970765b0d706fd7ac6396321070` |
| Linux AMD64 manifest | `b05627015757f27839902984df8214a2bf71318dc1db7439037fd50dea55b68c` |
| Linux ARM64 manifest | `0483b82e99449157ebd5db450d1b9418292038f047685beb1795e50ef1faeb41` |

## Deployment

The split-host installation upgraded from beta.26 by index digest. All five
writers stopped before the paired read-only application-data and archive-parent
snapshots were sealed. The stopped databases passed integrity checks. The
collector and media pins switched to the qualified AMD64 image.

Post-start checks verified the exact images, retained application state and
mounts, stable companion containers, and TLS and media identity. Bridge synced
and a fresh receipt scan passed for the enabled connection. Public and LAN HTTPS
served the expected assets, including the updated import bundle, and denied
anonymous requests to protected endpoints. Track numbering and resulting playback
positions were exercised in the isolated browser journey.

See the [installed-stack runbook](../../deploy/container/beta-update.md).
