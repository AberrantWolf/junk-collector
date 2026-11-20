# Home-lab scope

Status: proposed, not yet reviewed by the owner.

Owner direction for what Junk Collector is and isn't trying to be.

## Useful software before speculative contracts

Junk Collector is a home-lab tool. The web interface and Trash Amp are the client
targets; Phono Junk is not (existing shared code and data that it used stay).

Old proposed contracts may be overengineered or aspirational. Before building one,
identify a real consumer, the complete user outcome, the owner, minimal durable
state and a real failure/recovery case. Retire speculative interfaces, schema
reservations and pointless compatibility promises, while keeping clean layer
boundaries and concrete preservation and recovery guarantees.

## Provenance, not operational history

Keep originals and meaningful collection facts: physical inventory, manual
metadata, accepted metadata and artwork with their sources, and substantive
integrity/structure/catalog/audio evidence. A lookup is not a collection event;
"the user looked up album art on three different days" is not provenance. Command
receipts and unfinished jobs are kept as long as they can be replayed or resumed,
not forever. See [0008](0008-meaningful-provenance-and-bounded-lookup-history.md).

## Security proportional to a public home server

Authentication, authorization, trusted HTTPS, account recovery, public/private
separation, credential protection, bounded untrusted input and abuse limits are
required. Fix concrete vulnerabilities and take cheap useful hardening. Automated
trust rotation, discovery/failover, distributed rate limiting, exhaustive platform
certification and large load tests are deferred unless a real deployment needs
them. For public access, a short check of sign-in, playback and API protection at
the real HTTPS hostname is enough; don't repeat every exposure combination
through the proxy.

## Local recovery, optional media backups

A multi-terabyte library doesn't imply automatic full backups, a cloud account or
a separate backup destination. Provide a predictable local backup area and a
practical rollback for updates: a stopped-server Btrfs snapshot where possible, or
an explicit full backup. Extra copies of media are the operator's choice. Local
recovery points don't protect against losing their own disk.

The full backup includes originals; there is no database-only backup. A future
lighter mode would need the archive's inventory, manual metadata, evidence and
assets plus the private state and config, not just the SQLite files.

## Ordinary music CDs

Keep the complete raw capture and logs and accept documented Q-subchannel errors;
see the [CD capture tradeoff](cd-audio-capture-tradeoff.md).
