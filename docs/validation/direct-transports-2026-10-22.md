# Direct transport primitives — 2026-10-22

**Bounded feasibility evidence, not implemented automatic LAN playback.** The user
requested testing available approaches while minimizing private-network certificate
and network setup. Public hosting remains the operator's choice. No Tailscale
integration, access or configuration was used. The accepted manual LAN hostname
continues to work; this experiment made no NAS, router or production-server change.

## Environment and method

- Personal Mac: macOS 26.6.2 arm64; Firefox 157 and Chrome 155, using ordinary
  browser UI. Reduced user-agent OS strings were not treated as the real OS.
  Safari and the work Mac/GlobalProtect were not exercised in this experiment.
- Isolated CPython 3.14.6 environment with `aioquic==1.7.0` from PyPI. A temporary
  server bound only `127.0.0.1`: page/results on TCP 19978, WebTransport HTTP/3 on
  UDP 19979, and an ordinary HTTPS control on TCP 19980.
- A freshly generated seven-day P-256 self-signed certificate stayed in a private
  temporary directory. Its public SHA-256 hash came from the local fixture page.
  No browser flags, certificate exceptions, OS trust imports, DNS changes, STUN
  servers, TURN servers or external relay were configured.
- The page used actual browser APIs, not mocked transport responses. Source is
  retained as [server.py](../../scripts/experiments/direct-transports/server.py)
  and [browser.html](../../scripts/experiments/direct-transports/browser.html),
  outside production code. The portable copy adjusts the HTML location and adds
  graceful signal/key cleanup; its transport test logic is unchanged. Endpoint
  trust came from this owned loopback fixture; no real
  Collector authentication/signaling contract was tested.

## Observed results

| Check | Firefox 157 | Chrome 155 |
| --- | --- | --- |
| WebTransport with the supplied correct certificate hash | 65,536 bytes echoed intact | 65,536 bytes echoed intact |
| Incorrect certificate hash | Rejected before the 4-second bound | Rejected before the 4-second bound |
| No certificate hash, relying on ordinary public trust | Rejected before the 4-second bound | Rejected before the 4-second bound |
| Ordinary HTTPS fetch to the same untrusted certificate | Rejected | Rejected |
| WebRTC data channel, no configured ICE servers | 1,048,576 bytes intact; DTLS connected; nominated host/host candidate pair | Same |
| WebRTC candidate errors reported by the fixture | 0 | 0 |

The deterministic payload pattern was `byte[i] = i % 251`. Browser comparisons
checked each byte; independent Python SHA-256 checks agreed:

- 65,536 bytes: `4b640d85ab3ba30fd02c9fc9db4a8928f416322ad27022ea58a65aaee68a4df2`.
- 1,048,576 bytes: `631b84027d6b9e52b539c4e8373622d23032dfadc64d60af87339c9037e4f769`.

The WebRTC case used two peer connections in the same browser page on the same
computer. It proves a browser data-channel primitive, not NAS connectivity, peer
authentication or a physical cross-LAN path. WebTransport connected the browser
to the local Python HTTP/3 server, also on the same computer.

The existing native fixture also passed:

```sh
cargo test -p collector-server --test lan_tls
```

`paired_lan_authentication_and_ranged_download_work_without_a_public_route` passed
(1 test). It exercises explicit pairing trust, authenticated native access and
ranged reads using a generated self-signed certificate. It supports the feasibility
of software-retained native trust without an OS certificate import; it does not
implement automatic discovery or automatic authenticated delivery of new endpoint
trust in the real app.

A read-only check of the existing NAS HTTPS endpoint returned HTTP 200 for
`/api/v1/identity` with the original web Origin, but no
`Access-Control-Allow-Origin` response. Its CSP retains `connect-src 'self'` and
`media-src 'self' blob:`. Therefore the deployed browser cannot obtain alternate
media transport merely by replacing its URL. No credentials or media were sent in
that check.

## Interpretation and next acceptance

These results make WebTransport certificate-hash authentication a credible browser
candidate for avoiding manual private HTTPS configuration. It is a distinct API:
it does not make normal fetch/audio accept a self-signed server. Its certificate
hash must be bound to the known Collector through an already trusted authenticated
channel. The local server must implement HTTP/3/WebTransport and maintain suitable
short-lived certificates automatically; the current Collector TLS listener does
not implement that transport.

WebRTC also merits a bounded alternative trial. Its signaling, peer fingerprint
binding, application authorization, reliable binary/range adaptation and player
integration remain work. Encrypted channel establishment is not account access.
Do not substitute a voice-call audio codec for byte-preserving file playback.

Next test the actual Collector player against a separate LAN server: bounded
streaming/ranges, later seek, cancellation, wrong peer/hash, revoked/expired
authorization, restart/certificate rollover, UDP denial and browser local-network
permission behavior. Verify actual LAN/WAN traffic. Include the work Mac with
required GlobalProtect and actual Safari; a secure localhost page does not pass
public-origin local-network permission cases. No media throughput, audio fidelity,
WAN savings, failover or install-and-use acceptance is inferred from this fixture.

Reproduce with a fresh temporary environment (ports must be unused), open
`http://localhost:19978/` in an ordinary browser and click **Run bounded experiments**:

```sh
uv venv /tmp/collector-direct-transport-check --python python3
uv pip install --python /tmp/collector-direct-transport-check/bin/python \
  -r scripts/experiments/direct-transports/requirements.txt
/tmp/collector-direct-transport-check/bin/python \
  scripts/experiments/direct-transports/server.py
```

The server prints its temporary artifact directory and stops after 30 minutes or
Ctrl-C. This fixture is not a service installer and is not exposed to the LAN.
