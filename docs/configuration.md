# Configuration and access

[← README](../README.md) · [First setup](getting-started.md) · [Operations](server-operations.md)

Edit `collector.toml` and restart the server to apply changes. Unknown keys are
rejected. Relative paths are relative to the process's working directory, so use
absolute paths for a permanent installation.

## Storage and accounts

| Option | Default | Effect |
|---|---|---|
| `archive` | `".collector/archive"` | Originals, collection manifests, revisions, artwork, prepared audio and evidence. |
| `state` | `".collector/state"` | SQLite databases, accounts, credentials, TLS keys, jobs and upload/processing scratch. |
| `multi_user` | `true` | Users type their username at sign-in. `false` uses the default account and disables new account creation. Both modes require a password. |

Keep `state` on local disk even if `archive` is on a NAS, and keep it private to the
server account: it holds credentials and TLS keys. Leave room for originals, prepared
audio, uploads and conversion scratch.

## Listeners and browser access

| Option | Default | Effect |
|---|---|---|
| `public_bind` | `"127.0.0.1:9841"` | HTTP upstream for a public HTTPS proxy or tunnel. Must be loopback. |
| `private_bind` | `"127.0.0.1:9842"` | HTTP management listener for local use or a private HTTPS proxy. Must be loopback. |
| `lan_bind` | `"0.0.0.0:9843"` | Direct TLS listener with public-route exposure rules. |
| `private_tls_bind` | unset | Optional direct TLS management listener on an explicit interface address (e.g. `"192.0.2.20:9844"`); wildcards are rejected. |
| `local_playback_bind` | `"0.0.0.0:9845"` | Optional private media listener. |
| `tls_names` | `["localhost", "junk-collector.local", "127.0.0.1"]` | Names/IPs put in the certificate when `setup` creates it. Changing them later does not replace the certificate. |
| `allowed_origins` | `https://localhost:9843`, `https://junk-collector.local:9843`, plus the local management origin added by setup | Exact browser origins (scheme, host, non-default port) allowed to use sessions. |
| `authentication_origin` | unset | Stable HTTPS origin for passkeys. Also list it in `allowed_origins`. |
| `endpoints` | `["https://junk-collector.local:9843"]` | Addresses included in pairing data. Does not bind listeners or create DNS/mDNS records. |

A `.local` name must resolve on your network by other means; listing it here does not
make it reachable.

## Exposure

The public listener and the direct LAN listener serve client pages (browsing,
playback, downloads) by default. Management can be exposed there explicitly:

| Option | Default | Effect |
|---|---|---|
| `[exposure].public_curation` | `false` | Collection editing, imports, metadata review and their APIs. |
| `[exposure].public_administration` | `false` | Server, account and access administration. Also enables public curation. |

Private listeners always include every management page. Exposure never grants a
collection role. Disabled pages, bundles and APIs are absent from the public router,
and forwarded headers cannot make a public request private.

### Audio routes

| Option | Default | Effect |
|---|---|---|
| `remote_media_allowed` | `true` | Audio uses the normal signed-in connection, including through proxies. `false` requires a private or separate HTTPS audio route on public listeners. |
| `local_playback_enabled` | `true` | Starts the optional private media listeners. `false` disables them. |

**Administration → Server settings → Audio connection → Require a separate audio
connection** overrides `remote_media_allowed` and persists across restarts. New
requests see the change; active transfers continue.

For separate HTTPS streaming, set `[remote_media]` with `base_url` (an HTTPS URL ending
in `/`) and `socket` (absolute Unix socket path in a dedicated shared directory), and
run `collector-media` on that socket with read-only archive access. It is only
advertised when separate audio is required; if no separate route is reachable then,
playback fails rather than falling back to the public origin. See
[media deployment](../deploy/container/media.md).

## Example

