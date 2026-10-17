# Bounded HL-01 retention delivery — 2026-10-17

[Owning audit and policy](../retention-audit.md). This delivery uses temporary
installations and offline HTTP provider fixtures. No live archive, provider or
external submission is involved. Existing October 10 records remain dated evidence;
this report records the additional CAA scope and the current checks.

## Workflow and preservation evidence

`cover_art_retention_reuses_responses_with_item_owned_assets_and_durable_retries`
uses the real MusicBrainz/CAA adapters and the application/store workflow, with an
advancing clock. Ten successful artwork requests retain four distinct artwork
Snapshots: the original listing, image, changed image and negative listing. Repeated
listing/image/negative responses reuse IDs. Identical bytes under another query
are still distinct inputs; shared identity is collection-scoped by the existing
response-key contract. One shared MusicBrainz response seeds the two items.

Repeated image requests retain one automatic Asset for an item and do not advance
its authority revision; a second item gets its own Asset, and a second curator on
the first item does not create another cover. Changed artwork retains its new
Snapshot and Asset. Manual inventory title survives reindex. A pending request and
its original receipt survive reopening; it resumes against the earlier retained
response/Asset. A cancelled sibling stays cancelled. Revoked curation is denied.
An eleventh request with corrupt retained response bytes fails rather than reporting
unchanged success, and the previously selected materialized artwork remains usable.

The existing snapshot-conversion acceptance now exercises **both AccurateRip and
CAA** through the same implementation. It covers exact dry-run mappings and byte
accounting; unknown frozen fields/directory members; missing/corrupt backup and
newly unbacked state refusal; immutable historical IDs and payload-reference checksum
failure; reads, reindex and archive-only rebuild; and fresh response reuse with no
private acquisition history. Fault injection at Prepared, BytesStaged,
PayloadPublished and ProjectionApplied resumes the same plan/backup after reopening.
The pre-conversion verified backup is copied into empty roots after stopping the
original identity; exact archive/state tree hashes, original snapshot bytes, account
login and Owner grant are checked. Existing stopped-copy and recovery tests cover
originals, unknown module data, manual authority, loans/evidence and pending scratch.

No new archive/private schema or sibling shared-library change is needed. Existing
conversion sidecars/journals, publication, access and workflow recovery remain the
mechanisms. General history deletion, private replay expiry, spool purge and a
general backup/restore CLI are outside this bounded delivery and remain explicitly
unimplemented in the audit/queue.

## Browser acceptance

The full Chrome smoke includes a new real-UI journey: repeat Find cover art and
select the same acquired cover, wait for each durable Job, then immediately read
current collection state. Snapshot IDs, saved Asset IDs, artwork choice and authority
revision must remain unchanged. The existing suite also exercises setup, imports,
metadata, preparation, actual FLAC playback, lists, mobile layout, account/WebAuthn,
role and revocation behavior.

The first restricted-process run aborted at Chrome launch before UI execution.
The same temporary-installation smoke was rerun outside that restriction.

## Gate results

| Check | Result |
| --- | --- |
| Focused CAA workflow and duplicate conversion/recovery tests | Passed |
| Architecture, documentation links and fixture manifests | Passed |
| Rust formatting and diff whitespace | Passed |
| `cargo test --workspace` | Passed; archive integration: 221 passed, 3 existing manual tests ignored; owned-disc oracle also remains ignored |
| `cargo clippy --workspace --all-targets -- -D warnings` | Passed |
| Web type checking/build and Rust embedded-server rebuild | Passed |
| OpenAPI / generated TypeScript consistency | Passed |
| Full real Chrome smoke, including repeated-cover retention assertion | Passed; Google Chrome 153.0.8010.37 |

No commit or live archive conversion was performed. Physical LAN devices, live
provider coverage, general retention compaction and supported release qualification
are not established by these fixtures.


## Subsequent review — 2026-10-18

The October 17 checks above passed their original coverage, but review reproduced
concurrent creation failure and inability to reacquire an archived provider cover.
The [separate repair report](retention-review-2026-10-18.md) records their fixes and
new regression/validation results. The earlier sequential tests did not qualify
those lifecycle/concurrency cases.
