# Build once, publish validated images

This changes future releases after beta.4. Installed clients and Portainer keep
their existing update process and `preview` channel. It does not republish beta.4.

Release tooling stays in the existing **Container release** workflow. Validation
runs the repository checks, builds each native architecture once, imports the
runtime from its labelled OCI export into Docker, checks its config identity and
runs the actual native proxy/inbox/recovery smokes. It verifies both exports and
stores the merged archive plus `validation.json` as an immutable Actions artifact.
Publication copies that validated archive, preserving all image digests. It does
not rerun compilation, source tests or image builds.

1. Finish source/version changes, push the exact candidate, then run:
   ```sh
   gh workflow run container-release.yml --ref codex/your-release-candidate
   ```
   Empty inputs produce **Validate container candidate**, without registry writes.
   Wait for the entire run to succeed, including both native architectures and
   container verification. Record its run ID. No candidate changes after this
   point: even a documentation commit requires fresh exact-source validation.
2. Optionally rehearse consumption at that same source revision:
   ```sh
   gh workflow run container-release.yml --ref codex/your-release-candidate -f validation_run_id=RUN_ID
   ```
   With no release tag this performs **Rehearse validated container promotion**:
   restore/check the real immutable artifact, reverify both platforms/source
   labels, generate a local bundle and record provenance. It skips checks/builds,
   registry login, image publication, channel promotion and release attachment.
   A rehearsal is never accepted as a validation run.
3. After validation and the normal release approval, create/verify the signed
   `vVERSION` tag at that exact candidate and publish the GitHub release. The
   release event discovers the successful exact-source validation automatically;
   the newest matching validation run must be usable. It restores/reverifies
   its archive before logging in to GHCR. Anonymous version pulls on both
   platforms precede moving-channel promotion and pinned release assets.

An explicit publication retry can select `retry_release_tag=vVERSION`, optionally
with `validation_run_id=RUN_ID`. It must reference an existing published release,
checks out that exact tag and reuses its validated bytes. This is a publishing
operation; do not set the release tag for a rehearsal. The existing concurrency
lock and semantic-version comparison prevent an old release from downgrading
`preview` or stable `latest`. Stable and prerelease channels remain separate.

Artifacts are retained for 30 days. Missing, expired, ambiguous, corrupt or failed
validation is an error, never an implicit rebuild during publication. To recover,
explicitly dispatch empty-input validation at the exact tagged source, wait for
success, then retry publication with that run ID. Rerun **all** validation jobs
rather than mixing artifacts from different run attempts. A failed download leaves
no restored candidate; retries verify remote provenance again. Tags predating
this workflow have no reusable artifact; this change does not retrofit or mutate
them. Do not move an existing released tag to acquire a newer workflow.

Consumption binds the GitHub repository IDs, workflow identity/path, dispatch
kind/title, successful complete run, exact source SHA and current run attempt.
The artifact must belong to that run/repository, remain unexpired and carry its
immutable ZIP SHA-256. Extraction permits only the two expected regular files,
with size bounds; archive bytes, version/source/image, both platform digests and
merged digest must agree with sealed and freshly inspected evidence. A concurrent
validation rerun invalidates recovery until completion. These checks assume
GitHub and the repository's authorized source/workflow maintainers are trusted;
they do not execute downloaded workflow code in the publishing job.

`release-image.json` and the installation bundle retain the accepted manifest,
source and platform digests. Reused records additionally identify the validation
run, attempt, artifact ID/digest and archive SHA-256. `SHA256SUMS` covers the bundle
and image record. Local OCI/rehearsal evidence does not claim anonymous registry
publication, a NAS deployment or a new installed Trash Amp version.
