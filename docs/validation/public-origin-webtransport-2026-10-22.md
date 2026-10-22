# Public app origin → pinned LAN WebTransport — 2026-10-22

**Bounded Chrome public-origin qualification passed:** permission deny/allow,
trust/authorization controls, operator-confirmed playback and forward/backward
scrubbing, and the CSP-denial control. This extends the successful
[work-Mac LAN player trial](webtransport-lan-2026-10-22.md) to the actual origin
`https://collector.awkwardexplorer.com`, with the page/control requests arriving
through Cloudflare and media remaining on the NAS LAN WebTransport listener.
Automatic discovery and production endpoint enrollment are still unimplemented.
No Tailscale interaction or VPN change was made.

## Isolated public path and trust boundary

The operator explicitly approved the prepared Cloudflare route:

```text
hostname: collector.awkwardexplorer.com
path: ^/__collector_lan_test_20261022(/.*)?$
service: http://127.0.0.1:19980
priority: before the existing catch-all → http://127.0.0.1:9841
```

The new HTTP listener binds only NAS loopback. Only that path reaches the
disposable fixture. The production root still returns HTTP 200 through the
existing route, and production Collector was not restarted or reconfigured.
The existing manual LAN hostname remains available. The test containers have
eight-hour process bounds; they were explicitly stopped after qualification.
The temporary forwarding rule was retired to a static 404, as described below.

The fixture retains its own synthetic archive and accounts. Its public-origin
cookie is separately named `collector_wt_public_session`, Secure, HttpOnly,
SameSite=Strict and limited to `/__collector_lan_test_20261022/`. Production cookies
are not forwarded to the isolated Collector. The worker script and registration
scope are restricted to that test path; production-page clients remain outside
its scope. The unchanged real player still requests its normal media URLs, which
the test document's controlling worker adapts to bounded WebTransport reads.
The same-origin fixture can run trusted JavaScript; this path is an experiment
deployment, not an extension of the production account/enrollment contract.

The test page's CSP permits only self plus `https://192.168.1.106:19979` for
connections. Production retains `connect-src 'self'`: the current production app
cannot silently acquire LAN WebTransport by changing its URL. A test-only
`?block-wt=1` variant retains that stricter connection policy as a negative control;
its browser-denial outcome is now qualified below.

After real fixture login, the authenticated grant response binds the fixture's
actual ServerId, exact initiating origin, immutable media coordinates, endpoint
and fresh certificate SHA-256 hash. The browser validates those coordinates
against the trusted test descriptor before connecting. Each ephemeral server
grant is also bound to the exact WebTransport CONNECT origin: a grant issued to
one allowed origin is rejected when used through the other. This is prototype
trust delivery for the synthetic server, not production server pairing.

## Network classification and permissions

Home split DNS resolves the public hostname to `192.168.1.106`. Loading that path
directly from the NAS would not exercise a public-to-private network transition;
the existing NAS proxy correctly returns 404 for the new Cloudflare-only path.
Normal TLS curl with the hostname explicitly resolved to the publicly obtained
Cloudflare address `104.21.58.233` returned HTTP 200, TLS verification 0 and the
expected descriptor. The descriptor records the actual public origin, the scoped
control prefix and receipt through Cloudflare. No TLS-verification bypass was used.

Chrome on the available Mac also followed home DNS and received the expected 404.
Its Secure DNS control was disabled by browser management, so it was left
unchanged. No system/router DNS or browser management policy was overridden.
The operator's work Mac, whose required GlobalProtect already supplies public
resolution, performs the genuine browser permission test.

The page queries supported local-network permission states, records changes, and
starts its first LAN connection from the explicit sign-in/connect action in the
document. The service worker does not initiate a permission prompt itself. The
connection deadline allows time to answer the browser prompt; denial leaves the
player unattached and offers site-permission remediation, with no media fallback.

