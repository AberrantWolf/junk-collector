# GitHub #9 error boundary review — 2026-11-03

Issue #9 is valid at `2b15b29`. This delivery reviews the issue's 75 flagged
boundaries. It does not claim an exhaustive audit of all implicit/framework/library
errors or every line in the original mechanical index. The table below resolves
each lead with a correction or the rationale for retaining its grouping.

Application services own workflow, eligibility and summary policy. Adapters own
HTTP, OS, subprocess, decoding and persistence diagnosis. Core errors retain their
existing categories; module callbacks can carry an original host error through
`ModuleError::HostError`, without HTTP/device/storage dependencies in domain code.
Native HTTP decoding respects the API code before shared-status fallbacks, retains
safe server descriptions and retry delay, and does not turn every failure into a
permission error. No new archive format, compaction or data migration is required.
The private enrichment-error column retains its data and existing barriers; a
separate visible workflow-coordination step presents it without inventing a stage.

Public errors remain sanitized. Unknown/disabled accounts, wrong credentials and
cross-collection resources remain protected. Media status bodies expose no file
paths, account state, bearer tokens or private authority details. Mail setup uses
only the existing sanitized connector categories, never raw IMAP/credential data.
An aggregate headline need not enumerate all subordinate failures when the complete
member/job results remain available; it must not claim that one item's cause
explains the entire batch. Fail-fast pipelines report the blocking child, rather
than claiming that every scheduled child was evaluated.

Confirmed mutation followed by failed refresh is explicitly identified in attachment,
metadata, publication, administration and account recovery flows. The attachment
form retains the confirmed Asset and retries only its read. Receipt cleanup failures
cannot turn confirmed digital admission into rejection; original retry identity is
retained. Invalid receipts are visible and cannot silently create a fresh command.
Optional queue/preferences drafts are not authoritative command receipts.

## Disposition of the 75 leads

“Retain” means the shared outcome remains accurate for all listed causes, with the
stated audience/detail boundary. It is not a claim that all possible diagnostics
are equally detailed. References use the issue's identifiers because source line
numbers have changed.

