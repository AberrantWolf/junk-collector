# Email purchase suggestions

Branch delivery, 2026-10-30. Optional email intake proposes collection changes;
it does not automatically alter inventory or infer possession from a receipt.
Installed on Fi on 2026-10-31; see [local account setup](../deploy/proton-bridge/README.md#current-fi-installation--2026-10-31)
and [deployment evidence](validation/fi-email-deployment-2026-10-31.md).

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
Other users cannot enumerate the profile, credentials or suggestions. The initial
UI supports enable/pause; changing an existing profile's destination is refused.
Use a new profile ID to import to another collection. No public mail/SMTP tool is
exposed. Profiles live outside the portable archive, are never returned by the
API, and must be mode 0600/0400. The file contains reversible local Bridge
credentials and needs protected local storage and encrypted backups.

Pending/dismissed suggestions, connections and cursors live in private SQLite.
Accepted orders are portable, role-redacted Item records, retained by ordinary
Item edits, reindex and archive-only recovery. Readers with Viewer access receive
no purchase details. Full local backups include private SQLite; back up the
external credential file and Bridge home separately and privately.

## Review

Open **Purchase inbox**, select a collection, expand **Private mail connections**
and enable its configured account. Every receipt match needs explicit review.
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
cannot be applied again to another Item. Disabling a connection fences in-flight
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
attachments, shipping addresses, payment details and mail headers are not retained
or sent to browsers. The parser keeps selected product text, order reference,
status and source date; inspect these fields during review. No external HTML
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
100 MIME parts, 20 product names/mail and 100 suggestions/batch. Malformed mail or
unsupported decoding can leave a visible failed sync for operator review; no
failed batch advances its cursor. Templates without a receipt-like subject or
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
enables scanning for an authorized collection. Existing-profile rotation/removal
and non-administrator provisioning remain operator tasks.

Both setup routes belong to administration listener composition and require the
administrator role. Existing profile/connection reads remain owner-scoped. The
managed credentials are reversible private operational data, not archive content;
include them and Bridge's home in complete paired recovery points. No extra
per-message or per-poll history is retained.