This is consistent with current
[Chrome WebTransport LNA restrictions](https://developer.chrome.com/release-notes/147)
and the [granular local-network permission guidance](https://github.com/GoogleChrome/modern-web-guidance/blob/main/skills/modern-web-guidance/guides/security/local-network-access.md).
Browser/enterprise policy remains authoritative; no restrictions are disabled.

## Observed results

| Check | Result |
| --- | --- |
| Exact public test path through Cloudflare | HTTP 200, public peer `104.21.58.233`, normal TLS verification |
| Production root through Cloudflare | HTTP 200, existing catch-all still selected |
| Fixture request outside test path on loopback test listener | 404 |
| Production-named cookie supplied without a test cookie | 401; it is ignored by the fixture auth adapter |
| Cross-origin login/grant mutation | 403 / 404; neither issues a session/grant |
| Public-origin fixture login and authenticated descriptor | 200; exact public origin, 32-byte pin; scoped Secure/HttpOnly cookie |
| Origin-bound grant used on the other allowed origin | 401 in independent QUIC probe |
| Unknown CONNECT origin | 403 |
| Exact early/late media bytes, missing grant, roles and revocation | Existing independent probe passes after origin-binding change |
| Work Chrome, public page, Local network denied | Saved diagnostic: actual public origin, Cloudflare path, `local-network=denied`, `local-network-access=denied`, handshake failed, no audio source attached |
| Work Chrome permission allowed | Saved diagnostic: `local-network=granted`, WebTransport connected; loopback permission stays `prompt` |
| Work Chrome trust/authorization checks after allow | Wrong server, wrong origin and wrong pin all `PASS rejected`; missing grant 401, arbitrary path 403, oversized range 416, valid read 206 |
| Work Chrome media after allow | Operator confirmed playback and scrubbing worked as expected; saved readyState 4, time 50.7 s, disjoint early/late buffers, range changes and cancellation corroborate the journey |
| CSP-blocked browser control | Passed: granted LAN permission, but browser refuses WebTransport under self-only CSP and records a `connect-src` violation; audio remains unattached |

The denied diagnostic records the permission change at 10:40:26 UTC and
`WebTransportError: Opening handshake failed.` The audio element remains paused
at zero, readyState 0, with no source and no buffered media. The separate loopback
permission remains `prompt`; LAN access does not require loopback permission.
An operator description of the prompt itself is still pending; permission state
and connection failure are directly saved browser observations.

The later allowed diagnostic, saved after tests around 10:41 UTC, confirms the
same real public origin, secure context, `viaCloudflare=true` and the exact scoped
worker URL. It records `local-network=granted`, the legacy alias also `granted`,
and loopback still `prompt`. WebTransport is established. Real player ranges
move between later bytes (20,480,000 and 24,641,536), earlier bytes (3,473,408 and
3,932,160), later bytes again (around 16,000,000), and back to 8,781,824. Superseded
responses are cancelled. The saved audio element is paused at 50.7 seconds with
duration 180, readyState 4 and buffered intervals near 0–12, 22–28, 50–53, 89–93,
116–120 and 140–144 seconds. This corroborates connection and ranged decoding;
the operator subsequently confirmed playback and scrubbing worked as expected.

The operator reported a transient “wrong site/domain” message from the controls.
Saved events identify it as the intentional `wrongServer`/`wrongOrigin` negative
tests, both `PASS rejected`; the correct authorized read still returned 206.
It was not evidence of a wrong-site warning on the valid endpoint. The fixture
now explains expected rejections and keeps its control results visible after
Save diagnostics, instead of replacing the entire summary with “saved”.

The final saved negative control records, at 10:44:17 UTC:

```text
WebTransportError: Refused to connect to 'https://192.168.1.106:19979/media'
because it violates the document's Content Security Policy
CSP blocked connect-src: https://192.168.1.106:19979/media
```

Its Local network permission remains `granted`, secure context and Cloudflare
receipt remain true, and WebTransport is false. Audio is paused at zero, with no
source, duration or buffered media. The operator also reported the expected denied
text on this control page. This isolates CSP from user-permission denial: granting
LAN permission cannot override the site's connection policy.

A standalone Python HTTPS probe through Cloudflare was rejected with HTTP 403,
Cloudflare error 1010 (browser-integrity check), before obtaining its descriptor.
That protection was not disabled or evaded. Its failed attempt is not evidence
against browser WebTransport, and it is not counted as public-origin authorization
acceptance. The independent LAN-origin probe still exercises the shared live
Collector reads, exact bytes, revocation, roles and new cross-origin grant denial.

## Cleanup and remaining delivery

Actual permission deny/allow, public-origin playback/scrubbing and pin/descriptor/
authorization/CSP controls are now qualified on the work-Mac Chrome profile.
Keep the no-pin/untrusted ordinary HTTPS behavior and Safari/Firefox scope explicit;
do not infer universal managed-browser permission policy from one Chrome profile.
Certificate rotation/offline trust, route loss and efficient cancellation/overfetch
remain follow-ups. The manual LAN URL remains the listening stopgap.

The Cloudflare deletion dialog warned that deleting the route would also delete
its associated DNS record. That action was cancelled: this hostname shares its
public CNAME with the production catch-all. Instead, the exact test-path route was
changed to Cloudflare's static `http_status:404` service. No fixture forwarding
remains. This inert guard row remains in the tunnel configuration; it was not
deleted. Cloudflare refreshed the existing CNAME to the same tunnel target
`9e46842f-29ef-4fb9-b59b-dfa4a31e5984.cfargotunnel.com`; no new hostname or target
was introduced. Public verification returned test path 404 and production root
200 with normal TLS. Both disposable containers were stopped, and generated
WebTransport private key cleanup runs on graceful bridge shutdown. Synthetic
state and source remain for reproduction; production account/archive mounts
were never used by the fixture. No new production certificate was needed.

The operator-granted browser permission and path-scoped test worker/cookie may
remain in that browser. The worker cannot control production-page clients, and
the retired public path cannot issue grants. No browser management, system DNS,
router DNS or VPN policy was changed. Chrome's unavailable Secure DNS setting
was only inspected, so there was no temporary DNS setting to restore. Final checks
confirm the bridge exited 0 and its generated WebTransport private key is absent;
the disposable backend exited 137 after Docker's stop grace period (its temporary
shell wrapper did not forward SIGTERM). This is not a graceful backend recovery
qualification. Production retains its prior image/start time. Public production
and the manual LAN stopgap both return HTTP 200 with normal TLS; the retired test
path returns 404.

The next delivery is actual authenticated endpoint advertisement/trust lifecycle
and bounded automatic selection, including both browser and Trash Amp. This
experiment used explicit test coordinates and a disposable account: it does not
implement production enrollment, discovery, route loss/recovery or certificate
renewal. The accepted manual LAN hostname remains the active listening stopgap.

Source checks passed: architecture/document/fixture-manifest validation,
`git diff --check`, Python syntax compilation, service-worker syntax and esbuild.
The independent byte/authorization/origin probe passed on the LAN control path;
its public Python attempt remains the separately recorded Cloudflare rejection.
