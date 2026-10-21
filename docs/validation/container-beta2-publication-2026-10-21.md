# Beta.2 correction and publication — 2026-10-21

**Published and accepted on Synology DS1522+.** Beta.2 corrects the
[beta.1 upgrade failure](container-beta1-publication-2026-10-21.md).

## Repair

- Recreate obsolete catalog schemas before rebuilding projections from the archive.
- Detect beta.1's bad schema-14 stamp by checking the music-search column layout.
- Retain the previous catalog for diagnosis. Leave private state and archive records intact.
- Rebuild from metadata without hashing media payloads.

The regression covers both alpha.3's schema 13 and beta.1's bad schema 14.
It preserves old checkpoint/trigger names so missing-object recovery cannot hide the bug.
The test fails on beta.1 code and passes with the repair.
Existing checkpoint recovery/reuse tests also pass.

## Source review

Reviewed the schema check, cache replacement, interrupted-rebuild path and private-state boundary.
Passed: **463 workspace tests**, four existing ignored tests, focused checkpoint tests,
strict Clippy, web check/build, container tooling, architecture/docs/fixtures and formatting.
The rebuilt server reports `0.1.0-beta.2`.
No web assets changed; beta.1's actual Chrome acceptance is reused.

## Frozen source

- Signed tag: `v0.1.0-beta.2`.
- Source: `32992ca16c63c45177c07ebcce06fe2b06462977`.
- [Release build](https://github.com/AberrantWolf/junk-collector/actions/runs/39592350571).

## Public images

Both native image tests and anonymous pulls passed. Bundle checksums, source labels
and the pinned `.env.example` agree. `preview` matches beta.2; stable `latest` remains absent.
The published Compose/config files match the frozen source used by the NAS drill.

```text
ghcr.io/aberrantwolf/junk-collector@sha256:1eff7c4c2710e6e47087083a5be8e763415795b0688fd17a8fd2a6f215faa508
```

Platform digests: [release-image.json](https://github.com/AberrantWolf/junk-collector/releases/download/v0.1.0-beta.2/release-image.json).

## Isolated Synology acceptance

Host: **DS1522+**, x86_64 Linux 4.4.302+, Docker 24.0.2, Compose 2.20.1.
The native AMD64 image reports beta.2 and the frozen source revision above.

| Check | Result |
| --- | --- |
| Portainer 2.45 anonymous pull and re-pull/redeploy | Passed, alpha.3 → beta.2 |
| Retained login, session, config, server ID and TLS identity | Passed |
| Interrupted-upload resume and original-byte comparisons | Passed |
| Real FLAC preparation and HTTP range delivery | Passed |
| Read-only inbox cleanup refusal, then explicit writable cleanup | Passed; archived originals stayed downloadable |
| Sealed full backup, validation and restore into empty roots | Passed; login, originals and prepared-audio digest rechecked |
| Clean SIGTERM stops | Passed, 0.30–0.35 seconds in this fixture |
| Production isolation | All 17 live container IDs, images and start times unchanged |

Generated containers, Portainer volume, data and staging source were removed.
No production update occurred. Unconfigured optional metadata stayed visibly failed;
original preservation and explicit FLAC remained usable.

## Limits

This qualifies the named isolated Linux/Synology path. It does not add physical-device,
public-ingress or arbitrary-power-loss claims. See the [capability checklist](../release-readiness.md).
