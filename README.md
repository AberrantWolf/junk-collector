# Junk Collector

Current delivery target: [local web alpha](docs/local-alpha-plan.md) — guided raw-dump
imports, automatic metadata, browsing, CD playback and trusted local users. The
integration is planned; [implementation status](docs/implementation-status.md)
distinguishes current capabilities from remaining work.


A homelab collection archive for physical games and music CDs, their original dumps, artwork, and verification evidence.

**Development foundation — not yet the complete first release.** The server, portable archive, upload/thumbnail workflows, authentication, and artwork/text WebUI are executable. Native apps, shared audio, catalog verification and several curation workflows remain unfinished. See [implementation status](docs/implementation-status.md) and [architecture](docs/architecture.md). Remaining work is organized in the [handoff backlog](docs/handoff-backlog.md), with the [accepted release requirements](docs/release-requirements.md) preserved alongside it.

## Run locally

Requires Rust 1.95+; Node 22+ is needed only when changing the WebUI. Built web assets are included, so the server has no Node runtime requirement.

```sh
cargo build --workspace
# Prompts for a password without echoing it. Existing accounts are preserved.
cargo run -p collector-server -- setup
cargo run -p collector-server -- serve
```

Open the exact local management address printed by setup—by default **http://localhost:9842**—on the server computer. Sign in with the password from setup, choose **Create collection** (or open **Administration**), enter a name and choose its collection type, then choose **Add item**. Attach artwork and original files after saving the inventory item. The public and direct-LAN addresses are client surfaces by default and cannot perform initial management.

`setup` creates `collector.toml`, archive/state directories, a local TLS identity and the default account. Re-running it repairs the local browser origin while keeping accounts, passwords, data, exposure settings and existing certificates. It backs up an existing configuration before rewriting it. Stop the server before running setup. Passwords can also be piped through stdin; never put them in command arguments. `init` remains available for scripted first-time initialization.

For an existing installation showing access errors:

```sh
# Stop the running server with Ctrl-C first.
cargo run -p collector-server -- setup
cargo run -p collector-server -- doctor
cargo run -p collector-server -- serve
```

`doctor` checks configuration without acquiring the archive lock, so it also works while the server runs. An `origin_not_allowed` error means the browser address is missing from `allowed_origins`; it is distinct from an expired session and from a collection-role denial. Use `localhost` exactly for this local setup—`127.0.0.1` is a different browser origin. If the browser cannot retain a Secure cookie on localhost, the UI reports that explicitly; use trusted HTTPS as described below. Local HTTP onboarding is covered by the Chromium smoke test; other browsers remain to be qualified and may require trusted HTTPS.

### Existing collections without an assigned type

Older collections stay readable and retain their files. Before adding more inventory,
stop the server and inspect the collection using its ID from the collections API
(`GET /api/v1/collections` while signed in):

```sh
cargo run -p collector-server -- inspect-collection-type --collection-id <COLLECTION_ID>
# Use the revision and an eligible module from that report; choose explicitly.
cargo run -p collector-server -- assign-collection-type --collection-id <COLLECTION_ID> --expected-revision <REVISION> --module org.junk.music
# For games, the built-in module is org.junk.games.
cargo run -p collector-server -- serve
```

Both inspection and assignment require exclusive store access. Add `--config PATH`
before the subcommand when using a non-default configuration. Assignment preserves
the original manifest and records a new revision. Mixed or unavailable item types,
incompatible pending work, or invalid active metadata block assignment; inspection
reports the reason. Splitting mixed collections is not implemented. Historical
metadata overrides are preserved without automatically activating them. Full
application/job write gating and legacy UI notices are still being completed in
CT.3–CT.4; graph publication already rejects unassigned collections.

Default listeners:

| Address | Purpose |
|---|---|
| `127.0.0.1:9841` | Public HTTP origin for a tunnel/proxy; client routes only by default |
| `127.0.0.1:9842` | Private management HTTP origin for a private proxy/Tailscale Serve |
| `0.0.0.0:9843` | Direct LAN HTTPS client endpoint using the enrolled local certificate |

For direct private management HTTPS, set `private_tls_bind = "192.168.1.20:9844"` to an explicit LAN/tailnet interface. This listener includes management routes regardless of public exposure. Its certificate must be trusted by the browser; wildcard bindings are rejected.

For browser use, configure a browser-trusted HTTPS proxy and add its exact origin to `allowed_origins`. Point public Cloudflare Tunnel traffic at port 9841 and private Tailscale Serve at port 9842. Do not publish the private origin through a public tunnel. Browser sessions use Secure cookies; remote browser access requires HTTPS. Localhost is the tested local-browser exception. Do not bypass certificate checks. A locally trusted CA or Tailscale HTTPS is suitable for private management.

Set `authentication_origin = "https://your-authentication-hostname"` to enable WebAuthn passkeys, and include that origin in `allowed_origins`. Registration and login must happen on that origin. The browser must support the standard WebAuthn JSON conversion APIs. Registration/login pass a Chromium virtual-authenticator test; real password-manager/hardware validation remains a release gate.

The native client trust helper accepts a certificate from trusted pairing and connects directly over LAN. Export enrollment material with:

```sh
cargo run -p collector-server -- pairing > pairing.json
```

Transfer the pairing file through a trusted channel. A pairing file contains a public certificate, not an authentication token. The certificate's `tls_names` must cover the local names/IPs you use; configure these before `init`. Automatic discovery and key rotation are not yet implemented.

## Configuration

