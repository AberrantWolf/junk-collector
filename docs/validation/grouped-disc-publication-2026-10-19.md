# Grouped-disc publication revision recovery — 2026-10-19

The operator's NAS import of Octopath Traveller 0 disc 2 completed staging and
identification, failed Save originals with `revision conflict`, and failed again
through Retry this step. The existing failed import and inbox sources were left
in place. No successful NAS recovery is claimed by this record.

## Cause and change

Two sibling imports can hold the same reviewed Item revision. Publishing one
appends its integrity Evidence reference through an Item revision. The second
import then fails its unchanged revision fence; generic retry retains that fence.
The prior bulk test covered admission/group identities, not completed publication.
A new deterministic grouped preservation test failed on the original code.

Publication now reads digest-validated frozen/current Item revisions under the
archive writer. Added Evidence identities must belong to committed imports for
the same user, intake and destination. Application policy requires unchanged
reviewed inventory/domain/metadata, unchanged prior references and at least one
new sibling reference. Manual changes, deleted Items and unrelated evidence do
not bypass the fence. Capture/Asset records keep their independent revision rules.
The existing publication job and retry API share this path, so persisted failed
imports do not require conversion or new import/copy identities.

## Local acceptance

- Two already-staged grouped packages publish into one album and one physical
  copy despite the sibling's intervening Evidence revision.
- An already-failed publication with its original frozen expectation survives
  Store close/reopen and succeeds through `retry_ingest`, the API effect used by
  the web button. Both distinct original byte payloads remain in the archive;
  archive reindex retains the album, captures and single physical-copy identity.
- An intervening curator's physical-inventory note keeps the second publication
  conflicted after restart/retry, preserving the note and the first original.
- Policy vectors reject unrelated evidence, changes to prior Evidence digests,
  manual titles and unknown domain field changes.

These four targeted tests pass. They exercise actual Store/job/publication/retry
effects with tiny distinct preservation payloads, not Redumper decoding or live
MusicBrainz. This record does not claim successful disc identification, catalog
verification, full-disc audio verification or playback from those payloads.

Architecture/documentation/fixture checks, formatting, warning-denied all-target
workspace Clippy, TypeScript checking, web asset build and the rebuilt debug server
passed. Workspace tests were attempted normally and then with `--no-fail-fast` to
collect remaining results. This execution environment refuses local TCP listener
creation (`PermissionDenied` / `Operation not permitted`), so provider HTTP fixture,
LAN TLS and shutdown tests cannot pass here. A successful full workspace or real
browser smoke run is not claimed; run the full suite on the operator's normal
development host before treating the build as fully qualified.
The complete `--no-fail-fast` run finished: the archive suite had 230 passing,
6 failed and 3 ignored tests; every failure there was its HTTP fixture's denied
TCP bind. The four other failed targets were provider AccurateRip (6 tests),
provider metadata (8), LAN TLS (1) and shutdown (1), each with the same denied
socket creation. The four focused repair tests passed both in that run and again
after adding explicit archive-reindex assertions.
Actual NAS update, the failed Octopath import's retry, final grouping, metadata,
playback and new-installation backup/restore acceptance remain pending under
[HL-05](../remaining-work.md#hl-05--linux-nas-and-optional-public-internet-deployment).

## Operator build continuation — 23:02–23:06 JST

The operator reported success running `cargo test --workspace` and then building
`junk-collector:grouped-fix-20261019` for linux/amd64 in the normal Mac terminal.
Full logs/counts were not supplied; this is not an independently captured full-suite
pass. The operator subsequently confirmed transfer/load of that image on the NAS.
Portainer image replacement and live failed-import retry are still pending.
The pre-update stopped-server backup commands were supplied, but no completion
or validation result was returned. The user raised the full-archive-copy scaling
issue, now recorded under HL-05; no snapshot/incremental alternative has yet been
configured or qualified.

## Live NAS continuation — 23:09–23:13 JST

The operator provided `Verified backup` and `Backup valid: 401 files` output for
`/data/backups/before-grouped-fix-20261019`; the output explicitly includes private
credentials, TLS and pending-work scratch. This confirms backup creation and byte
validation, not a restored-instance drill. The tool-only SSH Compose file emitted
an orphan warning for Portainer's `junk-collector-cloudflared-1`; the operator was
instructed not to remove that required connector.

The operator then received instructions to change only the Collector image to
`junk-collector:grouped-fix-20261019` in Portainer, update without re-pulling, and
retry the existing disc-2 import. At 23:13 JST the operator reported that the fix
worked. This is actual-host operator-reported conflict recovery, without detailed
post-update logs or an independently captured final import state. Final disc/copy
grouping, metadata/artwork, playback/seek, original-byte comparisons and restore
remain separate acceptance checks; no catalog/audio verification is asserted.

At 23:15 JST the operator also reported working playback following the requested
Octopath disc-2 check. The exact track, seek result, final disc/copy inventory and
audio fidelity were not separately reported. This establishes operator-observed
playback through the deployed web workflow, not catalog or whole-disc audio
verification. Original download equality, independent backup and restore acceptance
remain outstanding for this permanent installation.
