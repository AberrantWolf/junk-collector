# Architectural proposals requiring further decision

Status: Proposed, 2026-11-03. These are not accepted ADRs, requirements or work
assignments. Source assessed: `eaff933`. Each row distinguishes an observed mechanism
from the policy that might warrant an ADR. Current operational requirements remain
in force while these questions are considered.

| Choice to consider | Present evidence and uncertainty | Decision needed before promotion |
| --- | --- | --- |
| One shared guarded SQLite connection/mutex | [Store](../../crates/collector-store/src/lib.rs) uses this topology. Contention fixes protect behavior but do not establish permanence of that topology. | Identify which consistency guarantees require serialization; compare read-pool alternatives against publication/recovery and authorization. |
| Inbox-to-private-to-archive staging copies | Current intake retains both staging steps; [remaining work](../remaining-work.md) explicitly allows investigating the extra copy. | Establish whether a durable intermediate private copy is required or archive-local staging can preserve cancellation, retry and cross-filesystem recovery. |
| Exact concurrency, retry, page and comparison budgets | [Dispatch](../../crates/collector-app/src/job_dispatch.rs) and [matching](../../crates/collector-app/src/digital_pipeline.rs) use deliberate values. Their permanence is not established. | Separate stable boundedness/fairness/explicit-expansion policy from tunable numbers; record measurement and change triggers. |
| Process-local rate limiting | [Security detail](security.md#rate-and-resource-controls) acknowledges restart reset. Awareness is not universal security-risk acceptance. | Define the supported threat model and when persistent or distributed limits become necessary. |
| Current physical TOML/JSON layout and schema encodings | [Foundation detail](foundation-contracts.md) and storage code establish current formats; portability does not automatically freeze every layout choice. | Identify actual external consumers and object-identity/recovery commitments before treating an encoding as a permanent contract. |
| Fingerprint prefix length, cache lifetimes and search strategy | [Matching pipeline](../../crates/collector-app/src/digital_pipeline.rs) and [current source contract](../architecture.md#matching-metadata-review-and-contention) contain intentional optimizations. Stable claim semantics do not freeze the optimization. | Define disclosure, quality, latency and resource tradeoffs independently of current heuristics. |
| Particular browser transport and proxy technologies | [Private playback](../automatic-private-playback.md) and [media isolation](../../deploy/container/media.md) establish current mechanisms. Trust/isolation ADRs do not mandate every library or proxy product forever. | Compare platform constraints, alternatives and replacement triggers while preserving the accepted trust and isolation rules. |
| Stopped-process backup and conversion | [Operations](../server-operations.md) currently require the server stopped. Consistent recovery is strongly intentional; rejecting every future online snapshot mechanism is not established. | Decide whether quiescence or stopping the process itself is the lasting support boundary. Current commands still require stopped operation. |
| React operation scopes and media-ref cleanup | [Operation scopes](../../web/src/operation-scope.ts) and [player cleanup](../../web/src/music-player.tsx) enforce intentional cancellation/lifetime guarantees. Their particular framework form is local. | Decide whether a frontend ADR adds value beyond the lifecycle contract and real regression tests. |
| Current client targets and optional feature deferrals | [Product scope](home-lab-scope.md) explicitly prioritizes consumers; many deferrals are planning decisions rather than architectural boundaries. | Identify a lasting technical constraint before promoting a target/deferral to an ADR; keep scheduling in the single open-work queue. |

Promotion requires explicit acceptance of a normative rule, its scope, rationale,
consequences and reconsideration triggers, with code/requirements/evidence checked.
A proposal must not be treated as accepted solely because a current test preserves
the described implementation. See the [accepted ADR index](README.md).
