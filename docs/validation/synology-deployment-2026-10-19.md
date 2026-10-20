# Synology deployment qualification — 2026-10-19

Partial HL-05 acceptance, performed interactively by the operator on a Synology
DS1522+, DSM 7.4.1-90080. This is an actual-host smoke and persistence check,
not completed failure/recovery, power-loss, scale or public-ingress qualification.

## Installation

- Operator built the repository Dockerfile for `linux/amd64`, tagged
  `junk-collector:synology-test`, transferred it and loaded it on the NAS.
  An immutable image ID/source revision has not yet been collected.
- Non-root image user UID 10001; direct NAS volume bind mounts beneath
  `/volume1/junk-collector-test`: `config` to `/config` (read-only while serving),
  `archive` to `/archive`, `state` to `/state`. SQLite is on the NAS host's own
  volume, not an SMB/NFS client mount. Filesystem type remains unrecorded.
- Host networking; read-only container root; 256 MiB temporary `/tmp` tmpfs.
  HTTP upstream/management and TLS listeners bind loopback on 9841/9842/9843.
  Public curation/administration disabled. Encoders and MusicBrainz contact
  unconfigured. No encoder/playback or live metadata acceptance is claimed.
- Browser access uses SSH local forwarding from Mac localhost:9842 to NAS
  127.0.0.1:9842. DSM initially prohibited forwarding; a `Match User rynban`
  rule allows local forwarding with `PermitOpen 127.0.0.1:9842`.
  The `.local` hostname became unavailable; numeric LAN IP restored connectivity.
  This is private tunneled HTTP, not remote trusted-HTTPS/passkey qualification.

## Results

- Operator initialized owner account and signed in using Firefox on the Mac.
- Uploaded complete owned `sq-relax-vol-2` dump and downloaded six originals.
  Assistant independently streamed SHA-256 over local source/download bytes:
  all six lengths/digests match before and after container recreation.
  Download names add `sq-relax-vol-2_`; bytes are unchanged.
- Original container `f325a2c78878` was stopped (exit 137); replacement
  `eb5219fb744f` started successfully using the same mounts. Exit 137 alone
  does not establish OOM or explain the kill. Graceful shutdown is unqualified.
- Operator reported another slow `docker stop -t 60`. Source inspection found
  the serve loop handles only `tokio::signal::ctrl_c()`, not SIGTERM; the
  Dockerfile declares no alternative stop signal. This explains why normal
  container stop can wait for its timeout before forced termination. Treat
  graceful container shutdown as a found deployment defect, not a passed check.
- Browser session survived recreation. Operator explicitly logged out and back
  in successfully; imported originals remained downloadable.
- TLS PEM SHA-256 printed identically before/after recreation:
  `fca16e53278c49317bf5a10530d7b425f3836c18c406a457aa6ca59ca80ceef8`.
- Stopped-server backup to `/backups/roundtrip-1` reported `Verified backup`.
  Full restore into separate `/restore/archive`, `/restore/state` and
  `/restore/config/collector.toml` reported successful identity/credentials/TLS
  preservation and private Unix permissions. No external paths were configured.
- Operator completed the instructed restored-instance doctor/start/login/download
  drill with the original stopped. Restored container uses SIGINT as its stop
  signal and `--restart no`. Doctor output has not been captured. Original
  container remains stopped; do not run both copies.
- Operator then tested `docker stop -t 15` on the restored container: stopping
  was reported near-instantaneous and Docker recorded exit code 0. This confirms
  the SIGINT stop-signal workaround for this idle instance, not active-job drain
  or a fix for the default SIGTERM defect. Restart was instructed after the check;
  subsequent restored-container running state has not been independently checked.
- Assistant independently compared all six files downloaded from the restored
  instance (`~/Downloads/sq-relax-after-restore`) against the original source:
  all lengths and SHA-256 values match the table below. This qualifies the
  exercised full backup/restore round trip on this NAS, not every recovery mode.

Source basenames share prefix `dump_260919_134632_disk7`:

