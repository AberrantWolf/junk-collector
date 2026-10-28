# Metadata cover upload and focused rip-check results — 2026-10-28

## Delivery

The requested manual-cover entry point is directly beneath saved-image selection
in Metadata review. A shared browser attachment form supplies the existing
collection Asset endpoint; the metadata variant fixes the subject to the current
Item and the role to Cover, with shared/curator visibility, alternative text,
PNG/JPEG/WebP input and a local preview. The original stays preserved. Uploading
selects the new saved image; **Use selected artwork** commits the existing manual
authority command. Provider availability does not gate manual upload.

The existing artwork/document editor also uses this form. Frozen upload inputs
and the idempotency key survive unconfirmed responses within the mounted form;
retry resolves the same effect. Detach aborts observation and fences callbacks.
Errors remain visible at the upload form. File drafts participate in existing
unsaved-change handling. Periodic metadata reads preserve an unsaved cover choice
and text drafts until an explicit authority save replaces them.

For the user's follow-up, application member-read policy selects non-excluded
members for AccurateRip batches. The existing storage query applies this filter
before its row/byte bounds and cursor calculation. Exclusions therefore consume
neither transmitted rows nor displayed pages. Frozen membership and summary
counts are retained. Other batch kinds, job execution, admission, cancellation and
recovery policy are unchanged. No new API DTO, storage schema or conversion is
needed; existing batches get the same focused view.

## Acceptance

Passed architecture/document links, Rust format, TypeScript, web build, queue/
context/label model checks and workspace all-target Clippy with warnings denied.
The server and media fixture executables were rebuilt with the served assets.

Passed shared-harness real-browser journeys outside the macOS seatbelt context:

- Short smoke: authenticated library/artwork/search/detail, management assets,
  viewer denial and mobile overflow.
- Metadata review (`fx04-review`): real invalid-image rejection; simulated 413
  refusal with editable correction; original-byte preservation; lost-response
  retry with one receipt and one Asset; immediate saved-image selection;
  background-poll survival alongside an unsaved title; authority save/reload;
  write-disabled upload hiding; desktop and 390px mobile rendering. The provider
  candidate presentation fixture is synthetic; uploads/storage/authority are real.
- Documents: existing upload/private default, uncertain-response recovery, native
  PDF preview, exact download, visibility changes and archive.
- Browse: existing image attachment and responsive navigation/import updates.
- CD library: 104 excluded entries alongside one real uninspected CD; only the
  relevant result is transmitted/displayed, with no empty pagination; a second
  batch contains only exclusions after confirmation, with no rows, next page or
  repeated provider work. The final library check uses search because this
  enlarged fixture intentionally puts the CD beyond the first library page.

Desktop, mobile upload-preview and mobile saved-choice screenshots were visually
inspected in `web/test-results/metadata-cover-*.png`. The file chooser, long filename,
preview, visibility/alt-text controls and selected-image action fit their layouts.

The initial UI-only check selection was superseded by the second request. An
in-flight build failed while mixing an older application artifact with the new
storage filter; subsequent final-source compilation and Clippy passed. The first
CD-library browser run passed its new batch assertions and failed its old
first-library-page assumption; the corrected search-based journey passed.

The broad provider/media/recovery browser journey also passed (actual Chrome
156.0.8078.48): existing image archive/focus handling, metadata/provider acquisition,
shared request cancellation and reopening, audio playback/seek/revocation, bulk
library/grouping/preparation, collection playlists, virtual WebAuthn and roles.

`cargo test --workspace --locked` passed, including the new storage/application
regression over 301 excluded records, selected pending/terminal results, last-page
cursor behavior, all-excluded batches and unchanged metadata-batch visibility.
The existing automatic/one-click CD/recovery regression also passed with the new
excluded-member assertion. The archive suite reports 284 passed and three
pre-existing opt-in cases ignored; other workspace suites and doc tests passed.
Existing opt-in oracle/scale tests and the native subprocess helper retain their
normal ignored status; they are not claimed as newly qualified.
Unrelated named browser journeys, optional scale/device qualification and container
release builds are omitted for this local source change. Testing/container tooling
fixture checks passed; their optional rootless-skopeo test was skipped by its
availability guard. No release qualification is claimed.

No commit, push, release, deployment or production-library mutation is included.
