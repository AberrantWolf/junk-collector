# Beta.24 publication — 2026-11-10 JST

[Beta.24](https://github.com/AberrantWolf/junk-collector/releases/tag/v0.1.0-beta.24)
is a signed prerelease at `4204ea324a3ec35c617daff6f2dd32a718bb9f7e`.
It includes [live playback formats](live-playback-format-2026-11-10.md),
[Amazon Japan receipt parsing](../email-purchases.md#import-and-parser-coverage),
and the documentation and browser harness updates.

The [first source run](https://github.com/AberrantWolf/junk-collector/actions/runs/42802597974)
passed the Rust tests and all browser journeys except game audio. The workflow
had passed a flag value where that fixture expected a FLAC executable path.
The runner finished the other independent journeys. The corrected commit
uses the installed binary path. Its focused game-audio browser check passed
before the [exact-source run](https://github.com/AberrantWolf/junk-collector/actions/runs/42806177630)
passed 642 Rust tests, strict Clippy, all 31 browser journeys, and the
architecture, contract-related, tooling and web checks. Five existing Rust
fixtures remained ignored.

[Container qualification](https://github.com/AberrantWolf/junk-collector/actions/runs/42806211346)
passed extended runtime checks on AMD64 and ARM64 and browser acceptance on
the AMD64 image. It sealed artifact `13760593428` from attempt 1.
[Publication](https://github.com/AberrantWolf/junk-collector/actions/runs/42809746829)
consumed that artifact, verified anonymous pulls on both platforms and
promoted the `preview` channel. The downloaded bundle and image record match
the published `SHA256SUMS`.

| Published object | SHA-256 |
| --- | --- |
| Multi-platform index | `aff46158f98ab4da9ff10fbda1c5ca442501b55aab2ce5c5eb7b1cb9668bc32a` |
| Linux AMD64 manifest | `24299fd7c764c3e32a3bfab0d89ad505a7391809e9d29e56e9a3316dcbe2c38a` |
| Linux ARM64 manifest | `9bb378621d10c3ab400e49813a379d0cc0eeb7cad92d731d1c04cc6a01af09e2` |

The application host pulled the exact AMD64 image by index digest and verified
its version and source labels. Its running beta.23 container remained healthy.
The beta.24 cutover still requires an operator-authenticated paired recovery checkpoint and post-update
acceptance. See the [installed-stack runbook](../../deploy/container/fi-beta-update.md).
