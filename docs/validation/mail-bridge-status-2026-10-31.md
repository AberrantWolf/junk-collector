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

Pending: commit/push, build and test the final committed-source images, take a
complete stopped-writer Fi/NAS recovery pair,
then update Fi and verify identity/TLS/data, isolation, status freshness and
unrelated services. No public registry release or new release tag is requested.
Actual Proton login/receipt-template calibration remains an operator workflow;
the status monitor itself does not read message contents.
