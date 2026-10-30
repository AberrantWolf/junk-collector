# Email purchase suggestions

Branch delivery, 2026-10-30. Optional email intake proposes collection changes;
it does not automatically alter inventory or infer possession from a receipt.

## Boundaries

`MailProviderPort` is the installed transport boundary. Its first adapter uses
local official Proton Mail Bridge; Gmail is not implemented. The connector is
embedded in the provider adapter and runs with Python 3's standard library.
Retailer interpretation is independent of IMAP inside `mail/connector.py` and can
be reused by a future Gmail adapter. Providers are compiled/configured adapters;
there is no dynamic executable plugin installation interface.

The administrator provisions a private profile for a specific Collector user.
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
The DS1522+ is x86_64. Proton offers CLI mode and the Linux `pass` keychain, while
Synology/container packaging is self-managed. Official Linux packages are used
unchanged; no Proton protocol reimplementation or public IMAP proxy is included.
