# Implementation status

This repository is an executable architecture/server foundation, not the complete first release described in the accepted plan. Capability reporting must stay honest as remaining modules land.

## Implemented

- Six-crate Rust workspace with typed collection model, module registry, application authorization boundary and a third-module contract test.
- Portable immutable item revisions with TOML manifests, typed JSON records, preserved unknown module data, SHA-256 metadata validation and rebuildable SQLite projection.
- Separate private operational database; collection roles, private field filtering, passwords, revocable devices and local password recovery.
- Separate public/private HTTP routers and direct LAN TLS. Public curation/admin independently configured; disabled APIs and management bundles are absent.
- WebAuthn passkey registration/login backend and browser integration when an HTTPS authentication origin is configured. Registration and passwordless login pass a Chromium virtual-authenticator end-to-end test; real password-manager/hardware acceptance remains a release gate.
- Web collection browsing/search with artwork alongside game/CD rows, explicit placeholders and identity states, inventory creation/editing, wishlist/owned-copy entry, original cover retention and bounded thumbnails.
- Resumable package upload, destination verification, stable retry identity, original downloads with byte ranges, integrity rechecks and persisted job records.
- Client trust construction, route selection/hysteresis and cache eviction policy. A TLS integration test proves authenticated ranged downloads over loopback LAN without any public route, and rejects an unrelated certificate.
- Configured server inbox inventory/import API using the same source-retaining staging and evidence path; background upload errors persist as failed jobs.
- Optional private TLS management listener bound to an explicit interface address.

## Required before the planned first release

1. Harden/review operational security: tested enrollment/recovery codes, recent-authentication gates, passkey revocation and counter races, authenticated certificate rotation, full threat-model review.
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

`setup` creates or repairs configuration with a hidden, confirmed terminal password prompt, preserves existing accounts/data/TLS identities, and prints the private localhost URL and first collection steps. `doctor` checks configuration without opening the archive. Browser origin failures have a distinct actionable error; login checks that the session cookie was retained. Single-user login uses the initial account rather than assuming its name is owner. Local HTTP onboarding is Chromium-tested; remote browsers still require trusted HTTPS. This is CLI guidance, not yet a comprehensive interactive network/proxy wizard.
