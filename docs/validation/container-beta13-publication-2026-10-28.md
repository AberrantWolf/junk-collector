# Beta.13 publication and NAS update — 2026-10-28 JST

[Beta.13](https://github.com/AberrantWolf/junk-collector/releases/tag/v0.1.0-beta.13)
is published on `preview` and installed on `fatchocobo`. It prepares Library
summaries when data changes and after startup/reindex, independently of browsing.
Library reads return ready summaries or safe basic pending rows without running
the detailed resolver. See [release notes](../releases/v0.1.0-beta.13.md) and
[implementation and regression evidence](change-driven-library-summaries-2026-10-28.md).

## Exact-source qualification

- Signed source and signed `v0.1.0-beta.13` tag resolve to
  `5837a65f3230d47cf3dc8f4db9f90cc1d2346dab`. Both signatures verify locally.
- [Checks 40636781292](https://github.com/AberrantWolf/junk-collector/actions/runs/40636781292),
  attempt **1**, passes architecture/document links, formatting, release/testing
  tooling, workspace tests, workspace/all-target Clippy with warnings denied,
  web type checking/build/models and all **27** configured browser journeys.
  Its receipt covers **24** changed paths since beta.12, reports full coverage
  at beta level and confirms unchanged source. Workspace tests took
  **462.67 seconds**, Clippy **76.00 seconds**, browsers **494.47 seconds**.
  The receipt is artifact **12794298387**, digest
  `sha256:14dab0e55a4f277416b914b13334f06421cff5c0cd65cce5583e8606314514d3`.
- [Container validation 40636796225](https://github.com/AberrantWolf/junk-collector/actions/runs/40636796225),
  attempt **1**, accepts that exact-source receipt and passes native AMD64/ARM64
  runtime/media/proxy/inbox smokes plus AMD64 release-image browser acceptance.
  Both required native outcomes and the browser outcome pass before sealing.

Redundant same-source main/tag push Checks **40638850482** and **40638846749**
were cancelled after complete qualification. No validation retry or source repair
was required. Optional physical/owned-disc, performance/scale and ignored opt-in
cases remain unqualified; none is counted as passed.

## Publication and independent image verification

[Publication 40638875584](https://github.com/AberrantWolf/junk-collector/actions/runs/40638875584)
passes by copying accepted exports without recompilation. Its check/build jobs
are intentionally skipped. It consumes validation attempt **1**, artifact
**12794915875**, ZIP digest
`sha256:829ab6d8eead5cd50b2b1072a945127dd60c4083f27500fd1f3afac83e142c83`,
and the **104,837,120-byte** archive with SHA-256
`sha256:1e1cb3bca541ce7a860f972dde78bf8377519d14f38524730e56bcdf7473fcf8`.
The separately downloaded sealed archive and both release assets pass their hashes.

Anonymous manifest/config reads verify both platforms' source/version/repository
labels and agreement with the sealed candidate. `preview` matches beta.13;
stable `latest` remains absent (404), matching the prepublication baseline.

```text
ghcr.io/aberrantwolf/junk-collector@sha256:162ec92b90df9ee6c70737a158487551f1d0e6fbe3805bf714998c1c564a3bf3
```

| Platform | Published manifest digest |
| --- | --- |
| linux/amd64 | `sha256:a8d141960062c3b4649c92ccfef65b6762b07ae02f431e814da2dbc99edcd718` |
| linux/arm64 | `sha256:ea483687dbe4df51dd61bbc2a5de23fcb77ad60b9401180c5520a4f41118e124` |

The Mac's resolver temporarily failed external DNS. Release tooling used a
short-lived, host-restricted loopback CONNECT relay that bootstrapped public IPs
through the NAS. HTTPS remained end-to-end with normal certificate verification.
Git operations retained strict GitHub host-key checking with a process-local
hostname override. No system DNS, browser, authentication or network protection
setting was changed. This workaround did not participate in browser timing.

## Recovery and actual deployment

The immutable image was pulled before stopping. Read-only old/candidate doctor
output agrees. Expanded Compose models differ only in core/media/one-shot
initializer image references. Setup and the initializer were not rerun. The data
root remains one Btrfs subvolume with no nested subvolumes.

Both old services exited **0**. After they stopped, a complete read-only snapshot
captured archive,
private state including SQLite recovery files, config and TLS together (UTC):

```text
/volume1/collector-recovery/before-beta13-20261028T053813Z
```

The old image and private adjacent recovery metadata are retained. The Portainer
definition was updated atomically, then only core/media recreated with
`--no-deps --pull never`. Interruption measured **4.28 seconds**. Both services
run beta.13 with **zero restarts**. The installed AMD64 image/config ID matches
the independently read registry config:
`sha256:08eb02247b3303bf2993c4f526d3a76bc33050cbe0f1a47adb8dd1c1bf3a43ce`.

Config/TLS hashes, ServerId, commands, environment, mounts and host configuration
are retained, including media isolation. All **18** unrelated running containers
retain their IDs, images and start times. Equivalent Docker mount arrays are
compared by target, preserving every field. No original was copied or hashed.
Private operation receipts remain in `collector-beta13-update-20261028` on the NAS.

The bounded read-only private-state sample immediately before stopping found
**zero unfinished jobs**. This update adds no active-user-job recovery claim and
starts no replacement production batch. Actual beta.11 recovery remains separately
recorded. Derived-summary restart/failure recovery is covered by the new tests.

## Summaries prepared before browsing

Before opening the post-update Library tab, an isolated read-only SQLite probe
recorded **79 required summary variants**, **79 ready**, **zero pending** and
**79 basic rows**, including removed entries. The sample was taken at
**05:38:39 UTC**, using current enabled-account grants/preferences and matching
source tokens. Catalog schema remains **15**. It neither calls Library nor reads
source media. The namespace is
`ba24f2781ff99b6c36b9704d727b7de5b9f4a8ee57307373aa7ecee8aac6a776`.

The user unlocked the Mac and acceptance opened a separate tab in signed-in
Firefox, retaining the existing album-detail tab. The first observed post-update
Library navigation renders all **40 visible entries**, with artwork, playback
attention and per-original AccurateRip status. All returned summaries are ready;
the API response has no full Item/capture/domain graph. A full refresh passes too,
and the loaded desktop was visually inspected and left open.

| Production Library request | Response complete |
| --- | ---: |
| First observed post-update navigation resource | **8 ms** |
| Repeat with browser HTTP cache bypassed | 26 ms |
| Repeat with browser HTTP cache bypassed | 12 ms |
| Repeat with browser HTTP cache bypassed | 11 ms |
| Resource after full page refresh | **10 ms** |

The first resource reaches response headers in **8 ms**; the refreshed resource
in **10 ms**. Repeat reads return HTTP **200**, **40 entries**, **30,784 UTF-8
bytes**, page state `ready` and ready row states, measured through body decoding
at **05:39:54 UTC** with `cache: 'no-store'`. The initial resource's same-origin
PerformanceResourceTiming is read in that sample; the reload sample is read at
**05:40:06 UTC**. Navigation separately reports DOM complete **111 ms** and load
event end **114 ms**; those lifecycle events are not an independently instrumented
React render-time measurement.

Unlike beta.12's first observed 4.86-second load, the worker has already prepared
these summaries. No production cache was deliberately flushed. If a later edit is
still being processed, Library returns a basic pending row rather than waiting
for the detailed calculation. That cold/invalidated resource contract is also
checked by denying full item/capture/observation/recipe reads in SQLite tests.
These production timings follow completion of the original batch and do not
qualify the original concurrent audio workload. No production metadata/artwork
mutation, playback, physical capture or new verification batch was performed.

## Public HTTPS and served assets

Certificate-validated control HTML returns **200**. Control/media identities match
the retained ServerId; media identity traverses the actual Unix-socket authority
connection. Control Library without a session returns **401**; media Library and
administration return **404**; media `/read` without a grant returns **401**.
All nine checked served assets match signed source. Changed `app.js` SHA-256:
`5bdfdef49e83b458e62147f246bc2c1f9e00283d2c18bfc4e449216d77f778ae`.

The first CLI control-HTML probe received Cloudflare **403 / 1010** for the default
Python user agent. The authenticated browser passed, and repeating the CLI checks
with the explicit `JunkCollector-ReleaseCheck/0.1.0-beta.13` agent passes HTML,
assets, identities and authorization responses. No proxy/security rule was changed
or challenge bypassed to satisfy acceptance.
