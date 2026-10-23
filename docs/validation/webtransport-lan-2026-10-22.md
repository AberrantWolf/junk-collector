# WebTransport across the NAS LAN with the real player — 2026-10-22

**Bounded browser playback qualification passed:** the operator confirmed Chrome
playback and forward/backward seeking on the work Mac under the requested
GlobalProtect-connected test conditions; saved diagnostics corroborate real range
changes and cancellation. This follows the [same-computer primitive tests](direct-transports-2026-10-22.md)
and does not replace the accepted manual LAN URL or implement automatic detection.
No Tailscale tool, API, configuration, discovery or provider telemetry was used.
Production Collector, NAS proxies, DNS, firewall and Cloudflare configuration were
not changed by this experiment.

## Actual topology and scope

- Personal macOS 26.6.2 arm64, ordinary Chrome 155 and Firefox 157 UI. NAS
  `192.168.1.106`, browser peer `192.168.1.226`. The generic OS route to the NAS
  was direct link-layer on `en9` (`HOST,LLINFO`, no gateway). The sidecar bound
  that NAS address, not a public or wildcard interface.
- A separate Collector container used the installed image
  `sha256:4c4e0619ee83cdc8ecc9df47430d02404c9e534d34d2fc3e83870696a27cd49a`,
  its own synthetic archive/private state/config and loopback listeners
  `19981`/`19982`/`19983`. No production archive, account store or inbox was mounted.
- Synthetic audio: 180 seconds, 44.1 kHz, stereo 16-bit PCM WAV, quiet pink noise,
  31,752,078 bytes. A real collection/item/upload/commit created the saved original
  through Collector's private HTTP API. Providers/encoding were not prerequisites.
- Test page/control `https://collector-lan.awkwardexplorer.com:19978/` used the
  already installed NAS certificate with normal CA and hostname validation.
  Independent curl: HTTP 200, remote IP `192.168.1.106`, TLS verification 0.
- Media used HTTP/3 WebTransport at `https://192.168.1.106:19979/media`, with a
  newly generated seven-day P-256 self-signed certificate and its SHA-256 hash
  delivered by the trusted HTTPS test origin. No trust import, browser flags or
  security-warning exception. The page's ordinary HTTPS certificate and the
  pinned media identity are distinct; this does not require a local public-CA
  certificate for WebTransport itself.
- CPython 3.12, `aioquic==1.7.0`, `aiohttp==3.13.3` in an isolated image based on
  official `python:3.12-slim` digest
  `sha256:cb33f7f771ecec2ef1225cf39b159591a040391bd73e4c01f00fdfc4c47611d2`.
  Test containers have memory bounds, dropped capabilities, no restart policy and
  two-hour lifetime bounds. Only the sidecar mounts the existing certificate
  directory, read-only; private keys/passwords are not in the source or image.

