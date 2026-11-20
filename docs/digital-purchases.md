# Digital albums (FLAC, MP3, WAV)

Digital albums are imported as ordinary music items. Every reviewed file is kept
byte for byte, including its tags. Supported originals play directly with no encoder
or conversion. A purchased MP3 is a preservation master; that makes no claim about
losslessness or how it was encoded.

## Supported inputs

| Input | Support |
| --- | --- |
| FLAC | Mono/stereo integer PCM, 16/24-bit, 44.1–192 kHz; Vorbis comments and pictures. FLAC after a bounded ID3v2 prefix is accepted |
| MP3 | MPEG-1/2/2.5 Layer III, mono/stereo, CBR/VBR, with or without Xing headers; encoder delay/padding recorded |
| Integer PCM WAV | 16/24-bit mono/stereo, 8–192 kHz; strict RIFF or PCM `WAVE_FORMAT_EXTENSIBLE`; a zero-padded 40-byte `fmt` chunk is accepted |
| 32-bit float WAV | Mono/stereo, 8–192 kHz; non-finite samples are rejected. Live FLAC/Opus and saved Opus use 24-bit PCM rounded and clipped; no saved FLAC |
| Tags | ID3v1, ID3v2.2–2.4 (all text encodings, unsynchronization), PIC/APIC, APEv2 on MP3, WAV INFO, FLAC Vorbis comments/PICTURE. Priority: ID3v2, then APE, then ID3v1 |
| Companions | All kept and downloadable by authorized users; unsupported companions do not block audio analysis |
| Unsupported | Free-format MPEG, compressed/encrypted ID3 frames, compressed/multichannel/RF64 WAV, AAC/ALAC/Opus originals, DRM, single-image albums with CUE splitting |

Malformed or truncated files fail analysis visibly. The originals are still preserved,
but no track is claimed playable.

**Limits:** 32 MiB of metadata per file, 256 tags/blocks, 16 pictures, 4 KiB per
interpreted text field, 64 KiB per encoded text frame, 8 MiB of tags per package,
20 MiB per embedded image. Image decoding is capped at 12,000×12,000 pixels.
Analysis is capped at 10,000 tracks, 24 hours per track and 20 million MPEG frames.
Analysis is cancellable throughout. Browsing, search and polling never decode media.

## Importing

Use browser file/folder selection or a configured inbox. Each top-level file or
directory in an inbox is one package. Import into a music collection; if you review
loose audio in another collection, Imports offers to switch.

**Folder layout** (browser folders):

- **Best guess** treats loose audio, or immediate format/disc folders, as one album and
  otherwise discovers top-level items.
- **One album in the selected folder** for an album with custom section folders.
- **Separate top-level albums / discs** for a folder of many imports.

`Album/FLAC/Disc 1` and `Album/Disc 1/FLAC` both work. Within each disc, the default
playback format is FLAC, then WAV, then MP3; mixed formats in one folder are treated
as alternate copies. Unknown audio branches, different track counts per format and
missing disc folders show **Needs attention** and need a confirmed selection.

Expand **Files and album playback** to review the selection:

- File and folder checkboxes choose playback files. **Select FLAC / WAV / MP3** switch
  the whole package to one format. **Curators only** restricts individual originals.
- **Number of discs** + **Apply disc count** puts automatically numbered tracks on
  Disc 1 and higher-numbered tracks on the last disc. Only discs with tracks are part
  of the album.
- **New disc** on the first track of the next disc, then **Apply to below**, assigns
  that disc to every later checked track in display order. Repeat for each disc.
- The arrows reorder checked tracks within a folder and format. **Track no.** overrides
  embedded tags; clear it to go back to automatic numbering.

Unchecked files have no disc or track controls but are still preserved. Manual disc
assignments win; otherwise numbered disc folders win over embedded disc tags. Reviews
are saved (**Import review saved**) and can be reopened; if the browser has lost its
file handles, use **Reselect folder for reviewed import**. A failed replacement preview
keeps the previous review. Folder trees are paged 50 children at a time.

Edit track numbers later with **Edit track details** in the entry's Music editor.

## After import

- If **Identify files** failed or was skipped, open **Review import** and choose
  **Retry this step**. It re-validates the preserved originals, analyzes them and
  applies the saved selection, disc assignments and track numbers. Manual edits win.
- A failing playback file is reported by name. A failing alternate outside the playback
  selection is kept with a warning.
- For older packages, **Analyze retained digital audio** in the Music editor reanalyzes
  preserved files and refreshes automatic metadata and artwork without touching
  originals or manual edits.
- WAV originals can be converted to FLAC with **Prepare FLAC after import** or later in
  the Music editor; the encoder is only needed for that. Prepared FLAC keeps rate,
  channels and bit depth and is checked against the source PCM.
- The prepared FLAC download is a temporary repack with current album/track details,
  cover and a descriptive filename; the PCM is identical. Two exports can run at once,
  up to 8 GiB each.

## Metadata, playback and identity

Local tags give album, artist and track defaults without any provider; conflicting
values are not merged, and missing titles fall back to filenames. These are
descriptive candidates, never catalog identities. An embedded public MusicBrainz
release ID can trigger the normal MusicBrainz/Cover Art Archive lookup; private
purchase fields are never sent. Manual metadata survives refresh and reindex.

Album playback uses one format per disc (FLAC, then WAV, then MP3) unless the review or
a later track override says otherwise. Same-format files competing for one track slot
need a choice. Queues, lists, matching and preparation use the effective selection.
Saved list entries keep pointing at the same files when titles or order change.

Original availability reports the actual codec, rate, channels, bit depth and known
delay/padding. No CD sectors, Disc ID or AccurateRip result is invented for digital
sources, and they get no CD preparation or AccurateRip prompts.

## Purchase details and privacy

An item can carry an optional purchase record: store, URL, date, cost/currency and
order reference. Digital imports create no physical copy; attaching a purchase keeps
existing physical inventory. Purchase fields are curator-only.

New imports share audio and `cover`/`folder`/`front` images by default; other
companions (receipt photos, booklets) default to curators only. Restricted audio
contributes no tags, artwork or listener availability, and a restricted preferred
track can make album playback unavailable to viewers. Original downloads enforce each
file's visibility; there is no whole-package download bypass. Existing files keep their
previous visibility. Purchase and visibility facts are portable archive data and
survive restore and archive-only rebuild.

## Matching albums with missing tags

Configure the AcoustID and MusicBrainz settings in [configuration](configuration.md).
Open the item's metadata workspace and choose **Find matching albums**. A background
job decodes the selected (non-restricted) audio and sends fingerprints and durations to
AcoustID; MusicBrainz supplies the linked releases. Results are bounded suggestions:
eight release details at first (expandable to 32), up to 32 recording searches and four
pages of alternate editions. A complete unambiguous mapping with scores ≥ 0.95 and
durations within two seconds stops the search early.

Reopen the panel to resume; cancel or retry through the job. The mapping editor shows
each local file and duration beside its release track, with scores and tentative
assignments. Applying requires a complete one-to-one mapping without conflicting
durations; review and confirm it first. Applying uses the normal metadata/artwork
workflow and keeps originals and manual values. Edits to inventory, sources or metadata
afterwards require a new search. Matching is not catalog verification.

To contribute fingerprints, confirm the mapping, choose **Contribute missing
fingerprints to AcoustID**, and enter your personal
[AcoustID user API key](https://acoustid.org/api-key). Collector sends fingerprints that
had no recording candidates, their durations and the reviewed MusicBrainz recording IDs.
AcoustID processes submissions asynchronously; afterwards use **Refresh lookups** to
bypass cached misses.
