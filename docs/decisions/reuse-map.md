# Reuse, extraction, and fixture map (D01)

Status: accepted inventory gate, 2026-09-09. This is a read-only assessment. An
operation is **supported** only when its source API and independent fixture establish
the stated byte/coordinate domain; **unqualified** means useful code exists but the
required oracle or environment has not been run; **unavailable** means the inspected
source does not implement the promised case.

## Inspected sources

| Checkout | Inspected revision and state | License/provenance note |
|---|---|---|
| `../junk-libs` | `1e424737db7b`, clean (origin main `57b6e8a`) | MIT; current shared implementation |
| `../retro-junk` | `e8792f111cb3`, dirty only by untracked sync documentation/scripts at inspection | MIT file still contains placeholder attribution; qualification/extraction must resolve provenance before redistribution |
| `../phono-junk` | `dab36e8b89d5`, untracked `AGENTS.md`; pins junk-libs `0c8ac1…` | MIT; published-vector and locally supplied owned-dump harnesses |
| `../trash-amp` | `796a2786ed99`, substantial unrelated tracked/untracked user changes | MIT; consumes sibling `../junk-libs` by path, so extraction must not overwrite those changes |

The SHAs identify what was inspected, not a request to reset any checkout. D02/D03
must re-check status and API drift before editing.

## D02.1 drift and provenance revalidation — 2026-09-10

The D02 implementation gate re-read every checkout without changing sibling files.
The inspected commits and dirty paths are unchanged from D01; the longer object IDs
and upstream relationship below are the boundary for subsequent child commits.

| Checkout | Revalidated revision/upstream | Revalidated working state |
|---|---|---|
| `../junk-libs` | `1e424737db7bb3ba329aea709de6fe3ca8fe9eea`; `main` is one commit ahead of `origin/main` | clean |
| `../retro-junk` | `e8792f111cb3cbfe86fa876b585251e34b2df944`; `main` is one commit ahead of `origin/main` | only untracked `scripts/README-retro-library-sync.md` and `scripts/sync_retro_library.fish` |
| `../phono-junk` | `dab36e8b89d59b76fec96186a29f7358587f0ece`; `main` matches `origin/main` | only untracked `AGENTS.md`; its exact junk-libs pin `0c8ac1a1ea3902f20a0e06ba2c922d2dc2b1774d` is present in the local junk-libs object store |
| `../trash-amp` | `796a2786ed99877e692f8f624d6b0e0bc9a371b7`; `main` is one commit ahead of `origin/main` | tracked changes in `Cargo.lock`, `README.md`, `amp-core` manifest/library, desktop main/UI, architecture and validation; untracked browse source/UI files |

The provenance gate remains unresolved for copying Retro-owned implementation:
Retro's MIT file still says `Copyright (c) [year] [fullname]`. D02.2 may make
Retro consume independently authored/shared contracts, but it must not move or
redistribute Retro source until attribution is resolved. Phono and junk-libs name
Scott Harper in their MIT files.

Fixture availability was also rechecked. Phono has committed ARver TOC/CUE vectors,
generated sparse BIN recipes, offline CRC/dBAR/offset tests, a network-gated ARver
WAV oracle, and an owned-dump/live-provider Redumper harness gated by
`JUNK_LIBS_REDUMPER_AUDIO_PREFIX`. Retro has a generated archive fixture. `chdman`
is installed at `/opt/homebrew/bin/chdman`, but no D02 CHD qualification has run
yet and no private Redumper payload was supplied. These facts qualify inputs; they
do not implement or verify any D02 extraction.

## Operation inventory

Coordinates are explicit: bytes are file bytes unless a row names normalized ROM
bytes, CD frames/sectors, DVD units, or decoded PCM frames.

D02.2 update (2026-09-10): `junk-libs-core` is now the only implementation of
`MultiHasher` and its generic hash/reader/error types. `junk-libs-disc` now exposes
`compute_cue_track_spans` for stored file-byte ownership and
`visit_chd_raw_tracks` for ordered 2352-byte CHD main-channel frames. Retro's
complete-disc CUE/CHD hashing uses both; its platform normalization and catalog
matching remain outside the shared crates. Synthetic gaps are retained as metadata
but never fabricated into hashed source bytes.

