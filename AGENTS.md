# Junk Collector development

Read `docs/architecture.md` before adding or changing a subsystem. `docs/implementation-status.md` distinguishes implemented behavior from planned capabilities; keep it current. For continued implementation, select one ticket from `docs/handoff-backlog.md` and read `docs/release-requirements.md`; complete its dependencies and acceptance criteria without silently broadening scope.

- This is architecture-first. If a module has no natural place, revise the boundaries before implementing it.
- Domain/core code has no HTTP, filesystem, database, GUI or device dependencies. Application services own policy; adapters own mechanisms.
- Never describe integrity hashing, structural inspection, provisional metadata or MusicBrainz identity as catalog verification.
- Preserve raw masters and provider observations. Manual metadata authority survives refreshes. Unknown module data round-trips without loss.
- Public/private access is listener/router composition, never a guess from source IP or forwarded headers. Both pages and backing APIs must honor exposure settings.
- Artwork belongs beside game/CD text rows, including search and offline browsing.
- Share format/audio algorithms through narrow `junk-libs` crates. Do not import old app orchestration. Extract shared code separately and check affected sibling projects.
- Document incomplete capabilities honestly; never substitute a successful-looking mock for a missing implementation.

## Lessons from implementation reviews

- Bind command receipts to submitted inputs before resolving mutable server configuration. Share the actual effect implementation rather than nesting independently committed commands.
- Keep provider workflow policy in application services; a frontend should not compose publication and enrichment into a second domain pipeline. Persist completion receipt inputs across panel reopen, fence every asynchronous continuation, and give sibling panels distinct stable React keys.
- Release media elements when their refs detach, not only in passive-effect cleanup; fence superseded play promises. Test the detached element’s paused/source state and real playback before close or revocation, rather than checking only DOM removal.
- Send typed availability/permission fields to clients; do not infer behavior from display or error text. Cancel obsolete UI requests and pollers, resume pending-job refresh after reopening, keep failures visible, and verify styles are included in served assets.

- Test repeated provider commands with advancing timestamps and enrichment of already-observed candidates. Include every resolved field/ranking/artwork input in immutable candidate versions, and keep current automatic candidates distinct from selectable history. Bound test-server teardown so a failed assertion cannot hide behind an unused HTTP fixture.
- Bind materialized metadata to the inventory revision it resolves. Later manual inventory edits must survive refresh; stable candidate and observation IDs cannot silently acquire different content. Test these conflicts before publication and again through reindex.
- Keep one provider acquisition registry and publication pipeline; adapt small responses to its streaming port. Share module-owned provider DTOs between producers and consumers instead of maintaining matching private structs.
- Keep parsing, byte decoding, normalization and reader traversal in shared libraries even when their inputs are authorized package handles. Collector modules map identities and apply domain policy; accepting readers rather than paths does not make a duplicated mechanism domain logic.
- Make evidence claims no broader than the work performed. Record coverage per algorithm/version; a candidate search is not an exhaustive search, and one matching file/track is not a complete package/disc match. Missing coverage stays inconclusive. Verify consumed snapshot bytes against their recorded length and digest before attributing new results to that snapshot. Retain actual computation coordinates even for partial or ambiguous results, and cross-check reported matches against frozen inputs. Test inconsistent result envelopes as well as valid outputs.
- Fail closed on malformed structural input instead of skipping entries, inventing lengths, padding missing bytes or ignoring unsupported directives. Test truncation, aliases, internal boundaries and the smallest supported inputs, not just ordinary complete fixtures.
- Enforce resource bounds and cancellation inside long reads, parsing and CPU loops. For subprocess work, check cancellation during validation and final hashing too; monitor I/O worker failures, terminate/reap the process group, and test failure after encoding rather than only cancellation before startup. A limit checked only after allocation or a cancellation check only after completion does not meet that contract. Test interrupted work and ensure it cannot publish success.
- Separate read authorization from write eligibility. Missing modules or unsupported schemas must not hide authorized historical evidence. Retain role-based redaction and cross-collection denial.
- Test claims with independent vectors and cases that defeat shortcuts (for example, v2-only matches without a v1 anchor). A green legacy test is not proof its expected behavior is correct; explain changes to expectations. Exercise new user workflows through their real UI, including setup and immediate refresh after mutations.
- Follow `docs/decisions/pre-release-evolution.md`: do not add compatibility wrappers or migrations merely for checkpoint stability. Preserve actual data and recovery guarantees. When advancing a handoff, update current status summaries, the child ledger and the ready-to-send prompt together; retain clearly dated historical validation records.

Checks: `python3 scripts/check_architecture.py`, `cargo fmt --all -- --check`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `npm --prefix web run check`, `npm --prefix web run build`. Rebuild the Rust server after changing embedded web assets. `npm --prefix web run test:smoke` uses a temporary local server/profile and a virtual WebAuthn authenticator; configure `CHROME_PATH` when needed.
