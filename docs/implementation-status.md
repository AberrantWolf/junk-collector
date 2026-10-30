# Implementation status

The [shared music review](validation/digital-album-matching-review-2026-10-30.md)
records follow-up completeness, coordinate, provider and fingerprint safeguards
and the consumer dependency update.

## Digital album matching — delivered, 2026-10-30

The [application review](validation/digital-album-matching-app-review-2026-10-30.md)
fixes artwork selection, stale review/retry handling, asynchronous UI ownership,
and Trash Amp enrichment/restoration races.

Collector now provides explicit audio-based discovery for original-file FLAC/MP3
albums, with durable cancellable/retryable jobs, source-bound fingerprint reuse,
AcoustID recording lookup, MusicBrainz release discovery and artwork previews.
Review shows coverage, durations and one-to-one track assignments. Only complete,
unambiguous mappings can be applied together through the existing metadata
workflow. Raw masters and manual metadata retain authority; inventory/source or
metadata-authority changes invalidate stale discoveries. No audio is read during
browsing, admission or polling.

`junk-libs-music` shares decoding/fingerprinting, community MusicBrainz models,
parameter-safe URLs, AcoustID decoding and comparison with Trash Amp. Trash Amp
supports explicit review/application for local folders and opens Collector's
workflow for managed albums. New Collector provider responses use the community
models; retained schema-1 evidence keeps its original interpretation.

The [evaluation](digital-album-matching-evaluation.md),
[probe](digital-album-matching-probe.md) and
[delivery evidence](validation/digital-album-matching-2026-10-29.md) record the
qualification and limits. Published chromaprint-next 0.1.0 is not used: a corrected
exact revision is pinned. A configured AcoustID application key is required.
Live AcoustID recognition, other codecs and manual partial/ambiguous mapping are
not qualified by this delivery. Matches are identity suggestions, not catalog
verification. No deployment is included.

## Change-driven Library summaries — beta.13 installed, 2026-10-28

Library reads no longer build missing summaries. Inventory supplies immediate
basic rows with typed pending/retry state, while a separate bounded worker
reconciles durable item generations for current role/language demand. Changes,
startup and reindex prepare views without a Library request; source tokens fence
publication, and failures retry without starving peers. File/playability/listening
filter predicates are materialized on changes too. Web polling includes empty
filtered pending pages. Focused storage/recovery/filter regressions and web model
checks pass, including atomic additive-install rollback. Exact-source workspace
tests/Clippy, all 27 browser journeys, both native images and release-image browser
acceptance pass. Beta.13 is published and installed after a complete stopped-server
snapshot and a 4.28-second interruption. All 79 required summaries are prepared
before browsing; the first observed Library request takes 8 ms for 40 entries,
and a full refresh takes 10 ms. Config/TLS/identity/mounts, all 18 unrelated
containers, HTTPS and nine assets pass. No user jobs were unfinished at the update.
See [publication and NAS acceptance](validation/container-beta13-publication-2026-10-28.md) and
[delivery and validation](validation/change-driven-library-summaries-2026-10-28.md).

## Compact Library row cache — beta.12 installed, 2026-10-28

The web Library now uses a compact application-owned row API and a persistent,
rebuildable server cache. Current role/language/module policy scopes cached rows;
transactional projection tokens invalidate changed summaries and fence concurrent
publication. Import progress remains live. Opening detail or curation reads the
full authorized Item separately. Exact-source workspace tests/Clippy, all 27
browser journeys, both native runtime checks and release-image browser acceptance
pass. Beta.12 is published and installed on the NAS after a stopped-server snapshot
and a 7.88-second interruption. Config/TLS/identity/mounts and all 18 unrelated
containers are retained; HTTPS and all nine assets pass. No jobs were unfinished
at this update. Signed-in Library navigation and full refresh render all 40 entries
with artwork/status. The refreshed cached request takes 8 ms and three repeats
take 15/9/9 ms for 29,800 UTF-8 bytes; the first observed post-upgrade request still
takes 4.86 seconds to build new rows. These are settled-batch measurements.
See [cache behavior](validation/library-row-cache-2026-10-28.md) and
[publication/deployment evidence](validation/container-beta12-publication-2026-10-28.md).

## Beta.11 published and installed — 2026-10-28

Beta.11 includes the one-second processing cancellation poll, manual metadata
cover uploads and focused AccurateRip batch results. Exact-source full checks,
all 27 browser journeys and native container acceptance pass; publication reused
those validated bytes. The NAS runs the pinned beta.11 image after a complete
stopped-server recovery snapshot and a 66.25-second interruption. Config, TLS,
identity, mounts and 18 unrelated containers are retained. All 12 unfinished jobs
retain their identities and creation timestamps; all subsequently completed.
Public HTTPS and all nine checked assets pass. The signed-in Library returns all
40 entries in 4.7 seconds and renders artwork/status after a full refresh. The
batch has settled: 22 successful, three needing attention, zero pending. This
post-batch check does not qualify responsiveness under the original load. See
[release notes](releases/v0.1.0-beta.11.md) and
[qualification, recovery and acceptance](validation/container-beta11-publication-2026-10-28.md).


## Library responsiveness during audio checks — 2026-10-28

The processing host shares an ephemeral cancellation poll across its package
readers, module CPU checkpoints and catalog parser. Clear durable Job checks are
reused for at most one second, measured with a monotonic clock before the storage query;
cancellation and errors are never cached as permission to continue. Sector-sized
redumper reads therefore no longer issue a shared-database query for every read.
Source validation, final authorization/cancellation checks and storage publication
fences remain direct durable checks. No archive records, results or worker settings
change. See [diagnosis and validation](validation/library-audio-check-contention-2026-10-28.md).
Included in beta.11 on the NAS; durable job recovery and post-batch production
Library loading are verified as recorded above.


## Metadata cover uploads and focused rip-check results — 2026-10-28

Metadata review now offers a manual cover upload beside **Cover choice**, using
its existing authorized Asset upload service and the shared attachment form.
Uploads retain the original, visibility and alternative text, immediately appear
among saved images and are selected for **Use selected artwork**. Unsaved artwork
choices survive background metadata polling. Invalid files and unconfirmed uploads
remain visible; retries reuse the frozen upload inputs and command identity.

AccurateRip batch member reads omit excluded entries before storage pagination,
including completed/older batches. Summary counts and durable frozen membership
remain intact; eligible pending and terminal results still appear. Other batch
kinds retain their existing member view. See [delivery and validation](validation/metadata-cover-upload-and-rip-results-2026-10-28.md).
Included in beta.11 on the NAS. Production cover uploads were not part of
deployment acceptance.

## Beta.10 published and installed — 2026-10-27

Beta.10 releases all current changes: one-click/automatic-ingest AccurateRip,
collection and live worker defaults, original artwork acquisition, Library import
progress and standalone capture/naming fixes. Broad exact-source checks, all 27
configured browser journeys, both native images and release-image browser
acceptance pass. Publication reused the accepted bytes and advanced `preview`.
The NAS runs the pinned beta.10 image after a complete read-only recovery snapshot
and a 4.25-second switch, retaining config/identity/TLS and 18 unrelated containers.
Public HTTPS and all nine changed assets are verified. See [release notes](releases/v0.1.0-beta.10.md)
and [publication/deployment evidence](validation/container-beta10-publication-2026-10-27.md).
No live-library batch was started; physical CD capture/eject and authenticated
production listening remain separately unqualified.


## One-click AccurateRip and configurable defaults — 2026-10-27

Source delivery adds **Check this disc with AccurateRip**, sharing one durable
application workflow across the album view, advanced checks, guided ingest and
library batches. It records missing structure, acquires the matching response and
checks audio without a manual fetch step. Only qualified matching audio becomes
verified. No-data, ambiguity, unsupported input and failures remain visible.

**Administration → Library settings** owns per-music-collection automatic ingest
checks (on by default) and the maximum offset. **Check unconfirmed rips** queues
existing eligible originals through the existing bounded batch/recovery flow;
confirmed sources and digital purchases are skipped. **Administration → Server
settings → Background workers** saves live processing/copy limits. Saved settings
survive restart and reindex; accepted ingest settings remain frozen. Existing
unfinished intents retain their previous policy. Other collection types have no
invented configuration fields. See [delivery and acceptance](validation/automatic-accuraterip-2026-10-27.md).
Workspace tests, Clippy, web/contracts/models and all selected Chrome journeys
pass. Included in beta.10 on the NAS; no live-library batch was started.


## Original provider artwork repair — 2026-10-26

Source implementation accepts full-quality CAA cover originals up to 128 MiB
through the streaming snapshot pipeline. When multiple standalone front covers
are available, application policy compares advisory original sizes and freezes
its choice across continuations/restart. Explicit selections retain their exact
image. Upload and decoder limits remain independent; oversized failures provide
recovery through explicit cover choice. Dismissed lookup failures stay dismissed
when the panel reopens, independently per account/album. Workspace tests, Clippy,
web checks/build/models and focused real-browser acceptance pass; see the
[delivery record and limits](validation/provider-artwork-originals-2026-10-26.md).
Included in the beta.10 NAS update recorded above.

## Library import progress — 2026-10-26

Album rows show a muted clock and “Import in progress” while their guided import
is uploading or processing, including downstream enrichment/audio preparation.
The Library refreshes active rows automatically. Playback-attention filtering
excludes importing albums; settled, cancelled or blocked imports resume the
ordinary playback-readiness display. See [validation](validation/library-import-progress-2026-10-26.md).
Included in the beta.10 NAS update recorded above.

## Beta.9 published and installed — 2026-10-26

Beta.9 includes the faster testing/packaging workflow, active-release/Disc ID
controls and copy cleanup. Full exact-source checks, both native images and
release-image browser acceptance pass. Publication reused validated bytes and
advanced `preview`; public digests/assets are independently verified. The NAS update used
the pinned beta.9 image after a complete recovery snapshot and a 3.94-second
switch, retaining configuration/identity/TLS and 18 unrelated containers. See
[publication and NAS evidence](validation/container-beta9-publication-2026-10-26.md).

## Faster checks and packaging — delivered, 2026-10-26

The requested [testing workflow](testing.md) now selects checks from the complete
change, exempts reviewed display/prose edits from unit tests, and separates short
browser acceptance from broad integration and optional scale qualification.
The shared harness prevents the known macOS seatbelt Chrome launch and bounds
owned process cleanup. Container source checks and native image builds run in
parallel, with dependency caching, exact-source check reuse and profile-aware
validation evidence. Discarded standalone release compilation is removed.

Full workspace/model/browser qualification, both native release-image smokes,
hosted cache reuse and exact-source nonpublishing promotion rehearsal pass. The
full candidate completed in 15m57s and artifact-only rehearsal in 47s; short smoke
takes about five seconds. See [dated evidence](validation/testing-and-packaging-2026-10-26.md)
for source/run identities, measurements and limits. Those timings describe the
original tooling qualification; subsequent beta.9 publication and NAS acceptance
are recorded above.


## UI copy cleanup — beta.9 deployed, 2026-10-26

Metadata, playback and import views use shorter guidance with fewer repeated
preservation assurances. Distinct import, grouping, replacement and recovery-code
rules appear as lists. Consequences, recovery actions and evidence limitations stay
visible. See [scope and validation](validation/ui-copy-cleanup-2026-10-26.md).

## Metadata active release and Disc ID action — beta.9 deployed, 2026-10-26

The Metadata panel now marks the resolved candidate with a prominent “Currently
using” banner and outline and places it first among visible matches. The effective
metadata summary identifies the provider/selection mode and calls out manual field
overrides separately; a manual display title can coexist with a provider release.
Proposed or unused matches do not receive the active cue.

The Disc ID submission action has moved beside the active MusicBrainz release.
It reuses authorized server coverage, hides on release/revision changes until
fresh coverage arrives, and ignores coverage for a different release. Identical
recorded TOCs share one link; distinct eligible discs have labelled links.
Registration/TOC eligibility remains application policy. The old disc-details
placement is removed. Full source checks and focused/full Chrome acceptance pass;
evidence is recorded
[here](validation/metadata-release-controls-2026-10-26.md). Published and installed
on the NAS in beta.9; see the publication record above.

## Metadata attention notice — 2026-10-26

Metadata review shows current attention reasons as a bulleted list under
“Needs Attention:”, followed by “(Manual metadata always takes precedence.)”.
See [validation](validation/metadata-attention-notice-2026-10-26.md).

## MusicBrainz track names and Disc ID submission — 2026-10-26

An effective single-disc release now supplies track names when recorded track
positions match even if its Disc ID is unregistered. Explicit multidisc assignments
also work without registration; positive matches to another disc still conflict,
and incomplete/ambiguous layouts stay unresolved. Manual titles retain authority.

Music → Disc details & checks offers **Submit Disc ID to MusicBrainz** beside each
complete recorded CD original when its ID is absent from the effective release.
It opens MusicBrainz with the recorded TOC and release ID for user review/sign-in
and confirmation. The button is hidden when that ID is already registered; refresh
the release in Metadata after submitting. Reads use current committed observations
without opening/hashing audio. No MusicBrainz credentials, automatic edits or
submission-success receipts are introduced. See [acceptance and limits](validation/musicbrainz-disc-submission-2026-10-26.md).
Source checks and full Chrome smoke acceptance pass. Published and deployed as
beta.8 on the NAS, with retained settings/identities, a complete recovery snapshot,
passing production HTTPS/served-asset checks and 18 unrelated containers unchanged.
See [publication and deployment evidence](validation/container-beta8-publication-2026-10-26.md).

## Evidence-aware editions and equivalent matches — beta.7 deployed, 2026-10-25

Automatic music edition preference uses current recorded CD IDs and disc positions
across attached originals before language/identity tie breakers. It reassesses on
reads and through paginated periodic maintenance; later discs may change the
preference. Manual release choices survive. The open Metadata panel refreshes
background changes while retaining unsaved text edits. Missing provider-listed
media remains in coverage but does not block playback/list expansion of available,
unambiguous discs; ordinary preparation and authorization still apply.

Matches differing only by internal version consolidate to one current/review
choice. Candidate content, artwork, ranking and provider identities must agree;
new MusicBrainz versions exclude unrelated response content. Immutable source
records and referenced selections remain preserved, including across reindex.
Historical displays retain their recorded resolution policy; new publications
freeze the chosen automatic edition. This is logical cleanup, not archive
compaction or exact-pressing/audio verification. See [acceptance and limits](validation/edition-fit-2026-10-25.md).

Published and installed as beta.7 with the security dependency repairs. Actual
NAS cleanup consolidated all 17 redundant active versions (44 choices → 27),
with zero remaining duplicates and all 131 older metadata revision files unchanged.
Runtime settings, identities and 18 unrelated containers are retained; see
[publication/deployment acceptance](validation/container-beta7-publication-2026-10-25.md).


## Standalone raw CD capture helper — 2026-10-25