| Operation | Source API and domain | Optional dependencies | Consumers / extraction target | Independent fixture | Status |
|---|---|---|---|---|---|
| Streaming CRC32, MD5, SHA-1 | `junk-libs-core` `MultiHasher`; exact bytes read | hash crates only | D02 → `junk-libs-core`; Retro, Collector | generated empty/short/chunked vectors compared with published digest values | supported |
| Platform normalization policy | Retro platform analyzers; header/byte-order rules over normalized ROM bytes | platform-specific | Remains in Retro; D04 invokes qualified analyzers | generated transforms plus independent emulator/format references per platform | unqualified per platform |
| NES iNES header removal | Retro NES `dat_header_size`; skips exactly the 16-byte iNES/NES2 header after magic detection. It does not separately skip a trainer advertised by the header, so trainer-bearing images are not qualified | none | D04 game module | generated iNES/NES2 vectors with and without trainers and independently calculated payload digests | unqualified |
| SNES copier-header removal | Retro SNES analyzer; 512-byte copier header heuristic | none | D04 | paired headered/headerless public-domain vector | unqualified |
| Nintendo 64 byte order | Retro N64 analyzer; z64/v64/n64 word/halfword normalization | none | D04 | generated permutations with independent expected normalized SHA-1 | unqualified |
| PC Engine copier-header removal | Retro PCE analyzer; optional 512-byte copier padding before whole 8-KiB banks | none | D04 | paired generated headered/headerless HuCard vectors | unqualified |
| Other inspected cartridge analyzers | Game Boy, GBA, DS, Genesis/32X, Master System/Game Gear and SG-1000 analyzers use the default zero-header/no-chunk-normalizer DAT path; this inventory found no advertised byte rewrite | platform-specific analyzers | D04 qualifies identity/header parsing separately; no normalization extraction | generated/public-domain vector per analyzer before any support claim | unavailable as normalization; analyzer qualification remains per platform |
| CUE parsing and ordered track spans | `junk-libs-disc` CUE/layout APIs and divergent Retro copy; CD frames, track-relative indexes | none | D02 reconcile into `junk-libs-disc`; Phono/Retro/Collector | explicit synthetic CUE spans covering INDEX 00/01, files and ordering | supported for covered layouts; divergent edge behavior unqualified |
| Stored versus synthetic pregap | junk-libs layout/CUE model; CD frames | none | D02 → `junk-libs-disc`; D05 | explicit CUE where pregap is stored and where `PREGAP` is synthetic | supported for synthetic fixture; real mixed mode unqualified |
| Raw 2352 CD sectors | junk-libs disc readers; 2352-byte frames | optional CHD for CHD input | D02/D05 | generated sector-tag fixture and explicit spans | supported |
| Cooked 2048 CD sectors | junk-libs readers; 2048-byte user-data sectors | optional CHD | D02/D04/D05 | generated sector-tag fixture | supported |
| Mode 2 sectors | junk-libs/Retro layout paths; 2336/2352 physical framing with form-dependent payload | optional CHD | D02/D05 | explicit generated mode/form vectors required | unqualified |
| CHD CD frames | junk-libs CHD reader; 2448-byte CD hunk/frame domain including subchannel | `chd` feature / libchdr | D02/D05 | version-pinned `chdman` from explicit CUE spans, compare extracted track bytes | unqualified until pinned tool gate runs |
| CHD DVD units | junk-libs/Retro CHD paths; 2048-byte DVD units | `chd` feature / libchdr | D02/D04 | version-pinned `chdman` generated image | unqualified |
| Complete ordered Redump match | Retro `hash_chd_tracks`; every ordered track with declared domain | CHD support plus catalog snapshot | D02 primitive, D04/D05 policy | multi-track generated/public vector and independent expected per-track hashes | unqualified pending independent catalog vector |
| Track-1 / largest-track identification | Retro `hash_cue_track1`, `hash_disc_container`, `hash_chd_raw_sectors`; one selected track only | format-specific | May support identification in D04/D05; never a verification result | generated image proving an altered non-selected track is not detected | supported only as identification |
| Logiqx/ClrMamePro DAT parsing | `junk-libs-dat` via Retro file/error adapter; catalog records, not payload bytes | quick-xml | D02 shared catalog-neutral crate / D04 | equivalent generated XML/text records plus Retro Redump/LibRetro regressions | implemented locally; independent corpus external |
| Catalog/provider downloading | Retro adapters/orchestration | network, provider credentials | Remains adapter code; D04 adds Collector adapter | recorded provider contract plus network gate | unavailable for shared extraction |
| Redumper sidecar validation | junk-libs disc redumper APIs; sidecar schema and referenced raw PCM ranges | none for parsing | D02/D05 | provenance-redacted Retro mixed-mode manifest; external owned payload root | unqualified without owned payload |
| Raw redumper PCM reader | junk-libs seekable audio; signed integer PCM frame coordinates | audio feature | D02 reader / D03 audio consumer / D05 | generated PCM plus Phono/Retro external owned dump | supported for generated PCM; real dump unqualified |
| Mixed-mode redumper layout | source parsers expose partial layout support | external real fixture | D05 | Retro provenance-redacted manifest plus environment-supplied owned dump | unavailable for advertised complete verification |
| MusicBrainz disc ID | `junk-libs-disc-id` via Phono compatibility wrapper; validated normalized CD TOC frames | hashing/base64 | D02 shared disc-identity API / D05 | published MusicBrainz/libdiscid vectors | implemented locally; live drive external |
| FreeDB disc ID | `junk-libs-disc-id` via Phono compatibility wrapper; validated normalized CD TOC frames | none | D02/D05 | published/libdiscid-compatible vectors | implemented locally; live drive external |
| AccurateRip disc IDs | `junk-libs-disc-id` via Phono compatibility wrapper; validated normalized TOC frames | none | D02/D05 | ARver and Phono published vectors | implemented locally; CRC/provider work remains D02.4 |
| AccurateRip v1/v2 checksum | `junk-libs-accuraterip` via Phono compatibility facade; packed integer CD PCM | none | D02 shared verification crate / D05 | ARver-derived and independently calculated regression vectors | implemented locally; real dump external |
| Frame-450 evidence and offset search | `junk-libs-accuraterip`; reconstructed packed-integer disc stream | none | D02/D05 | CUETools-derived frame-450 semantics and -2939..=2939 cross-track vectors | implemented locally; frame-450 alone never verifies |
| Offset search | Phono `verify_with_offsets`; signed sample-frame offsets with explicit window | none | D02/D05 | ARver plus locally supplied real harness | supported by code/vector; real-dump qualification outstanding |
| dBAR parsing and confidence | Phono dBAR parser/report types; database records keyed by disc/track | network only for download | D02 parser / D05 provider adapter | recorded public response vector | supported for parsing; live provider unqualified |
| Headless audio source/PCM contract | `junk-libs-audio`; opaque `Read + Seek + Send`, length/media hints, interleaved `f32`, integer frames | none | D03 shared decoder/resampler/DSP services | in-memory read/seek and PCM-shape boundary tests | implemented |
| WAV decode | `junk-libs-audio`; opaque reader to interleaved `f32` PCM | optional `decode-wav` (Symphonia PCM/WAV only) | D03 shared decoder service | generated WAV sample/frame, boundary, exact seek, and EOF vectors | implemented; consumer migration pending D03.4 |
| Compressed decode | Trash Amp decoder path; decoded integer PCM frames | Symphonia codecs | D03 | redistributable/public-domain encoded vectors per codec | unqualified per codec |
| Integer-frame seek | `junk-libs-audio` WAV plus opened-reader CUE/redumper adapters; decoded sample frames, never bytes or seconds | decoder-specific | D03 | generated WAV ramp plus two-track CUE and in-memory redumper vectors | implemented for WAV/CUE/redumper; compressed unqualified |
| Resampling | Trash Amp audio; explicit input/output sample rates and PCM frames | resampler | D03 DSP service | generated impulses/sines compared with independent numeric tolerances | supported for current fixture; broader formats unqualified |
| Playback coordinate contract | `junk-libs-playback`; opaque consumer keys, generations and track-local integer frames | none | D03 queue service | generic-key/range contract tests | implemented; transitions pending D03.3 |
| Playback queue | Trash Amp `amp-core`/`amp-audio`; logical track IDs and positions | none | D03 service | deterministic state-machine tests | supported in Trash Amp; shared migration pending D03.3-D03.4 |
| DSP chain | Trash Amp audio; decoded PCM buffers | optional DSP features | D03 service | independent impulse/gain/clipping vectors per effect | unqualified |
| Device output | Trash Amp CPAL path; host device frames | CPAL/hardware | D03 optional output adapter only | manual hardware matrix | unqualified; unavailable in headless CI |

