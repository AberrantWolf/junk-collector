# Bridge status and web setup — 2026-10-31 JST

Requested delivery: a status query that does not interrupt Bridge, visible sync
progress in Junk Collector, and a connection flow before a mail profile exists.
This extends the [Fi installation](fi-email-deployment-2026-10-31.md); it does not
change Bridge's account-wide cache or automatically enable receipt scanning.

## Implemented boundaries

- The pinned official CLI runs under a container supervisor. Its observed sync
  events become one bounded atomic snapshot with heartbeat/event times, phase,
  optional percentage/elapsed/remaining time and public certificate fingerprint.
  Account names, raw logs, message IDs and private keys stay out of that snapshot.
- The visible Purchase inbox setup card polls independently of receipt review.
  Unknown/stopped/interrupted/stale reports remain explicit; concurrent-account
  progress is not averaged and 100% alone is not a completion event.
- Administration-composed setup routes require an enabled administrator. The
  profile owner comes from authentication, not submitted data. Local IMAP
  credentials/folders undergo a certificate-pinned read-only probe before an
  immutable private profile and input-bound receipt publish together. Existing
  operator profiles remain supported; successful web creation loads immediately.
  Receipt scanning still needs a separate explicit authorized collection action.
- `collector-mail status` reads the sanitized snapshot without stopping Bridge.
  The [Fi helper source](../../deploy/proton-bridge/fi-mail.py) is retained in the
  repository; Proton login/2FA remains a private terminal operation.

