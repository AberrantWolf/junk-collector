# Shared music review — 2026-10-30

A fresh review of the junk-libs-music delivery found and corrected:

- Matching could label lists with track/medium gaps complete, and accept duplicate
  or zero medium positions. Completeness now requires contiguous coordinates;
  duplicate/zero media fail closed. Unpositioned tracks cannot invent disc zero.
- Empty track credits suppressed valid recording credits. Empty credits now fall
  back to the recording artist, preserving compilation track artists.
- Malformed track identities/positions/text and successful AcoustID envelopes
  missing results could pass the provider adapter. They now fail explicitly.
- Fingerprint searchability could bypass invalid envelope coordinates. `usable()`
  now requires a valid envelope before reporting searchable audio.

The final shared revision is `0becbd0c382df3d494244c7ccc20de05cba580cf`.
Nineteen shared tests and strict all-feature/all-target Clippy pass. Independent
exhaustive enumeration of all 512 three-by-three recording graphs confirms
maximum-cardinality assignment and ambiguity, and native fingerprint vectors
remain unchanged. Collector and Trash Amp manifests/lockfiles pin that exact
published revision. Loading persisted reviews also downgrades stale completeness
flags while retaining their track data, without upgrading incomplete evidence. No audio masters, schemas or manual metadata are rewritten.

Validation of the final pin passes:

- Shared crate: 19 tests and strict all-feature/all-target Clippy; core-only,
  fingerprint-only and MusicBrainz-only feature configurations also pass.
- Collector: the full `infra` change selector passes architecture/links, format,
  web checking/build, testing/container/release policy checks, 538 workspace
  tests, strict workspace Clippy, web models and all 28 browser journeys. Five
  existing opt-in/scale tests remain ignored.
- Trash Amp: 236 workspace tests, formatting and strict all-target Clippy pass;
  three existing opt-in tests remain ignored. Main was updated in
  `596bd5aa66e4c3a2611004fcc9f85c66b3f96a63`.

An initial browser attempt used stale default-target assets. Final validation
used explicit freshly rebuilt server/media executables and passed the served
asset checks. Native and browser validation ran outside macOS seatbelt. No
platform/live-service acceptance was inferred from offline fixtures.
The previous delivery record remains historical evidence; live AcoustID and
additional platform/codec qualification remain outside this review.