| Suffix | Bytes | SHA-256 |
|---|---:|---|
| `.fulltoc` | 268 | `727792d01ac734b92421235eb4e783f29f17085be833d8fb68bc9b25f8b3cd74` |
| `.log` | 2045 | `e6f17b32245313ae003eefbcca418b0b02e98006a170c2ea7d05f13d18660e9b` |
| `.scram` | 840270792 | `975c7c0e6d7bbe2b5b46df7b4c590357812cfe775c8f3cd7c6cd20ea6238d6a9` |
| `.state` | 210067698 | `7dcea2fcc479b51d600e7ef208e3605f8091a98107b136e4a9fa25a71b7a4e58` |
| `.subcode` | 34296768 | `7f9a009151da7765671f86f6ae86610c9f54cc93d30532c8c859660da19d6de9` |
| `.toc` | 180 | `27f74f1d5f8a7f77e357b122318b617d4cef340fd4ddd7b0dc220cd1e0f069ae` |

This establishes byte-preserving round trips, not catalog or CD audio verification.
Interrupted active work, permission failures, host reboot,
actual power-loss/storage failure, abuse/load checks and public ingress remain open.

## Evening live installation — Portainer and Cloudflare Tunnel

Continuation recorded 2026-10-19, through 22:22 JST. Earlier test-installation
results above remain historical; these observations concern a fresh permanent
installation. Evidence is operator-provided output/screenshots and source
inspection, not independent remote administration of the NAS.

- Operator built source checkout `9fcf42e` for `linux/amd64`, tagged
  `junk-collector:9fcf42e`, transferred and loaded it on the NAS. The new immutable
  image ID has not been collected. Modern `scp` failed because the SSH SFTP
  subsystem was unavailable; `scp -O` was the transfer workaround. The separate
  post-quantum key-exchange warning is not the SFTP failure. NAS administration
  follow-up is tracked in the consolidated remaining-work queue.
- Permanent roots are `/volume1/junk-collector/{config,archive,state,backups}`.
  DSM rejected initialization until the operator created the bind source
  directory. Initialization and owner setup then completed. The initial doctor
  output still advertised localhost with public management disabled; the cause
  of the unsuccessful proxy-config application is unresolved. Explicitly writing
  the retained config corrected it without rerunning account setup.
- Portainer manages the `junk-collector` stack. Collector runs UID/GID 10001,
  host networking, read-only root, dropped capabilities and temporary `/tmp`.
  Configuration is mounted read-only. Startup output confirms public HTTP
  `127.0.0.1:9841`, private HTTP `127.0.0.1:9842`, direct TLS
  `127.0.0.1:9843`, and available FLAC/Opus encoders.
- A dedicated `cloudflare/cloudflared:2026.9.1` host-network connector runs in
  that stack with its token held in Portainer configuration. The existing
  bridge-network connector and other service routes were left in place. The
  new tunnel connects `https://collector.awkwardexplorer.com` to the public
  loopback HTTP listener on 9841. Authentication/allowed origins use that HTTPS
  origin; public administration is explicitly enabled (also enabling curation).
  Operator reported working browser access. This does not complete the broader
  authentication, exposure, caching, abuse, upload or passkey qualification.
- The SMB inbox share `/volume1/junk-collector-inbox` is mounted at `/inbox`,
  configured as inbox `cds`. The initial share ACL allowed only administrators;
  Collector listing failed with permission denied. Operator created DSM group
  `collector-inbox-readers` (GID 65538); the stack adds this supplemental group.
  Operator requested read/write access and was instructed to grant share RW and
  use `:rw`. Container output confirms GID 65538 and successful directory listing;
  actual write/delete ability has not been tested. Current imports retain inbox
  sources; no automatic cleanup is implemented or exercised.
- Inbox review includes Synology/macOS housekeeping (`#recycle`, `@eaDir`,
  `.DS_Store`). Octopath packages initially included both CUE/BIN and Redumper
  descriptions; batch review reported multiple anchors but source inspection
  confirmed it omits the selector available in guided ingest. After the operator
  removed the alternate CUE/BIN view from the presented package inventory,
  all three packages (two Octopath discs and SQ Relax Vol. 2) showed analysis
  and FLAC availability. No import completion or audio/catalog verification is
  established by this preview.
