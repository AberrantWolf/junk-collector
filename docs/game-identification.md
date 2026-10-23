# Automatic standard-game identification — 2026-10-10

Upload standard DAT files or ZIPs in Administration → Game catalogs, review the
detected documents, import them and activate the saved catalogs. Activation queues
existing preserved games. New guided imports use the same identification service;
Metadata → Identify game requests it again. No system string or normalization
profile is required in this workflow. Catalog downloads remain manual.

## Supported detection matrix

This is a finite supported matrix, not universal ROM/container understanding.
The registry accepts canonical names and recognized aliases (including PS1/PSX).
An extension never establishes a format, system or game identity.

| Input / catalog | Supported interpretation | Acceptance and limitations |
| --- | --- | --- |
| Logiqx XML or ClrMamePro DAT | Content-detected dialect; header name/description, author/URL and version identify recognized No-Intro/Redump conventions | Unknown systems/conventions are unsupported. Conflicting recognized systems require a choice constrained to that document. Conflicting families cannot activate. |
| NES, No-Intro | Strict trainer-free iNES/NES 2.0 payload; headerless PRG/CHR bytes; explicitly headered DAT bytes | Header fields and exact declared lengths must agree. Headerless identity requires a complete catalog match. Trainers, miscellaneous ROM data and unsupported directives are refused. |
| Nintendo 64, No-Intro | z64, v64 and little-endian source orders normalized to big-endian checksums | Magic, header/boot-code boundary, word alignment and reserved header fields must agree. Catalog checksums must be big-endian. No new CIC checksum algorithm is claimed. |
| SNES, No-Intro | Headerless payload or a candidate 512-byte copier-header layout | Alignment only proposes a layout. Complete catalog evidence must support the interpretation; alignment alone never identifies SNES. |
| PC Engine/TurboGrafx-16, No-Intro | Headerless HuCard payload or candidate 512-byte copier-header layout | Complete catalog evidence must support stripping/interpretation. No byte deinterleaving or new layout algorithm is added. |
| PlayStation, Saturn, Mega CD/Sega CD, Redump | Content-recognized CUE and ordered BINARY raw 2352-byte track spans | Validate directives, references, source lengths, track/index order and boundaries. All declared matching tracks must match. This is catalog matching, not sector error-correction validation or media health certification. |
| Mega Drive/Genesis and GBA, No-Intro | Raw catalog bytes, with recognized header contradiction guards | Mega Drive ROM boundaries; GBA logo, fixed/reserved fields and header checksum are checked when recognized. Identity still requires a complete catalog match. |
| Game Boy, Game Boy Color, Master System, Game Gear, No-Intro | Exact raw-byte catalog evidence | No separate structural decoder is claimed. Headerless/non-self-describing bytes need an unambiguous complete match; filenames and plausible sizes provide no identity. |
| UNIF, CHD and compressed game containers | Recognized unsupported containers | Preserve originals and show Unsupported; no raw-catalog fallback. ZIP support in this delivery is for catalog uploads, not game-container extraction. |
| Other bytes, ambiguous packages or unsupported conventions | Unrecognized, Ambiguous, Malformed or Unsupported with reasons | Preserve originals and retain honest inspection results. Unanchored multi-file packages and multiple CUE anchors need review. |

A complete match uses every supplied checksum and the strongest usable checksum
for attribution. CRC-only matches retain their weaker-evidence qualification.
Equivalent complete rows from different snapshots can converge; competing systems
or different complete identities remain ambiguous. Partial matches remain proposals
and cannot supply an accepted automatic title or system. Computed coordinates,
interpretations, catalog identities and algorithm/settings versions are retained;
reported candidates are cross-checked against their computed matches.

## Upload bounds and active catalogs

Each uploaded file and extracted DAT is limited to 64 MiB. A batch permits at most
128 archive members/files and 512 MiB of expanded content. Reject traversal,
non-regular links, encryption, nested archives and expansion beyond bounds while
reading. DAT readers bound total input, XML events/text lines, nesting, game count
and files per row before uncontrolled allocation. Upload preview stages immutable
bytes; acquisition revalidates the selected detected interpretation.

Active slots are scoped by collection, canonical system, catalog family and byte
variant. Replacement keeps unrelated systems active and retains earlier snapshots
and evidence. Recognized historical configuration becomes a slot; unsupported
historical configuration remains readable, inactive and explained. No archive
cleanup or provenance compaction is performed.

## Jobs, authority and presentation

The application owns admission and orchestration. Shared `junk-libs-dat` and
`junk-libs-game-formats` own decoding, inspection and normalization; Games owns
system/convention eligibility and catalog interpretation. Provider adapters own ZIP
traversal and uploaded bytes. HTTP and React do not compose an identification
pipeline.

Activation atomically admits a durable collection scan. It traverses eligible
items in 100-item keyset pages, records each admitted member and cursor, yields its
processing slot while waiting, and recovers unfinished pages periodically. Direct,
import and scan requests retain durable ownership of equivalent active work.
Cancelling a requester stops a child only when its last owner is gone. Partial
failures remain visible; retry failed children before retrying their parent scan.

Workers validate actual bytes with cancellation checks, then inspect/match and
recheck before publication. Publication is fenced against source changes, item
generation, current association, collection binding, permission loss, cancellation
and superseded catalog sets. Later manual edits are resolved independently from
frozen identification inputs. Browse, preview, admission and polling use recorded
fingerprints and do not hash original payloads.

Title, edition details and system have Automatic / Manual override controls.
Automatic shows the effective detected value and source. Manual values require
editing and Save; Use detected value clears that field's override. Untouched import
labels remain provisional; deliberate edits survive refresh. Historical explicit
choices remain authoritative. New resolution records retain their inventory/domain
baseline, and historical resolution policies retain their original field-origin
interpretation. A reset is explicit, not inferred from text such as `disc`.

Accepted title, edition and system feed detail, browse and search immediately.
Games use game/game match/edition terminology, music uses album/release terminology,
and other modules use entry/match. Original inventory and preserved bytes remain
independent of descriptive identification.

## Delivery and qualification

See [dated validation](validation/game-identification-2026-10-10.md) for executable
fixtures, restart/ownership/publication tests and real browser acceptance. New
normalization/format algorithms, arbitrary systems/conventions, game artwork
providers and provenance compaction remain outside this delivery.

Collector pins the published `junk-libs` Git revision
`83177af57ca36a5b284298020fea9d5e8fb0c415`; no local Cargo patches are required.
Retro Junk's affected DAT constructors and facade were updated and checked against
these shared changes. The shared revision is pushed to the `junk-libs` repository.
