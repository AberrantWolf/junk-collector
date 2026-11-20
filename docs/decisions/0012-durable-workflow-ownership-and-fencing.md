# 0012: Durable workflow ownership and fencing

Status: proposed, not yet reviewed by the owner.

## Context

Panels close, workers lose claims, and several requests can share one expensive
child job. UI lifetime can't govern server work.

## Decision

- Admitted workflows have a durable request identity and owners independent of any
  UI. Closing a panel doesn't cancel server work.
- Shared work records each requester. Cancelling one owner doesn't cancel work
  another still needs; cancelling the last owner stops new child work.
- Converging requests and retries keep the first job's immutable inputs and
  creation time.
- Claims, generations, source currency, permissions and cancellation are checked
  when work executes and when it publishes. A stale worker can't publish.
- Child admission and parent ownership commit under the same claim. Upstream
  results commit before downstream work advances.
- Cancellation before commit cleans up unpublished effects (see
  [0004](0004-single-writer-publication-and-recovery.md)); late cancellation never
  rolls back a committed revision.
- Recovery periodically rescans unfinished workflows, including after restart.
  Capacity isn't reused until the cancelled work has actually stopped.

See: [acquisitions](../../crates/collector-store/src/acquisitions.rs),
[job claims](../../crates/collector-store/src/jobs.rs).
