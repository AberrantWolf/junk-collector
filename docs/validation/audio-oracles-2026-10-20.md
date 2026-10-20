# Real-tool audio oracles — HL-06A, 2026-10-20

## Outcome and scope

The published alpha.2 image passed actual NAS FLAC/Opus preparation and independent
decoder checks, plus real Chrome download/play/seek before and after full restore.
This completes the bounded HL-06A delivery; parent HL-06 remains partial for owned
disc/catalog references and other advertised formats. See the current
[audio matrix](../audio-preparation.md).

Host: Synology DS1522+, Btrfs, Docker 24.0.2, Compose 2.20.1, Python 3.8.15.
Browser: Chrome 153.0.8010.37 on the Mac. Runtime:

```text
ghcr.io/aberrantwolf/junk-collector@sha256:f86f46d2c8870f8d647fc9225ce2cfe7c4319521a29c89897b419abb77629433
```

Source remains `0f0e0f52fedc239ecdb180436583e69b7ce13a75`. This delivery adds
qualification tooling/documentation, with no new runtime image or storage format.
Tools actually executed in that image: **flac 1.4.2**, **opusenc/opusdec opus-tools
0.2 (libopus 1.3.1)**. Earlier dated records of unavailable tools remain historical.

## Frozen source and independent oracles

The generated CUE declares one AUDIO track at sector zero in `disc.bin`:
225 CD sectors, 132,300 stereo frames, 44,100 Hz, signed 16-bit little-endian PCM,
529,200 bytes, three seconds. For frame `n`, independently generated samples are:

```text
left  = round(8000 sin(2π997n/44100) + 2000 sin(2π443n/44100))
right = round(7500 sin(2π1499n/44100) + 2300 sin(2π661n/44100))
SHA-256(disc.bin) = 035be573f380842f135d422ecb911427203cf14f5e484266e85300cc0304c57a
```

The generated signal has distinct channels and nonzero samples. The actual prepared
FLAC was downloaded through its authorized Representation, with exact API
length/SHA-256 and HTTP 206 byte-range comparison. Independent CLI libFLAC decoding
recovered **every original integer sample exactly**; the product validator uses a
separate Rust decoder.

For each requested Opus preset, a separate imported item had no prepared FLAC.
Chrome selected **Prepare Opus N kbps** in the real curator, waited for its download,
and saved an actual browser download with matching API length/SHA-256. The normal
album player selected exactly that Opus Representation URL, reported three seconds
within 0.05 seconds, advanced during playback and sought successfully. Original
downloads stayed byte-exact; persistent playback and 390px mobile overflow checks
passed. The restored 192 kbps mobile screenshot was visually inspected.

The independent CLI `opusdec --rate 44100 --force-wav` decoded actual API bytes to
a seekable WAV. Each result had exactly 132,300 frames, two channels and 16-bit
44,100 Hz PCM. Channel comparison omitted 100 ms at either end only for signal
comparison, with no alignment search; frame-count validation includes every frame.
Predetermined thresholds were correlation > 0.98 and RMS gain between 0.8 and 1.2.

| Requested kbps | Encoded bytes | Decoded frames | Left correlation / gain | Right correlation / gain |
| --- | --- | --- | --- | --- |
| 96 | 57,943 | 132,300 | 0.999988 / 0.999159 | 0.999977 / 1.001870 |
| 128 | 72,089 | 132,300 | 0.999989 / 0.999183 | 0.999985 / 1.001083 |
| 192 | 98,545 | 132,300 | 0.999989 / 0.998832 | 0.999990 / 0.999835 |

These are observed output sizes, not assertions of exact average bitrate. Manual
negative controls confirmed the oracle rejects silence, swapped channels, a missing
frame and the wrong sample rate; the original PCM passes with correlation/gain 1.
This bounded signal check does not establish perceptual fidelity, gapless playback,
multi-track boundaries, bit-perfect device output, native Trash Amp Opus support,
Opus original imports or whole-disc/catalog verification.

## Recovery and test-harness review

The shared generated Compose journey also exercised config installation, inbox
group access, interrupted upload through container recreation and clean SIGTERM.
After stopped-server full backup, validation and restore into empty roots, all four
prepared audio states retained their exact Representation/file/MIME/length/digest
coordinates and original bytes. Opus independent decoder oracles and the three
Opus-only browser cases passed again, alongside the normal FLAC browser case.
Account/session, ServerId/config/TLS and pending upload guarantees also passed.
Missing metadata-provider configuration remained a visible failure; originals and
explicit audio were usable.

An initial browser handoff exposed a qualification race: the target JSON path
existed while the producer was still collecting the certificate/writing content.
The harness now writes a private 0600 temporary file, flushes/fsyncs/closes it, and
publishes its complete inode with a non-replacing hard link. Stale completion
markers are refused. All eight final browser handoffs passed. This changes only
test coordination; no product API, authentication or TLS policy was changed.
The browser independently verifies the PEM/name/ServerId before its isolated TLS
test context; no OS trust or operator browser profile is modified.

Both final NAS and browser drivers exited 0. Live Collector, operator Portainer and
both tunnels retained identical container/image IDs, start times and running state.
Fixture containers, generated data roots and remote/local private handoffs were
absent afterward. Screenshots and generated audio downloads are diagnostic
artifacts; no production account, media, config, ingress or trust was changed.

## Reproduction and checks

Run on a Linux Docker host with the chosen image available and source-tree helpers:

```sh
python3 -B scripts/check_container.py IMAGE_DIGEST \
  --data-parent TEST_PARENT --port-base 29841 --proxy --inbox \
  --browser-host NAS_HOST --private-ip NAS_IPV4 \
  --browser-handoff /tmp/collector-browser.json --audio-oracles
```

Securely copy each printed private handoff to the browser host and run
`node web/tests/container-host.mjs FILE`. Write `passed` to the remote `FILE.done`
only after success; delete the private local copy. Ordered suffixes:
`-prepare-opus-96`, `-prepare-opus-128`, `-prepare-opus-192`, the ordinary filename,
`-restored-opus-96`, `-restored-opus-128`, `-restored-opus-192`, `-restored`.
Do not print or retain these generated credentials. The main harness cleans its
own generated resources/handoffs.

Architecture/docs/fixtures, Python 3.8 syntax, Node syntax and container-tool/release
policy checks passed. This session's Rust formatting, workspace tests (**436 passed,
4 existing ignored**), strict all-target Clippy, web type checks/build also passed
for the unchanged runtime/web sources during HL-05C; no Rust or web application
sources changed afterward. Built embedded web assets stayed byte-identical. The
final changed qualification paths passed the actual NAS and eight Chrome journeys.
