# Container build-once promotion — 2026-10-22

State: source implemented; real CI qualification pending. Branch:
`codex/publish-validated-images`, following the completed beta.4 publication.

The user's authorized delivery replaces redundant publication rebuilds with
promotion of already accepted native images. The completed beta.4 signed tag,
registry images/channels, NAS and Tailscale remain unchanged. No new release or
production deployment is authorized for qualification.

Local checks passed:

- Eleven provenance/archive/recovery tests (ten local tests plus a rootless
  skopeo integration test enabled where the tool is installed) in `check_container_validation.py`: full
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

## Rootless OCI reader correction

[Validation at 556a135](https://github.com/AberrantWolf/junk-collector/actions/runs/39790565049)
passed all source checks and **both native build/identity/smoke jobs**. The merged
archive then failed rootless skopeo extraction: omitted directory headers caused
its untar mechanism to create/chown implicit root-owned parents. No reusable
validation artifact or registry write was admitted.

The archive now includes explicit `blobs` and `blobs/sha256` directory headers.
A local nonroot Skopeo 1.22.3 run reproduced the original permission failure and
accepted the corrected layout. The actual native exports from the failed run
were reassembled and copied with `--all --preserve-digests`; the merged index stayed
`sha256:122bc599c4787037096d8ffbee11a02204da588437bc6d2dc27bde2b564c9eae`,
with both runtimes and attestations retained. This used the official Skopeo image
`quay.io/skopeo/stable@sha256:fb3933825a29be3664abbe5634feb68d1686e8a47460fdcfa3a73bd72455e1d7`
as UID/GID 1001, disposable containers, and read-only fixture mounts. No Collector
server was executed or deployed in these local transport checks.

An executable rootless transport regression now checks real OCI inspect/copy and
preserved digests. The workflow runs it after installing skopeo, before accepting
candidate exports. Fresh exact-source full validation and reuse rehearsal remain
required; local reproduction does not make the earlier failed run acceptable.
