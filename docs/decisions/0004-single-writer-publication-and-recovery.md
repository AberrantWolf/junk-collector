# 0004: Single-writer publication and recovery

Status: proposed, not yet reviewed by the owner.

## Context

Publishing to the archive touches both files and database projections. After an
interruption, unfinished work has to be told apart from committed history before
anything new is written.

## Decision

- One archive writer owns publication. Running on several hosts requires fencing
  other writers operationally; local locks or matching archive UUIDs are not leader
  election.
- Publication journals its phases (prepared, staged, published, committed,
  projected, completed) and binds artifacts to expected identities and revisions.
  Renaming the immutable revision directory is the commit point. Evidence bodies
  are stored once as append-only JSON; revisions reference them.
- Startup reconciles interrupted publications before accepting writes. Reindex
  repairs projections but does not replace transaction recovery.
- Recovery keeps committed revisions and unknown `.staging` content, and fails
  closed on inconsistent artifacts instead of silently reverting.
- If an attempt is withdrawn before commit (cancellation, lost claim, contention),
  the publisher cleans up its own uncommitted journal and staging while still
  holding the writer. It never removes committed revisions or unknown staging. A
  lookup error is propagated; only an explicit "not found" allows a new publication.
- Startup requires the enrolled archive/private-state pair on the expected mounts.
  Missing or substituted storage is never initialized as an empty installation.
- Archive recovery metadata contains no credentials or grants; private-state
  phases live in private state.

## Consequences

No concurrent archive writers. Filesystem durability remains a deployment concern;
there is no universal power-loss guarantee.

See: [publication](../../crates/collector-store/src/publication.rs),
[storage guard](../../crates/collector-store/src/storage_guard.rs).
