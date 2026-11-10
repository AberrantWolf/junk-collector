# Configuration and access

Architectural rules: [ADR 0009](decisions/0009-exposure-composition-and-independent-authorization.md), [ADR 0010](decisions/0010-trust-before-credentials-and-native-enrollment.md), [ADR 0012](decisions/0012-durable-workflow-ownership-and-fencing.md).
This document retains detailed requirements, rationale and evidence; the ADRs do
not replace its supported-scope and recovery limits.

[← README](../README.md) · [First setup](getting-started.md) · [Operations](server-operations.md)

## Configuration reference

Optional: [enable backup-age reminders](local-alpha-installation.md#optional-backup-age-reminder).

Edit `collector.toml` and restart the server to apply changes. Unknown configuration keys are rejected. Relative filesystem paths are relative to the process's working directory, so absolute paths are recommended for a permanent installation or service.

### Storage and accounts

| Option | Default | Effect |
|---|---|---|
| `archive` | `".collector/archive"` | Original files, collection manifests, revisions, artwork, derivatives, and evidence. |
| `state` | `".collector/state"` | Local SQLite databases, accounts, credentials, TLS keys, jobs, and upload/processing scratch. |
| `multi_user` | `true` | Users enter their own username at sign-in. Set to `false` to use the default account on the sign-in form and disable new account creation. Both modes require authentication. |

Keep `state` on local disk, even if `archive` is on a NAS. Allow space for retained originals, prepared audio, uploads, and temporary conversion work. Keep the complete state directory private to the server account; it contains credentials and TLS keys.

### Listeners and browser access

| Option | Default | Effect |
|---|---|---|
| `public_bind` | `"127.0.0.1:9841"` | HTTP upstream for a public HTTPS proxy or tunnel. Must bind loopback. |
| `private_bind` | `"127.0.0.1:9842"` | HTTP management listener for local use or a private HTTPS proxy. Must bind loopback. |
| `lan_bind` | `"0.0.0.0:9843"` | Direct TLS listener, using the public route composition. The default listens on all IPv4 interfaces; choose a specific interface to narrow it. |
| `private_tls_bind` | Unset | Optional direct TLS management listener. Must name an explicit interface address, such as `"192.168.1.20:9844"`; wildcard addresses are rejected. |
| `tls_names` | `["localhost", "junk-collector.local", "127.0.0.1"]` | DNS names/IP addresses included in the server-managed certificate when setup creates it. Choose these before initialization. Changing this list does not replace an existing certificate. |
| `allowed_origins` | HTTPS origins for `localhost:9843` and `junk-collector.local:9843`; setup adds the local management origin | Exact browser origins allowed for session operations. Include the scheme, hostname, and non-default port. This does not establish certificate trust or grant access. |
| `authentication_origin` | Unset | Stable HTTPS origin for WebAuthn passkey registration/login. Also include it in `allowed_origins`. |
| `endpoints` | `["https://junk-collector.local:9843"]` | Addresses included in exported pairing material. Does not bind listeners, create DNS records, or advertise an mDNS service. |

A `.local` name must resolve on your network independently of Junk Collector. Merely listing it in `tls_names` or `endpoints` does not make it reachable.

### Public exposure

Audio uses the same authenticated connection as the website by default, including
through proxies and tunnels. Collector does not detect Cloudflare or enforce
provider restrictions. The server operator chooses the route policy; login,
collection permissions and exposure checks always apply.

| Option | Default | Effect |
|---|---|---|
| `remote_media_allowed` | `true` | Uses the normal signed-in connection for audio. Set `false` to require a private or separate HTTPS audio route on public listeners. |
| `local_playback_enabled` | `true` | Starts the optional private media listeners. They are unused by normal-mode playback. Set `false` for a setup without those listeners. |

In **Administration → Server settings → Audio connection**, enable **Require a
separate audio connection** to disable normal public-origin audio. Saved choices
persist across restarts and override `remote_media_allowed` in the config. New
requests observe the setting; reconnect other clients to clear cached routes.
An active transfer is not forcibly disconnected. Direct LAN listeners retain
ordinary media access.

For optional separate HTTPS streaming, configure `[remote_media]` with `base_url`
(a fixed HTTPS URL ending in `/`) and `socket` (an absolute Unix socket path in a
dedicated shared directory). Run `collector-media` with that socket and read-only
archive access. The endpoint may stay configured while normal audio is enabled;
it is advertised only when separate audio is required. If no separate/private
route is reachable in that mode, playback fails visibly without public fallback.
See the [operator instructions](../deploy/container/media.md).
No setting opens the router, creates DNS or supplies browser certificate trust.

These settings apply to both the public HTTP upstream and the direct LAN TLS listener. Private listeners include all management surfaces; users still need the appropriate permissions.

| Option | Default | Effect |
|---|---|---|
| `[exposure].public_curation` | `false` | Exposes collection editing, imports, metadata review, and their backing APIs on public routes. |
| `[exposure].public_administration` | `false` | Exposes server/account/access administration on public routes. Also enables public curation. |

Login, authorized browsing, prepared media, and downloads are available on client routes without enabling either management setting. Exposure never grants a collection role. Disabled management pages, JavaScript bundles, and APIs are absent from the public router; forwarded headers cannot make a public request private.

### Example: local installation with music support

Change the storage and executable paths for your machine. Omit the optional provider section if you do not want online metadata acquisition.

```toml
archive = "/srv/junk-collector/archive"
state = "/var/lib/junk-collector"
multi_user = true

public_bind = "127.0.0.1:9841"
private_bind = "127.0.0.1:9842"
lan_bind = "0.0.0.0:9843"
tls_names = ["localhost", "junk-collector.local", "127.0.0.1"]
allowed_origins = ["http://localhost:9842"]
endpoints = ["https://junk-collector.local:9843"]

[exposure]
public_curation = false
public_administration = false

[encoders]
flac = "/usr/bin/flac"
# opus = "/usr/bin/opusenc"
# ffmpeg = "/usr/bin/ffmpeg" # live playback; defaults to ffmpeg on PATH

[providers]
musicbrainz_user_agent = "JunkCollector/0.1.0 (you@example.com)"

[inboxes]
rips = "/srv/incoming"
```


### Live playback format

Use **Playback format** in the player, or **Account & security → Playback** on
a phone. Choose **Saved audio**, **FLAC**, or **Opus**. Opus accepts any whole
target bitrate from 6 to 256 kbps, including on mono sources. The preference
is saved for this account in this browser and takes effect the next time you
press Play or start a track.

With normal authenticated audio, Collector can stream a track directly from a
supported raw CD rip without preparing a file. It prefers saved FLAC for the
selected track, then its original, and can use a retained Opus copy when the
original cannot be read. FLAC avoids a new lossy encode; converting MP3 or Opus to FLAC
does not recover lost detail. Changing format does not change
the library or its saved files. An existing matching saved file can use regular
HTTP byte ranges. Live audio seeks by starting a new stream from the requested
time; the player needs a known duration to show a live seek bar.

The server uses FFmpeg (with its FLAC and libopus encoders) for live conversion.
The container includes it; native installs can set `encoders.ffmpeg` or install it
on PATH. Each live stream has its own encoder and source reader. A server accepts
up to eight concurrent live requests, with two per session or native credential
to allow overlap during seek and track changes. This limit is separate from
background processing. No conversion is saved. Dedicated WebTransport and
separate HTTPS media still deliver saved audio; they do not carry live conversion.

### CD library defaults

In **Administration → Library settings**, choose a music collection you own.
Automatic AccurateRip checking during new CD ingest defaults on; save the checkbox
to turn it off. The maximum offset accepts 0–2939 stereo sample frames and defaults
to 2939. A new import freezes these settings at acceptance. Existing unfinished
imports keep their previously accepted behavior. Preservation-only imports and
digital purchases do not acquire CD checks.

**Check unconfirmed rips** starts a bounded batch over the collection's existing
originals. Confirmed sources are skipped, and every other eligible source gets a
structure/response/audio workflow. Open the batch's album results for no-data,
ambiguity, missing files or unsupported packages. The button is an explicit run;
there is no perpetual retry of absent provider data. New ingests check automatically
when enabled. Defaults are private operational state retained through restart and
reindex, independent of portable metadata authority.

### Background workers

```toml
[jobs]
processing_workers = 4
copy_workers = 1
```

`processing_workers` accepts 1–8 and includes inbox copying, analysis, original
publication and audio preparation. `copy_workers` independently caps concurrent
inbox staging (including the older combined inbox import command); it accepts
1 through `processing_workers` and defaults to one. Provider acquisitions have
four additional slots overall, with at most one active Job per provider. Omitting `[jobs]` uses four processing workers and
one copy. These are startup defaults. **Administration → Server settings →
Background workers** saves an override that survives restart and reindex. Saved
limits apply to new claims immediately; reducing them lets current jobs finish.
Configuration-file edits require restart and do not replace a saved override.
Invalid settings are rejected before opening storage.

Ready copied discs get bounded priority for analysis and original publication.
With one worker this normally finishes one disc's originals before the next copy.
Workers are not permanently assigned to a disc: provider waits and unavailable or
failed steps cannot hold a slot. Other processing is offered FIFO service after
two preferred continuations. Copying gets a turn after two processing admissions
when both are runnable, allowing one ready save to finish a just-analyzed disc
before that copy turn. With multiple workers, the scheduler leaves
room for both waiting classes, and processing can use all available slots when
copying is idle. The copy cap still applies when processing is idle. Running jobs
finish or acknowledge cancellation before their capacity is reused.
Metadata resolution gets priority over queued encoding and ordinary processing,
sharing at most two preferred admissions with ready-disc continuations before
another processing turn.
It starts after originals are saved; copying keeps its own scheduling turns.

More workers can increase CPU, memory and storage contention. Four is the default;
eight is available for operator tuning, not a throughput guarantee. Original
publication still uses the existing archive writer. These settings do not remove
the extra inbox-to-private-staging-to-archive copy.

### Audio encoders

| Option | Effect |
|---|---|
| `[encoders].flac` | Path or executable name for `flac`. Enables validated lossless track preparation. |
| `[encoders].opus` | Path or executable name for `opusenc`. Enables 96, 128 and 192 kbps VBR track, album, batch and optional import preparation. See [audio preparation](audio-preparation.md). |

Both are optional and are probed at startup and by `doctor`. Use absolute executable paths when running as a service, whose `PATH` may differ from your shell. Unavailable tools do not prevent preservation or original-file downloads.

Preparation stores separate audio files in the archive; it does not replace the raw dump. It runs only when requested, either through the optional guided-import checkbox or the music preparation controls. Browsing and playback requests do not start conversion.

### Online metadata and artwork

`[providers].acoustid_api_key` enables explicit digital album matching when
`musicbrainz_user_agent` is also configured. Use an AcoustID application key;
restart the server after changing provider configuration. The matching panel
explains the disclosure before submission: fingerprints and durations go to
AcoustID; original audio, paths and private purchase data are not sent. Matching
is available to curators through the configured curation listener/API exposure.

`[providers].musicbrainz_user_agent` enables MusicBrainz and Cover Art Archive acquisition. Set it to a versioned application name with a real contact address, for example `"JunkCollector/0.1.0 (your-real-address@example.org)"`.

Guided imports can automatically gather available metadata and artwork. In an item's editor, you can also request metadata, inspect alternatives, set manual overrides, and choose cover art. Cover Art Archive choices show small previews before you archive an image. Manual overrides survive provider refreshes.

Without this setting, imports and manual curation still work. MusicBrainz identity is descriptive metadata, **not audio verification**. AccurateRip checks are a separate music operation and do not use this user-agent setting.

### Server inboxes

Each entry in `[inboxes]` maps a display name to a server-side directory:

```toml
[inboxes]
rips = "/srv/incoming"
```

Put each package in a direct child directory. A package may contain nested files; preserve the filenames and relative paths referenced by CUE sheets and other sidecars.

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

Curators select packages under **Add / import → Configured inbox**. Package contents are copied into staging; source files are retained. Symlinks and special files are rejected. The browser sees configured inbox names and package names, not server filesystem roots.

### Game DAT catalogs

Local DAT files provide reference data for game matching. Configure each under a unique source name:

```toml
[dat_sources.nes]
path = "/srv/catalogs/nes.dat"
system = "nes"
catalog = "NES headerless catalog"
byte_domain = "org.junk.game.nes.ines-payload"
byte_domain_version = 1
```

| Field | Effect |
|---|---|
| `path` | Local DAT file to import. |
| `system` | System label associated with the catalog. |
| `catalog` | Human-readable catalog label. |
| `byte_domain` | The exact byte representation covered by its checksums; must agree with the catalog and processing operation. |
| `byte_domain_version` | Version of that representation; defaults to `1`. |

Supported normalized domains include:

| Catalog bytes | `byte_domain` |
|---|---|
| Complete files, without normalization | `org.junk.game.raw-bytes` |
| CD tracks with 2352-byte raw sectors | `org.junk.game.cd.raw-2352-track` |
| NES iNES payload, excluding its header | `org.junk.game.nes.ines-payload` |
| Nintendo 64 big-endian ROM bytes | `org.junk.game.n64.z64-big-endian` |
| SNES headerless payload | `org.junk.game.snes.headerless-payload` |
| PC Engine HuCard payload | `org.junk.game.pce.hucard-payload` |

In **Administration**, import the configured DAT as an immutable snapshot and select the active snapshot. Results retain their original snapshot identity when you later select another catalog. A file hash, structural analysis, or tentative title is not a complete catalog match; consult each result's coverage and limitations.

The optional manual download helper supports an allowlist of catalogs:

```sh
python3 scripts/cache_game_dats.py --help
python3 scripts/cache_game_dats.py /srv/catalog-cache --catalog redump-psx
```

It retains downloaded data and a digest manifest. The server does not automatically download or refresh DATs. New DAT-o-MATIC downloads are disabled in the helper; local DAT files and verified cached data remain usable.

## Users and collection access

Users sign in with their own username by default. For an existing configuration
with `multi_user = false`, set `multi_user = true` and restart the server to enable
username entry and account creation. Create accounts in **Administration → Accounts**
and assign each account the appropriate collection role.

| Role | Access |
|---|---|
| Viewer | Browse authorized collection data, view shared artwork, play and download prepared media. |
| Curator | Viewer access plus inventory changes, imports, metadata/artwork work, and processing commands. |
| Owner | Curator access plus collection access management. |

Server administration and collection access are separate permissions. An administrator does not automatically receive permission to read every collection. Single-user mode uses the stored default account on the sign-in form and blocks new account creation in Administration and the API. Existing accounts and grants are retained.

### Passkeys

Set `authentication_origin` to a stable, browser-trusted HTTPS origin and add it to `allowed_origins`. Register and use passkeys on that origin. The browser needs the standard WebAuthn JSON conversion APIs. Password login remains available.

Use **Account & security** for Profile, Password, Passkeys, Recovery, Browser sessions and Apps. Changing the password retains existing access. **Sign out other sessions and apps** keeps this browser, passkeys and recovery codes. Using a recovery code replaces browser/app access and the old code batch, retaining passkeys; save replacement codes afterward. For local account recovery, see [server commands](server-operations.md#operations-and-backups).

## Access from another computer

For browser access, use HTTPS trusted by that browser. The certificate generated by setup is not automatically trusted by other devices. Do not bypass certificate verification.

| Connection | Destination | Setup |
|---|---|---|
| Private HTTPS reverse proxy or Tailscale Serve | `http://127.0.0.1:9842` | Provides management to trusted network users; application login and roles still apply. |
| Direct private HTTPS | Explicit `private_tls_bind`, e.g. `192.168.1.20:9844` | Trust the server certificate in the browser and ensure its names cover the hostname/IP you use. |
| Direct client HTTPS | `lan_bind`, normally port `9843` | Uses the server certificate and public exposure settings. |
| Public HTTPS reverse proxy or Cloudflare Tunnel | `http://127.0.0.1:9841` | Client access by default; enable public curation/admin only if desired. |

Add each browser-facing HTTPS origin to `allowed_origins`, including its port when non-default. A private proxy with a browser-trusted certificate avoids having to install the server's certificate on each browser. Tailscale HTTPS uses a tailnet hostname, not an arbitrary `.local` name.

Keep private and public proxy destinations separate. Public tunnels must target `public_bind`, even when they connect from localhost. Never publish the private management listener through a public tunnel.

## Direct HTTPS and certificate trust

A reverse proxy or tunnel normally manages the browser-facing certificate. It
does not use Collector's generated certificate when its upstream is loopback HTTP.

For direct HTTPS, configure certificate names before `setup`, obtain only the
public `state/tls/cert.pem` through a trusted administrator channel, and compare
its file SHA-256 with the fingerprint in startup output. Install it in the trust
store used by the browser/client, then connect using a matching name or IP.
Never distribute `key.pem` or bypass certificate validation. The generated
certificate is not automatically trusted by other devices.

Certificate names and advertised endpoints do not create DNS or mDNS records.
A `.local` name must resolve independently. Changing configured names does not
replace an existing certificate; plan certificate renewal/trust updates explicitly.
The optional direct listeners are separate from the proxy's certificate lifecycle.

## Remote archive admission

`[archive_mount]` with `nfs_source = "HOST:/EXPORT"` requires the exact Linux
NFSv4.1/4.2 source, a hard read/write mount and no nested archive mounts. Archive
identity and local-state binding are required by ordinary executable startup;
`enroll-archive` is the explicit stopped-server adoption command for existing
installations, while `setup` initializes new storage. Do not remove a binding
to bypass a refusal. `--nfs-source SOURCE` (or `COLLECTOR_NFS_SOURCE`) applies the
same source requirement to offline tools, including explicit import/restore
destinations. See [the split deployment and recovery procedure](../deploy/container/remote-archive.md).
