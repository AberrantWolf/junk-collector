# First-release requirements

Pre-release evolution follows [the accepted compatibility and contract policy](decisions/pre-release-evolution.md). Earlier checkpoint compatibility is not a release requirement; data preservation and recovery remain required.

The user revised the target on 2026-10-09 toward a clean, useful home-lab tool,
with proportional public-Internet security, reassessed contracts and meaningful
provenance. Phono Junk is no longer a target. This explicit direction supersedes
universal release gates and history-retention assumptions in the earlier plan from
`98e9187`. [Implementation status](implementation-status.md) describes what exists;
[remaining work](remaining-work.md) is the single remaining-work queue.
See [the accepted scope policy](decisions/home-lab-scope.md).

## Current home-lab release gate

- Preserve raw originals, physical collection facts, manual authority, unknown data
  and meaningful source-bound evidence. Keep accepted metadata/artwork attribution;
  do not require a permanent history of every lookup or equivalent provider response.
  Simplification/compaction must preserve actual data and recovery through an
  explicit tested policy, not silent deletion. The [bounded retention audit](retention-audit.md)
  records current owned policies and the limits of implemented response reuse/conversion.
- Deliver actual web and Trash Amp collection/listening workflows. Complete relevant
  LAN/native qualification and meaningful format/audio oracles for advertised
  support. Phono-specific acceptance is retired. Missing capabilities stay labeled.
- Provide an executable stopped-server backup/restore workflow and qualify the
  intended Linux binary/container and storage layout. A tested manual runbook is
  existing evidence; dedicated backup tooling is still open.
- For supported public access, qualify trusted HTTPS, listener/page/API exposure,
  roles, Origin/session controls, recovery/revocation, credential redaction and
  bounded untrusted work on the intended ingress. Keep focused security review;
  automatic rotation/discovery, exhaustive commercial matrices and distributed
  limiting are not default prerequisites.
- Test real user journeys, reasonable collection scale on the intended host,
  keyboard/mobile use and affected repository/sibling gates. State supported
  platforms, evidence limits and any upgrade/storage promise explicitly.

A separate Collector Slint app, full offline/activity parity across clients,
100k-item load gate, general Retro Junk migration, delta export and advanced
security certification are deferred/conditional outcomes in the backlog. Earlier
proposed contracts must be reviewed for validity and excessive complexity before
implementation. No older ticket is a mandatory stability/retention promise merely
because it was written down.

## Broader product target reference

The sections below retain the earlier product goals, detailed safety constraints
and dated user requirements. Their feature lists and broad acceptance matrix are
reference scope, not a second execution queue or universal home-lab release gate.
Current scope and retention policy take precedence. Completed requirements remain
useful regression criteria; deferred capabilities require fresh bounded design
before implementation. No unrun check is marked passed by this change.

## Automatic standard-game identification

The accepted 2026-10-10 delivery requires upload → review → activate → automatic
existing-game identification and future guided imports without technical fields.
The [supported detection matrix](game-identification.md) is the contract: finite
standard systems, supported No-Intro/Redump conventions and existing qualified
byte interpretations. No suffix/name/size-based identity or permissive fallback
from recognized malformed/unsupported input is permitted. Headerless bytes need a
complete unambiguous catalog match; CRC-only evidence is explicitly weaker.

Preserve failed inputs, originals, explicit authority, sources and unknown records.
Automatic/Manual controls must apply detected fields immediately to detail, browse
and search, while deliberate edits survive refresh. Qualify bounded DAT/ZIP batches,
multiple active systems, ambiguity/partial coverage, restart/page recovery, shared
owners, catalog replacement and source/permission fences through the actual worker
and UI. [Dated delivery evidence](validation/game-identification-2026-10-10.md) does
not imply universal format understanding or new format-algorithm qualification.

## Collection lists and playlists

Lists belong to one collection and are portable with its archive. Music list units
are individual audio tracks; album/disc additions append their supported tracks in
numeric order. Preserve intentional repeats, editable order, Play and Shuffle play.
Other collection lists use entries; future playable modules require an actual typed
target resolver. Shared collection roles govern read/play and curation writes.
Unavailable members and concurrent changes must be visible, never silently removed
or overwritten. List queries/admission must not hash or read full media payloads.