See [architecture](../architecture.md#bridge-setup-and-sync-status--2026-10-31),
[behavior/API](../email-purchases.md#bridge-status-and-web-connection) and
[operator setup](../../deploy/proton-bridge/README.md#current-fi-installation--2026-10-31).

## Review passes

Two review passes were performed in this chat, rather than by separate agents.

1. **Failure/recovery and subprocess ownership.** Found that a forced kill could
   be reported as a clean container stop, and waiting without draining shutdown
   output could block the child's exit. The supervisor now requests the official
   CLI exit, drains/discards bounded output while waiting, reaps its process, and
   reports nonzero on forced or unsuccessful shutdown. Initialization failures
   also enter cleanup. Tests exercise large output after work has started, normal
   exit, a child ignoring exit/TERM, actual PID disappearance and cleared progress.
2. **Authorization, private credentials, status consistency and UI lifetime.**
   Reviewed administration composition, enabled-user/owner binding, pin-before-
   LOGIN, input-bound retries, private file permissions, scope cancellation and
   explicit scan enablement. Found that a completion frame without an event time
   could be accepted. Missing event coordinates for reported activity/completion
   now fail closed. Terminal queries also reject boolean/nonfinite/oversized
   numeric data and non-regular files instead of inventing progress or blocking.
   Valid completion frames remain accepted. No credentials are returned by reads.

## Validation and encountered failures

The broad workspace baseline passes **565 Rust tests, 0 failed, 5 normally
ignored**, plus warning-denied workspace/all-target Clippy, formatting,
architecture/document links, web types/build and queue/context/labels models.
After the review changes, focused provider tests and workspace/all-target Clippy
pass again; generated OpenAPI/TypeScript contracts match. All **16** offline
parser/IMAP/reporter/query tests pass, including real process supervision, bounded
output, wrong-pin refusal before LOGIN, no-fetch connection probes, credential
control-character refusal, stale/partial/multi-account status and nonblocking FIFO
refusal. No real mailbox credentials/messages are used by development checks.

The focused real-browser journey passes setup against an actual disposable TLS
IMAP server, failed-login/no-publication, live profile creation, receipt-safe
retries across changed status, immediate refresh, enable/pause, reopening/restart,
viewer denial, partial/stale reports, desktop/mobile layout, and the retained
purchase accept/link/cancel/dismiss workflow. Desktop/mobile screenshots were
inspected. Browser launches use the shared harness outside macOS seatbelt.

An initial Clippy failure in new test code was fixed. A repeated workspace run
then exposed an existing AccurateRip HTTP fixture whose unbounded teardown hid
a scheduler-sensitive timeout assertion; that validation process was terminated
and is not counted as passed. The fixture now has cancellable accept/delay loops
and bounded socket I/O. Timeout behavior uses a clear response/deadline gap; exact
retry count is independently checked with fast HTTP 503s and a fourth available
response, so an unintended fourth attempt would be observed. A regression covers
unused-response teardown. Repeated focused runs and the full workspace pass.

One broad browser attempt timed out in the unrelated sparse Library attention
fixture. Its isolated rerun passed without a production code change. The complete
reviewed rerun passes all **30 browser journeys**, including the new connection
and partial-status checks. Failed attempts are not counted as qualification.

Earlier local release-image candidates passed disposable native upgrade,
configuration/inbox permissions, cleanup refusal/opt-in cleanup, FLAC, full
backup/restore and isolated-media/Caddy checks. A separate browser tested the
actual release image's served asset bytes, progress, TLS connection form,
enable/pause, reload persistence and mobile layout. The official Bridge image
also passed persistent GPG/pass/certificate and reporter restart/stop checks with
network disabled. Review fixes require final committed-source image builds and
artifact acceptance before live deployment; earlier candidate digests are not
the final deployment identity.

## Publication and Fi installation

Signed commit `18ae94b0f64c2bb0df30f2a43002d2d4dc455bd7` was pushed to main.
Its git archive SHA-256 is
`1760967505f710ba5e627a7be07fe273b894d28b1714d2e3fa763016619330fd`.
Fi built each final deployment target once from that archive, with the source
revision label; those exact tested bytes are installed:

- Collector/core and media: `sha256:6647dc5edae589b72180091fa09cc5978c5e21cee4be19d0af4fd0e0304a4c29`.
- Official Bridge plus reporter: `sha256:add5534fa0dcb507bbc9b6ea9040071abf037fdbe968645ff46a8b0a2975b5b1`.

Final native upgrade/configuration/group access, partial-upload update, read-only
inbox refusal and explicit cleanup, FLAC, SIGTERM, full backup/restore, media
isolation and Caddy fixture checks pass. The actual release-image browser passes
served asset equality, progress, desktop/mobile, TLS credential setup, immediate
enable/pause and reload persistence. The final official Bridge image passes
persistent certificate, heartbeat, restart and graceful zero-exit shutdown with
network disabled. Fixtures and their disposable homes were removed.

Before activation all five stack processes were stopped. The first strict stop
check refused nonzero legacy process outcomes and restarted the old stack; no
snapshot was accepted then. The recorded cutoff has core/media/Caddy exit 0,
cloudflared exit 2 and old unsupervised Bridge exit 143 after stop. No process was
left running; the new supervisor's zero-exit stop was separately qualified.
Complete read-only recovery points retain local databases/WAL, Bridge vault/cache,
configuration and the matching archive:

- Fi: `/var/backups/collector-fi-pairs/before-mail-status-20261031/fi-data`.
- FatChocobo: `/volume1/collector-recovery/before-mail-status-20261031`.

Snapshot database quick checks pass. The private Fi recovery directory additionally
retains paired-point metadata, old deployment/environment/unit, authority key,
image references, cutoff/after facts and final build/qualification logs.

All five production services are running; core is healthy. Server identity,
configuration/TLS files, 79 catalog items, user/grant counts, existing mount sources,
container isolation and all seven unrelated container start times are retained.
The existing proxy and tunnel container identities remain unchanged. Public HTTPS
health and served application asset equality pass; IMAP/SMTP remain refused over
Fi's LAN address. The fresh reporter certificate matches the original Bridge
certificate without a mailbox LOGIN. Bridge resumed its existing account sync and
reported **27.9%** at acceptance. `ssh fi-home /home/link/bin/collector-mail status`
works noninteractively and does not stop Bridge.

The operator can now enter local IMAP credentials in Purchase inbox and explicitly
enable scanning. No real mailbox credentials/messages were used by development
checks. Actual retailer receipt calibration remains an operator workflow. No
public registry release or new release tag was requested or published.
