# Original provider artwork — 2026-10-26 JST

## Beta.10 release follow-up — 2026-10-27

Included in the signed beta.10 source release. Collector core/media are deployed
on the NAS after complete exact-source and native image qualification. See [publication, recovery
point and actual-host acceptance](container-beta10-publication-2026-10-27.md).
The dated source-delivery results and unrun physical/live cases below are retained
as historical evidence; no live-library batch or physical capture/eject was run
as part of deployment.

## Original source delivery

## Problem and scope

The NAS SQEX-10945 import preserved its originals and prepared all nine FLAC
tracks, but Cover Art Archive's primary original exceeded the former 20 MiB
limit. Header inspection found a 61,037,975-byte PNG, 5625×5625, 8-bit RGB.
The other Front/Booklet original was 27,577,716 bytes and 5641×2813; its different
composition is not a substitute merely because it is smaller. The user requested
repairing artwork acquisition and is handling GLS-0130-02's separate C2-error rip.

This is source delivery. No NAS data, images, configuration or running services
were changed, and no release or deployment is claimed. Existing lookup failures
remain historical records; after installing this change, request the artwork
again through Metadata rather than reimporting originals.

## Delivered behavior and boundaries

- Provider originals accept up to **128 MiB**, independently of the **20 MiB**
  interactive asset/document upload limit. Downloaded bytes and pixels are retained
  unchanged; provider thumbnails are never silently substituted.
- Production CAA composition uses the existing streaming acquisition registry.
  Its adapter checks declared size, spools and SHA-256 hashes 64 KiB chunks with
  cancellation, and supplies a local reader to the normal verified snapshot
  publication pipeline. Temporary spools close on failure/cancellation. Error
  bodies retain the smaller of the request limit and the JSON body bound.
- The application considers standalone primary front covers from the frozen
  listing. For multiple choices, inspect at most sixteen originals with HEAD
  through the same pinned redirect/DNS policy. Prefer approved originals within
  the bound, then smaller known lengths; absent/unsupported HEAD sizes remain
  unknown. Removed choices are skipped; refusal stops inspection. Actual transfer
  bounds remain mandatory even after HEAD.
- Freeze the chosen release/image in the parent's existing private processing
  checkpoint under its live claim, before admitting shared image acquisition.
  Continuations and restart reuse that selection. Original command requests,
  timestamps, source snapshots and manual authority remain intact. The application
  checks the retained selection against the frozen listing before consuming it.
- Explicit selection acquires that exact image. Oversize errors include byte
  counts and point to another cover or manual upload. The existing failed-lookup
  panel supports dismissal before a fresh choice. Dismissal persists locally per
  account/album (up to 64 IDs); reopening still resumes active work, while an old
  dismissed failure cannot take over the panel after successful recovery. Server
  history is retained.
- Materialization verifies the snapshot length/digest and uses the existing
  bounded decoder: 12,000 pixels per dimension and 256 MiB allocation, then a
  192-pixel local thumbnail. This stage still consumes a bounded in-memory image
  snapshot; network acquisition does not buffer the complete original.
- Item/generation/claim fences, cross-collection and role checks, manual artwork
  authority, equivalent-response reuse, explicit restore and reindex still use
  the established pipeline. No stored-data conversion is required.

The byte limit is a fixed safety bound, not a new operator setting. Size inspection
is advisory and does not establish visual equivalence or image quality. An image
can fit the transfer bound and still fail the independent decoder limits.

## Validation

Passing acceptance:

- Architecture/documentation links and fixture manifests; Rust formatting;
  complete working-tree whitespace review.
- `cargo test --workspace --locked` (exit 0), including **281 passed / 3 declared
  ignored** in the archive suite. The ignored archive cases remain optional
  scale/performance qualifications, not passed acceptance. The owned-CD oracle
  also remains unrun without its declared independent input.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` (exit 0).
- Web type checking/build and model suites; rebuilt embedded server/media test
  executables.
- Actual Chrome **156.0.8078.47** short smoke and the retained integration journey
  (189.89 seconds), including the new original-image and failed-lookup recovery
  checks. Browse passed separately in 24.81 seconds using its bundled Chromium
  **153.0.8010.12**; documents passed in 5.14 seconds in Chrome 156. Mobile error
  wrapping/overflow and recovery controls were inspected at 390×844.
- Test-selection/provenance, container-tool, release-policy and validation-tool
  checks pass. Their rootless-skopeo-dependent test remains explicitly skipped
  because that prerequisite is absent; it is not container qualification.

Initial acceptance attempts were unsuccessful. One browser attempt omitted the
existing Dismiss lookup step; after correcting that workflow, reloading exposed
an actual defect: dismissed historical failures were promoted back into the active
panel. The UI fix and final integration run cover that recovery. A subsequent
combined run passed smoke/integration but timed out at the existing sparse-page
browse check; the unchanged browse/document suites passed when run separately.
The timeout's cause was not established. An older workspace run was deliberately
stopped after a final provider error-body-bound correction; the fresh full run
above completed successfully. Early new selection fixtures incorrectly assumed
that refreshing artwork replaced a manually selected immutable candidate; the
size-selection assertion now exercises the existing complete guided-import
journey and retains the established manual-authority behavior.

Omitted: the other named browser journeys, optional scale runs, container builds,
release publication, NAS deployment and a live acquisition of the 61 MB image.
The independent large PNG and real UI exercise the size boundary without live
provider dependencies. No shared junk-libs code or sibling orchestration changed.

The complete working tree also includes separately requested Library import-progress
and raw-CD capture changes. They were preserved, and the change selector was
previewed against staged, unstaged and untracked changes. Storage, provider,
dependency placement and internal contracts require workspace coverage. Checks
are against the working tree, not a frozen release candidate.

Focused regressions cover a valid 2700×2700 PNG above 20 MiB, unchanged originals,
local thumbnails, the independent manual upload cap and reindex; HEAD lengths and
redirects, absent/unsupported sizes, refusal, unknown-length transfers, truncation,
MIME failures and cancellation; claimed selection persistence through restart and
stale/cancelled claim denial. The existing guided-import retry journey now offers
a booklet spread and two original fronts, verifies that only the standalone
fronts are probed, and proves selection of the smaller original without repeated
probes across continuations.

The real browser integration journey exercises an oversize failure, mobile error
layout, Dismiss lookup and a fresh cover selection, then compares the downloaded
original's length and SHA-256 to an independent valid PNG above 20 MiB. Existing
repeat/reload/restore assertions continue against that original.