## Earlier local-alpha delivery

Accepted 2026-09-16: prioritize ingestion of raw dumps, automatic metadata,
artwork/text browsing, supported CD playback and multiple trusted local users in
the existing web UI. The [local-alpha plan](local-alpha-plan.md) records the earlier delivery;
current ordering is in the consolidated backlog. Automatic enrichment is default for its new guided import path;
playback derivatives remain optional and explicitly requested. Local-only limits
deployment scope, not authorization or transport protections. A tested stopped-server
backup/restore procedure is an alpha gate; complete portable export, native/offline
clients, public deployment and scale qualification remain full-release work.
LA1 guided ingestion and automatic enrichment is reviewed and corrected locally;
LA2 album preparation/listening is reviewed and corrected locally (2026-09-17),
while LA3 installation qualification is next. The alpha is a smaller delivery milestone,
not evidence that the requirements below are complete. Games initially use DAT
metadata; general online game artwork remains an explicit follow-up. Actual
supported dump formats require recorded qualification.

## Product and scope

Junk Collector is a self-hosted preservation and collection system, initially for physical video games and music CDs. It keeps original data, evidence, descriptive metadata and physical inventory, including inventory without files and unidentified captures. Price is secondary. Browsing, search and offline collections must show cover/album art beside text rows for games and CDs, with useful edition/platform/artist information, accessible labels, foreign-language text and consistent placeholders. Missing artwork and uncertain identity are different states.

The first release targets a Linux server and a Slint macOS client. The binary embeds a React/TypeScript web interface for curation and administration. Trash Amp remains a lightweight standalone music player, sharing reusable services and connecting through a collector-provider adapter. SwiftUI iOS through UniFFI and PriceCharting collection exchange are follow-on work. PriceCharting API availability and exchange permissions have not been established here.

## Architectural invariants

Shared libraries own parsing, hashing, analysis, decoding, DSP and optional device adapters. Collection foundation owns typed durable identities, inventory, assets, captures, representations and evidence. Domain modules own game/music schemas, identification, verification and presentation contributions. Application services own authorization, commands, queries, imports, resolution and jobs. Infrastructure owns files, providers, tools, archive transactions and SQLite. HTTP transports services; clients own connections, caches and personal activity; presentations call services.

Domain logic must not depend on HTTP, SQLite, a GUI or an application directory layout. Do not duplicate application policy in frontends. When a capability has no natural architectural home, settle that boundary before implementing it.

Distinguish works/editions, physical specimens, components/carriers, capture events, representations and supporting assets. Ownership, possession, wishlists, loans, acquisition and disposal are independent. A sale or loan never implicitly deletes archived bytes. Durable IDs do not depend on paths, servers or database rows. Hashes, money, dates, languages, coordinates and outcomes use explicit validated types where applicable; arbitrary provider payloads remain structured data, not a substitute for domain types.

Compiled modules have versioned schemas, migrations, capability handlers and presentation contributions. Host services mediate I/O and jobs, leaving a future isolation boundary. Preserve unknown module data during export/import/reindex and provide a generic client display. Runtime plugins are deferred.

Keep junk-libs as independently consumable workspace crates; no umbrella dependency or repository split is needed. Format and device backends are optional. Extract algorithms/audio in separate migrations with sibling-project checks. Reuse Retro Junk processing and Phono Junk primitives, not their scanning, persistence or application flow.

## Preservation and import

Readable directories contain versioned TOML manifests, original files and append-only JSON evidence. Stable IDs survive renames. SQLite catalog projections are rebuildable; accounts, credentials, grants, devices, durable jobs and personal activity are private operational state. SQLite stays on local storage even if archive bytes live on a NAS.

One archive adapter owns a single writer, staging, durable recovery records, publication and atomic projection updates. Expected revisions reject conflicts. Resolve interrupted transactions before accepting writes. Archive-only recovery restores history but requires explicit access reassignment. Full backup includes private state. Retain revisions, tombstones and exportable changes for future replication. Servers initially operate independently and never share an active archive writer.

