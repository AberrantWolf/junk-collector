# Fi email intake deployment — 2026-10-31 JST

The user requested updating the already migrated Fi server and adding Proton
Bridge for local account setup. Fi is the application host; FatChocobo supplies
the retained NFS archive. No new migration, registry publication, release tag or
mailbox login was performed.

## Exact installed bytes and checks

Collector source: `83a9e0d68bb1d99b1ec2d5a35ecc5f23de1c3e33`, the merged
`codex/email-purchases` branch. The existing Dockerfile built optimized server and
media executables once on Fi; those same image bytes were tested and installed.
Its Cargo version remains beta.14; the source revision and local image distinguish
it from the published beta.14 image.

```text
Collector local tag: junk-collector:83a9e0d
Collector image ID: sha256:0a38228c1ce62a5e77650524a6d5676f298ea32a347c36528d33d072425cbc53
Bridge local tag: junk-collector-proton-bridge:3.18.0
Bridge image ID: sha256:ab1f40ca44654adbfdf343f34dfbee8f8255c6373b987b8b68636ae0460845be
Bridge package: protonmail-bridge 3.18.0-1
```

The retained environment pins both image IDs. Bridge's build checks its pinned
official package SHA-256 and DEB signature. Both passed. The build emitted a
generic secret-name warning for `PASSWORD_STORE_DIR`; that environment value is
only the password-store directory path.

Source workspace/Clippy/web/broad-browser evidence is reused from the unchanged
[branch delivery and final review](email-purchases-2026-10-30.md); it was not rerun
as new full-source qualification. Fresh deployment checks passed:

- Architecture/document links and all eight offline mail parser/IMAP tests.
- Shipped container initialize/config transport, supplemental inbox group access,
  partial-upload distinct-image upgrade from published beta.14, clean SIGTERM,
  FLAC, full disposable backup/restore, read-only cleanup refusal and explicit
  writable cleanup. No production collection was used by these mutation tests.
- Shipped isolated-media Compose profile: read-only archive/root/authority,
  denied writes/renames/unlinks, retained fixture digests, parent/nested mount
  refusal and Caddy fixture TLS/Unix-upstream/routing/log behavior.
- Shared short browser smoke against a disposable installation of the exact
  release image on Fi: login, artwork/search/detail, administration persistence,
  Viewer denial, mobile layout and served-asset byte equality. Passed in 3.15 s;
  desktop/mobile screenshots were inspected. Chromium ran outside macOS seatbelt.
- Actual Bridge CLI startup, persistent GPG/pass initialization and repeated
  encrypt/decrypt startup probes; Bridge stores its encrypted vault key using
  the `pass` backend. The optional Secret Service probe warns because dbus-launch
  is absent, then uses pass. GUI startup is not a supported container workflow.
- Bridge CLI through the installed operator wrapper, followed by clean daemon
  restart; package version checked with dpkg-query. Telemetry and auto-updates
  disabled with their confirmation prompts. CLI reports a newer upstream
  version is available; the reviewed pinned 3.18.0 package remains installed.

The first container fixture failed to start because its default private-media
listener collided with production's port 9845. Its bounded teardown completed;
only the disposable harness copy was changed to disable local playback, and the
full selected container sequence passed on rerun. Production was not changed to
accommodate it. One deployment override initially had a duplicate restart key;
Compose refused it before starting services. The key was corrected, then startup
and acceptance completed. Neither failed attempt counts as passed evidence.

## Paired rollback point

The guarded Fi systemd unit was stopped; all four previous services exited 0.
No user jobs were unfinished. With all writers stopped, archive/local writeback
was synced and matching read-only Btrfs snapshots were taken:

```text
Fi: /var/backups/collector-fi-pairs/before-email-20261031-83a9e0d/fi-data
NAS: /volume1/collector-recovery/before-email-20261031-83a9e0d
```

The Fi snapshot covers the complete `/srv/junk-collector` subvolume, including
state/imports, config/TLS and Caddy persistent directories; no nested subvolumes
were present. The NAS snapshot covers `/volume1/junk-collector`, including the
entire retained archive. No full-media scan, hash or duplicate copy was required.
Both stopped databases passed quick_check through writable scratch copies of
their snapshot DB/WAL files; snapshots stayed read-only.

The Fi pair directory also retains old Compose files, environment, systemd unit,
fstab, image/container records, the authority service key with numeric ownership,
cutoff facts and the paired completion receipt. The old image is retained as
`junk-collector:before-email-83a9e0d`. Restore both matching data members and old
configuration with all writers stopped; never downgrade only the image against
the newer schema. Follow [paired recovery](../../deploy/container/remote-archive.md#coordinated-recovery-points-and-rollback).
This is a consistent rollback point, not a new split-installation restoration
drill. The separate inbox source was mounted read-only and was not cleaned.

## Live acceptance and next action

Collector and media use the same qualified new image. Core is healthy; core,
media and Bridge have zero automatic restarts. The retained systemd unit includes
the mail override and starts Bridge detached before its core/media foreground
group, so entering the Bridge CLI does not stop Collector. All services use
restart=no under systemd's guarded lifecycle.

All config/TLS file hashes, 79 catalog items, one user, two grants and 2,585 jobs
were retained. Private schema is now 22; catalog schema remains 15. There were no
unfinished jobs before or after, so active-job recovery is not newly qualified.
The existing Caddy and Cloudflare container identities were retained across the
stop/start, and all seven unrelated containers retained their IDs/start times.
Management health and public trusted HTTPS return 200. The served purchase-inbox
asset matches the checked source bytes. Existing listener exposure configuration
was retained; unauthenticated profile API access returns 401.

Bridge binds only host-loopback 1143/1025; connections from the Mac to both Fi LAN
ports fail. It has a read-only root, dropped capabilities, no-new-privileges,
bounded tmpfs and UID/GID 10001. Persistent mail data is on Fi under
`/srv/junk-collector/mail`, inside the local data snapshot boundary. The initial
mode-0600 profile file is `[]`, so no watcher account is advertised or enabled.

The [Fi operator commands](../../deploy/proton-bridge/README.md#current-fi-installation--2026-10-31)
open the private Bridge CLI, list enabled Collector account IDs and provision the
certificate-pinned connector using hidden IMAP credential prompts. No Proton
password, 2FA code, real local IMAP credential or mail content was used or stored
in this repository/chat. Actual login, logged-in restart persistence, unread-state
checks, archived-folder history and live retailer-template review remain the
operator's next acceptance steps. New authentication/privacy settings and WAN
mail ports were not introduced.
