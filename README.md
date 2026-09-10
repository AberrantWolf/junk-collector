# Junk Collector

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

Open the exact local management address printed by setup—by default **http://localhost:9842**—on the server computer. Sign in with the password from setup, choose **Create collection** (or open **Administration**), then choose **Add item**. Attach artwork and original files after saving the inventory item. The public and direct-LAN addresses are client surfaces by default and cannot perform initial management.

`setup` creates `collector.toml`, archive/state directories, a local TLS identity and the default account. Re-running it repairs the local browser origin while keeping accounts, passwords, data, exposure settings and existing certificates. It backs up an existing configuration before rewriting it. Stop the server before running setup. Passwords can also be piped through stdin; never put them in command arguments. `init` remains available for scripted first-time initialization.

For an existing installation showing access errors:

```sh
# Stop the running server with Ctrl-C first.
cargo run -p collector-server -- setup
cargo run -p collector-server -- doctor
cargo run -p collector-server -- serve
```

`doctor` checks configuration without acquiring the archive lock, so it also works while the server runs. An `origin_not_allowed` error means the browser address is missing from `allowed_origins`; it is distinct from an expired session and from a collection-role denial. Use `localhost` exactly for this local setup—`127.0.0.1` is a different browser origin. If the browser cannot retain a Secure cookie on localhost, the UI reports that explicitly; use trusted HTTPS as described below. Local HTTP onboarding is covered by the Chromium smoke test; other browsers remain to be qualified and may require trusted HTTPS.

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
```

Administration exposure implies curation exposure; neither grants a role. Disabled management routes and JavaScript entrypoints are absent from the public router. Proxy headers never grant private status. Exposure changes currently require editing configuration and restarting.

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

## Recovery

```sh
# Stop the server first: one process owns archive writes.
cargo run -p collector-server -- reindex
cargo run -p collector-server -- reset-password --username owner
```

Reset reads the new password from stdin and revokes device sessions. It does not remove passkeys. Back up the archive and private state together. Archive-only reconstruction does not restore users or permissions. Stop the server before copying SQLite files, or use SQLite-aware backup tooling.

The archive retains original filenames and immutable metadata revisions. Sources are not consumed. Unfinished staging and upload data are retained for recovery; automatic reclamation is not yet implemented.

## License

MIT. Third-party dependencies retain their respective licenses.
