# Home-lab scope and meaningful provenance

Accepted user direction, 2026-10-09. This revises scope, prioritization and future
contract/retention expectations. It does not claim implementation or authorize
pruning current user data. [The consolidated backlog](../remaining-work.md) is
the only remaining-work queue; HL-01 owns the concrete simplification policy.

## Useful, clean software before speculative contracts

Junk Collector primarily serves a home lab. Web and Trash Amp are the current
client targets. **Phono Junk is no longer a target.** Retire its outstanding PJ1
and Phono-specific native GUI/physical-device acceptance without marking them
passed. Keep reusable Collector, account, transport and shared-library behavior;
removing a target does not require deleting working integration code or data.

Earlier proposed contracts may be invalid, overengineered or aspirational. They
are inputs to reassessment, not obligations to implement. Before accepting one,
identify its actual consumer, complete user outcome, ownership, minimal durable
state and a real failure/recovery case. Revise or retire speculative interfaces,
schema reservations and pointless compatibility promises. Retain clean domain/
application/adapter boundaries and concrete preservation/recovery guarantees.
The pre-release policy remains applicable to actual stored data and consumers.

## Collection provenance versus operational history

Preserve originals and meaningful collection facts: physical inventory and manual
metadata authority; accepted metadata/artwork with sufficient source attribution;
substantive integrity/structure/catalog/audio evidence bound to actual source and
reference inputs, algorithms and coverage. Unknown collection data must still
round-trip. These are useful independent of the latest provider or client.

A lookup is not a collection event. “The user looked up album art on X, Y and Z
days” is not a provenance requirement. Do not automatically preserve every fetch,
identical provider response, tentative candidate version, access timestamp or
repeated result occurrence forever. Keep bounded cache/diagnostic state where it
helps retries or troubleshooting. Keep changed/accepted sources and meaningful
verification history when needed to explain current or historical claims. Raw
provider bytes supporting a retained claim may be needed; raw responses from every
unsuccessful or equivalent lookup are not intrinsically permanent collection data.

Command receipts and unfinished jobs need durable ownership/recovery while they
can still be replayed or resumed. That does not require immortal operational logs.
Retention windows and compact current state should follow real retry/recovery
semantics, not a wishful stability contract. User inventory changes and meaningful
verification results must not be conflated with disposable lookup diagnostics.

Existing append-only revisions, provider observations and occurrence records are
implemented facts, not a perpetual requirement for new data. Define dependency/
reference rules, dry-run reporting, backup and interruption-safe conversion before
compaction. Do not silently discard user-authored facts, evidence needed for retained
claims, unknown records or originals. This decision narrows the previous blanket
“preserve provider observations/history” policy for future design; safe treatment
of existing records remains part of HL-01, not a deletion performed by this edit.

## Security proportional to a public home server

Authentication, authorization, trusted HTTPS, recovery, private/public composition,
credential protection, bounded untrusted input and resource abuse controls remain
necessary. Promote concrete vulnerabilities and low-effort useful hardening.
Advanced automated trust rotation, discovery/failover, distributed limiting,
exhaustive platform certification and enterprise-sized load tests stay documented
but deferred unless actual deployment needs justify them. Ordinary TLS renewal
and explicit recovery instructions remain part of public deployment acceptance.

Backups must work, advertised evidence must be honest, and actual supported
clients must be exercised. A separate native inventory application, Phono delivery,
legacy importer for users without legacy data, full deployment matrix and indefinite
provider lookup history do not block a useful home-lab release.