`scripts/rip-cd.sh NAME DESTINATION [DRIVE]` creates a fresh named directory and
runs redumper acquisition, extended-area acquisition and conditional refinement, retaining
all raw masters/sidecars/logs without generating BIN/CUE or playback copies.
The 2026-10-27 `--auto DESTINATION [DRIVE]` option starts with a unique generated
name, then attempts one exact MusicBrainz lookup within a 15-second wall budget.
The optional release-built `collector-cd-identity` adapter reuses shared TOC
parsing/Disc ID code and reads no audio or drive. One release, matching medium
and usable catalog number allows an exclusive atomic folder rename, with a disc
suffix for multi-disc releases. Ambiguity, timeout, unavailable tools, unsafe names
and destination collisions retain the generated folder. Manual names skip lookup.
The lookup publishes no Collector metadata or verification evidence.
The 2026-10-27 follow-up preserves specific worker/identity/provider diagnostics,
distinguishes inconclusive metadata and rename errors, and ejects the pinned drive
after successful capture/naming, including generated-name fallbacks. The subsequent
operator-reported HTTP 400 is repaired by requesting only `inc=labels` on the
Disc ID endpoint, whose default response includes releases/media/disc IDs. The
corrected request returned HTTP 200. macOS now uses `diskutil eject` directly with
normalized block/raw drive identifiers; other platforms retain redumper eject.
Eject receives no image options and reports failures separately; failed/incomplete
captures leave the disc inserted. Offline diagnostics/eject acceptance is recorded with the
[naming delivery](validation/cd-folder-naming-2026-10-27.md).
After automatic initial selection it pins the logged drive for later invocations;
failure messages identify the stopped stage without declaring earlier reads failed.
The 2026-10-26 adjustment starts refinement only for nonzero initial SCSI/C2
counts; Q-only errors skip it, matching normal redumper refinement eligibility.
Missing/malformed summaries stop for review without starting refinement. Offline
regressions cover the decision and failures. The 2026-10-27 operator run exercised
the Q-only skip on one owned audio CD; additional manual Q refinement recovered
15 of 31 reported Q sectors, leaving 16. Another BU40N recovered none of those
remaining sectors at default optimal speed or requested 8x. This is bounded
operator evidence, not complete drive/format qualification or an AccurateRip result.
The [accepted capture tradeoff](decisions/cd-audio-capture-tradeoff.md) keeps
Q-only refinement optional, retains captured subchannels/logs and uses AccurateRip
to assess audio within its actual coverage. Persistent Q counts alone are not
mandatory recovery work or evidence of damaged audio.
This is an operator helper outside Collector's import/domain services, not an
integrated ripping subsystem. See [usage](getting-started.md#import-options) and
[validation/limits](validation/raw-cd-capture-2026-10-25.md). Broader physical-drive
acceptance remains unqualified; completion is not audio/catalog verification.

## Utoipa 6 maintenance cleanup — 2026-10-25

Utoipa 6.0.0 / utoipa-gen 6.0.1 and utoipa-axum 0.3.0 replace the final
unmaintained `paste` dependency with `pastey`. The refreshed RustSec audit reports
zero vulnerabilities and zero maintenance warnings, without advisory ignores.
The generated API stays OpenAPI 3.1.0; its 155 paths, operations, schemas and
body-limit extensions match the review checkpoint after response descriptions
and unordered nullable alternatives are accounted for. The server supplies
required response descriptions that the new serializer would otherwise omit.
See [upgrade validation and limits](validation/utoipa6-2026-10-25.md).
No release or deployment is included.

## Security review checkpoint — 2026-10-25

The review found and corrected a shared DAT read-bound bypass: literal delimiter
bytes inside XML attributes/comments/CDATA reset the old token counter. Published
`junk-libs` commit `317517c430744e4f1196e30850b37b3f13c10560` bounds each parser
event before buffering and retains the independent decoded-value/total-input
caps. Collector pins that correction. The prior ZIP, TLS, certificate authority,
CHD and PEM fixes remain intact; no further actionable regression was found in
those changed paths. See [review scope and validation](validation/security-review-2026-10-25.md).
The subsequent Utoipa 6 delivery above completes the remaining maintenance cleanup.

## Client PEM parser maintenance cleanup — 2026-10-25

`collector-client` uses `rustls-pki-types::PemObject` instead of the unmaintained
`rustls-pemfile` wrapper. Local playback still requires one certificate whose DER
SHA-256 matches the authenticated descriptor; the same parsed DER supplies TLS
trust. Missing, multiple, malformed and incorrectly pinned certificates are
rejected. The bounded descriptor, private-address, expiry, server-identity and
credential-release fences remain in place. The lockfile removes `rustls-pemfile`
without upgrading other packages at that checkpoint. Its audit reported zero
vulnerabilities and only the `paste` maintenance warning, now cleared above. Client/workspace tests,
Clippy, native LAN media, browser smoke/private playback and refreshed Trash Amp
integration checks pass. See [dated checks and limits](validation/pem-parser-maintenance-2026-10-25.md).
No release or deployment is included.

## Shared CHD/Zstandard security repair — 2026-10-25

Collector pins published `junk-libs` commit `e4d536b7c23d2075c8a98f6a1525e1bcaa32eef0`.
The shared minimum is `chd` 0.3.5 and Collector's lockfile resolves `ruzstd`
0.9.1. Its refreshed RustSec audit reported zero vulnerability advisories, with
the existing `paste` and `rustls-pemfile` maintenance warnings. The client cleanup
removed `rustls-pemfile`; the Utoipa delivery above removes `paste`. Shared
disc/audio/format suites, independent chdman 0.285 codec round trips and affected
Retro Junk/Trash Amp tests and Clippy pass. Collector's full workspace tests,
Clippy, web checks/build and real browser smoke also pass; see [dated acceptance and limits](validation/chd-security-2026-10-25.md).
No application policy, original bytes, archive
format, release image or live installation was changed.

## Shared XML security repair — locally validated, 2026-10-25

The XML delivery pinned published `junk-libs` commit `b343ea540dcc5a389718e55e2be590040071118c`,
upgrading the shared DAT parser to quick-xml 0.41. Text/entity/CDATA fragments are
joined before trimming, accumulated text stays bounded and malformed references
fail closed. The public DAT records and application authority policy are unchanged.
Both quick-xml advisories cleared from the lockfile audit; the subsequent CHD
repair above clears `ruzstd`. Two maintenance warnings remained at that stage;
the subsequent client/Utoipa cleanups above clear both. Sibling checks and source publication evidence are
recorded in [the dated validation](validation/quick-xml-security-2026-10-25.md).
The user authorized the previously blocked push; `codex/fix-quick-xml` is now
published and its remote head matches the validated pin. Fresh machines/CI can
fetch that source. No Collector release or deployment has occurred.

## Dependency security updates — 2026-10-25

The source lockfile updates Quinn, rustls, certificate validation/root certificates
and Tokio to patched releases. The provider ZIP dependency moves from 2.4.2 to
9.1.0, retaining bounded in-memory member inspection rather than filesystem
extraction. This initial pass reduced RustSec findings from 16 vulnerability
advisories to three and cleared Tokio's I/O safety warning. The subsequent XML
and CHD repairs above initially left two maintenance warnings; the client and
Utoipa cleanups above clear both, as tracked in [remaining work](remaining-work.md).
Both shared security fixes are now published.
See [versions, validation and limits](validation/dependency-security-2026-10-25.md).
These are source changes; the published images and NAS installation still need a
future release/update to receive them.

## Album playback attention — beta.6 published, 2026-10-24

Music library rows now show a warning when recorded whole-album playback is
blocked. Detail displays the resolver's reason, and Show → Album playback needs
attention finds those albums across paginated results. Missing release discs,
unassigned/alternate originals, incomplete track scope and unprepared audio are
covered without opening/hashing source audio. Opus-only prepared albums are
ready; FLAC is a preference rather than a prerequisite. Browser codec and connection
failures remain runtime concerns. The original Needs my attention filter still
finds requester-private failed work/import review. See [dated acceptance](validation/album-playback-attention-2026-10-24.md).
Published in beta.6 after exact-source checks and native AMD64/ARM64 container
smoke acceptance. Real Chrome browse, purchase and full smoke journeys pass;
public images/assets and `preview` are independently verified. See
[publication evidence](validation/container-beta6-publication-2026-10-24.md).
Beta.6 is now installed on the NAS; retained-state, runtime and production HTTPS
checks pass. See [deployment evidence](validation/container-beta6-nas-2026-10-25.md).
Physical iPhone acceptance of this UI remains outstanding.

## Browser lock-screen metadata — iPhone accepted, 2026-10-24

The web player now publishes track title, album artist, album and available cover
art using the standard Media Session API, for album and playlist playback. OS
play/pause, previous/next and seeking use the existing player and permission path.
Track transitions update metadata; release/logout/detachment clear metadata and
handlers. Older saved queue entries retain their title/album even without the new
optional artist field. [Real Chrome acceptance](validation/browser-media-session-2026-10-23.md)
covers metadata, controls, automatic transitions and cleanup. The
[published beta.5 image is installed on the NAS](validation/container-beta5-publication-2026-10-23.md),
and Cloudflare serves its exact web bundle. On 2026-10-24 the operator confirmed
the requested iPhone Safari lock-screen metadata check works on beta.5; see the
[physical-device follow-up](validation/browser-media-session-2026-10-23.md#physical-iphone-follow-up--2026-10-24).
The report did not separately itemize metadata fields or OS-button behavior.

## Direct HTTPS remote media — source delivered and NAS deployed, 2026-10-23

Cloudflare login/authentication/control can now issue short-lived, exact-track
browser/native grants for a configured direct HTTPS media origin. The separate
`collector-media` executable reads only through the configured archive root,
supports full/single/open/suffix ranges and HEAD/ETag/If-Range, and continuously
checks live parent/collection authority over a restricted core Unix socket.
It never accepts login cookies or long-lived account credentials. The shared
native client retains system HTTPS trust for media independently of control trust;
the web player uses ordinary anonymous HTTPS audio and preserves position on
lease renewal. Private routes stay preferred and control-origin music remains
refused. No VPN, relay, HLS or mobile app was introduced.

The optional Compose overlay gives media no networking, read-only archive/root,
no capabilities, dedicated socket named volumes and bounded resources. Startup
rejects effective writable archive/nested mounts. A dedicated Caddy frontend owns
TLS and certificate state. Its networking remains a distinct firewall/DMZ concern;
containers share the host kernel and mounted archive confidentiality is not isolated.
One-origin media-path composition also works with cookie stripping and its stated
browser-origin tradeoff. The normal release image includes both executables for
separate services; an optional Docker target supplies a lean media-only image.

The `/remote-media` page checks a fresh one-use challenge and real queued-track
early/late ranges without full-file hashing or changing network settings. A local
success cannot prove WAN reachability. See [contract](direct-https-media-plan.md),
[setup/rollback](../deploy/container/media.md) and [acceptance and limits](validation/direct-https-media-2026-10-23.md).
The authorized [NAS deployment](validation/direct-https-media-nas-2026-10-23.md)
first qualified the AMD64 source candidate with DNS-only media, IPv4 TCP forwarding
and a trusted public certificate. Disposable actual-NAS isolation and production
route/identity/CORS checks pass. DSM uses one-CPU affinity instead of unsupported
CFS quotas; its process-count limit is not enforced. Caddy remains networked
without a new DMZ/egress restriction. The operator reports successful basic
**iPhone Safari cellular playback**, seeking and next/previous, with Direct HTTPS
displayed; the setup test appeared to pass without a captured per-stage result.
The operator also heard a track change while the phone was locked, accepting that
background transition. On 2026-10-24 the operator also reported approximately
[three hours of background cellular listening without playback issues](validation/direct-https-media-nas-2026-10-23.md#extended-cellular-background-follow-up--2026-10-24),
accepting extended background playback. Lease expiry during a single long track,
network transitions and actual public certificate renewal remain unqualified. The later
[beta.5 publication/update](validation/container-beta5-publication-2026-10-23.md)
installs the verified release in core and media, preserving configuration and identities.
Trash Amp must consume the matching shared
client revision when rebuilt; sibling compatibility is checked separately from
changing its existing uncommitted work. HL-12 and HL-05 retain those external checks.

## LAN candidate selection — source repair accepted, 2026-10-23

The deployed beta.4 discovery could omit the host LAN address after sorting and
truncating many Docker interfaces. A source repair now prioritizes the kernel's
selected private source and ordinary interfaces before known container bridges,
deduplicating before the eight-candidate bound. Explicit binds, private address
eligibility and both clients' pin/identity checks remain. The user confirmed the
explicit NAS bind restores Trash Amp. Five new regression cases, full workspace
checks, real Chrome playback/seeking on explicit and wildcard listeners, and the
web smoke passed. The repair is included in the later direct-media NAS source
candidate and published beta.5; the explicit working bind is retained and wildcard
LAN discovery is not requalified on that installation. See
[implementation](automatic-private-playback.md) and [dated evidence](validation/lan-candidate-selection-2026-10-23.md).

## Build-once container publication — delivered and qualified, 2026-10-22

The existing release workflow now separates validation from publication. Validation
builds labelled native AMD64/ARM64 OCI exports once, imports those exact runtime
configs for the existing recovery/inbox/proxy smokes, verifies their merged index
and retains a 30-day immutable Actions artifact. Publication restores only a
successful trusted same-source validation, checks the artifact/archive digests and
platform/source evidence, and copies those bytes to GHCR without compilation.
Anonymous pulls, channel downgrade guards and pinned installation assets remain.
Missing/expired/mismatched artifacts fail closed; manual reuse rehearses acceptance
without publishing. See [operator contract](container-release-pipeline.md) and
[qualification record](validation/container-promotion-2026-10-22.md).
The first actual release using this path, [beta.5](validation/container-beta5-publication-2026-10-23.md),
successfully reused the accepted native exports without rebuilding.
Full exact-source validation (all source checks, native AMD64/ARM64 identity and
recovery/proxy/inbox smokes, merged OCI inspection) and a same-source reuse
rehearsal passed. The rehearsal skipped source checks and native builds, consumed
the current successful attempt's immutable artifact, retained exact image/platform
digests and generated matching bundle/provenance/checksums. All eleven CI policy/
rootless transport tests passed. Docker store identity and archive portability
failures found during qualification are corrected; history remains in the dated
record. Public registry copy/anonymous/channel steps retain their existing gates
and were deliberately skipped during rehearsal. No new release or deployment
was performed; beta.4/preview and the NAS are unchanged.

**Beta.4 published — 2026-10-22:** the automatic private playback image is verified
ready on `ghcr.io/aberrantwolf/junk-collector:preview`. Both native architectures,
container smoke, anonymous pulls, source labels and pinned installation bundle
passed. See [publication evidence](validation/container-beta4-publication-2026-10-22.md)
and [update/client requirements](releases/v0.1.0-beta.4.md). No NAS deployment was
performed; Trash Amp needs its matching new client build. The subsequent [build-once pipeline delivery](validation/container-promotion-2026-10-22.md)
now reuses validated images for future publication.

## Beta.3 published — 2026-10-22

The reviewed worker/progress/grouping delivery is published as beta.3 from signed
tag source `d80013530ff8147f8a534dba819eeaaaa813be1e`. Exact-source checks,
native AMD64/ARM64 container smoke tests and anonymous public pulls passed.
The bundle pins the verified multiarch digest; `preview` matches it and stable
`latest` remains absent. Rust 1.99.1 is unchanged. Actual beta.3 NAS/Portainer
updating remains an operator check, distinct from publication acceptance.
See [publication evidence](validation/container-beta3-publication-2026-10-22.md)
and [release notes and update behavior](releases/v0.1.0-beta.3.md).

## Automatic private playback — 2026-10-22 source delivery

The [production implementation](automatic-private-playback.md) now covers web and
Trash Amp together: authenticated private endpoint/pin discovery, software-managed
short-lived certificates, pinned browser WebTransport with a streaming worker,
and native HTTPS route selection through the shared range reader. Saved account,
connection, queue and cache identities remain stable. Each range checks current
authorization. Public media fallback defaults to disabled; ordinary control/login
access remains. No Tailscale integration or configuration is involved.

See [dated source acceptance](validation/automatic-private-playback-2026-10-22.md)
for actual playback/seek, trust, revocation, rollover and regression results and
remaining device/deployment limits. Private reachability does not establish a
physical LAN path or zero ISP upload. No new source is deployed to the NAS by
this delivery. The personal `collector-lan.awkwardexplorer.com` address remains the
accepted **manual stopgap** until the updated server and clients are installed.

Earlier feasibility records remain dated history: [primitive transports](validation/direct-transports-2026-10-22.md),
[actual NAS player](validation/webtransport-lan-2026-10-22.md) and
[work-Mac public-origin permission/CSP trial](validation/public-origin-webtransport-2026-10-22.md).
The operator confirmed Chrome playback and forward/back seeks under GlobalProtect,
and both clients using the [manual HTTPS stopgap](validation/lan-https-2026-10-22.md).
Those tests do not qualify every managed VPN, Safari, the new build on a second
device, physical-path billing or the deferred relay/Iroh work. HL-12 retains those
limits; the manual certificate renewal task is separate from automatic ephemeral
media certificate rotation. Existing public hosting/certificate choices are unchanged.

## Automatic multidisc album grouping — 2026-10-22 source delivery

Clear accepted MusicBrainz matches now group existing entries automatically in
the same collection. Every preserved source must have a recorded disc ID that
maps uniquely to a medium; medium layouts must agree across accepted release
variants, and positions must be distinct across entries. Same album/language
variants with coherent positions are allowed. Duplicate positions, incomplete or
conflicting matches, conflicting explicit metadata and unknown inventory/authority
data stay available for reviewed grouping. Filename/artwork similarity is not a
basis for the decision.

Startup and five-second reconciliation traverse all pages of the accepted-album
projection, including metadata accepted before this change. Busy affected imports,
Jobs and accepted bulk members defer a merge; admission/retry and publication
share fences. Existing atomic group publication preserves originals, physical-copy
records, captures, artwork and meaningful evidence. Larger existing destinations
remain stable as later discs arrive. No physical-copy assembly is inferred.

The UI identifies automatic groups and offers existing guarded undo. Metadata
views follow background merges and refresh undo eligibility after choices. Undo retains
a portable separation preference across restart/reindex. Old metadata links follow
the combined entry. Metadata proposals remain available for ambiguous cases.
Capture/artwork identity ordering is now canonical when a group is frozen, fixing
false undo conflicts after hydration/reindex. This supersedes the confirmation-only
behavior in the dated 2026-10-21 delivery below. See
[automatic-grouping acceptance](validation/automatic-album-grouping-2026-10-22.md).

The ambiguity follow-up separates membership from metadata. Entries group when
all viable MusicBrainz album options agree about their membership and distinct
disc positions, retaining unresolved alternatives as tentative metadata. Equivalent
explicit releases/versions no longer conflict solely because their candidate IDs
differ; a stable manual display choice and every original choice/history survive.
An accepted or manually chosen album can admit unresolved siblings when it is the
unique compatible established target. Competing targets and overlapping option
sets retain review. A compatibility graph prioritizes accepted
subsets before unresolved attachments, without letting one conflicting/ambiguous
peer veto the component or choosing arbitrary duplicate copies.

The commit review preserves incomplete/conflicting alternatives as unproven
instead of dropping them to justify a merge. Grouping also honors the same
automatic-metadata reset markers as display; retained history cannot silently
reactivate cleared metadata. Explicit later selection remains supported.
Removed-item recovery preserves already settled import outcomes after grouping,
while continuing to stop unfinished work.

The album-option index is many-to-many (catalog schema 15). Older disposable
catalogs rebuild from archive records while preserving private Jobs/access and
originals. Every assessed neighbor is fenced at publication, including newly
appearing competitors. Bounds remain 200 related entries/32 MiB of metadata per
assessment, 32 entries per publication and 128 preserved sources per entry; larger
or incomplete proofs retain manual review. The reconciler publishes one safe class
per connected component/pass, then reassesses remaining entries on the next pass.
See [follow-up acceptance](validation/automatic-album-grouping-2026-10-22.md#ambiguity-follow-up--2026-10-22).

## Configurable workers, metadata priority and Activity navigation — 2026-10-21 delivered in source

Startup `[jobs]` configuration defaults to four processing workers (allowed 1–8)
and independently caps inbox copying at one (allowed 1 through the processing
limit). Provider concurrency stays one. The execution-loop count and atomic Store
claim limits use the same application settings. Setup and doctor report limits;
invalid values fail before storage opens. Existing pending Jobs and claim fences
survive restart with a reduced limit.

Ready discs get bounded preference for analysis and original publication. With
one worker, that normally completes a disc's originals before the next copy;
provider waits, failures and optional encoding cannot pin a worker. Two processing
admissions offer copying its next turn, allowing one ready publication to finish
a just-analyzed disc first. Other processing receives normal FIFO opportunities
after two preferred continuations. Multiple slots leave room for both waiting
classes; copying keeps its cap even when processing is idle.
Metadata resolution now overtakes an encoding/ordinary-processing backlog for
at most two preferred admissions, shared with ready-disc continuations, before
returning to other processing. It starts after
original publication for its destination album group, without waiting for unrelated
intake members. Album text
and identity commit before optional artwork. Existing accepted album identities
produce grouping suggestions in Metadata → Grouping proposals; combining existing
entries requires review/confirmation and is not an automatic title-based merge.

Work & history now uses aligned compact filters, a collapsible collection chooser,
and selection-dependent bulk commands. Activity Back clears the selected work and
restores its list/filter state; Back/Forward and reload retain their recorded
return context without opening a duplicate modal. Existing library return behavior
is retained. See [configuration](configuration.md#background-workers) and
[dated acceptance and limits](validation/configurable-workers-2026-10-21.md).

## Import queue and active stages — 2026-10-21 delivered in source

The 2026-10-22 commit review also gates import source/review controls on restored
draft state, preventing a late IndexedDB load from overwriting an immediate choice.

The Imports album rows and focused import details distinguish active execution,
queued tasks, provider/upload waits and cancellation. Current stage labels include
inbox copying, source checks, file analysis, original publication, metadata/artwork
lookup and audio preparation; concurrent stages and grouped task counts remain
visible. Shared provider downloads expose the actual provider state and are marked
shared. Track jobs remain visible after their admission batch completes. Existing
two-second polling resumes after reload and continues through optional work.
Inbox step labels say “Copy files from inbox,” rather than “Uploading files,”
and browser upload/resume controls stay out of inbox-only batches. Intake member
rows remain visible on narrow screens, independently of library-detail hiding.

The authorized application summary projects durable Job ownership without payload
reads or integrity scans. Overall failure/completion remains separate from active
work. No import state/storage conversion or worker-capacity change is required.
The initial two-slot dispatch repair is superseded by the configurable-worker
follow-up above; the original two-slot acceptance remains dated evidence. Existing active work finishes before a newly
waiting lane receives its released slot. Provider capacity, cancellation and claim
fencing remain in force. The dispatch preference is transient; no stored-format
conversion is required. Source acceptance and deployment limits are recorded in
[the dated validation](validation/ingest-progress-2026-10-21.md).

## Beta.2 correction — 2026-10-21

**An existing installation was updated to beta.2.** [Update checks](validation/live-beta2-upgrade-2026-10-21.md).

Beta.1 passed source checks and public pulls, then failed the isolated NAS upgrade.
An old music-search table was incorrectly reused. Beta.2 rebuilds obsolete catalogs
and repairs the bad schema stamp. Archive records and private accounts stay intact.
Both native images, anonymous pulls and the isolated DS1522+ alpha.3 upgrade/full restore passed.
All 17 live containers stayed unchanged.
See [release scope](releases/v0.1.0-beta.2.md), [beta.1 failure](validation/container-beta1-publication-2026-10-21.md)
and [beta.2 publication acceptance](validation/container-beta2-publication-2026-10-21.md).

Pre-release evolution follows [the accepted compatibility and contract policy](decisions/pre-release-evolution.md). Earlier checkpoint compatibility is not a release requirement; data preservation and recovery remain required.

Wave 3 review follow-up (2026-09-11; validated review-fix commit): interrupted pre-commit cancellations now
resolve the Job and Import together at startup. Recovery of a committed Capture
finishes publication despite later Item edits, and does not reserve payload space
again. Exact final upload-chunk retries remain read-only after staging completes.
The browser can reselect and resume an interrupted package using its existing
Import identity; it checks retained prefixes before appending, which retransfers
those prefixes for comparison but never rewrites them. Saved reviews can be edited
again after a failed publication, and transient restore failures retain the saved
Import identity. These changes add permanent recovery and browser regression tests.

Use the [capability checklist](release-readiness.md) for current supported scope. The detailed entries below include earlier development stages.

Use the [consolidated backlog](remaining-work.md) for all remaining work and
[release requirements](release-requirements.md) for the revised home-lab gate.
Older dated entries below describe their state at the time, not the current queue.
Completed records retain their historical milestone labels; the 2026-10-19 queue
renumbering is documented in remaining work. Active remaining-work references use
its current IDs.

## Game-disc CD audio — 2026-10-21 delivered in source

Standard BINARY CUE/BIN game packages now record source-bound AUDIO ranges and
reuse FLAC/Opus preparation, listening catalogs, playlists and manual track titles.
Data tracks stay excluded. Games retain their identity, artwork and originals;
no duplicate music import is required. Shared-reader owned-disc, actual owned/browser
and isolated native streaming/offline checks pass, as do all 458 source tests and
required source gates. Trash Amp’s published `codex/game-disc-audio` delivery pins
Collector exactly and passes 32 affected tests plus desktop/Clippy checks. Its
original checkout and staged native release work remain unchanged. See [delivery](game-disc-audio.md) and
[dated evidence](validation/game-disc-audio-2026-10-21.md). This is post-alpha.3
source work, not a change to the published image's advertised formats.

## Focused release review and early body authentication — 2026-10-21

Curation routes now authenticate before JSON/byte body extraction; administration
routes also enforce application administrator policy first. Collection-specific
and handler authority checks remain current. A reproducing no-body-poll regression,
all 32 HTTP tests, 455 workspace tests, strict Clippy and actual Chrome workflows
pass. This post-alpha.3 source repair and its bounded review are recorded in
[acceptance](validation/ingress-review-2026-10-21.md). The
[release capability checklist](release-readiness.md) links accepted evidence and
keeps broader native/owned-format and game-audio scope explicitly incomplete.

## HL-09 explicit inbox source cleanup — 2026-10-21

The import requester's Work & history detail now offers explicit inbox cleanup
after committed preservation and settled processing, with confirmation, typed
progress/failure/cancellation and retry. Bounded background checks verify actual
archive originals and frozen source identities/checksums before deletion. Durable
quarantine/per-file intents preserve restart inputs and protect replacement source
names. Extra content stays retained; source files remain by default. Active cleanup
pauses new inbox admission, including aliased roots. Optional processing can retry
from the archive. Private schema 21 preserves previous data; downgrade needs the old
complete recovery point. The packaged bind remains read-only unless explicitly
changed through the new environment option. See [delivery](inbox-cleanup.md) and
[local real-browser/recovery evidence](validation/inbox-cleanup-2026-10-21.md).
Public alpha.3 packages this feature; both native container jobs and the isolated
actual-NAS Portainer upgrade/cleanup/full restore passed. See [publication acceptance](validation/container-publication-2026-10-21.md).

## HL-10 portable full archive export/import — 2026-10-21

Local `export-archive`, `validate-archive-export` and `import-archive` commands now
copy a frozen full portable archive with digest inventory and completion seal.
Originals, artwork, manual authority, evidence/history, lists, recipes and unknown
records stay intact. Operational locks/scratch, configuration and private account/
grant/session/server state are excluded; unresolved staging is refused. Imports
use empty roots, fence interrupted copies and require fresh setup plus explicit
first-Owner recovery. The real compiled CLI journey, byte equality, cold reindex,
fresh identity/access and focused interruption/change regressions pass. See the
[operator delivery](archive-export.md) and [dated acceptance](validation/archive-export-2026-10-21.md).
Public alpha.3 includes this feature; see
[publication acceptance](validation/container-publication-2026-10-21.md). Deltas, existing-archive
merge and Retro Junk conversion remain unimplemented. Portable records can include
private collection facts; this is not a redacted sharing feature or full backup.

## Public-ingress qualification scope — clarified 2026-10-21

Use the existing configured Cloudflare hostname and a short real-origin smoke,
reusing backend/isolated tests and operator-reported sign-in/playback. An additional
tunnel or exhaustive repetition of configuration/failure/load matrices is not a
home-lab release prerequisite. The live hostname passed normal trusted Chrome
sign-in-page loading without script errors and typed anonymous account/admin API
401 denials. No authenticated media journey was independently repeated or production
state changed. See [live observations and limits](validation/synology-deployment-2026-10-19.md#lightweight-live-https-check--2026-10-21)
and [scope policy](decisions/home-lab-scope.md#security-proportional-to-a-public-home-server).

## Backup operating scope — clarified 2026-10-20

The packaged container initializes `${COLLECTOR_DATA}/backups` (on the inspected NAS,
`/volume1/junk-collector/backups`). Backups remain explicit commands with a fresh
named destination; the implemented full backup includes media. No automatic media
backup schedule, cloud account, independent destination or lightweight database-only
backup is required for release. Local consistent points provide practical update
rollback; optional independent copies retain their qualified runbook.
The [accepted scope](decisions/home-lab-scope.md#local-recovery-and-optional-media-backups--clarified-2026-10-20)
removes the earlier destination-selection blocker. A small catalog/settings backup
would need more than SQLite alone; that mode is not implemented. Optional backup-age
reminders are delivered in post-alpha.3 source; see below and the earlier
[clarification validation](validation/backup-scope-2026-10-20.md).

## Numeric-IP authentication startup — repaired and NAS-qualified, 2026-10-20

Numeric-IP HTTPS authentication origins now support startup, password login and
recovery while explicitly reporting that passkeys require a trusted DNS hostname.
DNS passkey registration remains available. Setup/doctor/serve share strict origin
validation before storage writes; features/security guidance uses typed availability.
Real executable identity/TLS/session continuity and Chrome login/recovery/mobile
acceptance passed. An unpublished native candidate also passed numeric-IP HTTPS
startup/login and full browser/recovery acceptance on the actual NAS. See
[repair evidence](validation/numeric-ip-authentication-2026-10-20.md).
The published alpha.2 image predates this source repair; no production installation
or public image was replaced.

## Metadata save after storage permission loss — repaired and NAS-qualified, 2026-10-20

An isolated NAS permission-loss test exposed a failed manual-save reservation that
could conflict with a later successful retry and block restart/cold backup. Source
now releases provably uncommitted metadata reservations under the writer lock and
recovers an older losing reservation only with an independently attested immutable
winner. Unexplained digest changes and inconsistent responses still fail closed.
No archive format/schema changed. Rebuilt-server Chrome failed-save/retry, exact
originals, playback and full backup/restore passed with session/identity/TLS/manual
authority continuity; all 440 workspace tests and strict Clippy passed. See
[repair and acceptance scope](validation/storage-permission-recovery-2026-10-20.md).
An unpublished native candidate passed actual-NAS Btrfs permission loss, browser
retry without an intervening restart, cold backup and full restore, also using a
numeric-IP HTTPS origin. Identity/session/TLS/manual authority and exact originals
survived; fixture cleanup and unchanged live services were verified. The published
alpha.2 image predates these fixes. Parent HL-05 remains partial.

## Bounded archive ENOSPC recovery — qualified, 2026-10-20

The published alpha.2 runtime passed a real NAS/Chrome archive-full save failure,
unchanged originals/committed metadata, space reclamation, normal restart, successful
manual retry and full backup/restore. The fixture verified a dedicated 96 MiB tmpfs
before filling it; the real Btrfs disk was never filled. Session/ServerId/TLS/manual
authority and exact originals survived. Generated resources were cleaned and live
services matched baseline. This qualifies bounded archive ENOSPC, not full private
SQLite/Btrfs, lost power or public ingress. See [HL-05E acceptance](validation/storage-capacity-recovery-2026-10-20.md).

## Authentication rate-limit identity retention — source repaired, 2026-10-20

Rejected alias rotation can no longer grow the authentication/enrollment/poll
identity maps, and expired inactive identities are reclaimed. Retained identities
are bounded by admitted attempts in the current global window. Existing thresholds,
hashed account keys, direct TCP peer selection and 429 behavior are preserved.
Advancing-clock/flood regressions and a real executable/TCP account/peer/header/
restart journey passed; all 443 workspace tests, strict Clippy and full Chrome smoke
passed. This is bounded source acceptance, not public-ingress/load certification or
a published image. See [HL-05F evidence](validation/rate-limit-retention-2026-10-20.md).

## Actual NAS authentication thresholds — qualified, 2026-10-20

An isolated published-alpha.2 NAS installation passed actual Linux TCP account,
direct-peer and global login ceilings using distinct loopback source addresses.
Both listeners share limits; account/forwarded-header rotation cannot bypass the
peer ceiling. Existing sessions work while login is throttled; restart clears
transient limits and preserves accounts/sessions. Real Chrome playback/download/
mobile and full backup/restore passed afterward, with exact originals and identity/
TLS retained. Cleanup and unchanged live services were verified. This does not
qualify external clients, public ingress or sustained load, or ship the newer
limiter memory fix. See [HL-05G acceptance](validation/nas-authentication-limits-2026-10-20.md).

## Physical Safari LAN browser journey — qualified, 2026-10-20

An isolated actual-NAS alpha.2 installation with synthetic FLAC/MP3, artwork and
a test Viewer passed named TLS/CA/ServerId validation and real Chrome preflight.
The operator's iPhone 16 Pro (reported iOS 26.6.2) passed physical Safari trust,
artwork, FLAC/MP3 playback/seek and persisted Curator track editing. Viewer editing
controls disappeared after demotion. Grant removal stopped playback and denied the
album; the loaded player also refused replay while access stayed revoked, although
its personal queue label remained. Revoking the browser session returned Safari to
sign-in; fresh sign-in worked. Independent API write/media/range/HEAD/session denials
passed. The separate temporary-CA test installation and secret handoffs were cleaned,
with live services unchanged; device profile removal remains an operator action.
Parent HL-04 remains partial for native/offline/failure qualification. Preparation
also exposed a numeric-IP authentication-origin startup failure in WebAuthn; using
a DNS-name origin works. The source repair is recorded above; the published alpha.2
image still predates it. See the
[dated acceptance record](validation/lan-device-preparation-2026-10-20.md).

## Real-tool audio oracles — HL-06A qualified, 2026-10-20

Published alpha.2 passed actual NAS FLAC PCM equality and configured Opus
96/128/192 kbps preparation using flac 1.4.2 and opus-tools 0.2 / libopus 1.3.1.
Independent decoding recovered all expected FLAC samples and exact Opus frame
counts with distinct-channel signal checks. Chrome prepared/downloaded/played/sought
each Opus-only item; full backup/restore retained the exact audio identities/bytes
and repeated the independent Opus/browser checks. Test handoffs now publish complete
private JSON atomically. Live services were unchanged and fixture resources cleaned.
See the [current audio matrix](audio-preparation.md) and
[bounded evidence](validation/audio-oracles-2026-10-20.md). This is single-track
synthetic qualification, not owned-disc/catalog, gapless, bit-perfect device,
native Opus or additional original-codec acceptance. Parent HL-06 remains partial.

## NAS preparation recovery — HL-05C qualified, 2026-10-20

The published alpha.2 image passed real NAS cancellation during encoder execution,
failure after real FLAC output, and SIGKILL/restart of active encoding. Cancelled/
failed work published no audio; cancellation persisted across restart and generic
retry was fenced. Explicit web retry and crash recovery produced one playable
output while preserving originals and Job identity. Full restore retained those
batch outcomes/audio identities, account/session/config/TLS; real Chrome playback,
seek/download/mobile checks passed. Live services matched their baseline and test
resources were cleaned. See [bounded acceptance and reproduction](validation/nas-preparation-recovery-2026-10-20.md).
This qualifies existing runtime behavior using fixture controls, not a new runtime
image. Parent HL-05 public ingress, storage failure/abuse and physical-device/trust
qualification remain open.

## Published Docker alpha.2 — HL-07D delivered, 2026-10-20

The signed `v0.1.0-alpha.2` source tag and GitHub prerelease now supply public
AMD64/ARM64 images and a digest-pinned installation bundle. Version and `preview`
resolve to `sha256:f86f46d2c8870f8d647fc9225ce2cfe7c4319521a29c89897b419abb77629433`; stable `latest` is absent.
Native platform builds and actual native recovery smokes, all source gates,
anonymous all-layer/platform/source-label checks and both-platform Docker pulls
passed. The existing workflow now uses native runners after prolonged QEMU release
optimization; explicit retries freeze an existing published tag's source. Normal
empty-input manual dispatch remains nonpublishing. Channel promotion cannot
supersede a newer release in its channel.

Actual disposable Portainer 2.45 NAS acceptance passed a distinct-image old→public
version update while retaining an unchanged companion, and fresh digest install/
forced re-pull/redeploy/full restore. Saved original bytes, accounts/session,
ServerId, config/TLS and unfinished upload survived. Real Chrome playback/seek/
download/mobile checks passed after update and restore; independent native ARM64
startup/HTTPS/clean shutdown passed. All test resources/private handoffs were
cleaned; live Collector, Portainer and both tunnel identities/start times matched
the baseline. See [publication acceptance](validation/container-publication-2026-10-20.md)
and [alpha.2 notes](releases/v0.1.0-alpha.2.md). The dated
[preparation record](validation/container-release-preparation-2026-10-20.md) stays historical.
Direct alpha.1 upgrades, system-trusted second-device/public-ingress/remaining
format qualification remain outside this packaging delivery.

## Scalable update recovery — HL-05B, 2026-10-20

The actual DSM Collector root is an eligible single Btrfs subvolume. A qualified
[operations path](../deploy/container/snapshot-recovery.md) captures all cold roots
without copying/hashing masters, then exports GNU tar baseline/increments while
the service runs. A narrow adapter binds tar's cutoff to snapshot time, retaining
writes made between capture and export. Full CLI backup remains unchanged.

Isolated actual-host acceptance passed SIGKILL/restart and whole-point old-image
rollback with a pending upload, a NAS→Mac→NAS independent chain, complete cold
file/permission comparisons and recovered account/session/TLS/original/audio checks.
Chrome playback/seek/downloads passed before and after restore. The 32 MiB unchanged
payload was excluded from the 3.28 MB increment; the baseline was 34.51 MB.
See [dated evidence](validation/snapshot-recovery-2026-10-20.md). No production recovery
configuration was changed. Independent copies are optional under the clarified
operating scope; destination selection does not block release. Generic/DSM Btrfs incremental receive through the isolated bind
mount failed and is not accepted. Public pull/redeploy is accepted under HL-07D;
power-loss remains open.

## Reproducible NAS operations — HL-05A, 2026-10-20

Fresh-install proxy configuration uses guarded, byte-checked atomic stdin transfer.
The runbooks create DSM bind roots before initialize/restore, distinguish inbox ACLs
from mount writability, supply a shared supplemental-group override, and keep
Portainer runtime/SSH CLI image IDs aligned without removing tunnel services.
Startup/doctor distinguish configured provider contact from network/match success.

On the actual DS1522+ Btrfs host, isolated amd64 installations passed distinct-image
update with resumed partial upload, retained account/session/ServerId/config/TLS,
exact original downloads, inbox group access, explicit FLAC preparation, clean
SIGTERM and full stopped-server restore. Real remote Chrome exercised album/track
play, seek, persistent playback, mobile layout and exact downloads before and after
restore. Configured provider acquisition completed; absent configuration left a
visible metadata failure without blocking originals/audio. See [dated evidence](validation/nas-operations-2026-10-20.md).
The live Portainer installation and tunnels were inspected only. Permanent-installation
inventory/enrichment acceptance, public ingress, system-browser trust, power-loss
behavior remain open; HL-05B's isolated snapshot/incremental path is now qualified
separately above. Choosing optional independent media-backup storage is not a
release prerequisite. Public registry pull/update acceptance is now qualified
under [HL-07D](validation/container-publication-2026-10-20.md).

## Release/setup wording — HL-07C, 2026-10-20

Current startup/doctor, enrollment and certificate help describe web/Trash Amp
with its actual Settings → Servers URL/trust-file/Connect controls. Browser trust
and native approval remain separate. UI and operator instructions use saved/kept
for stored files and precise protection/reset wording for existing accounts, TLS,
manual choices and collection history. README intake/listening instructions match
the Imports/Prepare audio pages and persistent player. Technical storage/state
identifiers and dated Phono evidence are unchanged. See [dated acceptance](validation/release-wording-2026-10-20.md).
HL-05A installation/update operations are now accepted separately above; scalable
recovery remains HL-05B.

## Direct web playback — HL-07B, 2026-10-19

Album and preparation views expose Play album, per-import Play original and
individual-track Play. Explicit Play replaces the personal queue and starts
immediately; Add to queue remains available separately. Whole-album expansion and
numeric order reuse the existing authorized client playback snapshot. Fresh
availability requires a browser-decodable existing format for every selected track;
unavailable or ambiguous selections keep the old queue and show recovery guidance.
Digital alternate-encoding choices retain their existing metadata authority.
Closing/replacing a panel cancels its reads before they can change a newer queue.
See [dated validation](validation/direct-playback-2026-10-19.md). No Play action
starts encoding, and the persistent player's permission/revocation checks remain.

## Multi-disc inbox review — HL-07A, 2026-10-19

Batch import review now exposes CUE/Redumper anchor selection through shared
readiness, preserving file visibility and destination choices during replanning.
Album group labels are independent of optional manual titles. Physical-copy labels
are explicitly reviewed: matching labels within an album/intake identify one copy,
different labels identify separate copies. Numeric per-disc defaults are removed.
Known `.DS_Store` files and `#recycle`/`@eaDir` directory subtrees are excluded from
new inbox/browser intake. Unknown hidden files remain originals, and recovery of
previously accepted inventories retains declared housekeeping files.

Real Chrome inbox workflows cover automatic title/artwork, manual-title authority,
one-copy and separate-copy grouping, retained file privacy and exact original
downloads, draft reload and server restart. Existing grouped-publication failure/
retry and manual-inventory fences remain covered. See [dated acceptance](validation/intake-review-2026-10-19.md).
The operator's permanent NAS acceptance remains HL-05; optional source cleanup
remains unimplemented under HL-09.

## Digital FLAC/MP3 purchases — delivered 2026-10-19

[HL-02](remaining-work.md#hl-02--complete-flac-and-mp3-digital-purchase-support)
has coordinated Collector, junk-libs and Trash Amp implementations. Digital
acquisition and per-file visibility are portable; file sources have immutable
native-format track manifests, descriptive tags/artwork and direct original
playback without encoders or manufactured physical copies. Web intake/review,
search, queues, playlists and source-scoped track edits use the same contracts as
native streaming, download and retained offline metadata/artwork/playback.
Private companions default to curators, and private audio cannot publish tags/art.
Alternate encodings require explicit album inclusion choices.

See the [supported matrix and retention/compatibility policy](digital-purchases.md)
and [dated checks](validation/digital-purchases-2026-10-19.md). Real Chrome journeys,
independent codec/tag vectors, restore/rebuild and native TLS/device-callback
qualification are recorded there. The actual native GUI passed browse/search,
artwork, both original formats, seek/next/queue, downloads, connection removal and
server-stopped offline reopen/playback. HL-02 is delivered. Authorized coordinated
publication replaces local Cargo patches with exact pushed sibling revisions,
recorded in manifests and lockfiles; no sibling checkout is required.
Further codecs, DRM and album-image splitting remain deferred.

## HL-08 AccurateRip contribution investigation — 2026-10-18

The bounded [contribution investigation](accuraterip-contribution-investigation.md)
is complete. It recommends EAC or dBpoweramp on Windows with the physical disc,
AccurateRip drive calibration, secure full-disc extraction and the ripper's own
consented submission workflow. Maintainer statements distinguish those submitting
clients from Mac and file-verification tools; CTDB contribution is a separate
service. Collector's DBAR acquisition and source-bound comparison evidence do not
supply a supported write contract or attest to a calibrated physical rip. Direct
integration remains blocked pending that contract and typed provenance policy.
Sending results does not verify audio; later self-matches are not automatically
independent corroboration. See the [dated validation and limits](validation/accuraterip-contribution-2026-10-18.md).
No uploader was implemented, disc re-ripped, live archive changed or result submitted.
The external workflow was documented, not runtime-qualified.

## HL-08 Library return after nested work — 2026-10-18

Library “Back to results” now includes nested Work & history list/detail entries
in the distance to the originating results. Repeated modal visits return to the
loaded pages, scroll position and focused row. Intentional browser Back/Forward
still restores work views, and direct detail links retain their local return
fallback. A real-server Chrome regression reproduces the previous modal reopening
and covers both navigation intents, including same-document restoration after the
modal unmounts and the identity of the restored work. Self-review found no further
production-code defect. See the
[dated validation](validation/library-work-return-2026-10-18.md). Other HL-09
conveniences, game-disc listening and performance follow-ups remain open.

## HL-03 stopped-server backup and restore — 2026-10-18

The CLI supplies `backup`, `validate-backup` and `restore`, including explicit
archive-only recovery. The conversion backup implementation is shared by the
general workflow. Both roots are locked, startup settles journals and SQLite is
checkpointed before a complete copy of archive/private state, TLS/config and
pending-work scratch. Validation checks the sealed inventory and complete file set
without opening databases. Empty-root restore rechecks copied bytes, publishes
relocated configuration last and fences interrupted roots before normal startup.
Full recovery retains identity; archive-only recovery requires fresh setup and
explicit grants. The HL-03 review repairs add stopped-server `unowned-collections`
discovery and `recover-collection-owner` for the first explicit Owner assignment;
ordinary web grants still require an Owner. Existing Owners and disabled targets
are protected, and repeat assignments preserve the grant revision. The private
grant/audit commit uses the same grant effect as normal access changes. Restore
now recreates lost original lockfiles in writable surviving roots while preserving
live-writer refusal and read-only backup inputs. Compiled-CLI regressions cover
both findings, including normal access after local recovery. External inbox/DAT/tool paths are reported for review. See the
[operator runbook](local-alpha-installation.md#stopped-server-backup) and
[dated validation](validation/backup-restore-2026-10-18.md). No live archive was
backed up, restored or converted. Online backup, scheduling, export/deltas and a
dashboard remain outside this delivery. Linux deployment qualification remains HL-05.

## HL-01 cover reuse review repairs — 2026-10-18

Provider cover creation/reuse now converges under the archive writer for concurrent
curators, with matching-content, Item-generation and live-claim checks. Concurrent
identical cover selections preserve one committed authority change. Explicit CAA
image selection restores an archived cover through a new revision of the same Asset,
retaining its tombstone, bytes, visibility and alternative text. Automatic refresh
and older requests admitted before a later archive cannot restore it. Restore
permissions persist with the admitted Job across restart. Stale restore revisions and mismatched content/ownership fail
closed. See [repair validation](validation/retention-review-2026-10-18.md) and the
[updated retention policy](retention-audit.md). No live archive was changed.

## Bounded retention audit and CAA response reuse — 2026-10-17

The [audit](retention-audit.md) maps current storage/consumers to must-keep data,
bounded response/cache state and obsolete blanket lookup-history requirements.
CAA listings, images and negative responses now reuse exact validated equivalent
snapshots through the existing acquisition/publication pipeline. Automatic cover
Assets bind item and image Snapshot, independent of requester, preventing both
cross-item ownership collisions and repeat materialization by another curator.
Changed artwork keeps a distinct source/Asset. Existing Assets and archive IDs are
preserved. The explicit stopped-server backup/inspect/apply conversion now includes
proven duplicate CAA payloads, with the same recovery and unknown-data guards.

No live cleanup, general history deletion, command-receipt expiration or spool purge
was performed or implemented. Private Jobs/receipts still accumulate under their
existing replay contract; response reuse does not bound that database. General
operator backup/restore remains HL-03. See [dated validation](validation/retention-audit-2026-10-17.md).

## Automatic standard-game identification — 2026-10-10

Standard DAT/ZIP batch review detects No-Intro/Redump conventions and recognized
systems, without required technical selections. Scoped active slots queue durable
identification scans of existing entries; guided imports and Identify game use the
same application service. Complete unambiguous catalog evidence applies automatic
titles, edition details and systems. Partial/competing matches stay reviewable;
malformed/unsupported recognized input cannot fall through to raw identity.
Provisional import labels and deliberate per-field overrides are distinct, with
Use detected controls and game terminology. Unsupported historical settings remain
inactive and explained; originals and existing evidence are preserved.

The [supported matrix](game-identification.md) limits the claim to existing byte
interpretations and recognized standards. CHD/UNIF/compressed game containers,
arbitrary systems and new normalization algorithms remain unsupported/deferred.
[Delivery validation](validation/game-identification-2026-10-10.md) records format,
archive bounds, durable recovery, publication fences and browser acceptance.

## Current scope and remaining work — 2026-10-09

Web and Trash Amp are the client targets; Phono Junk is no longer a target.
PJ1 and Phono-specific GUI/device acceptance are retired, not passed. Existing
shared account/client/transport software remains implemented. Remaining useful work
is grouped only in the backlog: contract/retention simplification, web/Trash Amp
LAN acceptance, practical backup tooling, actual Linux/public-ingress qualification,
independent evidence for advertised formats and a bounded release audit. Optional
features and advanced security remain there with reasons for deferral.

Old proposed contracts require reassessment. Repeated artwork/metadata lookups do
not require permanent provenance history. The current append-only envelopes and raw observations still exist. The later
2026-10-10 and 2026-10-17 deliveries reuse equivalent responses and provide explicit
duplicate-payload conversion; general record deletion remains unimplemented. See [the accepted scope policy](decisions/home-lab-scope.md).

## Game-disc listening scope — 2026-10-10 (historical)

Listening to CD audio tracks preserved with games through the normal music
interface and Trash Amp is requested future work, not an implemented capability.
Preparation currently requires a music collection and does not support mixed-mode
audio extraction. [HL-09](remaining-work.md#game-disc-audio-in-music-listening-and-trash-amp)
owns the complete cross-module preparation, discovery, playlist and native/offline
delivery; no duplicate music import or proprietary game-audio decoding is required
by that initial scope.

## Compact album format availability — 2026-10-09

Album browsing and preparation use small inline FLAC/Opus labels beside track
names instead of full bordered availability boxes. Ready downloads show a check,
preparation buttons show a plus, active work an ellipsis, and unavailable formats
a muted minus with keyboard/touch-openable reason details. File sizes remain in
accessible download labels and hover text. Track/list/queue actions stay available;
format preparation is visually secondary. Album browsing now has a small cover/title
header and two-line track rows: one ellipsized title line, one small format line.
Native overflow popovers hold album management, list additions, track queue actions
and refresh. Disc checks, collection facts, metadata details, evidence and stored
files are collapsed until explicitly requested; unsupported playback guidance stays
in an inline help control. The dedicated preparation view retains its actual plan
and progress controls. The desktop entry panel now meets the
viewport right edge and player dock without page gutters; its full-height scroll
area stays fixed while the library scrolls. Mobile retains the ordinary page layout. See the
[dated validation](validation.md#compact-album-format-availability--2026-10-09).

## AccurateRip browsing shortcut — 2026-10-09

Album rows replace the independent legacy Identity flag with AccurateRip results
for their visible, currently associated preserved originals. Unchecked, verified,
mismatch, no-data, incomplete-search, ambiguous-offset, unavailable and unsupported
outcomes stay distinct; multiple originals show the verified fraction when any pass.
Mismatches remain explicit; mixed unsuccessful outcomes are inconclusive. This does
not assert completeness of album disc coverage. The typed presentation reads
recorded observations against committed source fingerprints without reading or
hashing audio. Curators can open the checks directly from the status column or
Album management → Check audio with AccurateRip; per-disc shortcuts focus that
original. The existing processing workflow owns snapshot acquisition and check
submission. Readers see results without write controls. See the
[dated validation](validation.md#accuraterip-browsing-shortcut--2026-10-09).
Real owned-disc/live AccurateRip qualification remains outstanding in HL-06.

## AccurateRip response selection repair — 2026-10-09

The check panel previously offered collection-wide AccurateRip responses and could
default to another disc's response, causing the worker to reject the query before
checking audio. Single-anchor packages now get a server-computed query from current
recorded structure/import analysis. Admission rejects mismatched references; the
panel chooses only responses matching track count, both AccurateRip IDs and CDDB
ID, repairs a stale saved choice, and loads older snapshot pages. Missing matches
show fetch guidance. Multiple-anchor packages keep explicit choices and the strict
worker check. No checksum algorithm, saved evidence or originals changed. See
[validation](validation.md#accuraterip-response-selection-repair--2026-10-09).
This does not complete HL-06 real owned-disc/live-provider qualification.

Follow-up recovery fix: empty response selections retain the source chooser and
show one actionable prerequisite instead of a UUID parse failure. Scan plans expose
the usable structure observation; acquisition accepts imported structure under the
same validation as a dedicated check. The fetch action is therefore available for
import-only layouts even when history is paginated. A missing layout has an explicit
Inspect CD structure action. Existing dedicated-check acquisition stays available.


## File-check progress and development audio performance — 2026-10-09

Active processing operations now carry typed indeterminate progress instead of
presenting an untouched source-byte counter as zero completion. Actual worker
phases distinguish source validation, AccurateRip checksum/offset computation,
final source recheck and recording the result. Both the check panel and Work
history use an indeterminate meter; other measured workflows retain counters.
Shared disc decoding and AccurateRip loops are optimized in development builds,
which previously ran unoptimized on the home-server debug binary. Algorithms and
coverage are unchanged, and running Jobs are not interrupted by this code change.
See [validation](validation.md#file-check-progress--2026-10-09).

## October 9 TODO delivery

Game and other non-music imports no longer offer FLAC/music controls or album-only
suggestions. Package plans return typed music-action eligibility; music retains
its preparation controls. No-active-catalog import results explain saved originals,
unavailable identification and that no catalog match was attempted.

Administration adds manual DAT download guidance, bounded browser upload, parsed
platform/version/entry review and explicit activation through the existing durable
catalog pipeline. Exact bytes and historical snapshots are retained; ZIPs require
manual extraction, failed validation cannot activate a snapshot, and prior games
can be checked afterward. See [the contract](game-catalog-upload.md) and
[validation](validation/october-todos.md). No provider downloader was added.

The tested checkout/CI/Docker baseline advances to Rust 1.99.0, with matching local
pin/build guidance and no Collector lint suppressions. Shared-library and consumer
tests pass; separate strict-Clippy findings in unchanged junk-libs remain recorded.
The dated alpha.1 Rust 1.95 evidence remains historical. Relevant LA3/Trash Amp
qualification remains open; Phono-specific gates are retired under the current scope.

## Experimental source alpha — 2026-10-09

`v0.1.0-alpha.1` is the first experimental source-release checkpoint, including
Git-consumable `collector-client` and server crates for Trash Amp builds/tests.
The [release notes](releases/v0.1.0-alpha.1.md) state installation, pinning, backup
requirements and the remaining qualifications. This tag does not complete the remaining installation/client acceptance,
promise stable interfaces/storage upgrades, or claim the planned first release
complete. No prebuilt binary or registry image is supplied.

## Shared-library release baseline — 2026-10-09

Actual-host continuation (2026-10-19): the Synology DS1522+ x86-64 container
passed owner login, import/download byte comparisons, recreated-container
session/account/original persistence and a full stopped-server backup/restore
with restored-original SHA-256 comparisons. The default SIGTERM defect is corrected
in source; SIGINT remains the workaround for the older tested image. The
[Compose install/update package](../deploy/container/README.md) and versioned
image/bundle release workflow are implemented; publication and actual DSM
pull-based update qualification remain incomplete. Automatically built, publicly
pullable amd64/arm64 images with version/digest references and documented moving
release channels are now an explicit release requirement (2026-10-19), tracked
under HL-07; users must not need to build/export/transfer images themselves.
Actual DSM acceptance of the new package remains open. Evening operator continuation established
a running Portainer-managed amd64 installation, a dedicated host-network Cloudflare
connector and working browser access through the configured public HTTPS hostname.
The SMB inbox initially denied UID 10001; supplemental DSM group access corrected
directory listing. Three raw-CD packages pass import preview; subsequent retry
recovery/playback reports are recorded below, while complete grouping, enrichment
and new-installation recovery acceptance remain pending.
The grouped-disc Save originals conflict has a source repair: publication can
advance a stale expected revision only across proven same-intake sibling Evidence
additions with unchanged inventory/metadata and prior references. Local tests cover
publication, persisted failure/restart/web-API retry, original bytes and manual-edit
conflicts. The operator reports successful normal-terminal workspace tests and
amd64 image build, plus transfer/load on the NAS; detailed logs and image ID have
not been collected. The operator subsequently provided successful backup/validation
output (401 files) and reported that the fix worked after the instructed Portainer
update and existing-import retry at 23:13 JST, followed by working playback at
23:15 JST. Final inventory grouping, metadata/artwork, explicit seek and
original-byte comparisons remain pending. The current
full-copy backup CLI remains available, but a scalable
routine-update process using qualified consistent NAS snapshots and incremental
independent backups is not delivered. The user explicitly requested addressing
this before full-archive copies become an update burden at collection scale;
actual NAS snapshot support and restore behavior remain to be checked.
See [repair validation](validation/grouped-disc-publication-2026-10-19.md).
Batch review lacks its multiple-anchor selector and couples album grouping to a
manual title; housekeeping filtering and requested successful-import inbox cleanup
are not delivered. The [work ledger](remaining-work.md#placement-of-the-2026-10-19-live-use-findings)
now assigns import-review repairs to HL-07A, direct Play actions to HL-07B,
wording to HL-07C, public Docker release automation to HL-07D, deployment/recovery
to HL-05A/B, and optional inbox cleanup to HL-09. HL-07A is the next code delivery;
the completed grouped-publication repair is not reopened. Observations remain in
the dated NAS record below.
Packaging changes and available checks are recorded in
[container packaging validation](validation/container-packaging-2026-10-19.md).
The operator guide now uses generic storage/host examples and leads with an
external HTTPS proxy/tunnel configuration; direct TLS remains an alternative.
Historical local installation observations are retained separately from setup.
See [partial qualification](validation/synology-deployment-2026-10-19.md).
Interrupted work, filesystem failure and public-ingress acceptance remain open
under HL-05.

Collector's seven `junk-libs` dependencies now use the published `v0.1.0` Git tag;
`Cargo.lock` freezes commit `a0abcdae1bafec8793ce77111f58aa9ab5675c4e`.
Normal builds no longer require a sibling checkout. The Docker builder includes
Git/CA certificates for dependency acquisition, and CI explicitly builds the
release server after its checks. The Linux ARM64 Docker image built and passed
non-root setup, authenticated HTTPS/web serving and recreated-container persistence
on Docker Desktop. Validation and wider deployment limits are recorded
in [validation](validation.md). This dependency baseline does not complete installation/client acceptance
or establish a supported Collector release/storage contract.

## Collection lists — locally accepted (2026-10-03)

The [CL1 contract](collection-lists-plan.md) adds collection-owned portable lists,
track-based music playlists, album/disc expansion, ordering, and actual playback
through the persistent player. Other collection lists contain Items. Shared-list
roles, atomic revision/receipt recovery and typed availability are implemented.
Repository gates and real Chrome playlist/shuffle, mobile and keyboard journeys
passed; evidence is recorded in [validation](validation/collection-lists.md).
2026-10-04 review repairs keep chooser destinations visible through search, fence
obsolete pagination, retain unresolved create/append receipts through reload/reopen,
and use shared dialog behavior. Disc/album additions reject incomplete track scopes
instead of silently omitting unavailable tracks. See the dated follow-up in the
validation record for current checks. Future video playback, smart/personal lists
and synchronized listening are separate.

## Bulk library workflows — locally accepted (2026-10-03)

The [bulk library delivery ledger](bulk-library-plan.md) records BL1 shared metadata
resolution/review, BL2 durable intake/confirmed grouping and BL3 bulk FLAC preparation
and Activity. Dedicated task pages, persisted metadata language, authoritative computed
disc matching, manual authority preservation, grouped archive commits/guarded undo,
frozen selections, rolling admission and per-member recovery are implemented.
See [validation](validation/bulk-library.md) for the repository gates, real browser
journey, publication interruption matrix, generated 2,400-package/album measurements
and separate live response qualification. Multi-terabyte codec processing, NAS and
second-device qualification are not established by those local tests.

## Historical local-alpha target — 2026-09-16

D06.3 is committed as `3a6143b`. The accepted [local-alpha plan](local-alpha-plan.md)
now takes precedence over the remaining wave order. LA1 guided ingestion and
automatic enrichment is reviewed and corrected locally. LA2 album preparation and
usable browsing/playback is reviewed and corrected locally (2026-09-17).
LA3 real local installation qualification is in progress: its installation/restore
runbook, owned-corpus journey, live MusicBrainz/CAA enrichment and actual
stopped-copy/restore drill pass on the selected Mac. The intended second LAN device
remains outstanding, so LA3 is only partially qualified. P05.1 remains paused.
The full first release, native clients and public deployment remain later targets.

## Implemented

- Album-match presentation (2026-10-08): music metadata review defaults to selected/accepted releases and current disc-layout or identifier candidates. Text-search suggestions and older results require an explicit disclosure. Retained provider observations and manual authority are unchanged; this is presentation filtering, not a new identity or verification claim.

- Startup performance follow-up (2026-09-28): startup projects prepared-audio
  metadata without reading its payloads. Derivative GET checks recorded length
  and SHA-256 before transfer and reports a specific player error on failure.
  Debug builds optimize SHA-256, and startup logs cumulative recovery, reindex
  and private-reconciliation times. P06 now reuses the maintained projection on
  unchanged startup and uses indexed graph neighborhoods for ordinary writes
  (2026-10-03). Stale/restored archives and missing/corrupt/incomplete projections
  trigger a full rebuild. Generated 100/1,000/10,000-item debug fixtures record
  startup, edit and browse timings in [P06 validation](validation/p06-indexing.md).
  Warm startup remains a metadata stat walk plus SQLite quick_check. Owned-corpus,
  NAS and R02 hardware scale qualification are not claimed.

- Album and inventory follow-up (2026-09-28, local changes): new guided imports
  record one owned physical copy; manual entries may remain without copies or be
  marked wishlist. MusicBrainz release observations now retain medium-specific
  track titles, and music browsing uses them only for a matching complete source
  layout while preserving manually entered titles. Music curation exposes provider
  lookup beside tracks. Existing provider observations without track lists require
  a fresh MusicBrainz response; unavailable providers or unmatched layouts still
  show numbered tracks. The replacement flow is labelled as original-file
  replacement and keeps its existing capture/history guarantees.

- Web album follow-up (2026-09-20): archived cover choices have authenticated
  thumbnail previews. Cover Art Archive options also show lazy provider previews
  before acquisition, with unavailable/retry states and no automatic archival.
  A curator-only, bounded server adapter fetches 250px thumbnails; editor close controls no longer float across cleared
  sections. Preparation updates refresh the parent Item/Capture display and
  transient polling errors retry. Album preparation saves independently of
  inventory; its error is shown beside the preparation controls. Music selection
  uses recorded source fingerprints for browsing/admission rather than hashing
  the whole disc on every poll and per album track. Background conversion retains
  actual-byte validation before encoding and publication.

- LA3 is partially qualified on macOS 26.6.2 arm64. Five isolated owned-package
  copies cover flat and nested layouts, mixed-mode game data, Redumper/BIN+CUE,
  existing FLAC sidecars and damaged input. Preservation, retained structure/raw
  hash observations, manual title authority, provider outage, interrupted staging,
  two-track FLAC preparation/real playback and local Owner/Viewer denial/revocation
  were exercised. The actual stopped-server copy into separate empty roots matched
  complete archive/state manifests before open, retained config/TLS/server identity,
  accounts and a Viewer grant, resumed four unfinished Jobs and played restored
  media. The source corpus remained unchanged and original/evidence/metadata record
  digests matched. Explicit CUE/TOC selection now survives analysis and preparation;
  shared game input omits the music-only field; the open import panel tracks the
  published association until edited. Live MusicBrainz identified `HOME` by `B’z`,
  CAA published a real cover, manual title authority survived, and the restored
  browser rendered cover/text and played the existing FLAC. The journey fixed a
  narrowly pinned Internet Archive CDN redirect, numeric CAA image IDs and retried
  catalog-completion receipt projection. A cached 10,001-row full-release Redump
  PlayStation view now parses and indexes from immutable local bytes. Explicit
  raw-2352 row coverage retains provider CUE observations outside the matching
  domain; the owned four-track disc produced a unique complete SHA-1 match for
  `Brave Fencer Musashi (USA)` and supplied automatic descriptive text without
  conflating it with preservation identity. A 2,661-row Headerless No-Intro NES DAT
  is also cached for the implemented iNES-payload domain, but no owned NES dump was
  supplied. DAT-o-MATIC refused the later N64 request as an unknown filter, so N64,
  SNES and PC Engine caches remain absent and automatic retries are disabled. The
  intended second physical LAN browser remains unavailable; LA3 is not accepted.

- LA2 adds application-owned whole-album FLAC scope over the existing derivative
  pipeline. A bounded frozen batch Job records selected associated-disc sources and
  stable members, fans out existing per-track Jobs, converges active/identical work,
  reports aggregate and per-track partial outcomes, retries unfinished members and
  cancels only owned unshared children. Prepare album and the optional guided-import
  checkbox use the same service. Generated plan/batch/cancel/retry contracts and the
  curation UI restore progress across reopen/restart without globally disabling
  independent controls; effective artwork remains beside album/player text. Focused
  coverage includes multidisc scope, source ambiguity, missing tools, overlap,
  partial failure, shared cancellation, retry, restart, permission revocation and
  valid-output reuse. Chrome 153.0.8010.37 with FLAC 1.5.0 completed the two-track
  album workflow and real player/revocation checks. LA2 review and both browser
  configurations passed; broader U05/P04 and LA3 external qualification remain open.

- LA1 adds collection-page guided intake without a pre-created Item for configured
  direct inbox packages and resumable browser uploads. Typed planning previews
  package boundaries, unique CUE/TOC anchors, blockers, destinations and provider
  availability. Accepted intent owns stable Item/Capture/Representation identities;
  existing Jobs continue analysis, publication, DAT/MusicBrainz metadata and CAA
  artwork without a browser-owned chain. Server-side bounded history reopens after
  reload, exposes steps/attempts and permissions, supports retry/cancel, safely
  discards unpublished staging, and corrects a published Capture association.
  Filename titles and provider artwork stay automatic rather than manual authority;
  provider failure never rolls back originals. Private schema 13 persists intent
  and cleanup state. Focused P04/U05 journey coverage is implemented, not their
  broader completion gates.

- 2026-09-15 workflow corrections to P04/U01/D07: upload, analysis, review and
  publication have distinct labels and next steps; analysis no longer announces
  publication. Music preparation appears at the top of the editor, and absent
  tracks show guidance. Track availability follows Capture revisions, including
  publication without an Item revision change. Management dialogs keep their close button visible,
  handle Escape with draft-discard confirmation, contain keyboard focus and
  restore focus on close. Administration explains encoder setup; browser-based
  encoder settings writes and browser-triggered automatic MusicBrainz/artwork lookup remain absent.

- D06.3 is reviewed and corrected locally. Curation-only generated
  contracts expose typed metadata prerequisites, provider availability, permissions,
  safe candidate history and typed catalog-job results without raw provider bodies.
  The archival-workbench review panel provides explicit MusicBrainz/CAA lookup,
  durable job status/cancel/retry/reopen behavior, field and artwork authority,
  selectable history, provenance and clear actions. Effective metadata/artwork now
  drives browse, search and detail, including inventory edits and reindex. Application
  services complete acquired results; preserved artwork needs explicit cover selection.
  Pending completion inputs survive reopen, and detached requests are cancelled.
  Viewer reads omit inventory baselines,
  private assets and observations. Deterministic browser fixtures exercised real
  provider jobs and published image bytes through Google Chrome at desktop and
  390px. Live providers and deployment qualification remain unrun.

- D06.2 is reviewed and corrected locally. Optional configured
  MusicBrainz and Cover Art Archive adapters use the existing durable acquisition
  registry with pinned production origins, bounded/cancellable retry and redirect
  handling, a shared one-request/second MusicBrainz limiter, and bounded positive/
  negative caches. Exact responses remain snapshots; immutable digest-versioned
  MusicBrainz candidates and exact D04-result-bound DAT candidates enter the
  existing revisioned metadata refresh effect. CAA images enter the existing
  authorized Item Asset pipeline before candidates may reference them. Manual
  titles, inventory corrections, selected history and explicit artwork remain
  authoritative. Deterministic local fixtures cover provider failures and reindex;
  LA3 adds one live MusicBrainz/CAA disc-and-cover result, while deployment remains
  unqualified.

- Six-crate Rust workspace with a typed collection graph, compiled-module registry, application authorization boundary and a schema-2 third-module contract test.
- Foundation types now include collection-owned Work/Edition records, exclusive Item identity links, ordered specimen Components with optional data-bearing Carriers, collection-level Capture associations, typed asset subjects, language/currency/coordinate validation, canonical digests, and graph/representation validation.
- Archive schema 2 and a migration-only durable checkpoint preserve schema-1 records unchanged while emitting typed Work, Edition, Item, and Capture revisions, deterministic Components, explicit legacy payload locators, and rebuildable Works/Editions/Captures/Components projections. A committed synthetic baseline fixture covers two revisions, duplicate copies, two discs, legacy case/booklet carriers, files/assets, evidence, manual metadata, and lexically ordered unknown JSON.
- Lossless raw-JSON envelopes preserve unavailable module/provider bodies. Compiled modules have namespaced capabilities, exact schema validation, checked adjacent migrations with explicit source-preserving transformation policy, host-owned representation/job ports, and safe presentation contributions. HTTP item reads add post-redaction presentation; unavailable data displays generically and is not executable.
- A04.1 collection/item/Evidence reads now cross a Principal-free mechanism port and application-owned authorization. Explicit response DTOs expose effective permissions, structurally omit viewer-private inventory/metadata/evidence/raw-module fields, and never expose legacy Capture archive locators. Server-administrator status still grants no collection access. The deprecated Wave 1 command adapter preserves protected fields and delegates bounded edits.
- A04.2 moves item/entity commands, media/assets/activity, imports/jobs, administration, account authentication, and passkey persistence behind application-owned policy ports. Item HTTP writes use a deny-unknown bounded command; Store adapters receive identities/effects rather than Principal. Opaque readers preserve range streaming without exposing filesystem paths. The architecture check rejects direct HTTP `state.store` calls.
- A04.3 adds private SQLite schema-3 command receipts and mandatory application command contexts. User/API-major/operation-scoped idempotency keys bind to typed-request digests; completed successes replay across restart, concurrent claims fail distinctly, changed payloads conflict, and failed validation/authorization/revision attempts do not consume keys. HTTP and React transport keys; authentication ceremonies, logout, and offset-governed chunks remain explicit exceptions. A04.5 closes the effect/receipt crash window using atomic private receipts or prepared filesystem outcomes, with process-termination replay coverage across all mutation families.
- A04.4 composes runtime APIs from annotated route groups and checks generated OpenAPI 3.1 plus TypeScript artifacts for drift. React consumes generated response/command types, every request body publishes its enforced limit, and no Swagger/schema route is exposed. A deprecated Wave 1 item envelope may edit only after its server-owned fields match canonical state exactly. A04 is complete; P01 owns crash coordination and archive schema 3.
- P01.1 defines archive-schema-3 canonical Item, revisioned Asset, standalone Evidence, tombstone, legacy-locator, and transaction-envelope records. A committed synthetic Wave 1 schema-2 archive/private-v2 fixture preserves originals, artwork, revisions, raw JSON, accounts, credentials, grants, activity, and jobs; opening it leaves every pre-existing archive byte unchanged and projects the newer standalone Capture.
- P01.2 adds a mechanism-only durable publication journal with synced monotonic phase records, safe archive-relative paths, artifact hashes, actual-file recovery classification, retained unknown staging, and fail-closed handling of corrupt known transactions. Private state advances incrementally to schema v4 for coordinated recovery records. Named in-process fault points are covered with real temporary archives.
- P01.3 additively migrates schema-2 archives to deterministic schema-3 sidecars/checkpoints. Canonical Items omit embedded Capture/Asset snapshots; standalone Evidence and revisioned collection Assets preserve IDs/content and legacy payload hashes without moving old bytes. Capture archive locators have moved out of the portable core into adapter records/projections. Catalog projection schema v3 rebuilds these rows and keeps newer standalone Capture revisions authoritative. Current mutation writers become journaled schema-3 publishers in P01.4/P01.5.
- P01.4 adopts the durable journal for normal Item, Work, Edition, Capture, standalone Evidence, and Asset-metadata publication. Restart aborts pre-commit metadata and reindexes committed revisions before cleaning their journals. Evidence IDs are immutable/content-checked, reverification appends bodies once, and Item history carries only Evidence references.
- P01.5 journals import payload bytes and the Capture revision under a stable upload transaction. Controlled faults and actual killed subprocesses at every phase recover to one complete Capture/Representation/Evidence result or a retained retry, with upload scratch preserved.
- P01.6 coordinates collection archive creation with its intended private owner grant. Startup holds the writer lock through private migration/recovery, archive migration, and projection rebuild; deterministic command-bound Collection IDs make interrupted creation safely retryable. Archive-only restoration still creates no grants. P01's target-Linux/filesystem qualification remains an external review gate, not a claimed power-loss certification.
- S02 implements separate revisioned account, browser-session, passkey, recovery, and audit models in private SQLite schema v5. Exact-Origin enforcement, five-minute recent authentication, scoped deterministic rate limits, ten-code 160-bit recovery batches, atomic password replacement/session revocation, passkey counter CAS, last-enabled-administrator/owner guards, default reassignment, and stopped-server explicit-UserId recovery are covered with real SQLite and deterministic concurrency tests.
- S03 advances private state to v6 with expiring targeted enrollment and separately hashed/revocable native credentials. Same-user recent browser approval, cancellation/expiry/replay protection, one-time issuance, header-independent native authentication, and native-device inventory/revocation are implemented. `collector-client` owns an injectable secure-storage port, refuses redirects/cross-origin credential delivery, validates enrolled server identity/certificate material, and completes the LAN TLS integration through real enrollment.
- U02 provides revisioned account/profile/default-user and collection-grant APIs plus generated-client administration, owner-grant, profile, recovery-code, session, passkey, and paired-device controls. UI affordances use server-computed `EffectivePermissions`; direct service/HTTP authorization remains authoritative. Disabled accounts are preserved rather than deleted. The 2026-09-26 FX-06 browser follow-up supplies direct collection access, bounded grants, signed-out recovery and in-place recent-authentication retry without changing that service policy.
- Private SQLite advances through explicit migrations and stores a durable default UserId selected once from legacy first-row behavior. First-account creation sets it transactionally; later accounts do not change it.
- The S01 threat/recovery contract and D01 pinned reuse/fixture map are complete design gates. S02/S03 now implement the credential/recovery/enrollment portions; authenticated TLS rotation remains S04. D02/D03 and D04.1's reader-only game primitives are implemented locally. D04.2–D04.3 now add the DAT adapter, snapshot/match policy and game workflow locally; D04 is reviewed locally. D05.1 is reviewed and corrected locally: authorized frozen BIN/CUE and current raw-redumper packages produce strict signed layouts, exact `i16` stereo 44.1-kHz PCM, published disc IDs where the layout permits them, and bounded cancellation-aware AccurateRip checksum/offset inputs. Mixed mode, synthetic gaps and unsupported/ambiguous sidecars cannot claim whole-disc audio. D05.2 is reviewed and corrected locally: the provider adapter freezes bounded exact dBAR/no-data observations for checked offline replay, and music-owned policy retains explicit per-track v1/v2 coverage while requiring every expected track and a justified unique offset for whole-disc verification. D05.3 is reviewed and corrected locally: music Import analysis, committed structure jobs, shared-verifier AccurateRip jobs, archived snapshots, history and web curation use the existing registry/job/publication paths. Synthetic workflow tests cover outage, retry, cancellation, exact snapshot-byte rejection, no-data, refresh and reindex. D07.1 is reviewed and corrected locally; D07.2 is reviewed and corrected locally; D07.3 is reviewed and corrected locally. Immutable fixed-profile recipes consume music-owned exact track ranges; the optional bounded process adapter validates FLAC by independently decoding every integer PCM frame and inspects Opus delay/padding; single-track outputs publish through the Capture journal and appear through a viewer-safe availability/read contract with strong ETag and Range behavior. Server-owned optional encoder configuration and curation-only explicit preparation leave import and original reads independent of tools. The bounded browser-player child is implemented locally over that shared DTO: album track selection is grouped by Capture/source and prefers a decodable FLAC; the 2026-09-22 shell-owned player keeps a local per-user cross-album queue with play/pause/seek/next/previous/ended transitions and honest codec/access failures. Google Chrome 153.0.8010.37 actually decoded and played generated FLAC; the child is reviewed and corrected locally. `opusenc`, live-provider, owned real-dump, gapless/sample-accurate playback and external mastering/deployment qualification remain unavailable/external.
- Separate private operational database; collection roles, private field filtering, revisioned accounts/grants, password/passkey recovery, browser-session inventory, and revocable paired native credentials.
- Separate public/private HTTP routers and direct LAN TLS. Public curation/admin independently configured; disabled APIs and management bundles are absent.
- WebAuthn passkey registration/login backend and browser integration when an HTTPS authentication origin is configured. Registration and passwordless login pass a Chromium virtual-authenticator end-to-end test; real password-manager/hardware acceptance remains a release gate.
- Web collection browsing/search with artwork alongside game/CD rows, explicit placeholders and identity states, inventory creation/editing, wishlist/owned-copy entry, original cover retention and bounded thumbnails.
- Resumable package upload, destination verification, stable retry identity, original downloads with byte ranges, integrity rechecks and persisted job records.
- Client trust construction, route selection/hysteresis and cache eviction policy. A TLS integration test enrolls through same-user browser approval, persists a native credential through the storage port, proves authenticated ranged downloads over loopback LAN without any public route, and rejects an unrelated certificate or credential-bearing cross-origin route.
- Configured server inbox inventory/import API using the same source-retaining staging and evidence path; background upload errors persist as failed jobs.
- P02.1/P02.2 advance private state to schema v8 and implement typed durable jobs
  plus a local executable scheduler. Its initial two-processing/one-provider limits
  are superseded by the configurable dispatch above. Transactional claiming initially enforced
  two processing executions plus one reserved provider acquisition globally,
  including two same-account processing Jobs; 30-second leases,
  five-second heartbeats, claim-token fencing, restart takeover, and bounded
  progress persistence are permanent-test covered. Three server execution loops
  share a process-instance identity; integrity verification was the first consumer. Its deterministic Evidence
  identity makes recovery after archive publication idempotent, and the existing
  verification HTTP action now returns the actual queued Job identity.
- P02.3 separates incomplete upload staging from executable queue state and moves
  browser upload publication plus configured-inbox copying/publication onto the
  same durable workers. Queued and bounded-unit running cancellation retain source
  material; a publication gate makes cancellation pre-commit or stably too late
  after `RevisionCommitted`, with committed recovery completed exactly once. Failed
  retries reuse the Job, Capture, and Representation identities, increment attempt,
  retain source bytes, and preserve prior failure details. The process-local job
  semaphore has been removed.
- P02.4 replaces the legacy job-list HTTP shape with cursor `JobView` pages and
  submitter-scoped detail, cancellation, and retry routes. Views provide bounded
  progress, attempt, safe diagnostics, timestamps, and actionable flags while
  excluding stored requests, prior-failure bodies, worker/lease/claim data, and
  other users' jobs. Mutations require idempotency keys. P02 is locally complete;
  target-Linux filesystem/process behavior and independent review remain external.
- U03.1 makes specimen lifecycle state server-owned. The bounded Item-save
  command omits specimens, while its Wave 1 adapter accepts only an exact
  canonical snapshot. Focused application commands and curator-only HTTP routes
  create/update specimens and ordered components, remove unreferenced components,
  start and explicitly return loans, and record disposal without deletion.
  Nested identities are deterministic per command; returned loans, captures,
  assets, evidence references, and payloads survive ordinary Item revisions.
  The previous broad-save “Add an owned physical copy” shortcut is removed.
- U03.2 provides the backed copy-level lifecycle editor: condition, ownership,
  possession, acquisition, private notes, ordered components/carriers, lending,
  explicit return with retained history, and disposal. Lifecycle drafts remain
  in React component state across stale Item revisions; the conflict panel names
  both revisions and offers reload/discard or manual reapply. The layout retains
  the existing archival-workbench visual language and is keyboard/mobile tested.
- U03.3 verifies viewer detail and browse DTOs structurally omit acquisition,
  private notes, contacts, and loan history while curator DTOs retain them.
  Focused lifecycle routes are composed only on curation-enabled listeners. Two
  independent copies, returned history, disposal, Capture/Asset relationships,
  and stored artwork bytes survive reindex/restart. U03 is locally complete;
  independent review remains external.
- U04.1 adds standalone, collection-scoped Asset creation for PNG/JPEG/WebP
  Covers, Photos, and Scans attached to Work, Edition, Item, Specimen, Component,
  or Capture subjects. One bounded image pipeline preserves originals and emits
  PNG thumbnails. Visibility/alt-text updates and tombstones create independent
  schema-3 Asset revisions without changing Item revisions; subjects remain
  immutable, payloads/history survive tombstoning, and reindex retains standalone
  records. The post-alpha.3 document delivery below adds PDF and text Assets.
- U04.2 exposes revisioned `PresentedAsset` metadata and collection-scoped
  original/thumbnail resources through application-owned authorization. Viewer
  DTOs omit visibility and stored filenames/digests, while curator-only metadata
  and both byte representations are denied before storage is opened. The old Item
  Asset URL is a compatibility adapter over the same policy. Edition-linked Items
  inherit Work artwork, and a role-aware server `primary_artwork` prefers visible
  module selection before deterministic newest-Cover fallback. Asset mutations
  remain curation-listener composed.
- U04.3 replaces the one-purpose cover shortcut with a backed supporting-image
  workbench. Curators choose the attachment subject, Cover/Photo/Scan role,
  visibility, alternative text, and image; receive a local preview; inspect
  lazy-loaded stored thumbnails; revise metadata; and archive with explicit
  retained-history guidance. Browse/detail consume server-selected artwork and
  visibly separate missing artwork from identity uncertainty. Keyboard flow,
  focus restoration, reduced motion, viewer filtering, and responsive layout are
  browser tested. General document upload/preview support is described below;
  broader offline artwork and independent review remain separate.
- P03.1 advances private state to schema v9 and persists provisional Import
  workflows under an `ImportId` distinct from Jobs and command transactions.
  Records retain a safe source summary, bounded file inventory with offsets and
  digests, preallocated Capture/Representation IDs, optional physical association
  and expected Item revision, lossless candidate observations, limitations, review
  decision, analysis Job link, and phase. Optimistic updates preserve immutable
  source/identity fields across restart, and the model permits a fully staged,
  reviewed, unidentified and unattached published state. No private filesystem
  locator is part of the model.
- P03.2 gives browser and configured-inbox sources one resumable staging path.
  Browser chunks enforce durable offsets, bounded exact replay, and declared digest
  checks. Inbox staging executes as a P02 Job, resumes from persisted offsets, and
  rechecks source entries/inventory for symlinks, special files, path escape, case
  collisions, size, metadata, and content changes. Curators see configured labels
  and direct package names only; private roots remain solely in the bounded worker
  request and are absent from `ImportView` and `JobView`. Both sources converge on
  the same digested `analyzing` state after staging.
- P03.3 submits optional Import analysis as a retryable/cancellable durable Job
  through the A03 module capability registry. Valid outputs persist lossless bounded
  candidates and limitations; unavailable built-in game/music analyzers proceed to
  review with an explicit D04/D05 limitation and no fabricated candidate. Focused
  review commands select a candidate or remain unidentified, validate an existing
  Item/Specimen/Component/Carrier chain or leave the Capture unattached, and retain
  the user's selection in `awaiting_review` when the expected Item revision is stale.
  Manual review records that no analyzer was selected.
- P03.4 publishes through a distinct durable P02 Job and the P01 journal using the
  Import's preallocated Capture and Representation identities. It re-reads staged
  bytes, publishes integrity Evidence, supports a canonical physical association or
  an unattached Capture without fabricated inventory, and completes committed work
  exactly once after restart. Cancellation retains source scratch before the Capture
  revision commit and returns a stable too-late conflict afterward; stale associated
  Item revisions preserve review state. The legacy browser-upload and administrator
  inbox endpoints delegate to the same Import workflow and distinct execution Jobs,
  with a bounded fallback only for pre-v9 in-flight uploads. P03 is locally complete.
- P04.1 adds a curation accession workspace over the executable Import API. It can
  select browser files or safe configured-inbox package names, stage and inspect the
  complete inventory with live progress, display persisted analyzer limitations and
  candidate alternatives without a verification claim, record an existing physical
  chain or no association, and publish the reviewed Capture. The responsive keyboard-
  named workflow is browser-tested at desktop and 390px.
- P04.2 adds the submitter's durable active/history Job ledger with bounded progress,
  attempts, safe diagnostics, and server-computed cancel/retry affordances. Visible
  active work polls at two seconds, idle state at 30 seconds, and reconnect/action/
  visibility changes refresh immediately. Retry keeps the same Job identity and
  updates its attempt; Import review inputs remain owned outside the polling ledger.
- P04.3 persists the current Item-scoped Import handle across page refresh, refreshes
  durable Import/Job state after reconnect, and rebases stale review submissions while
  retaining the user's identity and physical-chain choices for explicit reapply. The
  real browser matrix covers retry/cancel, cross-user and collection authorization,
  unavailable analyzer disclosure, keyboard/focus/live status, reduced motion, mobile
  layout, and normal/private listener composition. P04 is locally complete.
- D02.1 revalidates the D01 extraction boundary at exact sibling commits and dirty
  paths. It confirms the old Phono junk-libs pin is locally available, the published
  and generated disc vectors plus `chdman` are present, and the owned-dump/live
  oracle is still external. This is an implemented documentation gate only: no D02
  shared algorithm, consumer migration, or catalog verification capability exists
  yet, and Retro-owned source cannot be redistributed while its MIT attribution is
  a placeholder.
- D02.2 makes `junk-libs-core` the single implementation of generic streaming
  multi-hashing and its byte-reader/result contracts. `junk-libs-disc` now computes
  complete stored CUE file-byte spans and visits each true CHD 2352-byte
  main-channel track frame without padding or subchannel bytes. Retro's complete
  ordered-track hashing consumes those contracts, so changing a non-primary track
  changes the complete result while platform normalization and catalog matching
  remain product policy. Disc identity, AccurateRip, DAT extraction, Collector
  integration, and real-dump qualification remain required under D02.3-D02.5.
- D02.3 adds `junk-libs-disc-id` as the single checked implementation of the
  neutral audio-CD TOC and MusicBrainz, FreeDB/CDDB, and AccurateRip disc IDs.
  Phono re-exports the shared TOC and maps its results into provider-facing fields
  through compatibility wrappers; published ARver, MusicBrainz, and libdiscid
  vectors remain byte-identical. AccurateRip CRC/dBAR/offset behavior, neutral DAT
  parsing, Collector integration, live-drive qualification, and a published Git
  dependency pin remain required under D02.4-D02.5.
- D02.4 adds `junk-libs-accuraterip` as the single headless implementation of
  CRC v1/v2 over packed integer CD PCM, boundary skips, dBAR parsing/request
  validation, primary-checksum comparison, frame-450 evidence, and bounded
  sample-offset search across track boundaries. Phono's compatibility modules
  consume it while CUE/CHD construction, HTTP/rate limiting, evidence storage,
  and product errors remain local. DAT extraction and Collector music integration
  remain required under D02.5/D05; private/live/hardware checks remain external.
- D02.5 adds `junk-libs-dat` as the neutral `BufRead` implementation for Logiqx
  XML and ClrMamePro syntax, entity decoding, checksum normalization, and optional
  record observations. Retro now supplies only file/error adapters at that seam;
  its downloading, cache, index, matching, platform normalization, and naming
  policy remain product-owned. D02 is locally complete. Collector game/music
  integration remains D04/D05, and publishing/pinning shared SHAs plus private/
  live-disc qualification remain external.
- D03.1 establishes `junk-libs-audio` opaque `Read + Seek + Send` byte sources,
  length/media hints, nonzero sample-rate/channel formats, interleaved `f32` PCM
  chunks, and integer frame coordinates. `junk-libs-playback` establishes generic
  opaque track keys, generations, and validated track-local positions. These are
  executable contracts only: decoding, resampling, queue transitions, DSP, and
  consumer migrations remain required under D03.2-D03.5, and AccurateRip remains
  entirely on integer CD PCM.
- D03.2 adds feature-gated PCM-WAV decoding from those opaque readers with bounded
  output and exact track-local frame seeking. Path-free adapters accept already-
  opened CUE and redumper readers; CUE boundaries remain explicit and redumper
  quality observations are returned with playback PCM. No compressed codec is
  advertised. Resampling, shared queue transitions/DSP, and consumer migrations
  remain required under D03.3-D03.5.
- D03.3 adds bounded streaming sinc resampling with exact final length, qualified
  linear gain and explicit hard clipping, and a generic queue state machine for
  replace/append/truncate/track transitions. Seeks are generation-fenced and use
  integer track-local frames. Consumer adapters and their end-to-end behavior
  remain required under D03.4-D03.5.
- D03.4 migrates Trash Amp's WAV, CUE/redumper, resampling, and transport-policy
  paths to the shared crates while keeping filesystem discovery, compressed-codec
  compatibility, and CPAL output in its adapters. Collector can consume an
  already-authorized representation as the shared opaque byte source without
  exposing an archive path. Shared compressed codec support remains required;
  physical output and private fixtures remain external qualifications.
- D03 is locally complete and reviewed across Junk Libs, Trash Amp, and Collector.
  The headless default audio contract depends only on `thiserror`; decoding, disc
  readers, and resampling are optional mechanisms. Compressed shared decoding,
  richer effects, event delivery, published dependency pins, and hardware/private
  qualification remain required or external as recorded in the Wave 3 ledger.
- W3.R traces every Wave 3 root to executable tests, browser interaction, or an
  explicit external qualification in [the integration review](wave-3-review.md).
  The complete local Collector gate and affected sibling gates pass; no D04/D05,
  provider, derivative, event-streaming, document-upload, plugin, or Wave 4
  capability is counted as implemented.
- Optional private TLS management listener bound to an explicit interface address.
- WebUI onboarding distinguishes client-only and private management surfaces, offers first-collection/first-item next steps, preserves structured API error codes, and separates wrong-origin, expired-session and role-denial guidance without weakening listener or authorization policy.

## Processing implementation checkpoint — September 2026

[Dated validation](validation.md) and [the review](wave-4-review.md) record game/CD
processing, metadata, derivatives and their limits. The obsolete wave queue has
been removed; future backup/trust/routing scope is consolidated in remaining work.
W4.0 is reviewed and corrected. W4.1's portable processing/metadata types,
source-bound package-reader and application port contracts, deterministic source
fingerprints, and split game/music module files are implemented locally, reviewed and corrected. They add no analyzer, provider I/O, persistence, API, or runtime support
claim. The baseline/dirty-state inventory, API drift assessment, operation
matrix, exact dependency/tool selections, migration reservations, generated fixture
recipes and external-oracle instructions are recorded; schema reservations are
advisory under the pre-release policy.
W4.2 durable records and executors are implemented locally, reviewed and corrected.
D04.1–D04.3 and D05.1–D05.2 are reviewed and corrected locally; D05.3 is reviewed and corrected locally; D07.1 is reviewed and corrected locally; D07.2 is reviewed and corrected locally; D07.3 is reviewed and corrected locally. D06.1 durable metadata authority and deterministic resolution are reviewed and corrected locally; D06.2 providers and artwork are reviewed and corrected locally; D06.3 metadata review and effective presentation are reviewed and corrected locally.
Archive schema 4 and catalog schema 9 cover processing observations and compact occurrences,
streamed snapshots, metadata authority, and conversion recipes; private schema 14
persists frozen Job input, provisional Import observations, active catalog
selection, guided-ingest intent and restartable staging cleanup. Import publication
rebinds evidence to its committed Representation with checked file mapping and
retained provenance. Registered dispatch, authorization, restart/cancellation,
idempotent retry, archive-only recovery, and current-versus-historical summaries
have executable coverage. D04.2 adds a configured local-DAT provider using
`junk-libs-dat`, immutable exact-byte snapshots, rebuildable row indexes and
complete/ambiguous/partial/inconclusive game matching. D04.3 registers the
qualified game operations, connects Import and committed-Capture jobs, and exposes
admin/curation status without leaking configured paths. D04.1 adds a
reader-only shared game-format crate for exact raw, trainer-free iNES/NES2, N64,
explicit SNES/PCE, ordered raw-2352 CUE, and pinned-oracle CHD hashing. Collector
maps those outputs into validated module results; D04.3 registers the qualified
runtime subset, while CHD remains an external/unexposed gate. Actual Retro Junk
consumers use the same algorithms. The validation
evidence and remaining limits are recorded in [validation](validation.md).
The native user-testable surface in this wave is a diagnostic CLI; Slint,
offline-library management and the Trash Amp provider remain later tickets.

## Usability/lifecycle fixes — 2026-09-21

FX-01 in [the fix plan](ux-fix-plan.md) implements URL-backed browsing,
immediately visible responsive details, fenced evidence reads, explicit read-error
recovery, and server-paginated title/recent/artist/platform sorting with wishlist,
files-attached, playable-audio and requester-scoped attention filters. Catalog 7
adds rebuildable browse keys; browsing does not hash payloads. Local acceptance passed on 2026-09-21; see the [validation record](validation.md#fx-01-browsingnavigation--2026-09-21)
for browser evidence and exact Rust check/rerun outcomes.

FX-02 implements module-owned eligible-file planning, typed scan states, atomic
requester-scoped active-work convergence, durable parent/direct ownership, and
compact repeat-result occurrences. Actual checks retain first/last dates and counts;
receipt replay does not advance them. Equivalent reference bytes and parsed inputs
share result content while occurrences preserve snapshot provenance. Reindex retains
legacy records, reconstructs occurrence references and rejects inconsistent links.
The processing panel restores selected settings/current work, leads with View result,
and keeps unrelated editing usable. Manual import analysis resumes polling after
reopening. Admission uses recorded fingerprints; workers check actual bytes with
cancellation. [Local acceptance passed](validation.md#fx-02-scan-lifecycle--2026-09-21) on 2026-09-21.

FX-03 implements Work & history with collection/item/unattached, package/file-name,
date and lifecycle filters, bounded server pages, import/preparation grouping and
execution-attempt detail. Private schema 15 retains dismissal/restoration and durable
clear intents; cancellation and eligible staging cleanup settle before archival.
Per-target review shows eligibility, byte effects and partial outcomes. Pending clear
and dismissed history fence stale admission/retry; shared work remains protected.
Manual and guided imports reopen from server identities, including upload resumption.
Generated track files remain in the named Music track groups; ineligible derivative
CD-operation forms are suppressed using scan readiness while evidence stays readable.
FX-03 [local acceptance passed](validation.md#fx-03-work-and-history--2026-09-21) on 2026-09-21; no external release qualification is claimed.

FX-04 implements focused editing/intake (2026-09-24); FX-05 now implements
Remove/Restore and clean reimport; its current validation is recorded below. FX-06 access/recovery is locally implemented and accepted. FX-07's account and
personal-security paging, focused administration/security tasks, bounded metadata
candidate rendering, authorized collection search/pages and exact reads, catalog collection-scope correction,
and a typed metadata refresh outcome with a browser no-match regression are
implemented. The reviewed collection-search race and catalog lookup recovery
are corrected; three-import history, durable cleanup retry/restart and DAT
no-match/ambiguity passed real-browser journeys. FX-07, U05.1–U05.3 and P04.6
passed local acceptance with exact evidence in [the interface audit](fx07-interface-audit.md)
and [validation](validation.md). See the current sections below.
LA3's second-device qualification and broader release gates remain open.

## Remaining work and follow-on scope

See [the single remaining-work queue](remaining-work.md). The previous gap list
mixed completed interfaces/indexing/native integration with unrun qualification,
and treated optional clients/advanced deployment as universal release gates.
It is superseded by the 2026-10-09 reassessment. Obsolete work queues have been removed. Relevant unfinished scope is in
remaining-work.md; no missing capability is accepted by reprioritization.

## Local onboarding

`setup` creates or repairs configuration with a hidden, confirmed terminal password prompt, preserves existing accounts/data/TLS identities, and prints the exact private localhost URL and first collection steps. `doctor` checks configuration without opening the archive. The WebUI distinguishes browser-origin failures, recent-authentication requirements, expired/revoked sessions and genuine role denials, and client-only addresses point back to the printed private management address. Login checks that the session cookie was retained. Single-user login resolves the durable default UserId and hides account switching; administrators can explicitly reassign the default through the revisioned account UI. Local HTTP onboarding is Chromium-tested; remote and other unqualified browsers require separately qualified trusted HTTPS. This is CLI guidance, not yet a comprehensive interactive network/proxy wizard.

## Wave 1 review fixes

Capture revision/ownership regressions, incomplete migration recovery, graph identity/ancestry gaps and capability dispatch are corrected with retained regression tests. Capture file/locator changes cannot bypass import services; legacy payloads remain reachable after reassociation. Reindex hydrates canonical captures and clears removed component projections. Schema transformations retain exact source envelopes, and language tags use a syntax parser. P01 transaction/evidence recovery and the S02/S03 security runtime now have permanent restart, concurrency, and attack regressions; their external review/hardware qualifications remain outstanding.

## Wave 2 review fixes

Private schema v7 adds prepared command outcomes and grant revision counters. Scoped effect adapters commit private responses atomically or tie exact archive responses to immutable publication markers. The process-kill matrix covers 19 mutation families and early archive commits; startup reconciles upload bookkeeping and artwork cleanup before returning. Prepared partial files and recognized journal temporary files recover, while committed corruption remains fatal.

Application-owned security snapshots preserve transactional authority guards. Disabling accounts revokes browser/native access and pending enrollments permanently; replacing recovery codes preserves paired devices and pending enrollment. Grant removal never resets its revision history. Passkey ceremonies finalize against their original verified credential snapshot; the virtual-authenticator browser regression rejects stale ceremonies through HTTP.

The review findings and A04.5 are closed. Linux filesystem/power-loss/NAS qualification, real authenticators, trusted remote-browser HTTPS, and native platform secure storage remain explicitly unqualified; these are not claims of the local tests.

## Collection-type correction checkpoint

[CT.1](collection-types-plan.md#ct1--ownership-and-module-contracts) is implemented,
reviewed and corrected. Game processing
results and music verification policy now belong to their modules; only genuinely
shared CD/PCM coordinates and layout remain in core. Module descriptors declare
bounded scalar/read-only fields and explicit operation scope. Owner-dispatched
typed patches preserve omitted and untouched raw values, distinguish explicit null,
and reject unknown/wrong-typed edits. Successful operation output and typed metadata
domain overrides require owner validators.

[CT.2](collection-types-plan.md#ct2--persistence-and-existing-collections) is
reviewed and corrected locally. New collections persist an
Assigned owner selected from the registered module registry; creation idempotency,
recovery intent, collection DTOs and generated API contracts include that identity.
Pre-CT.2 manifests deserialize as LegacyUnassigned without rewriting original files
and remain readable; graph publication rejects unassigned collections. Local administrator inspection reports exact
current item-domain counts, assignment eligibility, pending work, active authority
conflicts and preserved historical overrides. Explicit homogeneous/empty assignment
uses an optimistic, provenance-bearing collection metadata revision through the
existing journal and is reconstructible from archive alone. Generic graph validation,
normal publication and reindex reject item-domain/assigned-owner mismatches.

Assignment validates full active authority payloads and pending operation scope;
unknown active jobs block assignment, and retries honor the original expected revision.
The CLI requires the stopped server and exclusive store access. A minimal browser
creation picker now sends the explicitly selected module using the existing feature
list.

[CT.3](collection-types-plan.md#ct3--application-policy-and-jobs) is implemented
and reviewed locally. One application policy now gates all implemented
collection-content mutation families on Curate authority, Assigned binding, an
installed owner and supported Item schema. Collection-domain analysis/processing
must belong to that owner; explicitly SharedMedia operations remain usable across
collection types. Durable processing/acquisition jobs freeze the binding and recheck
it, operation ownership/version and module availability at execution/publication.
Long import publication rechecks policy after copying bytes and before Capture
commit, preserving sources on denial. Stored unsupported schemas cannot be replaced
through ordinary edits. Already committed integrity recovery does not rehash payloads.
Already committed recovery completes without modules or grants. Legacy/unavailable
reads, authorized existing media/Artwork, personal activity and access administration
remain available under their independent policies.

[CT.4](collection-types-plan.md#ct4--user-facing-integration-and-completion) is
reviewed and corrected locally. Authenticated clients receive bounded
collection-type descriptors from the application registry; public feature discovery
does not expose that registry. Collection and Item DTOs carry server-computed content
editability and distinct assigned, legacy-unassigned, missing-owner and unsupported-
schema availability. Generic web forms inherit the collection owner, render module-
declared text/integer fields, visibly preserve structured fields as read-only, and
send owner-mediated patches rather than replacement module JSON. Browse labels and
safe summaries are descriptor/presentation driven, with no game/music module-ID UI
switch. A test-only third module passes create/edit/browse/reindex and forged-input
validation through real services, and Chromium covers a valid homogeneous archived
collection whose owning module is absent.
Unavailable module fallback remains readable; runtime plugins, automatic conversion,
legacy graph splitting and type changes are outside this refactor. Historical field
authority remains preserved verbatim and is never activated by assignment.

CT.4 review also fixes stable patch-command receipts, unsupported-schema form
controls and descriptor-independent access administration. Integer edits cannot
silently round beyond the browser's safe range. The generic third-module browser
fixture checks submission types; its production persistence remains covered by
real HTTP/service/reindex tests. D04.1 subsequently implemented and qualified the
bounded shared reader/result primitives; D04.2–D04.3 now integrate their local DAT
and game workflows and have passed local D04 review.

D04 review preserves exact DAT snapshot length/digest checks on every parse, rejects
malformed catalog entries rather than silently omitting them, and checks job
cancellation during package reads and catalog scans. Curator evidence reads remain
available without an installed owner module. The catalog currently uses bounded
per-job parsing and linear comparison; large-corpus/index performance is unqualified.

D05.1 review keeps PCM source concatenation/decoding in the shared disc crate.
The bounded AccurateRip verifier searches v1 across the declared interval and v2
at explicitly recorded candidate offsets. Missing v2 coverage yields incomplete
search rather than mismatch. D05.2 must retain that distinction and require every
expected track for whole-disc success. No music runtime capability is registered yet.

D05.2 retains that distinction in the music-owned result contract. Per-track
evidence records both computed checksum versions/confidence, the inclusive v1
range, explicit v2 offsets, physical edge limitations, provider response identity,
implementation version and exact input digests. Whole-disc success requires every
database track at one unambiguous offset; only a complete non-edge-limited v1/v2
search can report mismatch. The provider crate owns a pinned HTTPS dBAR adapter
with bounded retries/time/body/rate/redirect policy and exact archived-byte replay.
Local HTTP fixtures cover data, no-data, malformed/misdirected/oversized input,
timeouts, cancellation, outage/cache replay and distinct refreshes.

D05.3 registers music Import analysis, CD structure and AccurateRip verification
with the compiled owner and runs them through the existing frozen P02/W4.2 jobs.
The small provider port is adapted into the same streaming snapshot publication
executor; one server-composed provider instance owns the limiter. The provider
parser re-reads the complete archived dBAR payload and checks length/SHA-256 before
returning neutral entries, while the music result retains that snapshot digest,
the frozen Representation fingerprint, every input digest, computation offset and
per-version coverage. Provider failure fails only its retryable acquisition Job;
404/no-data publishes a distinct snapshot and inconclusive result. Curation shows
separate structure and catalog evidence, explicit frame units, progress/history,
limitations and artwork in the same Item workflow. No FLAC preparation or playback
was added. Synthetic workflow qualification is local; the opt-in owned-dump runner
is present but external qualification is unavailable here.

D07.1 keeps `ConversionProfile::Flac` and `Opus { bitrate_kbps }` as readable
presets while sealing source fingerprint, selected music-owned PCM ranges, fixed
settings, tool/version and exact formats into recipe identity. CUE conversion
retains stored HTOA on track 1 and the D05 next-INDEX-01 ownership for later stored
pregaps; synthetic gaps, mixed mode and ambiguous ranges fail closed. The provider
adapter directly spawns configured `flac`/`opusenc`, bounds streams/diagnostics,
kills and reaps cancellation, clears only its supplied scratch handle, and never
receives archive paths or labels. Local `flac` 1.5.0 output matches every source
`i16` frame under independent Claxon decoding. Opus 96/128/192-kbps recipes and
20-ms/48-kHz delay-padding inspection are implemented, but `opusenc` is absent and
therefore unavailable rather than simulated. D07.2 now publishes explicitly requested
single-track derivatives through the existing frozen jobs and Capture journal.
Recipe, validation evidence and output bytes are immutable artifacts of that
Capture commit; failed/cancelled pre-commit publication advertises nothing, and
committed recovery finishes the original output without an encoder or a new access
check. Encoder versions and exact source/track ranges determine output identity.
Concurrent requests converge; current Capture associations and existing Item edits
survive publication. Reindex checks stored derivative bytes against validation
evidence. D07.3 adds an authorized ordered track/profile availability DTO, client
media reads, curation-only preparation, server-owned optional encoder probing,
generated API contracts and web preparation/progress controls. Published output
remains readable after restart without its module or tool. Strong content ETags
and full, partial, suffix, open-ended, unsatisfiable and If-Range behavior are
exercised through HTTP. Import, inventory, verification and media GET still do
not request conversion. The following bounded browser child is now implemented
locally and is reviewed. Its album views use these rows directly for Capture/source-scoped track selection;
the 2026-09-22 shell player owns the local queue and actual viewer FLAC playback
without adding a backend player
or conversion-on-read path.

### LA3 independent review (2026-09-20)

Catalog track coverage now rejects malformed BINs and unknown payloads rather than
silently excluding them; only CUE metadata is excluded, through module-owned policy
shared by the adapter and matcher. The manual cache helper validates existing
digests, bounds expansion and stages complete new entries. New DAT-o-MATIC network
requests are paused in code following error 1260; cached data remains usable.
Second-device qualification remains outstanding.

### Folder intake and upload feedback (2026-09-20)

- Add / import defaults to folder intake, with manual inventory as a separate mode.
  Modules group same-stem redumper packages, CUE directory packages, and supported
  game single-file formats. Nested anchored CUE folders remain separate; multiple
  CUEs in one directory stay together for review. Unassigned files are listed.
- Every accepted browser package uploads in sequence and has a durable entry and
  import record before bytes transfer. Closing interrupts browser transfer, not
  saved entries or subsequent server jobs; selecting the same originals resumes.
- Both upload UIs share actual request-body progress, server acknowledgement,
  retained-prefix checks, abort handling and safe lost-response retry. Server work
  without measurable units uses an indeterminate bar rather than invented percent.
- Background import notifications refresh the collection without selecting another
  entry and unmounting its player. Association details show the current destination;
  correcting an association retires an unused automatic placeholder while retaining independent inventory, manual information and other discs.

- Placeholder retirement is an application-owned, recoverable follow-up to a committed
  guided association change. Settled/cancelled intent plus the current association
  identify cleanup candidates, including gaps from earlier server versions. The
  adapter compares the whole hydrated item and metadata-authority revision under
  the archive writer lock before publishing the existing portable tombstone.
  Other captures, assets, specimens, wishlists, identity/domain edits and manual
  metadata authority prevent automatic retirement. The original guided workflow settles
  before retirement; no payloads are deleted. Browser refresh clears retired
  selections, and explicit reassociation follows the destination only if the moved
  source was selected, preserving unrelated playback.

### Persistent browser music queue — 2026-09-22

The user-requested bottom player is implemented and locally accepted; see
[validation](validation.md#persistent-bottom-music-player--2026-09-22).
One shell-owned audio element survives album navigation, catalog refresh and import
completion. Album views append a track or prepared album; original audio files can
also be queued. The player supports transport/seek, cross-album advance, reorder,
remove and clear. A bounded localStorage queue is scoped to browser origin/user,
restores paused on reload and is never written to the server. Without writable
localStorage, it remains usable for the current session. Access is rechecked before
playback and periodically; loss of access/session releases buffered audio. This
completes a user-requested portion adjacent to FX-04, not the whole FX-04 delivery.

### View separation requirement — 2026-09-22 (historical requirement)

FX-04 now explicitly owns separate import-history and selected-import detail/progress
views, with preserved return context, direct-link/reload and thousands-of-records
acceptance. The combined history/detail panel did not meet this requirement at the
time of writing; FX-04 now implements separate views. FX-07 verifies meaningful
browse/detail/task separation across the interface. See [the fix plan](ux-fix-plan.md).

The 2026-09-22 selector defects are corrected by FX-04's authorized, bounded
server search and pagination. General collection results are labelled as search,
without claiming detected matches. The [growing-list audit](fx04-scale-audit.md)
records the access-list work completed in FX-06 and broader FX-07 checks.

### Automatic CD provider lookup requirement — 2026-09-22 (historical requirement)

Guided ingestion already acquires MusicBrainz data automatically with a disc ID
from its CD analysis. The metadata editor still presents an explicit optional
lookup and offers a search built from displayed metadata. FX-04 must make automatic
dump-derived identity lookup consistent across intake paths and remove reliance on
that explicit control for ordinary ingestion. Dump filenames/folders/timestamps,
including placeholder titles inherited from them, must never become automatic text
queries. Missing identity and provider/no-match/ambiguity states must remain honest;
manual metadata authority and original preservation remain protected. This broader
requirement was a planning statement. The primary new/existing intake path now
shares the guided coordinator and its derived-disc-ID acquisition. The advanced
manual review path remains for imports admitted through the older provisional API;
its explicit provider lookup is a recovery action, not automatic acquisition.

### FX-04 focused editing and intake — implemented 2026-09-24, locally accepted 2026-09-25

Review corrections on 2026-09-25 bind existing-destination revision checks to
first admission after receipt replay, initialize editor drafts when a guided
publication adopts a different Item, retain the chosen section on later Item
updates, and refresh every loaded Work & history page. See
[validation](validation.md#fx-04-review-corrections--2026-09-25).

The workbench presents Details, Copies, Files, Music, Artwork and Advanced evidence
as navigable task sections. Inventory title and edition edits are direct; metadata
authority/provenance is progressively disclosed. Files uses guided intake for new
and existing entries, with destination, source, readiness, preservation-only
choice, upload and durable server progress. Existing-Item admission freezes its
revision and preserves its other Captures and independent inventory. Missing
companions and provider/tool availability are visible before submission. The
existing private guided-intent table gains additive `created_item` and `analysis`
fields without resetting older intents; the private schema remains v15. The
timestamp-named music fixture confirms automatic MusicBrainz requests use the
parsed disc identifier. No-match, multiple-candidate, missing-identity and provider
failures have distinct progress text; manual metadata authority survives refresh.

Work & history now uses separate list and focused selected-work views, an
authorized URL identity and Back/Forward/reload. Import detail and current intake
do not preload the catalog or mount the global history. The destination chooser
searches server-side with 50-result pages, selected-entry context and revision
recheck. Catalog snapshots and inbox package selection are paged. The
[scale audit](fx04-scale-audit.md) records all growing surfaces and scope of later
access and history work. A temporary-server browser journey passed with 3,000
entries and 3,000 imports, including selection beyond the former 1,000-entry cap.

### FX-05 Remove/Restore slice — in progress 2026-09-25

The Item archive tombstone now has curator-only Remove and Restore commands with
revision and idempotency receipts. Ordinary search/browse excludes removed Items;
an explicit paged Removed entries view can reopen one and restore its same ID.
Evidence, activity and artwork backing reads require curator access for a
removed Item, including subject-owned asset routes.
The archived Item still retains physical inventory, metadata and links to
originals/evidence. Removed Items reject new guided destination admission and
item-owned Job admission; the removal boundary stops obsolete queued/running
work while retaining separately owned scans. Restart reconciliation settles
private work if interruption falls after tombstone publication. An Import whose
Capture committed before removal may finish its receipt and Evidence reference
without rewriting tombstoned inventory. Store tests cover reindex, restart,
curator/viewer visibility, pending guided work, queued-work cancellation and
that post-commit recovery boundary; the
real browser browse journey covers Remove, reload in Removed entries, and Restore.

This is not FX-05 completion. Clean reimport still needs a durable generation
claim, replacement Capture transition, optional reset choices, retained-byte
staging/validation, and the full restart/shared-work acceptance matrix.

### FX-05 complete implementation — locally accepted 2026-09-26

Clean reimport now uses the same Item with either its retained original package
or a browser replacement upload. A review step explains stopped/dismissed old
work, fresh automatic metadata, and optional manual metadata/artwork resets.
Manual choices, physical inventory and loans are preserved by default. Reset
choices take effect at admission; original bytes and observations are retained.
The prior current Capture remains usable until an archive Item revision switches
to the completed replacement. Other discs are unchanged. Generation-stamped jobs
and imports fence obsolete work, including generic retries and stale acquisition
completion. Independent scan/derivative owners retain shared work.

Retained copying checks actual bytes in cancellable background work, including
length bounds and final hashing. Failed browser uploads reopen with the existing
resume controls; retained staging offers Job retry. Archive preparation and private
admission keep the original command receipt recoverable. Portable lifecycle and
metadata cutoffs reproduce removal/current associations and explicit authority
resets after restart/reindex. Already queued historical files remain playable after
replacement and stop on removal or loss of read access. Reimport details are shown
only to their owning user, without exposing another curator's private workflow.

The shared guided-import destination editor compares the selected entry with the
saved association: unchanged attached or unattached destinations explicitly say
there is nothing to save. The save action appears only for a changed destination;
loading is distinct from unattached, and polling preserves a destination draft.

Implementation review and acceptance results are recorded in [validation](validation.md).
FX-06 moves collection access into its own collection-scoped panel, including empty
and content-read-only collections. Authorized grant search and pagination are
bounded in the Store; exact target lookup supplies the revision for updates beyond
the current page. The browser redeems recovery codes into the existing session,
shows safe one-time copy/download controls, and centrally resumes one pending
sensitive command after fresh password or passkey authentication while retaining
the owning draft on cancellation. Review fixes bind grant rows to their loaded
scope, fence and abort cancelled authentication attempts, and log out any fresh
replacement session when revoking the current session. Security policy, rate limits, revocation and
listener exposure remain in their existing server owners. Local validation is in
[validation](validation.md). FX-07 later passed local acceptance; the separate LA3
second-device qualification remains open.


### Work-context hardening — locally accepted 2026-09-26

The delivery in [work-context hardening](work-context-hardening.md) precedes FX-07.
Shared UI operation scopes guard results, errors and cleanup across target changes
and unmount. Commands waiting for authentication retain original account/inputs/key
and independent cancellation ownership. An optional actor precondition is checked
against the authenticated principal before API dispatch; session-specific revocation requires review
after authentication changes the session. Worker publication requires full claims,
derived assets require item generation plus durable source context, and job retry/
cancellation share generation/independent-owner eligibility. No persistent schema
or domain-identity replacement was needed. Required gates and browser journeys passed; exact results
are in [validation](validation.md). FX-07 later passed local acceptance; its
[interface audit](fx07-interface-audit.md) records combined coverage and the
bounded metadata candidate limit.

## Metadata candidate refresh and inspection — 2026-09-28

Metadata refresh replaces the active selectable candidate set rather than
accumulating superseded versions. An equivalent incoming candidate inherits the
current displayed choice; otherwise that choice remains selected and is marked
as unmatched. Manual field and artwork authority and archived provider responses
remain intact. Curators can inspect every active candidate's normalized fields
and compare two datasets side by side from metadata review. This is descriptive
metadata only, not catalog or integrity verification.

## Album-disc coverage and evidence presentation — 2026-09-28

The Album discs view orders known media by release position even when capture
records arrive out of order. Startup projects prepared-audio records without
reading their payloads; each derivative GET checks bytes against the recorded
length and digest before transfer. A mismatch has a specific player error.
Explicit full reindex still checks derivative payloads. Startup remains a full
metadata reindex in that dated checkpoint; P06 now implements reuse and indexed write validation (see the 2026-10-03 validation record).

Music browsing and curation now show release media beside attached original
packages, their recorded and provider-listed disc IDs, structural track count,
and AccurateRip outcome. Missing release media and unmatched originals stay
explicit. Curators can assign a source without a usable disc ID to a release
medium; a conflicting recorded ID or track layout withholds provider track names.
Track titles resolve per source/medium, rather than copying a global manual title
across multi-disc captures. The view offers the next structure, release, file or
audio-check action. Long processing details remain available in a collapsed
Advanced evidence panel, not in the primary collection workflow. This does not
establish that every physical disc or audio byte is correct, and no catalog read
performs full-payload hashing.

## Phono integration — PJ1 (updated 2026-10-07)

[New delivery ledger](phono-integration-plan.md): shared transport DTOs, indexed
song search, native client operations, authenticated range reader, Phono Keychain
service/worker, connection/search UI, remote personal queue and playlist editing
are implemented. FLAC vector/seek/truncation, continuous/mixed-rate PCM, journal
restart/exact-input, and real TLS range/create/replay/rename/conflict checks pass.
Native enrollment/restart, search tabs/artwork, repeated playlist edits and explicit
conflict reapplication, pause/resume/seek/repeat-one, persistent output and device
revocation passed. The user confirmed audible tone output. See the
[dated evidence and remaining acceptance](validation/phono-integration.md).
Collector, Phono, shared-library and affected sibling gates passed locally.
Focused audible gapless boundaries, remaining native failure/queue matrix and
physical second-Mac/LA3 evidence remain open. This does not complete
Trash Amp or unrelated native-client work. No preparation/offline/sync/discovery
capability is claimed.

## Account/connection usability repair — 2026-10-07

The [cross-app source review](account-connection-usability-review.md) identifies
remaining recovery, account administration and connection lifecycle gaps. Phono
now defaults to normal system-trusted HTTPS with existing browser-mediated device
authorization; Collector serves a real `/pair` approval page on client listeners.
Manual certificate entry is optional for owner-provided local trust. Saved system-
HTTPS credentials remain bound to the enrolled origin; pinned credentials retain
their trust. Retired `reset-password` is removed. Local recovery bypasses damaged
configuration and reports all revoked credentials. The remaining journeys in the
review are planned, not implemented; PJ1 physical-device acceptance remains open.


### Complete account/recovery/connection repair — 2026-10-07

The three software increments in [the accepted child ledger](account-usability-delivery.md)
are implemented. Enrollment is account-less until explicit recent browser approval;
status/cancel are secret-authenticated. Phono polls automatically, persists attempts
in Keychain, recovers partial finalization and separates saved accounts by server/user.
Actual legacy credentials convert only after `/me` verification; offline entries stay
unverified. Account switching invalidates reads/artwork and stops playback. Forget
retains uncertain edits and separates local removal from server revocation.

Account & security has Profile, Password, Passkeys, Recovery, Browser sessions and
Apps. Password changes retain access; separate other-access revocation and code
recovery apply the accepted atomic policies. Local recovery additionally revokes
passkeys and preserves enabled/admin/default authority and grants. Stopped lookup
and username-only recovery use configured account storage, bypass configuration with
`--state`, never open the archive and fail on live locks. `paths` lists configured
absolute locations and existence without opening storage, safe while serving.
Account creation exposes its durable identity before a separate permitted grant;
current/default badges and single-user default login refresh after mutations.
`setup` is the only initialization command. Private SQLite is schema 19; see the
ledger for conversion and backup/recovery guarantees.

[Validation](validation/phono-integration.md) distinguishes automated/browser/Keychain
evidence from native GUI, system-root deployment and second-Mac acceptance.

## Web navigation redesign — 2026-10-07

The shell now separates Browse (Library/Lists), Organize (Imports/Metadata/Music
preparation/Activity), Manage (collection access/server administration), and
account commands. Collection selection is a searchable, paginated disclosure
above the workspace with visible scope. Activity replaces the duplicate sidebar
Work & history entry. The misleading Manage collections command, which opened
an entry editor, is removed; Add / import remains a workspace action.

A blue navigation rail, clear active-page marker and task descriptions replace
the former mixed sidebar. Mobile uses a keyboard-accessible Menu disclosure;
its groups scroll without compression, with focus restored after selection or
Escape. Short desktop viewports and the persistent player retain reachable
navigation. Existing routes, permissions, cancellation and playback ownership
remain in their established modules. See the
[design and delivery record](web-navigation-redesign.md).

## Local certificate download and trust guidance — 2026-10-07

Administration → Server settings now offers the public HTTPS PEM download,
its existing PEM-file fingerprint and explicit macOS/Phono trust instructions.
It uses the authenticated pairing API and revokes its transient Blob URL on
panel cleanup. Failure/retry and mobile layout are qualified in the browser.
Startup/doctor distinguish the same-computer localhost HTTPS address from
advertised DNS names, print the absolute certificate file/fingerprint, and link
to management download/help. `paths` includes the public certificate location.
Phono shows matching self-signed help and an HTTPS URL example; collector-client
connection/timeout failures now give actionable diagnostics without inferring
security policy from error text.

Live localhost:9843 was reachable and validated with the server’s own public PEM;
normal system trust rejected the self-signed certificate and the advertised
junk-collector.local name did not resolve. Browser acceptance proved downloaded
PEM equality/fingerprint/no private key, certificate-validated HTTPS and local
loopback browser approval through one-time issuance. OS trust is not installed
automatically; this follow-up does not claim native GUI/system-root or second-Mac
qualification. See the [account delivery ledger](account-usability-delivery.md).

User verification — 2026-10-07: the user confirmed that local Phono-to-Collector
connection works and requested committing both projects. This establishes local
connection success; the trust mode and full native acceptance matrix were not
specified. Remaining failure/account-separation cases and second-physical-Mac
acceptance stay open.

## Trash Amp client integration (2026-10-07)

Shared list-entry DTOs now include authorized source coordinates and current audio
availability with recorded file length/digest. `collector-client` media readers
honor both enrolled system HTTPS and owner certificate trust. Connection errors
use client-neutral wording. OpenAPI and generated browser types are updated.
Collector workspace tests, architecture checks, Clippy, web check/build and real
Chrome smoke passed locally. The Rust server was rebuilt after asset generation.

The sibling Trash Amp delivery exercises real TLS enrollment, colliding item IDs
on two servers, revisioned duplicate list occurrences, real FLAC stream decoding
and retained offline decoding. Native Slint enrollment/artwork/list/playback and
download were exercised locally. This does not establish native audio fidelity or
complete all native adversarial acceptance scenarios; see Trash Amp's
`docs/collector-connection.md`. It does not advance unrelated handoff assignments.

## Preparation action clarity — 2026-10-08

Album preparation now shows a ready summary instead of a submission form when
all selected originals have complete FLAC outputs. Partial selections offer
missing FLAC preparation; scopes already covered by active preparation show
disabled progress. Track and album admission immediately reflect accepted work,
and plan polling also follows direct/other-user preparation after reopen. Bulk
FLAC review suppresses start when every selected track is already ready. Existing
Opus track actions remain available subject to encoder capability. Force-reencode
with replacement is not implemented; ordinary preparation still reuses valid
outputs. Scan, guided-import recovery and work-history controls were inspected
for equivalent start/retry conflicts; this is a focused audit, not full U05 acceptance.

## Frontend wording and optional help — 2026-10-08

The web UI retains collector terms including metadata, proposed, FLAC, Opus,
MusicBrainz and AccurateRip, while using Review metadata, Save originals,
Save destination and task-oriented progress labels. User-facing permission and
import/grouping messages describe actions rather than schema/authority machinery.
Short metadata, audio-format, import-main-file and password-length explanations
use a shared keyboard/touch-accessible info control. IDs, reference digests,
raw module data and detailed AccurateRip computation coordinates remain available
under expandable Technical details. Actionable failures, partial/inconclusive
results, destructive reset choices and security requirements remain visible.
This changes presentation only; evidence, authorization and workflow policy remain
unchanged. Browser checks cover info-control keyboard/mobile behavior and Escape
inside a parent dialog; validation outcomes are recorded separately.


## Concurrent processing and response reuse — 2026-10-10

Initial 2026-10-10 implementation (superseded by the configurable-worker follow-up
above): two global processing slots plus one reserved provider-download slot,
including two processing Jobs from the same account. Direct, guided and metadata
work use durable collection-scoped shared acquisitions with private requester Jobs,
independent cancellation and committed-result recovery. Reconciliation runs every
five seconds independently of workers and traverses unfinished workflow pages.
Identical AccurateRip/MusicBrainz responses reuse canonical snapshots; unchanged
metadata resolution does not append redundant authority/candidate revisions.
The web exposes typed provider progress and resumes pending AccurateRip requests
with their original receipt after reopening.

Administrator CLI backup/inspect/apply commands convert proven existing duplicate
payloads with both roots exclusively locked and a verified full backup. Historical
IDs and evidence remain readable through portable checksummed references, including
reindex and archive-only recovery. Unknown or unresolved inputs are skipped;
corruption fails closed. See [conversion](snapshot-duplicate-conversion.md) and
[delivery validation](validation/concurrent-processing.md). No live archive was
converted. Full-source validation is retained; disk contention and hashing redesign
remain separate work. This does not qualify live-provider or real-disc HL-06 checks.

### General document attachments — 2026-10-21

Post-alpha.3 source supports PDF and UTF-8 text Assets, up to 20 MiB, attached
through the existing subject, revision, visibility and archive pipeline. The
Documents editor defaults to Curators only and explicitly offers sharing. The
item view provides original downloads, browser-native PDF viewing and escaped
128 KiB text previews with a truncation notice. No server PDF renderer, PDF
structural-validation claim or artificial thumbnail is provided. Documents cannot
be selected as artwork. Originals and visibility survive reindexing; archived
originals remain in history. Existing purchase companion preservation is unchanged.
Shared byte classification/read bounds live in junk-libs-core, pinned to
`193e0a88faaa0c3d5636976932f120c237831ad0`. See
[acceptance and limits](validation/documents-2026-10-21.md). These changes are not
in the published alpha.3 images.

### Optional backup-age reminder — 2026-10-21

An optional `[backup_reminder]` section enables an administrator Server settings
notice and `doctor` output. Full-backup attempts record a small private receipt;
only successful final validation advances the completion date. A failed attempt
retains the previous date, and abrupt interruption stays unconfirmed. Missing,
malformed, unsupported or future history stays unknown. Status reads one bounded
receipt without scanning backups, opening media or rehashing bytes. External
completion/failure dates can be explicitly reported in configuration and remain
labelled operator reports that Collector has not observed or validated. The
reminder never schedules or creates a backup; removing its configuration disables
it. Existing backups without receipts stay unknown, and restored copies retain
their captured history rather than inventing a new validation. See the
[operator instructions](local-alpha-installation.md#optional-backup-age-reminder)
and [acceptance](validation/backup-reminder-2026-10-21.md). Published alpha.3 is unchanged.

### Native list response/restart recovery — 2026-10-21

Trash Amp source `0493b59` fences the complete inputs of an unresolved playlist
receipt before HTTP. Actual TLS tests truncate responses after genuine create and
append commits, reopen the native profile and retry saved requests without extra
playlists or entries. All eight affected adapter tests and strict Clippy passed;
the actual-output callback remains an existing opt-in test. The staged native
release checkout is preserved. See [acceptance and remaining physical/media-read
limits](validation/native-failure-recovery-2026-10-21.md). This source change is not
a new native or Docker release.

Native source `8afb19d` additionally qualifies real grant/device revocation after
an actual successful network chunk: the next uncached byte is denied to the
separate Viewer. Nine affected adapter tests and strict Clippy/formatting pass;
buffered and retained offline bytes are outside that revocation claim. See the
same [bounded native acceptance](validation/native-failure-recovery-2026-10-21.md).

Native source `339a871` adds accepted real-TLS range rejection/recovery,
header/body-stall cancellation and pinned-identity rejection with complete-catalog
retention. Eleven affected adapter tests pass with strict Clippy and formatting;
the physical-output callback remains an existing opt-in check. Native UI queue
continuity and the remaining physical-device/network/trust scenarios retain their
HL-04 limits. See the same
[finite acceptance](validation/native-failure-recovery-2026-10-21.md).
