# Reuse, extraction, and fixture map (D01)

Schema reservations below record earlier planning, not fixed compatibility obligations. New work follows [pre-release evolution](pre-release-evolution.md) and actual persisted state.

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
| Resampling | `junk-libs-audio`; contiguous bounded interleaved PCM and exact rounded output length | optional `resample` (Rubato) | D03 DSP service | generated 2x stereo impulse timing/channel/length vector | implemented; consumer migration pending D03.4 |
| Playback coordinate contract | `junk-libs-playback`; opaque consumer keys, generations and track-local integer frames | none | D03 queue service | generic-key/range contract tests | implemented; transitions pending D03.3 |
| Playback queue | `junk-libs-playback`; opaque keys, track-local frames and generation-fenced transitions | none | D03 service | replace/append/truncate/seek/transition, restoration and failed-mutation vectors | implemented; Trash Amp transport adapter migrated |
| DSP primitives | `junk-libs-audio`; explicit linear gain and symmetric hard clipping | none | D03 service | exact gain/peak/clipped-count vectors and invalid numeric probes | implemented; broader effects not implemented |
| Device output | Trash Amp CPAL adapter; shared PCM enters bounded ring-buffer frames | CPAL/hardware | D03 optional output adapter only | automated callback-independent pipeline plus manual hardware matrix | adapter implemented; physical output unqualified |

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

## Wave 4 baseline and qualification lock-in — 2026-09-11

This section is additive to the dated D01/D02 inventory above. No sibling checkout
was changed. Collector is `c5c3ecfb2ada2624e780ac128b290e4c37f1d468` with the
user's Wave 4 plan/status/backlog documentation edits present at inspection.

| Checkout | Current revision and working state | Drift from the prior inventory |
|---|---|---|
| `../junk-libs` | `91dd63dc1d06d711e61d238c3291423466ccb281`, clean | D02/D03 shared DAT, AccurateRip, disc-ID, audio and playback work is now committed; current `main` remains ahead of `origin/main` |
| `../retro-junk` | `d36bc9aefbd4251e72a684b168d3ee66af5f2e3a`, only the two previously recorded untracked sync files | consumes shared ordered disc hashing and neutral DAT parsing; MIT attribution placeholder remains unresolved; `main` is ahead of `origin/main` |
| `../phono-junk` | `f459c5161225223f3799b86fc6580b6b03e1f8ba`, only untracked `AGENTS.md` | consumes shared disc-ID and AccurateRip primitives; its published dependency still does not pin the current unpublished junk-libs revision; `main` is ahead of `origin/main` |
| `../trash-amp` | `cb95c57e7acbf249ccc1b0038dbf1bf339910e63`, with the substantial unrelated tracked/untracked edits listed above | migrated playback/audio consumers to shared services; the user worktree remains dirty and must not be overwritten; `main` is ahead of `origin/main` |

The D02/D03 APIs still match the Wave 4 boundary: `MultiHasher` streams exact
bytes; `compute_cue_track_spans` accepts declared CUE names and a length resolver;
`visit_chd_raw_tracks` yields ordered raw 2352-byte frames; `junk-libs-dat::parse_dat`
accepts `BufRead`; `junk-libs-disc-id` validates typed TOCs; `CueAudioReader` and
redumper `RawAudioReader` expose seekable integer stereo frames; and
`junk-libs-audio` keeps playback PCM floating-point. The planned D05.1 work remains
real: `verify_with_offsets` still takes `Vec<DiscTrackSamples>` and constructs a
whole-disc vector plus two prefix arrays, so it does not satisfy the 64-MiB reader
budget. There is also no generic host-level preservation PCM reader yet. These are
known gaps, not API drift that changes the selected extraction.

### Frozen schema and dependency selections

- Archive envelope schema 4 is additive. Private database v10 owns processing
  Job/checkpoint references, v11 owns active catalog selection, and v12 is reserved
  for TLS rotation. Rebuildable
  catalog schema v5 owns all new Wave 4 portable projections and snapshot pagination indexes. Client connection
  schemas v1 (C01) and v2 (S04 rotation) are independent.
- Shared algorithms use the path checkout `junk-libs` 0.1.0 at
  `91dd63dc1d06d711e61d238c3291423466ccb281`; publishing or an immutable Git pin
  is required before a distributable build. No floating Git dependency is allowed.
