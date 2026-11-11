# FLAC and MP3 purchases — HL-02

Architectural rules: [ADR 0006](decisions/0006-source-bound-evidence-and-coverage.md), [ADR 0007](decisions/0007-manual-metadata-authority.md), [ADR 0012](decisions/0012-durable-workflow-ownership-and-fencing.md).
This document retains detailed requirements, rationale and evidence; the ADRs do
not replace its supported-scope and recovery limits.

Implementation record, 2026-10-19. [Remaining work](remaining-work.md)
is the sole queue; [dated qualification](validation/digital-purchases-2026-10-19.md)
records independent vectors, real Chrome and actual Trash Amp GUI acceptance,
including streaming, seeking, downloads and server-stopped offline reopen. HL-02
is delivered for the supported matrix below.

The [2026-10-29 matching delivery](validation/digital-album-matching-2026-10-29.md)
adds explicit audio-based discovery, reviewed application and shared community
MusicBrainz models. Its [dependency assessment](digital-album-matching-evaluation.md)
records compatibility checks and upstream defects; see the matching workflow below
for configuration and limits.

## WAV addition and completion — 2026-11-01

Published in beta.15; see
[publication/deployment evidence](validation/container-beta15-publication-2026-11-01.md).

Digital ingestion also accepts integer PCM WAV originals: 16/24-bit mono/stereo,
8–192 kHz, strict RIFF or checked WAVE_FORMAT_EXTENSIBLE PCM. Preserve exact WAV
bytes and select **Prepare FLAC after import**, or prepare FLAC later in the
entry's Music editor. The configured FLAC encoder is required only for preparation.
Prepared FLAC retains sample rate, channels and bit depth and is independently
compared against source PCM before publication. MP3 and FLAC originals continue to
play directly, including in mixed packages, without redundant conversion.

WAV INFO and ID3v2.2/v2.3/v2.4 tags and embedded covers now feed the same
metadata/artwork workflow as FLAC/MP3. Manual metadata remains authoritative.
Explicit audio matching fingerprints WAV PCM directly, without an encoder, and
honors album inclusion/exclusion choices. Matching supplies identity candidates;
it does not verify the catalog or prove a complete release match.

For previously retained digital packages, open the Music editor, choose
**Analyze retained digital audio**, then **Analyze retained audio**. The durable
Job records file analysis and refreshes automatic metadata/artwork in place;
originals, Captures, physical inventory and manual edits survive. Repeating analysis
reuses artwork and does not restore explicitly deleted artwork.

The prepared FLAC download includes current accepted album/track details, available
cover preview and a descriptive filename. This is a temporary metadata repack:
archived originals and listening derivatives are unchanged, with identical PCM.
It follows the normal media-access policy, checks source bytes, supports cancellation,
and admits two concurrent exports per server state with an 8 GiB output bound.
MP3 originals remain unchanged and need no conversion.

Float/compressed WAV, multichannel, RF64 and continuous-image/CUE splitting remain
unsupported codec/layout forms, rather than inherent limitations of WAV. The
supported integer PCM matrix above is the delivered boundary. See
[current completion validation](validation/wav-digital-completion-2026-11-01.md)
and the [initial delivery](validation/wav-digital-originals-2026-10-31.md).
The dated FLAC/MP3-only delivery record below retains its original scope; WAV
preparation is the explicit exception to its direct-original playback policy.

## Inbox format compatibility — source work, 2026-11-01

Integer PCM WAV also accepts 40-byte `fmt` chunks containing an empty, entirely
zero-padded PCM extension. Nonzero unknown extensions remain unsupported; the
sample format, RIFF/chunk boundaries and original bytes retain the existing checks.
MP3 inspection accepts bounded APEv2 suffix metadata, with optional matching header
and final ID3v1 tag, including text and JPEG/PNG front covers. ID3v2 text retains
priority over APE, followed by ID3v1 fallback. MP3 decoding includes the upstream
zero-length-channel correction rather than skipping failed frames.

For imports whose **Identify files** step failed before originals were preserved,
open **Review import** and choose **Retry this step** after updating the server.
The same Job validates the archived originals against its original frozen file
fingerprints and runs retained-audio analysis. Preservation is not repeated and
no original bytes or existing manual metadata are replaced. Unsupported or damaged
sources still fail visibly. This change does not add float/multichannel WAV or new
codecs. See [delivery and validation](validation/audio-compatibility-2026-11-01.md).

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
when those labels/order change. Album playback defaults to one format per explicit disc, preferring FLAC, then
WAV, then MP3. This also applies to loose mixed-format files, without inferring
sample equivalence. A reviewed import selection or later track override takes
precedence. Same-format files with conflicting track slots still require a choice;
individual originals remain available. Browser queues, collection lists, audio
matching and preparation use the effective selection.

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
Each direct regular file or directory in a configured server inbox is one package.
A digital album keeps its format branches, numbered disc folders, named sections,
artwork and companions together. Separate top-level discs can use the same reviewed
album group, with a physical-copy label where appropriate.

