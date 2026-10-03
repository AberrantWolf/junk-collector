# Bulk import, metadata repair and music preparation

Date: 2026-10-03. BL1–BL3 local acceptance; see the
[delivery ledger](../bulk-library-plan.md). Tests use temporary profiles and offline
provider fixtures. Actual collection originals, private credentials, grants and
configuration were not modified during qualification.

## Repository gates

Architecture/document links/fixture manifests, Rust formatting, workspace tests,
workspace/all-target Clippy with warnings denied, web type checking, generated API
contract checking and embedded web build pass. The Rust server is rebuilt after
web assets. Chrome smoke covers the integrated journeys and the pre-existing
security, generic-module, import, metadata, processing and playback regressions.
The generated contract has 132 versioned paths.

The private real-CD oracle and generated timing experiments remain intentionally
ignored in ordinary workspace tests. The bulk timing experiment was run explicitly.
Temporary logs: `/tmp/junk-workspace-tests.log`, `/tmp/junk-clippy.log`,
`/tmp/junk-smoke.log`, `/tmp/junk-bulk-scale.log`, `/tmp/junk-group-kill.log`.

## Matching, metadata and inventory

Application fixtures cover authoritative disc layouts, same-album pressings,
language variants, genuinely different albums sharing a disc ID, incomplete album
identity (including one candidate), no match and name-only suggestions. Accepted
automatic information stays distinct from current suggestions and explicit selection.
Refresh preserves manual fields/artwork and exposes explicit field replacement.
Store tests cover durable language/receipt inputs, stale source and inventory fences,
partial artwork failure, restart/reindex and immutable observations/candidate versions.
Grouped entry envelopes explicitly retain every associated historical Evidence reference, including evidence owned by a merged source after its captures move. A captured-original regression checks reindex and undo. Older unstructured observations remain selectable; they cannot establish merge identity.
Existing DAT-backed game matching retains its source-fingerprint checks and rules.

Grouping fixtures cover same-collection confirmation, default distinct copies,
complementary mapped inventories, duplicate disc positions, manual information,
unknown data, identity/artwork conflicts, active loans and contradictory possession.
Missing physical components block physical-set assembly. Captures, assets and their
visibility, specimens/components/carriers and historical observations remain distinct
unless an explicit supported mapping applies. Source entries retain merge lineage.
Guarded undo succeeds before subsequent edits and is disabled after them.

The group publication suite injects failures at six stages. A separate subprocess
matrix kills the writer at every stage and verifies all-or-none archive visibility,
restart, complete projection recovery, replay of the same confirmation receipt and
undo. Immutable group manifests bind all metadata members; media is not read to
establish group visibility. A legacy browser fixture that deliberately rewrites a
synthetic archive now updates its synthetic commit digests as well; production
validation continues rejecting mutated committed group members.

## Durable bulk work

Tests exercise 130-package nested folder discovery and bounded pages, a 70-package
intake across admission pages/restart, incomplete manifest rejection, grouped
preconfirmed destinations, changed destination review/repair and unrelated package
continuation. Submitted expected package/file counts prevent starting a partial batch.
Session limits are 10,000 packages/100,000 files; manifests retain package limits and
bounded page/body sizes. Drafts and accepted manifests have stable command inputs.

Preparation tests freeze selection/exclusions/revisions/sources, admit gradually,
reuse outputs, isolate failed albums, retry only failed work, fence changed inventory
and parent claims, and recover per-member outcomes across restart. Damaged source
bytes do not make query/preview/admission read payloads; workers reject actual changed
bytes during processing. Shared ownership tests advance the clock between two bulk
requests and a later direct track request: cancelling one batch preserves the other;
last-owner cancellation cancels unused work while the direct request completes.
Generic parent cancellation shares the specialized effect; cancelled parents cannot
be retried into admitting more children. Root-admission failure exposes a typed error
and specialized retry without losing frozen membership or completed outcomes.

## Real browser journey

`web/tests/bulk-library-journey.mjs` is part of the complete Chrome smoke. It chooses
a browser folder containing two nested CD packages and an unassigned file, reviews
default automatic metadata/unchecked FLAC, interrupts upload, reloads and reselects
the entire folder, then follows both packages through preservation and resolution.
Unconfirmed packages remain distinct until explicit grouping review and confirmation.
It inspects the merged capture/copy graph, submits two FLAC tracks, reopens the whole
batch through Activity, follows completion, and plays actual prepared audio.
Metadata filters survive task navigation, return focus reaches the originating row,
keyboard confirmation works, mobile review does not overflow, and active audio keeps
its source and playback state across workspaces. The account journey saves Japanese metadata language, reopens it, checks persistence and restores English before the provider fixtures. The final Viewer artwork assertion now expects three shared covers (the original two plus the newly merged album), retaining its exact count and private-cover denial. Existing smoke separately exercises
retained upload prefixes, real seeking/queue advance, grant/session revocation,
unavailable module browsing and WebAuthn.

Visual checks inspect desktop and 390px mobile grouping screenshots under
`web/test-results/`. They reuse the existing visual tokens, show source artwork beside
text, and use a separate mobile review with a return action. Original bytes must
remain available in the browser until transfer finishes; server work continues after
accepted uploads. Reloaded browser uploads require reselecting files.

## Generated scale measurements

Run `cargo test -p collector-store --test archive bulk_generated_thousands_query_admission_and_restart_measurements -- --ignored --nocapture`.

Local macOS arm64, debug/test profile with the existing SHA-256 optimization,
temporary local filesystem. Fixture: 2,400 package manifests and 2,400 valid album
inventories; bounded discovery/manifest storage, intake admission, care listing,
frozen all-filter selection, rolling 16-member admission and checkpoint restart.
Admission makes zero provider requests and reads no media. Measurements are elapsed
wall time, one generated run; they are not production latency guarantees.

| Operation | Milliseconds |
| --- | ---: |
| Manifest acceptance | 255.3 |
| Intake admission | 640.8 |
| Care result page | 13.7 |
| Frozen 2,400-album selection | 1,816.7 |
| Rolling 16-member admission | 16.7 |
| Checkpoint restart | 204.4 |

This qualifies orchestration/paging/recovery at thousands of members. It does not
measure encoding thousands of CDs, multiple terabytes of audio, rich historical
inventories, NAS throughput or release-build performance. Real FLAC execution is
qualified separately in the browser journey and codec tests.

## Separate live-provider qualification

One read-only MusicBrainz disc-ID request using a recorded public identity returned
HTTP 200 (62,626 bytes), two releases belonging to one album identity and two media
containing the queried disc ID. One Cover Art Archive listing returned HTTP 200
(17,807 bytes), 23 image options with numeric identities. Contact used the project
URL; requests were bounded, and neither provider was retried. Public response-format
results are separate from the deterministic policy and complete browser fixtures;
no original audio was read or catalog/audio match claimed. DAT-o-MATIC was not
contacted and remains disabled pending its separate reviewed restoration.

LA3's trusted second physical LAN device, hardware/NAS scale and the wider release
qualification remain outstanding. Native/offline clients, new providers, general
post-edit splitting, backup UI and synchronized listening are outside BL1–BL3.
