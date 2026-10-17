# Concurrent processing and provider response delivery — 2026-10-10

This delivery implements the user's selected defaults: two processing executions
and one provider acquisition globally, same-account processing, collection-scoped
request sharing, response reuse and explicit existing-payload conversion. No live
archive was used. Fixtures exercise actual application/store/provider paths; the
browser uses a temporary real server with offline AccurateRip/MusicBrainz/CAA origins.

## Recovery and preservation evidence

Targeted tests cover reserved download capacity with two processing claims from one
account, private cross-user Jobs, pre-registry queued request adoption with immutable timestamps, later owners with an advancing clock, last-owner
cancellation admission fencing, retry, restart, committed snapshot recovery and
parallel track publication to the same Capture. Existing source-validation,
subprocess encoding/validation cancellation and grant/generation publication tests
remain in force. Old bindings reject stale effects; a new generation consuming the
same retained snapshot remains eligible despite the historical binding.

AccurateRip tests distinguish changed responses and no-data, reuse repeated no-data,
and verify snapshot byte corruption fails closed. MusicBrainz direct requests and
metadata resolution share one acquisition, including differing operational refresh
flags. Current preference changes still resolve; an identical repeat creates no
new metadata authority revision or occurrence, including no-data metadata across
legacy response IDs. Expired claims cannot attach shared work or consume owners. Identity contract tests separate
response limits, outcomes and interpretation versions while ignoring requester IDs,
timestamps and JSON key order.

Conversion tests verify exact backup contents, refusal of missing/corrupt backup
and newly unbacked state files, preservation of historical IDs, reads, reindex and
archive-only rebuild. Fault injection at journal creation, reference publication,
payload removal and projection rebuild confirms replay of the same plan/backup.
Portable sidecars and original envelopes remain separate; canonical references
never replace identities. Fractional creation times precede the ID tie-break. Plans
report removed payload bytes and exact added reference bytes separately; tiny or
empty responses can cost more storage after conversion. Fresh downloads reuse converted legacy canonical responses, including after private state is absent in archive-only recovery. Unknown frozen acquisition fields are skipped when proving conversion/reuse inputs;
unknown snapshot-directory members prevent conversion and remain untouched. See [the operator runbook](../snapshot-duplicate-conversion.md).

## Local performance measurement

Manual ignored `processing_concurrency_local_benchmark` uses the real FLAC 1.5.0
process adapter and two 90-second tracks in a 31,752,093-byte preserved package.
Both configurations retain two complete source validations per track: 127,008,372
full-validation bytes, plus codec input/output and independent output validation.
No hashing optimization was added. Queue-to-claim is measured by 2ms polling.

| Measurement | One worker | Two workers |
| --- | ---: | ---: |
| Queue to claim, first/second track | 5 / 1027 ms | 1 / 4 ms |
| Album processing elapsed | 1970 ms | 1156 ms |
| CPU user / system, entire fixture test | 2.894 / 0.160 s | 2.987 / 0.156 s |
| Peak process RSS, test plus descendants | 83,132,416 bytes | 85,377,024 bytes |
| Complete source-validation bytes | 127,008,372 | 127,008,372 |

OS `wait4` measures whole fixture CPU/RSS (including import/setup), not just encoding
and not aggregate concurrent process RSS. Reads above are algorithmic full-package
validation accounting, not physical disk-sector IO; the filesystem cache matters.
An earlier run during full-suite load measured 3085/2438ms album elapsed. These are
local temporary-volume results, not NAS/hard-disk qualification. Parallel validation
can contend for disk bandwidth; retain the current validation and reassess hashing
separately using the actual home-server disk and owned albums.

## Repository and browser gates

Final gate results (2026-10-10): Browser coverage includes
real submission during two encodes, equivalent request sharing, panel reopen and
immediate refresh; existing cancellation, playback, refresh and account isolation
journeys remain enabled. Live provider/owned-disc qualification remains HL-05 and
is not established by these fixtures.

| Gate | Result |
| --- | --- |
| Architecture, documentation links and fixture manifests | Passed |
| Rust formatting and diff whitespace | Passed |
| `cargo test --workspace` | Passed; archive integration: 219 passed, 3 manual scale/benchmark tests ignored |
| `cargo clippy --workspace --all-targets -- -D warnings` | Passed |
| Generated OpenAPI / TypeScript contract consistency | Passed |
| Web type check and embedded asset build | Passed |
| Rust server rebuild after embedded assets | Passed |
| Real Chrome browser smoke | Passed |

The final workspace run includes the expired-acquisition-lease regression and
unknown-layout conversion refusal/recovery tests. The manual real-FLAC fixture
and one/two-worker benchmark were run separately, as recorded above. Other ignored
scale qualifications were not run for this delivery.

Browser smoke explicitly completed two same-account encodes with a reserved
provider download, shared waiting, cancellation of one requester, panel reopening
and unchanged refresh. It also passed actual FLAC playback, interrupted bulk upload,
metadata/grouping/preparation, collection-list mutation/recovery and mobile/account
journeys. No live archive conversion or live provider qualification was performed.

## Combined local commit qualification — 2026-10-10

The remaining music/provider/conversion work was checked together with the
committed automatic-game delivery and pushed shared revision
`83177af57ca36a5b284298020fea9d5e8fb0c415`. Locked workspace tests passed: 407 tests,
no failures, 4 existing manual/scale tests ignored. Strict all-target Clippy,
Rust formatting, architecture/documentation/fixture checks, generated API contracts,
web type checking/build and embedded-server rebuild passed. The focused real-browser
audio-check status and AccurateRip response-selection journeys passed again.
The preceding full smoke qualification remains recorded above. No live archive
conversion, external submission or additional push was performed in this commit.

## Later bounded retention delivery — 2026-10-17

The [HL-01 audit](../retention-audit.md) extends response reuse and explicit duplicate
payload conversion to Cover Art Archive, and binds automatic materialized covers
to item + image Snapshot. [Its dated validation](retention-audit-2026-10-17.md) records
the new provider workflow, conversion recovery and repository/browser checks; the
October 10 results above remain historical evidence for their original delivery.

## Cover reuse review repairs — 2026-10-18

The [repair report](retention-review-2026-10-18.md) adds concurrent cover creation,
explicit restoration with admission-bound archived revisions, rejection of newer
archive decisions, and restore-publication recovery. These cases were absent from
the original sequential reuse evidence; the dated October 10/17 results above are
retained without extending their original claims.
