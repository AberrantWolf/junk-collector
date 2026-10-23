# Bounded archive capacity recovery — HL-05E, 2026-10-20

## Scope and mechanism

The actual NAS test uses a separate generated installation of the published alpha.2
image, on Synology DS1522+ / Docker 24.0.2 / Compose 2.20.1 / Python 3.8.15.
Private configuration/state/inbox/backup roots use Btrfs; only the synthetic archive
uses a dedicated **96 MiB tmpfs Docker volume**. The image is pinned to:

```text
ghcr.io/aberrantwolf/junk-collector@sha256:f86f46d2c8870f8d647fc9225ce2cfe7c4319521a29c89897b419abb77629433
```

Its source remains `0f0e0f52fedc239ecdb180436583e69b7ce13a75`. This delivery
qualifies existing runtime behavior; it does not publish a new image. The separate
[permission-loss repair](storage-permission-recovery-2026-10-20.md) concerns retry
without the intervening restart used here.

`scripts/storage_capacity_qualification.py` creates a uniquely named external local
Docker volume with `type=tmpfs`, a fixed size, `nosuid` and `nodev`. A networkless,
read-only, unprivileged sleeper keeps it mounted across server/CLI stops. Before
filling anything, the fixture independently requires the actual mounted filesystem
to report `tmpfs` and exactly 96 MiB. A bounded zero write attempts at most 104 MiB,
requires an actual `No space left on device` failure and zero free blocks. This
cannot fill the NAS's Btrfs payload disk. Only the generated filler is removed;
the test never deletes application journals to make recovery succeed.

Normal preservation's existing 64 MiB reserve remains enabled. A smaller initial
fixture correctly refused preservation with `insufficient archive space`; increasing
the bounded fixture size enabled normal setup before the intentional ENOSPC. The
generated Compose override lives outside `/data`, whose ownership initialization
intentionally transfers to UID 10001. An initial harness placement error was repaired
and its generated root cleaned before the accepted run.

## Acceptance

The complete isolated journey passed with exit status 0 and real Chrome
155.0.8059.85:

1. Normal setup, inbox group access, generated CUE/BIN/hidden originals, interrupted
   upload/container recreation and explicit FLAC preparation passed first. Missing
   provider configuration remained visibly unavailable without blocking originals
   or playback.
2. With the actual archive filesystem full, the ordinary metadata review save
   returned HTTP 500 and a visible storage error. The entered title remained in the
   panel; committed title/revision and exact original bytes were unchanged. Real
   playback/seek/download, persistent player and 390px layout still passed.
3. Removing only the filler and cleanly restarting the fixture let normal startup
   settle the interrupted journal replacement. Browser retry saved the manual title
   and advanced its revision. Playback and original-download checks passed again.
4. Stopped full backup, validation and restore into empty roots/a second tmpfs
   volume passed. The old session, owner, ServerId, configuration/TLS, manual title,
   prepared audio and all exact original bytes survived. The browser repeated its
   full playback/download/mobile journey after restore. Clean stops took 0.23–0.31s.
5. Both generated keeper/volume pairs, Compose input, data roots and private
   credential handoffs were absent after cleanup. Live Collector, Portainer and
   both tunnel containers matched their pre-test IDs, image IDs, start times and
   running state. The failed-save screenshot was visually inspected.

Python 3.8 syntax and actual NAS execution, JS syntax, architecture/document/fixture
checks, container tools and release-policy tests passed. This delivery changes
qualification tooling and documentation; Rust and product web assets are unchanged.
The source repair's 440-test workspace, strict Clippy and full real-browser smoke
results remain recorded in the [separate repair acceptance](storage-permission-recovery-2026-10-20.md).

## Reproduction and limits

Use the [HL-05C isolated reproduction](nas-preparation-recovery-2026-10-20.md#reproduction-and-checks)
with `--storage-full` in place of `--preparation-faults`. This mode requires a fresh
inbox/browser fixture and rejects update, snapshot, Portainer and other fault modes.
Browser suffixes are `-full-archive`, `-recovered-full-archive`, the ordinary filename
and `-restored`. Run the real browser harness for each private credential handoff,
write its success marker only on success, and delete local copies.

Shared failed-save/manual-title/exact-original assertions live in
`scripts/storage_failure_qualification.py`; permission and capacity adapters supply
their actual filesystem mechanisms. Restore switches to a second empty private
volume. Cleanup removes both generated keeper/volume pairs and fixture roots.

This establishes bounded tmpfs archive ENOSPC, process restart and backup recovery,
not a physically full Btrfs disk, inode exhaustion, lost NAS power, a full private
SQLite filesystem, arbitrary mid-publication interruption or permanent-installation
acceptance. Public ingress, real-device failure cases and the broader abuse matrix
remain separate. No production Collector, Portainer, tunnel, OS trust or real media
library is changed.
