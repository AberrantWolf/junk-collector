# Remaining work — home-lab priorities

Reassessed 2026-10-09 at the user's request. This is the **single authoritative
queue** for remaining work, including qualification and deferred ideas. It
supersedes assignment/order sections in earlier plans, the old serial waves and
older “required before first release” gap lists. Detailed plans remain design
hypotheses and evidence references; old proposed contracts require reassessment
and do not create separate queues. Obsolete backlogs and assignment prompts
have been removed; completed work is described by status and dated validation.
[Implementation status](implementation-status.md) records capabilities;
[release requirements](release-requirements.md) records the revised delivery gate.

Queue updated 2026-10-21 after public alpha.3 publication and isolated Portainer
update/cleanup/full restore, including HL-09 opt-in inbox cleanup and HL-10 full
portable archive export/import, plus HL-07A/B/C usability/setup acceptance and HL-05A
actual-host isolated install/update/restore, HL-05B snapshot/incremental recovery
qualification, HL-07D public alpha.2/Portainer acceptance and HL-05C active
preparation failure/recovery acceptance, HL-06A real-tool audio oracles, the numeric-IP
startup/storage repair, HL-05E bounded archive ENOSPC recovery and HL-05F bounded
authentication identity retention and HL-05G actual-NAS TCP login thresholds.
The 2026-10-19 native delivery and live-use triage remain recorded below.
HL-01, HL-02 and HL-03 remain delivered. HL-04's bounded physical Safari browser
journey passed; native/offline/failure qualification remains open. HL-07A/B/C are delivered. HL-07D public alpha.2 image/
pull/update acceptance is delivered; continue the remaining advertised-device,
format and permanent-ingress qualification when its required hosts/reference
material are available. NAS operations/recovery belong to HL-05A/B; Docker publication
and release usability belong to HL-07A–D. Keep the existing milestone numbers.
The earlier insertion assigned
digital purchases to HL-02, moved unfinished LAN qualification to HL-04 and former
HL-04–HL-13 to HL-05–HL-14. Dated completed records retain their original labels.

## Prioritization and execution

Optimize for one home server, a few trusted users, valuable originals, ordinary
LAN listening and optional authenticated Internet access. Deliver complete useful
workflows across layers. Preservation, recoverability and Internet-facing access
controls are essential even for one user. Keep working security controls; simplify
future scope rather than weakening authorization or TLS.

The numbered groups below are priority order, not a serial dependency chain.
Phono Junk is no longer a target. The bounded contract/retention reassessment is
complete (2026-10-17); HL-02 digital purchases are delivered (2026-10-19).
Continue with unfinished Group 1 qualification when its devices are available.
Unavailable hardware does not prevent independent backup or deployment work.
Group 2's public-access checks are required before describing an Internet-facing
installation as qualified. Group 3 is useful
when actual usage calls for it. Group 4 remains documented without blocking a
home-lab release. A concrete vulnerability or low-effort hardening fix moves ahead
of feature work regardless of group. No implementation or deployment is authorized
by this documentation consolidation alone.

Before implementing an entry, read AGENTS.md, architecture, current status and
its linked design/evidence; inspect the current checkout and affected sibling instructions.
Preserve credentials, grants, configuration, unresolved command journals, raw
masters, meaningful collection evidence, manual authority and unknown module data.
Apply [the revised scope/retention policy](decisions/home-lab-scope.md): old proposed
contracts are not mandatory, and repeated lookups are not collection provenance.
Do not delete existing records merely because their future retention is reassessed.
Use temporary installations for tests.
Complete the named outcome and its necessary dependencies; do not restart completed
roots or silently absorb optional features. Update this queue, the owning child
ledger, status and dated validation together. No commits or agents without an
explicit assignment. Apply repository gates at implementation boundaries; prose-only
changes need architecture/link/coverage checks.

## Placement of the 2026-10-19 live-use findings

This routes the new findings to their owning entries below; it is not a second
queue. Dated reports and workarounds remain in the
[NAS record](validation/synology-deployment-2026-10-19.md).

| Finding / requested outcome | Owning entry | Priority / disposition |
| --- | --- | --- |
| Missing anchor selector; grouping forces manual title; physical-copy label/defaults; NAS/macOS housekeeping in intake | HL-07A | Next complete import-review repair delivery; release readiness |
| Direct album and individual-track Play buttons | HL-07B | Bounded web listening usability delivery |
| Clear protection wording and removal of obsolete Phono instructions | HL-07C | Release copy/setup consistency |
| Fresh-install config, bind directories, inbox groups, provider readiness, Portainer/CLI image drift and orphan guidance | HL-05A | Reproducible installation/update operations |
| Routine updates without full archive duplication; local consistent snapshots and optional independent exports | HL-05B | Practical local recovery; external media copies optional |
| Automated public amd64/arm64 images, version/channel tags and tested pull updates | HL-07D | Explicit release gate |
| Optional cleanup of successfully imported inbox packages | HL-09 | Separate requested convenience with safe deletion/retry acceptance |
| NAS SFTP and SSH key exchange | HL-05 operator follow-up | Host administration; not a Collector release blocker |
| Grouped-disc revision conflict and repeated failed retry | Completed repair below | Keep regression coverage; remaining deployment acceptance stays HL-05 |

HL-07A/B/C and HL-05A/B's isolated operations/recovery deliveries are accepted.
Independent media backups are optional operator policy, not a publication prerequisite
under the 2026-10-20 scope clarification. HL-07D publication and isolated public-image update/recovery acceptance are delivered. Continue HL-04/HL-06 hardware and
format qualification when the required devices/reference material are available.
Do not wait for optional HL-09 cleanup or unrelated host administration to deliver
the core repair/release path.

## Completed live-use repair — grouped-disc publication, 2026-10-19

Disc 2 could fail Save originals after disc 1 added an Evidence reference and
advanced their shared album revision; retry reused the stale expectation.
The application-owned evidence-only revision rule and writer-fenced publication
repair are implemented, with regression tests for new publication, old failed
imports after restart/retry, original bytes/reindex, manual edits and unrelated
evidence rejection. The operator supplied verified backup/validation output
(401 files), reported successful retry after the image update at 23:13 JST,
and working playback at 23:15 JST. See [repair evidence](validation/grouped-disc-publication-2026-10-19.md).
Do not schedule the repair again or treat its recovery as still unimplemented.
Remaining final inventory/metadata/seek/original-download/restore checks are part of
HL-05's deployment acceptance, not a reopened implementation assignment.

## Completed requested delivery — automatic standard games, 2026-10-10