| Lead | Disposition and traced boundary |
| --- | --- |
| F01 | Corrected: Library substitutes transient guidance only for `catalog_busy`/`revision_conflict`; other 503/409 messages survive. |
| F02 | Corrected: shared 429 describes request rate/capacity, retaining Retry-After; it no longer diagnoses authentication attempts. |
| F03 | Corrected: bounded DAT streaming distinguishes oversized input (413) from unreadable/interrupted input (400), before accepting an upload. |
| F04 | Corrected: missing media is 404; operational open/metadata/seek/read failure is unavailable; changed/truncated shape remains conflict. |
| F05 | Corrected: native grant/control errors retain the original cause; only real authentication/permission revocation becomes PermissionDenied. |
| F06 | Corrected: the private general advancement barrier appears under Workflow coordination, not Metadata, Artwork or Staging. Actual Job stages retain their own outcomes. |
| F07 | Corrected: joined encoder I/O outcomes are inspected before a generic worker-stop error; failed cleanup is diagnosed separately and cannot replace the primary failure. |
| F08 | Corrected: poisoned mail locks/encoding are unavailable; no-clobber persistence is conflict only for AlreadyExists. Profile revisions retain real conflicts. |
| F09 | Corrected: inbox rename/directory errors state the failed operation and safe OS category, without asserting permissions or a mount cause. Originals remain retained. |
| F10 | Corrected: process-lock contention is WouldBlock; failures of the lock mechanism are storage errors with operator detail. |
| F11 | Corrected: only NotFound diagnoses the missing Python executable; other spawn failures identify startup and OS category. |
| F12 | Corrected: bulk permission redaction applies to denial/missing access; SQL/storage errors propagate instead of claiming revocation. |
| F13 | Retain core taxonomy: Invalid/Unavailable carry the originating safe explanation; Storage is operator-logged and publicly sanitized. Specific shared-status codes survive native/web boundaries below. No claim that every Invalid has identical repair steps. |
| F14 | Corrected host bridge: host categories survive callback conversion. Retain module capability/schema/operation Unavailable grouping: their distinct explanatory text survives, and none promises a successful operation or automatic retry. |
| F15 | Corrected: blocking JoinError is operator-logged; panic and task cancellation receive separate safe explanations. |
| F16 | Corrected: native decoder distinguishes API conflict/security/media codes and preserves other server messages/statuses. |
| F17 | Corrected: bounded native JSON reads decode error status/code instead of mapping every unsuccessful response to Forbidden. |
| F18 | Corrected timeout precedence. Retain the broad HTTPS connection category: reqwest's connect predicate does not reliably distinguish DNS/TCP/TLS; guidance lists possibilities without asserting one. |
| F19 | Corrected operation context for concurrent reads. Retain raw browser network errors when no stronger cause is observable; API status/code remain typed and unknown error bodies retain status. |
| F20 | Corrected: unreadable XHR envelopes retain HTTP status and do not imply an oversize/authentication cause; an unreadable successful response is unconfirmed. Same-package validation intentionally requires the complete original names/sizes. |
| F21 | Corrected: media authority conflict survives as 409. Retain 403/404 privacy grouping and bounded safe fallback for control-service failures; private upstream envelopes do not cross the restricted process. |
| F22 | Corrected: media read/early-limit errors return safe code/message bodies, including allowlisted CORS and no-store; capacity no longer looks like a browser permission/CORS failure. |
| F23 | Corrected: read deadline, unconfirmed authority and failed/changed bytes have accurate distinct stream diagnostics. Retain stream termination after headers: an established response cannot be replaced with a second JSON error. |
| F24 | Corrected: native metadata/range HTTP failures retain status before range-validator diagnosis. Retain grouped validator mismatch: an internally inconsistent range response is unusable whichever validator disagrees. |
| F25 | Corrected browser guidance: failure to establish a trusted route does not assert network reachability as its sole cause. Retain per-candidate native probe rejection: untrusted/unusable candidates cannot become usable routes; no candidate credentials are exposed. |
| F26 | Corrected: the service worker retains safe reply/exception detail instead of replacing every failure with public-fallback wording. Midstream errors still terminate the current read. |
| F27 | Corrected: missing audio no longer claims album permission revocation. Retain generic media-element failure until the separate status probe supplies evidence; HTMLMediaElement alone cannot establish a server-side cause. |
| F28 | Corrected: failed album selection identifies the affected track position and preserves the existing queue. Readiness/browser support remain independent facts. |
| F29 | Corrected common prepared-media explanation covers missing/unreadable/changed bytes, including export validation; it no longer diagnoses only length. Ordinary reads do not claim fresh whole-file digest checking. |
| F30 | Corrected accessibility: per-original check outcomes are in the accessible name as well as the tooltip. Retain the compact aggregate label; it does not replace those individual statuses or disc coverage. |
| F31 | Retain unknown historical result/label fallbacks: Details unavailable and Recorded check preserve readability without fabricating verification or successful parsing. Raw authorized evidence remains independently readable. |
| F32 | Retain bounded Job prose plus typed kind/phase/state/retryability and per-member records. A Job headline is a summary, not an unbounded provider dump. The harmful callback cause loss and concatenated aggregate failures are corrected separately. |
| F33 | Corrected: operational owner-check failures propagate; identical owner failures retain their common cause, while differing failures get one accurate aggregate eligibility outcome instead of the arbitrary last owner's diagnosis. No owners remains cancellation. |
| F34 | Corrected: heartbeat renewal returns its original error and guided-intent lookup is fallible. The operation's primary error takes precedence if both fail; heartbeat failure after successful work leaves its claim recoverable instead of marking the already performed effect failed. |
| F35 | Corrected: shared decoders/ranking receive cancellation signals, while their host captures and returns the original checkpoint error, including permission/storage/contention. |
| F36 | Corrected representative corrupt child/checkpoint/cleanup identities to storage diagnostics and malformed list offsets to Invalid. Real frozen-input/revision/claim mismatches retain conflict, preserving concurrency fences. |
| F37 | Corrected: ineligible preparation admission preserves reasons with track positions instead of substituting the first reason for every selection. |
| F38 | Corrected: bounded preparation headline counts failed/cancelled outcomes and points to the complete individual results; it no longer concatenates/truncates unrelated causes. |
| F39 | Corrected: storage/contention during bulk freeze propagates. Retain excluded for deterministic eligibility/dependency reasons; preview performs no provider acquisition or mutation for excluded members. |
| F40 | Corrected: completed metadata with artwork failure explicitly identifies partial completion. Retain failed/cancelled/review precedence for batch summaries, with individual preparation and AccurateRip results linked from the album. |
| F41 | Corrected: work summary includes CD-check admission errors. Retain active-first status while requested work remains active; terminal status is an aggregate with stages/Jobs in detail. |
| F42 | Corrected: catalog/game headlines count failed/cancelled tasks and refer to each visible result, rather than presenting one error as the batch's cause. |
| F43 | Corrected: all rejected packages retain their ordinal and reason, including when no package was accepted. Accepted/rejected partial results stay distinct. |
| F44 | Corrected: fail-fast matching identifies the blocking child Job/phase and its cause. Retain fail-fast policy: it does not claim an exhaustive list of future/unfinished child failures, and durable child results remain available. |
| F45 | Corrected: automatic grouping propagates storage/contention failures. Retain policy exclusions for incompatible/unsupported group membership; those are not successful group proposals. |
| F46 | Retain deferred summary rebuild: retrying accurately describes continued bounded rebuild attempts, with operator-logged cause and independent original record reads. It does not diagnose catalog contention or claim a successful projection. |
| F47 | Retain removal of unsafe write actions after failed write eligibility; it makes no public permission diagnosis and does not remove authorized read/history actions. Existing module/content permissions retain capability notices. |
| F48 | Corrected: malformed supported Music overrides cannot silently become empty assignments. Unknown/unsupported module/schema data still stays raw/readable. Retain tolerant generic historical presentation and unavailable catalog suggestions; neither substitutes invented parsed values for raw evidence. |
| F49 | Retain Unknown backup completion for absent/unreadable/inconsistent receipts: all mean no trustworthy known completion, not proof that no backup exists. Existing failed attempts do not replace successful completion; receipt-write failure does not replace the backup result. |
| F50 | Corrected: Bridge connection/revalidation uses safe authentication/certificate/folder/transport/response/adapter categories, without exposing credentials or mail. Probes occur before configuration publication. |
| F51 | Retain sanitized private setup-storage unavailability. Raw profile/receipt contents and serialization errors may contain secrets; a shared inability to read/save setup is accurate and cannot certify a configuration change. |
| F52 | Retain unavailable-or-stale Bridge snapshot: absent, malformed and stale data cannot establish live sync state. No stale snapshot is presented as completion; owner status/reporter inspection remains the remedy. |
| F53 | Retain sanitized adapter failure on nonzero exit/timeout/pipe/output errors: these all mean the adapter could not complete its protocol. Setup preserves safe connector categories when supplied; raw subprocess/mail data is not retained. |
| F54 | Corrected: eligibility operational errors propagate instead of silently skipping; real disabled/revoked accounts remain fenced before/after fetch. Unsafe provider exceptions remain the safe Adapter category. |
| F55 | Retain sanitized mail content/transport categories: credentials and message contents are deliberately excluded, while safe authentication/certificate/folder/transport fields survive. Unsupported content is a suggestion issue, never an accepted purchase. |
| F56 | Corrected: only revision_conflict receives list-edit conflict guidance. Other 409 code/message values survive. |
| F57 | Corrected: inventory/import special handling is code-based; a failed conflict refresh retains the original rejection and avoids unhandled async errors. |
| F58 | Corrected: confirmed attachment upload/save/archive is distinguished from failed refresh. Reloading a confirmed upload does not POST another attachment. |
| F59 | Corrected: confirmed metadata choices and catalog application have explicit refresh-failure wording; absent completed lookup results cannot claim artwork success. Failed panel reload remains visible and the existing lookup can be reloaded. |
| F60 | Corrected: default-account/catalog activation confirms the effect separately from settings/listing refresh failures. |
| F61 | Corrected: successful original publication plus failed read is explicit; a second status-read failure cannot replace the primary publication failure. |
| F62 | Corrected: login/recovery distinguishes completed authentication/credential replacement from failed account loading. Consumed recovery input is cleared and the message requests reload, not code resubmission. |
| F63 | Corrected endpoint labels for account snapshot reads. Retain atomic installation of the loaded form snapshot: a failed section does not fabricate empty security lists; load errors and confirmed mutation notices remain separate. |
| F64 | Corrected: independent mail loads retain successful profiles/connections/Bridge status, and failure messages identify the failed endpoint. Profile failure cannot erase a successful Bridge status. |
| F65 | Corrected context for concurrent playback/preparation, work, grouping, curation and capability reads. Retain Promise.all for computations that require every input; a labelled rejected read identifies its own operation, not a failed mutation. |
| F66 | Corrected: missing account labels/security capabilities now expose read errors; genuine unavailable optional features remain distinct from their failed load. |
| F67 | Retain Preview unavailable for image-event failure: browser events cannot safely distinguish denied/missing/network/decoder causes. Original/asset endpoints and authorized metadata remain available; no cause is invented. |
| F68 | Corrected: queue rejection and unconfirmed outcome have different headlines; rate-limit/in-progress/timeout remain uncertain and retain the same receipt. |
| F69 | Corrected authoritative receipt paths: cleanup cannot undo successful admission, failure to persist feedback cannot replace the primary error, corrupt receipts stay visible and cannot auto-admit a new command. Retain best-effort personal queue/preferences drafts as optional non-command state. |
| F70 | Retain bounded composite input validators where a safe rule/record description covers every rejected condition. They reject before mutation and do not claim concurrency, permissions or successful acceptance. This review does not replace the repository's domain validators with field-by-field wire schemas. |
| F71 | Retain fail-closed structural parser errors: malformed shape/flags/ordering/alignment share invalid structural input, not a fabricated cause or verification result. Existing independent truncation/boundary vectors remain meaningful; sibling algorithms were not rewritten. |
| F72 | Corrected operator startup keeps the provider's already-sanitized load/setup cause instead of one generic file/path diagnosis. No raw profile/credential bytes are printed. |
| F73 | Retain CD naming's bounded invalid-envelope categories; transport/timeouts/process causes already survive and CLI identity errors retain their cause. Atomic-rename fallback is capability absence, not permission failure masquerading as success. |
| F74 | Corrected: bearer fallback occurs only on actual credential rejection, never a storage/operational error. Retain deliberate account/password/passkey public redaction; successful authentication still checks credential kind and principal. |
| F75 | Retain cross-collection/resource denial and invalid authority privacy. Improvements above do not reveal protected existence or remove authorized historical evidence. |

