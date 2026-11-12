# Beta.28 publication and deployment — 2026-11-12 JST

[Beta.28](https://github.com/AberrantWolf/junk-collector/releases/tag/v0.1.0-beta.28)
is a signed prerelease at `151d2b3d989ac8325460945178c138a4493d55d8`.
It includes [folder and track ordering](album-import-review-2026-11-11.md#folder-and-track-ordering--2026-11-12).

[Qualification](https://github.com/AberrantWolf/junk-collector/actions/runs/43152303978)
reused the [exact-source check receipt](https://github.com/AberrantWolf/junk-collector/actions/runs/43152304277)
against the verified beta.27 baseline. Workspace Rust checks, browser smoke
and the digital purchase journey passed. The native runtime smoke checks
passed on AMD64 and ARM64, and browser smoke passed on the AMD64 release image.
The full browser matrix and extended native runtime checks were outside this
qualification. The run sealed artifact `13915530787` from attempt 1.

[Publication](https://github.com/AberrantWolf/junk-collector/actions/runs/43154147604)
reused the qualified artifact, verified anonymous pulls for both platforms, and
promoted the preview channel. The downloaded bundle and image record matched
the published checksums.

| Published object | SHA-256 |
| --- | --- |
| Multi-platform index | `7132db0a1defd5f6e8c0b360d60df49c11213c76b8b7df7cfad7401e2e0f08e1` |
| Linux AMD64 manifest | `f21961a3228b35a57fd2928de1b8bdf3c5577bf87e413350f58efcec8ecf1d28` |
| Linux ARM64 manifest | `f4ee71303f22dc55c0ae4e809b58c13476ba8723f86fd64725d7e521c09c9d05` |

## Deployment

The split-host installation upgraded from beta.27 by index digest. All five
writers stopped before sealing the paired read-only application-data and
archive-parent snapshots. The stopped databases passed integrity checks.
The collector and media image pins switched to the qualified AMD64 image.

Post-start checks verified the exact images, retained application state and
mounts, stable companions, and TLS and media identity. Bridge synced and the
enabled connection completed a fresh receipt scan. Public and LAN HTTPS served
the expected assets, including the updated import bundle, and denied anonymous
requests to protected endpoints. Folder and track ordering through publication
were exercised in the isolated browser journey.

See the [installed-stack runbook](../../deploy/container/beta-update.md).
