# Junk Collector

Junk Collector is a self-hosted archive for video games, music CDs, and supported digital music purchases: inventory, original dumps, artwork, metadata, and verification evidence in one place.

The web app supports collections with or without attached dumps, guided imports, optional metadata and artwork acquisition, game DAT matching, AccurateRip checks, playback of prepared CD tracks, and direct playback of supported FLAC/MP3 purchases. Each collection has one type: games or music. Original files are kept alongside derived files and recorded evidence.

## Alpha release

The latest experimental prerelease is [`v0.1.0-alpha.2`](https://github.com/AberrantWolf/junk-collector/releases/tag/v0.1.0-alpha.2),
with public AMD64/ARM64 Docker images and a digest-pinned installation bundle. Builds are suitable for evaluating supported workflows and
backed-up personal use as an experimental alpha. Keep source originals and back up
the complete archive, private state, and configuration before updates. Storage
formats and interfaces may change; a stable upgrade contract and completed
Internet-deployment security qualification are not claimed.

See the [tagged release notes](docs/releases/v0.1.0-alpha.2.md) for that release,
and [current implementation status](docs/implementation-status.md) for newer
features and remaining limits. Public image pulls and isolated Portainer
install/update/restore are qualified; the live installation was left intact.

## Quick start

For a Linux Docker host or compatible NAS, use the
[container installation guide](deploy/container/README.md). It covers reverse
proxies and Cloudflare Tunnel, one-time account setup, bundled audio encoders,
persistent storage, updates, and recovery. Bring a browser-trusted HTTPS hostname;
direct HTTPS is available when no proxy is used.

To run the native server from source, follow the steps below.

### Source build requirements

- Rust 1.99.0 (the tested release toolchain, pinned in `rust-toolchain.toml`), a C/C++ build toolchain, CMake, and pkg-config. Linux builds also need OpenSSL development headers.
- Network access on the first build to fetch Cargo dependencies, including the exact `junk-libs` Git revision pinned in `Cargo.toml`. No sibling checkout is required.
- Writable storage for the archive and private server state.
- Optional: `flac` for lossless CD preparation; `opusenc` for smaller audio copies.

Built web assets are included. Node.js 22+ is needed only to rebuild the WebUI, not to run the server.

The pinned version is the tested release baseline; a lower minimum supported version
has not been established. See [release toolchain maintenance](docs/release-toolchain.md).

### Build and initialize

Run these commands from the repository root. A release build is recommended for normal use, especially hashing and audio processing.

```sh
cargo build --locked --release -p collector-server
./target/release/collector-server setup
./target/release/collector-server doctor
./target/release/collector-server serve
```

Startup reuses the maintained catalog when archive metadata and its checkpoint
are unchanged. It checks file metadata without reading music or snapshot bytes.
A stale, missing or damaged catalog is rebuilt from the archive; explicit full
reindex checks prepared-audio bytes too. Prepared tracks are checked against their
recorded length and SHA-256 before GET transfer, with actionable player errors
for changed or unreadable files. Startup logs checkpoint reuse and cumulative
recovery, catalog and private-reconciliation times. The workspace optimizes
SHA-256 in debug builds; the release binary remains the recommended server.
See [P06 measurements and recovery checks](docs/validation/p06-indexing.md).

`setup` prompts for a password of at least 12 characters and creates:

- `collector.toml`, the server configuration;
- `.collector/archive`, containing saved files and collection records;
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

1. Sign in at a management-enabled address and choose **Create collection** or **Administration**. Choose the games or music collection type and give it a name.
2. Open **Imports**, choose the collection, and select **Browser folder / files** or **Server inbox**. Choose folders for package discovery, individual files for one package, or inbox directories followed by **Review packages**. Unassigned files are listed separately.
3. Review **Files and analysis** for each package. Choose the CUE or Redumper analysis anchor when several are present and review which files are shared. Group discs with the same **Album group** label; leave the manual title empty for automatic metadata. Matching **Physical copy label** values within an album mean one purchased copy; different values mean separate copies. Copy labels are not disc numbers. Known Synology/macOS housekeeping is excluded; other unknown and hidden files stay in the package.
4. Automatic metadata and artwork are enabled by default when available. **Prepare FLAC after import** is optional for supported CDs. FLAC/MP3 purchases play directly and do not create physical copies. Choose **Start import**.
5. Keep the page open until browser uploading finishes. Processing continues afterward. If interrupted, reopen the saved batch and reselect its original folder or package files to resume.
6. Open the item to inspect files, metadata, artwork and evidence. Use **Edit / attach files** to correct metadata, manage copies and loans, add images or attach another capture. To record inventory without files, use **Add / import** and its manual-entry option.

The Imports workspace supports up to 10,000 packages and 100,000 files per intake,
with bounded package and manifest pages. Keep complete package boundaries when
splitting larger folders.

When attaching an imported disc to an existing album, its unused automatic placeholder is removed from browsing. Entries with other captures, physical inventory, wishlists, artwork, or manual information are kept. Archived files and history are retained.

Keep complete dump packages together: BIN/CUE sets need their referenced BIN files, and supported redumper CD packages need their TOC, SCRAM, STATE, and SUBCODE companions. Preservation and identification are separate: a package can be retained even when it cannot be identified or verified. The preview and recorded results explain what analysis is available.

### Play music

Supported FLAC/MP3 purchases play directly after import. For CDs, configure the
[FLAC encoder](#audio-encoders), open **Prepare audio**, select albums, preview the
FLAC preparation and start it. Individual tracks and configured Opus profiles are
also available in the editor's music preparation view. Preparation is explicit
background work; prepared files are saved automatically.

Open an album and choose **Play album**, **Play original** for one import, or a
track's **Play** button. Play replaces the personal queue and starts immediately;
**Add album to queue** appends instead. Album playback follows disc/track order.
Unavailable audio and ambiguous originals need review before whole-album playback;
available individual tracks remain usable. Viewers can play authorized audio.
Closing details or navigating between tasks leaves the persistent player running.
Browser codec support determines which existing formats play directly; downloads
remain available.

### Connect Trash Amp

In Trash Amp, open **Settings → Servers**, enter the **Collector server URL**, and
choose **Connect / Reconnect**. For ordinary system-trusted HTTPS, leave
**Owner-provided trust file** empty. For a self-signed server, obtain its public
PEM or pairing JSON through a trusted owner channel; independently compare the
PEM-file SHA-256 with server startup output before entering its path in that field.
Never transfer the private TLS key or bypass certificate validation.

Use the displayed browser approval link/code and sign in with the intended account.
The app receives separate revocable access. Trusting a certificate in Trash Amp
does not establish browser trust; same-computer approval can use the printed local
management `/pair` address. Remote browser approval requires its own trusted HTTPS
origin. Names in configuration do not create DNS records. See Administration →
Server settings → HTTPS certificate for download and trust instructions.

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

Use **Account & security** for Profile, Password, Passkeys, Recovery, Browser sessions and Apps. Changing the password retains existing access. **Sign out other sessions and apps** keeps this browser, passkeys and recovery codes. Using a recovery code replaces browser/app access and the old code batch, retaining passkeys; save replacement codes afterward. For local account recovery, see the command reference below.

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

## Operations and backups

The examples below use the built binary; place `--config /absolute/path/collector.toml` before any command when needed.

| Command | Purpose |
|---|---|
| `setup [--username NAME]` | Initialize storage, TLS, the default account, and local browser configuration. Run while stopped. |
| `serve` | Run listeners and background workers. Stop with Ctrl-C or SIGTERM on Unix. |
| `doctor` | Check configuration and optional encoders without locking the archive; can run while serving. Does not test provider reachability, passwords, grants, or browser certificate trust. |
| `reindex` | Rebuild the catalog from archive records. Stop the server first. |
| `backup --destination PATH` | Lock the stopped installation, settle journals/checkpoint SQLite and write a verified complete backup to a new directory. |
| `validate-backup --backup PATH` | Check the sealed inventory, complete file set and every digest without opening databases or current configuration. |
| `restore --backup PATH --archive PATH --state PATH [--archive-only]` | Validate and copy into empty roots. `--config` names a new output configuration. Archive-only recovery requires a fresh identity and explicit grants. |
| `unowned-collections [--limit 50] [--cursor ID]` | List recovered collections with no Owner grant. Run while stopped. |
| `recover-collection-owner --collection-id ID --username NAME` | Explicitly assign the first Owner to an enabled account while stopped. Refuses replacing any existing Owner; repeating the same assignment leaves its revision unchanged. |
| `pairing` | Export server identity, public certificate, and endpoint data as JSON. Transfer it through a trusted channel; it contains no login credential and does not establish browser trust. Run while stopped. |
| `paths` | Show absolute configuration, account-storage, archive and public-certificate locations with existence status. Safe while serving; reads configuration only. |
| `accounts [--limit 50] [--cursor USERNAME]` | List sign-in usernames and enabled/admin/default status. Run while stopped. |
| `recover-account --username NAME` | Recover that sign-in username while stopped. Reads a new password, revokes browsers/apps/pending approvals/codes/passkeys, and preserves grants and enabled/admin/default status. |

These commands use `collector.toml` by default; select another configuration with `--config FILE` before the command. Account storage is the folder containing `private.sqlite` (normally `.collector/state`). Run `paths` to see its configured location. If configuration is damaged, `accounts --state PATH` and `recover-account --username NAME --state PATH` use that existing folder directly. They never create missing storage or open the archive. Recovery takes the username used for sign-in; no account ID is required.

The [account delivery ledger](docs/account-usability-delivery.md) documents schema 19, preserved credentials and the native account-keyed conversion.

### Full backup

Stop the server, then run:

```sh
collector-server --config /absolute/path/collector.toml backup --destination /backup/new-collector-backup
collector-server validate-backup --backup /backup/new-collector-backup
```

The destination must not exist; its parent must exist outside both storage roots.
Protect it like live private state. The completion marker is written last; failed
or interrupted backups must be retried at a fresh destination. Restart the source
only when the backup command has exited.

The state root includes pending uploads and work as well as SQLite databases and TLS keys. Do not copy live SQLite files as an ordinary filesystem backup. Keep inbox originals and external DAT source files in your backup plan too; those paths are outside the server roots.

Restore with the original server stopped:

```sh
collector-server --config /restore/collector.toml restore --backup /backup/new-collector-backup --archive /restore/archive --state /restore/state
```

The two roots must be absent or empty, their parents must exist, and the output
configuration must not exist. Review reported external paths and run `doctor`
before starting the restored server. Retire the original instance: full restore
preserves identity, accounts, grants and TLS keys, so never run both copies.
Interrupted roots are fenced from startup; retry with newly empty roots.
For a deliberate independent clone, use `--archive-only`, then `setup` for a fresh
identity. While still stopped, use `unowned-collections` to find its collection IDs,
then `recover-collection-owner --collection-id ID --username NAME` for each
collection you intend to claim. Sign in as that Owner to share access normally.
Existing Owners and disabled accounts cannot be bypassed through this command.
Missing original lockfiles are recreated when their writable directories survive;
live original locks still refuse restore. See the complete
[operator runbook](docs/local-alpha-installation.md#stopped-server-backup).

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