```toml
archive = "/srv/junk-collector/archive"
state = "/var/lib/junk-collector"

allowed_origins = ["http://localhost:9842"]

[encoders]
flac = "/usr/bin/flac"
# opus = "/usr/bin/opusenc"
# ffmpeg = "/usr/bin/ffmpeg"

[providers]
musicbrainz_user_agent = "JunkCollector/0.1.0 (you@example.com)"

[inboxes]
rips = "/srv/incoming"
```

## Live playback format

Choose **Playback format** in the player or **Account & security → Playback**. It is
saved per account per browser and applies from the next track.

- **Original** serves the WAV, FLAC or MP3 file; raw CD tracks are served as seekable
  16-bit stereo WAV.
- **High Bandwidth** (default) serves FLAC/MP3 originals, reuses prepared FLAC, and
  converts WAV and CD audio to FLAC on the fly.
- **Low Bandwidth** uses Opus, 128 kbps VBR by default (6–256 kbps), reusing a matching
  prepared copy when there is one.

The stream API takes `mode=original|high_bandwidth|low_bandwidth` (default high) and
`bitrate` for low bandwidth. Files and CD WAV support byte ranges; live streams seek by
reopening at `start_ms`. Live conversion needs FFmpeg with FLAC and libopus (included in
the container; set `encoders.ffmpeg` or put it on PATH otherwise). The server runs up to
eight live encoders, two per session or app credential. Live playback writes nothing to
the archive.

## CD library defaults

In **Administration → Library settings**, per music collection you own:

- automatic AccurateRip checking of new CD imports (on by default);
- maximum offset, 0–2939 sample frames (default 2939).

Imports keep the settings in force when they were accepted. **Check unconfirmed rips**
runs a one-off check over existing originals that are not yet confirmed.

## Background workers

```toml
[jobs]
processing_workers = 4   # copying, analysis, publication, audio preparation
copy_workers = 1         # 1..processing_workers; concurrent inbox copies
```

Provider lookups have four extra slots, one per provider. **Administration → Server
settings → Background workers** saves an override that survives restart; lowering it
lets running jobs finish. More workers mean more CPU, memory and disk contention.

The scheduler favours finishing a disc that is already copied before starting the next
copy, and gives metadata lookups priority over queued encoding.

## Audio encoders

| Option | Effect |
|---|---|
| `[encoders].flac` | `flac` path or name. Enables lossless track preparation. |
| `[encoders].opus` | `opusenc` path or name. Enables 96/128/192 kbps Opus preparation. |
| `[encoders].ffmpeg` | FFmpeg for live playback; defaults to `ffmpeg` on PATH. |

All are optional and checked at startup and by `doctor`. Use absolute paths for a
service, whose PATH may differ from your shell. Prepared audio is stored alongside the
original, never in place of it. See [audio preparation](audio-preparation.md).

## Metadata and artwork providers

- `[providers].musicbrainz_user_agent` enables MusicBrainz and Cover Art Archive. Use an
  application name, version and a real contact address, e.g.
  `"JunkCollector/0.1.0 (you@example.org)"`.
- `[providers].acoustid_api_key` (an AcoustID application key, plus the user agent)
  enables digital album matching. Fingerprints and durations are sent to AcoustID;
  audio, paths and purchase data are not. Submitting fingerprints also needs the
  reviewer's personal AcoustID user key, entered in the review panel and not stored.

Without providers, imports and manual editing still work. Manual edits always win over
provider data. MusicBrainz identity is metadata, not audio verification.

## Server inboxes

`[inboxes]` maps display names to server directories. Each direct child directory is
one package; keep the relative paths that CUE sheets and sidecars reference.

```text
/srv/incoming/
├── Album A/
│   ├── disc.cue
│   └── disc.bin
└── Album B/
    ├── dump.toc
    ├── dump.scram
    ├── dump.state
    └── dump.subcode
```

Curators pick packages in **Add to library** with **Source: Server inbox**. Files are copied into
staging and the sources are left in place. Symlinks and special files are rejected. The
browser only sees inbox and package names.

## Game DAT catalogs

DAT catalogs can be uploaded in **Administration → Game catalogs** (see
[game identification](game-identification.md)) or configured as local files:

