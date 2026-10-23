# FLAC and MP3 purchases — HL-02

Implementation record, 2026-10-19. [Remaining work](remaining-work.md#hl-02--complete-flac-and-mp3-digital-purchase-support)
is the sole queue; [dated qualification](validation/digital-purchases-2026-10-19.md)
records independent vectors, real Chrome and actual Trash Amp GUI acceptance,
including streaming, seeking, downloads and server-stopped offline reopen. HL-02
is delivered for the supported matrix below.

## Portable ownership and listening identity

A digital package uses an ordinary Music Item and Capture, with optional
`DigitalPurchase` store, source URL, date, cost/currency and order reference.
Guided and bulk imports create no Specimen/carrier for these sources. Attaching a
purchase preserves existing physical inventory. Every reviewed file is retained
exactly, including its original tags. A purchased MP3 is a preservation master;
that role makes no claim about losslessness or earlier encoding history.

Music's immutable `FileTrackManifest` is committed as import-analysis output bound
to the complete source fingerprint and rebound package-file IDs. Its source track
positions follow the reviewed file order. Native format and decoded frames belong
to that source; editable titles, disc numbers, printed numbers and album inclusion
are separate metadata authority. Existing saved list targets never change files
when those labels/order change. Alternative encodings with the same file stem or
matching disc/track/title claims require an explicit album inclusion/exclusion
choice. Individual originals remain playable. Album/list expansion refuses an
unresolved choice; the browser queue and native album view honor the same fields.

Availability uses `profile: null` for originals, exact file ID/index/digest/length,
actual codec, sample rate, channels, optional bit depth and known delay/padding.
It does not manufacture a FLAC conversion profile, CD sectors, Disc ID or
AccurateRip result. Decoded frame count records what the reader consumed; it is
separate from gapless knowledge. FLAC codestreams encode lossless integer PCM;
MP3 codestreams are lossy. Neither reveals earlier encoding history. CD recipes and
prepared outputs keep their existing identities and 44.1-kHz timeline.

## Intake, metadata and artwork

Browser file/folder selection and configured inboxes use the existing signed
plan, resumable staging, guided/bulk intents and ordinary publication pipeline.
Discovery groups disc subfolders and keeps independent nested packages disjoint.
Review exposes destination/group, file order, visibility and private purchase
facts before admission. Reviewed inbox order/visibility survive rescans; inventory
changes fail admission/recovery. Submitted purchase inputs and receipts survive
reopen. Failed analysis still permits preservation, with failed steps visible and
no successfully analyzed/playable track claim.

The narrow `junk-libs-audio` reader owns structure, tags, pictures and decoding.
Music maps its result to portable sources and descriptive candidates. Local tags
provide useful album/artist and track defaults without a provider. Conflicting
values do not become consensus; absent track titles fall back to filenames.
Candidates include immutable manifest/fingerprint inputs and remain descriptive
Name matches, never complete catalog identities. Manual metadata survives refresh
and reindex. Public embedded MusicBrainz release IDs can request the existing
optional MusicBrainz/CAA pipeline; private purchase fields never enter queries.
Sidecar or embedded cover bytes use the ordinary bounded Asset publication effect,
with import cancellation and Item generation fences at publication.

Originals play immediately after successful analysis/publication; no encoder or
duplicate audio is required. Digital sources do not offer redundant preparation or
missing-CD/AccurateRip prompts. Existing CD preparation is retained. Web search,
queue and collection playlists, and Trash Amp streaming/seek/download/offline
metadata/artwork use the shared availability/source contracts. Cache identities
include account/server and exact file/content identity.

## Input and resource matrix

| Input | Implemented boundary |
| --- | --- |
| FLAC | Native mono/stereo integer PCM; independent 16/24-bit vectors at 44.1, 48, 88.2, 96, 176.4 and 192 kHz; comments and pictures |
| MP3 | MPEG-1/2/2.5 Layer III, independent mono/stereo vectors, CBR/VBR and missing Xing headers; known encoder delay/padding retained separately |
| Tags | ID3v1 fallback; ID3v2.2/2.3/2.4, ISO-8859-1, UTF-16/BOM, UTF-8, whole-tag and v2.4 frame unsynchronization; PIC/APIC; FLAC Vorbis comments/PICTURE |
| Unsupported | Free-format MPEG, compressed/encrypted ID3 frames and unsupported layouts fail analysis explicitly; AAC/ALAC/Opus/DRM/image splitting remain outside this delivery |
| Companions | Every reviewed file is preserved; unsupported companions do not prevent supported audio analysis; authorized original download, without requiring PDF rendering |

Metadata is capped at 32 MiB per file before third-party parsing, 256 tags/blocks
and 16 pictures, 4 KiB interpreted text fields, 64 KiB individual encoded text
frames/comments and 8 MiB aggregate interpreted package tags. Embedded image data
is capped at 20 MiB; FLAC internal picture MIME/description/data lengths are checked
before allocations. Existing Asset decoding caps dimensions at 12,000×12,000 and
allocation at 256 MiB, with a 1-MiB preview ceiling. File analysis caps at 10,000
tracks, 24 hours per track and 20 million MPEG frames. Intake's existing package/
file limits still apply. Cancellation is checked inside reads, frame traversal,
metadata loops, decoding and source validation. Browsing, search, polling and
admission use recorded observations rather than hashing/decoding media.

Malformed/truncated structures, wrong suffixes and invalid text fail analysis.
There is no silent padding or skipped corrupt track advertised as a complete
album. Preservation remains available with the actual analysis failure visible.
This does not qualify every possible MPEG extension or sample-accurate MP3 gapless
output; missing timing knowledge remains absent.

## Privacy and recovery

Purchase fields are curator-only. New digital imports default audio and ordinary
`cover`/`folder`/`front` images to shared; other companions, including receipt
photos and booklets, default to curators. Review can explicitly share a booklet or
restrict an entire audio file. Private audio contributes no public tag candidate,
artwork or listener availability. Raw original authorization enforces each stored
file's visibility, including historical sources. Presented files retain their
original indices after filtering. There is no whole-package download bypass.

Existing Captures have no inferred purchase and existing stored files retain their
previous shared policy; no retroactive classification or physical-copy deletion
occurs. New visibility and purchase facts are portable archive data. The private
intent column supports in-flight admission, not the only copy of acquisition facts.
Full restore and archive-only rebuild recover purchases, visibility, originals,
manual authority and immutable playlist targets through existing mechanisms.
Account/grant recovery remains explicit for archive-only recovery.

The authorized coordinated publication replaces local Cargo patches with exact
Git revisions. Collector pins junk-libs revision `2c8eaf8fd5a7f3e5f4873d3463b8008039e9af98`; Trash Amp pins the
published Collector and same junk-libs revisions. Cargo manifests and lockfiles
record the resolved dependency identities. No sibling checkout is required.
Publication and dependency verification do not alter a live installation.
