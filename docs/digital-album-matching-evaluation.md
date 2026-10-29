# Digital album matching: dependency evaluation

This is the earlier research record from 2026-10-29. The subsequent
[implementation delivery](validation/digital-album-matching-2026-10-29.md) records
the shared crate, application integrations and their validation. Assessment-only
statements below describe the state before that implementation.


Research completed 2026-10-29. This is an implementation recommendation, not a
delivered matching feature or a dependency change. [Remaining work](remaining-work.md)
is the only implementation queue. Existing [FLAC/MP3 purchase support](digital-purchases.md)
and its preservation, privacy and manual-authority rules remain applicable.

## Recommendation

Use community implementations for fingerprinting and MusicBrainz protocol models.
Keep Collector's acquisition transport, durable workflow, source validation,
authorization, metadata authority and album-ranking policy.

- Prefer **chromaprint-next after its published 0.1.0 streaming defect is fixed**.
  The evaluated current source is a viable implementation candidate; use an exact
  revision if implementation starts before a corrected release. Do not add an
  unpinned dependency or copy its algorithm into Collector. Native Chromaprint
  remains the comparison oracle and a fallback if target-platform qualification
  fails, rather than a second runtime implementation to maintain by default.
- Use **musicbrainz_rs 0.14.0 models behind the provider boundary**, with small
  adapter-owned response envelopes and explicit conversion into Music's records.
  Preserve our transport. Its request builders become attractive after the
  reproduced parameter-escaping defect is fixed; until then the existing
  `Url::query_pairs_mut()` mechanism is preferable.
- Do not replace Collector's MusicBrainz application services with the crate's
  HTTP convenience methods. That would remove necessary behavior or require
  rebuilding it around a second transport stack.

Prior-art assessment should have preceded the handwritten protocol models.
There is useful community code here, and extending our private wire structs for
every new MusicBrainz field would increase avoidable maintenance. Conversely,
Collector-specific records and workflow policy are intentional boundaries, not
competing implementations of the public API.

## Evaluated inputs and environment

All experiments ran outside the production workspace in an isolated Cargo home
and temporary checkouts on macOS 26.6.2, arm64, Rust 1.99.1. No application
dependency, live collection, sibling checkout or installed server was changed.

| Input | Exact evaluated version/source |
| --- | --- |
| Published chromaprint-next | 0.1.0, source revision `8a952942731aad8751aff27356ad0924e8565162` |
| Current chromaprint-next | `7a384eee09caf17b4f6241f481dd784c2e937d9a` |
| Native Chromaprint | 1.6.2, `3b38c100022bdfb49be7797c4828a476f1621d55` |
| musicbrainz_rs | Published 0.14.0, `b9eb1ea9629e0e3fc486f719bd865a2046b97d91` |
| Resolved API transport dependency | api_bindium 0.5.3, ureq 3.5.0 |
| Existing Collector integration | `collector-providers/src/metadata.rs`, Music module DTOs and application metadata services |

