# Remaining work

Reviewed 2026-11-10. This is the only open-work queue.
[Status](implementation-status.md) owns implemented capabilities; [dated evidence](validation.md)
owns outcomes. The [earlier queue in Git](https://github.com/AberrantWolf/junk-collector/blob/7f4a56c8d6b38724a371549d09b978bbcb8e5cb1/docs/remaining-work.md) is historical
and must not supply assignments. Existing citations into it explain earlier scope.
No production deployment is authorized by this document.

## Release and source state

[Beta.24 qualification and publication](validation/container-beta24-publication-2026-11-10.md)
are complete. The split-host deployment passed acceptance after the operator
sealed a paired recovery point using the
[installed-stack procedure](../deploy/container/beta-update.md).

## Concrete follow-ups

| Work | Remaining outcome and acceptance |
| --- | --- |
| Active-work shutdown | Quiesce blocking audio work within the configured stop grace period without cancelling durable requests. Verify shutdown under load and restart with retained Job identities. [Observed failure](validation/container-beta11-publication-2026-10-28.md) |
| Release receipt recovery | Reproduce the source-receipt visibility race before changing recovery. Keep exact-source/run-attempt/artifact requirements and add a focused regression. [Observation](validation/container-beta14-publication-2026-10-30.md) |
| Native/device reliability (HL-04/08) | Qualify remaining system-trusted enrollment, rapid account/server switching, physical output-device loss, UI recovery, Internet-disconnected LAN use and download failure/restart on supported clients. Assess pinning/eviction and budget exhaustion against the actual client; preserve standalone playback and retained offline files. Reuse [accepted transport/offline cases](validation/native-failure-recovery-2026-10-21.md); do not infer physical output or all-device acceptance. |
| Advertised format evidence (HL-06) | Expand independent owned/legal references only for advertised claims, especially live whole-disc AccurateRip coverage and unsupported/partial boundaries. Existing [audio](audio-preparation.md) and [game](game-identification.md) evidence remains bounded. Missing reference material leaves qualification open. |
| Direct/private media qualification (HL-05/12) | Qualify remaining browser permission/UDP denial, offline LAN bootstrap limits, long-track grant renewal, network changes, certificate renewal and restart/rollback on the intended supported clients. Automatic private selection and direct HTTPS already exist; relay/Iroh work is not their prerequisite. |
| Supported deployment acceptance (HL-05) | Qualify the intended host/storage layout with restart, interrupted upload/publication/encoding, missing tools, permissions and consistent restore. Supported public use additionally needs one trusted-HTTPS sign-in/playback/seek and anonymous-API-denial check, reusing applicable existing evidence. No public gate is imposed on LAN-only use. |
| HTTPS frontend isolation (HL-05/12) | Review firewall/DMZ and egress restrictions for the networked TLS frontend. Media-container network isolation does not isolate the frontend; configured resource limits must be checked for actual host enforcement. Retain named CPU/process-limit exceptions. |
| Intake I/O (HL-09) | Assess avoiding the extra inbox-to-private-to-archive copy, preserving source currency, cancellation, cross-filesystem publication and restart/retry. Dispatch fairness is already implemented. |
| Representative performance (HL-09) | Measure actual startup, browse, search, imports and preparation on representative supported storage/workloads. Repair concrete unbounded/slow work; do not restart completed projection redesigns or impose speculative scale gates. |

## Conditional work; select a bounded delivery before implementation

- **HL-01 retention:** generic revision/candidate/occurrence/Asset pruning, receipt
  expiration and upload-spool reclamation require demonstrated growth and complete
  reference/replay rules, dry-run effects and tested recovery. Equivalent-response
  reuse and stopped-server duplicate conversion are delivered. Preserve originals,
  meaningful evidence, accepted sources, manual facts and unknown data.
- **HL-09 convenience:** game metadata/artwork providers require a real consumer,
  terms/access review and manual authority. Preparation-history/metadata pagination,
  upload resume and inbox traversal changes require a measured limitation. PDF/text
  attachments, inbox cleanup and game-disc audio are already delivered.
- **HL-10 transfer:** full portable export/import is delivered. Deltas require base
  identity and dependency closure. Legacy migration requires actual legacy data,
  dry-run mapping and restartable conversion; raw-file import is not migration.
- **HL-11 activity:** define event identity/conflicts and reconnect deduplication
  before offline activity sync. Add authorized event cursors with polling recovery
  only for demonstrated demand. A separate inventory desktop app is optional.
- **HL-12 transport:** general relay/failover, physical-path accounting and
  authenticated trust rotation remain deferred. Ordinary renewal/recovery still
  needs documentation and relevant acceptance, including disable/rollback and any newly claimed IPv6 support; private reachability is not proof
  of a physical LAN or zero-WAN cost. No vendor VPN integration is assigned.
- **HL-13 expansion:** synchronized listening needs session/control/join authority
  and drift policy. Broader originals (AAC/ALAC/Opus), album-image splitting,
  storefronts, richer DSP, smart/personal lists, broader document rendering/formats, mobile ports (including
  SwiftUI/UniFFI), PriceCharting after API/permission research, video, emulators,
  ripping hardware, replication, runtime plugins and cross-collection deduplication require separate complete designs.
  Digital album fingerprint matching and explicit full mapping review are delivered.
- **HL-14 hardening:** persistent/distributed limiting, exhaustive platform/load
  matrices and stronger power-loss qualification are conditional. The retained large-scale target is
  100,000 items/one million file-evidence records/five users on a recorded four-core,
  16-GB local-SSD host, warm browse/search below 500 ms p95; it is not a normal
  home-server release gate. Concrete exposed
  vulnerabilities, credential failures and preservation/recovery defects take priority.

## Assignment and completion rules

Choose one user-facing outcome with its dependencies and real failure/recovery case.
Do not restart HL-02 purchases, HL-03 backup/restore, completed interface milestones,
packaging, automatic private playback or digital matching because an old plan calls
them missing. Phono-specific acceptance is retired, not passed. AccurateRip submission
uses the [external-tool decision](accuraterip-contribution-investigation.md); no
Collector submission or blocked-provider scraping is assigned.

For each selected delivery, record owner, scope, dependencies and acceptance here.
On completion, remove its open entry, update the capability status and owning current
contract, and link one dated result. Keep failures and unrun checks visible in that
result rather than copying delivery prose into several documents.

Architectural boundaries are recorded in the [accepted ADRs](decisions/README.md).
The [lower-confidence proposals](decisions/proposals.md) retain questions for future
decision; they create no new assignment or prerequisite for this queue.
