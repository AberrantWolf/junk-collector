# Junk Collector

Junk Collector is a self-hosted archive for physical video games and music CDs: inventory, original dumps, artwork, metadata, and verification evidence in one place.

The web app supports collections with or without attached dumps, guided imports, optional metadata and artwork acquisition, game DAT matching, AccurateRip checks, and playback of prepared CD tracks. Each collection has one type—games or music CDs. Original files are retained alongside derived files and recorded evidence.

## Quick start

### Requirements

- Rust 1.95 or newer, a C/C++ build toolchain, CMake, and pkg-config. Linux builds also need OpenSSL development headers.
- A compatible `junk-libs` checkout beside this repository; Cargo uses its individual crates through local path dependencies:

  ```text
  projects/
  ├── junk-collector/
  └── junk-libs/
  ```

- Writable storage for the archive and private server state.
- Optional: `flac` for lossless CD preparation; `opusenc` for smaller audio copies.

Built web assets are included. Node.js 22+ is needed only to rebuild the WebUI, not to run the server.

### Build and initialize

Run these commands from the repository root. A release build is recommended for normal use, especially hashing and audio processing.

```sh
cargo build --locked --release -p collector-server
./target/release/collector-server setup
./target/release/collector-server doctor
./target/release/collector-server serve
```

Startup rebuilds the catalog from archive metadata and checks published snapshot
bytes, but does not read prepared-audio payloads. A prepared track is checked
against its recorded length and SHA-256 before each GET transfer; a changed or
unreadable file is rejected with an actionable player error. Explicit full
reindex checks prepared-audio bytes too. Startup logs cumulative times for
recovery, reindexing, and private-state reconciliation. The workspace optimizes
SHA-256 in debug builds; the release binary remains the recommended server.
Incremental startup indexing is tracked under P06 in the handoff backlog.

`setup` prompts for a password of at least 12 characters and creates:

- `collector.toml`, the server configuration;
- `.collector/archive`, containing preserved files and collection records;
- `.collector/state`, containing databases, accounts, TLS identity, jobs, and temporary work;
- the initial administrator account, named `owner` by default.

To choose a different account name, use `setup --username YOUR_NAME`. Passwords can also be supplied through standard input; do not put them in command arguments.

Open **http://localhost:9842** on the server computer and sign in. Use `localhost` exactly: `127.0.0.1` is a different browser origin. Local HTTP sign-in is tested with Chromium-based browsers; use browser-trusted HTTPS for remote browsers.

