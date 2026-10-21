# Beta.1 container publication — 2026-10-21

Release packaging is in progress for `v0.1.0-beta.1`. Source gates, image digests,
public pulls and actual isolated Synology upgrade/restore outcomes will be recorded
here as they finish. This file is not a successful-publication claim.

The intended host reports Synology DS1522+, x86_64 Linux 4.4.302+, Docker 24.0.2
linux/amd64. The existing workflow builds AMD64 and ARM64 natively. The isolated
upgrade uses public alpha.3 as its starting image and separate generated data,
ports and a disposable Portainer database. Production is not an update target.

## Source preflight

- **Passed:** architecture/docs/fixtures, formatting, strict workspace Clippy.
- **Passed:** 462 workspace tests; four existing ignored tests.
- **Passed:** web type check, API drift check, production web build and rebuilt server.
- **Passed:** actual Chrome smoke journey, including playback/seek and access revocation.
- **Passed:** four container-tool and six release-policy tests.
- **Reviewed:** version/lockfile changes, native architecture workflow, release scope and recovery steps.
- **Docs:** shortened README and beta notes; moved setup, configuration and commands to linked guides.

Docker publication and the actual-host update remain pending below.
