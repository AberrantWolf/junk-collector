# Wave 3 integration review

Review date: 2026-09-10. This document closes the local Wave 3 integration
checkpoint. The child-by-child implementation record and commit inventory remain
in [the live ledger](wave-3-plan.md).

## Acceptance trace

| Root | Implemented evidence | Not implemented / externally unqualified |
|---|---|---|
| P02 durable jobs | Typed schema-v8 migration, fenced FIFO claims, two-global/one-user bounds, lease takeover, throttled progress, integrity consumer, bounded cancellation, same-ID retry, publication reconciliation, submitter-scoped cursor/detail/mutation API, and upload/inbox worker migration are covered by the durable-job, claim, cancellation, retry, recovery, API-scope, and secret-redaction Rust tests. Six focused cancellation-boundary tests and two claim/restart tests pass. | C06 event delivery and distributed scheduling are not implemented. Target-Linux process/filesystem behavior and independent review are externally unqualified. |
| U03 physical lifecycle | Focused revisioned specimen/component/loan/disposal commands, history preservation, two-copy and multi-disc behavior, viewer redaction, conflict reapply, and restart/reindex round trips are covered by unit/integration tests and both Chromium suites. | Specimen deletion and trading are intentionally not implemented. Independent privacy/accessibility review is externally unqualified. |
| U04 supporting assets | The shared image pipeline, standalone revisioned Asset commands, role-filtered DTO/bytes, deterministic primary artwork, compatibility URL, keyboard/mobile editor, and tombstone retention are covered by invalid/oversize/fault tests, authorization/reindex tests, and Chromium interaction checks. | Document attachments and offline artwork are not implemented. Independent visual/accessibility review is externally unqualified. |
| P03 provisional imports | Schema-v9 Imports, browser/inbox convergence, durable offsets, source rechecks, optional analysis limitations, preserved stale-review decisions, preallocated-identity publication, unattached Capture publication, cancellation/reconciliation, and legacy delegation are covered by persistence, restart, adversarial-source, review, publication, and compatibility tests. | Built-in game/music analyzers remain D04/D05 and are not represented as verification. Target-Linux/NAS and real abrupt-power behavior are externally unqualified. |
| P04 import/job UI | Normal and private-local Chromium suites exercise browser and inbox staging, inventory, review, unattached publication, active/history jobs, retry/cancel, refresh, offline/reconnect, stale revision reapply, unavailable modules, cross-user/collection denial, keyboard order, focus restoration, live announcements, reduced motion, and 390px layouts. | C06 event streaming is not implemented; polling is the working behavior. Independent browser accessibility/security review is externally unqualified. |
| D02 shared disc primitives | Junk Libs owns generic multi-hash/readers, neutral disc IDs, AccurateRip algorithms, and neutral DAT parsing; Retro and Phono consume the shared implementations. Published/synthetic vectors and affected workspace tests pass. | Collector catalog/analyzer integration remains D04/D05. Shared commits are not published/pinned. Private dumps, live providers/drives, and independent corpus review are externally unqualified. |
| D03 shared audio | Junk Libs owns opaque reader/PCM contracts, exact PCM-WAV frame seeking, opened CUE/redumper adapters, bounded resampling, gain/clipping, and deterministic queue state. Trash Amp and Collector consume those boundaries without moving filesystem, HTTP, GUI, or device policy into shared crates. Generated vectors and all affected workspace tests pass. | Shared compressed decoding, richer DSP, events, and device policy are not implemented. Published pins, CPAL/loopback hardware, private codecs/dumps, and target-platform review are externally unqualified. |

## Final gates

- Collector: `python3 scripts/check_architecture.py`,
  `python3 scripts/api_contract.py`, `cargo fmt --all -- --check`,
  `cargo test --workspace` (139 tests),
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `npm --prefix web run check`, `npm --prefix web run build`, and
  `cargo build -p collector-server` pass.
- Browser: the normal HTTPS Chromium smoke suite and `LOCAL_SETUP=1` private
  listener suite pass. The focused SIGKILL matrix (three tests), cancellation
  boundary matrix (six), reindex (two), durable claims (two), Import listener/
  security (two), Asset role filtering, and Job scope/secret-redaction tests pass.
- Junk Libs: whole-workspace formatting, all-feature tests, and all-target
  warning-free clippy pass. Formatting-only commit `91dd63d` resolves the older
  unrelated workspace drift found during the final gate.
- Phono Junk: whole-workspace tests and warning-free clippy pass.
- Retro Junk: whole-workspace tests pass; focused changed-package clippy passes.
  Whole-workspace clippy retains previously recorded unrelated warnings and is
  not claimed green.
- Trash Amp: whole-workspace formatting, all 42 executable tests, and all-target
  warning-free clippy pass; one hardware-oriented test remains ignored. Only
  Wave 3-owned hunks were committed.

## Review conclusion

Wave 3 is **implemented and reviewed locally**. No root is being closed on a
contract-only or final-looking placeholder. D04/D05 analyzers, D06 providers,
D07 derivatives, C06 events, document uploads, runtime plugins, distributed
scheduling, and all Wave 4 work remain **not implemented**. Hardware, private
fixture, target-Linux/NAS, published-pin, abrupt-power, and independent-review
gates remain **externally unqualified** and must not be inferred from the local
passes above.