## Extraction ownership

- **D02** consolidates generic hashing and representation readers in
  `junk-libs-core`/`junk-libs-disc`. It may add narrow DAT or verification crates
  only where a catalog-neutral API has no natural existing home. Platform meaning
  remains in Retro Junk and provider/network access remains in adapters.
- **D03** extracts decoder, integer-sample seek, resampling, queue and DSP services
  from Trash Amp. CPAL remains an optional output adapter. Filesystem and Collector
  HTTP access remain behind authorized reader adapters.
- **D04** integrates only independently qualified game normalization, analyzer and
  DAT operations. A Track-1/largest-track helper can support identification but
  cannot be promoted to complete verification.
- **D05** integrates qualified TOC, ordered-track, AccurateRip and offset operations.
  Unsupported redumper and mixed-mode layouts remain explicitly unavailable.

## Fixture and legal policy

Committed fixtures are limited to small generated/public-domain byte vectors,
manifests, generation recipes and independently produced expected results. Disc-ID
and AccurateRip coverage uses the published MusicBrainz/libdiscid and ARver vectors.
Synthetic CHDs are produced with a recorded, version-pinned `chdman` and compared
against explicit CUE spans; the generated payload need not be committed when the
recipe and digest suffice.

Retro Junk's provenance-redacted mixed-mode redumper manifest and Phono Junk's real
AccurateRip harness may be reused only with an environment-supplied owned-dump root.
The root is never committed or hard-coded. Missing private dumps, `chdman`, network,
or an audio device yields a visible skipped/unqualified gate and cannot expand the
supported matrix.

