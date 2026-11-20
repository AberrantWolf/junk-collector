# Junk Collector

A self-hosted archive for your games and music. Keep original dumps and files,
organize collections, find metadata and artwork, and play supported audio in the
browser or in Trash Amp. Each collection holds either games or music.

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
| Back up or recover | [Docker recovery](deploy/container/recovery.md) · [Server operations](docs/server-operations.md) |
| Fix a common problem | [Troubleshooting](docs/getting-started.md#troubleshooting) |

All documentation is listed in the [documentation index](docs/index.md).

## Features

- Keep original files alongside metadata, artwork and verification results.
- Import complete dump packages, or FLAC, MP3 and PCM WAV files.
- Identify games against No-Intro/Redump DAT catalogs and check CDs with AccurateRip.
- Prepare CD audio for browser playback and Trash Amp.
- Share collections with Viewer, Curator and Owner roles.

See [implementation status](docs/implementation-status.md),
[audio formats](docs/audio-preparation.md) and [remaining work](docs/remaining-work.md).

## Releases

Container images are published to `ghcr.io/aberrantwolf/junk-collector` for each
[GitHub release](https://github.com/AberrantWolf/junk-collector/releases), tagged
with the version. `preview` follows prereleases and `latest` follows stable
releases. Each release also attaches an installation bundle pinned to its image.

This is prerelease software: storage and interfaces can still change. Make a
backup before updating and read the release notes.

## Development

Read the [architecture](docs/architecture.md) and [AGENTS.md](AGENTS.md) first.

```sh
npm --prefix web ci
npm --prefix web run check
npm --prefix web run build     # web assets are embedded in the server binary
cargo build -p collector-server
```

See [testing](docs/testing.md) for the full check suite.

## License

MIT. Third-party dependencies keep their own licenses.