## Review and validation

Implementation review checks commit outcome, retry identity, role/exposure redaction,
claim/cancellation fencing and whether a message asserts more than its operation
established. Review corrections include expiry-aware contention handoff (separate
#11 delivery), callback cause retention, committed-attachment read-only retry,
partial-completion wording, receipt cleanup/corruption, and media error CORS.

Focused native/module/application/provider tests pass, including a shared ranking
callback returning storage/permission/contention rather than cancellation, distinct
native 409/503/429 envelopes, grant renewal denial versus unavailability, and disk
full with a failing cleanup. Added server checks distinguish exact-size DAT,
oversize and interrupted bodies. Added media router coverage checks early capacity
failure with readable allowlisted CORS. Browser regressions exercise a genuine
attachment commit followed by failed refresh, unavailable Library cause retention,
and unreadable queue receipts without a new admission.

Final validation: the selected workspace run passes 633 Rust tests with five
existing ignores. The 50 affected application/client/media/server library tests,
worker-contention regression and corrected owner/provider tests also pass after
review. Strict workspace Clippy, formatting, architecture/link checks, web type
checking/build/models, tooling/container/release-policy/mail checks pass. All 30
browser journeys pass in the final serial run (568.51 seconds), including the
actual CD shortcut, mixed per-original accessible name, queue preservation,
confirmed metadata/attachment refresh failures, independent mail loads, and both
malformed JSON and invalid typed queue receipts. Mobile receipt/attachment
screenshots were inspected; no horizontal overflow or hidden receipt controls.

The selector reports `/tmp/jc-issue9-checks.json` and
`/tmp/jc-issue9-final-checks.json` retain their earlier failures honestly. Final
browser acceptance is `/tmp/jc-issue9-browser-final-serial.log`; final strict Clippy
and type-check output are `/tmp/jc-issue9-accepted-clippy.log` and
`/tmp/jc-issue9-accepted-types.log`. Review follow-up library results are in
`/tmp/jc-issue9-final-libs.log`. These are local development receipts, not release
image qualification. No production data, GitHub issue state, release publication
or deployment is changed.

The first full run exposed two expectations during review. The owner-availability
regression correctly required the real missing-module cause; the implementation
now retains identical/single-owner causes and uses a neutral aggregate only when
causes actually differ. The optional-provider test incorrectly attributed a failure
to artwork before an artwork Job existed; its meaningful preservation/manual-authority
assertions remain, and it now requires a visible failed coordination step with its
retained diagnostic. Focused reruns and full validation follow these corrections.

Focused browser review found a fixture advertising a short busy response as the
generic unavailable code. It now uses catalog_busy; a separate unavailable-code
case must retain its real diagnosis. Mail status polling keeps load errors separate
from mutation errors, so a successful later poll cannot erase a rejected command.

Final review also keeps successful work recoverable when heartbeat shutdown reports
a lease/storage error; it does not convert an already performed effect into a failed
Job. The worker runtime reports that error and normal claim recovery resumes it.

The corrupt-receipt browser case exposed that session-owned queue notices were
rendered only on Library, despite admission occurring in Metadata. Those notices
now render across ordinary app pages. The regression reloads the page so it tests
durable recovery rather than overwriting a still-known in-memory request.

The broad browser run reached the CD shortcut and exposed an exact accessible-name
assertion that excluded the newly included per-original outcomes. Acceptance now
checks the correct album row, the visible verified fraction, both Verified/Not
checked in its accessible name, and opening the actual audio-check controls. The
CD-library selector also permits the added accessible detail.