- DAT parsing is `junk-libs-dat` 0.1.0 at that revision with `quick-xml` 0.37.5.
  CHD reading is `chd` 0.3.4. Provider HTTP composition stays on the Collector
  lockfile's `reqwest` 0.12.28 and `rustls` 0.23.44.
- D03 playback decoding/resampling remains `symphonia` 0.5.5 and `rubato` 0.16.2.
  D07 production encoding uses directly spawned `flac` 1.5.0 for the first golden
  FLAC gate. `opusenc` is not installed at this baseline, so no Opus golden version
  is selected or claimed; D07.1 must record and pin the available `opusenc --version`
  before enabling that profile. Phono's `flac-bound` 0.5.0 / `metaflac` 0.2.8 path
  remains an inspected reference, not the selected Collector encoder adapter.
- The optional CHD oracle is `/opt/homebrew/bin/chdman` 0.285. A different version
  produces a distinct qualification row rather than silently replacing this one.

### Wave 4 advertised-operation matrix

“Advertised” here means eligible to become a registered capability only after the
named child and oracle pass. Every row was unavailable/unqualified at W4.0; the
status column now records the latest completed checkpoint without implying runtime
registration.

| Operation | Input domain -> output domain and exact target API | Feature/dependency | Fixture/oracle and provenance | Current status |
|---|---|---|---|---|
| Raw game hashing | one authorized package reader -> versioned exact-byte SHA-256/CRC32/SHA-1/MD5 observation | `junk-libs-game-formats::hash_raw` | D04 `raw-published` vector; public published `123456789` digest vector; MIT/generated | D04.3 registered as `org.junk.games.hash` and used by Import analysis |
| NES iNES/NES2 payload hashing | one validated iNES reader -> trainer-free normalized payload digest/version | `junk-libs-game-formats::{inspect_ines,hash_ines_payload}` | D04 payload plus iNES, NES2 exponent, trainer, misc-ROM and malformed-length cases; NESdev format reference | D04.3 registered for explicit iNES payload hashing and structure inspection; unsupported variants fail closed |
| N64 z64/v64/n64 normalization | one reader -> canonical big-endian ROM digest/version | `junk-libs-game-formats::hash_n64` | D04 three byte-order recipes and hostile three-byte reader chunks with independently fixed canonical bytes; generated | D04.3 registered under the explicit N64 profile |
| SNES copier-header handling | one reader plus explicit layout -> headerless payload digest/version | `junk-libs-game-formats::hash_snes_payload` | D04 paired 32-KiB generated payload/header recipe; SNESdev whole-bank copier-header rule | D04.3 registered only with explicit copier-header layout; real dumps external |
| PCE copier-header handling | one reader plus explicit layout -> bank-aligned payload digest/version | `junk-libs-game-formats::hash_pce_payload` | D04 paired 8-KiB generated payload/header recipe; Retro consumer cross-check | D04.3 registered only with explicit copier-header layout; real dumps external |
| Ordered BIN/CUE raw-2352 hashing | CUE plus complete frozen package readers -> per-track raw-byte digests/layout | `junk-libs-game-formats::hash_cue_raw_2352` over `junk-libs-disc` spans | combined/split sources, stored/synthetic pregap, missing/reordered/truncated/cooked cases | D04.3 registered as `org.junk.games.hash-tracks`; real mixed mode external |
| CHD raw-CD track hashing | authorized CHD reader -> ordered unpadded raw-2352 digests/layout | `junk-libs-game-formats::hash_chd_raw_2352` feature plus `junk-libs-disc`/`chd` 0.3.4 | pinned `chdman` 0.285 creates a 3/5-frame two-track CHD; independently computed per-track SHA-256 | not exposed by D04.3; pinned synthetic oracle passed, real dumps/tool versions external |
| DAT snapshot parse/match | bounded raw DAT snapshot -> typed catalog rows and zero/unique/ambiguous/partial/inconclusive result | `junk-libs-dat`, `quick-xml` 0.37.5 | exact generated XML/text bodies plus duplicate, wrong-system/domain, missing-checksum, CRC-only, reordered and modified cases | D04.2 locally implemented; independent real corpus external |
| CD CUE/redumper layout | declared package readers/sidecars -> typed sectors, tracks, INDEX 00/01 and gap ownership | `junk-libs-disc` | D05 generated CUE/PCM plus Phono published layouts; owned dump via `JUNK_COLLECTOR_W4_CD_DUMP_ROOT` | generated layouts ready; real redumper/mixed mode external |
| CD IDs | validated typed TOC sectors -> MusicBrainz/FreeDB/AccurateRip IDs; `calculate_disc_ids` | `junk-libs-disc-id` | D05 ARver 3-track published values | qualified primitive; Collector integration planned D05.1 |
| AccurateRip v1/v2 and signed offset search | exact i16 stereo 44.1-kHz frames -> per-track checksums and signed stereo-frame offset result | `junk-libs-accuraterip`; new bounded reader API | D05 extremes plus ARver/CUETools vectors; owned dump via `JUNK_COLLECTOR_W4_CD_DUMP_ROOT` | vector primitive exists; bounded path and real dump unqualified |
| AccurateRip dBAR acquisition | frozen typed disc query -> bounded raw response plus parsed snapshot | provider adapter, `reqwest` 0.12.28 | recorded dBAR response/local HTTP fixture; live opt-in only | planned D05.2 |
| Metadata resolution | portable candidates/authority -> effective typed display fields/artwork selection | app resolver | D06 generated multilingual bytes plus separate identity-evidence inputs; full provider and unknown-field history tests remain D06 | planned D06.1 |
| MusicBrainz/CAA/DAT metadata | frozen identifiers/search -> bounded raw response, candidates and Asset input | provider adapter, reqwest/rustls, shared limiter | recorded responses/local HTTPS fixture; live opt-in | planned D06.2; never audio verification |
| FLAC level 5 | exact i16 stereo 44.1-kHz reader -> immutable FLAC recipe/output plus decoded-PCM validation | configured `flac` 1.5.0 | D07 extreme PCM recipe and later encoded golden; generated | planned D07.1 |
| Opus 96/128/192 kbps | exact source PCM -> immutable 48-kHz stereo music-mode Opus recipe/output | configured version-pinned `opusenc` | D07 PCM recipe; golden blocked on absent tool | unavailable at baseline |
| Portable export/change inventory | frozen archive revisions -> verified directory bundle/inventory digest | Store/app export ports | P05 unknown lexical JSON and failure recipes; generated | planned P05.1 |
| Full backup/restore | stopped archive/private/config/scratch -> validated restoration or independent clone | SQLite backup API, both process locks | P05 temporary-root matrix | planned P05.2 |
| Persistent native connection | enrollment/trust observations -> client DB v1 plus secure secret reference | SQLite; macOS Keychain adapter | C01 single-route seed; C01.1 must add two-server/two-profile cases; opt-in `JUNK_COLLECTOR_W4_KEYCHAIN_TEST=1` | planned C01.1; Keychain external |
| Authenticated route/typed transfer | enrolled local TLS or Web PKI origin -> typed DTO/media bytes | rustls 0.23.44/reqwest 0.12.28 | C01 local TLS fixtures and immutable body digest | planned C01.2/C01.3 |
| TLS trust rotation | authenticated transition over existing trust -> client/server generation state | rustls/Axum server, schemas private v12/client v2 | S04 transition/state recipes; independent security review required | planned S04; no cryptographic qualification claimed |
| Discovery/probing | DNS-SD/configured host hints -> uncredentialed bounded candidates/health | production mDNS adapter | C02 forged/expiry recipes; real LAN via `JUNK_COLLECTOR_W4_LAN_TEST=1` | planned C02.1; multicast external |
| Ranged failover | immutable manifest plus trusted routes -> verified final bytes/resume state | HTTP Range/If-Range, TLS | C02 body/digest and deterministic disconnect recipes; real tailnet via `JUNK_COLLECTOR_W4_TAILNET_TEST=1` | planned C02.2; LAN/tailnet external |

Fixture recipes and exact expected digests live under `fixtures/wave4/`. Retro's
license placeholder, unpublished shared pins, absent `opusenc`, private owned-dump
input, Keychain, independent security review, and real LAN/mDNS/tailnet environments
remain visible gates. None is converted into a passing result by this lock-in.

W4.0 review correction: `scripts/check_wave4_fixtures.py` now reproduces every
seed vector/composite and is part of the normal architecture check. Seed-byte
checks do not certify normalization, provider schemas, PCM correctness or TLS.
D06 candidate strength comes from separate test-supplied identity evidence, never
provider search score; the incomplete S04 transition must fail complete parsing.
The Opus version and other named external/semantic gates remain open for their
owning children. They do not block W4.1 contract work.
