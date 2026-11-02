# Album artwork consumer review — 2026-11-02

Requested after artwork was reported missing in Trash Amp while Collector web
still showed it. Collector baseline: `9d99a34`; native baseline: `02bf8a8`.
Native changes were prepared on `codex/album-art-query-flow` in
`/private/tmp/trash-amp-album-art`, then merged to Trash Amp main at `595710b`
on the user's request. The worktree is retained.

## Findings and source repair

The server's compact album projection supplies collection/Asset identity and the
Asset revision. The native adapter correctly uses the authenticated control-origin
thumbnail endpoint. Its existing real TLS purchase test passed before changes,
but explicitly called the artwork helper and bypassed the catalog orchestration.
No server wire/DTO change was needed.

Two confirmed native failure modes were repaired:

- Catalog orchestration started artwork only after a successful final title
  result. A title refresh that published useful rows and then exhausted its
  pending-summary wait skipped artwork entirely. A new catalog regression failed
  before the fix and passed after it. The last published rows now permit the
  separate artwork stage while the title error remains visible.
- Successful covers were withheld from the application until the whole sequential
  batch finished. Each downloaded cover is now committed and published immediately,
  under the existing source/request fence. Later cover failures or cancellation
  retain those successes. Failure reports include the first actual request reason.
  Artwork errors no longer initiate a cache-only reload that would replace the
  artwork request's own ticket.

The source-provider port supports incremental artwork publication, with its
existing batch implementation as the default for other providers. Provider
mechanisms remain in `amp-collector`; application admission and UI/playback
notification remain in `amp-app`. No audio reads, provider acquisition, schema,
credential policy or server behavior changed.

[Consumer documentation](../album-metadata-consumers.md) and the API entry point
now explain artwork IDs/revisions, authentication/origin, accepted Assets versus
provider previews, pending/absent/failed artwork and independent progressive reads.
Native architecture/connection documentation describes the repaired ownership.

## Validation

- Existing real TLS digital purchase artwork/stream/download/offline test passed
  before changes.
- The new catalog regression reproduced skipped artwork after partial title
  publication followed by failure; it now passes.
- `cargo test --locked -p amp-app -p amp-collector -p amp-ui -p amp-apple-library`
  passed: 140 tests, two existing ignored test entries. One ignored entry is the
  subprocess helper exercised by the direct-HTTPS parent; hardware audio output
  remains unrun. The tests include progressive application publication, later
  cover failure, server-side archived artwork, actual PNG bytes, durable partial
  success, cancellation and existing stale-request rejection. An initial new test
  expected a literal HTTP `404`; the shared client reports typed `not found`.
  Its assertion was corrected to that actual failure reason and the suite rerun.
- `cargo clippy --locked -p amp-app -p amp-collector -p amp-ui -p amp-apple-library
  -p trash-amp --all-targets -- -D warnings` passed.
- Native `cargo fmt --all -- --check` and both repositories' `git diff --check`
  passed. Collector's docs-only change selector previews architecture/link checks;
  those checks pass. Collector unit/browser suites are omitted for prose-only
  changes; no generated API/schema files changed.
- The development desktop executable was built and exercised through the actual
  macOS UI with an isolated TLS Collector fixture and temporary profile. Enrollment
  used the fixture's normal native enrollment/owner-approval APIs. Album browsing
  showed the imported cover beside its title and in album details, with lazy-loaded
  FLAC/MP3 tracks. Home → Refresh → Browse retained that cover and selected album.
  The fixture cover is an opaque black 1×1 PNG; the displayed black image agrees
  with those source bytes. This is rendering acceptance, not artwork aesthetics.
  The app/server were closed, temporary Keychain authority removed, and temporary
  harness source files removed. Development builds were not deployed.

Native tests use the existing exact Collector pin `fb035466`, which contains the
reviewed layered album/artwork contract. No dependency pins were changed. The
installed player/version, user's account-specific album responses and iPhone
hardware were not inspected. These confirmed failure modes do not establish the
sole cause of the reported live symptom. No production deployment, release packaging or iPhone installation is claimed.

## Requested main merge

Trash Amp main fast-forwarded from `02bf8a8` to `595710b`; the committed source
is the source validated above. The unrelated local `browser-item.slint` edit was
preserved and excluded. Collector's consumer documentation and artwork delivery
records are committed separately on its existing main branch; unrelated digital
album presentation changes remain outside that commit. No remote push or
installation is part of this merge.
