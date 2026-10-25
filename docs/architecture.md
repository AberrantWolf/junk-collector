# Architecture

## Evidence-aware music editions and equivalent matches — 2026-10-25

Application metadata policy ranks editions against current, committed CD-layout
observations for attached originals, without opening audio payloads. Compatible
disc IDs/positions precede language and arbitrary identity tie breakers; fewer
unrepresented media is a preference, not proof of an owned edition. Manual release
choices remain authoritative. Reads reassess immediately and periodic, paginated
maintenance publishes revised automatic choices for search/reindex. Publication
uses existing inventory/revision fences. Historical materialized displays retain
their recorded policy; new displays freeze the chosen automatic edition.

Equivalent matches have identical provider/record ID, metadata, artwork, ranking
and parsed observation, differing only in internal version. Consolidate their
active identities and review presentation, preferring referenced identities. Retain
immutable candidate/source revisions and references; this is non-destructive
logical cleanup, not archive compaction. New MusicBrainz versions hash candidate
content rather than unrelated bytes in the surrounding response.

Album playback/list expansion includes available unambiguous discs even when a
provider edition lists additional media. Missing media remains visible through
album coverage; source ambiguity, authorization and unavailable audio still apply.

## Album playback readiness — 2026-10-24

Presented music entries carry typed recorded album playback readiness. Application
policy reuses the album/list target resolver, with committed track observations
instead of opening PCM, then checks that each selected track has published audio in any supported format,
including Opus without FLAC. Recorded availability prefers retained FLAC recipe
coordinates and falls back to retained Opus recipes for tracks without FLAC.
Ambiguous/unassigned originals, incomplete structure and unprepared tracks
produce an actionable reason. Provider-listed absent media remains in album
coverage but does not block playback of available unambiguous discs. This is independent of catalog
verification, device codecs and transport availability. Resolve against authorized
unredacted records before producing the role-redacted response.

The album-attention browse filter uses the existing authorized, sorted keyset
query over music candidates, then evaluates this policy in the application. It
examines at most 200 candidates per request, filling the requested result limit
where possible and preserving the underlying scoped continuation cursor. Sparse
API pages may be empty with a continuation; the web follows those cursors
automatically, with cancellation on a scope change, until results or the end. There is no persistent readiness
projection to become stale after metadata/assignment/preparation updates or reindex.
Ordinary browse and detail show the warning without attempting playback. The
existing requester-private failed-work filter retains its separate meaning.

## Browser now-playing presentation

The web music player publishes title, album artist, album and authorized artwork
through the standard Media Session API. Album/list presentation supplies metadata
to the personal queue; the browser adapter owns OS metadata and controls, while
existing player actions retain playback routing and authorization. It updates on
track changes and clears metadata/handlers when playback is released or its audio
element detaches. Unsupported browser actions do not prevent ordinary playback.

## Direct HTTPS media with Cloudflare control — source delivery, 2026-10-23

The separate `collector-media` executable serves scoped audio HEAD/GET, server
identity and narrow CORS preflights. Its router never composes the general client
or management router. Login, enrollment and grant issuance remain on the normal
control origin. Private playback remains first choice; the configured direct
HTTPS media endpoint is the remote choice, with Cloudflare music fallback refused.
See the [implemented contract and plan](direct-https-media-plan.md),
[operator setup](../deploy/container/media.md) and [dated acceptance](validation/direct-https-media-2026-10-23.md).
HL-12 retains source/client qualification; HL-05 retains live deployment acceptance.

Application services own playable-target eligibility, bounded hashed grants/read
handles and live session/device/account/collection authorization. The storage
adapter resolves an authorized immutable relative file locator; media never parses
archive manifests or opens the account database. `collector-media` depends only
on shared DTO/core value types and maintained HTTP/file libraries, with no
app/store/providers dependency. Both HTTP adapters reuse its bounded connection
mechanism. Audio/format algorithms remain in the existing shared libraries.

The supplied Compose profile gives media a read-only archive/rootfs, no core
state/config/TLS keys, no capabilities and `network_mode: none`. Named volumes
share two filesystem Unix sockets: frontend-to-media HTTP and media-to-core's
restricted authorize/recheck/release/identity adapter. Core still writes through
its own archive view. Media uses the existing numeric UID to read private archive
permissions; mount/network/PID/capability boundaries provide isolation. A Linux
startup guard refuses writable archive or nested mounts. The archive layout is
not an audio-only confidentiality boundary: compromised media can disclose mounted
content. Containers share the host kernel. Caddy has separate certificate state
and ordinary networking; restricting frontend egress/LAN access requires target
host firewall/DMZ policy and is not implied by media's network isolation.

A single directly hosted origin can proxy a fixed media prefix to the same separate
process, stripping cookies on that upstream. It retains filesystem isolation but
shares browser-origin trust. Separate hostnames supply the chosen Cloudflare/direct
routing and browser-origin separation; DNS cannot split traffic by URL path.
External reachability requires an outside observer. The authenticated test page
checks a fresh one-use challenge and bounded early/late reads from a queued track;
its cellular declaration is user-supplied, not network attestation. Actual iPhone
Safari, public ACME/renewal, router and NAS acceptance remain separate from local
source tests. No live infrastructure is changed by source implementation.

## Existing private playback and dated LAN design