Configured inboxes and resumable browser uploads inventory complete packages, validate paths, stage and hash, analyze identity candidates, allow review of provisional matches/physical associations, verify destination bytes, publish, then run independent verification/enrichment/derivative jobs. Preserve sources. Bound work, support cancellation and idempotent retries. Missing optional providers/tools cannot prevent basic preservation.

Preserve raw ISOs, cartridge images, redumper outputs and BIN/CUE packages. Integrity, structure, game DAT matches, AccurateRip, reconstruction and derivative validation are distinct evidence types. Record exact inputs, tool versions, catalog snapshots, outcomes and limitations. Changed inputs make old evidence historical. AccurateRip is the initial CD verification method; MusicBrainz identity never proves audio correctness. Qualify each analyzer operation with fixtures and independent reference results.

Select useful tentative metadata when ambiguous, retain alternatives and original provider responses, visibly mark uncertainty, and preserve manual overrides across refreshes/outages. Strong identity evidence outranks language; the persisted default metadata language chooses album variants, with English as the initial account preference, while original text remains retained.

## Playback and offline use

Expose media through authorized capability readers, not paths. Integer sample positions, seeking, queues and DSP belong below UI layers. Server-side playback derivative generation/storage is optional and initially explicit, not an automatic consequence of import. FLAC is the default format when a derivative is requested; offer configurable Opus offline profiles; record source/conversion settings and validate lossless output against source PCM. Originals remain intact.

Cache by server and immutable representation/profile identity. Downloads resume, include thumbnails/metadata/evidence summaries, support pinned albums, budgets and eviction. Favorites and playback activity synchronize after reconnection; collection edits require connectivity. Revocation prevents future access but cannot recall disconnected copies. There is no offline DRM requirement.

## Networking and security

Client, curation and administration are distinct pages and backing APIs. Public client access is enabled; public curation and administration default off. Enabling public administration also exposes curation; enabling curation alone does not expose administration. Roles remain enforced regardless of exposure.

Public tunnels/proxies always reach a public listener even when their upstream is localhost. Do not infer private status from source IP or forwarded headers. Private management binds explicit LAN/tailnet interfaces or a dedicated private proxy listener. Tailnet ingress is private but does not bypass authorization. Cloudflare Tunnel maps only to the public listener; tailnet-only Tailscale Serve may map to private management.

Native clients trust a server-managed local TLS identity provisioned through authenticated pairing or explicit fingerprint/QR enrollment. Verify the handshake and server identity before credentials or endpoint merging. No general invalid-certificate bypass. Learn .local, LAN IP, tailnet and public routes; prefer healthy direct LAN, then reachability/latency with hysteresis. Reject unexpected keys and support authenticated rotation. After pairing, encrypted LAN playback/downloads must work without Internet access. mDNS and Tailscale naming are distinct mechanisms; do not assume .local provides tailnet discovery.

Browser trust is separate: use a browser-trusted hostname/local DNS, trusted local CA or Tailscale HTTPS. Native pairing does not install browser trust. Tailscale HTTPS uses its full tailnet hostname, not arbitrary .local names. Current Chromium localhost onboarding is a convenience, not proof of other browsers or remote HTTPS readiness.

Use Argon2id passwords, standard password-manager-compatible WebAuthn, a stable HTTPS authentication origin, browser-mediated client enrollment, revocable device credentials, recovery codes and local administrator recovery. Single-user mode uses the default profile and hides user selection without making access anonymous. Passkeys remain bound to the authentication origin; enrolled device credentials support alternate routes.

Use versioned JSON APIs, generated OpenAPI/TypeScript contracts, cursor pagination, structured errors, optimistic revisions, idempotent commands, bounded resumable chunks, durable jobs, HTTP events with polling fallback and ranged media. Enforce authorization, private-field filtering, CSRF protection, secure sessions, rate limits, credential redaction and explicit proxy rules.

Authenticated clients obtain bounded collection-type descriptors and server-computed content availability. Item creation inherits the collection owner; generic forms may edit only module-declared scalar fields through owner-validated patches, while structured and unknown data remains preserved. Missing modules and legacy-unassigned collections are visibly read-only without disabling independently authorized access management, personal activity, artwork or media reads. Type labels and safe summaries are data-driven rather than a fixed games/music client enum.

## Release acceptance

