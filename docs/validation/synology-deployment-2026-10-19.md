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
