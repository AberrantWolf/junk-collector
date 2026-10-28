# Beta.11 publication and NAS update — 2026-10-28 JST

The user authorized releasing all current changes, updating the NAS and checking
unfinished AccurateRip jobs and Library loading. [Beta.11](https://github.com/AberrantWolf/junk-collector/releases/tag/v0.1.0-beta.11)
is published on `preview` and installed on `fatchocobo`. It includes the shared
one-second processing cancellation poll, manual metadata cover uploads and focused
AccurateRip batch results. See [release notes](../releases/v0.1.0-beta.11.md).
Durable job recovery and authenticated production Library loading are verified.
The browser check followed the batch's completion, so it does not qualify
responsiveness under the original concurrent audio workload.

## Source and qualification

- Signed source and signed `v0.1.0-beta.11` tag resolve to
  `dde55f6a3408024c3f3396abe20aec67872222f2`. Both signatures verify locally.
- [Exact-source Checks 40611860005](https://github.com/AberrantWolf/junk-collector/actions/runs/40611860005)
  passes architecture/documentation links, formatting, testing/container/release
  tooling, workspace tests, workspace/all-target Clippy with warnings denied,
  web type checking/build/models and all **27** configured browser journeys.
  The receipt covers all 30 changed paths since beta.10, reports full coverage at
  beta level and confirms unchanged committed source. Workspace tests took
  489.62 seconds; browser journeys took 523.47 seconds.
- [Container validation 40611942521](https://github.com/AberrantWolf/junk-collector/actions/runs/40611942521),
  **attempt 2**, accepts that exact-source Checks receipt and passes both native
  AMD64/ARM64 runtime/media/proxy/inbox smokes and AMD64 release-image browser
  acceptance before sealing. The source receipt is run attempt 1, artifact
  **12783931840**, digest
  `sha256:8455bbe49589d3fa99812c8e61235a525db33d77057e747bc3bb2ad6f2ea0128`.

Container attempt 1 passed native runtime checks but its AMD64 browser navigation
failed with `net::ERR_NETWORK_CHANGED` on disposable localhost administration.
The complete validation workflow was rerun on unchanged source; acceptance uses
attempt 2 alone. No application change or timeout relaxation was made for this
failure. Redundant same-source main/tag push Checks runs 40614095903 and
40614096165 were cancelled after complete exact-source qualification; required
validation and publication were retained.

This is beta qualification. Optional owned-disc oracles, manual performance/scale
cases and physical-device qualification remain unrun; ignored cases are not
passes. Local interim results remain separately documented in the
[contention diagnosis](library-audio-check-contention-2026-10-28.md) and
[metadata/batch delivery](metadata-cover-upload-and-rip-results-2026-10-28.md).

## Publication and independent verification

[Publication 40614097932](https://github.com/AberrantWolf/junk-collector/actions/runs/40614097932)
passed by copying the accepted OCI exports without recompilation. Its check/build
jobs are intentionally skipped. It consumed validation attempt **2**, artifact
**12784756036**, ZIP digest
`sha256:6d37ce62c50ee617c26333b78b58ad317f434acfd69065d3fd99341116364151`,
and 104,755,200-byte OCI archive SHA-256
`sha256:6ad540d6ee2fc2d9045d5ef4e974431078406d4af42a198997ee65ad42ac089b`.
The separately downloaded sealed archive and release assets pass their hashes.

Anonymous GHCR manifest/config reads verify both architectures and their source,
version and repository labels. `preview` matches beta.11; stable `latest` remains
absent (404), matching the prepublication baseline.

```text
ghcr.io/aberrantwolf/junk-collector@sha256:28fae6db45cd822def0bc39b2b696cfcee8f051142d17c0854332448bc6b44dd
```

| Platform | Published manifest digest |
| --- | --- |
| linux/amd64 | `sha256:f06a630cc1121ac22acf992710c53a6af766a2509859e775f68340b1628e0c58` |
| linux/arm64 | `sha256:6b6817d0dfd9bd4fc9c9e21dd7f295f76507a8a5db1d1a4d0cec3649656ab736` |

## Recovery and actual NAS update

The immutable published image was pulled before stopping services. Read-only
old/candidate doctor configuration and output agree. Expanded Compose models differ
only in core/media/one-shot initializer image references. Setup and the initializer
were not rerun; no new batch was submitted and no jobs were cancelled.

Media exited cleanly. The old core reached Docker's 60-second stop timeout and
exited 137 without OOM. After both processes were stopped, a complete read-only
Btrfs recovery snapshot captured archive, private state including SQLite recovery
files, configuration and TLS together (timestamp UTC):

```text
/volume1/collector-recovery/before-beta11-20261028T012658Z
```

The exact old image and private adjacent recovery metadata are retained. The
retained Portainer definition was updated atomically, then only core and media
were recreated with `--no-deps --pull never`. Interruption measured **66.25 seconds**.
Both services run beta.11 with zero restarts. Installed AMD64 config/image ID
matches the independently read registry config:
`sha256:4f8d587335825e0fb3c7faa3531d9ea797a8823be7b19fc619e5b01e337160bd`.

Config/TLS digests, ServerId, environment, commands, mounts and host configuration
are retained, including media isolation. All **18** unrelated running containers
retain their IDs, images and start times. No original media was copied or hashed
for deployment. The forced stop is recovery acceptance, not proof of graceful
shutdown during active blocking audio work; that follow-up is in the
[single work queue](../remaining-work.md).

## Durable job recovery

Immediately before stopping, a bounded read-only private-state query recorded
**12** unfinished AccurateRip jobs: **eight running and four queued**. The probe
used an already-installed isolated Python runtime with networking disabled, a
read-only root filesystem, no capabilities and only read-only state/script mounts;
it neither changed database permissions nor extracted credentials.

After restart, every recorded Job ID and immutable creation timestamp remains.
Six formerly running jobs completed after the stopped-server cutoff. Two formerly
running jobs were reclaimed by the new worker instance, and all four queued jobs
started. The post-restart sample therefore contains **six completed, six running,
zero missing and zero failed** jobs. This checks actual durable recovery of the
user's existing batch without submitting a replacement batch. Private operation
receipts remain in `collector-beta11-update-20261028` on the NAS. A subsequent
read-only sample after the Mac was unlocked confirms all **12 completed**, with
no missing IDs, changed creation timestamps or failed jobs in this restart cohort.
The earlier six-running sample remains the direct worker-reclamation evidence.

## Public HTTPS and authenticated Library acceptance

Certificate-validated control HTML returns 200. Control and media identities match
the retained ServerId; media identity traverses the actual Unix-socket authority
connection. Media administration returns 404 and `/read` without a grant returns
401. All nine checked served assets match the exact signed source; the three
changed assets are:

| Asset | SHA-256 |
| --- | --- |
| `app.css` | `97beaf04e8ae0357b6c7c9575acebdf5ff15d64231b0a28de8c04e38002b8f80` |
| `curation.js` | `91c5a8b872bf4ee88b319844639f6a417c647b23b47653002d72c98065eb6d8e` |
| `library-metadata.js` | `fadcb04cab27f46eaeaa9d373201c768305a91cb322bacab823d06f99c576683` |

Before the update, the signed-in Library showed the reported 504. A read-only
50-item request for Scott's Music also exceeded a 30-second client deadline under
load. Initial post-update browser acceptance was blocked by the locked Mac. After
the user unlocked it, the same authenticated 50-item request returned HTTP **200**,
all **40** entries, in **4,702 ms**, measured through response-body decoding at
01:52:54 UTC. Library navigation and a full page reload both rendered all 40
entries, including artwork and per-original AccurateRip summaries, without the
504. The loaded desktop screen was visually inspected and left open for the user.

The original batch now reports **Completed**, **22 successful**, **three needing
attention**, **15 excluded** and **zero pending**. The attention cases are Babel's
unrecovered redumper C2 audio, SQ Relax's no-data result and The 2nd Law's ambiguous
offsets. These remain visible rather than being promoted to verified results.
The batch settled at 01:34:19 UTC, before browser access was restored. This proves
post-update loading after recovery and completion, not a controlled before/after
benchmark under the same active load. No replacement production batch was started
merely to obtain that comparison.

Authenticated production listening, new manual cover uploads, physical CD
capture/ejection and optional scale/owned-disc qualification were not performed
as part of deployment. No broader claims are made from this acceptance.
