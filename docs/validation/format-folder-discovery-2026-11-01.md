# Digital format-folder discovery — 2026-11-01

> Deployment identifiers are anonymized; dates, source identities, outcomes and qualification limits are unchanged. Example addresses are not deployment instructions.

User-requested source delivery for Steam soundtrack layouts. Music owns path-only
intake policy; the existing browser/guided/bulk plans, review, source visibility,
exact-original publication and enrichment remain the consumers. There are no new
API/archive schemas, migrations, dependencies or sibling consumer changes.

## Supported behavior

- `Game Name OST/FLAC/01.flac` becomes one `Game Name OST` package, including
  parent cover files and nested booklet directories.
- Numbered Disc/CD folders work above or below the format wrapper, including
  mixed case and existing space/underscore/hyphen number separators.
- FLAC/MP3/WAV siblings remain distinct; review adds format labels when needed.
  Direct album tracks remain separate from explicit encoding branches.
- Nested named albums/sections and raw-disc anchors keep their own files.
  Original paths, input file order and default companion privacy are retained.
- Shared parent companions have one deterministic owner: direct tracks, if any,
  otherwise FLAC before MP3 before WAV. Format-local companions stay local.
  This is the existing disjoint intake contract, not duplication into every version.
- Only exact supported format names are wrappers. Decorated format names and
  arbitrary soundtrack section names are not guessed. Without a parent name,
  the format name is the fallback. No existing archived records are regrouped.

## Validation

Focused module fixtures cover adjacent albums,
parent/nested companions, both multi-disc arrangements, alternate encodings,
enumeration-independent ownership, unwrapped tracks, conservative names, root
folders and raw-disc anchors. The real-browser purchase journey adds folder
selection, three-package review, companion visibility, mobile overflow, distinct
Item publication, parent cover enrichment and immediate track refresh.

Passed checks:

- Focused discovery: nine tests, including the existing independent nested-package
  regression and five new Music fixtures.
- Workspace strict all-target Clippy; formatting; architecture/document links and
  fixture manifest checks; JavaScript syntax and diff whitespace checks.
- Web TypeScript/build and queue/context/presentation model tests.
- Rebuilt debug server/media fixtures; actual Chrome purchase
  (156.0.8078.74) and intake-review (153.0.8010.12) journeys, outside macOS seatbelt.
  Purchase artifacts:
  `<temp-dir>/collector-purchases-grfauI`.
  The 390px format-folder review screenshot was visually inspected; file paths
  wrap within the viewport. Stored originals include the private parent booklet
  and shared cover, and the three versions/albums have distinct Item IDs.

Full `cargo test --workspace --locked` passed: **596 passed, five existing
qualification tests ignored, zero failures**.

The selector preview conservatively classifies architecture documentation plus
browser fixtures as infrastructure. Manual selection uses the full Rust workspace
and affected purchase/intake browser journeys above. Unrelated broad browser
journeys, container/tooling qualification and native sibling tests are omitted:
there are no packaging, toolchain, shared wire-contract or native client changes.
Subsequent publication and application-host deployment were explicitly authorized and completed
as beta.16; see [release/deployment evidence](container-beta16-publication-2026-11-01.md).