```toml
[dat_sources.nes]
path = "/srv/catalogs/nes.dat"
system = "nes"
catalog = "NES headerless catalog"
byte_domain = "org.junk.game.nes.ines-payload"
byte_domain_version = 1   # default
```

`byte_domain` names exactly which bytes the catalog's checksums cover:

| Catalog bytes | `byte_domain` |
|---|---|
| Whole files | `org.junk.game.raw-bytes` |
| CD tracks, 2352-byte raw sectors | `org.junk.game.cd.raw-2352-track` |
| NES iNES payload without header | `org.junk.game.nes.ines-payload` |
| Nintendo 64 big-endian ROM | `org.junk.game.n64.z64-big-endian` |
| SNES headerless payload | `org.junk.game.snes.headerless-payload` |
| PC Engine HuCard payload | `org.junk.game.pce.hucard-payload` |

Import the DAT in Administration and choose the active snapshot. Results keep the
snapshot they were matched against. The server never downloads DATs itself;
`python3 scripts/cache_game_dats.py --help` is an optional manual download helper.

## Users and roles

Create accounts in **Administration → Accounts** and grant collection roles:

| Role | Access |
|---|---|
| Viewer | Browse, view artwork, play and download. |
| Curator | Viewer plus inventory edits, imports, metadata/artwork and processing. |
| Owner | Curator plus managing who can access the collection. |

Server administration is separate: an administrator cannot read a collection without a
role on it.

**Account & security** covers profile, password, passkeys, recovery codes, browser
sessions and apps. **Sign out other sessions and apps** keeps this browser, passkeys
and recovery codes. Using a recovery code signs out all browsers/apps and replaces the
code batch; passkeys are kept. Passkeys need `authentication_origin` set to a stable
browser-trusted HTTPS origin. For local account recovery see
[server operations](server-operations.md#account-recovery).

## Access from another computer

Browsers need HTTPS they trust. The certificate generated by `setup` is not trusted by
other devices automatically; never bypass certificate checks.

| Connection | Points at | Notes |
|---|---|---|
| Private HTTPS proxy or Tailscale Serve | `http://127.0.0.1:9842` | Management for your network; login and roles still apply. |
| Direct private HTTPS | `private_tls_bind` | Install the server certificate; its names must cover the host you use. |
| Direct client HTTPS | `lan_bind` (9843) | Server certificate, public exposure rules. |
| Public HTTPS proxy or Cloudflare Tunnel | `http://127.0.0.1:9841` | Client access; management only if enabled. |

Add every browser-facing origin to `allowed_origins`. Public tunnels must target
`public_bind`, even from localhost; never publish the private listener. Tailscale HTTPS
uses the tailnet hostname, not a `.local` name.

For direct HTTPS, set `tls_names` before `setup`, copy only `state/tls/cert.pem` to the
client, check its SHA-256 against the startup output, and install it in that client's
trust store. Never distribute `key.pem`.

## NFS archive

`[archive_mount]` with `nfs_source = "HOST:/EXPORT"` requires the archive to be a hard,
read/write NFSv4.1/4.2 mount of exactly that source with no nested mounts. The
`--nfs-source SOURCE` flag (or `COLLECTOR_NFS_SOURCE`) applies the same check to offline
commands. Each state directory is bound to its archive; `setup` creates the binding for
new storage and `enroll-archive` adopts an existing archive while everything is
stopped. Do not remove a binding to get past a refusal. See
[remote archive](../deploy/container/remote-archive.md).

## Backup-age reminder

```toml
[backup_reminder]
after_days = 30
# For backups made outside Collector, report them yourself:
# externally_managed = true
# external_completed_at = "2026-01-01T00:00:00Z"
# external_failed_at = "2026-01-02T00:00:00Z"
```

Administration → Server settings and `doctor` then show when the last full backup
completed and whether it is due. Only a successful `backup` run counts; the reminder
never runs backups or checks that they still exist.