Source and reproduction are under
[`scripts/experiments/direct-transports/lan`](../../scripts/experiments/direct-transports/lan/README.md).
The harness bundles the **actual unchanged** `web/src/music-player.tsx` and queue.
An experiment-only service worker converts its original media URL/HEAD/Range
requests into bounded 256 KiB reads through the page's WebTransport connection.
There is no whole-file Blob or decoder replacement. A CORS-mode attribute is set
on the mounted audio element by the harness, following
[Chrome's service-worker media guidance](https://developer.chrome.com/docs/workbox/serving-cached-audio-and-video).
The fixture deliberately throttles chunks to expose unbuffered seeks.

Login, collection roles, session revocation and media reads use the actual
isolated Collector. The sidecar's small opaque media grants expire after 120
seconds, remain in memory and retain the associated session for live authorization
on each read. Only the seeded representation and GET/HEAD are accepted; no
arbitrary URL proxy or private/admin surface is available. Test cookies are
renamed to `collector_wt_session` externally: cookies are not port-scoped, so the
fixture must not overwrite or forward the production `collector_session` cookie.

## Observed acceptance

| Check | Observation |
| --- | --- |
| Pinned WebTransport across physical LAN | Connected in Chrome and Firefox; NAS reports peer `192.168.1.226` |
| Actual player media path | Real HEAD and ranged GET requests reached Collector over WebTransport; metadata duration 180 seconds decoded in both browsers |
| Wrong certificate pin, Chrome | Rejected; valid session continued independently |
| Missing media grant | 401 |
| Grant used for admin/arbitrary path | 403 |
| Request larger than 256 KiB | 416 |
| Authorized exact range | 206; payload agrees byte-for-byte with the independent source |
| Ordinary HTTPS media fallback | Disabled: direct media URL returns 410; browser media requests were served by the worker/WebTransport adapter |
| Real session revocation | Old still-unexpired media grant returns 401 after Collector logout |
| Real player after revocation, Chrome | Availability polling paused audio, removed its source, reset buffer/duration and showed access loss; session-expired callback observed |
| Valid account without collection access | Real Collector returns 403; real player refuses to attach/play audio |
| Wrong CONNECT Origin | 403 in independent QUIC probe |
| Audible playback / advancing playback clock | Operator confirmed working in Chrome on the work Mac; original personal-Mac audio-output blocker remains documented below |
| Real player seek completes and resumes playback | Operator confirmed forward and backward seeking; saved worker diagnostics show distinct late ranges and cancellation of superseded responses |
| Work Mac with required GlobalProtect | Chrome passed the requested test; operator noticed no errors or unexpected prompts. This qualifies this device/profile, not every managed VPN policy |

An independent QUIC probe compared source bytes at three coordinates. It verified
the SSH-obtained **public** certificate against the trusted HTTPS descriptor and
performed ordinary TLS verification with that exact certificate as its trust
anchor; no verification bypass was used. It also independently passed the live
authorization, revocation, role, bounds, Origin and HTTP-fallback checks above.

| Offset | Length | SHA-256 of matching bytes |
| --- | --- | --- |
| 0 | 65,536 | `d0f689af07d82d51b67efb0cc6d4733f9509f67db81f5ad8ea1fef8c9aabd6b9` |
| 21,168,078 (near 2:00) | 262,144 | `90eeddd4d8eadfd65ae442567c63f002632efb54bfe946e38dc4a258c956952a` |
| 31,686,542 (near end) | 65,536 | `c010bdbe5f1f4b8fe2118790a36fd90f4259e00993e80f100b8953ada3c6c209` |

The initial real-player attempts found two bridge bugs that the echo test could
not expose: an upstream `read(n)` returned fewer than the promised range bytes,
and the repair needed explicit bodyless HEAD handling. Both were corrected and
the range/authorization probe rerun successfully. Stream buffering changed to a
bounded queue of one 256 KiB chunk; no whole-track preload is required by the
adapter. A browser may still choose to download its full media response—Firefox
eventually buffered the entire synthetic track. Production efficiency needs
separate byte-overfetch and cancellation measurements.

## Local audio blocker and work-Mac handoff

Both browsers showed an attached real audio element, 180-second duration and
buffered data, but `currentTime` remained zero. Firefox reached readyState 4 and
buffered the entire track without time advancing; Chrome initially buffered about
1.857 seconds and stalled. Neither a play event nor the UI's “Now playing” label
was counted as playback acceptance. Muting did not establish advancing playback.
`system_profiler SPAudioDataType` reported no devices and independent
`afplay -t 1` on the source failed with `AudioQueueStart failed (-66680)`.
This was evidence of an unavailable local audio output, not proof that every
browser buffering/adapter issue was resolved. The subsequent work-Mac result below
settles the bounded Chrome playback/seek question; Firefox playback remains open.

With GlobalProtect left connected, open the test HTTPS page in Chrome or Firefox,
use the separately supplied disposable `listener` credentials, click **Sign in and
connect**, then **Play synthetic track**. Confirm quiet noise and advancing time;
seek forward to around 2:00 and back to 0:20. Run the pin/authorization checks,
then **Save diagnostics** and report browser, both seek results and any error or
local-network permission prompt. Page load failure is distinct from a successful
HTTPS page followed by WebTransport connection failure. Do not change corporate
VPN controls or import a certificate. The temporary fixture's earliest stop is
approximately **14:21 JST on 2026-10-22**; extension requires deliberately
restarting the disposable fixture, not a background automation.

Saved diagnostics contain only test counters, media coordinates, browser/audio
state and bounded events. They omit passwords, cookies, grants and private keys.
Stop `collector-wt-lan-bridge` and `collector-wt-lan-backend` to end early. Synthetic
archive/state and the test image remain available for scoped cleanup/reproduction;
the sidecar removes its generated WebTransport key on graceful expiry/stop.

### Availability follow-up — evening of 2026-10-22

The operator subsequently reported the URL unreachable from every computer.
Inspection at 19:19 JST found both disposable containers had exited normally:
backend at 14:21 JST and bridge at 14:25 JST, matching their two-hour bounds.
The URL refused connections because no test HTTPS listener remained. This does
not establish a GlobalProtect or browser problem.

Both fixture containers were restarted with the same synthetic archive/accounts
and explicit eight-hour timers at 19:20:48 JST, expiring at approximately
**03:20 JST on 2026-10-23**. The bridge regenerated its pinned identity and
serves the new hash through the trusted descriptor; reload the page and sign in
again. Normal DNS-based HTTPS now returns HTTP 200 for the page and JavaScript,
remote IP `192.168.1.106`, TLS verification 0. The independent exact-byte,
range-bound, session revocation, collection denial and Origin checks passed again.
Production and Tailscale remain unchanged. Restoring availability alone did not
count as playback/seek acceptance; the subsequent operator test supplies that result.

### Work-Mac Chrome acceptance — 2026-10-22

The operator reported: Chrome playback and seeking forward and back both worked,
with no errors or unexpected prompts noticed, following the requested work-Mac
test with GlobalProtect left connected. The saved diagnostic identifies Chrome
155; its reduced user-agent OS string is not treated as the real macOS version.
This is operator-observed playback, not an inference from a play event or slider.

The saved diagnostic independently records these actual media requests and
stream cancellations (UTC times):

- 10:22:41: initial `Range: bytes=0-`.
- 10:22:47: `bytes=11534336-`; initial response cancelled at byte 3,932,160.
- 10:22:48: `bytes=22511616-`; superseded response cancelled at byte 12,320,768.
- 10:22:52: `bytes=3997696-`; later response cancelled at byte 24,084,480.

The saved audio snapshot is at 26.68 seconds with duration 180, readyState 4,
and disjoint buffered intervals near 0–30, 65–69 and 128–133 seconds. It is paused
at save time; the operator's report supplies successful playback/resumption.
These requests show that seeking did request unbuffered later bytes and then
earlier bytes, with superseded streaming responses cancelled rather than waiting
for a full-file Blob. They do not measure exact overfetch or every in-flight
request's cancellation latency.

The work-Mac **Run pin and authorization checks** result is also saved:
wrong pin rejected, missing grant 401, arbitrary path 403, oversized range 416,
authorized read 206. Earlier live Collector revocation/role checks remain the
independent evidence for those cases; the user did not claim to repeat session
revocation on the work Mac. No new certificate import or VPN change was needed.

## Remaining limits

This establishes real NAS/LAN encrypted authorized byte transfer and genuine
player playback/forward-backward seeking in Chrome on the operator's work Mac.
It does not implement automatic discovery or install-and-use delivery.
The test page is on the existing private
HTTPS origin; a normal public app origin reaching a private endpoint may trigger
different local-network permissions/CSP behavior and remains untested. Safari,
Firefox advancing playback, other GlobalProtect profiles, offline trust/bootstrap,
interruption/recovery, expired
lease behavior in the player, certificate rollover/expiry, sustained throughput
and production abuse/cancellation/backpressure hardening remain unqualified.

The observed media endpoint, peer and direct OS route avoid Cloudflare on this
test path. No ISP billing, WAN counters or packet capture were used; do not infer
zero-WAN cost for arbitrary private/overlay routes. Native Trash Amp transport and
automatic delivery of alternate trust remain separate unimplemented HL-12 work.
The manual LAN URL remains the accepted listening stopgap for both clients.

Source validation: architecture/document/fixture-manifest checker, whitespace
check, Python syntax compilation, service-worker JavaScript syntax and esbuild
bundle all passed. No production Rust or embedded web assets were changed.
