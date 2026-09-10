# API v1 contract and Wave 1 migration

`openapi.v1.json` is generated from the same annotated route groups used to
compose the server. `web/src/generated/api.ts` is generated from that document;
neither file is hand-edited.

Run `python3 scripts/api_contract.py --write` after changing an annotated route
or DTO. Run `python3 scripts/api_contract.py` (or `npm --prefix web run
check:api`) to regenerate into temporary files and fail on drift. The server does
not expose the document or a Swagger UI route.

## Mutation contract

Application commands require an `Idempotency-Key` containing 1–128 visible ASCII
characters. Authentication ceremonies, logout, resumable upload chunks, and
one-time secret issuance use their own state-machine or offset semantics and are
exceptions. A successful retry with the same typed request returns the original
result; changing the request while reusing the key returns
`idempotency_conflict`. An active duplicate returns `command_in_progress`.

Mutable resources carry an `expected_revision`; zero means create. A stale
revision returns `revision_conflict` without publishing a revision or consuming a
new command receipt.

## Item-save compatibility

New clients send the following envelope (command fields abbreviated here):

```json
{"command":{"collection_id":"…","title":"…"},"expected_revision":1}
```

The command schema contains the complete editable field set. IDs in the path and
command, revision history, timestamps, deletion state, metadata observations,
Capture representations/locators, Evidence, and Assets remain server-owned.

For Wave 1 clients, the endpoint temporarily also accepts
`{"item":{…},"expected_revision":1}`. Before delegation, the application service
loads the canonical item and rejects any difference in server-owned fields. A
new legacy item must have revision zero and empty/default server-owned state. The
adapter therefore cannot publish compatibility Capture or Asset snapshots or
overwrite an authoritative standalone Capture revision. Clients should migrate
to the command envelope; the compatibility shape is not the long-term
architecture.