Browser folder selection has a **Folder layout** control. **Best guess** treats
loose audio or immediate format/disc folders as one album and otherwise
discovers top-level items. Choose **One album in the selected folder** for an album with custom section
folders, or **Separate top-level albums / discs** for an imports container. Original
paths and all reviewed files are preserved in either mode. Raw-disc anchors keep
their established analysis boundaries.

Use a music collection to import playable albums. If you review loose audio
in another collection, Imports identifies that destination and offers a switch
to an editable music collection. Review the source again in the music collection.

Expand **Files and album playback** to inspect the suggested selection. File and
folder checkboxes choose the album's playback files. In a mixed directory, audio
files are grouped by format; **Select FLAC**, **Select WAV** and **Select MP3**
replace playback selection across the package with the named format. The
**Curators only** checkboxes control access to individual originals.

Set **Number of discs** and choose **Apply disc count**, then assign checked
tracks or folders with their **Disc** controls. Applying the count puts tracks
with automatic numbering on Disc 1 and moves tracks from higher disc numbers
onto the last disc. Only discs assigned to tracks become part of the album.
Every reviewed original is preserved, regardless of playback selection or disc
assignment. To change track numbers after import, use **Edit track details**
in the entry's Music editor.

To split an ordered track list, choose **New disc** on the first track of the
next disc and then choose **Apply to below**. This assigns that disc to every
later checked track in the displayed file order, including tracks in collapsed
folders or on later pages. Repeat from the start of each next disc.

Use the up and down arrows to move checked tracks within the same folder and
format. **Apply to below** follows that reviewed order. The **Track no.**
placeholder shows the audio file's position before tag analysis. Enter a number
and press Enter or Tab to place the track among checked tracks on the same disc
in that folder and format. The entered number overrides embedded tags. Clear
the field to use automatic numbering again. Unchecked tracks have no disc or
track number controls and remain available as originals.

The default prefers FLAC, WAV, then MP3 within each discovered numbered disc.
`Album/FLAC/Disc 1` and `Album/Disc 1/FLAC` both work. Mixed formats in the same
folder are treated as alternate copies. Unknown named audio branches, different
format track counts and missing disc folders in a multi-disc layout show **Needs
attention** and require a confirmed selection before starting. The proposal uses
names and counts. Actual audio validation remains background work after staging.
An explicit choice is retained across draft reload, retry and catalog rebuild,
and later manual track edits keep authority. After **Import review saved**, the
review can be reopened. If the browser has released its file handles, use
**Reselect folder for reviewed import** to restore the originals without replacing
the review.

The tree is built only for opened reviews. Folder children are paged in groups of
50. At more than 32 path levels, the remaining original path is shown at the leaf.
Existing intake and metadata limits remain applicable. The format default also
applies to older mixed-format packages without a manual track choice. Existing
package boundaries and saved track targets retain their identities.
See [current source validation](validation/album-import-review-2026-11-11.md).
The [beta.16 evidence](validation/format-folder-discovery-2026-11-01.md) records the
historical separate-format policy.
A successful replacement preview replaces the reviewed files. A failed preview
leaves the previous review and its selected originals available.

Review exposes destination/group, playback membership, disc assignment, visibility
and private purchase facts before admission. Format grouping is display only.
The arrow controls and track number edits can change reviewed file order.
Reviewed inbox order and visibility survive rescans; inventory
changes fail admission/recovery. Submitted purchase inputs and receipts survive
reopen. Failed analysis still permits preservation, with failed steps visible and
no successfully analyzed/playable track claim.

Manual disc assignments take precedence. For digital originals, numbered disc
folders supply the default before embedded disc tags. Automatic preparation
uses the source belonging to that import after its playback selection is applied.
If the selection cannot be applied, the import reports the failure and retains
the preserved originals for retry.

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
artwork or listener availability. A restricted preferred track can make album
playback unavailable to a viewer. The other formats remain individual originals. Raw original authorization enforces each stored
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
Git revisions. Collector and independent clients declare their own exact dependency identities. Cargo manifests and lockfiles
record the resolved dependency identities. No sibling checkout is required.
Publication and dependency verification do not alter a live installation.

## Matching albums with missing tags

Open the item's metadata workspace and choose **Find matching albums**. Configure
both provider settings described in [configuration](configuration.md) first.
Review any alternate-file selection and file visibility before starting. Restricted
files are not disclosed; a queued background job validates recorded source bytes,
decodes the selected audio and sends fingerprints/durations to AcoustID. MusicBrainz
then supplies linked release candidates. Initial release detail demand is eight, with explicit expansion to 32. Recording
searches are bounded to 32 and alternate-edition browsing to four pages. A complete
unambiguous mapping with scores at least 0.95 and known duration differences within
two seconds stops further recording-membership searches. Optional editions/artwork
do not delay usable candidates. These are bounded suggestions, not exhaustive results.

Reopen the panel to resume polling. Cancel or retry through the persisted job.
Coverage and track mapping remain visible beside artwork; applying requires every
selected file and every release track to have an unambiguous assignment without
conflicting known durations. Application retains the chosen source snapshot,
uses the existing metadata/artwork workflow and preserves originals and manual
values. Later inventory, source or metadata-authority edits require a fresh search.
Partial matches can inform manual MusicBrainz selection but cannot apply an entire
track mapping automatically. These suggestions do not establish catalog verification.
