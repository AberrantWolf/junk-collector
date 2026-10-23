# Pre-release contract and storage evolution

**Scope update (2026-10-09):** use [remaining work](../remaining-work.md)
and [home-lab policy](home-lab-scope.md). Older future contracts/order and blanket
history requirements below need reassessment. Preserve actual data/recovery and
valid current boundaries; Phono Junk is no longer a target.

Accepted 2026-09-11 after the user's W4.2 review request. This policy overrides
compatibility-only requirements in earlier wave plans, schema reservations and
API migration notes. It does not change product scope or preservation guarantees.

## Compatibility policy

Until an explicit supported-release baseline is declared, Rust interfaces, JSON
APIs, private databases, catalog projections and archive schemas may change together.
A wave/checkpoint is not a supported release. Keep useful format/version markers
for detection; do not allocate a new version or a permanent reader/adapter merely
because another child ticket is complete. Coordinate changes against the actual
checkout rather than reserving migration numbers for distant features.

Update all in-repository callers, generated contracts, fixtures and affected tests
in the same change. Remove obsolete interfaces when their callers are migrated;
no mandatory deprecation period, compatibility re-export, or API-major wait.
Do not remove working compatibility code as a cleanup project by itself; simplify
it when touching that boundary and it has no remaining data or consumer purpose.
Sibling projects and external formats are independent consumers: inspect them and
preserve their real requirements, rather than extending this policy to their data.

Existing user archives, original bytes, observations, overrides, loans, credentials,
trust and configuration remain valuable. Breaking format changes require a concrete
backup/conversion path or an actionable unsupported-format error before writes.
Never silently reset user state. Prefer one explicit, restartable conversion for
formats actually in use over a permanent chain of historical runtime readers.
Rebuild disposable catalog projections. Test recovery of the current format and
any conversion actually supplied; exhaustive support for every wave snapshot is
not an acceptance condition. Unknown module/provider data still round-trips.

Hash/recipe encodings identify real stored objects: deliberate encoding changes
must be versioned and existing affected references converted or retained explicitly.
Crash recovery, expected revisions, idempotence, access checks and preservation of
history are required now, regardless of compatibility policy.

## Define the shape early, implement behavior in complete workflows

Before each remaining root ticket, review its contract with its immediate producers
and consumers. Record these in the root's plan before implementation:

- Owning service/port and typed identities, input/output semantics and authority.
- Durable versus derived versus private data, publication point and cancellation.
- Role/exposure filtering, bounded pagination/streaming and structured failures.
- Retry identity, source currency and capability availability.
- One real end-to-end example and the most important denied/interrupted example.

Define shared concepts and narrow cross-layer contracts before their consumers.
Do not attempt to freeze every endpoint or provider field for the entire release.
Use a small executable workflow to validate an uncertain abstraction, then revise
its callers together. An implementing model must report a contract conflict and
revise the owning plan before introducing a parallel API or untyped escape hatch.

Do not ship placeholder success responses, empty-result stand-ins, fake jobs,
pretend playback or default no-op trait methods. Planned operations stay out of
production route registration and generated OpenAPI until implemented. A capability
may report a typed unavailable reason when that is real supported behavior. Tests
may use explicit fakes; those are not production implementations. Design-only
contracts must be labeled planned and must not inflate implementation status.

## Remaining wave checkpoints

Wave 4: D04/D05 review the operation/result validation contract and bounded snapshot
access before extraction; D06 reviews metadata authority and artwork selection;
D07 reviews immutable recipe/representation/media availability. P05 restores the
current archive/private format and any explicitly supported conversion, not every
historical wave. C01 defines the connection-keyed secret/transport and shared read
DTO boundary; S04/C02 extend the same trust/route model without obsolete adapters.

Wave 5: before C03–C07 integration, review a single typed client-service contract
for browse/artwork, media manifests, queues, download profiles, offline metadata,
cache identity and personal activity. Slint and Trash Amp consume these services;
neither gets a parallel policy model. Implement each complete workflow and update
all callers when the contract improves. P06 measures real queries and can reshape
projections/APIs with corresponding client changes.

Wave 6: R01 retains real Retro Junk import compatibility and original-source
preservation. R02/R03 document supported deployment/backup formats and run the
current release acceptance suite. Completion of these waves does not automatically
freeze the backend. Declare a supported baseline explicitly when sustained external
use/release readiness warrants it; record versions, upgrade support, backup/restore
rules and unsupported-format behavior at that point, then enforce that promise.
