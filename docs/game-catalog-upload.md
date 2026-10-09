# Game imports and manual catalog upload — 2026-10-09

This delivery implements the three game/import TODOs requested on October 9.
It does not change LA3/PJ1 physical qualification or implement synchronized listening.

## Contract

The application returns `music_actions_available` with each ingest package plan.
Guided import uses it to show music preparation controls and to fence the submitted
preparation flag. Game collections omit FLAC controls, audio-preparation suggestions,
Music editing navigation, album grouping and the scoped Prepare audio navigation.
Music retains those controls, including honest missing-encoder availability.
Non-music import details omit the music preparation step. A game imported without
an active catalog reports saved originals, unavailable automatic identification,
and that no catalog match was attempted; this is not a failed dump verification.

Browser DAT uploads require server administration and content-write eligibility
for the intended Games collection. The administration route is composed only on
eligible management listeners. Authorization precedes body consumption. At most
two uploads are buffered concurrently, each bounded to 64 MiB; local configured
sources retain their existing 512 MiB streaming limit. Empty, oversized, ZIP and
unsupported byte-variant inputs receive actionable errors. Archives must be
extracted by the user. This workflow makes no provider download requests and
leaves the disabled DAT-o-MATIC network path unchanged.

The storage adapter syncs exact bytes into a private content-addressed spool before
admission. A narrow application port supplies opaque seekable readers, never paths,
to the existing local DAT provider. The acquisition query freezes platform,
catalog family, byte domain/version, SHA-256 and length. The upload command receipt
binds submitted metadata and digest/length before resolving mutable binding and
timestamps. Repeating the request retains its first Job and snapshot timestamp;
changing bytes or metadata conflicts. An accepted upload survives browser closure
and server restart. The browser retains the accepted Job identity in account-scoped
session storage and resumes polling when Administration reopens.

Existing acquisition Jobs, cancellation, strict shared XML/ClrMamePro parsing,
index bounds and immutable snapshot publication are reused. Normalization and
index validation check cancellation inside their loops. Staged bytes must match
the frozen length and SHA-256 before parsing/publication. Malformed input produces
a failed Job and no saved catalog. Snapshot observations include up to three
parsed entry names and up to three filenames per entry for review; the sample is
not exhaustive. Reading a snapshot verifies a supplied sample against its parsed
bytes. Older observations without samples remain readable; original observations
are not rewritten. Private upload spools are retained for durable Job retries and
are included in private-state backups; automatic spool garbage collection is not
provided by this delivery.

Upload and activation are separate. The UI shows the entered platform/family,
the document's name/version, row count, byte variant and a parsed sample before
explicit activation. One active local-DAT catalog is selected per collection;
previous snapshots and results remain. Activation does not automatically recheck
existing games or replace manual authority. Instructions direct the curator to
Compare with a DAT snapshot in the entry's processing panel, then apply recorded
catalog metadata in Metadata.

## Download guidance

The UI links to [DAT-o-MATIC](https://datomatic.no-intro.org/), the official
[No-Intro download guide](https://wiki.no-intro.org/index.php?title=Database_Navigation_Guide),
and [Redump downloads](http://redump.org/downloads/). The No-Intro page and guide
were checked on October 9; the guide describes Download → system → settings.
The Redump page timed out during this check, so availability is not claimed.
No DAT download, form submission, scraping or provider retry was performed.
Instructions explain supported XML/ClrMamePro catalogs, manual ZIP extraction,
matching platform, raw-file versus headerless cartridge variants, and raw-2352
Redump CD tracks with retained CUE metadata excluded from that track byte domain.

## Acceptance

The exact checks and remaining limits are recorded in
[October TODO validation](validation/october-todos.md). The repeatable offline
browser journey is `npm --prefix web run test:game-catalog`; it uses a disposable
server/profile and fails on any external browser request. Storage regressions cover
restart, advancing-clock replay, changed-input conflicts, exact original bytes,
activation history/reindex, denied roles, malformed input, tampering, private-spool repair from freshly supplied exact bytes and cancellation.
HTTP regressions cover route exposure, unauthorized/body-read ordering, ZIP/empty/
oversized input, and byte/metadata receipt binding.
