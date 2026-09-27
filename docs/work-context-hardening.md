# Work context hardening — locally accepted 2026-09-26

This delivery precedes FX-07. It preserves typed domain identities and existing
services, worker claims, archive recovery, and durable shared-work ownership.

## Contract and scope

- UI operation scopes bind a target and one attempt, abort superseded work, and
  guard success, failure and cleanup callbacks. Separate lanes allow independent
  work. Closing a view cancels observation, not durable server execution.
- Apply the shared lifetime mechanism to collection grant reads/writes, destination
  search, guided import selection/polling, metadata refresh, and authentication.
- Pending authenticated commands retain immutable path/body/key and their original
  account, with a server-checked actor precondition on the actual request. Authentication may issue a new session, but cannot transfer commands
  to another account. Session-targeted commands declare their retry policy.
- Background asset writes carry an explicit item generation and source identity;
  the archive writer validates both immediately before publication. Manual writes
  are explicitly distinct. Existing worker publication keeps claim fencing.
- Consolidate repeated job-generation/independent-owner eligibility checks used by
  retry and cancellation. Preserve direct owners and committed-publication recovery.

## Acceptance

Delayed result/failure/cleanup, A-to-B-to-A navigation, close/reopen, cancelled
reauthentication with surviving other requests, wrong-account authentication,
immutable retry inputs, stale source publication, independent shared ownership,
and restart recovery must be covered through focused and real-browser tests.
Run repository gates and record exact results in validation. Update this record,
architecture, implementation status, FX ledger and handoff together. FX-07 remains
the next UX delivery; external device qualification remains separate.

## Delivered boundaries

| Boundary | Identity and enforcement |
| --- | --- |
| UI read/mutation lane | `OperationScope` captures a scope and attempt. The React hook disposes the old scope at commit/unmount; guarded callbacks cannot affect its replacement. |
| Pending authenticated command | Original account, URL, body, headers and command key are retained. Waiters cancel independently. `X-Collector-Actor` is compared with the authenticated principal before dispatch. Session-bound actions declare review instead of automatic retry. |
| Worker publication | Full `JobClaim`; storage checks recorded scope, attempt, token and current eligibility, including during long snapshot reads and final publication. |
| Derived asset publication | Explicit `ItemEffectContext` binds item, generation and source. The archive writer/publication gate rechecks the source's durable identity and state. Manual effects are explicit variants. |
| Shared-work cancellation/retry | Existing scan and derivative request registries remain authoritative. A common obsolete-job check retains independent owners and committed-publication recovery. |

The ownership audit retained the existing transactional admission/last-owner
cancellation design. Import cancellation removes the import's reference rather
than an independently owned computation. Parent-fenced child admission and the
committed-Capture recovery exception remain intact. The repeated generation policy
used by generic retry, preparation retry and worker cancellation was consolidated;
no competing registry, scheduler, persistence model or generic workflow engine was
added.

New UI flows should use separate operation lanes where work is independent, attach
the lane's signal to requests, and use guarded continuations for intermediate
steps. Durable cancellation always remains an explicit application command.
New background effect paths must choose their explicit source context and enforce
it in the owning storage write boundary; a display label or cached ownership
boolean is never publication authority. These are reusable contracts, not a claim
that FX-07's whole-interface audit has already been performed.

## Acceptance record

All scoped implementation and acceptance checks passed on 2026-09-26; exact
results are in [validation](validation.md). FX-07 is the next delivery. Preserve
these contracts during its whole-interface review.
