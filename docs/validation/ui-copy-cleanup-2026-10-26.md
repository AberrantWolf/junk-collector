# UI copy cleanup — 2026-10-26

Conservative presentation-only pass across 15 web components:

- Removed repeated preservation assurances from refresh/cancel feedback, catalog
  administration, documents, metadata browsing and import attachment messages.
- Removed the paragraph explaining ordinary Play/album/track queue behavior;
  kept action labels/tooltips, queue locality and unavailable-track feedback.
- Reduced metadata matching guidance to changing automatic matches and the
  exact-pressing limitation. The manual-precedence attention note remains.
- Listed independent rules for import grouping, physical-copy combination,
  replacement effects, inbox cleanup and recovery codes.
- Kept destructive-action confirmations, override-reset choices, grouping undo
  conditions, upload/recovery instructions, permissions and unavailable features.

No domain/application behavior, requests or persistence contracts change. Existing
uncommitted work, including release/Disc ID controls, remains intact. No commit,
publication or deployment is included.

Passed architecture/documentation checks, Rust formatting, workspace tests,
workspace Clippy with warnings denied, web type checking and the production web
build. Rebuilt `collector-server` with the updated embedded assets.

Chrome acceptance passed: full smoke (including metadata, grouping, playlists,
playback and mobile layout), full intake review/cleanup/progress, replacement
workflow and account usability/recovery. Visually inspected mobile metadata and
grouping screenshots; revised lists wrap within the existing panels.

Two kinds of existing browser assertions were corrected when encountered:
recovery-code counting/waiting now targets the named code list instead of all
instruction bullets; the ingest proposal check follows every page and requires
every intake peer, allowing valid peers created by earlier scenarios in the same
collection. The initial smoke/account attempts counted instruction bullets; the
initial intake attempt rejected two valid earlier peers. Reruns passed after
these test corrections. The equivalent recovery-code selector in FX-06 was also
scoped; that older full journey was not run. No production behavior changed.
