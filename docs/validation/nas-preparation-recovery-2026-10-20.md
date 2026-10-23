# NAS preparation interruption and recovery — HL-05C, 2026-10-20

## Outcome and scope

The existing audio-preparation workflow passed an isolated actual-NAS failure and
recovery journey, including real Chrome cancellation/retry and playback. This
qualifies a bounded part of parent HL-05; it does not complete public ingress,
physical device/system-trust, storage-full/read-only, power-loss or arbitrary
mid-publication failure qualification.

Host: Synology DS1522+, Btrfs, Docker 24.0.2, Compose 2.20.1, Python 3.8.15.
Browser: Chrome 153.0.8010.37 on the Mac. The published alpha.2 image was selected
by its previously verified index:

```text
ghcr.io/aberrantwolf/junk-collector@sha256:f86f46d2c8870f8d647fc9225ce2cfe7c4319521a29c89897b419abb77629433
```

Its image source remains `0f0e0f52fedc239ecdb180436583e69b7ce13a75`; this delivery
adds qualification tooling, not a different runtime image or storage format.
See [publication acceptance](container-publication-2026-10-20.md) for its source,
platform, anonymous pull and Portainer evidence.

## Controlled real-process faults

`check_container.py --preparation-faults` requires a fresh generated Compose
installation, inbox and private browser handoff. Its separate
`preparation_failure_qualification.py` adapter configures a fixture-only executable
under the generated private state root. The wrapper reports the real shipped
FLAC version and delegates ordinary encoding to `/usr/bin/flac`:

- Blocking mode replaces the encoder child with a bounded sleep before it consumes
  PCM, allowing cancellation or container death during actual subprocess execution.
  The fixture records that child PID before the browser cancellation starts.
- Failure mode runs the real FLAC encoder to successful completion and lets its
  output reach the conversion adapter, then returns exit status 42 with an explicit
  qualification diagnostic. It cannot claim successful conversion/publication.
- Pass mode runs the real encoder. No HTTP, provider response, Job outcome,
  prepared bytes or player result is replaced with a successful-looking mock.

The fresh-install config guard remains exercised before installing those fixture
private files. An initial harness sequencing error correctly triggered the guard;
installation ordering was repaired without changing the product protection.

## Actual acceptance

All stages completed with exit status 0 using generated CUE/BIN/unknown-file
packages and normal application commands:

1. **Cancel during execution:** after the blocking encoder existed, Chrome opened
   the real preparation panel and selected **Cancel outstanding tracks**. It showed
   one cancelled track and zero ready. The encoder PID disappeared from the
   retained container's `/proc` (terminated and reaped within a bounded wait),
   and no lossless derivative was published. Original downloads stayed byte exact.
2. **Durable cancellation and explicit retry:** after clean stop/restart, the batch
   stayed user-cancelled with no audio. Generic retry of its cancelled child returned
   HTTP 409. **Retry unfinished tracks** in the actual web panel then produced one
   available FLAC, played/seeks correctly, and retained originals.
3. **Failure after output:** the real encoder completed output before the injected
   failure. No derivative was published. Failure/error remained after server restart
   and closing/reopening the web curator. UI retry completed the same child Job,
   preserving its original creation timestamp and recording a later explicit
   attempt. Exactly one derivative became available; originals stayed unchanged.
4. **SIGKILL during encoding:** the fixture waited for another blocking configured encoder process,
   then killed/reopened only its generated Collector container. Recovery completed
   the original child Job, preserved its creation timestamp and published one audio
   Representation. Chrome showed the recovered batch ready and played/seeks/downloads
   correctly. Crash claim reclamation resumes an existing attempt; it is not required
   to create a new explicit-retry attempt. Self-review removed the initial test's
   incorrect extra-attempt expectation rather than changing valid recovery behavior.
5. **Full recovery:** the normal stopped-server backup was validated and restored
   into empty roots while its source was stopped. All three preparation batches
   remained successful with their same prepared Representation identities and exact
   original bytes. The original session, ServerId/config/TLS and partial-upload
   guarantees from the shared journey also passed. Chrome playback, seek, original
   SHA-256 downloads, persistent player and 390px mobile overflow checks passed
   after restore. Cancellation and recovered-player screenshots were inspected.

The test browser independently verifies the generated PEM/hostname/ServerId before
login, then uses an isolated TLS test context. It does not install OS trust or use
an operator browser profile. Missing provider configuration produced a visible
metadata failure while original preservation and explicit audio remained usable.

## Reproduction and checks

Run on a Linux Docker host with the selected image already available, using the
source-tree qualification scripts and small generated data only:

```sh
python3 -B scripts/check_container.py IMAGE_DIGEST \
  --data-parent TEST_PARENT --port-base 29841 --proxy --inbox \
  --browser-host NAS_HOST --private-ip NAS_IPV4 \
  --browser-handoff /tmp/collector-browser.json --preparation-faults
```

For each printed private JSON handoff, securely copy it to the browser host, run
`node web/tests/container-host.mjs FILE`, and write `passed` to the remote
`FILE.done` only on success. The ordered suffixes are `-cancel-preparation`,
`-retry-cancelled-preparation`, `-retry-failed-preparation`,
`-restarted-preparation`, the ordinary filename, and `-restored`. Do not print or
retain these generated credential files. The main harness deletes its handoffs,
containers and data roots; the browser driver must delete its private local copies.

Architecture/document/fixture checks, Python 3.8 syntax, container-tools/release
policy checks, JS syntax, Rust formatting, workspace tests (**436 passed, 4 existing
qualification tests ignored**), strict all-target Clippy, web type checks/build
passed. Built web assets remained byte-identical. The changed normal browser path
also ran before and after the final full restore.

After acceptance, the generated containers, data roots and all remote/private local
browser handoffs were absent. Live Collector, operator Portainer and both tunnels
matched the before-run container IDs, image IDs, start times and running state
exactly. No production account, media, configuration, route or backup policy changed.