The accepted home-listening **stopgap** uses a dedicated manual LAN HTTPS hostname,
`collector-lan.awkwardexplorer.com`, resolving to the reserved NAS address even
through public DNS. It serves the same public/client listener and account store;
the exact browser origin is allowlisted without changing the configured passkey
authentication origin. It introduces no application transport or helper subsystem.
See the [current LAN delivery](lan-playback-first.md#current-delivery-explicit-lan-hostname--2026-10-22)
for operator-confirmed work-Mac browser and Trash Amp playback/seek with
GlobalProtect connected. Unattended certificate renewal remains outstanding.

The real solution remains automatic detection of and connection to a reachable,
trusted local Collector in **both the browser and Trash Amp**, with authorized
remote access when LAN is unavailable. It is unimplemented and tracked in HL-12;
the manual hostname does not complete it. Browser HTTPS/origin/authentication and
local-network permission handling require a bounded design/feasibility step.
Native transport selection follows the Iroh trial independently; native-only
success cannot stand in for browser automatic selection. Preserve account/content
identity and validate server/endpoint authority before sending credentials. This
tracking clarification selects no browser transport mechanism or new subsystem.

The expanded [route and browser design](https-relay-and-lan-switching.md#route-policy-trust-reachability-and-transfer-cost)
first filters authorized, media-eligible endpoints, then prefers direct LAN
reachability; the unresolved Cloudflare media route is not an automatic fallback.
Endpoint/path cost remains unknown when unobservable. There is no Tailscale
integration or provider-specific discovery; already configured private HTTPS
routes use the normal OS stack. Application services own endpoint authority,
media eligibility and scoped read-grant policy; HTTP/browser adapters own probes,
TLS/CORS/CSP, media-element and permission mechanisms. Browser feasibility retains
the normal app/login origin and tests encrypted direct local media with scoped
authorization. The earlier ordinary-HTTPS media-lease baseline is a control for
the private transport alternatives below. It does not widen login cookies or trust
unauthenticated discovery.
These are planned boundaries, not implemented automatic playback or budget controls.

The user clarified that private playback should approach install/sign-in-or-pair/use
without manual internal certificate, DNS or router setup. Public server operators
retain responsibility for their chosen HTTPS/tunnel/ingress; Collector must not
require a particular provider, vendor certificate service or relay product.
Evaluate automatically retained native trust, browser WebTransport certificate
hashes and authenticated WebRTC data channels alongside operator-provided HTTPS.
Browser pinning here is specific to WebTransport; it does not make ordinary
fetch/audio requests accept a self-signed certificate. Peer/signaling authorization,
HTTP/range adaptation, certificate lifecycle and browser playback remain application
and adapter work, not solved by an encrypted echo test. Loopback-only experiments
belong under `scripts/experiments/direct-transports`, outside production crates.

The next bounded LAN experiment keeps a separate disposable Collector account,
archive and listeners on the NAS. An experiment-only browser adapter bridges the
unchanged `MusicPlayer` media requests through a service worker and page-owned
WebTransport session. The HTTP/3 sidecar forwards only bounded media reads to that
Collector's public router, retaining its session and collection authorization on
every read. The test HTTPS origin uses the already installed operator certificate;
the distinct WebTransport endpoint uses a generated short-lived pinned identity.
This tests playback mechanics and authorization across devices, not automatic
discovery, production endpoint enrollment or a new production subsystem.

The public-origin follow-up mounts this same disposable fixture only beneath an
exact test path at the existing public hostname. Test assets/control/cookies and
the service-worker registration stay path-scoped; the worker controls test-page
clients, never production pages. Its separate CSP names the pinned LAN endpoint.
The LAN handshake and ephemeral media grant are bound to the initiating exact
origin. Public tunnel routing is limited to that test path and loopback-only
fixture HTTP listener; neither production APIs nor private management are proxied
by the fixture. Verify the page uses the public route: split DNS resolving the
hostname to the NAS cannot qualify public-to-private browser permission behavior.

## Planned HTTPS relay and native route selection — 2026-10-22

### Automatic private playback — 2026-10-22 source delivery

The user now authorizes implementing automatic local playback for web and Trash
Amp together, with remote/Cloudflare media fallback disabled. The normal saved
connection remains the authenticated control plane. A typed authenticated local
descriptor advertises bounded generic OS interface candidates and software-managed
short-lived certificate trust. It does not inspect any overlay provider or infer
authorization from addresses. The server adapter owns interface enumeration,
P-256 identity rotation, HTTPS/WebTransport listeners and bounded range framing.
The 2026-10-23 address-selection repair stays in that adapter: preserve the
kernel-selected private source address first, then ordinary private interfaces,
known container bridge/veth interfaces and finally loopback. The route lookup
uses an untransmitted UDP connection, not an Internet reachability probe. Names
only affect priority, never trust or eligibility; no overlay-provider API is
consulted. Deduplicate addresses before the eight-candidate limit; explicit binds
remain authoritative. The same selection feeds certificate SANs, descriptors and
periodic rotation for both web and native clients. Application services continue
to own session-bound, origin/representation-scoped browser
grants and live role/revocation checks. The new listeners expose media and server
identity only, never management. Public HTTP media eligibility is an explicit
listener capability; a rejected local route cannot retry media on the control
origin. Ordinary library/login traffic retains that origin.

Web uses a same-origin service worker controlled by a document-owned pinned
WebTransport broker, with permission remediation and bounded candidate probes.
Native retains its existing ranged reader and decoders, obtaining per-route HTTPS
trust from the authenticated descriptor and verifying ServerId before sending its
credential. Endpoint selection does not change account, saved connection, queue
or cache identity. Reuse the existing `collector-client` across Trash Amp rather
than duplicating a transport/resume implementation. Generic external mechanisms
come from established transport libraries; a new shared networking crate is not
needed solely to wrap those dependencies. See the [implemented contract](automatic-private-playback.md) and
[dated acceptance](validation/automatic-private-playback-2026-10-22.md). Source
implementation and deployed-device qualification remain distinct.

**Earlier design record, superseded for private playback by the implementation above:**
the requested [relay and LAN-switching plan](https-relay-and-lan-switching.md)
is design only. The later [LAN-first plan](lan-playback-first.md) selects local
DNS for the existing HTTPS origin and a trusted home proxy to `public_bind`,
enabling browser and system-HTTPS Trash Amp home listening before the Iroh trial.
It changes network destination without changing browser origin or enrollment;
it is not active-stream failover. Subsequent authorized setup installed and assigned
a production certificate to the NAS public-listener proxy, reserved the NAS address
and enabled the scoped home DNS override. Direct local HTTPS passes normal CA,
hostname and ServerId checks; public AAAA/HTTPS/SVCB answers are suppressed locally.
Unattended renewal and acceptance of this earlier same-origin route remain pending; see
[partial setup](validation/lan-https-2026-10-22.md).
The required work-machine VPN may override DNS or deny direct LAN access; both
must be tested independently before this topology is qualified for that client.
A normal remote HTTPS frontend forwards through an authenticated outbound
home connector to the existing public/client listener; it must never expose the
private management listener. Browsers keep same-origin HTTPS. Native transport
selection follows a bounded Iroh integration trial before implementing a general
HTTPS route/resume manager. Reusable trust/stream/range mechanisms belong in a narrow
planned `junk-libs-net` infrastructure crate; Collector retains enrollment,
endpoint authority, account/collection policy and receipts, and Trash Amp retains
Keychain, playback and presentation. Per-endpoint trust must precede credential
use, and resumed reads must retain immutable byte identity. No relay deployment,
shared-crate extraction, automatic discovery or live failover is implemented by
this plan. HL-12 tracks the requested delivery without replacing the active queue
assignment or adding a universal release gate.

Catalog startup replaces obsolete projection databases before replaying archive records.
It also detects beta.1's incorrectly stamped music-search table. Old catalog files
are retained for diagnosis; private state and archive records are separate.
This rebuild does not hash media payloads. See [beta correction](validation/container-beta2-publication-2026-10-21.md).

Container packaging (2026-10-19) lives in `deploy/container` and the Dockerfile,
outside domain/application services. A root one-shot initializer prepares only
the selected mount roots and initial config; the server/CLI run as UID 10001.
Linux host networking preserves loopback HTTP and explicit private TLS listener
composition. Deployment configuration/credentials persist independently of image
replacement. The process adapter accepts both SIGINT and SIGTERM through the
existing stop path; this is not a new active-job drain guarantee.
The fresh-install proxy-config transport (HL-05A, 2026-10-20) refuses existing
archive/private state and empty input before atomic replacement. Setup/doctor own
configuration validation and provider-contact readiness. The optional inbox Compose
override supplies the same default read-only bind and supplemental group to server and CLI;
filesystem ACLs remain a host mechanism. Actual Synology update/resume/restore and
remote-browser checks are recorded in [NAS operations acceptance](validation/nas-operations-2026-10-20.md).
HL-05B uses an operator stopped-server Btrfs point over those roots; standard
GNU tar exports to other storage are optional. The deployment adapter binds the tar working
cutoff to snapshot time; application backup/storage formats and domain services are
unchanged. See [qualified recovery and limits](validation/snapshot-recovery-2026-10-20.md).
The [2026-10-20 backup-scope clarification](decisions/home-lab-scope.md#local-recovery-and-optional-media-backups--clarified-2026-10-20)
keeps local rollback practical and external media copies optional. `${COLLECTOR_DATA}/backups`
is the packaged local area; full backup remains explicit and includes media. No
scheduler or lightweight catalog-only mode is implemented. The post-alpha.3
optional backup-age reminder below reads bounded completion receipts.

Build-once publication (2026-10-22, after beta.4; native validation and reuse qualified): validation builds
one labelled OCI export per native architecture, imports that exact runtime image
for smoke acceptance, merges/verifies exports and retains immutable GitHub Actions
artifacts. Publication is a separate consumer: a successful same-repository,
exact-source workflow run and artifact digest bind the validated image bytes,
version, platforms and smoke/check outcomes. It performs no source compilation
or image rebuild. Missing/expired/mismatched evidence fails closed and requires
explicit fresh validation. `scripts/container_validation.py` owns CI artifact/OCI
mechanisms; existing release policy, anonymous verification, channel safeguards
and bundle assembly stay in `scripts/container_release.py`. These are deployment
adapters, with no application/domain changes. The completed beta.4 tag and image
are immutable. A nonpublishing reuse rehearsal qualifies this boundary without
publishing another release or changing `preview`; see [dated acceptance](validation/container-promotion-2026-10-22.md).

Release planning/OCI verification/bundle assembly (HL-07D, 2026-10-20)
live in `scripts/container_release.py` and the existing workflow, outside application
and domain services. Public version verification precedes `preview`/stable `latest`
promotion; bundles pin the verified manifest digest. Local OCI verification is
separate from actual registry/pull deployment acceptance. Public alpha.2 and its
isolated Portainer/NAS/browser recovery acceptance are now [qualified](validation/container-publication-2026-10-20.md).
Release builds now use native AMD64/ARM64 runners with architecture-scoped caches
and real native container smoke checks, then assemble their digest-addressed outputs.
The source revision is frozen before checks/builds. A manual retry may select an
existing published release tag; it fetches that release and checks out its exact
`refs/tags/…` source. Empty-input manual runs remain nonpublishing OCI exports.
The exported native indexes are merged with content digest checks; public version
verification still precedes channel promotion and installation asset publication.
The disposable `scripts/portainer_qualification.py` lifecycle adapter reuses the
container recovery journey without application changes. It renders the shipped
Compose service, initializes a separate loopback Portainer database using a temporary
password file, pulls through its Docker gateway and checks asynchronous deployment
results. Service-targeted updates retain an unchanged networkless companion; a
Collector-only stack separately exercises forced re-pull/redeploy. Only generated
fixture resources are cleaned up; the operator Portainer is not accessed.

NAS preparation-failure qualification (HL-05C) reuses the same generated Compose
journey. A fixture-only encoder wrapper controls real subprocess blocking or
failure after output; Chrome uses ordinary cancellation/retry controls. No production
API or conversion policy is added. Application Job ownership/claim fencing and
adapter process termination/publication remain responsible for actual recovery;
see [actual-host acceptance](validation/nas-preparation-recovery-2026-10-20.md).

Archive permission qualification uses the same generated installation and a separate
`scripts/storage_permission_qualification.py` adapter. It saves exact archive modes,
removes write permissions only within that fixture, exercises an actual browser
metadata save/read/playback, restores the modes and retries. Application metadata
publication and storage recovery own the outcome; no HTTP result or successful
publication is mocked. This is permission loss, not disk-full or power-loss injection.

Bounded capacity qualification adds a dedicated external Docker volume backed by a
96 MiB tmpfs, mounted only as the generated fixture archive. A networkless sleeper
keeps that mount alive across CLI/server stops. The fixture verifies the filesystem
type and exact capacity before a bounded fill reaches real ENOSPC; it never fills
the host's Btrfs storage. Removing only the filler and performing normal startup
settles interrupted application journals before browser retry. Restore selects a
second empty private volume; cleanup removes both volume/keeper pairs. Shared
metadata assertions live in `scripts/storage_failure_qualification.py`; the capacity
and permission adapters supply real filesystem mechanisms without mocking results.

Bounded authentication qualification reuses that fresh Compose installation and
performs actual HTTP connections with Linux loopback source addresses. It isolates
account, direct-peer and global login thresholds by varying real peers/accounts;
forged forwarding headers cannot select a bucket. Normal fixture restarts reset
transient state between finite cases, and original-session continuity plus the
ordinary browser/backup/restore journey remain required. This exercises the TCP
adapter; no application outcome or peer identity is injected into a router.

Real-tool audio qualification (HL-06A) adds a separate fixture oracle adapter,
`scripts/audio_oracle_qualification.py`, to that generated Compose journey. It
compares actual FLAC output against independently generated PCM and actual Opus
output against CLI decoder/frame/channel oracles; Chrome prepares and plays the
three configured Opus profiles without FLAC masking. Private test handoffs publish
complete files atomically. Production conversion/domain responsibilities are
unchanged; see the [audio matrix](audio-preparation.md) and
[bounded evidence](validation/audio-oracles-2026-10-20.md).

**Current direction (2026-10-09):** [remaining work](remaining-work.md)
is the only remaining-work queue. Older assignment/order and gap statements below
are historical. Proposed contracts require reassessment under
[the home-lab scope/retention policy](decisions/home-lab-scope.md); their presence
does not require implementing speculative machinery or permanent lookup history.
Phono Junk is no longer a target. Implemented contracts describe current behavior,
not a claim that cleanup or redesign has already happened.

Pre-release evolution follows [the accepted compatibility and contract policy](decisions/pre-release-evolution.md). Earlier checkpoint compatibility is not a release requirement; data preservation and recovery remain required.

Junk Collector preserves evidence and owned collection history independently of how content is consumed. The server is a modular monolith: one process composes independent domain, application, storage, authentication, HTTP, and job capabilities. It is not a set of microservices.

## Digital purchases — HL-02, 2026-10-19

Purchased FLAC/MP3 packages use existing Music Items, collection-level Captures
and preservation Representations without physical Specimens or CD layouts.
`Capture.purchase` owns portable private acquisition facts; `StoredFile.visibility`
protects individual originals. Role is independent of codec/lossiness. Existing
stored files retain their old shared policy; old physical inventory is preserved.

Music owns fingerprint-bound immutable file-track manifests and descriptive tag
interpretation. Narrow junk-libs readers own bounded structural/tag/picture
parsing and native PCM decoding. Application services resolve direct originals,
metadata, artwork and album/list inclusion through existing intake, publication,
authorization and recovery boundaries. Adapters retain I/O. A nullable conversion
profile and native audio format distinguish originals from CD derivatives; media
and cache identities include the actual package file. Display order never changes
source coordinates. Ambiguous alternate encodings require an album choice.

Catalog queries use recorded observations, never full-media decoding/hashing.
Background analysis and artwork validate consumed source bytes with cancellation;
publication retains Item/import generation fences. Digital integrity, decode
validity and descriptive metadata never assert CD or catalog verification.
See [implementation and supported matrix](digital-purchases.md) and
[qualification evidence](validation/digital-purchases-2026-10-19.md). Further codecs
and image splitting remain outside this initial delivery. Actual Chrome and native
GUI streaming, seeking, downloads and retained offline use passed on the named
clients; broader LAN/device qualification remains HL-04.

## Stopped-server backup and recovery contract — HL-03

The operator CLI composes an offline recovery workflow. Application-owned backup
inventory validation defines the complete archive/private/config unit and full
versus archive-only recovery. The store adapter owns exclusive root locks, journal
settlement, SQLite checkpoints, bounded metadata reads, streaming copies/digests,
durability and restrictive permissions. No HTTP or background backup service is
introduced. `backup`, `validate-backup` and `restore` are local operator commands.

A backup includes all regular files and empty directories under both roots,
including unknown data, TLS and unfinished-work scratch, plus the configuration.
Symlinks/special files are refused. The digest inventory is sealed with a completion
marker written last. Validation never opens or migrates a database and rejects
missing, changed, duplicate, unsafe or unlisted files. Restore validates first,
locks available original roots and both empty destination roots, checks each copied
file again, then publishes the relocated configuration last. Interrupted restores
leave markers in both roots; normal Store startup refuses them before database
opening. Retry uses newly empty roots, never an overlay. Backups are immutable
operator inputs; failed backups have no completion marker and must be retried at
a fresh destination.

Full recovery preserves server identity, TLS, credentials, grants and pending work;
the original and restored instance must never run together. Remote/offline copies
cannot be fenced by local filesystem locks, so the operator must retire the source.
Archive-only recovery copies no private files or TLS and requires setup of a fresh
identity plus explicit access reassignment. External inboxes, DAT source files and
encoder executables are reported for operator relocation, not silently copied or
rewritten. Neither mode deletes or alters the backup or source records.

### Portable full archive export

HL-10's local `export-archive` command owns an offline Store and its two process
locks. It also holds the writer/publication gates while freezing and copying the
archive. Application-owned inventory policy includes every portable archive file
and empty directory, including unknown records, and excludes only the operational
writer lock and empty staging/journal directories. Unsettled scratch is refused,
never silently omitted. No private state, configuration, credentials, grants,
pending jobs or server identity is exported. Collection records can still contain
private collection facts; an export is not a redacted public download.

The filesystem adapter shares bounded tree traversal, private file creation,
streaming copy, hashing and sync mechanisms with backup/restore. A versioned
inventory records relative names, lengths and SHA-256 values without source paths;
its digest completion marker is written last after byte validation. Validation
opens no databases. `import-archive` validates first, copies only into an empty
archive root, checks copied bytes again and fences partial copies with the existing
restore marker. It creates no configuration or private state. Setup, reindex and
explicit first-Owner recovery use the existing local commands. Later source writes
do not change a completed export. This is a full local copy, not delta export,
online replication, legacy conversion or a replacement for full recovery backups.

HL-03 review repairs: `unowned-collections` provides bounded stopped-server
discovery and `recover-collection-owner --collection-id … --username …` assigns
the first Owner explicitly. Local filesystem authority plus both process locks
authorize this recovery; ordinary HTTP grants still require a collection Owner.
Application security policy refuses disabled targets and replacement of an existing
Owner (including a disabled Owner); repeating the same successful assignment is a
read-only success. The adapter evaluates that policy and applies the shared grant
effect plus a private audit entry in one transaction. No module availability or
content write eligibility is required to recover read access to historical data.

An original root may survive disaster recovery without its lockfile. Restore
creates/acquires the normal lock there when absent, or acquires the existing lock
read-only, before touching destination roots. Backup lockfiles remain required and
are never created or rewritten. A visible live original still refuses restoration;
missing original roots do not need to be recreated. New source lockfiles contain
no data and only fence local startup; source archive/private records are untouched.

## Standard game identification delivery — 2026-10-10

Shared junk-libs readers own content inspection and DAT decoding; Games owns a
finite standard-system registry and catalog interpretation. Application policy
owns multi-catalog identification and activation-triggered background work.
Filename extensions are routing hints, never identity evidence. A recognized
malformed/unsupported input cannot fall through to raw matching. Headerless
inputs need complete catalog evidence. Provisional labels are not manual authority.
Active DAT selection is scoped by system, family and byte domain; snapshot bytes
and accepted sources remain immutable. Durable jobs own cancellation/recovery;
client polling never hashes originals. The [supported matrix and workflow](game-identification.md)
record limits; [dated validation](validation/game-identification-2026-10-10.md) records acceptance.
Activation scans page durable member admissions and yield while children run.
Equivalent direct/import/scan requests share active work with durable ownership.
Per-field Automatic/Manual controls preserve explicit authority independently of
source-bound identification; effective system fields are module-owned metadata.

## Bulk library delivery contract

The accepted [bulk-library delivery](bulk-library-plan.md) extends intake,
metadata resolution, grouping and FLAC preparation through application-owned
services and dedicated task pages. Metadata language is private user preference;
accepted automatic resolution is portable and distinct from manual selection.
Computed disc IDs establish album matching; release/language variants of the same
album do not constitute ambiguity. Confirmed curated merges require a portable
group commit manifest and atomic graph projection; independent entity writes must
not emulate an atomic merge. Private batch selection/ownership and paginated
recovery reuse existing worker/publication policies. This section records the
implemented BL1–BL3 contract; local acceptance is recorded in
[bulk validation](validation/bulk-library.md).

Automatic multidisc grouping (2026-10-22 delivery): application policy assesses
accepted MusicBrainz album identities and recorded source layouts, never titles,
artwork or new payload hashing. All preserved disc sources must map uniquely to
consistent medium positions; duplicate positions across entries, incompatible
release media, manual/unknown-data conflicts and explicit separation remain review
cases. Album-level grouping retains distinct physical copies and every Capture,
Asset, evidence reference and source; it does not infer physical-set ownership.
The existing group publication/journal and graph projection perform the atomic
merge. Automatic results identify their basis and retain guarded undo; undo writes
a portable automatic-grouping opt-out so periodic recovery honors that decision.
A paginated adapter query over the accepted-album projection supplies authorized
candidate cohorts, including preexisting metadata on startup and periodic recovery.
Busy entries wait for their guided imports, item Jobs and accepted bulk members to
settle. The adapter rechecks this under the existing writer/publication gates;
Job admission/retry share that gate so grouping cannot invalidate admitted work.
No frontend orchestrates a merge and no new provider or payload path is introduced.

Ambiguity-aware grouping (2026-10-22 follow-up): application policy separates
album membership from the displayed metadata choice. A rebuildable many-to-many
album-option projection includes viable current MusicBrainz alternatives as well
as accepted/manual choices. Conditional grouping requires the same viable album
options and consistent distinct recorded disc positions under every option;
overlapping options alone do not establish membership. A uniquely compatible
accepted target can admit unresolved siblings without overriding another accepted
album choice. Multiple compatible established targets remain ambiguous.
Incomplete or contradictory layout coverage leaves its album option unproven;
it cannot silently eliminate that alternative. Membership and display share the
inventory-reset policy for current candidates, retaining cleared candidates only
as history unless deliberately selected again.

Build a bounded compatibility graph of eligible entries. Only entries with the
same closed compatibility neighborhood merge, with accepted subsets taking
precedence over unresolved attachments. This avoids arbitrary choices between
competing subsets; invalid/busy/explicitly separated entries do not veto an
independent safe subset. Equivalent manual candidate versions/releases are
compatible by album identity and placement; a stable explicit display choice is
retained and all original choices remain in group history. Unresolved alternatives
remain current after merging, rather than becoming an invented accepted identity.
Current alternatives are the intersection supported by every merged source;
historical/unmatched candidates remain selectable history. Temporary publication
guards fence every assessed neighbor's inventory/metadata as well as the affected
entries, so concurrent choices cannot invalidate a membership proof. The existing
atomic publisher, permission/busy fences and guarded undo remain the only writer.
Catalog projection evolution rebuilds the disposable index from preserved archive
records; private state, credentials, Jobs and original payloads are retained.

Grouped-disc publication revision recovery (2026-10-19): under the archive writer,
the adapter validates the frozen/current Item archive records and resolves added
Evidence references to committed sibling imports from the same user, intake and
destination. Application policy permits an evidence-only revision advance when
the reviewed inventory/metadata and existing references are unchanged. Manual
inventory/domain/metadata changes and unrelated evidence retain the conflict.
The publication path applies this rule on first execution and ordinary retry,
including old failed imports after restart; no data conversion, credential reset,
new Import identity or blanket expected-revision bypass is introduced.

HL-07A import review (2026-10-19): intake album group keys identify destinations
independently of optional manual title authority. Matching explicitly reviewed
copy labels within one album/intake share one deterministic physical copy; different
labels create distinct copies. Nullable titles retain existing recorded manual
titles; no stored inventory is regrouped. Shared module readiness replans selected
CUE/Redumper anchors without changing the reviewed inventory or destination.
Application-owned new-intake housekeeping policy excludes only `.DS_Store` files
and `#recycle`/`@eaDir` directory subtrees. Inbox discovery, browser discovery and
staging use that policy; adapters retain declared housekeeping members for recovery
of older accepted inventories. Unknown hidden files and saved archive records remain
unchanged. See [acceptance](validation/intake-review-2026-10-19.md).

## Collection lists

Collection-owned lists are portable revisioned archive entities, independent of
Item inventory, metadata and the personal browser playback queue. Each occurrence
has a stable ID; repeated items/tracks are intentional and independently movable.
Ordinary collections list Items. Music collections list source track coordinates
(Capture, preservation Representation and track position), never derivative URLs.
Application policy expands album/disc additions, uses numeric disc/track order,
and requires a source choice for alternate originals. Disc/album expansion checks
the complete recorded audio-track scope against availability; unavailable originals
cannot silently shrink an addition to its prepared subset. Individual available
tracks remain addable. Additions freeze inventory and Capture revisions for a
publication-time currency check. Current prepared
outputs and descriptive titles are resolved when viewing/playing; list browsing
never hashes payloads or reads audio bytes. Capture ownership follows a confirmed merge in the same
collection; removed or superseded sources remain visible as unavailable entries.

Browse roles read lists; Curator/Owner roles write collection-shared lists through
curation routes. The storage adapter rechecks grants and revisions under the archive
writer, publishes using the existing journal/rename commit, and atomically projects
the list and command receipt. Archive-only recovery rebuilds lists without grants.
Catalog v12/checkpoint format 5 include list projections. Lists have at most 5,000
occurrences; name length is 256 bytes, list/entry pages are at most 100 rows, and
entry cursors bind to the list revision. Conflict reload is explicit. Deletion is
an archive tombstone and never removes originals. Unknown collection modules can
still read lists; unsupported playback is labeled rather than invented. A future
video capability will need its own target resolver before becoming playable.

The addressable Lists page belongs beside Library. Album, disc and track controls
append to a list through one shared chooser using the common dialog and operation
scope helpers. Search keeps the selected destination visible and fences obsolete
pages. Unconfirmed create/append inputs and receipt keys survive reload and panel
reopen in a user/source-scoped browser session draft; retries resolve that original
command before another addition. Playlist Play/Shuffle play resolve all bounded
pages at one list revision before replacing the personal queue, and show
unavailable-track exclusions. Shuffle changes the playback order, not the saved
list. Reload restores a paused queue; the shell player persists across navigation.

HL-07B direct Play controls reuse the existing application-owned client playback
snapshot and fresh authorized availability. The browser chooses only an existing
decodable format, then atomically replaces its personal queue through the shared
start operation. No encoding or provider command is admitted by Play. A panel-owned
abort controller fences obsolete reads before queue mutation; the shell player
retains media permission checks and lifecycle ownership. See [acceptance](validation/direct-playback-2026-10-19.md).

## Local-alpha guided ingestion

New guided imports create one default owned, present physical copy on their new
Item. Attaching files to an existing Item preserves its inventory. Manual Item
creation records no copy unless the curator adds one. The guided copy has a
deterministic identity, so recovery cannot duplicate it; placeholder retirement
recognizes only its untouched default state as automatic inventory.

MusicBrainz release observations retain bounded medium-specific track names.
Music availability resolves the effective candidate under metadata authority and
uses a medium only when the release is selected or unambiguous, its recorded disc
ID matches (or no disc ID exists and there is exactly one medium), and its track
positions cover the source layout.
Manual music track titles take precedence. Older observations without track lists
remain valid but need a fresh provider acquisition to gain names. A metadata
refresh does not replace a Capture or assert audio verification.

Reassociation can retire an otherwise unused, automatically created intake Item.
Application policy distinguishes placeholders from independent inventory/manual
information. The committed Capture association and original guided intent form a
durable cleanup request; paginated reconciliation recovers an interruption after
that command's receipt. Retirement uses the existing Item tombstone, with a writer-
locked comparison of the hydrated Item and authority revision to fence independent
Capture, Asset and metadata changes. Archive history and payloads remain intact.
Cleanup waits for the original workflow to settle so it cannot race its own jobs.


Folder discovery is a read-only application command over validated logical names
and sizes. Each collection module proposes disjoint package memberships; the
application checks their indices and builds ordinary signed ingest plans. Native
filesystem traversal stays in adapters. The browser uploads every accepted package
through the shared resumable chunk transport; durable server intents own all
subsequent publication, identification, metadata and preparation. Name-level
redumper/CUE boundaries are provisional, not structural validation. Unassigned
files are listed explicitly and never silently uploaded.


The accepted [local-alpha plan](local-alpha-plan.md) schedules complete ingestion,
enrichment and listening journeys before native/public deployment work. LA1 is
implemented locally: application services coordinate existing Import, Job,
metadata, Asset and publication effects using a durable private intent and stable
output identities; modules own package interpretation, and adapters own I/O. No
browser-owned pipeline or generic workflow engine was added.
Automatic metadata protects manual authority; filename-derived defaults remain
provisional. Optional provider/preparation outcomes are independent of preservation.
Existing low-level/manual imports retain their behavior; only the new guided import
intent requests automatic enrichment. Optional preparation uses the LA2 durable album service.
Guided admission checks a selected existing Item's collection, module and revision
only on first command execution. The Store rechecks that revision in the same
transaction that commits the Import and intent. A completed receipt replays its
accepted result after later Item edits; a new request must use the current revision.
Module-owned readiness describes name-level anchor/companion constraints; staged
structural analysis validates actual contents. Failed analysis retains the package
for unidentified preservation. Worker reconciliation paginates all unsettled intents
on startup and every five seconds, after durable job outcomes are recorded.
Cancellation fences job admission in the same private-state transaction; discard
records its receipt and cleanup intent together, with actual cleanup progress read
separately. Settled and cancelled flags prevent repeated terminal effects.

## FX-02 scan lifecycle (implementation contract)

Application-owned scan plans normalize module-owned selections against recorded
package inventories before admission. Planning and admission use recorded digests;
actual bytes are checked with cancellation in the worker. Unknown/missing inputs
are blockers, never successful-looking empty scans. Structural name checks do not
claim structural or catalog verification. Immutable reference bytes and parsed observation, operation
version, source fingerprint and normalized settings define equivalence.

Private scan admissions are requester-scoped, matching the existing private Job
boundary. Different command receipts converge atomically on equivalent queued,
running or cancelling work. Completed work is visible as a result; a deliberate
new check gets a new execution/occurrence. Cancellation does not free an active
slot until settled, and generic retry must acquire the same slot. Durable direct
and parent request references are committed with admission; parent cancellation
must retain work still needed by a direct request. Cross-user private Job details
are never exposed by a plan. No new scheduler is introduced.

Portable completed observations retain a substantive result once. Subsequent
identical computations publish compact immutable occurrences referring to that
result, with their actual check time and per-occurrence snapshot/rebinding provenance.
The canonical result retains the common frozen inputs; occurrences do not copy those
inputs or output payloads. Result
identity includes complete output, coverage, coordinates, limitations and algorithm
inputs; timestamps/Job/receipt identities are excluded. Cache reuse creates no
occurrence and never advances verification time. Original old observation bytes
remain untouched; rebuildable grouping is conservative when old input provenance
is unavailable. Reindex reconstructs references and rejects missing, inconsistent
or cross-collection result links. Publication/recovery use the existing archive
writer and journals, not a separately committed UI workflow.

## FX-03 work/history contract

The application work service owns requester-private history presentation and action
eligibility. Store queries group import Jobs under their Import and preparation
members under their batch, with direct ownership retained as independent work.
Queries filter authorized collections before bounded keyset pagination; timestamps
and IDs provide stable ordering. Names come from current authorized catalog rows.
Dismissal is private presentation state, never deletion of results or payloads.

Cancel-and-clear records a durable private intent with its command receipt. Existing
cancellation/discard services settle that intent before history is dismissed; a
published Capture wins over discard. Shared direct requests prevent staging disposal.
The worker periodically reconciles pending intents across restart. Failures remain
visible and retryable; restored history never restarts execution. Bulk UI actions
submit an explicit reviewed target set with per-target receipts and outcomes, never
an unbounded “all matching” mutation. Original payloads and successful evidence
remain outside cleanup. Generic retry must reject pending clear/dismissed targets.

## FX-05 item lifecycle contract

An Item tombstone is an archive revision, not deletion of its Captures, Assets,
inventory, loans, metadata authority, or evidence. Application commands own
Remove and Restore and require collection content-write eligibility, an expected
Item revision, and an idempotency receipt. Ordinary browse excludes tombstones;
an explicitly selected removed view uses the same authorized, bounded query.
Direct Item reads retain role-based redaction so a curator can inspect and restore
one. Removed Item evidence, activity and owned artwork require the same curator
role, including their direct backing APIs. A removed Item is ineligible as a
destination for new work.

Removal must fence accepted guided imports and item-owned Jobs at the same
writer/publication boundary used by capture publication. Durable import intent
cancellation survives restart. Direct owners of shared scans remain attached;
removing an import owner cannot cancel their work. Workers must check the Item
state and workflow claim again before publishing, including after expensive
byte reads. A restart reconciler must repeat any cancellation left between the
archive tombstone and private-state settlement. The archive tombstone remains
the authoritative source when rebuilding projections.
If a Capture revision committed before removal, its Import receipt and Evidence
reference finish after recovery; this narrow finalization can only republish
the exact current tombstoned Item and cannot change its inventory.

Clean reimport is a new generation of work for the same Item identity. It keeps
the previous usable Capture until the replacement Capture publishes, validates
retained bytes while consuming them, then changes the current association
recoverably. Historical Captures, observations, and evidence remain addressable.
Manual metadata, inventory, loans and chosen artwork remain by default; explicit
reset flags govern authority changes. Generation is captured with admission and
checked at each child admission and final publication, so older work cannot
become current after a reset. A failed replacement remains resumable without
creating a second Item.

`Item.lifecycle` stores the generation, historical Capture IDs, and pending
replacement identity in portable Item revisions. Guided admission stamps Import
and Job generations in the same private transaction as their ownership records.
The archive preparation deliberately does not complete the command receipt: an
interrupted admission replays its deterministic Import identity, then commits
private admission and its receipt together. Generic retry and worker checkpoints
reject obsolete generations. Direct scan and derivative owners retain shared
computation; a committed Capture may still finish its evidence/receipt bookkeeping.
The final Item revision makes the replacement current and hides only the selected
old Captures, leaving other discs untouched. Reindex derives the same current view.

Reset choices are reviewed before admission and take effect when the reimport
starts. Portable metadata-revision cutoffs clear current automatic candidates and,
only when requested, manual field/domain/selection or artwork authority. Historical
observations and Asset bytes remain. Later manual edits are resolved against the
latest inventory revision rather than overwritten by replacement publication.
Old acquisition snapshots cannot be applied by a new completion after the generation
changes; receipt replay remains separate from applying an effect again. Provider
artwork also checks the expected generation under the archive writer before
publication. Superseded, unfinished replacement Capture IDs stay excluded even
when an earlier partial publication later completes its bookkeeping.

Retained-original staging is a claimed, cancellable Job. It copies bounded chunks,
checks recorded lengths and SHA-256 against consumed bytes, and checks cancellation
during the final staged-file hash too. A stored fingerprint alone is never treated
as a fresh integrity result. Failed uploads and staging jobs use the existing
resume/retry flow; retained staging does not offer a browser-upload resume form.
Player queues keep immutable file coordinates. Their lightweight authenticated
HEAD checks and media reads can resolve retained historical Captures, while
rechecking the current Item/Capture association and denying removed Items.

## FX-06 access and recovery contract

Collection access is a collection-scoped task independent of Item selection or
content-write eligibility. Application authorization checks ManageAccess before
bounded grant queries and exact grant reads; the Store pages username-ordered
results. Grant writes retain expected revisions, command receipts, recent
session authentication, and last-owner rules. Curation route composition controls
both the panel asset and its backing APIs. Loaded grant rows carry their query
scope; a collection/search change immediately makes those rows and their cursors
unavailable, and removal checks that the originating scope is still current.

FX-07 uses the same bounded read principle for administrator accounts. The
administration service checks the administrator role and page/search limits;
the Store returns username-ordered keyset pages and reads one account directly
for exact operations. Account rows and cursors are bound to the browser search
scope. Catalog snapshot rows are likewise bound to their selected collection;
a catalog Job completion refreshes only the collection that admitted it.
Authorized collection browsing uses a separate ID-keyset, name-search query
with optional minimum role and module filters. The Store applies grant and
search filters before its 50-row page; the application validates inputs and
computes presented permissions. An exact collection read checks the grant.
Navigation and management selectors share the paged client chooser and keep
selected records reachable outside the current page. The former unbounded
collection-list HTTP response is removed.
Work history checks an explicitly selected collection in application policy;
its paged Store query joins grants for unfiltered history instead of first
materializing every authorized collection ID.
Personal security histories use bounded creation-time/ID keyset pages. The
authentication and enrollment mechanisms retain their complete internal reads
where WebAuthn ceremonies need them; browser sessions, passkey views and native
device views each have a separate presentation query and no credential secret in
their page result. The security panel keeps a profile draft across its focused
tasks and scopes list reads to the panel lifetime.

Signed-out browser recovery consumes the existing one-use code through the auth
service and uses its returned session cookie. Codes and replacement passwords stay
in component memory and request bodies; neither is placed in a URL or browser
storage. The shared API client pauses a sensitive command on the typed recent-auth
error, obtains a fresh password or passkey session, then retries the original body
and idempotency key once. Cancellation rejects the pending command and leaves the
owning form mounted with its draft. An expired or revoked session still goes through
normal unauthenticated handling. Each prompt has a distinct generation and uses the shared UI operation lifetime.
The authentication coordinator shares a prompt while retaining each command's own
abort signal. Cancelling one waiter leaves others alive; cancelling the last waiter
aborts the prompt. A settled or cancelled prompt cannot settle its successor.
Commands snapshot their URL, body, headers, idempotency key and submitting account.
Before retry, the client verifies the authenticated account through `/me`. Every
account-bound API request also sends `X-Collector-Actor`: the common HTTP
authentication adapter compares it with the independently authenticated principal
and rejects a mismatch before dispatch. The header never supplies authority or
selects an account; it is an optional precondition for other clients. Ordinary
commands retry once; session-targeted revocation explicitly requires review of the
refreshed session list after authentication, with no automatic retargeting.

## Work identity and effect boundaries

[Work-context hardening](work-context-hardening.md) retains distinct Item, Import,
Job, attempt/claim, command receipt and workflow-generation identities. They are
not interchangeable. UI observation lifetimes are independent of durable execution:
closing a panel aborts its network reads and callbacks, not a server Job. Explicit
cancel commands continue to use durable request ownership.

`OperationScope` owns a lane's attempt and abort signal and guards success, failure
and settlement. A bound React scope is replaced on target/query change, including
A-to-B-to-A navigation; old closures cannot start work in the replacement scope.
Separate read and mutation lanes preserve independent operations. Loaded rows also
carry their scope, so stale rows never become actions against a new target. The
shared mechanism is used in grant management, destination search, guided import
selection/polling, metadata reads and authentication. Existing domain-specific
completion receipts and durable polling/resume identities remain authoritative.

Worker snapshot and processing publication receive the full `JobClaim`. The Store
validates recorded user, collection, item, attempt and claim token and checks
cancellation/generation itself during publication; caller checkpoints add policy
checks, never replace the Store's fence. Exact immutable snapshot replay can return
an existing record without writing, including after archive-only recovery.

Asset effects explicitly distinguish direct manual uploads, item-bound commands,
and derived publication. `ItemEffectContext` carries the exact item, generation
and source (guided import or acquired snapshot). Under the archive writer and
publication gate, the Store validates the target and source's durable association
and current generation before writing. Cancelled guided imports and missing or
foreign snapshot producers cannot publish via a matching item generation alone.
Historical observations and original bytes remain readable and preserved.

Generic retry, preparation retry and worker cancellation share one obsolete-job
predicate. It consults durable direct scan/derivative ownership rather than a cached
ownership boolean, and retains the narrow exception for completing bookkeeping
after irreversible Capture publication. Scan/derivative request admission and
last-owner cancellation keep their existing transactional ownership registries;
this delivery does not introduce a second scheduler or duplicate ownership store.

## Dependency rules

The Phono PJ1 delivery implements `collector-api-types` for list, music availability
and native browsing DTOs. Remaining DTO extraction is incremental. D04.2 introduced `collector-providers` at its reserved boundary. The
architecture checker enforces both dependency directions. Archive v4 processing
records and projections remain implemented through existing core, application,
and store boundaries.

- `collector-core`: portable typed identities, records and errors. No filesystem, database, networking, or presentation.
- `collector-api-types`: shared serializable client-facing read DTOs. It depends only on `collector-core` plus serialization/schema libraries and owns no services, authorization, storage, or transport. Callers migrate to the shared DTO owner together.
- `collector-modules`: module-owned schemas, source-preserving migrations, presentation values, and host-mediated operation contracts. Reader-only game normalization delegates to narrow `junk-libs-game-formats` primitives; the module owns operation policy and result validation. Music modules likewise map package identities into shared `junk-libs-disc` integer PCM readers; byte decoding and source concatenation remain shared mechanisms. No provider requests or application scheduling.
- `collector-app`: use cases, authorization, effective permissions, and explicit response DTOs over mechanism-only storage ports. Frontends do not bypass these rules.
- `collector-providers`: bounded HTTP, provider/catalog parsing adapters, recorded-response fixtures, and optional encoder-process mechanisms. It may depend on `collector-core`, `collector-api-types`, application port contracts, module-owned catalog DTOs, and narrow `junk-libs` crates; it never depends on Store, server, or a GUI.
- `collector-store`: archive publication, local catalog projection, private state, package/asset storage. It implements mechanism ports using validated identities/effects; application services own authorization policy.
- `collector-server`: HTTP composition, WebAuthn ceremonies, route exposure, worker limits, static UI delivery. Blocking storage/CPU work runs outside async workers.
- `collector-client`: endpoint trust, transport, offline cache policy. No server storage dependency.

Domain modules return observations, candidates, safe presentation values, and verification results. Durable hosts supply cancellation-aware readers and cooperative checkpoints for CPU-only loops; cancelled work cannot publish successful observations. They never decide to replace an archive, switch a view, scrape another provider, or construct a scheduler. The host owns those decisions and exposes only authorized opaque `Read + Seek + Send` package-file handles and bounded job submission. Package inventories are bound to a Collection and either an Import or Representation; file identities and validated package-relative logical names cross the boundary, while adapter filesystem paths do not. Relative member references resolve lexically within that inventory and cannot escape it. The compatibility single-file reader succeeds only for a one-file package and reports multi-file ambiguity explicitly. A test-only schema-2 stamp module proves registration, additive migration, multi-file byte access, job requests, and presentation do not require modifying a universal enum.

Processing inputs bind an operation/version to a deterministic `SourceFingerprint` over the typed source, every declared file in order, exact lengths and SHA-256 digests, optional layout digest, and sorted string parameters. Its versioned fixed binary encoding contains no filenames, filesystem paths, unordered maps, debug output, or floating-point values. Changing any package member changes the fingerprint. Durable processing Jobs persist validated frozen inputs, preallocated output identities and timestamps before dispatch. Import- and Representation-scoped readers expose the exact frozen inventory through opaque handles. Provider responses retain bounded raw bytes and lossless parsed observations for domain policy to interpret. The local DAT adapter reads administrator-configured source names or authorized opaque upload-spool handles, archives exact XML or ClrMamePro bytes, and rebuilds its typed row view from those bytes for each comparison. It checks the complete stream against the frozen snapshot length and SHA-256 before returning rows; malformed or truncated catalogs fail without dropping entries. The game module owns catalog byte-domain coverage: raw-2352 matching may exclude retained CUE metadata but rejects malformed or unknown payloads, and the matcher checks adapter coverage declarations. This is bounded parsing and linear matching, not a persistent indexed lookup or a scale-qualified catalog engine. Large catalog acquisition and snapshot access use bounded readers rather than buffered DTOs (512 MiB maximum); derived DAT indexes are capped at 64 MiB and small provider responses are capped at 8 MiB. Acquisition jobs freeze their query/output identity without inventing a file source. Potentially network-backed snapshot streams are bounded and spooled outside the archive writer lock; local spool bytes are checked against recorded length and SHA-256 before publication. Observation reads use scoped cursor pagination and curator authorization independent of module write eligibility, so missing modules do not make historical evidence unreadable. Source currency compares against server-owned committed file records, not caller-provided fingerprints; this does not perform live integrity verification. Stored operations and registered capabilities share one validated identity type, and dispatch uses registered capability ownership. Conversion recipes bind typed settings, source, ordered track ranges, encoder version reports and PCM formats to a checked canonical digest; optional junk-libs-audio derivative validation owns integer FLAC comparison and Ogg Opus inspection, while Collector owns subprocess lifecycle and cancellation/publication policy; metadata text overrides have one value per field and artwork has one authority. Unsupported layouts, missing dependencies, external qualification limits, and execution failures are distinct outcomes. CD sectors, PCM frames, and signed stereo-sample-frame offsets are distinct types.

Explicit music derivative requests use the existing command receipts and frozen
job executor. Each independently playable track has one sealed recipe and output
identity, determined by Capture, exact source/track range, preset and full encoder
report. The application maps music-owned PCM ranges and enforces source currency,
current authorization and cancellation. Store owns scratch handles and per-attempt
spools; subprocess encoding and shared validation run outside the archive writer
lock. The existing Capture revision journal treats output, immutable recipe and
validation Evidence as dependencies published before the Capture commit marker.
Cancellation wins until the final gated revision rename. Committed recovery uses
retained bytes/evidence without re-encoding or re-authorizing; current Capture
associations and Item edits survive. Explicit full reindex validates derivative
references and stored payload identity, without claiming a fresh codec or catalog
verification. Startup projects derivative metadata without reading their payloads;
each derivative GET checks the stored length and SHA-256 before transfer, while
HEAD remains a lightweight availability check. A failed byte check is reported
as prepared-media failure and does not imply that the preserved original changed.
Music layout selection for availability and admission uses the committed source
fingerprint to find observations; it is not a live integrity check. It must not
rehash preservation payloads during browsing or once per track during admission.
The conversion worker checks actual bytes before encoding and before publication.
The viewer-safe ordered availability contract carries source Representation,
track position and integer PCM coordinates, safe title, profile and only
preparing/unavailable/available state, plus a typed actor-specific preparation
permission. UI behavior never depends on human-readable reason text. Preparation
receipts bind the submitted request before resolving mutable tool configuration.
Published state adds immutable
Representation/package-file identity, length, media type and digest; archive
paths, raw evidence and private Job details remain server-side. The client router
serves this contract and existing opaque media readers with strong content ETags
and byte ranges. Only the curation router can enqueue preparation. Server-owned
optional encoder configuration supplies the probed full version report; browser
inputs cannot name tools or version claims. Published outputs remain readable
when the module or tool later disappears. Import and media reads never request
conversion automatically.

LA2 preparation keeps a frozen batch Job over existing derivative Jobs. Application
policy owns immutable member scope, admission bounds, outcome projection and the
shared-work cancellation rule. Private indexed batch/member references and direct
request pins are committed with admission. Store evaluates that application rule
under the same transaction that changes cancellation state. Fan-out rechecks the
parent and its claim before each child admission; cancellation cannot admit a late
child. A direct request made after batch acceptance protects that shared work too.
Frozen recipe/output identities determine equivalence; the first accepted creation
time is retained on reuse. Parent failures remain visible independently of child
outcomes, and retries retain attempt diagnostics and enforce guided cancellation.

Collection reads cross `CollectionReadPort`, whose methods receive durable identities and queries rather than a Principal. `CollectionService` owns grant checks, role redaction, and conversion into explicit collection/item/evidence response types. Viewer JSON structurally omits acquisition, private notes, loans, metadata observations/overrides, detailed Evidence fields, and raw module envelopes; unavailable modules retain a safe non-executable notice without their raw generic value. Capture DTOs never expose archive locators. Owner and curator DTOs retain permitted curation detail, and every authorized collection/item response carries server-computed effective permissions. `CollectionService` is constructed from narrow ports; the combined `Catalog` trait remains only as an internal Store compatibility adapter while callers use the policy-owned service families.

Application mutations carry a validated `CommandContext`. Private SQLite schema 3 claims receipts by `(UserId, API major, operation, Idempotency-Key)` and binds the claim to a SHA-256 digest of the validated typed request, path identities, and operation. Authorization and validation run before a claim. A completed receipt replays its non-secret serialized result across restart; a changed digest or active claim returns a stable conflict; an effect error abandons an unprepared claim; a prepared publication remains recoverable until startup resolves its commit marker. Authentication ceremonies, logout, resumable upload chunks, and internal failure bookkeeping use their own single-use or offset state machines instead. The HTTP and React layers transport keys but never decide replay policy.

Command execution creates an explicit `CommandEffects` scope bound to a typed `CommandReceipt`. The scope shares the Store's locks and connections; it carries no Principal, thread-local state, or ambient authorization. Private mutations complete the response receipt in the effect's SQLite transaction. Filesystem mutations durably prepare the exact response in private `command_outcomes`, bound to a root-relative publication marker and digest before publishing. Startup validates the marker and replays committed results, or releases uncommitted claims, before opening services. Upload bookkeeping and committed artwork scratch cleanup are reconciled after projection rebuild. An operation must use one of these two commit mechanisms; returning an effect and subsequently writing its first durable receipt is not an acceptable adapter implementation.

Failed manual metadata updates can reserve an immutable revision before a filesystem
error prevents journal creation. Under the archive writer lock, the storage adapter
releases such an in-progress reservation only when its marker is absent and the
publication journal directory is empty; uncertain I/O and unfinished journals fail
closed. This runs during abandonment and before another publisher claims the same
path. Startup can also resolve a previously retained losing metadata reservation
when the actual revision, manifest and an independent completed or exactly matching
pending command attest the winning record. It never replays the winner as the
failed command's result. Unattested digest changes or inconsistent result envelopes
still reject recovery without modifying the archive.

Security rules live in `collector-app::security`: account/grant authority snapshots, default-account eligibility, credential revocation sets, and enrollment transitions. Store gathers facts under an immediate SQLite transaction, evaluates that policy, and applies its effects before committing. CAS and serialized snapshot loading are storage mechanisms; last-owner/admin/default and lifecycle decisions are application policy. Grant revision counters survive removal, so recreating access cannot make a stale revision current again.

API route groups are annotated once and supply both Axum runtime routing and the checked OpenAPI 3.1 document. PJ1 shares list command/read and music browsing DTOs through `collector-api-types`; unrelated application command types remain in `collector-app`. Those Rust contract types generate `api/openapi.v1.json`; `openapi-typescript` generates the React-only type surface. The regeneration check writes temporary outputs and fails on byte differences. Request-body limits are included as `x-body-limit-bytes`. The server does not expose OpenAPI or Swagger UI. The bounded Wave 1 item envelope is a deprecated application adapter: it loads canonical server-owned state, rejects differences, then delegates only editable fields to the current command.

Authenticated collection-type discovery returns only registry-validated module descriptors: bounded labels, scalar field contracts, read-only structured-field notices and capability availability. It exposes no module configuration or management switch. Application read DTOs combine grants, durable binding and registry/schema availability into explicit content permissions; access management and personal activity remain separate. Domain-patch receipts bind the submitted patch under their own operation rather than its state-dependent resolved payload. Generic clients submit owner-bound domain patches, never replacement raw envelopes, and render type labels/summaries without branching on built-in module IDs. Legacy assignment guidance names the stopped-server inspection and assignment commands; unavailable owners remain safe read-only presentations.

Mutations cross bounded application commands. Item input contains editable inventory/module fields only; Capture representations, Assets, Evidence, metadata observations, revisions, deletion state, timestamps, and storage locators cannot be deserialized into it. Work/Edition commands preserve migration provenance and server-owned revision state, and Capture commands can change associations while preserving representations; legacy locations exist only in store adapter records/projections. Content, import, administration, authentication, enrollment, and access adapters receive validated identities/effects rather than Principal or Role. Media returns an opaque synchronous read/seek capability so neither application code nor HTTP sees an archive path; HTTP streams it on blocking workers with range semantics. The server source is checked against direct `state.store` calls. Authentication freshness, authority guards, passkey CAS, recovery, and native enrollment are application policies backed by transactional Store mechanisms.

Module wire payloads use a lossless raw-JSON envelope. Known current schemas are parsed into module-owned views without replacing the raw body. Supported older schemas migrate one adjacent version at a time. Additive checks are the default; a version-specific transformation may rename or convert fields only through an explicit policy with a rationale. Every exact input/intermediate envelope is returned as provenance and must be retained by the committing host. Unknown fields remain preserved in those envelopes. Capability ownership is indexed and validated at registration, never inferred by splitting a capability string at dispatch. Each registered operation declares collection-domain or shared-media scope, and every successful output passes its owner's required result validator before publication. Module descriptors expose only bounded labels and typed scalar/read-only field descriptions. Owner-mediated patches distinguish omission from explicit null and preserve untouched raw values; clients do not replace arbitrary module envelopes. Newer, gapped, unavailable, and invalid schemas remain distinct outcomes; unavailable data survives but is not described as executable. Generic presentation is an additive, post-redaction read model and does not expose operation commands before A04. Raw provider JSON is source evidence, not the common field model. Runtime plugins will adapt the same capability boundaries; a native ABI is deliberately not promised.

## Identity and ownership

Collection, item, work, edition, specimen, component, carrier, capture, representation, package file, processing observation, snapshot, recipe, asset, evidence, user, device, and server IDs are distinct Rust newtypes. Paths and SQLite row numbers are never identity. Works and editions are collection-owned records; an inventory item may be unidentified or linked to a work or edition. Duplicate physical copies are separate specimens. Components describe physical parts, while only data/audio-bearing components have carriers. A capture is collection-level and may remain unattached to an item or physical carrier. No physical specimen or catalog identity is fabricated by importing a dump. Wishlists, ownership, possession, loan records, and disposal are independent. A tombstone does not delete payloads. The complete reviewed relationships are recorded in [foundation contracts](decisions/foundation-contracts.md).

Game and music data, processing results and domain metadata overrides live in module-owned types. Core retains common title/subtitle/artwork authority and genuinely shared CD/PCM coordinates and layout. Unknown historical field overrides are retained verbatim in `preserved_field_overrides`, including language and unknown values; they are not applied as active authority until explicit owner-aware conversion. Reading/reindexing does not rewrite their original archive records. The collection-binding contract is Assigned to a ModuleId or LegacyUnassigned. New creation persists an Assigned owner; a missing historical field becomes LegacyUnassigned. Graph publication rejects that holding state. Application content writes centrally require Curate authority, an Assigned binding, an installed owner and supported schema; collection-domain operations must belong to that owner, while explicitly shared-media operations need not. Durable processing inputs freeze and recheck the binding, but already committed recovery completes without module availability or grants. Collection write helpers live in `collection_types.rs`; stored unsupported Item schemas cannot be overwritten through ordinary edits. Long import adapters invoke application policy after payload staging and before a new Capture commit. A denied guard clears provisional publication copies and retains source uploads. Integrity progress also rechecks eligibility; committed integrity recovery never rehashes payloads. These checks are not a serializable transaction with concurrent grant changes. Explicit local assignment is application policy in `collection_types.rs` and publishes a journaled collection-metadata sidecar without changing the creation manifest. Its caller holds exclusive stopped-server access throughout inspection and assignment. Module validators check active authority; shared-media operations do not imply collection ownership, and unknown pending jobs block assignment. Core inventory remains useful without a dump or definitive identity. Metadata candidate ranking uses identity strength before preferred language. Manual choices and overrides are separate from observations. A provisional display selection never becomes checksum evidence.

## Archive and publication

### Browse navigation and query projection (FX-01)

BrowseQuery carries typed sorting and a single typed filter. Application services
retain collection authorization and role redaction; module-owned descriptive
fields supply artist/platform keys. Catalog schema 7 adds disposable indexed
browse keys for effective title, artist, platform and first retained revision
time. Metadata publication updates the effective-title key with search. Reindex
reconstructs creation order from digest-checked earliest metadata revisions,
never filesystem timestamps or the latest edit. No archive schema changes.

Cursor tuples include the sort key, title and stable item identity and are bound
to the user, collection, search, sorting and filter. Paging is a live keyset view,
not a frozen snapshot across concurrent edits. Reads hydrate only the returned
bounded page. Files-attached means published Capture files. Playable means a
published supported audio master or prepared track with its retained recipe;
browser codec support is still a playback concern. Needs-attention means the
requester's failed Jobs or Imports awaiting review, without exposing other users'
private work. These filters query committed metadata only; they do not scan or
hash payloads. UI query parameters identify scope and selected item; history
state stores page count, scroll and focus identity, never private item contents.
The explicit detail return tracks the distance to the originating results entry.
Shared Work & history URL navigation counts its list/detail pushes in that same
distance, so closing nested work cannot redirect “Back to results” into an older
modal. Browser Back/Forward still restores the requested work URL. Direct detail
links without a recorded results target replace the current entry on return.
The mounted browser caches loaded pages for incremental Load more; a scope change,
session clear or explicit refresh invalidates reuse. Reload restores only the
number of pages previously requested, using bounded cursor reads.

The archive is portable and contains no credentials or access grants:

```
collections/<readable-name>--<collection-id>/
  collection.toml
  collection_metadata/<collection-id>/revisions/<zero-padded-revision>/...
  works/<work-id>/revisions/<zero-padded-revision>/...
  editions/<edition-id>/revisions/<zero-padded-revision>/...
  items/<readable-title>--<item-id>/
    revisions/<zero-padded-revision>/...
  captures/<capture-id>/
    revisions/<zero-padded-revision>/...
    payload/<representation-id>/<original-package-paths>
  assets/<asset-id>/
    revisions/<zero-padded-revision>/...
    payload/original.<format>
    payload/thumbnail.png
  evidence/<evidence-id>/revisions/<zero-padded-revision>/...
```

The tree above remains the schema-3 layout for existing records. Archive envelope schema 4 additively introduces ProcessingObservation, compact ProcessingOccurrence, provider/catalog Snapshot, MetadataAuthority, and ConversionRecipe record kinds without rewriting schema-1/2/3 bytes. Mixed-version reindex preserves unknown record and module envelopes. Import-scoped observations are rebound at Capture publication to the committed Representation only after the ordered file identities, lengths, and digests match; their provisional fingerprint and explicit file mapping remain provenance. Migrated schema-1/2 artwork remains byte-for-byte at its item-local path and is reached through adapter-only locators; it is not moved or rewritten.

MetadataAuthority revisions keep typed common title/subtitle and artwork authority,
an owner-bound domain override envelope, and preserved unknown historical fields.
A refresh appends its bounded lossless observation in the same revision as the
materialized display. It replaces the active candidate projection and remaps the
displayed choice to an equivalent incoming candidate; a nonmatching displayed
choice remains selected and visibly unmatched. Manual field and artwork authority
is unchanged. Reindex validates each historical authority revision against its
then-live candidates, retaining original observations in the archive while
projecting only the newest active candidates and authority/display revision. Resolution applies field
overrides, explicit selection, then deterministic automatic ranking by complete
identity, evidence strength, English preference, provider priority and stable
provider identity. Metadata identity remains distinct from integrity, catalog
verification and conversion validation.

Candidate identities are immutable: changed normalized content or raw responses
require a new provider record version (a response digest when no upstream version
exists). Observation IDs are also immutable and bound to their item/collection;
receipt identity, observation identity and publication transaction identity are
separate. Publication rejects contradictory history before committing bytes, and
reindex checks the same invariants. Catalog v6 projects candidates and refreshes.
Each new authority record freezes its inventory revision/title/subtitle. Later
inventory text edits become manual authority at the application read/next command;
unchanged inventory fields do not undo explicit overrides or clears. Publication
checks that inventory baseline under the archive writer lock. Reindex validates
materialized display against frozen inputs and candidate history at that revision.
Automatic ambiguity remains tentative even when unrelated manual choices exist.

Authority revisions additionally retain current candidate identities for automatic
resolution. Superseded unused versions leave the selectable projection, but their
provider observations and immutable archived revisions remain preserved.
Provider-derived versions include raw/normalized content, ranking and artwork inputs.
MusicBrainz/CAA adapters join the existing acquisition and catalog-parser registries;
module DTOs own provider interpretation, and application services share existing Job,
Asset and metadata-refresh effects. Acquisition receipts bind submitted item/query
inputs before timestamps or other frozen state. Non-JSON negative responses remain
exact snapshots referenced by metadata; malformed successful responses are failures.
The HTTP connector validates its actual DNS answers, disables ambient proxies and
pins HTTPS provider/storage origins. MusicBrainz has one composed admission/cooldown
limiter and a process-local cache bounded to 128 entries/32 MiB. Curation-only metadata
HTTP/UI exposes explicit acquisition and authority changes; imports do not implicitly
request enrichment. Application services dispatch completed, caller-owned, item-bound
acquisitions to the existing single metadata or Asset effect. Artwork publication
preserves an image for explicit cover selection without requiring another metadata
request. Clients persist completion receipt inputs across panel reopen and abort
obsolete requests. Effective display and rebuildable search share application-owned
resolution, including later inventory corrections; adapters do not invent precedence.

TOML manifests contain schema version, entity kind, typed identities, revision and the SHA-256 of the corresponding record. Archive schema 3 uses deterministic additive sidecars over schema-1/2 history. Canonical Item records contain inventory state but no embedded Captures or Assets; standalone Capture revisions and revisioned collection-level Asset records are authoritative. Append-only Evidence has collection identity, optional Item identity, one or more exact Representation identities, and is referenced by digest from revision envelopes. Legacy payload locations are adapter-only records. Item responses remain compatibility views hydrated from these canonical rows. The module/provider envelope preserves lexical JSON, including key order, number spelling, and nulls.

The A02 migration preserves every schema-1 `record.json` and manifest byte-for-byte and writes schema-2 sidecars plus typed identity/capture records. P01.3 likewise preserves all schema-1/2 bytes and adds schema-3 Item/Capture sidecars, standalone Evidence, Asset metadata revisions, legacy locators, per-Item checkpoints, and a completion marker written last. Outputs and deterministic transaction identities must match on restart; conflicting reused Evidence identities fail migration. Neither migration reconstructs private grants, so archive-only recovery has no access assignments.

A single process lock owns archive writes. Current item/typed-entity publication checks expected revisions under the writer lock, stages and syncs content, atomically renames the revision directory, and updates SQLite in a transaction. The rename is the archive commit point. Capture and collection-binding records use the same typed revision publisher during migration, recovery and normal updates. Collection graph validation enforces unique physical/representation identities, assigned-owner/domain consistency and transitive association ancestry, including omitted intermediate links; writes and rebuilds reject cross-collection identity reuse. Ordinary writes load only the candidate and indexed inbound/outbound graph neighborhood, then apply the same core graph invariants. Catalog triggers maintain global typed identity ownership and references transactionally. Whole-collection owner reassignment and explicit rebuild still validate the complete graph; final deployment scale qualification remains R02.

Durable publication journals contain transaction/entity identities, expected/target revisions, safe relative paths, hashes, and monotonic phases only—never users, grants, credentials, or secrets. Startup discards recognized interrupted journal replacements and aborts incomplete Prepared staging. After BytesStaged it validates staged digests, and published bytes are always validated. It retains unknown staging, removes durably completed known scratch, and blocks writes on corrupt known transactions. Entity and import writers use the same phases; resumable upload chunks stay private and separate. An import's stable Job identity also identifies its payload transaction, so recovery before the Capture commit removes known uncommitted artifacts while recovery after it preserves the Capture and completes deterministic Evidence/Item projections. Collection creation instead uses a private recovery row written before archive mutation because its intended owner grant is not portable archive data. Startup completes or aborts that row under the writer lock before any service is exposed.

Collection creation writes an immutable collection manifest and a private recovery intent for the owner grant before archive mutation. Startup completes the intended grant when the collection commit exists or removes the abandoned intent when it does not. Copying only the archive never reconstructs a grant.

SQLite state must be local to the server. The current rebuildable `catalog.sqlite` is schema v15 and `private.sqlite` is schema v21. Catalog v4 adds portable processing, snapshot, metadata-authority, and recipe projections to the v3 projections; catalog v5 adds the snapshot creation-time index used for stable pagination. Private v10 adds frozen processing Job inputs and provisional Import observations to the v9 authentication, access, Import, Job, command, and recovery state. Private v11 adds collection/provider active-catalog selection; exact snapshot bytes and observations remain portable archive records, while this replaceable selection is local policy. Private v12 was an unused planning reservation. Private v13 adds guided-ingest intent and restartable staging-cleanup records; source bytes and published archive records remain outside that private workflow state. Catalog v8 adds rebuildable substantive-result groups and occurrence membership; catalog v9 adds typed entity ownership/references and the archive filesystem observation/checkpoint tables; private v14 adds requester-scoped scan admission keys and durable direct/parent ownership references. Existing frozen jobs acquire admission keys on startup without changing their original input timestamps. Private v15 adds requester-private history visibility and durable clear intents, with pending-intent and import-job indexes. Catalog v10 introduces metadata-health/language display; v11 adds indexed album-peer and preparation readiness; v12 adds portable collection-list projections and v13 song-search projections; v14 binds song-search rows and cursors to optional source representations for game audio; v15 indexes all viable album options per entry for ambiguity-aware grouping. Private v16 stores metadata language, v17 stores intake sessions/discovery and v18 stores frozen bulk work. Private v19 permits native enrollment requests awaiting account approval; v20 adds shared acquisition ownership, specific pending dependencies and requester snapshot/generation bindings; v21 adds durable opt-in inbox source cleanup claims, frozen filesystem observations and deletion progress. Grouped merge revisions use portable immutable publication-group manifests and archive lineage; readers expose members only after the complete committed metadata graph validates. These are additive pre-release migrations: older on-disk user data is converted in place and is never reset automatically; no permanent reverse-compatibility layer is promised solely for earlier Wave checkpoints. Client connection storage is independently versioned: C01 starts at client schema v1 and S04 reserves client schema v2 for rotation state. Future version assignments follow actual persisted state and the pre-release policy; W4.0 reservations are advisory. Reindex validates the collection graph and processing envelopes and replaces all catalog projections in one transaction. Browser sessions and native credentials are distinct types and tables; migrated sessions are non-fresh and never become native credentials. Transaction-based migrations select the former first-row account once; later account or `multi_user` changes do not change it. A missing or disabled default is an actionable configuration error. Back up both the archive and private state. Copying only the archive restores records but no access. SQLite databases must be backed up through SQLite's backup mechanism or while stopped, not by copying active WAL files independently.

P06 reuses the incrementally maintained catalog at startup when a checkpoint matches the canonical archive path, projection epoch and complete filesystem observation inventory. Observations cover metadata revisions, snapshot files and migration checkpoints using file size, modification time, and on Unix inode/device/change time; payload trees are pruned. This is a change hint, not byte integrity or catalog verification. Warm startup still walks metadata names/stats and runs SQLite quick_check; it does not parse archive records, revisit completed migration records, or hash snapshot/media bytes.

The archive rename remains the commit point. After projection commit, a separate SQLite transaction updates observations for only the publication's artifacts and seals its projection epoch, before removing the durable journal. Projection-table/FTS/ownership/reference changes invalidate that epoch. A pending or incomplete journal, absent/incompatible checkpoint, different archive path or changed filesystem observation forces an atomic complete rebuild; external changes are not selectively replayed. Full rebuild validates archive records/snapshots and refreshes the checkpoint in the same catalog transaction. Missing/corrupt/incomplete catalogs rebuild; damaged catalog files are retained under unique diagnostic names while private state remains untouched. A restored archive is authoritative, never a stale local projection. Pre-release schema changes must advance the checkpoint format or catalog version whenever projection semantics change. The measured fixtures and recovery evidence are recorded in [P06 validation](validation/p06-indexing.md). Payloads are never read merely to browse. Writes are single-server; independent clones must not reuse operational identity. Replication requires additional protocol work, although IDs, revisions, immutable payloads and tombstones do not assume a server location.

## Ingest and assets

### Explicit inbox source cleanup

Cleanup is an optional local-inbox action, separate from import acceptance. The
application permits it only for the requester's published guided import after all
requested processing settles, with current curation authority. Optional metadata/
preparation failures do not block cleanup: retries use the preserved archive. A durable cleanup
Job freezes the selected configured source path and filesystem identities at
admission; command replay never re-resolves mutable inbox configuration. The
adapter conservatively pauses new inbox admission while a cleanup Job is active
(including configured root aliases), refuses unfinished inbox imports/copies, and checks every source and archive master digest in cancellable bounded
reads before removing anything. Old imports can use the same explicit action;
their committed file inventory supplies the preservation contract.

A private `.collector-cleanup-<import-id>` directory beside the source holds an
atomically renamed package. Its durable intent precedes rename, so restart and
retry address that exact directory/inode, never a replacement at the original
name. Only inventoried, unchanged files with independently readable archive copies
are unlinked, with a durable per-file deletion intent before each unlink. Unknown
and housekeeping files are retained, and only empty directories are removed.
Cancellation/failure preserves remaining inputs and reports progress/recovery
location. Explicit retry can refresh a pre-move observation; after quarantine it
must resume the same retained package. Generic Job retry is refused for this kind.

The web uses typed cleanup status/permissions, an explicit confirmation that the
source is no longer being copied/edited, and ordinary detail polling. External
programs must finish writing before opting in; filesystem permissions and identity/
change checks do not revoke already-open external write handles. Archive masters
are never cleanup targets. This is not general archive pruning or automatic inbox
deletion, and inbox cleanup recovery directories remain external to full backups.

### Ingest execution visibility — 2026-10-21

Application-owned `IngestSummary.work` describes current running, queued, waiting
and cancelling stages independently of the overall workflow outcome. The private
adapter groups active Job rows using the import's durable ownership, preparation
member links and the requester's current acquisition wait. A provider wait exposes
the actual shared provider execution state, marked when shared; a completed batch
admission still exposes its unfinished track jobs. Item identity alone never
selects related work. The projection reads no media bytes or frozen source inputs,
adds no stored-state migration, and retains requester/collection authorization.

Import rows and focused details show every concurrent stage/count and keep polling
through enrichment/preparation. Missing jobs between reconciliation passes mean
waiting for the next stage; incomplete browser uploads remain waiting for upload.
Analysis and audio workers report source checking, computation/conversion,
rechecking and publication boundaries without invented completion percentages.
See [local acceptance](validation/ingest-progress-2026-10-21.md).

### Focused intake and work navigation

Guided ingestion is the primary new and existing Item intake coordinator. Its
accepted intent freezes the destination Item and revision, analysis policy and
package inventory before staging. A selected existing Item receives one package
without creating a placeholder Item; its independent inventory and other Captures
survive reassociation. Preservation-only intake skips structural analysis,
automatic provider lookup and audio preparation while retaining original bytes and
an honest unidentified outcome. The private intent records whether an Item was
created by this workflow so cleanup never retires a pre-existing destination.

Supported music analysis derives the MusicBrainz disc identifier from the parsed
CD layout. The application admits one durable provider acquisition per import and
advances it after analysis, independently of the browser panel. Source filenames,
folders and placeholder titles are never automatic provider queries. Advanced
manual provider lookup remains an explicit recovery tool. Neither a provider
candidate nor structural evidence verifies catalog identity.

Work & history has a bounded list query and an authorized focused entry read. A
selected import's detail is a separate URL-backed view; list filters and loaded
pages remain mounted in state for Back navigation without rendering the list under
the detail. The current-import panel reads only that import. Destination selection
uses authorized paged catalog search and checks the chosen Item's current revision
before submitting or correcting an association. Configured inbox labels are read
without package enumeration; package search scans the selected direct inbox root
with a bounded sorted response and cursor. The former full-package response remains
available for pre-release callers, while the browser uses the paged path.

Uploads declare a complete file inventory, optional source SHA-256 values, item and expected revision. Names reject traversal, absolute paths, control characters and Windows separators. Chunks are bounded to 8 MiB; browser chunks use 4 MiB. Durable file lengths recover upload cursors after restart. Duplicate identical chunks are idempotent; conflicting overlaps fail. Completed staging still accepts byte-identical chunk replay read-only. Browser resumption validates the reselected inventory and compares retained prefixes before appending remaining bytes. Startup resolves abandoned cancellation acknowledgments atomically with their Import state; once a Capture is committed, recovery completes its existing publication rather than reopening pre-commit Item revision review.

Commit checks complete lengths/digests, copies into archive-local staging, rereads destination SHA-256, publishes files and then item/evidence. Source uploads are retained. Integrity means those bytes match, not catalog identity. Reverification appends evidence. Catalog verification, reconstruction, offsets and audio derivatives require separate providers and independent oracles.

Artwork is stored as an original image plus a bounded PNG thumbnail. Image decoding has dimension/allocation limits. Both asset and original-media requests enforce collection access and asset visibility. Private attachments are not merely hidden in HTML.

## HTTP exposure and trust

Authentication-origin configuration is validated by the server's WebAuthn adapter
before setup/doctor/serve opens storage. HTTPS DNS origins enable passkeys; HTTPS
numeric IP origins keep password/recovery available and report typed
`dns_name_required` passkey unavailability. An absent origin reports `not_configured`.
Malformed/non-HTTPS origins fail rather than silently changing login policy. One
configuration enum owns both the live verifier and its unavailable reason; the
features API and security panel consume that state. TLS validation, exact Origin
allowlisting, listener composition and stored credentials are unchanged.

Runtime curation/API composition authenticates callers before body extraction;
administration composition also calls application administrator policy before
reading a body. Handler/application checks revalidate current authority and all
collection-specific/freshness/command policy after extraction. This prevents large
anonymous curation bodies from being parsed while keeping explicit bootstrap
routes and exposure composition independent of caller headers. See the
[focused ingress review](validation/ingress-review-2026-10-21.md).

Three surfaces: client, curation, administration. Public curation and administration default off. Public administration implies curation exposure; roles remain mandatory. Routers omit disabled pages, JavaScript management entrypoints and mutation APIs. The public listener stays public regardless of `Forwarded` or `X-Forwarded-For`.

Authentication/enrollment throttling is a process-local HTTP adapter mechanism.
Global, direct-peer and hashed-account buckets share a monotonic 60-second window;
forwarded headers never select a peer. Every check evicts expired empty buckets and
creates identity buckets only after admission. Rejected alias rotation therefore
cannot grow retained state: current identities are bounded by the corresponding
global admitted-attempt limit. Enrollment polling follows the same rule. A process
restart resets these transient limits, while persisted credentials/sessions retain
their normal recovery semantics. This is single-process abuse protection, not a
distributed limiter or permanent account lockout.

The process exposes two loopback HTTP origins for separate public/private proxies and one direct LAN TLS client listener. An optional `private_tls_bind` adds direct private management HTTPS on an explicitly named interface; wildcard private TLS binds are rejected. Connect a public tunnel only to the public origin. A private proxy or Tailscale Serve connects to the private origin. LAN TLS is public-surface by default; local connectivity alone never upgrades it to administration. Allowlisted browser origins are exact and configured, not derived from proxy headers.

Native pairing carries server ID, local certificate and its SHA-256 fingerprint. Trusted enrollment is required; a fingerprint downloaded unauthenticated is not trust. The local client installs only the enrolled certificate as its trust anchor and uses normal certificate/hostname/handshake validation. No accept-invalid-certificate option exists. Certificates must include the configured LAN names/IPs. Public routes use ordinary Web PKI separately. Credentials must never follow cross-origin redirects.

Passkeys use one configured HTTPS authentication origin and standard WebAuthn. Challenges are bounded, server-side, expiring and single-use. Registration is bound to the authenticated user and ceremonies retain the original credential record/revision, and counter updates compare-and-swap that verified snapshot together with session issuance. Reloading a newer record after verification is forbidden; zero-counter passkeys retain the WebAuthn library’s verification semantics. Credential IDs are unique across accounts. CLI setup explicitly allowlists the private listener’s localhost browser origin; it does not change public exposure or trust forwarded headers. Browser sessions are HttpOnly, Secure and SameSite; browser ceremonies and cookie mutations require an exact normalized Origin. Native enrollment start/complete are the narrow unauthenticated bootstrap exception: they grant no authority before same-user, recently-authenticated browser approval and authenticate completion with a stored-hash high-entropy secret. The issued 30-day native credential is returned once, stored only as a hash server-side, and selected only by successful native-credential lookup—never by a caller header. Single-user presentation hides account selection; it is not anonymous access.

## Clients and shared audio

### Game-disc CD audio — active delivery, 2026-10-21

Games retains the Game Item, catalog identity, artwork and source-specific manual
track authority. Shared `junk-libs-disc` owns strict CUE geometry, gap semantics,
authorized-handle PCM traversal and concatenation of admitted audio ranges. A
module-owned recorded disc-audio manifest binds the selected CUE anchor and exact
logical/physical audio coordinates to its enclosing source fingerprint. Data tracks
are excluded; unsupported raw/multisession layouts remain explicitly unavailable.
No music duplicate, new importer, new player or automatic soundtrack decoder is
introduced. A module's declared listening support enables the existing application
preparation/listening policy; it does not change collection ownership.

Application services reuse durable derivative jobs, recipes, source/claim/role
fences, media authorization and source-bound manual metadata. Browsing, song search,
native snapshots and admission use committed manifests/recipes; background work
validates actual source bytes and rechecks geometry before encoding/publication.
Historical imports can request explicit background audio inspection. Game songs
retain their game artwork/title and actual track numbers, with explicit source
choice for alternate originals. Collection-entry lists keep their existing kind;
explicit audio playlists use typed track targets and the same collection boundary.
Web and Trash Amp consume the normal track/media/list contracts. New game audio
remains outside advertised release coverage until synthetic and owned-disc, actual
UI/native/recovery acceptance is recorded.


Album-disc coverage is an application read projection over the chosen descriptive
MusicBrainz release, each active preservation-master Capture, and current
source-fingerprint-bound CD structure/AccurateRip observations. It never opens or
hashes original payloads during browsing. The music module owns the typed manual
source-to-release-medium assignment; the application validates that its source is
attached and its medium belongs to the currently chosen release. Track-name
resolution applies a release medium separately to each source, requires complete
track positions, and withholds names on a recorded disc-ID conflict. The UI may
show an unmatched original or an unrepresented release medium without claiming
catalog or audio verification. Technical processing history remains accessible
behind the collector-facing disc summary.

The bounded browser player has one owner at the authenticated application shell,
independent of album views, navigation and curation refreshes. Album views enqueue
one browser-decodable prepared representation per selected track; they do not own
media elements. A fixed bottom dock appears for music browsing or an existing
queue. Its personal queue (at most 5,000 entries, immutable item/representation/file
coordinates and display metadata) is local to this browser origin and user, never
server activity. Reload restores a paused queue, not autoplay. Selecting, removing
or clearing the current entry releases obsolete playback; reordering/adding other
entries and refreshing catalog data preserve the current media element and time.
Current-file availability is rechecked before playback and periodically even away
from its album. Access loss stops buffered audio, and session loss/sign-out detaches
the player and clears in-memory personal state. Local queue data grants no media
access. Ref detachment releases the media element and superseded asynchronous
requests/play promises are fenced. HTML media seconds remain a presentation
projection; native queue/PCM/DSP services remain separate shared-library work.
Client playback never requires the curation bundle or surface.

Artwork is part of browsing data, not a decorative optional gallery: games and CDs show cover thumbnails next to text, including search and offline snapshots. Unidentified/tentative identity is distinct from missing artwork.

Client connection/cache logic stays below Slint and SwiftUI. LAN preference is independent of browser certificate trust or passkey origin. Offline cache identity includes server, representation, and conversion profile; pinned entries survive automatic eviction. Offline content cannot be recalled by server revocation.

`junk-libs` already allows per-crate Cargo dependencies. Keep one workspace unless releases diverge. Extract disc IDs, verification, decoding, queues and DSP as narrow reusable crates, without archive/accounts/UI dependencies. Trash Amp retains its standalone local-player role. Cross-project extractions are separate migrations with sibling regression checks.

## Extension discipline

Before adding a module: identify its owned data, invariants, capabilities, persistence needs, and presentation contribution. If its implementation would bypass an existing boundary, change this architecture and its contract tests first. Avoid a second importer, authorization policy, parser or playback queue inside a frontend.

### Ephemeral provider artwork previews

Cover Art Archive listing options carry their retained Snapshot identity. The
curation-only preview API resolves that verified listing and enforces collection
Curate access before passing its release/image IDs to `ArtworkPreviewPort`.
The existing CAA adapter implements the port using its pinned redirect/DNS policy
and bounded reader, requesting the provider's [250px endpoint](https://musicbrainz.org/doc/Cover_Art_Archive/API).
Previews are transient image bytes, not Assets, provider observations or evidence;
only explicit acquisition archives the original. The HTTP adapter limits concurrent
reads to four, propagates disconnect cancellation and keeps the permit until the
blocking read exits. Bodies are limited to 1 MiB and raster image MIME types; the
browser uses authenticated same-origin image URLs and retry placeholders. CSP
remains same-origin and previews are not stored in browser HTTP caches.

## Phono Collector integration (PJ1, 2026-10-04)

The [delivery ledger](phono-integration-plan.md) records implementation separately
from physical acceptance. A rebuildable music_search projection indexes current
materialized song title/artist/album and fingerprint-bound recorded track positions.
Reindex restores it; browsing does not hash or open masters. Native read DTOs expose
listener-effective capabilities. Album playback reuses existing target expansion.
collector-client owns certificate-pinned enrollment, authenticated bounded JSON and
artwork reads, origin checks, structured errors and validator-bound HTTP ranges.
Phono keeps Keychain credentials and a separate durable command journal, and feeds
shared lazy decoding/resampling into one continuous Kira voice.

## Account connection usability repair — 2026-10-07

A normal browser-trusted HTTPS address is the default native enrollment route.
Collector-client uses system HTTPS roots with redirects disabled, freezes the
server identity at enrollment start, and stores the transport trust mode alongside
the Keychain credential. System-HTTPS enrollment retains only its authenticated
origin; it does not send credentials to advertised alternate routes. Existing
pinned-certificate credentials retain their actual trust and route data. Owner-
provided certificate enrollment remains explicit for locally issued identities.
The browser `/pair` page is a client surface independent of curation/admin exposure;
approval uses recent browser authentication and atomically binds the enabled
approving account.
Phono views call the existing service/worker; no second account authority is added.
See [the usability review](account-connection-usability-review.md) for remaining
account/recovery gaps and acceptance boundaries.

## Complete account/connection delivery — accepted 2026-10-07

Native enrollment starts without an account; explicit recent browser approval
atomically binds it to an enabled account. Secret-authenticated status/cancellation
reveals no credentials and checks ownership before state transitions. Connections
are keyed by server and account throughout Keychain, requests and playback.
Application services own password/revocation policies; Store commits their effects,
audit and receipts together. Ordinary password change retains access; explicit
other-access revocation preserves the current browser and passkeys/codes; code
recovery revokes sessions/apps/enrollments/codes and issues a fresh session, while
local recovery additionally revokes passkeys. Private-state conversion preserves
bound enrollment rows and credentials. Native legacy credential conversion verifies
`/me` before committing account-keyed storage; no identity is guessed from edits.


Stopped-server account lookup and recovery share an existing-state opener and the
same exclusive private lock as the server. They do not open the archive. The CLI
reads only storage locations from configuration by default; an explicit `--state`
override bypasses configuration for damaged-configuration recovery. Recovery
targets exactly a sign-in username. The read-only `paths` command reports absolute
configured locations without opening or locking storage, including while serving. Account creation and
collection grants remain separate durable application commands. Server authority
never implies collection access. The [account delivery ledger](account-usability-delivery.md)
records schema 19 and native storage backup/recovery details.

Password-change receipt input binding uses deterministic per-account/command Argon2
through the authentication mechanism port. It retains exact-input replay without
adding a fast password fingerprint to private receipt data. Existing account-create
receipts retain their recorded input binding so pending commands remain replayable.

## Web navigation presentation — 2026-10-07

The destination rail groups Browse, Organize and Manage separately from account
commands. Collection scope is a searchable/paginated workspace disclosure, using
existing authorized search and navigation cancellation. Navigation presentation
adds no application policy or durable state; routes/history and the persistent
music player retain their existing owners. See the
[web navigation design](web-navigation-redesign.md) for decisions and acceptance.

## Local certificate onboarding — 2026-10-07

Administration's certificate download reuses the authenticated pairing read port;
it exports only its public PEM and displays the existing PEM-byte fingerprint.
The browser owns the transient Blob download and cleans it up with the panel.
Neither the browser nor native UI decides server trust from downloaded material
alone. Local operators can obtain the certificate through the private loopback
management interface or the server's printed filesystem location, then establish
trust explicitly in Trash Amp or macOS. The server reports a local HTTPS listener
address separately from advertised names, which do not create DNS. No OS trust
change, certificate replacement or validation bypass is automatic.

## Trash Amp native consumer

Trash Amp uses `collector-client` as its transport boundary. Native credentials
select either normal system HTTPS trust or explicit owner certificate trust; the
media reader uses the same enrolled trust policy as catalog requests. Presented
list occurrences expose authorized stable source coordinates and current prepared
file length/digest observations for explicit offline retention. These fields are
resolved by the existing list application service, subject to its collection
authorization, and do not trigger payload hashing or server preparation. Native
caches, queues and download policy belong to Trash Amp, not Collector core.

## Manual game catalogs — 2026-10-09

The [manual DAT delivery](game-catalog-upload.md) extends the existing provider
registry and acquisition/publication pipeline with administration-authorized
browser uploads. Application policy binds metadata and exact byte identity;
Store syncs a private content-addressed spool and supplies opaque readers through
`UploadedCatalogPort`. Uploads are bounded to 64 MiB and two concurrent HTTP
bodies. Acquisition Jobs own validation, cancellation, restart and immutable
snapshot publication; activation remains a separate collection command. Parsed
snapshot samples are bounded and checked against actual bytes on snapshot reads.
No provider downloader or second catalog publication pipeline is introduced.
Ingest package plans carry typed music-action eligibility for frontend controls.

### AccurateRip browse presentation

Application presentation exposes recorded audio-check status per visible, currently
associated preservation master on music items. It reuses the music coverage query's
fingerprint-bound observation resolver; browse does not read or hash source bytes.
The web list summarizes original-level results and opens the existing processing
workspace, whose application commands remain responsible for eligibility, snapshots
and publication. A verified original count does not establish album disc coverage
or MusicBrainz identity.

### AccurateRip contribution boundary — 2026-10-18

Collector's AccurateRip provider acquires reference responses; it does not submit
contributions. Comparison offsets/checksums are not optical-drive calibration or
secure-extraction attestations. The completed
[HL-08 investigation](accuraterip-contribution-investigation.md) recommends a
supported Windows ripper's physical-disc workflow. Future direct contribution
would first require provider support and application-owned eligibility, consent,
replay/recovery and self-match policy. Submission receipt state must not promote
audio-verification status. No new command, transport or durable schema is introduced.

### AccurateRip response selection — 2026-10-09

For a package with one eligible CUE/TOC anchor, scan planning derives the expected
AccurateRip query from current fingerprint-bound recorded structure (including
import analysis when no separate structure check exists). The application returns
that query and blocks a saved response whose track count or any of the three disc
IDs differs. This reads recorded metadata, never payload bytes. The check panel
filters saved responses by that complete identity, repairs obsolete saved choices,
and traverses snapshot pages before choosing a default. No matching response gives
fetch guidance. Multiple-anchor packages retain explicit selection and the worker's
strict comparison against the actual selected layout; planning does not guess from
a source-level observation. The worker still validates bytes and snapshot identity
before computing checksums; admission guidance is not audio verification.

AccurateRip readiness also exposes the current structural observation ID. Acquisition
accepts either a dedicated structure observation or one unambiguous layout inside
music import analysis, with the same module/schema, source-fingerprint, permission
and complete-layout validation. The frontend uses this typed eligibility rather
than searching its paginated history for a top-level layout. Missing responses are
ordinary planning blockers, retain anchor choices and never parse an empty UUID;
execution still requires a valid reference. When no usable recorded layout exists,
the panel offers the existing structure check before fetching.

### File-check progress — 2026-10-09

Processing-operation Jobs do not measure checksum/offset-search completion in the
admitted source-byte counter. Application presentation marks active work with
`JobProgressKind::Indeterminate`; measured workflows retain their counters. The
processing worker records actual phase transitions for source-byte validation,
module computation, final source recheck and result publication. Phase updates
preserve claim fencing and translate cancellation races to cancellation. The check
panel and Work history render unmeasured work without a false zero-percent meter.
Development builds optimize the shared disc/AccurateRip mechanisms; algorithm,
reference identity, offset coverage and validation rules are unchanged.


### Configurable dispatch — 2026-10-21

Startup `[jobs]` settings expose 1–8 processing workers
(default four), with an independent inbox-copy cap (default one, no larger than
the processing limit). Provider concurrency remains one. Application policy owns
validated limits and a bounded preference for ready Import analysis/publication;
Store enforces capacity, copy caps, fairness and existing claim fences atomically.
Server execution-loop count reads the same settings through the application port.
Invalid configuration must fail before opening storage. No stored format changes.

Prefer finishing a copied disc through analysis and original publication before
starting another copy, especially with one worker. This is a bounded scheduling
preference, not permanent worker ownership: provider waits, failures and optional
track preparation cannot pin capacity. Two processing admissions precede a copy
turn, with one ready publication allowed to finish the just-analyzed disc first;
multiple slots leave space for both waiting classes. The bounded preference stores at most eight account/import keys in memory; after
two preferred continuations, normal FIFO has an opportunity within processing.
Only ready work participates: no worker/claim is pinned across dependencies.
Ready metadata resolution (including applying an existing snapshot) outranks
optional encoding and ordinary processing. Metadata and ready-disc continuations
share at most two preferred admissions before an ordinary processing turn, so
alternating priority classes cannot starve older work. Copy turns and the ready-publication
exception still apply. Guided metadata begins once originals for its destination album group are
published; unrelated intake members do not block it. Text/album identity commits
before optional artwork. Accepted MusicBrainz album
identities feed automatic distinct-disc grouping under the 2026-10-22 policy above;
ambiguous/conflicting cases retain reviewed grouping proposals.
See [configuration](configuration.md#background-workers) and
[acceptance](validation/configurable-workers-2026-10-21.md).

### Concurrent processing and shared acquisition — 2026-10-10

The application owns provider equivalence and requester authorization. The private
store commits requester Job admission, acquisition owner links and command receipts
in one transaction. Internal provider Jobs have no assignable account identity;
requester Jobs and outcomes remain account-private. Collection, provider/
operation, normalized query, source fingerprint and response limits identify active
work. Item generations and metadata preferences belong to requesters. MusicBrainz
cache bypass is operational and does not split equivalent active work.

The execution loops share one process-instance identity and existing fenced
leases. Configured processing capacity (default four, range 1–8) plus one provider
slot determine both loop count and transactional admission. Inbox staging has a
separate cap (default one), including the older combined inbox-import command.
Bounded ready-disc priority and capacity reservation keep copying and processing
moving; inactive capacity is available to processing while copying remains capped.
Running jobs are not preempted. The application owns limits/preference policy;
Store enforces it under the claim transaction. Transient preferences may reset on
restart without changing pending Job identities or recovery. Waiting workflows
release processing claims. The specific pending
acquisition is durable, its outcome commits first, and an independent five-second
reconciliation loop visits all workflow pages even while processing is occupied.
Cancellation retains running capacity until work stops; existing subprocess group
termination/reaping and cancellable validation remain in force.

Provider byte spooling/parsing occurs outside the serialized archive writer. Under
that writer, equivalent AccurateRip/MusicBrainz responses reuse the retained snapshot
ID after cancellable length/digest validation. Identity includes media type, parsed
outcome and interpretation version, excluding acquisition time and cache bypass.
Requester-to-snapshot results commit before effects. Positive item/generation
bindings replace freshness inference from unrelated historical Jobs; each effect
still checks source, generation and authorization. Equivalent effective metadata
and resolution inputs return existing authority without another occurrence/revision.
Schema adoption attaches pre-existing queued/running provider requests
transactionally, preserving their immutable Jobs. Expired claims cannot attach
request ownership or consume acquisition results. Response comparison reads small
record pages with cancellation checks; it does not retain the response history in
memory. Converted legacy snapshots carry input identity through portable references,
allowing fresh response reuse after archive-only rebuild. Downloads never advance
audio verification dates. Admission and browse use recorded fingerprints, without
hashing payloads.

Existing response payloads are converted only by explicit CLI commands holding both
storage-root locks with the server stopped and a verified full backup. Portable
checksummed payload-reference sidecars preserve immutable envelopes and evidence
IDs. The private conversion journal makes publish-reference/remove-payload/reindex
steps resumable. See [the conversion runbook](snapshot-duplicate-conversion.md).
This is a provider-owned policy, not a general retention/workflow engine.


### Bounded provider retention — 2026-10-17

The [retention audit](retention-audit.md) records the current consumer/reference
assessment and supersedes blanket permanent equivalent-lookup history requirements.
Application-owned response reuse and stopped-server duplicate conversion now include
Cover Art Archive listings, images and negative responses alongside MusicBrainz and
AccurateRip. Exact collection/input/bytes/interpretation identity and validated reads
remain required. Changed responses retain distinct sources. Automatic CAA Asset
identity binds item and image Snapshot, independent of requester; a shared response
cannot give two items the same owned Asset. Existing Assets, metadata authority,
unknown data, historical envelopes and private retry/recovery state remain intact.
No general archive or operational-state garbage collector is introduced.

The 2026-10-18 review repair makes provider cover materialization an atomic
create/reuse effect under the archive writer, with the existing Item/Job fences.
Application policy freezes archived Asset revisions at explicit CAA image admission
through a bounded indexed projection read. Only those revisions may be restored;
a later archive invalidates an older queued lookup. Restoration publishes a new revision of the same Asset,
retaining the tombstone, original bytes, visibility and alternative text; automatic
refresh cannot restore archived artwork. Concurrent equal selections converge on
the already committed artwork choice without rewriting unrelated metadata.

### General document Assets — 2026-10-21

Document Assets reuse subject ownership, explicit shared/curator visibility,
immutable original publication, archive/reindex and authorized media downloads.
The selected initial formats are opaque PDF files and UTF-8 text, bounded by the
existing 20 MiB asset upload policy. Shared byte classification and bounded text
reader mechanics live in junk-libs-core; application services own allowed formats,
preview length and authorization. PDF signature recognition is not structural
validation: the preserved original is rendered by the browser's native PDF viewer
on explicit request. Text previews contain escaped plain text with typed truncation;
HTML is never interpreted. No PDF server renderer, invented thumbnail or metadata
verification claim is introduced. Documents cannot become album/game artwork.

### Optional backup-age reminder — 2026-10-21

An opt-in configuration supplies an age threshold and selects locally recorded
full-backup validation or an explicitly operator-reported external backup. The
application computes recent/due/failed/unconfirmed/unknown status; adapters read a
small receipt and render it in administrator Server settings and `doctor`. Reads
never enumerate backups or hash media. Full-backup attempts retain a prior confirmed
completion and record started/failed/completed outcomes; only final successful
validation advances the completion date. Abrupt interruption stays unconfirmed.
External reports are labelled unobserved and never called Collector validation.
This adds no scheduler, automatic backup, catalog-only backup or destination scan.

### Work navigation and filter layout — 2026-10-21

Work links record their immediate return URL in browser history alongside the
existing library depth. Activity owns its embedded Work view; normal Activity
work links never open a second modal. Back clears the selected detail when the
restored URL has no work target; reload and Forward reuse the recorded context.
Direct links keep explicit list fallbacks. Collection filters use a compact
searchable disclosure; normal-height controls align at the top and bulk commands
appear after selection. These are frontend observation/navigation changes, with
existing application authorization and polling unchanged. See
[acceptance](validation/configurable-workers-2026-10-21.md).
