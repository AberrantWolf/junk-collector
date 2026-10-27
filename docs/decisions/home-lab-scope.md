# Home-lab scope and meaningful provenance

Accepted user direction, 2026-10-09. This revises scope, prioritization and future
contract/retention expectations. It does not claim implementation or authorize
pruning current user data. [The consolidated backlog](../remaining-work.md) is
the only remaining-work queue; HL-01 owns the concrete simplification policy.

The [2026-10-17 bounded audit](../retention-audit.md) now records concrete
consumer/reference rules and the delivered CAA response/Asset simplification.
General history/log deletion remains conditional, not an implemented capability.

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

## Music-CD capture tradeoff — clarified 2026-10-27

For ordinary music CDs, retain complete captured raw packages and logs while
accepting documented Q-subchannel imperfections. Q-only refinement remains
optional; use AccurateRip to assess audio within its reported coverage, without
claiming complete subchannel preservation. Persistent Q counts alone do not
create mandatory recovery work. See the
[accepted decision and optional operator command](cd-audio-capture-tradeoff.md).

## Security proportional to a public home server

Authentication, authorization, trusted HTTPS, recovery, private/public composition,
credential protection, bounded untrusted input and resource abuse controls remain
necessary. Promote concrete vulnerabilities and low-effort useful hardening.
Advanced automated trust rotation, discovery/failover, distributed limiting,
exhaustive platform certification and enterprise-sized load tests stay documented
but deferred unless actual deployment needs justify them. Ordinary TLS renewal
and explicit recovery instructions remain part of public deployment acceptance.

Public-ingress qualification is proportional to that scope (clarified 2026-10-21).
Use the existing intended HTTPS hostname; an additional isolated tunnel is not a
prerequisite. Reuse backend/LAN tests and operator-observed working sign-in/playback.
Add a short real-origin check for trusted TLS, the normal browser workflow and
protected APIs, with playback/seek where that has not already been observed.
Do not repeat every exposure combination, recovery scenario or abuse test through
Cloudflare, or change production configuration just to exercise alternatives.
Investigate concrete failures; broader load/failure matrices remain follow-ups,
not a home-lab release certification exercise. Unrun cases remain unqualified.

Backups must work, advertised evidence must be honest, and actual supported
clients must be exercised. A separate native inventory application, Phono delivery,
legacy importer for users without legacy data, full deployment matrix and indefinite
provider lookup history do not block a useful home-lab release.

## Local recovery and optional media backups — clarified 2026-10-20

The user clarified that a multi-terabyte media library should not imply automated
full-library backups, a paid cloud account or a separately selected independent
backup destination. None is a release or ordinary-update prerequisite. Provide a
predictable local backup area and practical consistent update rollback. The packaged
container already initializes `${COLLECTOR_DATA}/backups`; the CLI still requires an
explicit fresh destination and does not run a scheduled backup automatically.

Keep complete local recovery points for updates, using a consistent stopped-server
Btrfs snapshot where eligible or an explicitly chosen full backup. Independent media
copies remain optional operator policy using existing storage/tools. Local points
support rollback; they do not protect against loss of their storage device. Preserve
actual recovery and integrity guarantees without mandating duplication of unchanged
masters before every image update.

The implemented full-backup command includes originals; there is no database-only
or lightweight catalog backup mode. Such a future mode would need the archive-owned
inventory/manual metadata/evidence and relevant assets plus complete required private
state/config, not just a copy of SQLite files. Do not label a rebuildable index or
private database alone as a restorable catalog.

A backup-age reminder is a reasonable optional follow-up, not implemented or a
release gate. Base any future reminder on known successful, validated recovery
points; an attempted/failed backup must not reset it, and externally managed backups
must not be presented as observed. This clarification changes operating scope, not
stored data, the accepted full-backup format or dated qualification results.
