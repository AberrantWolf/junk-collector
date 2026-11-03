# Incremental digital matching and independent review choices — 2026-11-03

Metadata review supports a manual storefront/distribution source for digital-only
file albums, such as Steam, Bandcamp or GOG. This is a collection fact independent
of a MusicBrainz disambiguation note: it neither selects nor fabricates an edition.
There is no batch application in this delivery.

Music owns the optional bounded `digital_source` metadata override; application
services own digital-only eligibility and effective manual field projection.
The existing metadata authority command enforces authorization and expected
revision. The web editor preserves other domain override fields, exposes save and
clear, and retains an unsaved draft on refresh/conflict. Source edits invalidate
older matching reviews under the existing metadata revision fence. Refresh and
release adoption preserve saved manual authority. Portable authority records and
reindex retain the field without conversion or changes to preserved originals.
Existing acquisition/purchase facts remain independent and unchanged.

## Incremental search and edition discovery

Fingerprinting and AcoustID recognition retain their existing durable concurrent
Jobs and source validation. Album discovery admits one useful MusicBrainz request
at a time: start with the strongest available recording, rank release summaries by
shared hits, digital format and track-count fit, then fetch and compare a release.
When a candidate fails, an unexplained/conflicting track supplies the next search.
A complete unambiguous one-to-one assignment with all scores at least 0.95 and all
known duration differences within two seconds stops additional per-recording
membership searches. This does not assert a unique edition or verified audio.

The first complete match is checkpointed while release-group browse finds other
editions, each of which must undergo its own detailed comparison. Consume known candidates
before requesting another edition page; a page advertising 200 editions need not
be fully traversed to satisfy an eight-detail budget. No release group
means stop after the complete match. Optional edition refusal is reported without
invalidating that match. Bound discovery to 32 recording searches, eight initial
or 32 expanded release details, and four release-group pages of at most 100
summaries. Advance group offsets by the actual response length; validate offsets
and group identity. Truncation/expansion stays explicit. Already-admitted work in
an existing checkpoint retains its ownership and is consumed safely; the new
policy cannot undo requests already sent by an earlier executable.

Comparison and bounded-result construction use the existing one-second cancellation
poller. Full claim, permissions, metadata revision and source currency checks occur
at execution/checkpoint boundaries. Recorded source resolution deduplicates per-file
references to the same original package. No browsing/polling path hashes audio.
Within one claim, ranking is recalculated only when local recordings or loaded
releases change. Every resumed claim ranks canonical checkpoint inputs, independent
of the bounded presentation payload. Complete matches precede weaker editions when
the response budget drops candidates.

## Independent artwork and mapping review

A release card can request artwork only. A separate release-ID input supports
artwork from a release absent from current metadata candidates. Both listing and
image selection use the existing provider registry, durable resolution workflow,
bounded snapshot reads and atomic item-owned Asset publication. Explicit image
selection retains existing archive-restore fencing and records the actual supplying
release in the Asset attribution. Selection changes manual artwork authority only;
it does not select the supplying release or apply its album/track text.

The mapping table aligns each local filename with release disc/track/title, duration
difference and recognition score. It includes unrecognized files and lists release
tracks without a local file. Application blockers show declared release totals,
local file totals and per-medium declared/received track counts instead of the
previous blanket instruction. Exact complete layouts retain the existing apply
rules; extra tracks and incomplete/ambiguous layouts are not silently accepted.
MusicBrainz's [browse documentation](https://musicbrainz.org/doc/MusicBrainz_API#Browse)
supports release-group discovery and actual-length pagination.

## Validation

The workspace run before the final edition-page demand adjustment passed
`cargo test --workspace --locked`: 620 tests,
five existing ignores (owned CD dump, subprocess fixture, generated scale and
manual processing benchmarks). The final page-demand adjustment is covered by
all application unit/doc tests and seven focused real-audio workflow tests, including
the 200-edition budget/bonus-track case. Its fixture needed a larger finite drain
bound (160 continuations instead of the previous 40) to cover the full detail/cover
workflow; it now settles within that bound. Strict workspace/all-target Clippy, formatting,
architecture/document links, generated OpenAPI/TypeScript consistency, web type
checking/build and all three web model suites pass. The change selector's preview
selected broad coverage; its source run was stopped after successful workspace
tests so browser dispatch could run outside macOS seatbelt. Remaining checks were
run explicitly. No selector receipt is represented as a complete release gate.

Focused application/storage fixtures exercise one recording-membership lookup for
a complete album, release-group follow-up, expansion after an unrelated seed,
partial recognition with explicit complete mapping, optional edition refusal,
shared multi-album processing, cancellation and restart. Source-only coverage
checks invalid text, stale expected revisions, unrelated-field preservation,
clear, reindex and rejection on a non-digital album. Provider fixtures check
release-group query parameters and reject a response attributed to another group.

The browser journey exercises source save/clear/reopen and refresh/adoption
retention, artwork-only listing from a release card, image selection from an
otherwise unselected release, retention of album/track authority and source,
empty listing behavior, retained manual cover after re-adoption, incomplete release
counts, desktop/mobile rendering and multi-album navigation/admission. Concurrent lookup
assertions bind to their acknowledged Job IDs rather than assuming list order. Intermediate
runs exposed and corrected the new listing response bound, a test that assumed
lookup receipt ordering, and test polling that did not tolerate typed catalog
contention. An intermediate multi-album run under concurrent workspace-test load
reported a failed Job without its cause captured; the subsequent focused run
passed, and diagnostics now include the Job/error if it recurs. All 30 selected browser journeys subsequently passed outside macOS seatbelt
(582.56 seconds combined), including real playback, authentication/role/exposure,
metadata review, digital matching, imports, recovery and storage availability.
The final scoped table layout adjustment is additionally qualified by a repeat of
the digital matching journey with an explicit complete 28-track presentation fixture:
all 28 files map to distinct slots, application is enabled, desktop columns fit
without sideways scrolling, and mobile rows retain labels and avoid page overflow.
This fixture exercises presentation only; real provider/application publication is
qualified separately by the independent two-track audio workflow in the same test.
No test applies the synthetic 28-track fixture as real collection metadata.

