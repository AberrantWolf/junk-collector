# API v1

`openapi.v1.json` is generated from the annotated route groups the server is built
from, and `web/src/generated/api.ts` is generated from that. Do not edit either by hand.

After changing an annotated route or DTO, run `python3 scripts/api_contract.py --write`.
`python3 scripts/api_contract.py` (or `npm --prefix web run check:api`) fails on
drift. The server does not serve the document or a Swagger UI.

Player clients: see [album metadata and artwork](../docs/album-metadata-consumers.md)
for the read sequence and artwork handling, and
[layered library refresh](../docs/layered-library-refresh.md) for pagination and
lazy track reads.

## Mutations

Application commands require an `Idempotency-Key` of 1–128 visible ASCII characters.
Authentication ceremonies, logout, upload chunks and one-time secret issuance use
their own state or offset semantics instead. Retrying with the same key and request
returns the original result; the same key with a different request returns
`idempotency_conflict`; a duplicate still in progress returns `command_in_progress`.

Mutable resources take an `expected_revision` (zero means create). A stale revision
returns `revision_conflict`.

AcoustID contributions bind the receipt to the item, lookup, release and reviewed
file assignments. The personal key is not stored. A replay returns the accepted
submission IDs, or `unknown` if the contribution may have reached AcoustID.

## Item saves

Clients send:

```json
{"command":{"collection_id":"…","title":"…"},"expected_revision":1}
```

The command holds the full editable field set. IDs, revision history, timestamps,
deletion state, metadata observations, captures, evidence and assets are server-owned.

The endpoint still accepts the older `{"item":{…},"expected_revision":1}` shape,
but rejects any change to server-owned fields; a new item in that shape must have
revision zero and default server-owned state. Clients should use the command envelope.

## Read retries

Reads can return `503` `catalog_busy` during brief publication contention; retry
with a bounded backoff. `unavailable` and storage errors are not transient. The web
client retries GET/HEAD up to three times.
