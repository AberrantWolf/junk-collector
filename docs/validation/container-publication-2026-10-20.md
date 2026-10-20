# HL-07D container publication acceptance — 2026-10-20

## Source and workflow

The signed `v0.1.0-alpha.2` tag identifies
`0f0e0f52fedc239ecdb180436583e69b7ce13a75`.
The [GitHub prerelease](https://github.com/AberrantWolf/junk-collector/releases/tag/v0.1.0-alpha.2)
and [release workflow](https://github.com/AberrantWolf/junk-collector/actions/runs/39344034491)
were started under the user's explicit publication assignment.
Architecture/link checks, container-tools/release policy tests, Rust formatting,
workspace tests, warning-denied all-target Clippy, web checks/build, committed asset
comparison and the release server build passed on GitHub. The native container
build and shipped proxy/inbox install/update/full-backup/restore smoke passed.
The earlier [preparation record](container-release-preparation-2026-10-20.md) retains
its dated, pre-publication state.

**In progress:** multi-platform image publication, anonymous registry acceptance,
channel/bundle verification and public-image Portainer acceptance. No public digest
is recorded until those operations finish.

## Isolated Portainer acceptance mechanism

`scripts/check_container.py --portainer-image …` reuses the existing real application
journey. `scripts/portainer_qualification.py` supplies only the service lifecycle:
a disposable Portainer CE 2.45.0 with its own generated database volume, temporary
admin account, loopback listener and Docker-socket environment. It never opens the
operator Portainer database or registry credentials. Fixture project names include
a random suffix; cleanup targets only that generated stack/container/volume/root.

The stack is rendered from the shipped Compose definition and inbox override,
retaining UID/GID, read-only root, capabilities, host networking, bind roots and
supplemental inbox group. One-shot initialization/setup/backup/restore still uses
the same CLI image as the selected server. Public images are pulled through the
actual Portainer Docker gateway without registry credentials. A Collector-only
stack exercises Portainer 2.45's `RepullImageAndRedeploy`. With
`--portainer-keep-companion`, an unchanged, networkless `sleep` control container
must retain its container ID and start time: the adapter pulls Collector's image
and updates its service with whole-stack forced redeployment disabled. This
qualifies service targeting, not a simulated tunnel. Both modes wait for the
asynchronous deployment result and require a replaced Collector with the selected
immutable image ID.
See [Portainer's API explanation](https://docs.portainer.io/api/examples) and
[the versioned update implementation](https://github.com/portainer/portainer/blob/2.45.0/api/http/handler/stacks/stack_update.go).

A preflight with the retained `grouped-fix-20261019` and `hl05a-qualification` local
images passed, including retention of the companion through update and restore.
This establishes the test adapter, not public alpha.2 acceptance.
Self-review caught that Portainer stack stop removes containers; maintenance now
stops only Collector via its container API and checks its retained clean exit.
The ordinary Compose path also passed after this shared test refactor.

The journey preserves an already committed package and a separate partial upload
through a distinct-image update, retaining original login session/account,
ServerId, exact config and TLS certificate. A complete old-image CLI backup is
validated before update. It compares every original byte, resumes the recorded
upload offset, checks one publication, group-readable inbox import, explicit FLAC
preparation and ranged media, then validates/restores a full backup into empty
roots while the source server is stopped. The unknown companion file is retained.
Unavailable optional metadata remains a visible failure without blocking originals
or explicit audio preparation.

## Pending published-image results

Public version/channel digests, both-platform anonymous pulls, downloaded bundle
checks and actual public-image install/update/browser/restore results will be added
after execution. The local preflight is not a substitute for these results.

## Scope and preservation

The live Collector, operator Portainer and both tunnel containers were inspected,
not redeployed. Their IDs, image IDs and start timestamps are recorded privately
for before/after comparison. Test installations use temporary NAS home-directory
roots on Btrfs, separate ports and generated credentials. No production account,
archive, configuration or backup policy is changed.

The named old image is `grouped-fix-20261019`, not the original alpha.1 source tag.
No direct alpha.1 upgrade, system-browser trust, second-device native, public-ingress
security or power-loss claim follows from this acceptance. Independent media
backups remain optional under the accepted scope; a complete consistent local
recovery point and matching old image precede updates.
