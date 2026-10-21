# Document attachment acceptance — 2026-10-21

Post-alpha.3 source delivery; published images and production installations are
unchanged. PDF and UTF-8 text documents up to 20 MiB use the existing Asset subject,
revision, visibility, archive and authorized-resource pipeline. Documents cannot
be selected as artwork. No private/catalog schema migration is required.

The Documents editor defaults to Curators only. Explicit sharing allows Viewers to
read/download; unrelated collections and users remain denied. Text previews read
at most 128 KiB plus bounded Unicode lookahead and expose a typed truncation flag;
React renders escaped plain text. The download preserves the complete original.
PDF recognition checks signatures only, not structural validity. An explicit link
opens the browser PDF viewer; there is no server renderer, invented thumbnail or
successful-looking substitute for unsupported previews.

Shared classification and bounded reader mechanics are published in junk-libs at
`193e0a88faaa0c3d5636976932f120c237831ad0` on `codex/document-previews`, signed with
the operator’s new noninteractive local Git setup. Collector pins that revision
without local patches. Core/disc/audio consumer tests, strict Clippy and formatting
passed. Independent fixtures cover PDF recognition, empty/oversize/binary input,
Unicode boundaries, malformed UTF-8 and bounded reads. The tiny PDF committed here
was generated for this test and contains only “Collector test manual”.

Store and actual HTTP regressions pass original-byte/digest retention through
reindex, no fake thumbnails or artwork, private/shared preview access, cross-user
and collection denial, original ranges, idempotent uploads and archive visibility.
A valid text upload replaces the earlier explicitly unsupported Document case;
binary documents still fail without publication. Existing image behavior remains
covered by the prior regressions.

Actual Chrome 155 against an isolated fresh installation passed setup and manual
entry creation, the private default, PDF upload, lost-response retry with one
stable command key and no duplicate Asset, browser-native PDF viewing, escaped
Unicode text/truncation, mobile overflow, exact full original download, visibility
editing and archive. The test grants browser trust only to the disposable
certificate public key. Chrome’s viewer runs in its own internal frame; an early
test incorrectly expected its plugin in the outer page. The corrected test observes
the real viewer. No response interception remains except deliberate lost upload
responses; product framing/CSP headers were not relaxed.

Self-review covered authorization before opening bytes, historical visibility,
artwork exclusion, immutable originals, publication/replay, UTF-8/resource bounds,
module-independent read access and upload failure recovery. Unknown uploads retain
the submitted file/query/key and freeze inputs until retry resolves the receipt.
The browser’s existing automatic retry and an explicit retry all reuse that key.
Management panels have distinct keys; shared runtime imports avoid duplicate
React instances. New CSS is part of the served build.

All 459 workspace tests passed (four existing opt-in tests ignored), strict
all-target Clippy, Rust formatting, architecture/document/fixture checks, annotated
OpenAPI/TypeScript comparison, web type checking/build and embedded server rebuild
passed. The existing full Chrome smoke also passed real FLAC playback and
revocation, upload/bulk/provider recovery, artwork, lists, WebAuthn and mobile use.

This qualifies the named browser and generated fixtures. Native/browser-specific
PDF capabilities can vary; original downloads remain available. Arbitrary PDF
structural validity, OCR, thumbnails, annotations and broader document formats are
not implemented or claimed. No copyrighted or private document is committed.
