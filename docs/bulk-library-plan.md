# Bulk import, metadata repair and preparation

Accepted 2026-10-03. This user assignment takes precedence over the previously
active serial handoff. Preserve actual archive/private data and all P06 recovery
and lightweight-query guarantees. Full accepted scope: dedicated Library,
Imports, Library care (Metadata / Music preparation), Activity pages; authoritative
computed disc-ID album matching and medium placement; user metadata language;
shared durable resolution; resumable 10,000-package / 100,000-file intake;
confirmed grouping including curated-entry conflict resolution and guarded undo;
frozen explicit/all-filter selections; durable bulk FLAC preparation with shared
request ownership and bounded rolling admission.

## Delivery ledger

| Delivery | State | Acceptance |
| --- | --- | --- |
| BL1 shared resolution, language and Metadata page | Locally accepted | Disc IDs, equivalent releases/languages, real ambiguity, retained manual/automatic decisions, import integration and browser review |
| BL2 bulk intake and confirmed grouping | Locally accepted | Thousands of packages, restart/resume, curated conflicts, group commit, preserved identities/history, guarded undo |
| BL3 preparation and Activity | Locally accepted | Frozen selection, rolling admission, shared cancellation/retry, real playback and scale measurements |

Defaults: metadata automatic, prepare-after-import explicit and unchecked,
FLAC only, music plus existing DAT sources, originals preserved before review,
copies kept distinct unless explicitly mapped, same-collection merges, confirmed
entry combination even where matching is authoritative. Language variants of one
album are not ambiguity. Filename/text-search matches remain suggestions. A disc
ID identifying genuinely different albums requires review.

## Boundaries and recovery

Application library-care services own decisions and typed ports. Store adapters
own indexed queries, private batch state and archive group publication. Modules
own album/provider semantics. Frontends present server-owned readiness/actions
and transport/resume uploads, never implement a second provider workflow.
Resolution is shared between intake and repair. Accepted automatic metadata is
separate from manual candidate selection and latest suggestions. Manual fields,
artwork and unknown module records survive refresh and merging.

Grouped merges stage immutable entity revisions and publish a complete portable
group manifest last. Archive readers ignore uncommitted group revisions; recovery
removes incomplete group artifacts before that marker and finishes the complete
projection after it. Projection/ownership/reference updates and the P06 checkpoint
commit together. Merge lineage and observations remain portable. Private batch
claims/request ownership fence child admission and cancellation in one transaction.
No query, preview or admission hashes media bytes.

## Delivery record — 2026-10-03

All three deliveries are implemented across application, store, versioned API and
addressable web pages. Acceptance evidence and its limits are recorded in
[bulk validation](validation/bulk-library.md). The archive remains authoritative;
private schema 18 retains credentials, grants and server configuration. Catalog 11
and checkpoint format 4 rebuild metadata health, language, album-peer and FLAC
readiness indexes without reading media for listing, selection or admission.

BL1 uses one metadata-resolution Job for guided imports, individual lookups and
bulk repair. It acquires immutable provider observations, commits text, and then
attempts optional artwork through the existing acquisition/publication effects.
Computed layouts establish authoritative disc IDs. Same-album pressings and language
variants resolve automatically; incomplete album identity and genuine ambiguity
remain reviewable. Accepted information survives ambiguous refreshes; manual title,
artwork, fields and explicit release authority survive automatic updates. Account
settings persist Default metadata language; existing accounts initially use English.

BL2 saves bounded discovery and intake manifests, expected complete manifest counts,
package/group destinations, admission progress and per-package failures. Package
preservation proceeds independently of uncertain metadata. Browser originals must
remain available until upload completes; after reload, reselect the original folder
to resume every unfinished package. Large review drafts use IndexedDB. Destination
conflicts can preserve separately without stopping unrelated packages. Group previews
resolve manual information, identity/artwork choices, copy mapping and disc positions.
Unknown physical inventories must be completed before combining sets. Group commits
retain original records and source lineage, update ownership/references/projections
atomically, recover every publication phase, and allow undo only while all affected
records remain at the resulting revisions.

BL3 freezes explicit or all-filter selections, exclusions, inventory revisions and
source observations. Rolling parent admission fills existing worker/album limits,
shares track jobs through durable request ownership, and keeps independent outcomes.
Completed outputs are reused; retry targets failed work. Cancellation removes only
that request's ownership. Both root-admission and member failures have visible
recovery actions. Activity and Imports reopen whole batches, with bounded outcome
pages; periodic recovery covers all unfinished pages. Metadata and preparation filters,
loaded result pages, selections and review drafts survive task navigation. The player
remains outside task pages; mobile reviews have an explicit return path and keyboard
focus returns to the originating result.

## Next handoff

No BL1–BL3 implementation remains assigned. Continue the existing
[LA3 second-device qualification](local-alpha-plan.md#ready-to-send-la3-prompt)
with the intended trusted physical LAN device. Preserve the passing local corpus,
provider and restore records; qualify new changes separately. Keep blocked DAT
acquisition disabled. General post-edit splitting, new providers, native/offline
clients, synchronized listening and backup UI remain separate scoped deliveries.
