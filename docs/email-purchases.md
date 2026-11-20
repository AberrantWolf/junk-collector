# Email purchase suggestions

Optional email intake reads retailer receipts and proposes purchases for review. It
never changes inventory by itself or infers that you own something from a receipt.
Install Proton Mail Bridge with the [Bridge setup guide](../deploy/proton-bridge/README.md).
Gmail is not supported.

## Design

Mail access goes through `MailProviderPort`. The only adapter uses a local official
Proton Mail Bridge over IMAP; its connector (`mail/connector.py`, Python standard
library only) keeps retailer parsing separate from IMAP so another provider could
reuse it. There is no plugin interface.

- Credential profiles live outside the portable archive (mode 0600/0400) and contain
  reversible Bridge credentials. APIs never return them. Back them up privately
  together with Bridge's home directory.
- Pending and dismissed suggestions, connections and cursors live in private SQLite.
  Accepted purchases are portable item records. Viewers see no purchase details.
- IMAP access uses EXAMINE and BODY.PEEK only: mail is never marked read, moved,
  deleted or sent. (The Bridge credential itself could do more, so keep it private.)
- Bodies, attachments and raw messages are not stored or sent to the browser. Only
  the extracted product text, order reference, status and date are kept, plus a
  private locator (folder, UID, UIDVALIDITY, Message-ID, subject, date) visible only
  to the profile owner. No external HTML resources are fetched and no mail data is
  sent to any lookup service.

## Connecting

An administrator connects their own Bridge from the **Connect your email** card in
**Purchase inbox**, entering Bridge's local IMAP credentials (Proton login and 2FA
happen in the Bridge terminal). Collector checks the certificate pin and read-only
folder access, then saves the profile with scanning disabled. Operator-provisioned
profile files are also supported.

The profile's owner enables scanning for one collection they can curate. Other users
cannot see the profile or its suggestions. A profile's destination collection cannot
be changed; use a new profile ID for another collection.

**Folders:** during setup or with **Edit scanned folders**, use **Find mail folders**,
pick from **Available mail folders** and **Add folder**; or type 1–16 exact names, one
per line, and **Save folders**. Include `INBOX` to keep scanning the inbox; archived
receipts need their folders or Bridge's All Mail folder. New folders start from the
beginning; removed folders stop being read but keep their suggestions. Changes apply to
the next batch without a restart. Inaccessible folders show a scan problem.

**Revalidate Bridge connection** replaces the saved credentials after you confirm the
current certificate fingerprint; cursors, suggestions and enablement are kept.
Removing profiles and provisioning them for non-administrators are operator tasks.
Both setup routes exist only on administration listeners.

## Scanning

Polling runs one account at a time, one minute after the previous batch, with a
90-second fetch deadline. History is read from UID 1 in windows of 100 UIDs; each
batch commits its suggestions and cursor together, and a failed batch never advances
the cursor. A UIDVALIDITY change rescans the folder; duplicate messages across folders
and rescans are suppressed.

Only known retailer sender domains with receipt-like subjects are fetched. Headers are
routing hints, not proof of authenticity.

**Supported retailers:** Amazon Japan, Joshin, Square Enix e-STORE, Nintendo eShop and
Steam, with labelled Japanese/English product and order fields; Steam also supports
receipt item cells and store links. Amazon Japan's English **Ordered**, **Shipped**,
**Out for delivery** and **Delivered** notices are recognized by a labelled order number
and marked product links (titles come from link text or image alt text; recommendation
links are ignored). These are marked as physical purchases. Status is taken from the
subject, not footer help text.

MIME encodings, Japanese character sets and modified UTF-7 folder names are decoded
locally. Limits: 2 MiB per mail, 100 MIME parts, 20 products per mail, 100 suggestions
per batch. Unknown layouts, oversized mail, bad dates and undecodable content become
**unparsed** suggestions that need manual details. Gift, wallet, subscription, quantity
and partial-refund cases need manual review. Unrecognized templates can be missed.

## Reviewing

Open **Purchase inbox** and select a collection. Every suggestion needs explicit review:
correct title, platform, edition, format, order reference and status, then link an
existing entry or create an unidentified Games entry (creating entries in other kinds
of collection is not supported). Nothing else — physical copy, dump, catalog identity —
is created. Manual titles and metadata are not changed by linking.

- Matching ignores Unicode width, case and punctuation. A linked order's title becomes
  an alias for later suggestions, so Japanese and English titles converge. Matching
  checks the first 5,000 entries and shows up to 20 matches; use library search
  otherwise. Overlaps are warnings; a duplicate can be marked intentional.
- Cancellation and refund notices can update a matching order; source dates prevent an
  older message overwriting a newer status. To update an accepted order from a later
  notice, link it to the existing entry and choose the order in **Order action**.
- **Dismiss** suggestions you don't want; **Restore suggestion** brings one back.
- **Reopen purchase review** (then **Confirm reopen purchase**) removes the accepted
  purchase from the item and returns its receipts to review. Inventory, metadata and
  files are untouched; the earlier purchase stays in archive history.

Review decisions are idempotent and checked against the suggestion and item revisions,
so concurrent or replayed submissions cannot double-apply. Disabling a connection stops
in-flight batches without losing its cursor.

## Status and recovery

Each folder shows its last check and whether it is unchecked, has backlog (by UID
position), or was caught up. For a complete first pass, let Bridge finish syncing,
then use **Rescan configured folders**. Rescan resets the folders' cursors but keeps
suggestions and the enabled/paused state; use it again after a parser fix. A rescan
fills missing fields on matching unaccepted suggestions; dismissals stay dismissed.

`GET /api/v1/mail/setup` (administrators only) reports Bridge's sync phase, progress,
time estimates and certificate fingerprint; Purchase inbox polls it every five seconds.
Reports expire after twenty seconds; progress older than two minutes is shown as
waiting, without an ETA. The container supervisor writes this sanitized status to
`${PROTON_BRIDGE_STATUS}/status.json` (no account names or message IDs). A missing or
stale file does not mean sync finished.
