# HL-07D release preparation — 2026-10-20

## Delivered source preparation

The existing workflow now plans exact version/source labels, separates `preview`
prereleases from stable `latest`, verifies an amd64+arm64 index and bound attestation
descriptors, checks every platform's version/revision/source labels, and records the
build's exact manifest/platform digests. Release verification copies all image data
anonymously with Skopeo and performs both Docker platform pulls using a temporary
empty credential config. It does not log an operator out or change saved settings.
Only successful verification can precede channel promotion and published bundles.

Channel promotion is serialized and consults **all** GitHub release pages, rejecting
missing/inconsistent current metadata and preventing an older release from moving
its channel backward. The verified source index is copied to its channel and the
resulting digest is checked. Prereleases cannot select `latest`.
[Docker documents carbon-copy index promotion](https://docs.docker.com/reference/cli/docker/buildx/imagetools/create/).

The bundle pins `.env.example` to the verified manifest digest, includes image/source
records and the complete inbox/snapshot helper/runbooks, and rewrites evidence links
to the exact source revision. Published instructions describe pulls rather than
obsolete build/export directions. Manual dispatch instead verifies a local OCI
archive with no push/channel mutation and retains correct local-build instructions.
The workspace/lockfile prepare `0.1.0-alpha.2`; [draft notes](../releases/v0.1.0-alpha.2.md)
classify storage/rollback and the exact qualified starting image honestly.

## Local validation

- Six policy/bundle regression tests pass: stable/prerelease status mismatch,
  numeric prerelease ordering/invalid tags, a newer version on a later release page,
  missing/duplicate/foreign platforms, unrelated attestation sources, all three
  immutable source labels and actual platform mismatch, unverified public bundle
  refusal, pinned/self-contained published instructions and honest manual bundles.
- A real Buildx multi-platform **verifier fixture** exported an OCI archive containing
  amd64/arm64 configs, payload layer and provenance descriptors. The actual verifier
  checked the index digest, copied all four manifests/layers with Skopeo, checked
  both configs and generated the nonpublishing bundle. The fixture is explicitly
  labelled as a test layer; it is not a substitute Collector image or public release.
- Actionlint 1.7.12 checked both workflows successfully; its official release archive
  checksum was verified before running it from a temporary directory.
- Repository architecture/link checks, Rust formatting, workspace tests, all-target
  warning-denied Clippy, web checks/build and rebuilt embedded server passed for
  alpha.2. The unchanged actual user workflows retain HL-05A/B and HL-07A/B/C's
  full smoke/real-browser/NAS evidence rather than repeating unrelated device work.

Self-review corrected global Skopeo option ordering, a dry-run bundle referring to
an unavailable runner-only image, stale source-only publication text, and digest
inspection through the documented JSON envelope. Downloads bind to the checked
version digest and preserve/recheck it, fencing a mutable-tag change during verification. Published-bundle construction
refuses an unverified record before writing output. No registry credentials are
placed in image records or bundles.

## Still open — not publication acceptance

No source/tag push, release, image publication, moving-channel update or live service
redeployment occurred. The new workflow has not run on GitHub against alpha.2.
Anonymous public Collector pulls, both-platform registry manifests, channel promotion
and real Portainer pull/redeploy still need execution. First publication must also
ensure package visibility permits anonymous access; GitHub supports anonymous reads
for public container packages, and the workflow fails if those reads fail.
[GitHub container-registry permissions](https://docs.github.com/en/packages/working-with-a-github-packages-registry/working-with-the-container-registry).

HL-07D stays **open**. Permanent backup destination/adoption and remaining relevant
LAN/public-ingress checks remain in the existing queue. This commits complete release
preparation, not a successful-looking claim that missing publication was implemented.
