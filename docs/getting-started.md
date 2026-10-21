# Getting started

[← README](../README.md) · [Docker installation](../deploy/container/README.md) · [Configuration](configuration.md)

## Build from source

- Rust 1.99.1 (the tested release toolchain, pinned in `rust-toolchain.toml`), a C/C++ build toolchain, CMake, and pkg-config. Linux builds also need OpenSSL development headers.
- Network access on the first build to fetch Cargo dependencies, including the exact `junk-libs` Git revision pinned in `Cargo.toml`. No sibling checkout is required.
- Writable storage for the archive and private server state.
- Optional: `flac` for lossless CD preparation; `opusenc` for smaller audio copies.

Built web assets are included. Node.js 22+ is needed only to rebuild the WebUI, not to run the server.

The pinned version is the tested release baseline; a lower minimum supported version
has not been established. See [release toolchain maintenance](release-toolchain.md).

### Initialize the server

Run these commands from the repository root. A release build is recommended for normal use, especially hashing and audio processing.

```sh
cargo build --locked --release -p collector-server
./target/release/collector-server setup
./target/release/collector-server doctor
./target/release/collector-server serve
```

Startup reuses an unchanged catalog. A missing or damaged catalog is rebuilt from the archive.
See [catalog recovery and measurements](validation/p06-indexing.md) for details.

`setup` prompts for a password of at least 12 characters and creates:

- `collector.toml`, the server configuration;
- `.collector/archive`, containing saved files and collection records;
- `.collector/state`, containing databases, accounts, TLS identity, jobs, and temporary work;
- the initial administrator account, named `owner` by default.

To choose a different account name, use `setup --username YOUR_NAME`. Passwords can also be supplied through standard input; do not put them in command arguments.

Open **http://localhost:9842** on the server computer and sign in. Use `localhost` exactly: `127.0.0.1` is a different browser origin. Local HTTP sign-in is tested with Chromium-based browsers; use browser-trusted HTTPS for remote browsers.

For permanent storage locations or custom certificate names, create `collector.toml` using the [configuration reference](configuration.md) **before running setup**. Supply a custom configuration path before the command:

```sh
./target/release/collector-server --config /absolute/path/collector.toml setup
./target/release/collector-server --config /absolute/path/collector.toml serve
```

Run the server as an ordinary user with access to its storage and configured inboxes. Only one server process may write an archive.

## Add your first collection

1. Sign in at a management-enabled address. Choose **Create collection**, then games or music.
2. Open **Imports**. Select the collection and **Browser folder / files** or **Server inbox**.
3. Review each package under **Files and analysis**. Use the options below when needed.
4. Choose **Start import**. Keep the page open until browser uploading finishes.
5. Open the item to see files, metadata, artwork and evidence.

### Import options

| Option | Use |
| --- | --- |
| Folders | Discover packages. For server inbox directories, choose **Review packages**. |
| Individual files | Assemble one package. Unassigned files appear separately. |
| Analysis anchor | Choose the intended CUE or Redumper anchor when several exist. Review shared files. |
| Album group | Give related discs the same label. Leave manual title empty for automatic metadata. |
| Physical copy label | Matching labels within an album mean one purchased copy. Different labels mean separate copies, not disc numbers. |
| Metadata/artwork | Acquired automatically when available. Manual edits remain authoritative. |
| Prepare FLAC after import | Optional for supported CDs. FLAC/MP3 purchases play directly without physical-copy records. |
| Interrupted upload | Reopen the saved batch and reselect its original folder/files to resume. |
| Edit / attach files | Change metadata, copies and loans, add images or attach another capture. |
| Add / import → manual entry | Record inventory without files. |

**Keep complete dump packages together.** BIN/CUE sets need their referenced BIN files.
Supported Redumper CDs need TOC, SCRAM, STATE and SUBCODE companions.
Preservation can succeed even when identification or verification is unavailable.

Intake supports **10,000 packages / 100,000 files**. Split larger folders at package boundaries.
Known Synology/macOS housekeeping is excluded; other hidden and unknown files stay.

Attaching a disc to an existing album removes its unused automatic placeholder from browsing.
Entries with meaningful inventory or manual information stay. Archived files and history are retained.

### Play music

Supported FLAC/MP3 purchases play directly after import. For CDs, configure the
[FLAC encoder](configuration.md#audio-encoders), open **Prepare audio**, select albums, preview the
FLAC preparation and start it. Individual tracks and configured Opus profiles are
also available in the editor's music preparation view. Preparation is explicit
background work; prepared files are saved automatically.
See the [audio support matrix](audio-preparation.md) for qualified tools,
profiles, clients and remaining limits.

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
