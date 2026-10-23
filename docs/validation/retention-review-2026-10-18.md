# HL-01 review repairs — 2026-10-18

The [initial delivery report](retention-audit-2026-10-17.md) remains dated evidence.
Review reproduced two missing cases through the real offline provider workflow:
concurrent curators could race into an Asset IdempotencyConflict, and explicit
reacquisition of an archived cover failed forever with Conflict. This repair closes
those cases within the [bounded retention policy](../retention-audit.md).

## Implementation and regression evidence

Application policy uses an atomic provider-artwork materialization effect under
the archive writer. Existing content must match collection, subject, kind, media
type, byte length and SHA-256. Item-generation and live Job/cancellation checks
apply to reuse, restoration and creation. Explicit CAA image selection alone freezes
the exact archived revisions visible at admission, using an indexed read capped at
4,096 covers (overflow gives a manual-attachment alternative). Restoration preserves the tombstone and
payloads and publishes a new revision of the same Asset, retaining curated visibility
and alternative text. A stale restore cannot override a newer tombstone. Concurrent
identical metadata selection converges on the committed choice without replaying
other fields. Ordinary manual attachment commands retain their existing semantics.

New offline regression tests:

- `concurrent_cover_creation_converges_across_curators_and_metadata_selection`:
  two curators, one shared download, two processing workers with one process identity,
  completed requester Jobs, one Asset and one authority change; actual image reads,
  preserved manual title and reindex.
- `explicit_cover_reacquisition_restores_history_but_automatic_refresh_cannot`:
  archived provider cover with curated private visibility/description; a real
  automatic text/listing/image workflow leaves it archived with an artwork error;
  explicit reacquisition restores the same Asset and original bytes, keeps the exact
  historical tombstone, and remains stable through receipt replay, reindex and a
  later fresh lookup. An explicit restore queued before reopening still succeeds;
  a lookup queued before a later archive cannot restore it even with an unchanged
  clock, while a fresh command admitted afterward can.
- `provider_cover_materialization_fences_content_restore_revision_and_job_claim`:
  different subject/bytes, stale generation/restore revision, expired/cancelled
  claims and a later tombstone are denied; duplicate valid restoration creates no
  extra revision.
- `provider_cover_restoration_recovers_before_and_after_revision_commit`:
  faults before/after archive commit, reopening, retry and reindex retain usable
  original bytes and restore only once.

The Chrome smoke extends the earlier repeated-cover journey with archive, immediate
refresh, page reload and explicit selection of the same provider image through the
real UI. It checks stable Snapshot/Asset identities and metadata revision, the
archive/restore revision pair, preserved visibility/description and a usable thumbnail.
The existing archive-control UI is exercised earlier in the same smoke journey.

## Validation results

| Gate | Result |
| --- | --- |
| Focused review regressions | All four passed individually and in the final workspace run |
| Architecture, documentation links and fixture manifests | Passed |
| Rust formatting and diff whitespace | Passed |
| `cargo test --workspace` | Passed; archive integration: 225 passed, 3 existing manual tests ignored; owned-disc oracle also remains ignored |
| Strict all-target workspace Clippy | Passed |
| Web type check/build and embedded server rebuild | Passed |
| Generated API contracts | Passed |
| Full real Chrome smoke | Passed; Google Chrome 153.0.8010.37, including archive/reload/reacquisition |

All validation uses temporary installations and local provider fixtures. No live
archive conversion, cleanup, external provider submission, commit or sibling-project
change is part of this repair. General history deletion and private replay expiry
remain outside HL-01's completed bounded scope.


The final full workspace run, strict Clippy, server rebuild and API checks were
rerun after the admission-bound restore permissions and indexed archived-cover
read were finalized. The final real Chrome run used that rebuilt server. Earlier
in-progress builds are not the acceptance result recorded in the table above.
