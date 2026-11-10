# Junk Collector

**Keep your games and music in one self-hosted archive.** Save original dumps,
organize collections, find metadata and artwork, and play supported audio.
Each collection contains either games or music.

## Start here

| I want to… | Go to |
| --- | --- |
| **Install on Docker or Synology** | [Container installation](deploy/container/README.md) |
| Build the server myself | [Source setup](docs/getting-started.md#build-from-source) |
| Import my first collection | [Import guide](docs/getting-started.md#add-your-first-collection) |
| Listen in the browser | [Playback](docs/getting-started.md#play-music) |
| Connect Trash Amp | [Connect the app](docs/getting-started.md#connect-trash-amp) |
| Add users or configure HTTPS | [Configuration and access](docs/configuration.md) |
| Update a container | [Update steps](deploy/container/README.md#update) |
| Back up or recover | [Docker recovery](deploy/container/recovery.md) · [Server commands](docs/server-operations.md) |
| Fix a common problem | [Troubleshooting](docs/getting-started.md#troubleshooting) |

## Releases

Use a versioned image or digest from the [published releases](https://github.com/AberrantWolf/junk-collector/releases).
The installation bundle pins its validated image. `preview` follows prereleases;
`latest` is reserved for stable releases. Source candidates are not publication evidence.
See [current capabilities and release evidence](docs/implementation-status.md).

Keep a complete consistent recovery point before updating. Prerelease interfaces
and storage can change; follow the selected release's upgrade and recovery notes.

## What it supports

- Keep original files alongside metadata, artwork and recorded evidence.
- Import complete dump packages or supported FLAC/MP3 and integer PCM WAV originals.
- Prepare supported CD audio for browser playback and Trash Amp.
- Share collections with Viewer, Curator and Owner roles.
- Check supported game catalogs and AccurateRip results with explicit coverage.

See [current capabilities](docs/implementation-status.md),
[audio formats](docs/audio-preparation.md) and the [roadmap](docs/remaining-work.md).
Hashing, structural checks and descriptive metadata are separate from catalog verification.

## Development

Use the [documentation index](docs/index.md) to find current guidance and evidence.
Read the [architecture](docs/architecture.md) before changing a subsystem.
[AGENTS.md](AGENTS.md) lists required checks.

```sh
# Rebuild the web assets, then embed them in the server.
npm --prefix web ci
npm --prefix web run check
npm --prefix web run build
cargo build -p collector-server
```

The [short browser smoke test](web/tests/quick-smoke.mjs) uses a temporary installation.
Set `CHROME_PATH` if Chrome is outside the default macOS location.

## License

MIT. Third-party dependencies retain their respective licenses.