The explicit user assignment for automatic standard-game identification is delivered
across shared mechanisms, Collector services/storage/API and the real web workflow.
See [supported matrix](game-identification.md), [status](implementation-status.md#automatic-standard-game-identification--2026-10-10)
and [dated validation](validation/game-identification-2026-10-10.md). This supersedes
older descriptions requiring manual system/profile selection in the ordinary game
catalog workflow. No second remaining-work queue is created.

New format/normalization algorithms, arbitrary custom systems/conventions, game
artwork providers and provenance compaction remain deferred under existing scope
and HL-01/HL-06/HL-09. The finite matrix does not claim all conceivable standards.
That delivery originally pinned shared-library revision
`83177af57ca36a5b284298020fea9d5e8fb0c415`. HL-02 subsequently advanced the exact
published dependency pins; manifests/lockfiles and its dated validation record
are current. No local Cargo patches or sibling checkouts are required.

## 1. Finish the usable installation and protect the collection

### HL-01 — Simplify contracts and retain meaningful provenance

**State:** the bounded retention audit and justified CAA response/Asset simplification
are delivered 2026-10-17, with the concurrent-creation and archived-cover reacquisition
review repairs implemented 2026-10-18, including admission-bound restoration that
respects later archive decisions. See [audit and policies](retention-audit.md),
[initial validation](validation/retention-audit-2026-10-17.md) and
[repair validation](validation/retention-review-2026-10-18.md). Equivalent CAA
listings/images/negative responses share validated snapshots; automatic covers
belong to item + image Snapshot, independent of requester. Existing duplicate CAA
payloads use the explicit stopped-server conversion. No live archive was pruned.
The audit retires blanket permanent equivalent-lookup history. General deletion of
revisions/candidates/occurrences/Assets, private receipt/log expiration and upload-spool
reclamation remain unimplemented, conditional on actual growth/need and exact
reference/replay rules. They do not reopen the completed bounded assignment.
The earlier processing/request-sharing delivery remains accepted; see its
[validation](validation/concurrent-processing.md) and the
[stopped-server conversion](snapshot-duplicate-conversion.md).
**Maps:** A04/P01/D06/FX-02 and old history requirements. **Owner:** application
policy, module evidence semantics and archive/private-state adapters.

The completed [audit](retention-audit.md) defines the minimal owned policies and
maps actual consumers: originals, inventory/manual facts, accepted sources,
substantive evidence and unknown data are retained; equivalent provider bytes
and current projections/cache state do not require duplicate permanent copies.
Inspection reports exact conversion effects. Verified backup, interruption-safe
conversion, current reads, reindex, archive-only rebuild and private retry/recovery
remain required. Existing envelopes, choices and meaningful dates stay intact.

Broader deletion remains conditional future work under this entry. Before touching
revisions, candidate/occurrence history, Assets, receipts/logs or upload spools,
establish a demonstrated need, complete reference/replay rules, exact dry-run
reporting and tested backup/recovery. A current graph index or a completed Job is
not proof of safe deletion. Do not invent a generic retention engine. No deletion
of originals as routine history cleanup, and no permission to prune the user's
live archive is implied. See [scope and retention](decisions/home-lab-scope.md).

### HL-02 — Complete FLAC and MP3 digital purchase support

**State:** delivered and published across Collector, junk-libs and Trash Amp,
2026-10-19. No implementation or acceptance work remains in this milestone.
See the [supported matrix and implementation record](digital-purchases.md),
[current status](implementation-status.md#digital-flacmp3-purchases--delivered-2026-10-19)
and [dated acceptance and publication checks](validation/digital-purchases-2026-10-19.md).

The delivery preserves exact DRM-free FLAC/MP3 originals, including high-resolution
FLAC, mono/stereo MPEG-1/2/2.5 MP3 and mixed/multi-disc packages. Digital acquisition
creates no physical copy; purchase facts and reviewed file visibility survive
recovery. Source-bound tags/artwork, manual track edits, explicit alternate-encoding
selection and immutable file identities feed normal browse/search, direct original
playback, queues and repeated playlist occurrences without an encoder/provider.
Trash Amp streams, seeks, downloads and retains artwork/text/search/playback offline.

Independent codec/tag/malformed-input vectors, privacy and recovery regressions,
real Chrome and actual macOS GUI/device-callback journeys passed. The native GUI
also passed connection removal and server-stopped offline reopen/playback/seek.
All three repositories are pushed on `codex/hl02-digital-purchases`; exact sibling
revision pins and lockfiles replace local Cargo patches. An isolated native source
export passed its gates without the unrelated staged release work or sibling
checkout fixtures; full affected workspace, lint and browser checks passed again.

**Remaining scope elsewhere:** HL-04 owns actual second-device LAN/trust/failure
qualification; HL-08 owns the broader native reliability matrix; HL-06 owns
outstanding advertised-format/audio oracles. AAC/ALAC/Opus originals and other
codecs, storefront/archive intake, DRM and album-image/CUE splitting remain deferred
under HL-13. MP3 decode/timing support does not certify sample-accurate gapless or
bit-perfect output, and descriptive tags never assert CD/catalog verification.
Do not reopen the accepted FLAC/MP3 workflow as a dependency of those follow-ups.

### HL-03 — Make stopped-server backup and restore dependable to operate

**State:** delivered 2026-10-18. The local operator CLI supplies `backup`,
`validate-backup` and empty-root `restore`, plus explicit `--archive-only` recovery.
The shared conversion/general backup locks both roots, settles known journals,
checkpoints SQLite and includes complete archive/private state, TLS/config and
pending scratch. A sealed inventory/completion marker and copied-file revalidation
precede restored database opening. Interrupted restores are fenced; live locks,
corruption, unsafe paths and overlays fail closed. Full recovery preserves identity;
independent recovery uses fresh private identity and explicit grants. External
inbox/DAT/tool paths and restrictive Unix permissions are reported.

Review repairs on the same date add real stopped-server discovery and first-Owner
assignment (`unowned-collections`, `recover-collection-owner`), preserving existing
Owners, disabled-account restrictions and ordinary web authorization. Missing
original lockfiles no longer prevent recovery from otherwise lost source roots;
normal live locks and unsafe aliases still refuse restore. Compiled-CLI regression
workflows replace the earlier storage-only Owner assignment shortcut.

See [operator workflow](local-alpha-installation.md#stopped-server-backup),
[architecture](architecture.md#stopped-server-backup-and-recovery-contract--hl-03)
and [dated acceptance](validation/backup-restore-2026-10-18.md). Temporary-root
adapter and compiled-CLI drills cover exact originals, unknown data, loans,
manual authority/evidence, accounts/grants/TLS and resumed partial imports. The
historical LA3 installation drill remains dated evidence. No live archive was
changed. Linux/NAS and actual-host restore qualification remain HL-05. Online
backup, scheduling and a dashboard are not required; portable export/deltas remain
HL-10. Do not reopen this tooling delivery as a new remaining task.

### HL-04 — Complete web and Trash Amp LAN qualification

**State:** web and Trash Amp workflows implemented; external acceptance incomplete.
HL-02 already qualifies actual Chrome and macOS FLAC/MP3 playback/seek/transitions,
queues, native explicit downloads, connection removal and offline reopen with
artwork/text/search. Reuse its [named-client evidence](validation/digital-purchases-2026-10-19.md);
it does not establish system-trusted enrollment, a second LAN device or failure
behavior beyond the cases actually tested.
**Maps:** remaining relevant LA3 and C01/C04/C07 checks. **Owner:** application,
transport and Trash Amp presentation; fixes belong in their owning layers.

2026-10-20: physical Safari on the operator's iPhone 16 Pro (reported iOS 26.6.2)
passed trusted certificate, artwork, FLAC/MP3 playback/seek, persisted Curator
track editing, Viewer control denial, grant-revoked playback/loaded-player retry,
and session revocation/fresh sign-in against the isolated NAS. Independent API
write/media/range/HEAD/session denials passed. Fixture cleanup passed and live
services were unchanged. Parent HL-04 remains partial for the cases below. See the
[dated acceptance and limits](validation/lan-device-preparation-2026-10-20.md).

Use [the installation runbook](local-alpha-installation.md), Collector's
[Trash Amp status](implementation-status.md#trash-amp-client-integration-2026-10-07)
and sibling `docs/collector-connection.md`. Finish:

- Trash Amp system-trusted HTTPS enrollment/browser approval and persistence;
  multiple servers/accounts, cancelled or interrupted enrollment and switching
  during playback or uncertain edits. Reassess old native matrix cases against
  the actual Trash Amp implementation before adding new requirements.
- Native playlist command response loss/restart is now qualified in source:
  actual TLS create/append commits, truncated responses, atomic pending-input
  binding, profile reopen and exact replay without duplicate playlists/entries.
  See [bounded acceptance](validation/native-failure-recovery-2026-10-21.md).
- Actual native uncached-read grant and device-credential revocation is also
  qualified in source with a distinct Viewer: a real chunk succeeds before the
  permission change, and the next uncached read is denied. Reuse the same
  [bounded acceptance](validation/native-failure-recovery-2026-10-21.md); buffered
  bytes and retained offline copies are not recalled.
- Bounded actual native transport faults are qualified: changed/ignored/truncated
  range rejection without partial publication, successful read/seek recovery,
  cancellation during header/body stalls, and wrong pinned-TLS identity rejection
  while retaining the complete catalog. See the same
  [acceptance](validation/native-failure-recovery-2026-10-21.md).
- Exercise remaining native UI playback/transitions, seek and queue continuity
  through output-device loss and transport failures on the actual supported
  client. Reuse the accepted transport, revocation and playlist recovery cases
  above; they do not establish physical output behavior or every UI transition.
- Reuse the qualified actual second-device Safari journey above; do not repeat
  its trusted browse/artwork/playback, Curator/Viewer or grant/session checks.
  Verify Internet-disconnected LAN use on the actual client. Use Trash Amp on a
  second Mac only for claims about that native platform; a web-device gate need
  not require a Mac. Verify remaining supported native download/failure behavior
  on its actual client.

**Done when:** the remaining relevant cases have reproducible reports naming host,
browser/client, trust mode and output device. LA3 stays partially qualified without
its required physical-device evidence. Reuse passing corpus/provider/restore work
unless affected code changes. Phono's PJ1/account GUI and second-Mac acceptance
are retired, not passed, and do not block anything. Shared Collector/account/TLS
mechanisms remain useful; do not remove working services just to retire a client.

## 2. Secure and qualify the actual home-server deployment

### HL-05 — Linux, NAS and optional public-Internet deployment

**State:** Linux ARM64 Docker Desktop runtime/persistence passed locally;
[Synology x86-64 import/download, recreated-container persistence and full backup/restore](validation/synology-deployment-2026-10-19.md)
passed on the earlier test installation. The permanent Portainer/Cloudflare setup
now has operator-reported browser access, grouped-import retry recovery and playback,
plus captured successful full backup/validation output. The SIGTERM source fix,
Compose package and release workflow exist; public image publication and isolated
Portainer pull/update acceptance are delivered under HL-07D. HL-05C additionally
qualifies isolated active-encoding cancel/failure/crash/full recovery with real
browser controls. HL-05E adds bounded tmpfs archive ENOSPC/restart/full recovery;
HL-05D's permission-loss repair and numeric-IP startup now pass actual-NAS candidate
acceptance.
Remaining current-installation, broader storage-failure/abuse and
public-ingress qualification stays open. Reuse the dated evidence without treating
the earlier test install as acceptance of every permanent-installation path.
**Maps:** R02/R03
practical deployment, S01–S03 external qualification. **Owner:** release/server
adapters and focused security review.

Qualify the intended Linux binary or non-root container on the actual host, with
explicit archive/state/scratch mounts, local SQLite and optional NAS payloads.
Exercise restart, interrupted publication/upload/encoding, missing optional tools,
storage permissions and backup restore there. Record the filesystem and failure
model; local process-kill tests are not power-loss/NAS certification.

For Internet access, execute one intended trusted-HTTPS proxy/tunnel configuration
end to end. Public ingress must reach only the public listener; private management
uses explicit private LAN/tailnet ingress. Curation/admin stay publicly disabled by
default, and may be explicitly enabled as in the operator's existing installation.
Reuse source/isolated tests for exposure combinations, role/Origin/session controls,
recovery, revocation and redaction. A short smoke at the intended hostname checks
trusted TLS, the normal sign-in/playback/seek journey and anonymous API denial;
reuse operator-observed steps already working there. Exercise another authenticator
when it is actually part of the intended workflow. Keep platform claims limited
to the clients exercised, without a new tunnel or exhaustive proxy matrix.

2026-10-21: the operator selected the existing Cloudflare hostname and requested
proportional qualification. Normal trusted Chrome loaded its sign-in page without
script errors; same-origin anonymous account/admin API reads returned Collector's
typed 401 responses. The configured public administration is intentionally enabled,
consistent with the earlier deployment record. Existing operator-reported browser
access/playback remains useful evidence. See the [lightweight live check](validation/synology-deployment-2026-10-19.md#lightweight-live-https-check--2026-10-21).

Risk-driven follow-ups include account/peer/global rate buckets, process-restart reset behavior,
unauthenticated enrollment/challenge floods, slow/oversized uploads and ranges,
provider/parser CPU/memory bounds and Job cancellation under realistic home-server
load. Keep forwarded headers untrusted. Fix demonstrable abuse gaps; distributed
limiting is unnecessary for a single process. Reuse accepted HL-05C–G checks;
do not repeat them through the public tunnel. Broader stress/failure cases are
follow-ups unless a concrete deployment failure makes them necessary. Document operator recovery and
certificate renewal/re-enrollment for the chosen ingress. Obtain focused review
of exposed authentication, upload/media and listener boundaries; broader protocol
certification is HL-14.

**Done when:** the recorded installation supports its ordinary browse/listen flow
over trusted HTTPS, practical recovery and expected access denial, using the
combined deployment/operator/source evidence. Public qualification is a
gate for supported public use, not for a LAN-only release. A commercial deployment
matrix, automatic route switching and every browser/platform are unnecessary.

#### HL-05A — Reproducible NAS and Portainer operations

**Numeric-IP repair, 2026-10-20:** source now starts with HTTPS IP authentication
origins using password/recovery, reports typed passkey hostname requirements, and
validates malformed origins before setup/startup writes. Actual executable TLS/
identity/session continuity and Chrome login/guidance/recovery passed; see
[repair acceptance](validation/numeric-ip-authentication-2026-10-20.md). Published
alpha.2 still predates this repair; retain the dated
[original failure](validation/lan-device-preparation-2026-10-20.md#newly-observed-numeric-ip-startup-failure).

**State:** delivered 2026-10-20 for the isolated actual-host operations path.
Guarded config transfer, DSM bind-root creation, inbox supplemental groups,
provider readiness and Portainer/CLI image-alignment guidance are shipped. Actual
DS1522+ Btrfs/amd64 tests passed a distinct-image update with pending-upload resume,
account/session/config/TLS persistence, inbox import and explicit FLAC, exact original
downloads, clean SIGTERM and full restore. Remote Chrome passed playback/seek/downloads
before and after restore. See [dated acceptance and limits](validation/nas-operations-2026-10-20.md).
The permanent installation/tunnels were not changed. Its final inventory/enrichment
checks and public-ingress qualification remain under parent HL-05. Registry pull
acceptance is delivered under HL-07D; HL-05B owns the qualified scalable recovery path.
**Owner:** process/container adapters and operator documentation.
Completed delivery contract:

- Create bind source directories before initialization; DSM rejected a missing
  source. Resolve or explain the failed proxy-config copy step: initial setup
  still advertised localhost with administration disabled, while explicitly
  writing the actual configuration corrected it. Configuration correction must
  not reset accounts.
- Document inbox share permissions and supplemental container groups, distinguishing
  host file access from the bind mount's read/write setting. Read/write permission
  does not implement source cleanup; that requested feature belongs to HL-09.
- Keep the Portainer runtime image and SSH maintenance-tool image aligned across
  updates. Explain why the CLI-only Compose file calls the Portainer-managed
  tunnel connector an orphan, and do not advise removing that connector. Preserve
  existing bridge-network tunnel routes when composing the host-local connector.
- Make MusicBrainz contact configuration and actual provider readiness explicit
  before promising automatic metadata/artwork. A monitored contact is required,
  not a MusicBrainz account. Qualify doctor/readiness, import and the expected
  unavailable-provider behavior. Coordinate wording with HL-07C.

**Acceptance:** a fresh installation and later pull/update follow the instructions
without ad hoc config repairs, missing mount paths or UID/group failures. Accounts,
configuration, originals and pending imports are not overwritten. Complete the
remaining actual-host checks: final disc/copy inventory, metadata/artwork, explicit
seek, original download comparisons, restart persistence and a restored-instance
drill. Working access/playback alone is not full Internet qualification.

#### HL-05B — Scalable update backups and recovery

**State:** isolated operations path qualified 2026-10-20; no independent-destination
selection is required to continue release work. The actual Collector root is one
Btrfs subvolume without nested roots. Short whole-root cold snapshots and the
optional GNU tar baseline/increment path passed actual NAS→Mac→NAS restore,
complete cold file/permission comparison, SIGKILL/restart, old-image whole-point
rollback/pending-upload resume and remote Chrome playback/seek/downloads. See
[operator procedure](../deploy/container/snapshot-recovery.md) and [dated acceptance](validation/snapshot-recovery-2026-10-20.md).
No production configuration or backup schedule was changed. Btrfs incremental receive
failed in the isolated mount and is not an accepted route.
**Owner:** deployment/recovery integration over the existing application backup
contract.

Apply the [clarified local recovery scope](decisions/home-lab-scope.md#local-recovery-and-optional-media-backups--clarified-2026-10-20):
make updates practical for 1,000+ albums without copying/rehashing all unchanged
masters. Use a complete consistent local point over archive, required private
state/scratch and config; Btrfs eligibility and boundaries are qualified on the
actual host. The packaged local backup area is `${COLLECTOR_DATA}/backups`.
The existing explicit full-backup command includes media; it is not a small
catalog/database backup or a scheduled job. Independent exports/copies remain
optional using existing tools/storage. Neither cloud payment nor selecting a
second storage destination blocks publication or ordinary updates.

Keep per-build storage/upgrade notes and the old image with its complete recovery
point. Image-only rollback is unsafe after incompatible writes. Full restore and
interrupted-update recovery remain required evidence; their accepted tooling and
qualification are not reopened. Local points cover rollback, not loss of the
storage device. This does not implement HL-10 portable export/deltas.

**Optional reminder delivered in post-alpha.3 source, 2026-10-21:** administrator
Server settings and `doctor` report the age of a known full-backup validation
receipt, preserving failure/unconfirmed/unknown status. Explicit external reports
are labelled unobserved. Reads use one bounded receipt, with no backup enumeration,
media hashing or archive scan. Configuration is opt-in; no scheduler or automatic
backup is added. See [acceptance](validation/backup-reminder-2026-10-21.md).
A future lightweight catalog/settings backup needs a separate bounded restore
contract covering archive-owned metadata/evidence/assets and required private state;
copying SQLite alone is insufficient. No new backup subsystem or scheduler is assigned
by this clarification.

#### HL-05C — Actual NAS preparation interruption and recovery

**State:** qualified 2026-10-20 on the published alpha.2 image. Actual encoding
cancellation/reaping, failure after real FLAC output, and SIGKILL/restart passed.
Cancellation stayed terminal across restart and fenced generic retry; explicit UI
retry and crash recovery preserved original Job identity/creation time and published
one playable output. Original bytes, recovered batch/audio identities and session/
config/TLS survived full backup/restore; actual Chrome cancel/retry/play/seek/download/
mobile journeys passed. Live Collector/Portainer/tunnels matched their baseline;
fixture cleanup passed. See [acceptance](validation/nas-preparation-recovery-2026-10-20.md).
**Owner:** existing application Job/recipe policy, process/storage adapters and web
recovery controls. Qualification uses controlled external-tool faults in generated
roots; no runtime image or production configuration was changed. Parent HL-05's
public ingress, permission/storage-full/abuse and arbitrary commit/power-loss cases
remain separate. Do not reopen accepted HL-05A/B/Docker publication as dependencies.

#### HL-05D — Storage permission failure and metadata recovery

**State:** source repair and actual NAS candidate/full recovery accepted
2026-10-20. Two isolated published-alpha.2 runs found that a failed metadata save on
a read-only archive, followed by a successful retry, could block restart/cold backup
with a stale command-marker digest mismatch. Original bytes remained intact.
The storage adapter now releases proven absent reservations under the writer lock
and resolves older losing metadata reservations only with an independently attested
winner. Unexplained corruption still rejects recovery. See [dated repair evidence](validation/storage-permission-recovery-2026-10-20.md).
**Owner:** existing storage command/publication adapter and qualification harness.
The unpublished native candidate passed read-only Btrfs archive, real browser retry
without an intervening restart, cold backup and full restore at a numeric-IP HTTPS
origin. Exact originals and session/identity/TLS/manual title survived. Generated
resources were cleaned and live services matched baseline. Public alpha.2 remains
unchanged and predates the repair. Broader storage failures, public ingress/abuse
and power-loss claims remain separate.

#### HL-05E — Bounded archive capacity failure and recovery

**State:** qualified 2026-10-20 on published alpha.2 using an isolated actual-NAS
96 MiB tmpfs archive. Real ENOSPC produced a visible failed browser save without
changing committed metadata or exact originals. Freeing only the fixture filler,
normal restart and browser retry succeeded; full backup/restore retained session,
identity/TLS/manual title/prepared audio and original bytes. Real Chrome playback,
seek, downloads, persistent player and mobile checks passed throughout. All generated
volumes/keepers/data/secrets were cleaned and live services matched baseline. See
[acceptance and reproduction](validation/storage-capacity-recovery-2026-10-20.md).
**Owner:** bounded qualification adapters; no runtime change. This does not certify
a physically full Btrfs/private SQLite filesystem, inode exhaustion, power loss,
arbitrary publication interruption or permanent/public deployment. Keep these limits
and reuse the accepted bounded case instead of repeating it.

#### HL-05F — Bound authentication limiter identity retention

**State:** source accepted 2026-10-20. Authentication/enrollment/poll checks formerly
inserted identity buckets before rejection and retained inactive identities across
windows. Rejected alias rotation can no longer grow these maps; expired empty
buckets are reclaimed and current identities are bounded by globally admitted work.
Existing account/peer/global thresholds and direct-peer selection stay intact.
Deterministic-clock flood/window regressions, actual public/private TCP listeners
with spoofed forwarded headers, existing-session continuity and process-restart
reset passed. All 443 workspace tests, strict Clippy and full Chrome smoke passed.
See [acceptance](validation/rate-limit-retention-2026-10-20.md).
**Owner:** existing process-local HTTP abuse adapter. No image/public ingress was
deployed. External multi-client/proxy load, broader enrollment/media abuse and
permanent deployment qualification remain under parent HL-05; do not infer them
from the bounded memory/loopback source acceptance.

#### HL-05G — Actual NAS TCP authentication thresholds

**State:** qualified 2026-10-20 on isolated published alpha.2. Real Linux loopback
source addresses independently exercised account (10), direct-peer (30) and global
(100) login ceilings across public/private listeners. Rotating account names and
forwarded headers did not bypass the peer limit. Existing sessions remained usable;
restart reset transient limits without changing the account/session. Full backup/
restore and Chrome playback/seek/exact downloads/persistent player/mobile passed.
Generated resources were cleaned and live services matched baseline. See
[acceptance](validation/nas-authentication-limits-2026-10-20.md).
**Owner:** bounded qualification adapter; no runtime change. Distinct loopback TCP
addresses are not distinct physical clients or a public proxy/NAT. External ingress,
enrollment/media abuse, sustained load and physical-device claims remain separate.
The published image still predates HL-05F's memory-bound source repair.

#### HL-05H — Reject untrusted upload callers before body reads

**State:** delivered in source 2026-10-21. Runtime curation/authentication and
administration/application-role guards run before body extraction. Handler and
collection authority checks remain current. The reproducing body-poll regression,
32 HTTP tests, full workspace/Clippy gates and actual Chrome workflows passed.
The focused listener/authentication/upload/media review and limits are in the
[acceptance record](validation/ingress-review-2026-10-21.md); the
[capability checklist](release-readiness.md) consolidates existing release evidence.
This source repair is after alpha.3. No live service changed.

#### NAS administration follow-up — not a product release gate

Investigate supported DSM updates/settings to enable SFTP on the DS1522+
(`fatchocobo.local`). Modern `scp` failed with `subsystem request failed on
channel 0`; `scp -O` successfully transferred the images. Assess missing
post-quantum SSH key exchange separately from SFTP availability. This operator
task does not block Collector deployment or require a Collector code change.

### HL-06 — Qualify advertised preservation, verification and audio support

**State:** implementation and independent synthetic vectors exist; coverage gaps
remain. **Maps:** D02/D04/D05/D07 and R03 real oracles. **Owner:** shared libraries
for mechanisms, modules for semantics, application/adapters for work and evidence.
The initial FLAC/MP3 original-file matrix, bounded tag/picture parsing, native
rates/timing/seek and named-client playback are accepted under
[HL-02](#hl-02--complete-flac-and-mp3-digital-purchase-support). Retain those vectors
as regressions; do not repeat that delivery or infer broader fidelity/gapless,
CD/AccurateRip or unsupported-codec coverage from it.

Maintain a per-operation supported/unsupported/externally-unqualified matrix.
Use owned or legally supplied independent references for the formats actually
advertised: cartridge normalization, ordered game tracks, CD structure and
AccurateRip v1/v2/offset/coverage, and FLAC PCM equality. Live MusicBrainz/CAA and
one owned Redump PlayStation complete match already passed; these do not qualify
all formats or audio checks. Live AccurateRip/whole-disc owned audio verification
remains open. Preserve partial/inconclusive results and exact computation coverage.

**HL-06A delivered 2026-10-20:** actual NAS real FLAC integer PCM equality and
configured Opus 96/128/192 kbps independent decode/frame/channel checks passed on a
known non-silent stereo fixture. Real Chrome preparation/download/Opus-only playback/
seek and full-restore repetition passed. The [current audio matrix](audio-preparation.md)
and [dated evidence](validation/audio-oracles-2026-10-20.md) bound these claims to the
named tools, source and client. Do not repeat this qualification or infer multi-track/
gapless, native Opus, original-codec expansion or owned-disc verification from it.

CHD runtime, trainer-bearing NES and other unsupported/ambiguous layouts remain unavailable
until their named independent vectors/oracles and bounded readers pass. Additional
owned NES/N64/SNES/PC Engine matches require the actual dumps and matching catalogs;
they are not prerequisites for a PlayStation/music home-lab delivery. Use manually
supplied DATs, never retry blocked DAT-o-MATIC downloads or restore scraping.

**Done when:** each advertised claim has independent evidence and explicit limits,
including truncation, aliases, smallest inputs, interrupted work and changed
snapshots. Metadata identity, integrity and structure never masquerade as catalog
or whole-disc audio verification. Expand formats only in response to actual use.

### HL-07 — Release and maintenance acceptance

**State:** public experimental alpha.3 is published; no supported home-lab release
baseline is declared.
**Maps:** bounded R03, release-toolchain maintenance and independent UI review.
**Dependencies:** accepted HL-01–HL-03 capabilities plus remaining HL-04–HL-06
checks only for the platforms/capabilities advertised; safe backup/recovery remains
mandatory. **Owner:** integrated release review.

#### HL-07A — Repair multi-disc inbox import review

**State:** delivered 2026-10-19. Shared anchor replanning, independent automatic/manual
album titles, explicit physical-copy choices and narrow housekeeping exclusion are
implemented through discovery, review and staging. Real Chrome acceptance covers
automatic artwork/title, separate copies, exact original downloads, privacy, draft
reload, restart and mobile/keyboard review; Rust regressions preserve failed retry,
reindex and older accepted inventories. See [dated acceptance](validation/intake-review-2026-10-19.md).
No live NAS data was changed; deployment qualification remains HL-05 and source
cleanup remains HL-09. The completed delivery contract follows for reference.
**Owner:** application intake/grouping policy, module readiness, filesystem adapters
and web review, with the existing [bulk delivery contract](bulk-library-plan.md).
Deliver one complete inbox → review → grouped import → automatic metadata workflow:

- Expose the existing guided flow's CUE/Redumper anchor selection in batch review.
  Replan through the shared readiness implementation without losing file visibility,
  grouping or other choices. Choosing an anchor must not require removing either
  package view; moving generated CUE/BIN files was only the operator workaround,
  not the required user workflow. No new permanent-storage requirement for
  redundant derivatives is introduced.
- Separate album group identity from manual title authority. Two discs can belong
  to one album while its title stays automatic; an explicitly entered manual title
  remains authoritative. Do not require importing one disc first just to avoid
  a forced title override.
- Replace/explain "Physical set" as a physical-copy grouping choice, not a disc
  number. Matching labels within an album/intake mean one purchased copy; distinct
  labels mean separately owned copies. Remove unexplained per-package numeric
  defaults in favor of reviewed inventory intent. Album group, physical copy and
  disc positions stay distinct; do not silently regroup existing inventory.
- Ignore only known new-intake housekeeping: `#recycle` and `@eaDir` directories
  and nested contents, `.DS_Store`, and Synology metadata streams under `@eaDir`.
  Apply the same narrow policy to inbox chooser, preview and actual staging.
  Meaningful hidden/unknown files still round-trip. Do not delete source files or
  prune already-saved archive records.

**Acceptance:** real web UI imports a two-disc CUE/BIN + Redumper package set as
one album/one explicitly chosen physical copy with automatic title/artwork when a
matching provider result exists. Verify separate duplicate copies, explicit manual
title authority, partial failure/retry/restart, unchanged raw originals and unknown
files, and housekeeping-free new intake. Keep the grouped-publication conflict
regressions; do not manufacture metadata success when no provider match is found.
Cleanup of source directories is outside this delivery.

#### HL-07B — Direct album and track playback actions

**State:** delivered 2026-10-19. Album/track Play and explicit per-import Play
reuse existing authorized playback scope and atomically replace the personal queue.
Real Chrome qualifies multi-disc order, direct FLAC/MP3 originals, unprepared and
ambiguous selections, explicit encoding choices, seek/next, obsolete-response fencing
and a request denied by actual grant revocation. See [dated acceptance](validation/direct-playback-2026-10-19.md).
The completed delivery contract follows for reference.
**Owner:** web presentation over existing authorized listening/queue services.

Provide clear **Play album** and individual-track **Play** buttons where albums
and tracks are presented. Album play uses numeric disc/track order; track play
starts the selected track. Reuse the persistent player and queue for prepared CD
audio and supported digital originals. Explain unavailable/unprepared audio and
require explicit source choice for ambiguous alternate encodings/imports. Play
must not silently start encoding. Make queue replacement/continuation behavior
clear. Exercise multi-disc album play and individual tracks through the real web
UI, including seeking, next-track behavior, unavailable audio and permissions.
This adds access to working playback; it does not rebuild the player or reopen HL-02.

#### HL-07C — Clear release and setup wording

**State:** delivered 2026-10-20. CLI startup/doctor, enrollment, certificate help,
README and operator instructions now describe actual web/Trash Amp controls and
precise saved-file/protection/reset guarantees. Historical evidence and technical
identifiers are unchanged. Single/multi-account pairing, trusted-PEM download/TLS,
mobile help, full browser smoke and repository gates passed. See [dated acceptance](validation/release-wording-2026-10-20.md).
**Owner:** CLI/web presentation and documentation; executable NAS setup acceptance
remains HL-05A. The completed delivery contract follows for reference.

Remove obsolete Phono-specific setup/doctor/startup advice for the retired client.
Use "not affected", "won't be overwritten", or a precise equivalent when explaining
protection of existing configuration, credentials, manual choices or other data.
Use "kept" or "saved" for stored files. Match wording to the actual guarantee;
do not promise that every setting is unaffected by every update. Technical
retention policies and stored identifiers do not require renaming. Verify served
UI, CLI output and instructions agree with current web/Trash Amp behavior.

#### HL-07D — Automated pullable Docker releases

**State:** delivered 2026-10-20. Signed alpha.2 source, public AMD64/ARM64 version
and `preview` images, anonymous both-platform/source/digest checks and digest-pinned
installation assets are published. Native builders replace the slow QEMU build;
normal manual builds remain nonpublishing, and an explicit existing-release retry
freezes the tagged source. Actual disposable NAS Portainer old→public update,
unchanged companion, fresh digest install/forced re-pull, pending upload, original
bytes/session/config/TLS/full restore and remote Chrome passed. Live Collector,
Portainer and both tunnels matched their original IDs/images/start times; fixture
cleanup passed. See [publication acceptance](validation/container-publication-2026-10-20.md)
and [alpha.2 notes](releases/v0.1.0-alpha.2.md). Earlier
[preparation evidence](validation/container-release-preparation-2026-10-20.md) remains dated.
Do not reopen packaging implementation or these accepted isolated drills. This is
an experimental alpha; parent HL-07's remaining advertised-capability qualification
is not implied complete.

Follow-up release, 2026-10-21: signed public **alpha.3** packages the post-alpha.2
fixes, full archive transfer and opt-in cleanup on qualified Rust 1.99.1. Native
AMD64/ARM64 cleanup/recovery smokes, anonymous pulls/index/preview checks and the
actual-NAS Portainer alpha.2 → alpha.3 update/full restore passed. Production stayed
unchanged. See [alpha.3 acceptance](validation/container-publication-2026-10-21.md)
and [release notes](releases/v0.1.0-alpha.3.md).

**Owner:** release CI/container packaging with HL-05 deployment acceptance.

Release packaging requirement added 2026-10-19: users must be able to install and
update by pulling published Docker images, without building source or exporting,
transferring and loading image archives. Complete and exercise the existing
`.github/workflows/container-release.yml` rather than introducing a second build
pipeline. It already gates release-triggered GHCR builds on checks, performs a
container smoke test, targets linux/amd64 and linux/arm64, and prepares the install
bundle. The fulfilled contract includes a documented moving release/channel tag
for convenient updates (`latest` for stable releases, with prereleases kept on an
explicitly named channel), alongside version tags and
recorded digests for reproducible recovery. Verify anonymous pulls, both platform
manifests, release version/source labels, and a real Portainer pull/redeploy update
that preserves configuration, credentials, originals and unfinished work. Document
the registry image, channel choice and update/rollback procedure, coordinated with
HL-05's scalable backup work. Published and tested pull-based installation/update
is required before calling the product released; an unexercised workflow file or
a source tag alone does not meet this requirement.

#### Final release acceptance

Publish a capability/evidence checklist for the chosen web/Trash Amp scope,
installation/upgrade/backup instructions and explicit unsupported behavior. Run
repository gates, rebuild embedded assets, exercise actual setup/import/metadata/
listen/list editing and targeted keyboard/mobile/accessibility review. Include CD
preparation and direct FLAC/MP3 originals in the chosen scope; playable digital
originals must not require preparation. Reuse unaffected HL-02 acceptance evidence.
Check current stable Rust with [the maintenance procedure](release-toolchain.md)
before each release; the October upgrade is done, not a recurring feature ticket.
Track affected sibling lint findings only when touching or qualifying those crates;
do not silently modify sibling projects to clear unrelated warnings.

**Done when:** every claimed capability links to tests or actual manual evidence,
residual defects have scoped entries, and any supported API/storage/upgrade promise
is explicitly declared under [pre-release evolution](decisions/pre-release-evolution.md).
An experimental source tag is not full acceptance. Published Docker images and
pull-based update acceptance are required packaging, not proof of all runtime
qualification. Phono Junk, a separate
Collector Slint app, 100k-item benchmark, TLS rotation and legacy migration are not universal gates.

## 3. Useful follow-ups, selected by actual home-lab need

### HL-08 — Finish Trash Amp reliability and offline acceptance

**State:** adapter, native Slint browsing/list editing, CD-derived FLAC and direct
FLAC/MP3 streaming/downloads are implemented. HL-02 passed the actual native GUI
purchase journey, both formats through the real audio callback, and offline
artwork/text/search/playback/seek after connection removal and server shutdown.
The broader reliability matrix remains partial. **Maps:** C01/C03–C05/C07.
**Owner:** Trash Amp adapter/presentation over shared client/audio services.

Inspect sibling `docs/collector-connection.md` and the
[HL-02 evidence](validation/digital-purchases-2026-10-19.md) before assigning work.
Reuse the passing ordinary streaming/download/offline paths and immutable cache
identity regressions. Remaining qualification covers output capture/fidelity,
device loss, stalled/changed ranges and revocation, rapid server/account switching,
two-account live authorization isolation and comprehensive download failure/restart
UI journeys. Assess pinning/eviction and budget exhaustion against C05 and the
actual implementation; record missing behavior before implementing it. The
existing 10 GiB download budget and 128 MiB artwork cache are not proof that
all older C05 requirements pass. Preserve standalone local playback and retained
offline files; revocation cannot recall disconnected copies. No need to rebuild
C07, duplicate a player or require C02 failover before single-route downloads.

### HL-09 — Collection conveniences, game-disc listening, provider contributions and performance gaps

**State:** optional scoped work; most earlier web gaps are completed. **Maps:** U04,
D05/D07 listening extensions, D06 follow-ups, FX-04 history bound, P06/R02 practical scale.

#### Optional cleanup after successful inbox import

**State:** delivered in public alpha.3, 2026-10-21, with native container and isolated
Portainer cleanup/recovery acceptance. **Owner:** application intake
completion/ownership policy and filesystem adapter, with explicit web controls.

Explicit cleanup is available in the requester's Work & history detail after
preservation commits and requested processing settles. Optional processing failures
do not block it: retries use the archive. Background checks bind source identities
and actual source/archive checksums, then quarantine and delete only recorded files.
Cancellation, durable per-file intents and restart preserve remaining inputs; retries
cannot target a replacement at the old name. Unimported content stays retained.
Default imports and packaged inbox binds remain non-destructive/read-only;
`COLLECTOR_INBOX_READ_ONLY=false` and host write permissions are explicit opt-ins.
Inbox admission pauses during active cleanup to fence shared/aliased source use.
See the [delivery guide](inbox-cleanup.md) and [focused real-UI/recovery acceptance](validation/inbox-cleanup-2026-10-21.md).
This is not general archive pruning. Both native images and the actual-NAS
Portainer alpha.2 → alpha.3 drill passed read-only refusal, write-enabled cleanup
and full restore; see [publication acceptance](validation/container-publication-2026-10-21.md).
Existing passing HL-07A imports do not require cleanup.

#### Collection convenience and performance follow-ups

Completed 2026-10-18: Library “Back to results” now returns through nested Work &
history entries to the original list, preserving loaded pages, scroll and focus.
Intentional browser Back/Forward work restoration and direct-link returns remain
covered by the [browser regression and validation record](validation/library-work-return-2026-10-18.md).
Self-review strengthened restoration coverage after the modal unmounts; it passes.
This closes the defect observed in the 2026-10-09 AccurateRip shortcut journey.

- Completed 2026-10-21: general PDF/UTF-8 text document Assets, private-by-default
  attachment editor, authorized original downloads, native browser PDF viewing and
  bounded escaped text previews. Existing visibility, revisions, archive/reindex
  and upload receipts apply; documents cannot become artwork. See
  [acceptance and limits](validation/documents-2026-10-21.md). This is post-alpha.3
  source work; broader document rendering/formats remain conditional.
- Add a game metadata/artwork provider only for an identified collection need,
  with provider terms/access established, bounded acquisition, meaningful source
  attribution and manual authority preserved under HL-01 retention policy. Manual
  artwork and uploaded DATs work today.
- Page old preparation batches if Work & history is insufficient; the item view's
  latest-50 bound is disclosed. Page metadata responses before increasing the
  current 4,096-version per-item ceiling. Improve prefix-comparison upload resume
  bandwidth or inbox traversal only after measuring an actual bottleneck.
- Measure startup/browse/search/import/preparation on representative collection
  sizes and the actual low-powered host/NAS. P06 and generated bulk measurements
  are complete. Fix observed slow/uncancellable/unbounded work; do not reopen
  incremental indexing wholesale or require enterprise-sized load as a release gate.

Each bullet is independently assignable with real UI/recovery acceptance. Smart
lists, private/personal lists, post-edit splitting and richer DSP/codecs remain
optional ideas needing product/contract design, not established missing features.

#### Game-disc audio in music listening and Trash Amp

**State:** delivered in post-alpha.3 source, 2026-10-21.
Standard BINARY CUE/BIN ranges, shared extraction, Collector/web preparation and
listening, source-specific metadata/search and separate audio playlists are
implemented. Owned-reader and full owned-browser journeys, isolated native streaming/offline
and all source gates pass. Trash Amp commit `e389cc7` on `codex/game-disc-audio`
pins Collector `59faf34` and shared libraries `993afc9`; 32 affected tests,
desktop checking and strict affected-consumer Clippy pass. Existing staged native
release work was preserved. Future release packaging is separate.
See [delivery](game-disc-audio.md) and [dated evidence](validation/game-disc-audio-2026-10-21.md).
**Owner:** Games owns game identity and descriptive metadata; shared `junk-libs`
readers own disc layout and PCM extraction; application services own preparation,
authorized listening projections and list expansion; web and Trash Amp consume
the shared listening contracts.

Expose audio tracks included on CD-based game discs through normal music
browse/search, queues and playlists, plus Trash Amp streaming and explicit offline
downloads. Keep game artwork and source attribution beside track text. Start with
actual CD audio tracks, including mixed data/audio discs; proprietary in-game audio
files and soundtrack decoding are separate future scope. Preserve the original
game package and its collection identity without requiring a duplicate music import.

The selected contract settles cross-module listening ownership and source-bound
track metadata, including manual titles and useful unnamed-track defaults. It
extends the existing preparation/availability/media,
music discovery and list pipeline rather than introducing a second importer,
player or workflow. Complete bounded, cancellable audio-range extraction with
explicit data-track exclusion and gap/boundary policy; reuse exact source
coordinates, immutable recipes, lossless validation and durable publication.
Browse and admission must use recorded observations without hashing masters.
Automatic game metadata/artwork providers are not prerequisites.

**Done when:** an owned mixed-mode game demonstrates track discovery and explicit
FLAC preparation, ordinary web music browsing/search and listening, playlist
expansion, Trash Amp streaming and offline artwork/text browsing and playback.
Independent vectors cover audio/data boundaries, stored/synthetic gaps and malformed
or truncated inputs. Cancellation/restart, reindex, manual metadata authority and
collection authorization/revocation preserve the existing guarantees. Data tracks
never become playable, unsupported layouts remain explicit, and successful
extraction does not imply catalog or whole-disc audio verification. HL-06 owns
qualification of the advertised layouts; HL-04/HL-08 provide client acceptance
patterns without turning this new feature into a prerequisite for existing LAN
listening. Deliver the complete workflow across layers as one assignment.

#### AccurateRip contribution investigation

**State:** bounded investigation delivered 2026-10-18; supported external-tool
workflow documented, not executed. Collector submission support is not implemented.
**Owner:** application/provider policy; adapters own any supported submission mechanism.

The [investigation and operator guidance](accuraterip-contribution-investigation.md)
recommend physical-disc extraction and the built-in contribution workflow in EAC
or dBpoweramp on Windows. Maintainer statements identify those two submitting
clients and explicitly exclude Mac dBpoweramp. CTDB submission is a separate
service. AccurateRip drive calibration, secure full-disc extraction and explicit
consent are prerequisites for the recommended workflow. A Mac-only setup or
missing physical disc/configured drive blocks this route; stored Collector CRCs
and comparison offsets are not calibrated-ripper submission evidence.

Motivating case: **SQ Relax - Krystelle's Chillwave
Vol. 2**, by SQUARE ENIX MUSIC & Krystelle. An independent original-TOC calculation
and fresh HTTP/HTTPS requests confirmed no data for its exact disc layout; see
[lookup validation](validation.md#sq-relax-vol-2-live-accuraterip-lookup--2026-10-10).
That observation remains dated; the investigation did not repeat it or submit data.

Direct Collector integration is blocked by the lack of an established supported
provider submission contract and typed physical-drive/calibration/rip provenance.
No public write schema or acceptance contract was established. Provider-specific
agreement and a separate scoped assignment would precede implementation. Missing
data stays distinct from failure; sending results does not verify audio, and later
self-matches need explicit interpretation. See the
[dated evidence and limits](validation/accuraterip-contribution-2026-10-18.md).
The bounded research assignment is closed; external submission still requires
explicit user authorization. It is not a new implementation assignment.

#### Completed presentation follow-ups

Completed presentation follow-up, 2026-10-09: album FLAC/Opus availability now uses
compact secondary labels and on-demand unavailable reasons, two-line track rows,
overflow actions, and collapsed inspection information. The desktop entry
panel also meets the right viewport edge and dock at every scroll position; see
[status](implementation-status.md#compact-album-format-availability--2026-10-09) and
[validation](validation.md#compact-album-format-availability--2026-10-09).

Completed presentation follow-up, 2026-10-09: album rows show recorded AccurateRip
results instead of the independent Identity flag. Curators can open audio checks
from the column, album menu, or disc inspection; this retains honest original-level
coverage and adds no audio reads to browsing. See
[status](implementation-status.md#accuraterip-browsing-shortcut--2026-10-09) and
[validation](validation.md#accuraterip-browsing-shortcut--2026-10-09).
HL-06 live owned-disc qualification remains outstanding.

### HL-10 — Portable export and one-way legacy adoption

**State:** standalone full export/import delivered in public alpha.3, 2026-10-21; deltas and
legacy adoption remain optional/unimplemented. **Maps:** P05.1, R01.
**Owner:** application/archive export and legacy-import adapters.

The local CLI now exports and validates a coherent frozen full archive retaining
IDs, originals, assets, lists, recipes, meaningful history/evidence and exact unknown
records under HL-01, with digests/completion marker and no operational credentials/
grants/configuration. Import into an empty archive requires fresh setup and explicit
first-Owner recovery. Actual compiled-CLI acceptance covers partial copy, later
source writes, cold import/reindex, byte equality and private-state absence; see
[delivery](archive-export.md) and [dated acceptance](validation/archive-export-2026-10-21.md).
Public alpha.3 includes this delivery; see
[publication acceptance](validation/container-publication-2026-10-21.md). It is a full local copy,
not online export, a redacted public share or a replacement for full recovery backup.
Optional deltas require explicit base digest and dependency
closure; they do not implement replication.

Only implement Retro Junk migration when a real legacy archive must be adopted.
Inspect actual versions, dry-run mapping/unmappable fields, retain original sources
and manual choices/history, and make restart/idempotence explicit. Importing raw
files today does not migrate legacy IDs/history. Use export/backup contracts and
independent fixtures; R01 is conditional, not a mandatory rewrite for every owner.

### HL-11 — Personal activity, events and additional native surfaces

**State:** basic activity service and polling exist; offline reconciliation/events
and a standalone Collector Slint app are incomplete. **Maps:** C06, residual C03/C04.

Specify favorites/playback-event identity, conflicts and reconnect deduplication
before adding cross-device personal activity. Add HTTP events with bounded authorized
cursors and polling recovery only if polling latency/load warrants it. Neither
requires synchronized playback or archive replication. A dedicated Collector Slint
inventory/browser app is optional now that web and Trash Amp cover practical
use; reuse typed services, readers and artwork rather than creating parallel policy.
Native/offline gaps apply to the actual supported client, Trash Amp.

## 4. Deferred advanced capabilities and retained long-term requirements

### HL-12 — Automatic certificate rotation, discovery and route failover

**State:** S04/C02 unimplemented; manual trusted endpoints work. The design below is
a reference to reassess, not a fixed contract.

Automatic authenticated local-identity rotation with generations, old/new grace
listeners, offline transitions, atomic trust storage and restart recovery is substantial
work. Ordinary HTTPS renewal and explicit re-enrollment guidance belong to HL-05.
Prioritize this machinery only if expiry/recovery burden or a concrete threat warrants
it. Never accept unexpected keys or bypass certificate validation in the meantime.

mDNS, health probing, LAN preference/hysteresis and verified ranged failover are
conveniences for multi-route use, not prerequisites for manually configured encrypted
LAN or public access. Inspect C01 gaps in the actual consumers first; verified trust
must precede credentials, and resumed bytes must bind to immutable content. Re-run
backup qualification if rotation adds durable state. Actual LAN/tailnet tests remain
required before claiming these capabilities.

Potential rotation design, if needed: authenticate new trust through an already
trusted route; bind old/new certificate fingerprints and increasing generation to
ServerId; persist trust atomically before use. Stage key/certificate pairs together,
recover interrupted activation, and explicitly choose a bounded old-trust grace
route or re-enrollment. Never send credentials to an unexpected identity. Test
replays, wrong identity, expiry, offline devices and restart/restore with real TLS.
Grace listeners must not expose management implicitly. These are design constraints,
not a mandate for a multi-listener generation framework.

Potential routing design, if needed: treat mDNS/DNS-SD as untrusted hints, keep
public/tailnet routes explicit, bound uncredentialed probes and verify identity
before attaching credentials. Prefer healthy LAN with measured hysteresis; resume
only against the same immutable representation, length/digest and validator.
Persist only necessary download progress. Test forged/expired hints, changed keys,
route loss/recovery and actual LAN/tailnet transfers. `.local` is local-link
naming, not tailnet discovery. Exact probe counts/timing are implementation choices
based on a measured need, not inherited stability contracts.

### HL-13 — Synchronized listening and broader product expansion

**State:** ideas retained, not implemented. Shared listening was requested 2026-10-03:
join ongoing playback across rooms/devices without restart, support same-user and
other authorized listeners, define close LAN synchronization/drift correction, and
allow WAN shared listening without tight timing. Design session ownership, control/
join permission and coordination first; each participant retains media authorization.
This is distinct from HL-11 personal activity sync.

Also retained: SwiftUI iOS/UniFFI, PriceCharting exchange after API/permission research,
runtime plugin isolation, archive replication, cross-collection deduplication, video,
emulators, ripping hardware, smart/personal lists and general post-edit splitting.
Further digital-album scope belongs here: AAC/ALAC/Opus and other originals,
storefront integrations, archive extraction, DRM handling, continuous album-image/
CUE splitting and broader DSP. The delivered FLAC/MP3 source/availability pipeline
is the starting point; each selected addition needs a bounded complete workflow
and independent format/client evidence. No hidden dependencies on these for the
home-lab server.

### HL-14 — Advanced hardening and large-deployment qualification

**State:** deferred retained scope, not certification. **Maps:** S01/R02/R03 advanced
qualification. Preserve the original [security contract](decisions/security.md),
[broader target requirements](release-requirements.md#broader-product-target-reference)
and dated implementation evidence. The advanced scope is retained here;
there is no separate historical work queue.

Retain persistent/distributed rate limiting, exhaustive password-manager/platform/
hardware matrices, broad independent protocol/security/accessibility review,
100,000 Items/one million file-evidence records/five-user load on a recorded
four-core/16-GB/local-SSD host (warm browse/search <500 ms p95), full deployment
matrix and stronger power-loss/storage qualification. They are not automatic
home-lab blockers. Distributed queues/multi-server active writers are outside
current scope, not solutions to invent for a single server.

Promote low-effort, concrete hardening and any finding affecting exposed access,
credential safety, untrusted parsing, data preservation or recovery immediately.
A focused Internet-facing security review remains HL-05; deferring exhaustive
enterprise-scale assurance does not defer actual authentication/access/resource
safety. If a deployment needs restart-resistant limiting or stronger durability,
record that requirement and implement/qualify it for that deployment.

## Reassessment and source coverage

| Earlier work/source | Disposition in this queue |
| --- | --- |
| A01–A04, U01–U04 image/inventory/access, S01–S03, P01–P04, D01–D07 local implementation | Keep completed history; only concrete residual qualification/features above remain |
| LA1/LA2, FX-01–FX-07, work-context hardening, U05/P04 local interface audit | Completed locally; do not schedule a second implementation/audit wholesale; HL-04/05/07 retain external checks |
| BL1–BL3, CL1, P06, navigation/wording/preparation help, account software repairs, October game/DAT/toolchain TODOs | Delivered; evidence limitations mapped to HL-04/05/07/08/09 |
| LA3 installation and shared account/connection software | HL-04/05; retain relevant web/Trash Amp checks |
| PJ1 and Phono-specific account/GUI/physical-Mac acceptance | Retired target; preserve dated evidence, do not complete its remaining matrix |
| Previous P05 proposals | HL-03 full backup first; HL-10 export and optional deltas later |
| Earlier native-client proposals | Reassess against implemented Trash Amp; HL-04/08/11 residual work, no wholesale native rewrite |
| S04/C02 | HL-12; no artificial dependency on them for single-route playback/offline downloads |
| R01 | HL-10 only for actual legacy adoption |
| R02/R03 and release requirements | HL-05/06/07 practical release; HL-09 representative performance; HL-14 extreme matrix |
| U04 documents, D06 extra providers, FX-04 scale bounds, C06 | HL-09/11; optional measured demand |
| Shared listening and follow-on scope in old plans/status | HL-13; HL-09/11 where appropriate |
| Digital FLAC/MP3 purchase support requested 2026-10-19 | HL-02 delivered and published; reuse acceptance in HL-04/06/08; additional original codecs/intake/splitting remain HL-13 |
| Obsolete wave queues and music-first proposal | Removed; relevant unfinished scope and advanced design are consolidated here |
| Reviews, foundation/reuse/security decisions, UX review, validation ledgers and release notes | Technical/evidence references; HL-01 reassesses validity, complexity and retention; no independent assignment order |

Dropped as unnecessary scheduling requirements: restarting completed roots; serial
wave handoffs; checkpoint-only compatibility wrappers/schema reservations;
mandatory new Collector desktop app or legacy migration for every installation;
mandatory rotation/discovery before single-route use; a huge synthetic benchmark
before ordinary home-lab adoption. Data conversion/recovery for actual archives
and independent evidence for advertised claims remain required. Old proposed contracts
are subject to redesign; append-only history is an implemented mechanism, not an
unlimited future-retention requirement.

The disabled DAT-o-MATIC downloader stays disabled. Manual download/upload is
implemented; provider-access restoration or automated scraping is not an open task.

## Active autonomous assignment — 2026-10-21

The user assigned independent release-readiness review, remaining useful software
work and game-disc audio, with self-review before completion and a commit per
milestone. Complete focused authentication/upload/media review and actionable
repairs first (HL-05H and the bounded checklist are now delivered). Game-disc audio
across the shared reader, Collector, web and Trash Amp is delivered in source, with
owned/browser/native acceptance; general PDF/text documents are also delivered.
The opt-in backup-age reminder is also delivered. Bounded native software failure
qualification now covers durable command loss/restart, actual-read grant/device
revocation, changed/ignored/truncated ranges, header/body stall cancellation and
wrong pinned-trust recovery. The remaining HL-04 UI/physical-device/network checks
and conditional provider/product decisions retain their named limits. Preserve staged sibling release work.
Read-only access to useful material in `~/dumps` is explicitly authorized; use owned
mixed-mode references when available, keep originals unchanged and never commit
copyrighted media. Keep actual-device/reference gaps unqualified and continue other
independent work. Game-disc audio may remain outside the next advertised release
scope until its relevant validation passes. Reuse accepted alpha.3 publication and
NAS recovery evidence; no production update or giant repeat matrix is assigned.
Deferred provider integrations, legacy migration without actual legacy data,
new codecs, synchronization and enterprise matrices remain conditional.

## Ready-to-send next assignment

HL-01 retention/CAA simplification, HL-02 FLAC/MP3 purchases and HL-03 stopped-server
backup/restore tooling are delivered. Do not restart those completed scopes.
The grouped-disc publication repair is also implemented and operator-reported as
working on the NAS. HL-07A is delivered with [local acceptance](validation/intake-review-2026-10-19.md).
HL-07B is delivered with [local acceptance](validation/direct-playback-2026-10-19.md).
HL-07C is delivered with [local acceptance](validation/release-wording-2026-10-20.md).
HL-05A is delivered with [actual-host isolated acceptance](validation/nas-operations-2026-10-20.md).
HL-05B's isolated path is also qualified with [independent recovery acceptance](validation/snapshot-recovery-2026-10-20.md).
Independent media copies are optional; no destination choice blocks this queue.
The packaged local backup area and consistent rollback policy are documented.
HL-04 device and parent HL-05 permanent-installation/public-ingress qualification
remain available when the actual hosts/devices are accessible. HL-05C active preparation recovery is also qualified; see its dated record above.
HL-07D is delivered with [public-image Portainer acceptance](validation/container-publication-2026-10-20.md).
HL-06A's real-tool FLAC/Opus profiles are qualified with
[independent audio acceptance](validation/audio-oracles-2026-10-20.md).
HL-05D's repaired candidate passed numeric-IP startup and actual NAS recovery; see its
[permission-failure record](validation/storage-permission-recovery-2026-10-20.md).
HL-05E's bounded archive ENOSPC/restart/full recovery is qualified; see its
[capacity-failure record](validation/storage-capacity-recovery-2026-10-20.md).
HL-05F bounds authentication identity retention and qualifies real loopback
throttling/restart; see its [source acceptance](validation/rate-limit-retention-2026-10-20.md).
HL-05G independently qualifies actual-NAS login ceilings from multiple loopback TCP
sources and full recovery; see its [acceptance](validation/nas-authentication-limits-2026-10-20.md).
HL-09's explicit inbox cleanup is delivered with
[real-UI and recovery acceptance](validation/inbox-cleanup-2026-10-21.md).
The requested cleanup and release assignment is complete: public **alpha.3**
packages it, portable full archive transfer and the post-alpha.2 fixes. Both native
images and one isolated actual-NAS Portainer pull/update/cleanup/full-restore drill
passed; see [publication acceptance](validation/container-publication-2026-10-21.md).
All 17 live container identities/images/start times stayed unchanged and generated
fixtures were removed. Reuse this acceptance; no new deployment matrix or live
production update is assigned.
HL-10's standalone portable full export/import is delivered with
[local CLI acceptance](validation/archive-export-2026-10-21.md); deltas and legacy
adoption remain conditional follow-ups.

> Continue the remaining HL-04 trusted-device/LAN, HL-05 actual public-ingress and
> HL-06 owned-format qualification only when their named devices, deployment access
> or independent reference material are available. Reuse the accepted HL-07D public
> alpha.3 installation/update/recovery evidence; do not repeat packaging or infer
> system-trusted second-device/public-ingress acceptance from it. Keep experimental
> release limits explicit. Independent media copies, paid cloud backup and backup-age
> reminders remain optional and do not block this queue. Production deployment
> changes require an actual assignment; the publication drill left live services intact.

Completed bug repair, 2026-10-09: AccurateRip check panels no longer default a
single-anchor disc to an unrelated collection response. Current recorded layout
supplies the full query for admission and response selection, including matches on
older pages and correction of stale browser preferences. Missing responses prompt
acquisition. The worker retains its strict selected-layout check; real owned-disc
qualification remains HL-06. See [status](implementation-status.md#accuraterip-response-selection-repair--2026-10-09)
and [validation](validation.md#accuraterip-response-selection-repair--2026-10-09).

Follow-up to that repair, 2026-10-09: missing-reference readiness keeps eligible
source files and has no empty-UUID error; imported layouts expose an actionable
fetch through server-owned structural eligibility. No recorded layout offers the
existing structure check. Acquisition keeps source/module/role and ambiguity guards.
See the same status and validation record above; HL-06 remains open.

Completed progress follow-up, 2026-10-09: active file checks report typed
indeterminate progress and actual source/computation/publication phases instead
of misleading zero completion. Development builds optimize shared audio loops.
Algorithm/coverage and live-owned-disc qualification are unchanged; see
[status](implementation-status.md#file-check-progress-and-development-audio-performance--2026-10-09)
and [validation](validation.md#file-check-progress--2026-10-09).