- User clarification (2026-09-28): importing raw dumps defaults a newly created
  entry to one owned physical copy. Manual entry can remain without an owned copy
  and can be wishlist-only. Music imports should expose proper track names when a
  matching provider release has them, with an obvious metadata lookup for older
  albums. Reimport describes replacement of original files; gathering names does
  not require it.
- User clarification (2026-09-28): multi-disc albums need a collector-facing
  per-disc checklist: distinguish each attached original, expose recorded versus
  release-listed disc IDs and missing media, make uncertain disc assignments
  explicit, and show how to complete structure and audio checks. Detailed
  processing history remains accessible but must not dominate the album view.

- User clarification (2026-09-21): provide manual item removal with clear retained
  history/originals semantics and restoration, plus clean reimport of an existing
  item without duplicate visible entries or obsolete work becoming current again.
  Allow selected cleanup of failed/cancelled and unfinished work, with durable
  cancellation and actual staging reclamation where applicable. Preserve manual
  information by default and make explicit reset choices reviewable.
- User clarification (2026-09-21): prevent equivalent active scans server-side as
  well as in the UI, show completed results as the primary action, and consolidate
  equivalent results with lightweight actual-recheck dates/counts. Equivalence
  includes source, algorithm/version, settings, reference content and coverage;
  identical outcome labels alone do not suffice. Cached reuse is not a fresh
  verification. [The proposed UX fix set](ux-fix-plan.md) maps these requirements
  and related unfinished U05/P04 criteria to implementation deliveries.

- User clarification (2026-09-15): explicitly prepare an entire album as separate
  FLAC tracks, including selected associated discs, using durable batch submission
  and existing per-track jobs. Independent work remains queueable while earlier
  jobs run throughout the application; only real dependencies/conflicts or explicit
  admission limits block it. Verify partial outcomes, duplicate prevention, retry,
  cancellation and restart. [D07.4–D07.5 and U05](background-work-plan.md) own this
  web completion requirement; bounded execution is compatible with queued work.

- User clarification (2026-09-15): every interface needs useful valid defaults,
  eligible choices, up-front prerequisite guidance and clear outcomes/recovery.
  Audit all implemented screens/actions and resolve findings before claiming web
  UI completion, not only user-reported examples. Apply the same acceptance
  standard to future interfaces. [U05](interface-readiness-plan.md) owns this gate;
  analysis must choose valid CUE/TOC anchors and identify missing package members
  before execution while preserving basic import without optional analysis.

- User clarification (2026-09-15): web curation is incomplete without identifiable
  import and per-attempt history, actionable outcomes, reopening older imports,
  and cleanup of erroneous imports. Discard unpublished staging with explicit
  byte-reclamation results; dismiss/restore terminal history and correct mistaken
  published associations while preserving originals/evidence. Acceptance belongs
  to [P04.4–P04.6](import-management-plan.md).

- Recover interrupted publication and catalog rebuild without losing originals, manual overrides, loans or evidence; retain unknown module data and directory-independent IDs.
- Exercise duplicate specimens, multi-disc editions, unidentified captures, wishlist-only inventory, lending and disposal.
- Validate normalized hashes, track structure, AccurateRip offsets and FLAC PCM against independent vectors and real-dump oracles.
- Demonstrate tentative metadata, durable manual corrections and provider-outage behavior.
- Verify artwork beside game/CD rows in browsing, search and offline views; placeholders, lazy loading and accessibility work.
- Exercise all public exposure combinations, pages and APIs, role denials, malicious paths, upload resumption, credential recovery/revocation and spoofed proxy headers.
- Pair once, disconnect the Internet, and demonstrate native LAN playback/downloads. Exercise spoofed discovery, certificate changes/rotation and actual LAN/public transfer failover.
- Benchmark 100,000 items and one million file/evidence records with five concurrent users on a recorded four-core/16-GB/local-SSD host. Warm paginated browse/search target: below 500 ms p95.
- Validate the native Linux binary and minimal non-root container with explicit archive/state/scratch mounts, public tunnels, private tailnet management, NAS payloads/local SQLite and missing optional tools.

Replication, cross-collection byte deduplication, runtime plugins, video, emulators and ripping hardware are outside the initial release. Preserve architectural room for them without implementing them as hidden prerequisites.
