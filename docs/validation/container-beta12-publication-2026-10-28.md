# Beta.12 publication and NAS update — 2026-10-28 JST

[Beta.12](https://github.com/AberrantWolf/junk-collector/releases/tag/v0.1.0-beta.12)
is published on `preview` and installed on `fatchocobo`. It delivers compact,
server-cached Library rows while keeping authorized album details fresh and
import progress live. See [release notes](../releases/v0.1.0-beta.12.md) and
[cache behavior and regression evidence](library-row-cache-2026-10-28.md).

## Source and qualification

- Signed source and signed `v0.1.0-beta.12` tag resolve to
  `2f169715d6a8083854b985b671b9bc2cb72f9545`; both signatures verify locally.
- [Exact-source Checks 40624264312](https://github.com/AberrantWolf/junk-collector/actions/runs/40624264312),
  attempt **1**, passes architecture/document links, formatting, release/testing
  tooling, workspace tests, all-target workspace Clippy with warnings denied,
  web type checking/build/models and all **27** browser journeys. The receipt
  covers all **30** changed paths since beta.11, reports full coverage at beta
  level and confirms unchanged source. Workspace tests took **322.52 seconds**;
  browser journeys took **455.31 seconds**.
- [Container validation 40624319196](https://github.com/AberrantWolf/junk-collector/actions/runs/40624319196),
  attempt **1**, accepts that same-source receipt and passes native AMD64/ARM64
  runtime/media/proxy/inbox smokes and AMD64 release-image browser acceptance.
  The source receipt is artifact **12789506004**, digest
  `sha256:739c9c0ac92cbee2de4381a866bddca615da1701edf7fb14c57325e0c8003be7`.

The initial candidate `82650ba` was superseded during review before publication
to repair a language-key race. Its runs **40622449540** and **40622466544** were cancelled; none of their
images were accepted or deployed. Redundant same-source main/tag push Checks
**40625954407** and **40625951357** were cancelled after complete qualification.
Optional physical/owned-disc, performance/scale and opt-in ignored cases remain
unqualified. Preliminary local checks are recorded separately and do not replace
this exact-source evidence.

## Publication and independent verification

[Publication 40625979651](https://github.com/AberrantWolf/junk-collector/actions/runs/40625979651)
passes by copying accepted OCI exports without recompilation. Its check/build
jobs are intentionally skipped. It consumes validation attempt **1**, artifact
**12789376907**, ZIP digest
`sha256:fd9803add3d5f77e81fa2269c2c31ef81f3d9e1a4cfbc97a12db42afa6b6b35f`,
and the **104,806,400-byte** archive with SHA-256
`sha256:d2d408f8b703f2c588e459a0366e3ed122a18699a520060b0a17de4d52e4c652`.
The separately downloaded sealed archive and both release assets pass their hashes.

Anonymous manifest/config reads verify both architectures and their source,
version and repository labels. `preview` matches beta.12; stable `latest` remains
absent (404), matching the prepublication baseline.

```text
ghcr.io/aberrantwolf/junk-collector@sha256:0af684495436a4ef8f8fae36237d9b39e7c104e8e19d4c1b3abbc651a72279c1
```

| Platform | Published manifest digest |
| --- | --- |
| linux/amd64 | `sha256:e9e891ab7e81c96495b2db8a074e29884461da1c7ef9c8ba9ad9b0fc4924af6f` |
| linux/arm64 | `sha256:579f16c81aac1d477ae377e39f3f8cac28925a90b868490a67d14aa05d7f35f7` |

## Recovery and actual NAS update

The pinned image was pulled before stopping. Old/candidate read-only doctor
output agrees. Expanded Compose models differ only in core/media/one-shot
initializer image references. Setup and the initializer were not rerun. The data
root remains one Btrfs subvolume with no nested subvolumes.

Both old services exited **0**. After both stopped, a complete read-only recovery
snapshot captured archive, private state including SQLite recovery files,
configuration and TLS together (timestamp UTC):

```text
/volume1/collector-recovery/before-beta12-20261028T032157Z
```

The old image and private adjacent recovery metadata are retained. The retained
Portainer definition was updated atomically and only core/media recreated with
`--no-deps --pull never`. Interruption measured **7.88 seconds**. Both services run
beta.12 with **zero restarts**. The installed AMD64 image/config ID matches the
independently read registry config:
`sha256:cb32c04c3eded7128b98abc65a604a81c3e4e0fd4693db003ada08b12279d138`.

Config/TLS hashes, ServerId, environment, commands, mounts and host configuration
are retained, including media isolation. All **18** unrelated running containers
retain their IDs, images and start times. A strict verification first flagged
Docker reordering the two media volume-mount entries. Comparing those arrays by
target, while retaining every field, verifies equivalent configuration; no server
or Compose setting changed to satisfy the check. No original was copied or hashed.
Private operation receipts remain in `collector-beta12-update-20261028` on the NAS.

A bounded read-only private-state sample immediately before stopping found
**zero unfinished jobs**. The post-restart sample likewise has zero unfinished
cohort members and no missing IDs. This update cannot demonstrate new active-job
reclamation. Beta.11's actual twelve-job recovery remains separately recorded;
no new production batch was submitted merely for acceptance.

## Public HTTPS and authenticated Library acceptance

Certificate-validated control HTML returns **200**. Control/media identities match
the retained ServerId; media identity exercises the actual Unix-socket authority
connection. Control `/api/v1/library` without a session returns **401**, media
`/api/v1/library` and administration return **404**, and `/read` without a grant
returns **401**. All nine checked served assets match the signed source. The
changed `app.js` SHA-256 is
`1adbb0b19df852a8c8c59c253a47456b68d3fedb0d45363886be1164b024b318`.

The Mac initially locked again, then browser access returned. A separate tab in
the user's signed-in Firefox profile loaded Scott's Music without disturbing the
existing Imports tab. All **40 entries** render with artwork, playback attention
and per-original AccurateRip status, including the existing inconclusive cases.
A full page refresh passes without the 504; the loaded desktop was visually
inspected and left open.

| Production request | Response headers | Response complete |
| --- | ---: | ---: |
| First observed post-upgrade Library resource | 4,859 ms | 4,860 ms |
| Cached read, browser HTTP cache bypassed | 15 ms | 15 ms |
| Cached read, browser HTTP cache bypassed | 9 ms | 9 ms |
| Cached read, browser HTTP cache bypassed | 9 ms | 9 ms |
| Library resource after full page refresh | 8 ms | 8 ms |

The three repeat reads returned HTTP **200**, **40 rows**, **29,800 UTF-8 bytes**
and no further page at **03:24:45 UTC**. They use `cache: 'no-store'` and include
response-body decoding. The initial resource and full-refresh resource timings
come from the browser's same-origin PerformanceResourceTiming; the full-refresh
sample was read at **03:25:00 UTC**. Navigation timing separately reports DOM
complete at **67 ms** and load-event end at **70 ms**; these are browser lifecycle
events, not an independently instrumented React render-time measurement.

The first observed load still incurs the detailed calculation needed to fill
new cache rows. This is consistent with cold behavior, but no production cache
was deliberately flushed for a controlled cold benchmark. Ordinary repeats and
full refresh reuse server rows, and their response omits full item/capture/track
payloads. This acceptance follows completion of the original batch and does not
qualify responsiveness under its original concurrent workload. No live production
playback, cover upload, new verification batch or under-load benchmark was performed.
