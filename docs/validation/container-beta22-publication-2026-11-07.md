# Beta.22 publication and Fi update — 2026-11-07 JST

[Beta.22](https://github.com/AberrantWolf/junk-collector/releases/tag/v0.1.0-beta.22)
is a signed prerelease at source `6faa8cd195f45a4b2418089d3aad766005f65695`.
It includes [Opus album preparation and readiness](opus-preparation-2026-11-07.md),
[saved import review and bulk edits](github-imports-2026-11-07.md), and
[explicit username sign-in](../account-usability-delivery.md#username-sign-in--2026-11-07).

## Qualification and publication

- [Checks 42357087523](https://github.com/AberrantWolf/junk-collector/actions/runs/42357087523)
  passed at the signed candidate, using beta.21 as the baseline for the complete
  candidate diff. The beta selector chose full source coverage. The run passed
  640 Rust tests, strict workspace Clippy, architecture and documentation links,
  format, tooling, web types/build/models and all 31 browser journeys. Five
  existing Rust ignores were excluded. The preceding main run at `83d79cc` also
  passed. The redundant tag-push Checks run was cancelled after exact-source
  qualification.
- [Qualification 42357103822](https://github.com/AberrantWolf/junk-collector/actions/runs/42357103822)
  reused the exact-source Checks receipt. Both native release exports passed
  runtime checks, and AMD64 passed release-image browser acceptance. The
  validated OCI export was sealed as artifact `13569156004`, attempt 1.
- [Publication 42359379853](https://github.com/AberrantWolf/junk-collector/actions/runs/42359379853)
  published from the sealed archive. Both anonymous platform pulls passed.
  The release bundle and `release-image.json` match `SHA256SUMS`. Independent
  anonymous registry reads confirmed both manifests and their source/version
  labels. The downloaded artifact ZIP and its sealed OCI archive matched their
  recorded digests. The `preview` channel points to the beta.22 index. Stable `latest`
  remained absent.

| Object | SHA-256 |
| --- | --- |
| Multi-platform index | `f0807e6455524d216ba2fe186531fc122787cf0a1421dd23c11f84ceb1e1b427` |
| Linux AMD64 manifest | `9027c56934d4ce9a63e8758be82798e8ee9f29f9000596d2f0c97f5a5fd425c8` |
| Linux ARM64 manifest | `62822fa220410dad01ec93744708807fb247bb93187ef7f5aa08ef9c89d619d5` |
| Sealed artifact ZIP | `36c2591034c702c18066b2eef3554ad0048192ffe2acaf8117f7caab5a130bd0` |
| Sealed OCI archive | `efc61b28d02dfc517a84cc13b2fc03fe85086f8b35a2c28e0877f20af950f072` |

## Fi recovery point and live acceptance

Fi anonymously pulled the exact multi-platform index and checked the AMD64
image's source and version labels before downtime. The guarded systemd service
stopped all five stack containers with zero unfinished Jobs. The full stopped
Fi private-state snapshot and whole NAS archive-parent snapshot are read-only.
Both SQLite databases passed `quick_check` against the snapshot. The sealed pair
also records the old image, environment, Compose and systemd files, and media
authority key.

- Fi: `/var/backups/collector-fi-pairs/before-beta22-20261107-6faa8cd/fi-data`.
- FatChocobo: `/volume1/collector-recovery/before-beta22-20261107-6faa8cd`,
  snapshot UUID `7f9cfd26-73c6-1247-b518-b6c34fd25ad1`.

The service now runs the immutable beta.22 index for core and media. Both use the
qualified source with zero restarts. Core is healthy and Bridge is synced with
the existing certificate pin. The cutoff and live counts match for 123 Items, 6,811 Jobs, two users and
two grants. No Jobs were unfinished. The verification compared mail profiles,
the paused mail connection and cursor, configuration and TLS digests.
Bridge, Caddy, tunnel and seven unrelated containers retained their identities.

Public and LAN HTTPS returned healthy responses and the original ServerId. Eight
served assets on each origin match the tagged source. Both origins require an
explicit username, with no default username exposed by the feature API, and
return 401 to anonymous requests for playback routes, media bytes, FLAC export
and the admin audio setting. The separate media HTTPS endpoint reports the same
ServerId. Live acceptance covered health, identity, assets and anonymous access.
Authenticated production playback was outside this check.
