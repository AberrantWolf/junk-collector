# Beta.1 publication — failed upgrade, 2026-10-21

**Do not upgrade to beta.1.** Use [beta.2](../releases/v0.1.0-beta.2.md).
The signed beta.1 tag remains unchanged.

## Published build

- Source: `765288415ee085a87ff0078f3d4bfb9b862cf9f2`.
- [CI run](https://github.com/AberrantWolf/junk-collector/actions/runs/39588226176): passed.
- Native AMD64/ARM64 container tests, anonymous pulls and bundle checksums passed.
- Index: `sha256:ed93a4357a21ef53da8431ecb843464c9c8393dca2bd33c196ff5382c285d1cb`.
- Source gates: 462 tests passed, four existing ignored; strict Clippy, formatting, architecture and web checks passed.
- Actual Chrome smoke passed. README and release docs were shortened and linked.

## Synology result

**Failed:** isolated alpha.3 → beta.1 Portainer update on DS1522+.
The public AMD64 image ran, but resuming an upload failed:

```text
storage failure: table music_search has 7 columns but 8 values were supplied
```

Startup stamped schema 14 onto alpha.3's seven-column search table.
The cached checkpoint then skipped rebuilding it. Fresh-image tests did not expose this upgrade path.

The disposable fixture was cleaned up. All 17 live container IDs, images and start times stayed unchanged.
No production update occurred. Full restore acceptance was not reached.

See [beta.2 correction and qualification](container-beta2-publication-2026-10-21.md).