Sources: [chromaprint-next](https://github.com/attilagyorffy/chromaprint-next),
[native Chromaprint](https://github.com/acoustid/chromaprint),
[musicbrainz_rs](https://github.com/RustyNova016/musicbrainz_rs),
[published MusicBrainz package](https://crates.io/crates/musicbrainz_rs/0.14.0).
The published fingerprint crate and current repository source both identify as
0.1.0; the version string alone does not identify the tested implementation.

## Fingerprint comparison

Native code was compiled from source, initially with Apple's Accelerate FFT.
An additional native build forced KissFFT, which is also used by the upstream
comparison harness on Linux. This second build still ran on macOS; it is not Linux
or x86 qualification.

| Check | Current source | Published 0.1.0 |
| --- | --- | --- |
| Independent whole-buffer vectors against native | 160/160 equal | 160/160 equal |
| Independent streaming vectors, 997 samples per feed | 160/160 equal | 80/160 differ |
| Upstream comparison/configuration suite, Accelerate | 22 passed | 13 passed, 9 failed |
| Upstream comparison/configuration suite, KissFFT | 22 passed | Not repeated |
| Current-source library unit tests, optimized build | 37 passed | Not repeated |

The independent vectors use deterministic changing chords plus noise, with and
without leading/intermittent silence. Each is 12 seconds long. The matrix spans
8, 11.025, 22.05, 44.1, 48, 88.2, 96 and 192 kHz; mono/stereo; and all five
algorithms. Equality compares nonempty raw fingerprints, compressed strings and
hashes for whole buffers, plus raw fingerprints and compressed strings for
streaming. Empty fingerprints cannot pass the main comparison.

The release defect affects feeds split inside interleaved stereo frames.
Its debug build panics on partial frames; its optimized build loses samples,
producing different fingerprints. All 80 independent stereo streaming cases
differed. Whole-frame input did not exhibit this problem. Current source carries
partial frames across calls, and resets that state between streams. The upstream
fix is [f3a0804](https://github.com/attilagyorffy/chromaprint-next/commit/f3a0804).
The nine published-suite failures were partial-frame assertions, including the
algorithm tests; they do not demonstrate nine distinct algorithm defects.

The current-source comparison also covers reference PCM fixtures, known encoded
fingerprints, silence, encode/decode, random chunk boundaries, restart behavior,
resampler lengths and classifier constants. Those tests supplement the independent
vectors rather than replacing them.

Short-input probes produced no fingerprint for empty input, one sample or one
second at 11.025 kHz; three seconds of silence produced three subfingerprints.
Nonempty output alone therefore cannot establish useful identification. Invalid
zero channels and tested sample rates of 0/100 Hz were rejected. Collector must
still enforce its supported format bounds and meaningful-result policy.

In the final independent run, timed whole-buffer fingerprint generation totaled
about 140 ms for current Rust source versus 319 ms for native. These are synthetic
in-process timings, excluding generation, decoding, source validation and network
requests, with no statistical benchmark protocol. They are not an album-import
performance claim or a reason to choose the library on their own.

The Rust library accepts streamed interleaved i16 PCM. Its preprocessing buffers
are bounded, but accumulated fingerprints grow with audio duration. Bound input
duration, feed sizes and concurrency in our wrapper; checkpoint cancellation
between bounded decode/feed calls. Do not invoke its whole-file convenience API
on arbitrary untrusted inputs. Define float/24-bit-to-i16 conversion explicitly
and qualify it against native on actual decoded files before delivery.

The crate declares `MIT AND LGPL-2.1-or-later`, not a choice of MIT or LGPL. Resolve
the project's distribution obligations before adoption. It is also a small, young
project with unreleased fixes and architecture-specific unsafe SIMD. These checks
are compatibility evidence, not a memory-safety audit.

## MusicBrainz crate assessment

An isolated executable compiled **musicbrainz_rs 0.14.0 with default features
disabled**, built release lookup, paginated release-by-recording browse and search
requests, and deserialized responses without using the crate's network client.
`as_api_request()` exposes the URI, method, headers and body, so protocol building
can be separated from HTTP execution. Models can be deserialized directly from
our already bounded, retained response bytes.

One public response each was captured for release lookup, release search,
release-by-recording browse and Disc ID lookup. All four parsed successfully.
The sampled release was Portishead's *Dummy*; this checks actual CD response
shapes, not a representative digital-release catalog. A separate synthetic
digital-medium fixture retained recording IDs, positions and millisecond track
lengths. Browse pagination arguments (`limit=100`, `offset=100`) were generated
successfully. No exhaustive enumeration or online album-identification claim is
made.

### Concrete defects and constraints

1. **Search parameter escaping is incorrect.** A query containing
   `release:"A & B" AND artist:"X/Y"` generated a URI with an unescaped ampersand.
   Parsing that URI with `url` did not recover the submitted query. The defect is
   in api_bindium's use of `encode_special_query` for individual parameter values.
   That escapes a query component insufficiently for a parameter value. Correct
   form/parameter encoding is required; Lucene expression escaping is a separate
   concern. Keep our existing URL construction until a corrected dependency is
   tested. Do not repair its output by splitting and rejoining the broken URI.
2. **Models are stricter than our current subset.** A missing medium `id` fails
   deserialization; unknown language/script codes fail rather than round-trip.
   All sampled live responses passed. Existing deliberately minimal offline
   fixtures are not evidence of an upstream defect merely because they omit
   required public fields. Review actual retained snapshots before migration;
   preserve raw bytes and explicit parse failures, and upstream demonstrated
   real-world schema gaps rather than inventing identities or defaults.
3. **Envelope forward compatibility needs attention.** A synthetic additional
   field on `BrowseResult<Release>` caused rejection. Use a tiny tolerant envelope
   for count/offset/entities if needed, while retaining the community entity
   models. Unknown raw response fields remain in our source snapshot, not in
   reserialized crate DTOs.
4. **Transport defaults do not match Collector.** Rate limiting is an optional
   feature, disabled by default; its async limiter permits a burst of five, while
   the synchronous orchestration inspected does not call that limiter. Default
   retries are ten, retry handling singles out 503, and blocking sleeps have no
   Collector cancellation checkpoint. Async HTTP execution delegates blocking
   ureq work to worker threads. Dropping a future is not our durable cancellation
   or publication fence.
5. **There is no completely transport-free dependency mode.** Even without
   sync/async features, api_bindium/ureq are dependencies. This adds a second HTTP
   library to the dependency graph, though our bridge need not execute it. Assess
   that cost against schema maintenance savings; a future upstream models-only
   split would improve the fit.

The API client is configurable, and api_bindium has a body limit (10 MiB by
default). This assessment does not claim those features are absent. They do not
by themselves provide our 8-MiB response boundary, pinned/public DNS checks,
per-hop redirect policy, cancellation, shared cooldown, negative cache, raw-byte
snapshot publication or metadata authority.

### What to replace and what to keep

| Existing responsibility | Recommended owner |
| --- | --- |
| Private upstream release/artist/medium/track/recording/disc structs | Community entity models inside `collector-providers` |
| Simple response envelopes | Community models where tolerant; otherwise small adapter envelopes |
| Endpoint/include/search construction | Community builders after escaping fix; existing `url` encoding until then |
| HTTP/DNS/redirect/timeout/cooldown/cache/cancellation policy | Existing Collector provider transport |
| Exact response bytes and digest validation | Existing acquisition/snapshot pipeline |
| Portable normalized observations and validation | Music module, with explicit adapter conversion |
| Candidate ranking, manual precedence, grouping, publication/recovery | Existing application/module policy |
| CAA acquisition, original covers and Asset effects | Existing bounded streaming provider and Asset pipeline |

The handwritten public-response types occupy roughly 80 lines in the current
provider; the surrounding file also contains CAA and shared transport mechanisms.
A community crate will remove protocol upkeep, not most of that file. Do not
replace our domain DTOs with the crate's complete serializable object graph or
expose upstream enums in persisted/public Collector contracts.

For the existing integration, migrate its response parsing as part of the same
bounded matching delivery. Compare normalized observations on retained real and
offline fixtures: artist join phrases, language/script, release-group identity,
disc IDs, medium positions, track-title fallback, missing fields and unknown
fields. Include negative/non-JSON 404 responses, malformed successful bodies and
multi-medium ambiguity. Preserve old snapshots and interpret their original
schema; do not rewrite accepted candidate identities or silently discard data
because a new parser is stricter. New matching inputs require an explicit schema
evolution, not retroactive mutation of stable observations.

## Recommended complete user-facing delivery

Implement **Find matching albums** for supported FLAC/MP3 purchases, ending with
reviewed metadata/track mapping and artwork through the existing publication path.
It must work on already imported albums as well as newly imported purchases;
fingerprinting must not block preservation or original playback.

1. **Freeze the actual album inputs.** Use current file-track manifests, source
   file identities/digests, explicit inclusion choices and inventory revision.
   Tags and user-confirmed filenames/folder hints can support text search. Do not
   automatically send paths, private purchase data or placeholder titles.
2. **Compute missing fingerprints in background.** Add a narrow shared
   `junk-libs` mechanism over authorized readers/the existing audio decoder.
   Use default Chromaprint Test2 (algorithm ID 1), the first at most 120 seconds,
   and the full track duration for AcoustID lookup. Keep the analyzed time range,
   sample conversion and implementation/algorithm version explicit. Cache by
   exact file content identity plus algorithm/settings; store lookup/ranking
   state separately. Check consumed bytes through the existing frozen-source
   validation path, with cancellable bounded work. Do not hash a whole package
   separately for each track or during browse/admission/polling.
3. **Look up recording candidates through AcoustID.** Add one provider adapter to
   the existing acquisition registry, with an application key, bounded responses,
   provider-wide rate limiting and cancellation. Send fingerprints/durations,
   without audio uploads. Keep multiple returned recording alternatives and their
   provider scores. No fingerprint submission is needed for this feature.
4. **Discover releases and fetch their actual tracklists.** Browse releases by
   recording, union results across tracks, and combine with optional confirmed
   text/identifier search. Page within explicit request/candidate budgets; expose
   truncation as incomplete search. Fetch shortlisted releases with recordings,
   artist credits, release groups and relevant edition fields. Search summaries
   alone are insufficient. Existing normalized track observations retain only
   position/title; add recording ID, duration, medium/count/format and relevant
   edition fields as explicit immutable matching inputs.
5. **Rank albums with a one-to-one file-to-track assignment.** Count distinct
   matched files and candidate positions; one recording match cannot satisfy two
   different local files automatically. Respect repeated songs, alternatives,
   compilations and multi-disc placement. Compare whole-release track coverage,
   counts/order and durations, then supporting text and edition clues. Prefer
   digital media when supported by evidence without excluding an otherwise
   useful recording match. Missing, extra or ambiguous tracks stay visible.
   AcoustID scores are track evidence, not album-match probabilities. A release
   group can be clear while its exact edition remains unresolved.
6. **Review and apply through existing authority services.** Show artwork beside
   candidate text, track coverage, proposed mapping, edition differences and
   unresolved files. Require a reviewed release/mapping in the first delivery.
   Apply metadata and optional artwork through the existing application pipeline,
   retaining source attribution, manual authority and untouched masters. A
   release suggestion is not catalog verification, a CD TOC or AccurateRip evidence.

The parent workflow owns durable child admissions, resumes all unfinished pages,
shares equivalent acquisitions and retains first immutable job timestamps.
Cancellation, access loss, inventory edits and superseding requests fence every
continuation and final publication. The frontend presents progress and submits
review choices; it does not orchestrate a second provider pipeline.

Preserve reusable computed fingerprints and sources needed to explain accepted
metadata. Use bounded cache/diagnostic retention for unaccepted repeated lookups;
do not introduce permanent histories of identical responses or silently compact
existing records. Private file results must not leak through shared candidates
or artwork; provider disclosure must be explicit for restricted files.

### Acceptance before production adoption

- Run the same native/Rust comparisons on intended Linux arm64 and amd64 targets,
  including real supported FLAC/MP3 decoding, 16/24-bit conversion, clipped/short
  inputs, frame boundaries, cancellation during decode/feed and bounded resources.
- Exercise AcoustID with a configured application key and owned music. Measure
  recording/album discovery outcomes independently of bitwise fingerprint parity.
  No AcoustID request was made in this assessment; no application key was available.
- Use offline matching fixtures for absent/sparse/conflicting tags, out-of-order
  tracks, duplicate recordings/alternate encodings, compilations, missing/bonus
  tracks, multi-disc releases, ambiguous editions, missing AcoustID links,
  paginated/truncated results and malformed provider envelopes.
- Qualify the actual review/apply/artwork flow, immediate refresh, preserved manual
  edits, stale inventory conflicts, cancellation, last-owner semantics,
  restart/retry and reindex. Check affected junk-libs consumers, including Trash
  Amp. Follow [change-based validation](testing.md) for implementation.

AcoustID requires an application key and documents a maximum of three requests
per second and noncommercial free-service terms. MusicBrainz documents an average
one request per second per IP and a contact-bearing User-Agent. Configure these
through existing readiness/admin patterns rather than embedding example keys.
Sources: [AcoustID API](https://acoustid.org/webservice),
[MusicBrainz rate policy](https://musicbrainz.org/doc/MusicBrainz_API/Rate_Limiting),
[native fpcalc defaults](https://github.com/acoustid/chromaprint/blob/v1.6.2/src/cmd/fpcalc.cpp).

## Evidence and reproducibility

The [probe appendix](digital-album-matching-probe.md) preserves the independent
executable source, manifest, setup and captured result summary. It runs without
provider calls; live response parsing is optional. Temporary logs/checkouts were
left under `/tmp/jc-album-matching-eval` for this session and are not durable
collection data. They include the exact resolved Cargo lockfile.

Research checks above completed; the published crate failures are recorded as
failures. Repository changes are documentation only. Architecture/documentation
checks are the selected repository validation; workspace tests, Clippy, browser
journeys and deployment builds are omitted because no product code, dependencies
or served assets changed. Production digital matching remains unimplemented.
