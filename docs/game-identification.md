# Game catalogs and identification

## Using it

1. Download DAT files from [DAT-o-MATIC](https://datomatic.no-intro.org/) (see the
   [No-Intro download guide](https://wiki.no-intro.org/index.php?title=Database_Navigation_Guide))
   or [Redump](https://redump.info/downloads). Collector never downloads catalogs itself.
2. In **Administration → Game catalogs**, upload the DAT files or ZIPs, review the
   detected documents, import them and activate the saved catalogs.
3. Activation queues identification of existing games in the collection. New guided
   imports use the same service, and **Metadata → Identify game** runs it again for one item.

No system string or normalization profile has to be entered. A game imported without
an active catalog keeps its originals and reports that no catalog match was
attempted; that is not a failed verification. Game collections do not show the
music-only import and preparation controls.

## Supported formats

The supported set is finite. An extension or file name never establishes a format,
system or game identity. System names accept recognized aliases (for example PS1/PSX).

| Input / catalog | Interpretation | Limits |
| --- | --- | --- |
| Logiqx XML or ClrMamePro DAT | Dialect detected from content; header fields identify recognized No-Intro/Redump conventions | Unknown systems or conventions are unsupported. A document that matches several systems needs a choice |
| NES (No-Intro) | Trainer-free iNES/NES 2.0; headerless PRG/CHR; headered DAT bytes | Header fields and lengths must agree. Trainers and miscellaneous ROM data are refused |
| Nintendo 64 (No-Intro) | z64, v64 and little-endian, normalized to big-endian | Header must be consistent |
| SNES (No-Intro) | Headerless, or a 512-byte copier header | Layout must be confirmed by a complete catalog match |
| PC Engine / TurboGrafx-16 (No-Intro) | Headerless HuCard, or a 512-byte copier header | Layout must be confirmed by a complete catalog match |
| PlayStation, Saturn, Mega CD (Redump) | CUE with ordered BINARY 2352-byte tracks | CUE structure and track boundaries are validated; all tracks must match. Not a sector error-correction check |
| Mega Drive / Genesis, GBA (No-Intro) | Raw bytes, with header sanity checks | Requires a complete catalog match |
| Game Boy, Game Boy Color, Master System, Game Gear (No-Intro) | Raw bytes | Requires an unambiguous complete match |
| UNIF, CHD, compressed game containers | Recognized but unsupported | Originals are kept and shown as Unsupported |
| Anything else | Unrecognized, Ambiguous, Malformed or Unsupported, with reasons | Originals are kept. Packages with no anchor or several CUE files need review |

A complete match uses every supplied checksum; CRC-only matches are marked as weaker
evidence. Identical rows from different catalog snapshots agree; competing systems
or titles stay ambiguous. Partial matches are proposals and never become an
automatic title or system.

## Upload limits and active catalogs

- Each uploaded file and extracted DAT is at most 64 MiB; a batch allows at most
  128 files/archive members and 512 MiB expanded. Path traversal, links, encryption
  and nested archives are rejected.
- Uploading requires server administration plus write access to the Games
  collection. The route only exists on management listeners.
- An accepted upload survives browser closure and restart; Administration resumes
  showing its progress. Repeating the same upload returns the same job; different
  bytes under the same request conflict.
- Malformed input fails the job and saves no catalog.

One catalog is active per collection, system, catalog family and byte variant.
Replacing one keeps other systems active and keeps earlier snapshots and results.
Unsupported older configuration stays readable but inactive, with an explanation.

## Identification jobs

Activation starts a durable collection scan that walks items in pages of 100 and
resumes after restart. Equivalent work requested by an import, a scan or a direct
request is shared; cancelling one requester stops the work only when no other
requester remains. Failures stay visible; retry failed items before retrying the scan.

Workers read the actual bytes (cancellably), match, and re-check before publishing.
Publication is abandoned if the source, item, collection, permissions or active
catalogs changed meanwhile. Browsing, previews and polling never hash originals.

## Metadata authority

Title, edition details and system each have **Automatic / Manual** controls.
Automatic shows the detected value and its source. Manual requires editing and
**Save**; **Use detected value** clears the override. Untouched import labels stay
provisional, deliberate edits survive refresh, and resets are always explicit.
Accepted values appear immediately in detail, browse and search.

The decoding and matching code lives in the shared `junk-libs-dat` and
`junk-libs-game-formats` crates; the Games module decides eligibility and
interpretation.
