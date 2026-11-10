# Album metadata and artwork for players

Architectural rules: [ADR 0007](decisions/0007-manual-metadata-authority.md), [ADR 0013](decisions/0013-bounded-projections-and-client-refresh-state.md).
This document retains detailed requirements, rationale and evidence; the ADRs do
not replace its supported-scope and recovery limits.

Collector resolves accepted/manual metadata and artwork on the server. Remote
players such as Trash Amp read that published state; a Library refresh does not
query MusicBrainz, acquire Cover Art Archive images or prepare audio. Metadata
identity and missing artwork are separate states. A release match does not promise
that a cover exists or that its provider acquisition succeeded.

## Read sequence

Use the authenticated control origin and the enrolled account for JSON **and
thumbnail bytes**. The separately advertised media service handles audio; it is
not the thumbnail origin. A native client can use `EnrolledClient` for these reads.

1. Check identity/trust and `GET /api/v1/me`. Read permitted collections through
   `GET /api/v1/client/collections`.
2. Get `GET /api/v1/client/library?collection_id=COLLECTION`, then
   `GET /api/v1/client/albums?collection_id=COLLECTION&revision=CATALOG_REVISION`.
   Preserve the query scope and revision when following `next_cursor`. URL-encode
   opaque revisions/cursors. Publish title pages progressively; query tracks only
   on album open/play/download. See [layered refresh](layered-library-refresh.md)
   for byte bounds, retries and track headers.
3. Each `ClientAlbum` has `id`, `collection_id`, `title`, `artist`, `row_revision`,
   `artwork`, `artwork_revision` and `summary_pending`. `artwork` is an Asset UUID,
   not a URL, embedded image, release UUID or provider image ID. When the artwork
   pair is present, independently fetch:

   ```http
   GET /api/v1/collections/COLLECTION/assets/ASSET/thumbnail?revision=ASSET_REVISION
   Authorization: Bearer ENROLLED_CREDENTIAL
   ```

   The response is PNG bytes. In Rust, use
   `client.thumbnail_at(collection_id, asset_id, Some(artwork_revision))`.
   The asset revision is numeric; `row_revision` and `catalog_revision` are opaque
   strings with different purposes. Do not send either as the asset revision.
4. Display each successful thumbnail as it arrives. One slow/failed cover must not
   hold back successful covers or titles. Retain successful downloads across later
   failures and cancellation; bound requests, fence superseded callbacks and expose
   incomplete artwork loading separately. Retry missing covers on explicit refresh.
   A title-refresh error after useful pages must not suppress artwork for already
   committed, authorized rows.

For example, an album row with `artwork: "ASSET"` and `artwork_revision: 3`
uses `/api/v1/collections/COLLECTION/assets/ASSET/thumbnail?revision=3`, even
when the album Item revision differs. Cache the bytes by server/account,
collection, asset ID and asset revision. Keep album-to-cover associations separate
from the byte cache so a changed selection does not reuse an old cover. Decode
image contents rather than requiring a filename extension: Trash Amp's cached
thumbnail filenames are extensionless. Retained offline copies use the last
successfully published metadata/artwork; they do not prove current server access.

## Pending, absent and failed artwork

| Observation | Consumer action |
| --- | --- |
| `summary_pending: true` | Show the safe title row and retry title pages with bounded backoff. Pending rows intentionally omit stale artwork; this is not proof of permanent absence. Ready rows can load their covers independently. |
| Ready row with null artwork pair | Show the missing-cover placeholder. Inspect Collector's metadata/artwork workspace if a cover was expected; a player read must not initiate provider work. |
| Thumbnail 200 | Retain bounded bytes and publish that cover immediately. The shared Rust client limits a thumbnail to 2 MiB. |
| Thumbnail 304 after `If-None-Match` | Reuse existing bytes only when that validator and cache identity match. The Rust thumbnail helper returns bytes and does not itself send conditional headers. |
| Thumbnail 409 | Asset revision changed. Requery the album descriptor; do not store returned/error bytes under the old revision. |
| 401/403 | Report authentication/access failure. Cache presence grants no server authorization; do not retry indefinitely. |
| 404 | The asset/thumbnail is absent or archived, or the collection association is wrong. Requery the descriptor and leave a visible missing-cover state. |
| Timeout, transport or server failure | Keep titles and successful covers; show the failed artwork stage and allow retry. Do not turn failure into a cached permanent absence. |

Search hits and playlist entries can carry an artwork Asset ID without its
revision. Prefer the corresponding current album descriptor; otherwise the
authorized `GET /api/v1/collections/{collection}/assets/{asset}` metadata read
returns the asset revision and thumbnail availability. Do not enumerate album
tracks or call the provider preview endpoint to resolve a cover.

`/items/{id}/metadata/artwork/.../thumbnail` is an ephemeral curation preview of
a provider candidate, not the accepted album thumbnail contract. Accepted covers
are Assets and use the collection-scoped route above. Manual artwork authority
survives provider refreshes. Pages and backing APIs follow configured listener
exposure; a working web login does not grant a separate native account access.

## Diagnosing a player-only failure

For the **same account**, inspect one album row and its exact authenticated
thumbnail request. Record status, asset/revision, pending state and whether cached
bytes decode; never record credentials. If Collector web shows a cover and that
request succeeds, inspect the player's cache association, incremental publication
and UI invalidation. If the pair is absent, compare pending state, selected
account/collection and permissions before investigating provider acquisition.

The [generated OpenAPI](../api/openapi.v1.json) is the exact wire schema;
[API maintenance](../api/README.md) describes generation and command semantics.
