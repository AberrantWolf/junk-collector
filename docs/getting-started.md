# Getting started

[← README](../README.md) · [Docker installation](../deploy/container/README.md) · [Configuration](configuration.md)

## Build from source

You need:

- the Rust toolchain pinned in `rust-toolchain.toml`, a C/C++ toolchain, CMake and
  pkg-config (plus OpenSSL headers on Linux);
- network access on the first build, to fetch Cargo dependencies including the
  `junk-libs` Git revision pinned in `Cargo.toml`;
- optionally `flac` (lossless CD preparation), `opusenc` (Opus copies) and `ffmpeg`
  (live playback conversion).

Built web assets are committed, so Node.js 22+ is only needed to rebuild the web UI.

```sh
cargo build --locked --release -p collector-server
./target/release/collector-server setup
./target/release/collector-server doctor
./target/release/collector-server serve
```

Use a release build for real use; hashing and audio processing are slow in debug builds.

`setup` asks for a password (at least 12 characters) and creates:

- `collector.toml`, the server configuration;
- `.collector/archive`, the saved files and collection records;
- `.collector/state`, databases, accounts, TLS identity, jobs and temporary work;
- the first administrator account, `owner` unless you pass `--username NAME`.

The password can also be piped on standard input. Never pass it as an argument.

Open **http://localhost:9842** on the server and sign in. Use `localhost` exactly;
`127.0.0.1` is a different browser origin. For other computers, use browser-trusted
HTTPS (see [configuration](configuration.md#access-from-another-computer)).

To use permanent storage paths or custom certificate names, write `collector.toml`
**before** running `setup`, and pass it before the command:

```sh
./target/release/collector-server --config /absolute/path/collector.toml setup
./target/release/collector-server --config /absolute/path/collector.toml serve
```

Run the server as an ordinary user with access to its storage and inboxes. Only one
server process may write an archive. Startup reuses an unchanged catalog and rebuilds
a missing or damaged one from the archive.

## Add your first collection

1. Sign in on a management address and choose **Create collection** (games or music).
2. Open **Add to library**, select the collection, and choose **Browser folder / files** or **Server inbox**.
3. Check the detected packages. Packages that need a decision are listed first;
   in large imports the rest stay in a compact list until you choose **Change**.
   Music collections show **Files and album playback**; other packages show
   **Files and analysis**.
4. Choose **Start import**. Browser uploads need the page open until they finish;
   inbox imports open **Review → In progress** for that import.
5. Use **Review** to see what is still running, what was added and what needs you.
   Open an entry there to fix its name, match, cover or import, or accept a reason
   as it is.

| Option | Use |
| --- | --- |
| Folders | Review discovered packages. Set **Folder layout** for a browser selection; inbox entries use **Review packages**. |
| Individual files | Assemble one package. Unassigned files are listed separately. |
| Analysis anchor | Pick the intended CUE or Redumper file when there are several. |
| Digital album playback | Choose tracks or a format, set the disc count and assign tracks to discs. All originals are kept. |
| Album group | Give related discs the same label. Leave the title empty to use automatic metadata. Packages named like `Album (Disc 1)` and `Album (Disc 2)` are suggested as one album; choose **Group as one album** or **Keep separate**. |
| Physical copy label | The same label within an album means one purchased copy; different labels mean separate copies. |
| Prepare FLAC after import | Optional for supported CDs. FLAC/MP3 purchases play directly. |
| Save originals only | Keep the files without analysis, lookups or audio preparation. |
| Interrupted upload | Reopen the unfinished import and reselect the same folder/files to resume. |
| Add / import → manual entry | Record inventory without files. |

Keep dump packages complete: BIN/CUE sets need their BIN files, and Redumper CDs need
the TOC, SCRAM, STATE and SUBCODE files. Files are preserved even when identification
or verification is unavailable. One intake can hold up to 10,000 packages / 100,000
files; split larger folders at package boundaries. Synology/macOS housekeeping files
are skipped; other hidden files are kept.

### Ripping a CD

With redumper installed:

```sh
cargo build --release --locked -p collector-cd-identity   # once, for automatic naming
./scripts/rip-cd.sh /path/to/dumps disk7 --auto
./scripts/rip-cd.sh /path/to/dumps disk7 "Album - Disc 1"  # fixed folder name, no lookup
```

The folder name (or `--auto`) comes last, so for the next disc you can recall the
command and change only the name. The middle argument is the drive (`diskN` on macOS,
`/dev/sgN` on Linux); omit it to use the first drive. Linux media must be unmounted. `RETRIES` (default 100) and
`REDUMPER=/path/to/redumper` are honoured. See `./scripts/rip-cd.sh --help`.

The helper writes `disc.*` raw files and logs into a new `cd-<timestamp>-<suffix>`
folder. It never refines: if the dump reports SCSI or C2 errors, it finishes with a
"check this disc" message and a `redumper refine` command you can run by hand after
cleaning or inspecting the disc. With
`--auto` it then spends up to 15 seconds on a MusicBrainz lookup and renames the folder
(for example `SQEX-11226`, with ` - Disc N` for multi-disc releases) only when exactly
one release matches; otherwise the generated name stays. Naming needs Python 3 and
`collector-cd-identity` on PATH or in `target/release` (override with
`COLLECTOR_CD_IDENTITY`). After a successful capture the disc is ejected.

Check `disc.log` for errors, then import the folder with `disc.toc` as the analysis
anchor. Q-subchannel errors alone are acceptable for ordinary music CDs; run
**Check audio with AccurateRip** to check the audio itself.

## Play music

Every supported original plays: the server converts CD audio and WAV while it
streams. Saved FLAC or Opus copies are optional extras. To make or manage them,
configure the [encoders](configuration.md#audio-encoders) and open **Saved formats**.
See [audio support](audio-preparation.md).

On an album choose **Play album**, **Play original** or a track's **Play**. Play replaces
your queue; **Add album to queue** appends. The player keeps running while you navigate.

The site follows the system color theme; **Account & security → Appearance** overrides it
for this browser.

## Connect Trash Amp

In Trash Amp, open **Settings → Servers**, enter the server URL and choose
**Connect / Reconnect**. For normal trusted HTTPS, leave **Owner-provided trust file**
empty. For a self-signed server, get its public certificate PEM or pairing JSON from the
owner, compare its SHA-256 with the server's startup output, and enter its path. Never
copy the private key or disable certificate checks.

Approve the app with the displayed link/code while signed in as the intended account.
The app gets its own revocable access. Trusting the certificate in Trash Amp does not
make browsers trust it.

## Troubleshooting

| Symptom | Check |
|---|---|
| Login reports `origin_not_allowed` | Open the exact configured address, or add its origin to `allowed_origins` and restart. |
| Management pages are missing | Use a private listener, or enable the public exposure setting. |
| "You do not have access to this resource" | Check the account's collection role. Exposure never grants permission. |
| Certificate warning | Use a hostname covered by a browser-trusted certificate. |
| Music preparation is unavailable | Run `doctor`, check encoder paths, restart after config changes. |
| Metadata or artwork is unavailable | Check `musicbrainz_user_agent` and the job's error. Manual editing still works. |
| An import or preparation failed | Read the job error and fix the cause before retrying. Keep the source package and its filenames intact. |

For development checks see [testing](testing.md).