## Comparison measurement

An ad hoc probe linked the same debug shared library, using 256 unique recording
IDs, one-to-one complete layouts and known equal durations. Median of five runs:

| Releases | Comparison time | Algorithm checkpoints |
| --- | --- | --- |
| 1 | 3.02 ms | 771 |
| 8 | 24.11 ms | 6,168 |
| 32 | 95.94 ms | 24,672 |

These are in-memory algorithm measurements, excluding storage guards, fingerprints,
providers and network latency; they are not Fi or deployment timings. The previous
application callback performed full album/source validation at each checkpoint.
The new callback uses bounded cancellation polling and retains full boundary
validation. The real-audio provider request-count fixtures are two-track albums;
the 256-track measurement does not establish large-album provider acceptance.

No deployment, production lookup, installed-server change, shared-library revision
or sibling-project modification is included.

## Six-finding repair and second review — 2026-11-03

The follow-up review reproduced artwork recovery selecting an earlier release's
image, an optional edition detail refusal disabling a complete match, and a bounded
result dropping the only strong match while the persisted rank cache prevented
recomputation. It also found post-seed membership expansion before pending
recognition settled, checkpoint JSON decoding under the shared catalog mutex, and
global reconciliation writes in every idle worker poll.

Repairs recover artwork stages from frozen server-owned operations and order
history by admission, including empty/pending listings. UI continuations are fenced
and choices carry their listing Job. Resumed matching ranks canonical inputs and
prioritizes complete candidates for presentation. Discovery waits for remaining
recognition after the first seed. Optional edition detail refusal reports an edition
error, retains the usable match, and stops further edition downloads; source,
snapshot-envelope and cancellation checks retain their separate failure paths.
Checkpoint parsing occurs after releasing the database mutex. Idle workers no longer
reconcile globally; completed/yielded work wakes game/provider continuations and the
single five-second recovery owner handles interruptions. Claim preflight remains
read-only when no work is eligible, avoiding SQLite write reservations on idle
polls. Admission of an owner of an already-completed cached download marks it
runnable in that same transaction, without waiting for a later recovery tick.
Cancellation reuses the
same transaction-scoped provider reconciliation, retaining shared-owner fences.

New regressions cover running artwork-listing reopen with a different prior image,
newest empty-listing reopen, delayed remaining recognition with multiple first-track
associations, edition detail refusal with successful later application, canonical
ranking after real projection truncation, catalog availability during 4 MiB
checkpoint decoding, idle polling with database writes disabled, and periodic
continuation recovery. A valid edition exceeding the matching track bound is
skipped without invalidating the usable match. Existing
cancellation, shared ownership, retry, multi-album and game-scan tests also apply.
Validation and second-review results are recorded below.

The second review replaced full frozen-input deserialization on resolution-history
reads with a compact provider/query projection, kept active Jobs in the bounded
history window, and retained newest-admission ordering for artwork choices. A
superseded older listing cannot overwrite newer choices. It also checked admission
against an already-terminal provider: existing advancing-clock cover-retention
regressions exposed that waiting admission and now qualify the immediate-ready
repair. The shared-owner cancellation test now holds provider acknowledgement with
a bounded fixture gate, checking that last-owner cancellation reaches the provider
without recovery polling and that new ownership is refused before settlement.
The prior test relied on a race before provider acknowledgement and could fail
when cancellation became immediate. Its no-resurrection guarantee is retained.

Source qualification: the workspace run passes 626 Rust tests with five existing
ignored tests. After the second review's final claim-decoding lock-scope adjustment,
all 32 Store unit tests, 15 claim/recovery tests, ten worker/budget tests and nine
digital-album workflows pass; strict workspace/all-target Clippy, API consistency,
format, architecture/link checks, web type/build/model checks and the rebuilt
server/media artifacts also pass. The final claim change releases catalog and
dispatcher guards before frozen-input decoding and avoids parsing the request
twice. The idle regression covers both an empty runnable queue and exhausted
processing capacity with SQLite writes disabled.

Intermediate full runs exposed two cached-artwork admission waits, plus fixture
issues: the new refusal fixture also rejected its unrelated setup release, a
request-count assertion included that setup download, and the old cancellation
assertion raced provider settlement. Those are repaired and qualified above.
Browser runs concurrent with workspace tests encountered catalog contention in
raw test fetches; the album reader now retries only typed `catalog_busy` and
reports other response errors directly. Final browser qualification runs without
competing workspace tests. Reopened empty-listing acceptance waits for its actual
history response and rendered cover controls before asserting no older options.

Final browser qualification passes all 30 journeys (587.18 seconds combined) on
the rebuilt server, including broad integration, real media, ownership/cancellation,
metadata/artwork recovery and storage availability. The final strengthened album
journey also passes separately (27.70 seconds), including its history-response and
render barrier for empty artwork recovery; its reopened cover screenshot was
visually inspected. Desktop/mobile 28-track mapping screenshots remain readable
without sideways page scrolling. Only that test synchronization changed after the
30-journey run; application source/artifacts remained unchanged.

The completed second review leaves no unresolved actionable findings in these
changes. Existing omissions are five ignored Rust qualifications and one
container-validation qualification gate. No container image,
production lookup, deployment, commit or push is claimed by this source repair.
