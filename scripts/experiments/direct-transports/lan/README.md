# Disposable NAS WebTransport player experiment

This is a dated bounded experiment, **not the automatic LAN feature or an installer**.
The production delivery now has its own `web/tests/local-playback.mjs` integration
check. This experiment imported the player before that delivery; rebuilding it
against the new player also imports the production broker and is not a replay of
the recorded feasibility test. Use the production test for current regression checks.
It imports the actual Collector `MusicPlayer` and queue unchanged. The surrounding
page signs into a separate real Collector; its service worker adapts the player's
HEAD/GET/Range requests to a page-owned WebTransport session. No full-file Blob,
custom audio decoder or HTTP media fallback is used. The fixture adds CORS mode
to the mounted audio element, following the browser's service-worker media guidance.

The public-origin follow-up also listens on **loopback only** TCP 19980, accepting
only `/__collector_lan_test_20261022/` and descendants at the exact public origin.
It needs an explicitly approved, path-limited tunnel rule before the existing
catch-all. Assets/control URLs and worker scope follow the path automatically;
the separate public test cookie is restricted to it. Do not expand the worker to
the root scope or forward production cookies to the test Collector. The fixture
Collector must also allow the exact public origin. Media grants bind their
origin as well as the real session and immutable representation.

The page records supported permission states and waits for an explicit connect
action before any LAN handshake. `?block-wt=1` serves a strict self-only connection
CSP for a browser negative control. `check.py --public-ip PUBLIC_IP` can route only
the probe's public-host lookup to its public DNS answer while preserving TLS SNI
and verification; this does not change system/browser DNS. Cloudflare currently
rejects that Python client with error 1010; leave browser-integrity protection
enabled and qualify actual browser behavior instead. See
[public-origin evidence](../../../../docs/validation/public-origin-webtransport-2026-10-22.md).

The source deliberately names the owner's existing NAS/test origin. Do not point
the fixed loopback backend at production. No Tailscale tool, discovery, setting or
integration is used. See the dated validation record for what passed and what did
not; a playing label alone is not proof of advancing or audible playback.

## Boundaries

- HTTPS page/control: `192.168.1.106:19978`, using the existing certificate for
  `collector-lan.awkwardexplorer.com`. Its private key is mounted read-only from
  the existing NAS certificate directory, never copied into the image/repository.
- WebTransport media: UDP `192.168.1.106:19979`, generated seven-day P-256 identity.
  The browser obtains its hash over trusted test HTTPS; incorrect hashes fail.
- Separate Collector: public `127.0.0.1:19981`, private `127.0.0.1:19982`, native
  TLS `127.0.0.1:19983`, separate archive/state/config and synthetic audio.
- The HTTPS proxy accepts only test login/logout/me and diagnostic/control routes.
  Media over HTTPS returns 410. Its external `collector_wt_session` cookie is
  renamed to/from Collector's cookie so production cookies on the same hostname
  are neither forwarded nor overwritten (cookies are not port-scoped).
- Opaque 120-second in-memory grants permit only the seeded media coordinates.
  Every read still calls the real Collector with that session: expired/revoked
  sessions and missing collection grants are denied by the actual application.
  Exact CONNECT/HTTP mutation Origin checks, GET/HEAD-only media, maximum 256 KiB
  ranges, eight active requests per connection, timeouts and no redirects bound
  the mechanism. These are experiment controls, not a reviewed production lease
  protocol. No new credentials or private keys are logged.
- The bridge defaults to two hours and deletes its generated WebTransport key.
  `EXPERIMENT_LIFETIME_SECONDS` can explicitly select 60–86,400 seconds; use the
  same bound for the separately launched Collector's process timer. The operator
  follow-up restarted both with an eight-hour window for manual work-Mac testing.
  Both use named disposable containers, no restart policy, memory limits and
  dropped capabilities. No production mounts or router/tunnel changes.

## Reproduction outline

Prepare a fresh private fixture directory outside the Docker build context with
`collector.toml`, a random test-only `password` file (mode 600), and `noise.wav`.
Use a 180-second, low-amplitude stereo PCM WAV, for example:

```sh
ffmpeg -f lavfi -i 'anoisesrc=color=pink:amplitude=0.02:sample_rate=44100:duration=180' \
  -ac 2 -c:a pcm_s16le /tmp/noise.wav
node scripts/experiments/direct-transports/lan/build.mjs /tmp/app.js
```

Copy only `server.py`, `seed.py`, `index.html`, `sw.js`, `requirements.txt`,
`Dockerfile` and the generated `app.js` into a clean image build directory. Build
that directory; keep credentials, state, archive and certificates outside it.
Use the same tested Collector image as the installed version, with an entirely
separate `/fixture` mount and the loopback ports above. Configuration needs:

```toml
archive = "/fixture/archive"
state = "/fixture/state"
public_bind = "127.0.0.1:19981"
private_bind = "127.0.0.1:19982"
lan_bind = "127.0.0.1:19983"
authentication_origin = "https://collector-lan.awkwardexplorer.com:19978"
allowed_origins = ["https://collector-lan.awkwardexplorer.com:19978", "http://localhost:19982"]
tls_names = ["localhost", "127.0.0.1"]
endpoints = ["https://collector-lan.awkwardexplorer.com:19978"]
[exposure]
public_curation = false
public_administration = false
```

As the fixture owner, run `junk-collector --config /fixture/collector.toml setup
--username listener` with the fixture password on stdin, then serve with that
configuration. Run `seed.py` with `/fixture` mounted and host networking. It uses
only the fixed isolated private loopback listener to create the real collection,
an account named `denied` without a collection grant, an item and a resumably
uploaded original. It writes `/fixture/bridge/fixture.json` after publication.

Start the Python image with host networking, `/fixture/bridge` writable by its
UID, existing test-origin HTTPS cert/key directory mounted read-only at
`/nas-cert`, and no production archive/state mounts. `fullchain.pem` and
`privkey.pem` are expected there. Do not use certificate exceptions or disable
TLS verification. Open the test HTTPS URL and use the test-only password.

For an independent byte/authorization probe, obtain only `wt-cert.pem` over the
authenticated NAS SSH connection, then run in an environment with the pinned
dependencies:

```sh
python check.py --certificate /tmp/wt-cert.pem --password-file /tmp/test-password \
  --audio /tmp/noise.wav
```

The probe checks that the SSH public certificate matches the trusted HTTPS
descriptor, validates TLS against that exact certificate, compares three ranges
with the source bytes, rejects forbidden requests, revokes its own real session
and confirms cross-collection denial and wrong-Origin rejection. It never disables
certificate validation. It is independent of the browser's audio-output path.

Stop `collector-wt-lan-bridge` and `collector-wt-lan-backend` to end the current
fixture early. Remove only those disposable containers and their explicitly named
fixture directories when no longer needed; do not touch production services.
