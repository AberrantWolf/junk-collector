# Background catalog contention — 2026-11-03

Issue #11 is valid on `37030e2`. Matching is already driven by durable server Jobs,
not browser polling. A 200-ms read-lock timeout can arise during a worker's current
input/checkpoint reads and previously entered generic `fail_job`. This repair yields
only typed `CatalogBusy`; provider refusal, invalid input and storage failures retain
their existing outcomes. Previously failed production Jobs are not rewritten.

The application chooses deferral; Store atomically verifies claim and lease,
releases the claim and queues the same Job without incrementing attempts or changing
request/frozen inputs/progress/creation time. Concurrent cancellation settles as
cancelled. A committed publication retains `revision_committed`, which is runnable
and recovered through the existing committed path. Stale workers cannot defer or
complete replacement claims. The worker returns its idle outcome to take the normal
250-ms backoff. Child ownership and checkpoint rows are retained.

Lock review: original validation/staging, PCM decoding, provider I/O and matching
run outside the catalog mutex. Publication holds the separate writer mutex across
filesystem work and takes the catalog mutex for its SQLite projection transaction;
removing that transaction boundary would compromise atomicity. The preceding
`2d18c6f` already moved multi-MiB checkpoint/frozen-input parsing outside the catalog
mutex and stopped idle recovery/polling writer reservations. This pass additionally
moves retained candidate JSON decoding (up to 4097 records) and cached track JSON
encoding/decoding outside the catalog mutex. SQLite query/projection work remains
serialized; this is not a measured production latency bound for arbitrary storage.
No full payload hashing was introduced in polling, admission or browsing.

Review fixes: preserve committed phase on re-claim, settle concurrent cancellation,
reject expired/stale claims, and retain the immutable first timestamp. Focused tests
pass for those transitions and a real service/shared-provider run that returns
CatalogBusy once and completes without frontend action or manual retry.

Broader validation: architecture/link checks, formatting, web type/build/model
checks, tooling/container/release-policy/mail checks, workspace tests (628 passed,
five existing ignores) and strict workspace Clippy pass. All 30 browser journeys
pass using the rebuilt debug server/media artifacts. The first two integration
attempts exposed old copy/warning-class assertions from #8/#10; their corrections
and rationale are recorded in the Metadata delivery. The final broad run passed
integration and every remaining journey. The earlier selector report retains its
failed-browser outcome; `/tmp/jc-issue11-browser.log` records the successful rerun.
An advancing-clock expired-claim regression and focused Clippy were also rerun
after review. No production state, deployment or release artifacts were changed.
