# Implementation status

This repository is an executable architecture/server foundation, not the complete first release described in the accepted plan. Capability reporting must stay honest as remaining modules land.

Use the [handoff backlog](handoff-backlog.md) for ordered, scoped implementation tickets and the [release requirements](release-requirements.md) for the repository-local target specification. The gap list below is a summary, not a substitute for ticket acceptance criteria.

## Implemented

- Six-crate Rust workspace with a typed collection graph, compiled-module registry, application authorization boundary and a schema-2 third-module contract test.
- Foundation types now include collection-owned Work/Edition records, exclusive Item identity links, ordered specimen Components with optional data-bearing Carriers, collection-level Capture associations, typed asset subjects, language/currency/coordinate validation, canonical digests, and graph/representation validation.
- Archive schema 2 and a migration-only durable checkpoint preserve schema-1 records unchanged while emitting typed Work, Edition, Item, and Capture revisions, deterministic Components, explicit legacy payload locators, and rebuildable Works/Editions/Captures/Components projections. A committed synthetic baseline fixture covers two revisions, duplicate copies, two discs, legacy case/booklet carriers, files/assets, evidence, manual metadata, and lexically ordered unknown JSON.
- Lossless raw-JSON envelopes preserve unavailable module/provider bodies. Compiled modules have namespaced capabilities, exact schema validation, checked adjacent migrations with explicit source-preserving transformation policy, host-owned representation/job ports, and safe presentation contributions. HTTP item reads add post-redaction presentation; unavailable data displays generically and is not executable.
- Private SQLite advances through explicit migrations and stores a durable default UserId selected once from legacy first-row behavior. First-account creation sets it transactionally; later accounts do not change it.
- The S01 threat/recovery contract and D01 pinned reuse/fixture map are complete design gates. Their S02-S04 and D02-D05 runtime work remains explicitly outstanding.
- Separate private operational database; collection roles, private field filtering, passwords, revocable devices and local password recovery.
- Separate public/private HTTP routers and direct LAN TLS. Public curation/admin independently configured; disabled APIs and management bundles are absent.
- WebAuthn passkey registration/login backend and browser integration when an HTTPS authentication origin is configured. Registration and passwordless login pass a Chromium virtual-authenticator end-to-end test; real password-manager/hardware acceptance remains a release gate.
- Web collection browsing/search with artwork alongside game/CD rows, explicit placeholders and identity states, inventory creation/editing, wishlist/owned-copy entry, original cover retention and bounded thumbnails.
- Resumable package upload, destination verification, stable retry identity, original downloads with byte ranges, integrity rechecks and persisted job records.
- Client trust construction, route selection/hysteresis and cache eviction policy. A TLS integration test proves authenticated ranged downloads over loopback LAN without any public route, and rejects an unrelated certificate.
- Configured server inbox inventory/import API using the same source-retaining staging and evidence path; background upload errors persist as failed jobs.
- Optional private TLS management listener bound to an explicit interface address.
- WebUI onboarding distinguishes client-only and private management surfaces, offers first-collection/first-item next steps, preserves structured API error codes, and separates wrong-origin, expired-session and role-denial guidance without weakening listener or authorization policy.

## Required before the planned first release

1. Implement the accepted S01 security contract: recovery codes, recent-authentication gates, passkey revocation/counter CAS, last-admin/owner protection, paired native enrollment, authenticated certificate rotation, and attack tests.
2. Complete user/profile/grant administration UI, physical component/condition/acquisition/loan editing, sensitive attachments, job management and durable restart scheduling. Complete the configured server inbox selection/review UI.
3. Implement analyzer/provider adapters using extracted Retro Junk/Phono Junk logic, DAT/catalog ingestion, MusicBrainz/CAA enrichment and independent real-dump verification oracles. No catalog verification is currently claimed.
4. Add source-bound FLAC and configurable Opus derivative services and validated PCM transforms. Browser preview currently only plays already-playable original formats.
5. Implement one-way Retro Junk archive migration, complete export/backup/recovery commands, recovery UI and scalable incremental reindexing.
6. Deliver an actual Slint macOS app with shared playback, browsing, artwork, pairing, download persistence and personal activity sync. Extract/share audio with Trash Amp and add its server provider. Current client crate is not a desktop application.
7. Add authenticated endpoint discovery, mDNS, automatic health checks and actual transfer failover. Extend the tested manual LAN trust path to automatic discovery and full native-client failover.
8. Generate full OpenAPI/TypeScript contracts, validate body schemas/limits throughout, add event delivery/polling and comprehensive UI/integration tests.
9. Execute the 100k-item/1m-record benchmark on a recorded 4-core/16GB SSD host; validate native/container/tunnel/NAS deployments and load behavior.

## Follow-on releases

SwiftUI iOS through UniFFI; PriceCharting collection exchange; runtime plugins; replication; video/emulation; ripping hardware. These remain explicitly deferred by the accepted plan.

## Local onboarding

`setup` creates or repairs configuration with a hidden, confirmed terminal password prompt, preserves existing accounts/data/TLS identities, and prints the exact private localhost URL and first collection steps. `doctor` checks configuration without opening the archive. The WebUI distinguishes browser-origin failures, expired/revoked sessions and genuine role denials, and client-only addresses point back to the printed private management address. Login checks that the session cookie was retained. Single-user login resolves the durable default UserId; account switching/reassignment UI remains U02. Local HTTP onboarding is Chromium-tested; remote and other unqualified browsers may require trusted HTTPS. This is CLI guidance, not yet a comprehensive interactive network/proxy wizard.

## Wave 1 review fixes

Capture revision/ownership regressions, incomplete migration recovery, graph identity/ancestry gaps and capability dispatch are corrected with retained regression tests. Capture file/locator changes cannot bypass import services; legacy payloads remain reachable after reassociation. Reindex hydrates canonical captures and clears removed component projections. Schema transformations retain exact source envelopes, and language tags use a syntax parser. General P01 transaction/evidence recovery and the security runtime work listed above are still outstanding.
