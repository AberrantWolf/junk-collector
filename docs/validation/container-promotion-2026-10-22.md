# Container build-once promotion — 2026-10-22

State: complete — full native validation and same-source artifact reuse passed. Branch:
`codex/publish-validated-images`, following the completed beta.4 publication.

The user's authorized delivery replaces redundant publication rebuilds with
promotion of already accepted native images. The completed beta.4 signed tag,
registry images/channels, NAS and Tailscale remain unchanged. No new release or
production deployment is authorized for qualification.

Local checks and all eleven CI policy/rootless transport tests passed:

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

## Accepted exact-source validation and reuse

- Source: `daab2ef4349100d2bfe519d035b258bccaf2ab82`.
- [Full successful validation, attempt 2](https://github.com/AberrantWolf/junk-collector/actions/runs/39795019729/attempts/2):
  architecture, Python policy tests, web check/build/assets, Rust fmt/workspace
  tests/Clippy/optimized build, both native OCI build/import/config identity and
  proxy/inbox/recovery smokes, rootless OCI inspect/copy, merged verification and
  reusable artifact/bundle generation all passed.
- [Successful nonpublishing reuse rehearsal](https://github.com/AberrantWolf/junk-collector/actions/runs/39802057613):
  same source, explicit validation run selection, accepted artifact recovery and
  archive SHA-256, both platforms/source labels and merged digest reverified.
  Source checks and native builds did not run. Registry login, image copy,
  anonymous public pulls, channel promotion and release attachment were skipped.
- Automatic exact-source discovery was also checked against the authenticated
  GitHub API: it selects validation run `39795019729`, current attempt 2, and
  excludes the successful rehearsal by its distinct run kind/title.

Accepted immutable artifact: `validated-container-2`, ID `12435082576`.
Its GitHub ZIP digest is
`sha256:a43819c5fd33f9801e3b16963ae57e145fd0203923c78b44b464af4f52b26b9f`.
The 98,416,640-byte OCI archive has SHA-256
`sha256:b5774d0fc1f919ab1689e9c95cce172ac6f971322a2656e736307130a89756e2`.

| Checked image | Digest |
| --- | --- |
| Merged index | `sha256:e4e4f379ee82ca5269628940098cf65c8ee20c73866eb34f803ca93ef98a8bbc` |
| linux/amd64 | `sha256:3fd86e9144bc4d3684a66689df8a9b2ffe3f1fafb3039b20eba956ff9b4de57d` |
| linux/arm64 | `sha256:5cd0147388ad422aa79d1638b98f1714d849c66bfd0adfc983436cea1f610c97` |

Independent downloaded-artifact checks confirmed the full archive SHA/length,
sealed validation versus reused platform/source/digest evidence, artifact/run/
attempt provenance and both bundles' SHA256SUMS/internal image records. The reused
record honestly keeps `anonymous_docker_pulls: false`; its bundle is a local
rehearsal bundle, not a new published installation. Registry publication paths
retain the existing anonymous/version/channel gates; no new public release was
made to test them. The OCI preservation path was additionally exercised by real
rootless Skopeo copies of actual native exports, described below.

The final evidence commit changes documentation only. A future release candidate
including that commit still needs its own exact-source validation, as documented
in the [operator contract](../container-release-pipeline.md). This qualification
does not authorize moving the existing beta.4 tag or image.

### Coherent full-attempt retry

The first attempt at this final source hit an intermittent existing Rust fixture
failure: `process_failure_and_cancellation_never_report_output_success` could not
start freshly written `fake-flac` (`Text file busy`, Linux error 26). Its writer
was closed before execution; no pipeline code or test/gate was bypassed. A full
rerun at the identical source passed every gate and produced only attempt-2
accepted artifacts. The reuse job checked that same current successful attempt.
The earlier failed attempt supplied no accepted image.

### Published image unaffected

After rehearsal, an independent anonymous registry check reconfirmed beta.4 and
`preview` still match the originally published
`sha256:2b9e4a051ec097ee77345aaae582a12916ff5cb99f203b83fc5381d2439d959d`,
with unchanged source/version/platform labels and release bundle pins/checksums.
No NAS, Portainer, VPN/Tailscale or public-media fallback changes were performed.

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
runtime identity or the native smoke. The subsequent full acceptance above
qualifies the corrected identity check.

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
candidate exports. The full validation and reuse rehearsal above subsequently
passed; local reproduction did not make the earlier failed run acceptable.
