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

## Fi deployment

Fi upgraded from beta.23 to beta.24 using the verified AMD64 image by index
digest. The same cutover updated the tunnel connector from 2026.9.1 to
[cloudflared 2026.10.7](https://github.com/cloudflare/cloudflared/releases/tag/2026.10.7),
pinned to `sha256:4955524aeaa379e154023d5a027c250f7b35041a2e7e950cf13f144bc9b9c5ba`.
The connector's version and tunnel command ran on Fi before cutover, and the
installed Compose configuration accepted all three candidate image pins.

The existing Docker operator route on each host sealed the stopped-writer
recovery pair `before-beta24-20261110-4204ea3` before activation. The Fi
snapshot and whole NAS archive snapshot were read-only. Both stopped SQLite
databases passed integrity checks. The private recovery record includes both
snapshot UUIDs, the old configuration and image identities.

Acceptance verified the running image identities, stable container state,
retained mounts and companions, and a fresh completed Bridge sync. The
application state comparison covered accounts, grants, mail connections,
purchase suggestions, Items, Jobs, imports, the archive binding and TLS.
Public and LAN HTTPS served the release assets and rejected anonymous access to
protected routes, including live streaming. The media HTTPS origin returned
the same server identity. The retained album's revision matched the stopped
snapshot. A signed-in Firefox session played the album track with FLAC
selected and then with 128 kbps Opus selected. Receipt scanning and the
browser's playback preference were restored after acceptance.

See the [installed-stack runbook](../../deploy/container/fi-beta-update.md).
