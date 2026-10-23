# Storage permission failure and metadata recovery — HL-05D, 2026-10-20

## Defect and repair

Two isolated Synology DS1522+ / Btrfs runs against the published alpha.2 image
reproduced the same failure. Removing write permission from the generated archive
caused a real browser metadata save to return HTTP 500 with a visible storage error.
Committed metadata and original bytes stayed intact. Restoring the exact original
permissions allowed a normal browser retry to succeed, including playback and
downloads. However, the subsequent cold backup failed while opening storage with
`command commit marker digest mismatch`.

The failed save retained a private in-progress command outcome before it could
create a publication journal. The later successful save used the same immutable
metadata revision path with different content. Startup interpreted the earlier
failed reservation as a committed record whose digest had changed.

The storage adapter now releases an absent manual-metadata outcome under the archive
writer lock only after proving there is no publication journal. Another publisher
also performs that check before claiming the same path, covering a failed abandonment
while storage is still unavailable. Permission errors and journal ambiguity retain
the outcome. Other command families retain their existing recovery behavior.

Existing losing reservations can be settled at startup only when a coherent actual
metadata record and manifest, plus an independent completed or exactly matching
pending command, attest the winner. The failed request is never completed with
another request's result. Startup hashes and parses the same frozen marker bytes;
unexplained digest changes and mismatched result envelopes still fail closed.
No storage schema, archive format, original media or production installation changes.

## Qualification mechanism

`scripts/check_container.py --storage-permissions` requires a fresh generated
Compose installation, inbox and private browser handoff, and rejects update,
snapshot, Portainer or other bounded-fault modes in the same run.
`storage_permission_qualification.py` records every generated archive mode, removes
write bits, exercises the actual HTTP/browser save and restores every exact mode
in a `finally` block. Only its generated fixture archive is affected; it does not
fill a real disk, alter operator files or replace application responses.

The browser shows the actual error, retains the entered title, then retries through
the ordinary metadata review panel after permissions return. API checks require
unchanged committed title/revision after the failed save, a later manual title and
advanced revision after retry, and byte-exact original downloads throughout. The
normal journey includes playback/seek, original SHA-256 downloads, persistent player
and 390px layout, followed by full backup/validation/restore and the same checks.

The private handoff suffixes are `-readonly-archive`, `-recovered-archive`, the
ordinary filename and `-restored`. Run `node web/tests/container-host.mjs FILE` on
the browser host, mark `FILE.done` as `passed` only after success and delete local
credential copies. The main harness cleans its containers, data roots and handoffs.
Use the [HL-05C reproduction](nas-preparation-recovery-2026-10-20.md#reproduction-and-checks)
with `--storage-permissions` in place of `--preparation-faults`.

## Source acceptance and limits

Source acceptance passed using the rebuilt native server on this Mac and Chrome
155.0.8059.85, with a fresh generated CUE/BIN/hidden-file package and the real FLAC
encoder. The fixture removed actual archive write bits, observed the visible HTTP
500 without a committed title/revision change, restored exact modes and saved the
manual title through the browser. Playback/seek/download/persistent-player/mobile
checks passed while read-only, after retry, before backup and after full restore.
Stopped backup, validation, restore into empty roots, doctor and reindex passed.
The old session, ServerId, TLS certificate, manual authority and byte-exact originals
survived. The temporary server and credential/data roots were cleaned; screenshots
remain outside the fixture, and the failure screenshot was visually inspected.

Regression cases exercise a real filesystem
journal-creation failure followed by successful save, reopen, reindex and cold
backup; older completed/pending winners; absent attestation; changed actual bytes;
and a correct digest paired with the wrong result. Rejected recovery retains the
unresolved private record and leaves the archive bytes unchanged.

Workspace tests passed (**440 passed, 4 existing qualification tests ignored**),
including the existing concurrent import cancellation-after-commit guarantee.
Strict all-target Clippy, formatting, architecture/document/fixture checks,
container tools/release policy tests, Python 3.8 syntax, web type checks/build and
JS syntax passed. Generated web assets remained byte-identical. The two new
regressions were rerun after final self-review.
The full actual-browser smoke suite also passed: WebAuthn registration/login,
session expiry, inventory/roles, grouped/bulk import, concurrent preparation,
playback/revocation, collection lists, artwork and mobile journeys.

The public alpha.2 image predates this repair and the numeric-IP startup repair.
The two failed NAS runs above remain defect evidence; the subsequent repaired
candidate acceptance follows. No release tag, registry channel, live
Collector/Portainer/tunnel, OS trust or real media library was changed.
Storage-full, arbitrary power loss, public ingress and physical-client failure
cases remain outside this bounded permission-loss test.

## Actual-NAS candidate acceptance

The nonpublishing [native workflow run](https://github.com/AberrantWolf/junk-collector/actions/runs/39431537120)
passed source gates, AMD64 and ARM64 native builds/container smoke, and combined
OCI platform/label verification. Its source is frozen at
`f2efdfadc1183c9d76eae1cb2639a78ddf223153`, including both the numeric-IP and metadata
recovery repairs. No release retry tag was supplied and nothing was pushed to a
registry or promoted to a channel.

The selected AMD64 OCI export was checked against its recorded native index,
every content-addressed blob, actual platform and source labels, then converted to
a Docker-load archive without changing its image config:

```text
native index: sha256:d5c7c0cb1ed8e4deb301cbcfaead3bf60d8c54bf80235b6ab069d615b393ef10
manifest:     sha256:fa6b014019e66a9cb327479eed3d0b0dee2e1e06c12eb8654517bcab60d36564
image ID:     sha256:88bf2f48fcef65a4642b855bcc1466c2d6d2ab20653b8c943ebedcccf304541f
```

The NAS loaded that exact image ID with matching `amd64` and source revision.
Its package version label remains `0.1.0-alpha.2`; this is an unpublished test
candidate from newer source, not the released alpha.2 digest.

On Synology DS1522+ / Btrfs / Docker 24.0.2 / Compose 2.20.1 / Python 3.8.15,
the complete `--storage-permissions` journey passed with exit status 0. This run
used a **numeric-IP HTTPS authentication/browser origin**. Setup/doctor/startup,
password login and typed `dns_name_required` passkey unavailability passed. Chrome
155.0.8059.85 independently validated the actual certificate/IP/ServerId before
using an isolated TLS test context.

Removing generated archive write bits caused a visible failed browser save while
committed metadata and exact originals stayed unchanged. Restoring each original
mode allowed successful browser retry without restarting first. Unlike the released
image's two failures, the following clean stop, cold backup, validation and full
restore now succeeded. The manual title/authority, old session, owner, ServerId,
configuration/TLS, available FLAC and exact CUE/BIN/hidden originals survived.
Actual playback/seek/download, persistent-player and 390px layout checks passed
while read-only, after retry, before backup and after restore. Clean stops took
0.22–0.23s. The failed-save screenshot was visually inspected.

Generated containers, data, private handoffs, transfer directory and candidate image
were cleaned. Live Collector, Portainer and both tunnels matched their pre-test IDs,
image IDs, start times and running state before and after cleanup. No phone test
or system trust was required. This completes HL-05D's bounded candidate acceptance;
parent HL-05 remains partial for its separately named deployment/failure cases.
The later rate-limit retention source repair is outside this candidate's revision.
