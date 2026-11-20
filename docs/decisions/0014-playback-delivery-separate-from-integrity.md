# 0014: Playback is separate from integrity verification

Status: proposed, not yet reviewed by the owner.

## Context

Verifying a whole file before each playback read makes playback cost proportional
to file size. A recorded digest is not a new integrity check.

## Decision

- Playback serves the requested byte range without hashing the whole file first.
  HEAD and availability reads don't hash payloads either.
- Delivery still checks range, identity, authorization and read length/EOF.
- Import validation, explicit integrity checks and completed-download validation
  keep their own verification.
- Clients surface stream failures and apply their own recovery; a failed stream is
  never reported as a passed check.
- This applies equally to the main HTTP server, private media, direct media and
  native offline playback.

## Consequences

Same-length corruption may go undetected until an explicit check runs.

See: [HTTP media](../../crates/collector-server/src/lib.rs),
[media server](../../crates/collector-media/src/lib.rs).
