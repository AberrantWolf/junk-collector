# Bounded retention audit — 2026-10-17

HL-01 delivery: audit the current consumers, retire blanket permanent lookup-history
requirements and extend the existing response policy to Cover Art Archive (CAA).
This is an owned provider policy, not a generic garbage collector. No live archive
cleanup is part of this delivery. The [single queue](remaining-work.md) remains the
only source of future assignments.

## Contracts and ownership

Application policy in `collector-app/src/acquisitions.rs` selects the providers for
which exact response reuse is justified: AccurateRip, MusicBrainz and now CAA.
`collector-store/src/processing.rs` supplies bounded, cancellable comparison and
validated byte publication; `snapshot_compaction.rs` supplies explicit stopped-server
conversion. Provider adapters and music-owned DTOs define interpretation. CAA
listing/image queries, status/outcome, schema version, image count and media type
are retained in that interpretation. Frontends continue using the existing durable
resolution command and its returned Snapshot ID; they do not invent a cache or
compose provider effects.

One home-lab outcome: looking up the same album cover again retains the existing
listing/image snapshots and the same item-owned automatic cover. Another item may
use the same response, but receives its own Asset. A second curator on the same
item reuses that automatic Asset, including concurrent requests. Changed image bytes or interpretation retain a
new snapshot and Asset. Explicit independent image attachments/publication commands
remain user actions and are not silently merged or removed.

Response equivalence binds collection, normalized acquisition input (including
operation, source fingerprint and response limit), provider, actual length/digest,
media type and complete parsed interpretation. Acquisition/request time and Job
identity do not create collection evidence. Only MusicBrainz's operational `refresh`
flag is excluded. No cross-collection reuse, inferred release equivalence, lossy
JSON normalization of raw bytes or verification-date advancement is allowed.
The worker checks consumed download and retained bytes before reuse. Browse and
admission still read recorded observations; they never hash originals.

## Retained, bounded and obsolete mechanisms

| Storage and actual consumers | Retention rule and disposition |
| --- | --- |
| Original Capture payloads and historical associations; authorized original/media readers, reimport and recovery | Must keep. Removal/reimport tombstones are not payload deletion. This delivery never converts or deletes masters. |
| Item/collection/Work/Edition revisions, specimens, loans and manual authority; restoration, historical graph validation and reindex | Must keep under the current format. A later revision does not establish that earlier user facts are redundant. |
| Accepted metadata/artwork, changed candidate versions and raw supporting sources; `metadata_view`, historical selection, `metadata_candidates`, authority validation during reindex | Keep current and historical claims and their sources. Current candidate projection is replaceable; archived candidates/revisions are not proven unreachable merely because no longer automatic. Preserve unknown authority/provider/module data. |
| Processing results/occurrences and reference snapshots; source-bound checks, UI history, canonical-result links and rebuild | Keep substantive results, coverage, exact coordinates and meaningful explicit rechecks. Existing occurrences are not deleted by age. Cached reuse creates no fresh check. One result/track never establishes complete package/disc verification. |
| Equivalent AccurateRip/MusicBrainz/CAA snapshots; provider response picker, metadata resolution, artwork preview and acquired-image materialization | Bound **new** storage to one validated snapshot per exact response identity. Changed results remain distinct. Historical duplicate envelopes/IDs stay readable; only their redundant payloads are eligible for explicit conversion. DAT/other providers are excluded pending an actual consumer assessment. |
| Automatic CAA Assets; item artwork selection and normal authorized artwork readers | One new materialized Asset per item and image Snapshot, independent of requester. Existing Assets remain intact, including old requester-derived identities. The first refresh after the identity correction can create one item-owned Asset; there is no automatic deletion or rewriting of old Assets/choices. Automatic refresh does not resurrect archived Assets. Explicit CAA image selection may restore the exact archived revision as a new revision of the same Asset, retaining tombstone history, payloads, visibility and alternative text. |
| Provider in-memory cache and ephemeral previews; request throttling and curation preview | Existing MusicBrainz cache is bounded to 128 entries/32 MiB with TTL; previews are transient, bounded reads and do not create Assets/evidence. No permanent lookup/access history is required. |
| Private command receipts/outcomes, Jobs, frozen input, acquisition owners/bindings, parent intents and clear/retry state; `commands.rs`, `acquisitions.rs`, Job retry and periodic workflow reconciliation | Keep existing rows until there is an explicit replay-expiry contract. A completed Job can still be needed by an uncertain command retry, later direct requester, generation binding or parent recovery. Dismissal changes presentation only. There is no demonstrated safe age-only deletion rule today. This retention is an implementation dependency, not a new perpetual product requirement. |
| Upload/catalog spool, provisional observations, staging and derivative scratch; resumable uploads, generic retry and publication recovery | Existing cleanup owns known completed scratch and explicit discard. Catalog-upload spools can still be needed by a failed Job retry. No time-based purge or unknown-file deletion. Anonymous download spool is temporary and removed when its handle closes. |
| Accounts, grants, trust/TLS/config, sessions, native credentials, enrollment/recovery records and security audit | Preserve private identity and access/recovery controls. Expiry denies access but is not proof an entire credential/recovery graph can be deleted. No security-state pruning in a provider simplification. |
| `catalog.sqlite`, indexes, browse/search/current candidates and checkpoints | Derived and rebuildable from retained archive records. Reindex may replace projections; checkpoints are acceleration, not an independent source of collection truth. Neither is an archive reachability oracle. |
| Archive/publication/command/conversion journals and conversion backup inventories | Keep until the owning recovery protocol settles; unknown or inconsistent state fails closed. Conversion receipts must survive replay. No blanket expired-log cleanup. |

