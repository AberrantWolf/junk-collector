# Optional inbox source cleanup

Implemented in source for HL-09, 2026-10-21. Imports continue to keep their source
files by default. This feature is not in the published alpha.2 image; the next
Docker release packages it. See the [acceptance record](validation/inbox-cleanup-2026-10-21.md).

Open **Work & history**, select your completed inbox import, and expand its details.
Under **Inbox source files**, confirm that copying and editing the source package
have finished, then choose **Remove imported inbox files**. The action is separate
from import acceptance, never selected automatically, and applies to this package.
It requires your current curation access and the original import requester.

Preservation must have committed and requested metadata/preparation must have
finished. Optional processing failures do not block cleanup; those steps can retry
from the archive. Cancelled imports and unfinished preservation are ineligible.
The server checks every archive original's actual length/checksum and every source
file's identity, length/checksum and modification time before removing anything.
This is copy integrity, not catalog verification. No payload hashing is added to
browsing, inbox listing or status polling.

Finish/discard other unfinished inbox imports first. New inbox admission is paused
while a cleanup Job is active, including across differently named or nested inbox
roots. Browser uploads and normal browsing/listening remain available. This
conservative policy avoids racing another import's source ownership. A failed or
cancelled cleanup releases the admission pause; its retained recovery inputs remain
bound to that request. Only one cleanup is admitted at a time.

## Writable inbox configuration

The shipped inbox override stays read-only by default. To offer deletion, deliberately
set `COLLECTOR_INBOX_READ_ONLY=false` in `.env`, grant the configured supplemental
group write/traverse permission on the inbox and imported-file parent directories,
and recreate the service with the normal Compose/Portainer workflow. Both server
and CLI receive the same bind choice. A mount flag does not grant filesystem rights.
Do not recursively change ownership of a real music share. Existing custom Portainer
binds retain their configured mode; installing an image does not rewrite them.

The first cleanup attempt on a read-only mount fails visibly with no files removed.
Correct the fixture/installation permissions or mount choice, then explicitly retry.
Write access alone never enables automatic deletion.

## Failures, cancellation and recovery

The server freezes source filesystem identities in a private request, then moves the
package atomically beneath a private `.collector-cleanup-<import-id>/package` recovery
directory beside its original name. It deletes only unchanged imported files.
Meaningful extra files cause pre-move refusal. Housekeeping and other files that are
not deletion targets are retained; only empty recorded directories are removed.
A small `source.json` receipt directory can remain even when all payloads are gone.
Recovery directories are excluded from direct inbox package selection; this does
not filter similarly named unknown directories within preserved packages.

Progress reports confirmed removals, failures and a relative recovery location.
Cancel stops further removals at the next checkpoint; it cannot undo files already
removed. The preserved archive originals remain available. Retry after a partial
attempt uses the same recovery directory and identities, never a replacement package
created at the original name. A crash after unlink is reconciled from the durable
per-file deletion intent. Unfinished running work resumes through normal Job
recovery; cancelled/failed work requires an explicit new request. Generic Job retry
is disabled for cleanup so it cannot bypass the source checks.

If a filesystem/permission problem leaves files in the recovery area, repair only
that named directory before retrying. It is private to the server account; a host
administrator/container root may be needed to inspect or move it. Retain changed
files for review instead of forcing cleanup or deleting the recovery directory.
Other programs must finish copying/editing before confirmation: these checks cannot
revoke an external writer's already-open handle. Unrecognized files and source
changes are never a reason to skip validation or pretend cleanup succeeded.

Inbox/recovery directories are external inputs, not part of the CLI's archive/state
backup. A pending cleanup restored on another filesystem fails closed when its
original source identities/recovery directory are unavailable. Keep those external
inputs when recovering unfinished work. Completed archive originals remain backed
up normally; this feature does not prune them or implement archive compaction.

Private schema 21 adds cleanup requests/progress without resetting previous data.
For downgrade, restore a complete consistent pre-update recovery point with its old
image; alpha.2 cannot open the new private schema.
