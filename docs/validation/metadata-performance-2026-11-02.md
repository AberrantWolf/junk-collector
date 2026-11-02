# Responsive Metadata and digital matching — 2026-11-02 JST

The user requested a separate worktree implementation, repeated self-review and
repairs, then combined-main publication, a new beta and Fi installation. The
candidate includes the separate declared-WAV/trailing-original delivery from main.

## Ownership and behavior

Application services own the checkpointed fingerprint/AcoustID/MusicBrainz/review
workflow. Per-track processing Jobs share the configured 1–8 worker budget and
are fairly selected between albums. Provider dispatch has four reserved slots,
with one active request per service; each adapter still enforces its own request
spacing, refusal and cancellation. Dependent requests start when their inputs
arrive instead of waiting for every track or an unrelated website.

The Store stages and validates each fresh selected file through anonymous scratch
(maximum 4 GiB per active track). Shared music and PCM mechanisms decode only the
fingerprint window. Recorded total frames/duration and actual decoded/analyzed
frames are distinct. Cached fingerprints are scoped by collection, digest/length,
implementation and computation coordinates; the disposable cache is limited to
2,048 entries/32 MiB. Malformed or corrupt entries are not published as success.
Existing retained observations and accepted attribution are not compacted.

The shared provider registry reuses successful AcoustID, MusicBrainz and Cover Art
Archive listing requests for 24 hours, negative results for one hour, and supports
explicit MusicBrainz/AcoustID refresh. Verification datasets and explicit image
acquisition keep their own fresh-command semantics.
Consumed snapshot bytes still undergo their length/digest checks. Snapshot reuse
uses an expression index to select matching digest/request candidates, retaining
the existing safe treatment of legacy records and corrupt retained evidence.

Metadata lists select compact role/language/token-fenced projections without
hydrating captures or evidence. Pages append incrementally; cached pages revalidate
on reopening. Resolved metadata has a bounded memory cache. A lightweight revision
header tracks source/authority/artwork/inventory/access/language changes, including
related-album changes, without triggering a full review every poll. Optional
related-album details load on demand. Command responses refresh affected UI data.

Admission is owned by the signed-in session rather than the album component.
Receipts freeze submitted inputs, navigation cannot abort a pending SPA admission,
and unresolved requests remain visible/replayable. Restart recovers unfinished
work; terminal cancellation and last-owner behavior remain durable. Candidate
results are revisioned, bounded and available before artwork completes. Polling
omits unchanged results. All-file reviewed mapping and manual authority checks
remain the same. Search budgets are explicit; this is identity suggestion work.

## Qualification

Focused shared-library vectors, real-audio discovery/application/retry, three-album
queue/restart, orphan cancellation, independent-provider overlap, explicit refresh,
cache corruption/bounds and compact-header/row read tests have passed during
implementation. Full combined-source validation, final browser measurements and
release/deployment results are recorded below when completed.

The implementation uses shared-library revision
`280ddc7132eed41966720cdc93d608ecf54c2e94`. Its long synthetic FLAC vector confirms
prefix/complete-decode fingerprint equality and actual 120-second coverage;
existing independent native Chromaprint vectors remain passing. `fpcalc` is not
installed locally, so no same-machine Picard/fpcalc timing equivalence is claimed.
Actual Fi album recognition/provider latency and phone/hardware acceptance are
separate from offline fixture recognition and source qualification.


## Final review and combined local qualification — 2026-11-03 JST

Repeated self-review repaired child-admission/ownership races, convergent scan
identity, orphan cancellation after restart, final checkpoint persistence,
explicit-refresh isolation, cache policy scope, fresh-schema cache invalidation,
and bounded response/checkpoint growth. A subsequent review found no further
blocking issues. The album fixture identifies independent known fingerprints
rather than assuming arrival order, so parallel execution cannot swap titles.

The combined candidate includes the WAV delivery from `f53cbbb`. Workspace tests
pass **615 tests, zero failures, five existing ignores**. Strict workspace/all-target
Clippy, architecture/format, web types/build/models, API generation and release
policy/tooling/mail checks pass. All 30 selected browser journeys pass across the initial run and repaired
remainder. The focused
actual Chrome digital-matching journey passes, including partial/manual mapping,
artwork, stale-source protection and three queued albums after leaving the first
panel before its POST acknowledgement. Its compact Metadata requests measured
**15 ms and 2 ms locally**; these fixture timings are not a Fi album benchmark.
Mobile rendering was inspected for wrapping and overflow.

Shared music tests pass **20 tests**, including the long prefix/full fingerprint
comparison; shared strict Clippy passes. Isolated Trash Amp `amp-library` consumer
tests pass **58 tests with one existing ignore**, and consumer strict Clippy passes.
The original Trash Amp checkout remains clean. Shared revision `280ddc7` is merged
and pushed to its main; that repository has no GitHub Actions runs configured.

The full local report is `/private/tmp/collector-perf-full-checks.json`; logs and
release/deployment operation evidence are retained in `/private/tmp/collector-beta19`
and `/private/tmp/collector-perf-*.log`. Publication and Fi acceptance follow below.


The initial full run stopped in `fx04-review`: its synthetic full-review response
changed without the corresponding lightweight revision token. The fixture now
changes both consistently and restores both real routes before storage commands.
The repaired review journey and every remaining journey pass in
`/private/tmp/collector-perf-final-browser.log`. The original report deliberately
retains its failed status; earlier successful Rust/tooling checks and the first
12 browser journeys remain applicable. A final removal of unused former polling
cleanup passed web types/build and the affected browser journeys on rebuilt assets.
No selected local gate remains failed or omitted. Exact committed-source GitHub
qualification is still required before release.

Fi preflight is healthy on beta.18 with zero unfinished Jobs and saved worker
settings of eight processing/two copy workers. The new per-track pipeline uses
that existing budget; no production setting or Job was changed for benchmarking.


## Exact-source CI repair

The first main [Checks run 41471071689](https://github.com/AberrantWolf/junk-collector/actions/runs/41471071689)
passes its Rust checks but fails the broad integration journey when a transient
catalog lock response leaves a reopened album on a manual Retry screen. This is
a real read recovery gap. The Store now returns typed `CatalogBusy`; HTTP exposes
`503` / `catalog_busy`. The shared browser adapter retries only GET/HEAD, at most
three times with 100/200/300 ms delays, preserving cancellation and submitted
account identity. Exhaustion and permanent unavailability remain visible; writes
are not resubmitted by this policy. Model cases exercise recovery, exhaustion,
non-read/permanent failures, cancellation and account changes. A Store case holds
and releases the real catalog mutex, and the actual browse journey injects a
transient detail failure before reopening normally.

Review also corrected Metadata's one-time cursor restart to recognize the actual
`revision_conflict` code; its browser journey injects that response and requires
visible recovered rows. The previous candidate's image builds/runtime gates and
AMD64 release-image browser acceptance passed, but validation run 41471106794 was
cancelled after source acceptance failed. Those bytes were never published or
installed. The repaired source requires new exact-source qualification.

The repair passes the real contention/release Store test, web types/build/models,
strict workspace/all-target Clippy, architecture/format and focused actual Chrome
`integration`, `browse`, `digital-matching` and `storage` journeys. The latest
compact Metadata page timings are 4 ms and 2 ms locally. Self-review of retry
bounds, account/cancellation fences and cursor recovery found no further issue.
