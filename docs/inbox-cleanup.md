# Removing imported inbox files

Imports keep their inbox source files by default. Removing them is an explicit,
per-package action.

Open **Activity**, select the completed inbox import and expand its details. Under
**Inbox source files**, make sure nothing is still copying or editing the source
package, then choose **Remove imported inbox files**. You need curation access and
must be the user who requested the import.

The import must have committed its originals and finished any requested metadata or
preparation steps; failures in optional steps do not block cleanup, since those can
be retried from the archive. Cancelled imports and unfinished imports are not eligible.
Before removing anything, the server checks every archived original's length and
checksum and every source file's identity, length, checksum and modification time.

Only one cleanup runs at a time, and new inbox imports pause while it runs. Finish or
discard other unfinished inbox imports first. Browser uploads, browsing and playback
are unaffected.

## Writable inbox

The container inbox mount is read-only by default. To allow cleanup, set
`COLLECTOR_INBOX_READ_ONLY=false` in `.env`, give the configured supplemental group
write and traverse permission on the inbox and the imported files' parent
directories, and recreate the service. Do not recursively change ownership of a real
music share. A cleanup attempt on a read-only mount fails without removing anything;
fix the permissions and retry. Write access never enables automatic deletion.

## Failures and recovery

The server first moves the package into a private
`.collector-cleanup-<import-id>/package` directory next to it, then deletes only the
unchanged imported files. Unexpected extra files stop the cleanup before the move.
Other non-target files are kept and only empty recorded directories are removed; a
small `source.json` receipt may remain.

Cancel stops further removals but cannot undo completed ones; the archived originals
remain. Retrying a partial attempt reuses the same recovery directory, and a crash is
reconciled from the recorded per-file intent. Generic job retry is disabled for
cleanup so it cannot skip the checks.

If a filesystem or permission problem leaves files in the recovery directory, fix
only that directory and retry. Keep changed files for review rather than forcing
the cleanup. The checks cannot stop a program that already has a file open.

Inbox and recovery directories are outside the archive/state backup. A pending
cleanup restored onto another filesystem fails safely if its source files are missing.