For permanent storage locations or custom certificate names, create `collector.toml` using the [configuration reference](#configuration-reference) **before running setup**. Supply a custom configuration path before the command:

```sh
./target/release/collector-server --config /absolute/path/collector.toml setup
./target/release/collector-server --config /absolute/path/collector.toml serve
```

Run the server as an ordinary user with access to its storage and configured inboxes. Only one server process may write an archive.

## Add your first collection

1. Sign in at the private management address and choose **Create collection** or **Administration**. Choose **Games** or **Music CDs** and give the collection a name.
2. Open the collection and choose **Add / import → Dump folder**. Select a folder containing complete dumps; subfolders are scanned for packages. Unrecognized files are listed separately and are not uploaded. To track an item without files, choose **Enter inventory manually**.
3. Review the package count. **Package details and import options** shows missing companions or unsupported operations and lets you change an ambiguous analysis anchor. Metadata and artwork gathering is enabled by default.
4. For supported CDs, optionally select **Prepare supported music as FLAC after preservation**. Choose **Import … packages**. Each package gets its own saved entry automatically; no title or separate inventory save is required.
5. Keep the page open while uploading. Progress shows bytes sent and bytes saved on the server. After uploads finish, server processing continues even if you close the page. If interrupted, reopen an import and select the same folder to resume. You can also choose individual files or configured inbox packages, then **Preview import → Start import**.
6. Open the item to inspect its files, metadata, artwork, and evidence. Use **Edit / attach files** to correct metadata, manage physical copies and loans, add images, or attach another capture.

Folder intake accepts up to 10,000 files and 64 detected packages at a time; select smaller subfolders for larger archives.

When attaching an imported disc to an existing album, its unused automatic placeholder is removed from browsing. Entries with other captures, physical inventory, wishlists, artwork, or manual information are kept. Archived files and history are retained.

Keep complete dump packages together: BIN/CUE sets need their referenced BIN files, and supported redumper CD packages need their TOC, SCRAM, STATE, and SUBCODE companions. Preservation and identification are separate: a package can be retained even when it cannot be identified or verified. The preview and recorded results explain what analysis is available.

### Play a music CD

With the [FLAC encoder configured](#audio-encoders), open the CD's editor and choose **Music preparation → Prepare selected album**. Select the intended disc sources when necessary. You can also prepare individual tracks or an available Opus profile.

Preparation runs as durable background work. Prepared tracks are saved automatically—**Save inventory is not required**. Once available, close the editor and use the **Disc player** in the item details, or download a track. Viewers can play prepared tracks. Closing the item details stops browser playback; browser codec support determines which formats play directly.

## Configuration reference

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

Use **Account & security** to manage account credentials and sessions. For local account recovery, see the command reference below.

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

## Operations and backups

The examples below use the built binary; place `--config /absolute/path/collector.toml` before any command when needed.

| Command | Purpose |
|---|---|
| `setup [--username NAME]` | Initialize storage, TLS, the default account, and local browser configuration. Run while stopped. |
| `init [--username NAME]` | First-time scripted initialization, accepting the password through standard input. |
| `serve` | Run listeners and background workers. Stop with Ctrl-C. |
| `doctor` | Check configuration and optional encoders without locking the archive; can run while serving. Does not test provider reachability, passwords, grants, or browser certificate trust. |
| `reindex` | Rebuild the catalog from archive records. Stop the server first. |
| `pairing` | Export server identity, public certificate, and endpoint data as JSON. Transfer it through a trusted channel; it contains no login credential and does not establish browser trust. Run while stopped. |
| `recover-account --private-state PATH --user-id USER_ID` | Recover one account while stopped. Reads a new password, revokes sessions/passkeys, and preserves grants and administrator status. Use the exact private-state root and account UUID. |

### Full backup

1. Stop the server.
2. Copy the complete `archive` directory, complete `state` directory, and `collector.toml` together to the backup destination.
3. Verify the copy and protect it like the live private state.
4. Restart the server.

The state root includes pending uploads and work as well as SQLite databases and TLS keys. Do not copy live SQLite files as an ordinary filesystem backup. Keep inbox originals and external DAT source files in your backup plan too; those paths are outside the server roots.

To restore, copy the saved roots into place, adjust configured filesystem paths if necessary, and start one server using that configuration. Do not run two copies of the same server identity concurrently. Archive-only reindexing does not restore accounts or access grants.

## Troubleshooting

| Symptom | Check |
|---|---|
| Login reports `origin_not_allowed` | Open the exact configured address. Add its origin to `allowed_origins` and restart. `localhost` and `127.0.0.1` are different origins. |
| Management pages are missing | Use a private listener, or intentionally enable the relevant public exposure setting. |
| “You do not have access to this resource” | Check the signed-in account's collection role. Public exposure does not grant permission. |
| Certificate warning | Use a hostname covered by a browser-trusted certificate. Pairing JSON alone does not make a browser trust it. |
| Music preparation is unavailable | Run `doctor`; check encoder paths, restart after configuration changes, and inspect the package's reported limitations. |
| Metadata or artwork is unavailable | Check the provider user agent and the job's error. Manual curation and preservation remain available during provider outages. |
| An import or preparation failed | Read the job/track error and fix its cause before retrying. Keep the complete source package and its original relative filenames. |

## Development

```sh
npm --prefix web ci
npm --prefix web run check
npm --prefix web run build
cargo build -p collector-server
python3 scripts/check_architecture.py
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Rebuild the Rust server after changing embedded web assets. `npm --prefix web run test:smoke` runs the browser journey against a temporary profile; `LOCAL_SETUP=1` selects localhost onboarding. Set `CHROME_PATH` when Chrome is not at the default macOS path.

See [architecture](docs/architecture.md) for dependency boundaries and [implementation status](docs/implementation-status.md) for detailed capability coverage.

## License

MIT. Third-party dependencies retain their respective licenses.
