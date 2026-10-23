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

## Beta.5

[`v0.1.0-beta.5`](https://github.com/AberrantWolf/junk-collector/releases/tag/v0.1.0-beta.5)
adds isolated direct HTTPS music delivery, browser Now Playing metadata and a LAN
discovery repair. Login/control can stay on Cloudflare while music uses a separate
endpoint. Docker images support
**linux/amd64** and **linux/arm64**; publication and anonymous pulls passed.
Synology DS1522+ uses AMD64 automatically; the permanent NAS update passed.
See [publication and deployment evidence](docs/validation/container-beta5-publication-2026-10-23.md).

**Back up before updating.** This beta targets personal home-lab use. Storage and
interfaces may change. Read the [release notes](docs/releases/v0.1.0-beta.5.md)
for media setup, security boundaries and update requirements.

## What it supports

- Keep original files alongside metadata, artwork and recorded evidence.
- Import complete dump packages or supported FLAC/MP3 purchases.
- Prepare supported CD audio for browser playback and Trash Amp.
- Share collections with Viewer, Curator and Owner roles.
- Check supported game catalogs and AccurateRip results with explicit coverage.

See [current capabilities](docs/implementation-status.md),
[audio formats](docs/audio-preparation.md) and the [roadmap](docs/remaining-work.md).
Hashing, structural checks and descriptive metadata are separate from catalog verification.

## Development

Read the [architecture](docs/architecture.md) before changing a subsystem.
[AGENTS.md](AGENTS.md) lists required checks.

```sh
# Rebuild the web assets, then embed them in the server.
npm --prefix web ci
npm --prefix web run check
npm --prefix web run build
cargo build -p collector-server
```

The [browser smoke test](web/tests/smoke.mjs) uses a temporary installation.
Set `CHROME_PATH` if Chrome is outside the default macOS location.

## License

MIT. Third-party dependencies retain their respective licenses.