- Source inspection confirms the batch album group/title field sets a manual
  title. Operator wants the two Octopath discs grouped with an automatic title.
  Importing the first disc then attaching the second to its resolved album was
  suggested, but this workflow has not yet been exercised in this installation.
- MusicBrainz was initially unconfigured. Operator supplied a monitored contact
  and received instructions to append the provider user agent and restart only
  Collector. No successful live provider response, accepted metadata/artwork,
  preparation or playback is recorded yet. Startup/doctor still print obsolete
  Phono-specific instructions despite that retired target.

Open fixes and requests live only in the
[remaining-work queue](../remaining-work.md#placement-of-the-2026-10-19-live-use-findings),
triaged to HL-05 deployment/recovery, HL-07 release usability/packaging and HL-09
optional inbox cleanup. The chronological evidence here does not assign work.
New-installation import/download equality, playback, restart persistence,
stopped-server backup/restore and Internet-ingress checks remain outstanding;
earlier test-instance results must not be attributed to this new installation.

### Grouped-disc failure — 22:30–22:33 JST

The operator subsequently reported Octopath disc 2 failing with `revision conflict`.
Its progress screenshot shows staging/identification completed, Save originals
failed, and metadata/artwork/preparation waiting. Retry this step returned the
same conflict. The operator was instructed to retain that existing import and
sources. A local grouped-publication regression reproduced a sibling Evidence
revision invalidating the frozen album revision; its source repair and local
acceptance are recorded in [repair validation](grouped-disc-publication-2026-10-19.md).
The NAS has not yet been updated to that repair, and actual recovery is pending.

### Update preparation and backup scaling — through 23:06 JST

- At 23:02 JST the operator reported that the normal Mac terminal command block
  (`cargo test --workspace` followed, on success, by the amd64 Docker build)
  succeeded. No detailed test transcript/counts were supplied, so this is an
  operator report distinct from the restricted assistant-runtime results.
- At 23:03 JST the operator confirmed exporting, transferring with `scp -O`, and
  loading `junk-collector:grouped-fix-20261019` on the NAS. Its immutable image ID
  has not been collected. The running Portainer service has not been reported
  switched to this tag.
- Stopping Collector, backing up to
  `/data/backups/before-grouped-fix-20261019`, and running `validate-backup` were
  instructed. The operator raised the scaling concern before providing results;
  stopped/running state and backup completion/validation are unconfirmed. Do not
  assume this backup exists or this installation has completed its first backup.
- The operator expects 1,000+ albums and hundreds of game dumps, and rejects full
  archive duplication as a routine requirement for every Portainer update. The
  existing backup/validation commands copy and hash the full inventory. Short
  stopped-server NAS snapshots plus incremental independent backups were proposed,
  not configured or tested. Actual Btrfs/DSM eligibility, consistency across roots,
  retention, restore and independent destination remain unqualified. The request
  and acceptance criteria are recorded in HL-05 of the consolidated queue.

## Lightweight live HTTPS check — 2026-10-21

The operator selected the existing `https://collector.awkwardexplorer.com` for
public-ingress checks and challenged the disproportionate qualification scope.
An additional isolated tunnel is not required. Reuse existing backend/isolated
coverage and operator-reported working browser access/playback; broader proxy,
failure and load matrices are follow-ups unless a concrete problem warrants them.

Read-only checks found a responding Cloudflare-backed endpoint and matching
configured public curation/administration, intentionally enabled as recorded above.
Chrome 155.0.8059.85, using normal certificate validation and a fresh anonymous
context, loaded the sign-in page without script errors. Same-origin browser reads
of `/api/v1/me` and `/api/v1/admin/settings` returned application JSON HTTP 401 with
`unauthenticated`. Plain automated HTTP requests also encountered HTTP 403; those
responses were not used as proof of Collector authorization.

No login, account mutation, upload, flooding, production reconfiguration or trust
installation was performed. Authenticated cookie/media/seek behavior was not
independently repeated in this check; retain the earlier operator reports and
existing tests rather than presenting these read-only observations as a new full
end-to-end media test. This establishes a useful live HTTPS smoke check, not
security certification. The previously pending source CI run also completed
successfully before this follow-up.
