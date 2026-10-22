# Container build-once promotion — 2026-10-22

State: source implemented; real CI qualification pending. Branch:
`codex/publish-validated-images`, following the completed beta.4 publication.

The user's authorized delivery replaces redundant publication rebuilds with
promotion of already accepted native images. The completed beta.4 signed tag,
registry images/channels, NAS and Tailscale remain unchanged. No new release or
production deployment is authorized for qualification.

Local checks passed:

- Ten provenance/archive/recovery tests in `check_container_validation.py`: full
  source/version/run binding; failed/untrusted/rerun identity; expired/mismatched
  immutable artifact metadata; ZIP digest, member/path and expansion controls;
  deterministic real OCI graph assembly with exact native config identity;
  wrong architecture/corrupt blob rejection; freshly inspected evidence matching;
  failed download cleanup and retry; missing artifacts never launching work.
- Existing six release-policy/bundle tests and four container-tool tests.
- Architecture/documentation checks, diff whitespace check and official
  actionlint 1.7.12 workflow/expression lint (shellcheck disabled).

Required remaining acceptance: full nonpublishing native validation at the exact
new workflow commit, then a nonpublishing reuse rehearsal at the same source.
Record their run IDs, accepted artifact/manifest digests and skipped build jobs.
Publication's registry-copy/anonymous/channel/bundle paths retain existing gates;
a new public release is deliberately outside this qualification.

## First native qualification and correction

[Validation at e44841c](https://github.com/AberrantWolf/junk-collector/actions/runs/39786984960)
passed all source gates and the ARM64 build/smoke. AMD64 stopped at the new smoke
identity check, before running its smoke or admitting a reusable artifact. Its
newer Docker/containerd store identifies images by target manifest digest; the
classic store uses the config digest. Treating both IDs as config hashes was an
incorrect tooling assumption. No publication occurred.

The identity gate now checks the actual config bytes from a bounded Docker save
when the inspect ID differs, still requiring the exact OCI config SHA-256. It
accepts both save layouts and rejects altered config bytes; the regression test
uses real config blobs, plus classic/manifest-ID controls. This does not waive
runtime identity or the native smoke. Fresh exact-source CI acceptance is required.