The indexed entity-reference graph covers inventory entities, not every raw module,
metadata, snapshot, receipt or filesystem reference. It cannot authorize a general
mark-and-sweep deletion. Archive-only recovery intentionally lacks grants and private
retry state; a full backup must preserve both roots plus configuration. Repeated
lookup requests still create private operational Jobs/receipts under today's replay
contract; response reuse does not claim to bound that database's total growth.

The remaining growth mechanisms are concrete: authority revisions carry accumulated
observations; text-first then artwork resolution can publish separate authority
transitions; explicit rechecks add occurrences; distinct provider responses and
changed candidates add history (the selectable candidate ceiling is 4,096 per item).
This delivery bounds equivalent CAA responses and cover materialization, not all
authority history or distinct response storage. Historical revision validation and
selection currently consume that history, so removing it requires a format/reference
decision rather than an age cutoff. A CAA image Snapshot and its item Asset also
serve different consumers (provider interpretation versus authorized artwork/media
and thumbnail publication); sharing across those two storage kinds is not added
merely to avoid one intentional source/materialization pair.

## Explicitly retired proposed contracts

The earlier A04/P01/D06/FX-02 blanket interpretations that every provider fetch,
raw response, tentative version or equivalent occurrence is permanent collection
provenance are superseded by the table above and the
[home-lab decision](decisions/home-lab-scope.md). Retaining raw bytes that support a
claim remains required. Retaining another identical byte copy does not explain a
new claim. Existing meaningful dates, immutable identities and selectable history
remain valid data; their preservation is not evidence for requiring future duplicate
payloads. No new handoff, retention engine, Phono consumer or checkpoint-compatibility
wrapper is needed.

## Existing-record conversion and recovery

Use the [stopped-server conversion runbook](snapshot-duplicate-conversion.md).
Inspection reports each duplicate/canonical ID, collection, exact removed payload
bytes, added reference bytes and skipped records. It deletes nothing. Apply requires
both exclusive root locks, a verified full backup and the reviewed plan. Only
proven equivalent response payloads are replaced by durable checksummed portable
references. Envelopes, IDs, accepted metadata, manual choices, timestamps, credentials,
unknown data and pending work remain unchanged. Unknown fields/layouts, insufficient
legacy acquisition provenance and unfinished related work are ineligible; corruption
refuses conversion. Tiny negative responses can cost more bytes as references, so
inspection reports both quantities rather than claiming universal storage savings.

Reference publication, payload removal and projection rebuild use the existing
conversion journal and fault-tested replay. Full restoration copies the verified
pre-conversion backup into empty roots with the original instance stopped; never
overlay a running installation or start two restored private identities. Archive-only
rebuild resolves portable references without private acquisition history.

## Acceptance and boundary

See [dated validation](validation/retention-audit-2026-10-17.md). The audit and this
bounded provider simplification are complete when those checks pass. General
revision/candidate/occurrence/Asset deletion, private receipt/log expiration and
upload-spool reclamation are deliberately unimplemented: each needs a demonstrated
need and its own exact reference/replay policy before deletion. They remain visible
within HL-01, not a new queue or mandatory framework. General operator backup/restore
commands remain HL-03; this delivery reuses the existing conversion backup/recovery.


## Review repairs — 2026-10-18

The review found that the initial application read-before-create could race, and
that permanent Snapshot reuse made an archived cover impossible to reacquire.
Provider artwork now uses one writer-locked create/reuse/restore effect. It compares
collection, typed subject, kind, media type and actual input length/digest before
accepting an existing Asset, and rechecks Item generation and live Job claim.
Manual attachment creation retains its independent command semantics.

Application policy permits restoration only for explicit CAA image selection,
freezing the exact archived Asset revisions present at command admission. An indexed
read captures at most 4,096 cover revisions without payload reads; larger sets give
an actionable manual-attachment alternative. A lookup admitted before a later
archive cannot restore that later decision. Missing older frozen permissions grant
no restoration; selecting the image in a fresh command supplies current permission. The store publishes a new revision of
that Asset through the existing recoverable publisher. Concurrent restorers may
reuse the already restored effect; an intervening new tombstone invalidates a stale
restore revision. Automatic text/artwork refresh reports the archived cover as an
optional artwork failure with explicit-selection guidance, retaining the tombstone.
Concurrent equal cover selections accept the already committed choice while leaving
other metadata intact. The [dated repair validation](validation/retention-review-2026-10-18.md)
records regression, recovery and browser evidence. The October 17 gate report remains
historical; it did not cover the two defects found by review.
