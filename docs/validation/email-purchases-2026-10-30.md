# Email purchase intake branch delivery — 2026-10-30

Candidate branch: `codex/email-purchases`, based on `0c68271`.
No production deployment, published image, mailbox login or user credentials were
used. The feature and setup are described in [email purchase intake](../email-purchases.md)
and the [Bridge runbook](../../deploy/proton-bridge/README.md).

## Implemented

- Optional operator-provisioned, user-owned Proton Bridge mail provider, with an
  extensible transport port separate from retailer receipt parsing.
- Read-only, certificate-pinned local IMAP; bounded subprocess/time/output limits;
  resumable folder UID/UIDVALIDITY history and atomic cursor/candidate commits.
- Synthetic Japanese/plaintext/HTML parser coverage for Amazon Japan, Joshin,
  Square Enix e-STORE, Nintendo eShop and Steam. Unknown templates require review.
- Private paginated pending/accepted/dismissed inbox; explicit link/create,
  reviewed correction, cancellation/refund order updates and intentional duplicates.
- Portable accepted Item order facts, ordinary-edit retention, Viewer redaction,
  idempotent review, writer-fenced races and archive-only recovery.
- Self-managed official Bridge container build, persistent pass/GPG setup,
  host-loopback Portainer override and local hidden-input provisioning.

## Validation

Completed targeted checks:

- Eight offline Python tests cover all five retailers, Japanese MIME encoding,
  multi-item HTML, source deduplication, subject-only status, spoofed sender-domain
  rejection, privacy/size limits, read-only IMAP commands, UIDVALIDITY reset and
  certificate failure before LOGIN, plus an independent RFC 3501 Japanese-folder
  encoding vector.
- Three storage/application tests cover private access, repeat imports, receipt
  replay, cancellation updates, retained manual titles, no invented physical/file
  ownership, paused-sync fencing, restart/archive-only recovery, pre/post-commit
  interruption recovery, and filtering before pagination past 60 older reviews.
- Real browser journey passed against disposable Collector state: accept a new
  game, link a cancellation to its order, intentional duplicate, dismiss, reload,
  mobile layout and public page/asset/API listener denial. Desktop/mobile captures
  were inspected separately.
- Official DEB digest matched; its embedded origin signature was verified locally
  with GPG against the pinned Proton public key. Package contents include the
  launcher and official headless binary. CLI/loopback/pass behavior was inspected
  in the pinned upstream release source.

Final checks passed:

- `cargo test --workspace --locked`: **560 passed, 0 failed, 5 ignored** by the
  repository's normal policy (owned-media, subprocess and manual scale fixtures).
  The three focused email tests were rerun after the final ordinary-entry edit
  preservation cleanup and passed again.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` and
  `cargo fmt --all -- --check`.
- Generated OpenAPI/TypeScript contract comparison, web type checking and build;
  actual request limits and the query-only inbox view schema are registered.
- All **30 browser journeys** and three queue/context/labels model checks passed
  across isolated runs. The final rebuilt assets passed email review (including
  retaining an intentional-duplicate choice across order updates) and smoke again.
  One earlier digital-purchase run passed its assertions but timed out closing
  Chrome; its isolated rerun completed successfully. Two fixture file-URL paths
  were corrected to support checkout directories containing spaces.
- Architecture/document links, test selector/evidence checks, container tooling,
  release policy and container-validation checks. The existing container-validation
  suite retains one intentional policy skip; no container runtime acceptance is
  claimed from these source/tooling checks.
- Compose merge/render passed without a Docker daemon. Both services use the
  documented host namespace, Bridge publishes no ports, credential mounts are
  read-only, and the sidecar retains non-root/capability/read-only-root settings.
- The Bridge entrypoint passed shell syntax checks; local provisioning passed
  Python syntax checking. The entrypoint's actual pass/keychain runtime remains
  part of container qualification below.

The branch's builds used reduced debug information after the Mac initially ran
out of space. Only this checkout's generated build directory was removed. A
concurrent rebuild invalidated an earlier doc-test artifact; the successful final
workspace run was serialized with other Rust builds. No failed attempt is counted
as qualification, and no debug executable was deployed to the NAS.

## Limits

No Docker engine is available locally; actual Bridge container build/start,
headless account login, reboot persistence, live mailbox unread-state/history and
NAS resource use still require isolated local qualification in the runbook.
Personal receipt templates have not been uploaded or calibrated. Parsers are
conservative template adapters, not universal retailer/order APIs. Gmail, dynamic
plugin installation, automatic acceptance, quantities, reliable partial refunds
and automatic cross-language identity resolution are not implemented.
No release/publication, production migration or credentials in chat are required
for this branch delivery.
