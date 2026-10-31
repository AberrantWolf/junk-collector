# Inbox connector recovery and review corrections — 2026-10-31

Source delivery in the working tree based on `56b6618`; includes the earlier
[folder editing](mail-folder-editing-2026-10-31.md) change. No deployment, production
mail access, or release-image qualification was performed.

## Delivered behavior

1. Candidate receipts with malformed dates, unsupported decoding/layouts, excessive
   MIME parts or oversize bodies become explicitly unparsed suggestions. The MIME
   factory refuses the 101st message node during parsing. Header-only fallback uses
   an exact raw-byte identity for rejected trees; supported no-Message-ID messages
   retain their earlier canonical identities. Unparsed receipts have typed issues
   and private locators, require manually reviewed details/status before acceptance,
   and can be dismissed/restored. Transport errors and incomplete literal bytes
   still fail the batch without advancing its cursor. Later valid receipts can
   progress past malformed content. Bodies remain bounded to 2 MiB and subprocess
   requests to a 90-second deadline.
2. **Revalidate Bridge connection** accepts the same local username, current IMAP
   password and explicitly confirmed certificate fingerprint, with expected profile
   revision. An authentication-only pinned probe precedes active-file replacement;
   this does not claim folder access. Account/profile identity, cursors, folders,
   suggestions and enablement survive. Operator profiles get ID-specific managed
   overrides; multiple profiles for one owner remain repairable and original files
   stay unchanged. Prepared/activated, input-bound repair receipts contain safe
   responses/digests, not old credential copies. A durable activation marker proves
   which request actually applied; genuinely historical successes can replay without
   rolling back current credentials. The active registry advances immediately after
   rename, and provider configuration guards serialize both batch and failure
   publication with repair. New scans record the frozen configuration revision;
   superseded errors show waiting status until a new scan reports.
3. Reviewing an older receipt against the same retailer/reference attaches its source
   key without replacing newer accepted fields, date/status or duplicate intent.
   A newer receipt updates that order normally. Receipt source records are immutable;
   manual acceptance changes only the accepted purchase facts.
4. **Restore suggestion** reverses dismissal with a private review revision check.
   **Reopen purchase review** previews and confirms the removal of the selected
   current purchase row, returning all its linked receipts to review. Prior accepted
   details remain in archive revisions; inventory, metadata and files remain intact.
   Users can then correct fields or select another Item through ordinary review.
   This is two explicit user decisions, not a frontend-composed publication pipeline.
   Reopening uses the existing Item archive/command journal and expected Item revision.
   Private generations reserve new review ownership before publication; interruption
   can require refresh, but cannot make old review forms valid or report an
   uncommitted archive change as successful.
5. Each configured folder reports not checked/history backlog/caught up at its last
   timestamp. Backlog counts UID positions, not messages. Typed authentication,
   certificate, folder, transport, invalid-response and adapter failures preserve
   the saved cursor. Owner-private locators retain bounded folder/UID/UIDVALIDITY,
   Message-ID, subject and email date. They never enter portable Item records or
   another owner's inbox. Matching rescans refresh coordinates without replacing
   original receipt content. **Rescan configured folders** resets cursor/progress,
   retains suggestions and enabled/paused state, and fences prior work.

## Clean-context review

An independent agent with no inherited conversation reviewed the whole connector
and corrected implementation. It found three additional actionable issues:

- Deep no-Message-ID MIME could recursively serialize during quarantine. The bounded
  parser factory/header-only fallback fixes this; the reviewer independently reran
  a 1,100-level fixture and received a reviewable `too_many_parts` suggestion.
- IMAP response status shadowed the requested failure category. The corrected helper
  preserves `folder` and its locator; the independent refusal reproduction passes.
- A prepared-but-never-activated repair could replay another request's success at
  the same generation. Activation markers, prior-active settlement and explicit
  generation ownership now reject it. Post-rename errors also keep the runtime
  configuration fence aligned with the visible file.

The reviewer inspected the corrected paths and failure tests, independently reran
both parser/refusal reproductions, and reported **no remaining actionable findings**.
The final audit also added the submitted suggestion revision to dismissal (not just
acceptance/restore), preventing stale tabs from redismissing a restored/reopened
receipt. The reviewer verified frontend/API/receipt/transactional guard alignment
and its regression, with no remaining actionable findings.
No reviewer edits or deployment were performed.

## Validation

Full selection is `python3 scripts/check_changes.py --level full --classification
infra --journey email-purchases`; the complete staged/unstaged/untracked diff was
previewed. Workspace-wide source steps passed (571 Rust tests, five existing ignores).
The final focused checks also pass: nine Store email tests, six provider setup tests
(including two post-review failure cases added after the full run began), 21 offline
mail tests, TypeScript/build/generated contracts, final strict workspace Clippy,
format/architecture/documentation links, and expanded real email browser acceptance.
Desktop and 390px mobile rendering were inspected.

The full selection completed with every selected step passed, including all 30
browser journeys outside macOS seatbelt. Final focused browser acceptance was rerun
after the dismissal contract change and passed. The provider/Store regressions,
strict workspace Clippy, generated-contract check, TypeScript/build and formatting
were also rerun for the final affected source. The operator helper compiles as a
Python script and its `--help` output was inspected; no operator command was run.
Unchanged workspace evidence is reused with the focused
post-review checks, rather than treating earlier source snapshots as exact-byte
release evidence. This dirty working-tree run is not a sealed release receipt.

Existing ignores cover owned-disc/manual scale/performance cases and an independently
invoked subprocess fixture; the container-validation script also retains its
rootless-skopeo prerequisite skip. Personal receipt calibration, physical mobile/
cellular acceptance and production/release-image tests remain unrun. All fixtures
are synthetic, offline or isolated loopback services.

Bridge's initial sync completeness remains separate from receipt scanning. Finish
initial Bridge sync and explicitly rescan selected folders for a history pass.
Unsupported retailers/templates may still be missed, and new entries remain Games
only; these intentional limits are unchanged. Raw email is not retained, so older
unparsed suggestions remain reviewable/dismissible if later parser repairs produce
new suggestions. Locators are current private diagnostics, not permanent raw-mail
provenance or proof of sender authenticity.
