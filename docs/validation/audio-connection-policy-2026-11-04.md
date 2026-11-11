# Operator audio connection policy — 2026-11-04

> Deployment identifiers are anonymized. Dates, source and artifact identities, outcomes, and qualification limits are retained.

## Delivery

Audio now defaults to the normal authenticated website connection, including
proxies/tunnels (`remote_media_allowed = true`). Collector does not detect
Cloudflare or model provider restrictions. Session and collection permissions
still authorize every read. Public exposure remains listener/router composition.
Existing explicit `remote_media_allowed = false` configurations retain their
separate-route choice; no user configuration is silently rewritten on upgrade.

Administration → Server settings → Audio connection exposes Require a separate
audio connection. A private, revision-fenced setting overrides the config default
and uses existing input-bound command receipts. It persists across restart without
changing archive records. Normal discovery omits alternate endpoints even if they
remain provisioned, so existing web/native clients use their signed-in/enrolled
connection without private probes. The saving tab clears its route cache; other
clients may need reconnecting. Active reads are not forcibly disconnected.
Separate mode retains private-first/direct-HTTPS selection and public-origin byte
refusal. Explicit direct LAN listeners retain media eligibility.

The remote-archive profile permits normal audio alongside the provisioned media
authority. It requires that authority section because its Compose stack starts
both processes; a fresh media volume otherwise has no service key. Its health
check uses the configured core HTTP listener, which starts after the configured
authority. Generic core-only installations need no separate service. Storage
identity, mount, filesystem and writable-root checks remain.

## Application host beta.21 installation — 17:27 JST

Signed source `280b04f` is now published and installed on the application host as beta.21. Normal audio
remains selected (`remote_media_allowed = true`, `local_playback_enabled = false`);
the original `[remote_media]` endpoint is restored without changing other parsed
config values. The installed admin toggle can therefore reenable separate audio
without editing config. Identity, private state, originals, credentials and the
existing Bridge/Caddy/tunnel topology are preserved. Full source/image acceptance,
matched stopped-writer snapshots, anonymous 401 checks and exact public/LAN HTTPS
asset checks pass. Authenticated live playback on the application host was not retested; real playback
is covered by the isolated source/release-image journeys. See [the final operation
record](container-beta21-publication-2026-11-04.md) for immutable digests and evidence.

## Initial application host configuration acceptance — beta.20, 14:36 JST

The application host retains release `0.1.0-beta.20`, source `253e37e`, runtime image
`sha256:36b95f5a7c731d9cff5d5fb1c1664819d520875c748d5e56ad4206d6c72572d5`.
No new source/image was deployed. On this older build, enabling normal audio and
retaining `[remote_media]` is rejected, so the active config now sets
`remote_media_allowed = true`, `local_playback_enabled = false`, and omits that
table. Other parsed configuration values were checked unchanged before atomic
replacement. Exact original config is retained at
`/srv/junk-collector/config/collector.separate-audio.toml`. The media containers,
DNS and TLS provisioning remain available for later tests; the unused endpoint
has no active core authority in this configuration.

The initial core-only Docker restart stopped the systemd-owned Compose stack.
Its authority-socket health check also could not qualify normal mode. The stack
was restored through the host systemd manager after applying the optional-authority
preflight and HTTP readiness check. Both old deployment files were retained beside
the originals with `before-normal-audio-20261104` names. This was transient downtime,
not a successful initial restart. No database, credential, archive or original was
modified by the configuration tools. The same release image remains installed.

At **2026-11-04 14:36 JST**, the host service is active, core is healthy and all
five stack containers are running. Jobs remain 6,560 completed, 240 cancelled and
11 failed, with no unfinished work before or after restart. Trusted public HTTPS
returns HTTP 200 with the same server identity; unauthenticated route discovery and audio
requests return 401. Authenticated production playback is not claimed from these
checks; real signed-in playback/range acceptance belongs to the isolated source
fixtures below. The new admin toggle requires a future release installation.

To test the old separate configuration before that release, inspect the retained
config for unrelated edits before restoring it and restart
`junk-collector-remote.service` through systemd. Do not restart a constituent
container independently. The retained preflight/Compose changes support either
mode and do not need reverting. With the new release, keep the endpoint configured
and use the admin toggle instead.

## Validation

The complete diff selects `infra`: workspace Rust/Clippy, web types/build/models,
API contracts, architecture/format, tooling and all 30 browser journeys. These
checks are covered by the runs below; this dirty source delivery is not an
exact-source release qualification receipt.

- All Rust test binaries have passing coverage across the workspace run and
  corrected/remaining-suite reruns. The workspace selector's original run remains
  recorded as failed: the new receipt test incorrectly expected `Conflict` for
  changed inputs under a reused key. The actual contract is `IdempotencyConflict`.
  After correcting that assertion, all seven `job_dispatch` tests, eight
  `storage_guard` tests, five `wave2_regressions` tests and workspace doc tests pass.
  Previously passing unchanged workspace suites were retained, including 311
  archive tests. Five existing Rust ignores remain; no ignored oracle/scale
  acceptance is claimed.
- Focused current-source HTTP, private/native TLS and isolated-media tests pass.
  They cover normal and private native range readers, public separate-mode refusal,
  normal-mode exact ranges/HEAD, anonymous 401, cross-collection 403 and omitted
  alternate endpoints despite retained configuration. The old private-only native
  expectation is now explicitly selected and both routing choices are exercised.
  Store tests cover default selection, admin denial, stale edits, immutable receipt
  inputs/replay, restart persistence and saved choices overriding changed defaults.
- Strict workspace Clippy, format, architecture/document links, generated API
  contracts, web types/build and all three web model suites pass. Tooling,
  container/preflight, release-policy, container-validation and mail-parser checks
  pass. The existing rootless-skopeo container-validation case remains skipped on
  this host; no new container/image acceptance is inferred from it. The rebuilt
  executable's doctor output correctly reports the normal-audio configuration
  default and saved override behavior.
- All **30 browser journeys pass** outside the macOS seatbelt context. The changed
  remote-media journey uses real FLAC/MP3 audio and the actual core/separate media
  executables: default signed-in same-path playback, saved/reopened admin choice,
  route change without page reload, separate playback/seek and protected control
  refusal. Mobile/desktop screenshots were inspected; the control wraps without
  overflow and retains its accessible name. Normal playback does not construct a
  private transport. Existing session revocation and detached-source release pass.

Early fixture compile errors and the intentionally changed default-route
expectation were corrected and rerun; they are not successful validation evidence.
The original failed selector receipt and subsequent command logs remain under
`/tmp/audio-connection-*` for this working session. Release images, publication,
iPhone/Safari/cellular and authenticated live playback on the application host were not run or claimed.

## Release review — 2026-11-04

The review found that optional authority admission was incorrect for the existing
remote-archive profile: it always starts media, whose first startup needs the
service key. Retaining a previously generated key on the application host had hidden the fresh-volume
failure. Preflight now requires the provisioned authority regardless of route
preference, while allowing normal audio. Offline admission coverage checks normal
mode with configured authority, wrong sockets and absent authority. During the beta.21
update, the application host regained its original endpoint section without enabling separate
routing. Later admin toggling is now possible without a config edit.
No UI/API authority, revision/receipt, role/exposure or media-read finding remains
from the review. Release qualification, publication and the application host update completed successfully; see the final operation record above.
