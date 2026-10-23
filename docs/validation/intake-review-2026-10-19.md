# HL-07A multi-disc inbox review — 2026-10-19

Local acceptance uses temporary installations and deterministic offline MusicBrainz/
CAA fixtures. No live NAS archive, source inbox or credentials were changed.

## Delivered behavior

- Batch review selects CUE or Redumper anchors through the existing shared readiness
  service. Replanning retains all originals, reviewed visibility, grouping and copy
  choices; neither view needs moving out of the source package.
- Album group keys identify a shared destination independently of optional manual
  titles. Automatic titles stay automatic; explicit group titles remain authoritative.
  Mixed manual/automatic title policies in one saved group fail before admission.
- Physical-copy labels have no numeric per-disc default. Matching reviewed labels
  within one album/intake share one copy; distinct labels create separate copies.
  Album grouping, copy identity and disc position stay independent.
- New intake ignores `.DS_Store` files and `#recycle`/`@eaDir` directory subtrees.
  The chooser, browser discovery, planning and actual inbox staging share this narrow
  policy. Other hidden files and files merely named `@eaDir` remain originals.
- Previously accepted inventories retain declared housekeeping files during staging
  and startup recovery. Existing archive records are not pruned or regrouped.

## Acceptance and review

`npm --prefix web run test:intake-review` uses actual Chrome 153.0.8010.37 and a
real server over test-only self-signed HTTPS. It performs real setup and collection
creation, selects two inbox packages containing CUE/BIN plus an alternate TOC view,
reviews privacy and anchors, reloads the draft and imports the pair. It verifies
one album/one explicitly chosen copy with automatic provider title/artwork, then a
second import with an explicit manual title and two distinct purchased copies.
Every saved original is downloaded and compared to its source bytes. Server restart
preserves titles, captures and copies. No provider success is asserted without a
matching fixture response. These synthetic packages do not qualify Redumper decoding,
CD audio fidelity, MusicBrainz identity as verification, or actual NAS ingress.

Rust regressions cover housekeeping bounds/unknown files, explicit-anchor index
remapping, old accepted housekeeping staging, mixed title-policy rejection before
Job admission, and automatic title authority through failed publication/retry,
restart and reindex. Existing grouped-publication tests retain manual-inventory and
unrelated-evidence conflicts. Separate-copy publication passes through the real UI.

Self-review found mobile horizontal squeezing/overflow in the batch row. Review
choices now use a responsive grid with separate package details and a single-column
mobile layout. The final desktop/mobile screenshots were visually reviewed;
keyboard submission and the full rerun passed. Artifacts: temporary installation
`collector-intake-review-xogEgo` (both automatic/manual review screenshots).

## Checks

Passed: architecture/link checks, formatting, workspace
tests, workspace Clippy with warnings denied, generated API consistency, TypeScript
check, production web build and rebuilt Rust server. Targeted new regressions passed
in addition to the initial workspace run. The full browser smoke suite passed,
including WebAuthn, role denials, interrupted folder uploads, provider work, actual
playback/seek/revocation, grouping, lists and mobile/keyboard review. The focused
HL-07A journey passed again after the responsive CSS fix and embedded-asset rebuild.

The permanent NAS deployment, second-device/trusted-HTTPS qualification and optional
source cleanup remain HL-05, HL-04 and HL-09 respectively. No source deletion,
provider scraping or new storage migration was introduced.
