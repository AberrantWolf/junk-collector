# Alpha.3 container publication — 2026-10-21

**Passed:** public native AMD64/ARM64 images, anonymous pulls, version/preview
agreement, digest-pinned installation assets and an isolated actual-NAS Portainer
alpha.2 → alpha.3 update/cleanup/full-restore journey. Production stayed unchanged.
This packages the post-alpha.2 fixes, [portable full archive transfer](archive-export-2026-10-21.md)
and [explicit inbox cleanup](inbox-cleanup-2026-10-21.md). See the
[release notes](../releases/v0.1.0-alpha.3.md) for scope and schema-21 whole-point
rollback requirements.

## Frozen source and public coordinates

- Signed tag: `v0.1.0-alpha.3`; verified SSH signature.
- Source: `56b3b49939d6a9a9c924b84b6a0db90c57856a71`.
- Version image: `ghcr.io/aberrantwolf/junk-collector:v0.1.0-alpha.3`.
- Index: `sha256:d29576eeddd96ddc9391f5027c3671d0684d534dc4effb5fe244dfbf5874c1ab`.
- AMD64: `sha256:125df1b8872b559847c05cdfcb631405c6f812d1d408a37cc922011e3e72990f`.
- ARM64: `sha256:d49630028bd46767c9743472db562f40f3712d887447979dbc738438111168f4`.

The [release run](https://github.com/AberrantWolf/junk-collector/actions/runs/39529644154)
succeeded from the frozen source. Both native jobs passed initialization, guarded
configuration transport, supplemental-group access, setup, partial-upload recreation,
clean SIGTERM, actual FLAC preparation and full backup/restore. Both also refused
cleanup on the read-only bind with zero removals, then completed an explicit
write-enabled retry as normal UID 10001 while retaining downloadable archive originals.

Registry verification downloaded both platforms without credentials, checked their
source/version labels and manifest digests, then promoted **preview**. Stable
**latest** was not advanced. Independent preview inspection matched the index.
The [published release](https://github.com/AberrantWolf/junk-collector/releases/tag/v0.1.0-alpha.3)
contains `junk-collector-container.tar.gz`, `release-image.json` and `SHA256SUMS`.
Downloaded asset checksums matched; the bundle pins this index and retains default
read-only inbox mounts plus the explicit write opt-in for both server and CLI.

## Source and browser qualification

The official current-stable Rust **1.99.1** was installed and qualified. Development,
CI, Docker and README pins agree; no shared-library algorithms or Git pins changed.
Architecture/documentation/fixture checks, format, OpenAPI/TypeScript drift check,
TypeScript check, deterministic web build, container/release tooling and strict
all-target Clippy passed. Workspace tests: **454 passed, zero failures, four existing
opt-in qualification tests ignored**. The optimized executable reports
`0.1.0-alpha.3`; portable export/import commands are present. Linux CI repeated the
required source gates successfully.

The rebuilt server passed actual Chrome 155.0.8059.85 smoke: setup, login/expiry,
virtual WebAuthn registration/login, role denial, inventory/artwork, interrupted bulk
upload/grouping, provider concurrency/cancellation/reopen, FLAC preparation and
actual playback/seek/advance/revocation. Collection lists passed append/repeats,
reorder/conflict/recovery/shuffle and mobile/keyboard use. The affected cleanup
browser/recovery journey passed separately; its retained-source/retry/mobile/replay
checks are in the linked acceptance record. These are temporary fixtures, not new
physical-device claims.

## Isolated actual-NAS Portainer acceptance

A Synology DS1522+ Btrfs installation with Docker 24.0.2 and Compose 2.20.1 used
newly generated user-home data, five loopback ports and a disposable **Portainer
2.45.0** database. No operator Portainer credentials/database or real media were used.
The starting public alpha.2 index was
`sha256:f86f46d2c8870f8d647fc9225ce2cfe7c4319521a29c89897b419abb77629433`.

The new Portainer anonymously pulled alpha.3 through its Docker gateway. Actual
stack re-pull/redeploy changed the image and container while retaining the original
account/session, server identity, exact configuration, TLS certificate, committed
originals and a partial upload's durable offset. The upload resumed and published;
original downloads matched. Actual FLAC preparation and audio range/download checks
passed. Unconfigured optional metadata failed visibly without blocking preserved
originals, FLAC or later cleanup.

Cleanup first failed on the read-only mount with no removals. Explicit write-enabled
recreation/retry removed the imported source package as UID 10001; preserved
originals remained downloadable. Full stopped-server backup/validation, restore into
empty roots, doctor and redeploy retained sessions/identity/TLS, original bytes and
prepared audio. Clean SIGTERM exits took 0.24–0.26 seconds in this bounded fixture.
This tests an upgrade and new-image restore, not image-only downgrade to alpha.2.

Generated stacks/containers/Portainer volume, temporary credentials, data roots and
source transfers were removed. Before/after checks found **all 17 live container
identities, image IDs and start times unchanged**. Production accounts, configuration,
media, tunnels and OS trust were untouched. Pulled image cache entries remain available.

## Evidence limits

Prior physical Safari, native audio, public-origin and format evidence is reused
where unchanged. This does not add large-library throughput, all-format verification,
gapless/bit-perfect, hostile external-writer or power-loss claims. Cleanup requires
finished external copying/editing and explicit writable mounts/host rights; external
quarantine inputs are outside archive/state backup. Independent media backup and
cloud storage remain optional operator choices. The authoritative queue retains
conditional device/reference and future feature work.
