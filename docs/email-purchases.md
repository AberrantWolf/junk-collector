# Email purchase suggestions

Branch delivery, 2026-10-30. Optional email intake proposes collection changes;
it does not automatically alter inventory or infer possession from a receipt.
Installed on Fi on 2026-10-31; see [local account setup](../deploy/proton-bridge/README.md#current-fi-installation--2026-10-31)
and [deployment evidence](validation/fi-email-deployment-2026-10-31.md).

Folder editing and inbox recovery are source deliveries dated 2026-10-31 and have not been deployed;
see [folder editing](validation/mail-folder-editing-2026-10-31.md) and
[recovery validation](validation/inbox-recovery-2026-10-31.md).

## Boundaries

`MailProviderPort` is the installed transport boundary. Its first adapter uses
local official Proton Mail Bridge; Gmail is not implemented. The connector is
embedded in the provider adapter and runs with Python 3's standard library.
Retailer interpretation is independent of IMAP inside `mail/connector.py` and can
be reused by a future Gmail adapter. Providers are compiled/configured adapters;
there is no dynamic executable plugin installation interface.

The administrator provisions a private profile for a specific Collector user.
Administrators can connect their own Bridge from the visible **Connect your
email** card in Purchase inbox. It accepts Bridge's distinct local IMAP
credentials, validates the server-provided certificate pin and read-only folder
access, then saves the profile without enabling receipt scanning. Proton account
login/2FA remains in the private Bridge terminal. Legacy operator-provisioned
profiles remain supported; read APIs never return credentials.
That user enables it for one explicitly selected collection with curate access.
Other users cannot enumerate the profile, credentials or suggestions. The UI
supports enable/pause and **Edit scanned folders**; changing an existing profile's
destination is refused.
Use a new profile ID to import to another collection.

**Edit scanned folders** accepts 1–16 exact IMAP folder names, one per line, then
**Save folders**. Include `INBOX` to keep scanning inbox mail; archived receipts
require their folders or the exact Bridge All Mail folder name. Saving retains
the enabled/paused state, suggestions and saved folder scan positions. New folders
start at the beginning; removing a folder stops its future reads without deleting
prior suggestions. Changes apply to subsequent batches without restarting Collector
or entering credentials again. Folder access is checked by the next enabled scan;
invalid/inaccessible folders produce a visible scan problem. Cancel and reopen the
editor if another update causes a revision conflict. Folder choices persist in
private connection state; provisioned defaults and credential files remain intact.

No public mail/SMTP tool is exposed. Credential-bearing profiles live outside the
portable archive and must be mode 0600/0400. Read APIs return owner-scoped summaries,
never credentials. Profile files contain reversible local Bridge credentials and
need protected local storage and encrypted backups.

Pending/dismissed suggestions, connections and cursors live in private SQLite.
Accepted orders are portable, role-redacted Item records, retained by ordinary
Item edits, reindex and archive-only recovery. Readers with Viewer access receive
no purchase details. Full local backups include private SQLite; back up the
external credential file and Bridge home separately and privately.

## Review

Open **Purchase inbox**, select a collection, find **Connect your email**
and enable receipt scanning for its configured account. Every receipt match needs
explicit review.
Correct the title, platform, edition, format, order reference and status; then
link an existing entry or create an unidentified Games entry. Creating entries
for other collection modules is not implemented; existing entries can be linked.
No specimen, carrier, game dump, catalog identity or ownership assertion is
fabricated. Cancellation/refund suggestions can explicitly update a matching
existing order; source dates prevent an older message replacing a later status.
Manual titles and metadata remain unchanged when linking.

Matching normalizes Unicode width, case and punctuation. A title from a reviewed
order linked to an Item also acts as an alias for later suggestions, allowing
Japanese/English receipt titles to converge through manual review. Platform and
edition remain visible; overlapping matches are warnings, not proof of a mistaken
purchase. A reviewer can mark a duplicate intentional. No external title/AI lookup
receives mail data. Matching is bounded to the first 5,000 entries and 20 displayed
matches per suggestion; use explicit library search when that is insufficient.

Every accepted source key is retained with the order. Inbox acceptance is derived
from those archive-owned keys. The same Item publication commits Collector's
existing command receipt, so interruption cannot record a successful review
without its order. Concurrent/replayed decisions use idempotency receipts,
expected Item revisions and the archive writer lock; dismissed/accepted receipts
require an explicit restore/reopen before they can be reviewed again. Disabling a connection fences in-flight
batch commits without discarding its history cursor.

## Import and parser coverage

Polling runs every minute after the prior batch completes, one account at a time.
Each account fetch has a 90-second deadline. History starts at UID 1 in configured
folders, in windows of 100 UIDs; every successful batch commits candidates and
cursor together. Repeated polls drain history. UIDVALIDITY changes rescan that
folder; profile-scoped message/line hashes suppress duplicate messages across
folders and rescans. IMAP uses EXAMINE and BODY.PEEK; it does not mark mail read,
move it, delete it or send mail. This is an application restriction: Bridge's
credential itself can read/write the account and must remain private.

Only matching retailer sender domains and receipt-like subjects trigger body
fetches. Headers are routing hints, not proof of sender authenticity. Bodies,
attachments and raw messages are not retained or sent to browsers. The parser
keeps selected product text, order reference, status and source date. Bounded
owner-private locators retain folder/UID/UIDVALIDITY, Message-ID, subject and the
email date so an original can be found; they stay outside the portable archive and
are never exposed to other users. No full header block is retained. Locator text
can itself contain private sender-supplied details; inspect it only in the private
review UI. Matching rescans refresh coordinates without rewriting receipt facts. No external HTML
resource is fetched. Other email clients can still modify mailbox flags; Bridge
sync itself is separate from the extractor's read-only command set.

Supported parsers: Amazon Japan, Joshin, Square Enix e-STORE, Nintendo eShop and
Steam, with labelled Japanese/English product/order fields; Steam also supports
receipt item cells and store app links. MIME transfer encodings and declared
Japanese character sets are decoded locally. Japanese folder names are encoded
as IMAP modified UTF-7, including literal ampersands. Amazon quoted-subject title fallback
is tentative and may be truncated. Status comes from the subject to avoid
cancellation/refund help text in receipt footers. Unknown layouts/oversize receipts
become an unparsed review entry rather than an owned game. Limits are 2 MiB/mail,
100 MIME parts, 20 product names/mail and 100 suggestions/batch. Malformed dates,
unsupported decoding and excessive MIME parts become unparsed suggestions with a
typed issue and locator; later messages can progress. Unparsed suggestions require
manual details/status before acceptance, can be dismissed to skip or restored.
The received date is used when a malformed sender date cannot be parsed. Transport
failures, incomplete message bytes and invalid response envelopes keep the saved
cursor and return typed authentication/certificate/folder/transport/adapter errors.
No failed batch advances its cursor. Templates without a receipt-like subject or
recognised sender can be missed. Gift, wallet, subscription, quantity and item-level
partial-refund semantics need manual review; no claim of universal template support
is made. Tests use synthetic receipts; actual personal receipt calibration remains
part of local setup, without uploading messages to chat.

## Proton Bridge deployment

See [the Portainer and authentication runbook](../deploy/proton-bridge/README.md).
Fi is the AMD64 Linux application host; FatChocobo supplies the NFS archive.
Proton offers CLI mode and the Linux `pass` keychain; container packaging is
self-managed. Official Linux packages are used
unchanged; no Proton protocol reimplementation or public IMAP proxy is included.

## Bridge status and web connection

The administration-only `GET /api/v1/mail/setup` query reports a typed Bridge
phase, an optional last-reported percentage/elapsed/remaining time, event time and
public certificate fingerprint. Purchase inbox polls every five seconds while
open, independently of receipt review. Unknown, stopped, interrupted and stale
reports are visible; no percentage or completion is invented. The reporter's
heartbeat expires after twenty seconds. Percentages cover an observed single
account; concurrent-account progress is not averaged. These are Bridge mailbox
events, separate from Collector's receipt-history cursor and last-check/problem
fields. Progress older than two minutes is explicitly labelled as waiting for a
new report and its old ETA is hidden.

The container supervisor runs the pinned official CLI, reduces its sync events to
one bounded atomic status file, and keeps raw output/account names/message IDs
out of that file. Only a public certificate fingerprint crosses the shared
read-only status bind. Certificate export stays inside Bridge's protected home,
including its private key. The terminal query uses the same snapshot without
opening another Bridge process or interrupting sync:

```sh
ssh fi-home /home/link/bin/collector-mail status
```

`POST /api/v1/mail/setup` binds the profile owner to the authenticated enabled
administrator and accepts only local IMAP credentials and folder names. A pinned
STARTTLS/login/EXAMINE/STATUS probe must succeed before publication; it never
fetches message bodies. Credentials and the input-bound command receipt publish
together in a private immutable file under `state/mail-profiles`. Repeated inputs
converge after interruption/restart; reusing the same receipt with different
inputs conflicts before mutable Bridge status is read. Successful creation is
visible immediately without restarting Collector. The user then explicitly
enables scanning for an authorized collection. **Revalidate Bridge connection**
accepts the same local IMAP username, current password and an explicitly confirmed
current certificate fingerprint. A pinned authentication probe precedes atomic
replacement of the active private profile. Expected configuration revisions and
input-bound, secret-free repair receipts fence stale updates and recover uncertain
responses, including after restart. The provider serializes batch/error publication
with replacement, so obsolete fetches cannot report success or failure afterward.
Connection identity, cursors, enablement and suggestions remain intact. Existing
operator profiles use a managed override with the same ID; their original files are
retained unchanged. Receipt replay does not restore superseded credentials. Profile
removal and non-administrator credential provisioning remain operator tasks.

Both setup routes belong to administration listener composition and require the
administrator role. Existing profile/connection reads remain owner-scoped. The
managed credentials are reversible private operational data, not archive content;
include them and Bridge's home in complete paired recovery points. No extra
per-message or per-poll history is retained.

## Recovery and review corrections — 2026-10-31

Each configured folder shows its last checked time and either not checked, history
backlog, or caught up at that check. Backlog uses UID positions, not message counts.
It covers only mail exposed by Bridge; initial Bridge sync and parser coverage are
separate. For an initial complete history pass, finish Bridge sync, then use
**Rescan configured folders**. Rescan resets the selected folders' cursor/progress,
retains suggestions and enabled/paused state, and fences prior work. A paused scan
stays paused until explicitly enabled. It also provides a retry after parser repair.
Older unparsed suggestions remain available for review/dismissal when a later parse
produces a new suggestion; rescanning never silently accepts a receipt.

**Restore suggestion** returns a dismissed receipt to Needs review. **Reopen purchase
review** previews the affected accepted purchase and requires **Confirm reopen
purchase**. The command removes that current purchase from the Item and returns
all of its linked receipts to review; earlier accepted details remain in archive
history. Review again to correct purchase fields or select another entry. No Item
inventory, manual metadata or files are removed. All review mutations bind the submitted suggestion revision. Both recovery effects
are owner-scoped and curation-authorized; accepted reopening also checks the Item revision. Private
review generations fence old forms. Reopening reserves those generations before
archive publication: an interrupted unsuccessful attempt can require a refresh,
but leaves either the accepted purchase or its committed reopened state intact.

Receipt review attaches older source keys to an existing matching order without
replacing newer status/date/details or duplicate intent. Newer receipts can update
that order normally. Orders with different retailer/reference still conflict.
Source review data stays immutable; corrections live in the accepted purchase row.