## Required downstream checks

The exact baseline command families assigned to each owning checkout are:

| Checkout | D02/D03 command gate after an extraction |
|---|---|
| junk-libs | `cargo fmt --all -- --check`; `cargo test --workspace --all-features`; `cargo clippy --workspace --all-targets --all-features -- -D warnings` |
| Retro Junk | `cargo fmt --all -- --check`; `cargo test --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; then the D04 per-platform independent-vector command recorded beside every enabled operation |
| Phono Junk | `cargo fmt --all -- --check`; `cargo test --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; then published disc-ID/ARver vectors and the explicitly gated owned-dump harness |
| Trash Amp | `cargo fmt --all -- --check`; `cargo test --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; then D03's decoder-feature matrix and an explicitly skipped/manual CPAL device gate |
| Junk Collector | `python3 scripts/check_architecture.py`; `cargo fmt --all -- --check`; `cargo test --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; `npm --prefix web run check`; `npm --prefix web run build`; applicable normal/local browser smoke |

D02/D03 run and record these in the affected sibling checkout rather than treating
this read-only inventory as a pass. D04/D05 run Collector's full gate and the named
independent vectors.

The current divergence, old Phono pin, Retro license placeholder, Phono's stale
"future offset" documentation, dirty Trash Amp checkout, single-track identification
helpers, and externally gated real-dump/audio/hardware tests are blockers or
qualifications owned by D02-D05; none is silently declared resolved by this map.