```toml
archive = "/srv/junk-collector/archive"
state = "/var/lib/junk-collector"
public_bind = "127.0.0.1:9841"
private_bind = "127.0.0.1:9842"
lan_bind = "0.0.0.0:9843"
multi_user = false
tls_names = ["junk-collector.local", "192.168.1.20", "localhost", "127.0.0.1"]
endpoints = ["https://junk-collector.local:9843", "https://192.168.1.20:9843"]
allowed_origins = ["https://collector.example.net", "https://collector.example-tailnet.ts.net"]
authentication_origin = "https://collector.example.net"

[exposure]
public_curation = false
public_administration = false

# Optional. Paths or executable names are probed at server startup and by doctor.
[encoders]
flac = "/usr/bin/flac"
# opus = "/usr/bin/opusenc"

# Optional MusicBrainz and Cover Art Archive enrichment. MusicBrainz requires a
# meaningful versioned application name and a real maintainer contact.
[providers]
musicbrainz_user_agent = "JunkCollector/0.1.0 (you@example.com)"
```

Administration exposure implies curation exposure; neither grants a role. Disabled management routes and JavaScript entrypoints are absent from the public router. Proxy headers never grant private status. Exposure changes currently require editing configuration and restarting.

Edit the existing `[encoders]` section (or create it if absent), set the path to
your installed tool, then restart the server. Open a CD item’s editor and use
**Music preparation → Prepare FLAC**; the completed track becomes a download link.
Close the editor to use the **Disc player** in item details. Viewers can play
prepared tracks without curation access; playback currently stops when details close.

Music derivatives are prepared only when a curator requests one. FLAC is the
default choice and Opus is available at explicit 96/128/192-kbps presets when its
encoder is configured. Missing tools do not block import, preservation, analysis,
or original downloads, and neither import nor a media request starts conversion.

MusicBrainz metadata and Cover Art Archive acquisition are optional. If
`providers.musicbrainz_user_agent` is absent, preservation, imports and manual
metadata continue normally; provider Jobs report the missing provider rather than
inventing results. Configure a real contact before enabling it. Provider responses
and images are bounded and archived as immutable observations; MusicBrainz identity
is descriptive metadata and never audio verification.

## Development

```sh
npm --prefix web ci
npm --prefix web run check
npm --prefix web run build
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

The public app and management entrypoints are separate bundles. Rebuild the Rust binary after changing embedded assets. Domain/core libraries cannot depend on database, networking or UI crates.

## Server inboxes

Configure named inbox roots in `[inboxes]`, for example `rips = "/srv/incoming"`. Curators can list safe package names through the collection-scoped Import API and stage a direct child directory into the unified review workflow; configured roots are never returned. The curation workbench selects browser files or a configured inbox package, shows its inventory/progress and honest analyzer limitations, records catalog/physical review, and publishes the capture. Its submitter-scoped background-work ledger shows active/history progress and offers server-authorized cancel/retry controls, polling every two seconds while work is active and every 30 seconds while idle. The deprecated administrator endpoints remain compatibility adapters: `GET /api/v1/admin/inboxes` lists configured names and `POST /api/v1/admin/inboxes/{name}/import`, with `item_id`, `expected_revision`, and `package`, creates a provisional Import plus staging Job that still requires review before publication. Sources are retained; symlinks and special files are rejected. The `/api/v1/uploads` surface is also a compatibility adapter and is scheduled with the administrator inbox surface for removal in a future API major.

## Game DAT catalogs

Configure local catalog files by safe source name; source paths stay server-side:

```toml
[dat_sources.nes]
path = "/srv/catalogs/nes.dat"
system = "nes"
catalog = "My NES DAT"
byte_domain = "org.junk.game.raw-bytes"
byte_domain_version = 1
```

The private administration panel imports exact DAT bytes as immutable snapshots
and selects the active snapshot for future analysis. Curators can run explicit
game hashing, structure, track and catalog operations against preserved captures;
old results remain tied to their original source bytes and snapshot after refresh.

For a manual, rate-limited cache of the currently supported Redump/No-Intro
systems, run:

```sh
python3 scripts/cache_game_dats.py /path/outside/the/repository --catalog <key>
```

The script lists its allowlisted keys in `--help`, preserves raw
provider ZIP/DAT bytes beside a digest manifest, and never performs automatic
server refresh. See the [LA3 installation runbook](docs/local-alpha-installation.md)
for filter, rate-limit and provider-refusal details. New DAT-o-MATIC downloads
are currently paused after a provider refusal; existing verified caches and local
DAT imports remain usable. Refreshes use a new cache root to retain old snapshots.

## Recovery

For the bounded local-alpha installation and stopped-server copy/restore drill, use
[the LA3 installation runbook](docs/local-alpha-installation.md). It covers the
archive, complete private-state root (including TLS and pending scratch),
configuration, digest manifests and the prohibition on running original and restored
copies of one server identity concurrently. It is not the planned P05 portable
export/change-bundle workflow.

```sh
# Stop the server first: one process owns archive writes.
cargo run -p collector-server -- reindex
cargo run -p collector-server -- reset-password --username owner
```

Reset reads the new password from stdin and revokes device sessions. It does not remove passkeys. Back up the archive and private state together. Archive-only reconstruction does not restore users or permissions. Stop the server before copying SQLite files, or use SQLite-aware backup tooling.

The archive retains original filenames and immutable metadata revisions. Sources are not consumed. Unfinished staging and upload data are retained for recovery; automatic reclamation is not yet implemented.

## License

MIT. Third-party dependencies retain their respective licenses.
