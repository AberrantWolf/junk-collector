# Native failure/recovery qualification — 2026-10-21

Post-alpha.3 source work, using an isolated Trash Amp checkout. Native milestone
`0493b59` on `codex/game-disc-audio` repairs pending receipt input binding and adds
actual-TLS committed-response/restart qualification. The operator's original
Trash Amp checkout and staged native release changes are untouched.

The actual Collector fixture imports and prepares audio, enrolls a native client
with explicit fixture certificate trust, then truncates only the response JSON
**after** the real create/append effect commits. Real server reads confirm that
commit; the native adapter retains its pending command. Changed title, server
identity or expected revision cannot replace those saved inputs. Reopening the
profile and retrying its original receipt returns the same list/revision and
retains one playlist and the same single entry identity. No server effect or
successful outcome is mocked, and retry does not invent a new receipt.

Self-review repaired the generic command upsert, replacing it with atomic
compare/admit under the existing profile database mutex. The complete request is
persisted before delivery; a confirmed response clears its pending state. Existing
Lists retry/discard controls use this path; discarding a local request does not
undo a server effect that may already have committed. No schema migration, UI
rewrite or indefinite duplicate command history is introduced.

All eight affected adapter tests passed (four unit and four actual-TLS journeys),
including existing CD, purchase and game-audio streaming/download/offline checks.
One existing physical-output-device test remains ignored. Strict all-target Clippy
and formatting passed. Published Collector `59faf341a0141c0424231d1700cfc16a006146a7`
and shared `993afc932c6b2acb5cf02c95d3cb8a387d9a70a5` pins are unchanged; no local
patches or private media are delivered. The single lockfile addition exposes the
already-resolved Axum dependency to the fault test.

This qualifies adapter/transport/profile recovery. System-trusted second-device
enrollment, physical output-device loss, Internet-disconnected LAN operation and
broader stalled/malformed media-read cases retain their separate HL-04 limits.
No native package, Docker image or production update is published by this work.

## Actual uncached-read revocation

Native `8afb19d` enrolls a distinct non-administrator Viewer and reads a complete
256 KiB original-media network chunk through the native playback RangeReader.
The test then removes that collection grant or revokes that native credential
through real application policy before the next uncached byte. Both reads fail
with PermissionDenied. Worker-owned readers/runtimes and bounded coordination
prevent teardown hangs. Nothing mocks a successful read or authorization result.
All nine adapter tests and strict Clippy/formatting pass; the physical-output test
remains opt-in. This qualifies future reads, not recall of buffered bytes or
retained offline copies, and does not establish output-device loss handling.

## Actual range faults, cancellation and pinned-trust recovery

Native `339a871` adds a bounded real-server response fault adapter. After a normal
chunk, changed ETag, ignored Range/200 and a streamed body ending short all fail
without publishing a byte to the caller or advancing its position. Normal retry
and backward seek then succeed. Other cases hold real response headers or the
actual authorized body; cancellation returns Interrupted before the normal timeout.
The fixture is released before assertions and readers/runtimes stay on workers.
No stored master is altered and no successful source bytes are manufactured.

The two-server regression also sends a saved pinned connection to a different
TLS identity: refresh fails, the last complete catalog is retained exactly, and
restoring the original route/trust permits a new successful refresh. This tests
owner-provided certificate trust, not system-trusted second-device enrollment.
All eleven adapter tests pass (four unit/seven actual TLS), with one existing
physical-output callback opt-in/ignored, strict all-target Clippy and formatting.
No dependency versions or production adapters change in this qualification.

The finite transport cases above are accepted. Native UI playback/queue behavior
across every fault, actual output-device loss, system-trusted second-device
approval and actual Internet-disconnected LAN use remain separate. Reuse these
software checks instead of restarting an exhaustive matrix or duplicating them
through a public tunnel. Existing physical Safari and normal native playback
acceptance remain valid within their recorded limits.
