# Configuration and access

[← README](../README.md) · [First setup](getting-started.md) · [Operations](server-operations.md)

## Configuration reference

Optional: [enable backup-age reminders](local-alpha-installation.md#optional-backup-age-reminder).

Edit `collector.toml` and restart the server to apply changes. Unknown configuration keys are rejected. Relative filesystem paths are relative to the process's working directory, so absolute paths are recommended for a permanent installation or service.

### Storage and accounts

| Option | Default | Effect |
|---|---|---|
| `archive` | `".collector/archive"` | Original files, collection manifests, revisions, artwork, derivatives, and evidence. |
| `state` | `".collector/state"` | Local SQLite databases, accounts, credentials, TLS keys, jobs, and upload/processing scratch. |
| `multi_user` | `false` | When `false`, login uses the default profile and hides username selection. When `true`, users sign in to separate accounts. Authentication remains required in either mode. |

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
multi_user = false

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

[providers]
musicbrainz_user_agent = "JunkCollector/0.1.0 (you@example.com)"

[inboxes]
rips = "/srv/incoming"
```

### Audio encoders

| Option | Effect |
|---|---|
| `[encoders].flac` | Path or executable name for `flac`. Enables validated lossless track preparation. |
| `[encoders].opus` | Path or executable name for `opusenc`. Enables explicit 96, 128, and 192 kbps track preparation. |

Both are optional and are probed at startup and by `doctor`. Use absolute executable paths when running as a service, whose `PATH` may differ from your shell. Unavailable tools do not prevent preservation or original-file downloads.

Preparation stores separate audio files in the archive; it does not replace the raw dump. It runs only when requested, either through the optional guided-import checkbox or the music preparation controls. Browsing and playback requests do not start conversion.

### Online metadata and artwork

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

For separate trusted users, set `multi_user = true`, restart, and create accounts in **Administration**. Assign each account the appropriate collection role:

| Role | Access |
|---|---|
| Viewer | Browse authorized collection data, view shared artwork, play and download prepared media. |
| Curator | Viewer access plus inventory changes, imports, metadata/artwork work, and processing commands. |
| Owner | Curator access plus collection access management. |

Server administration and collection access are separate permissions. An administrator does not automatically receive permission to read every collection. Single-user mode keeps authentication and uses the default account; it is not anonymous access.

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
