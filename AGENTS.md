# Junk Collector development

Read `docs/architecture.md` before adding or changing a subsystem. `docs/implementation-status.md` distinguishes implemented behavior from planned capabilities; keep it current. For continued implementation, follow the active assignment in `docs/remaining-work.md` and read `docs/release-requirements.md`. The consolidated `docs/remaining-work.md` is the only remaining-work queue and supersedes older plan/handoff orders. Follow `docs/decisions/home-lab-scope.md`: Phono Junk is no longer a target, old proposed contracts require reassessment, and repeated metadata/artwork lookups do not require permanent provenance history. Implement one complete user-facing delivery across layers, not one layer per handoff. Complete its dependencies and acceptance without silently broadening scope.

- This is architecture-first. If a module has no natural place, revise the boundaries before implementing it.
- Domain/core code has no HTTP, filesystem, database, GUI or device dependencies. Application services own policy; adapters own mechanisms.
- Never describe integrity hashing, structural inspection, provisional metadata or MusicBrainz identity as catalog verification.
- Preserve raw masters, meaningful collection evidence and sources supporting accepted metadata/artwork. Manual metadata authority survives refreshes. Unknown module data round-trips without loss. Existing records need an explicit safe retention/conversion policy before compaction; do not require indefinite retention of repeated lookups or redundant provider responses.
- Public/private access is listener/router composition, never a guess from source IP or forwarded headers. Both pages and backing APIs must honor exposure settings.
- Artwork belongs beside game/CD text rows, including search and offline browsing.
- Share format/audio algorithms through narrow `junk-libs` crates. Do not import old app orchestration. Extract shared code separately and check affected sibling projects.
- Document incomplete capabilities honestly; never substitute a successful-looking mock for a missing implementation.
- Remove clearly pointless checks or tests when encountered. Keep assertions tied to meaningful features, visibility, correctness and real failure/recovery guarantees; do not freeze incidental implementation or layout choices. Do not preserve or rewrite a useless assertion merely to make it pass.

## Lessons from implementation reviews

- Keep browsing, availability polling, and album admission free of full-payload hashing. Resolve recorded observations with committed source fingerprints; validate actual bytes in cancellable background processing before encoding/publication. Reusing a helper must not accidentally turn a catalog query into a disc-sized integrity scan per poll or per track.

- External download forms are not stable APIs. Validate advertised field values, not just names; stop the batch on refusal and do not retry a blocked provider. Bound expanded archives and validate cache digests before reuse. Use offline fixtures for repairs.

- Shared background work needs durable request ownership, not a frozen “owned” boolean. Commit references with admission, fence child admission with the parent claim in the same transaction, retain the first immutable Job timestamp on convergent requests, and test later direct requests plus last-owner cancellation with an advancing clock.

- Recover all pages of unfinished workflows periodically, not only the first page at startup. Commit the upstream Job outcome before advancing downstream work; preservation is not the end of enrichment. Persist terminal/cancelled state, fence generic Job retry as well as specialized commands, and keep UI polling until the whole requested workflow settles.

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
- Follow `docs/decisions/pre-release-evolution.md`: do not add compatibility wrappers or migrations merely for checkpoint stability. Preserve actual data and recovery guarantees. When completing work, update current status, relevant delivery records and `docs/remaining-work.md` together; retain clearly dated historical validation records.

## Change-based validation

Use [the maintained test selection policy and commands](docs/testing.md). Inspect
staged, unstaged and relevant untracked changes; release selection covers the whole
candidate since its validated baseline. Preview with `python3 scripts/check_changes.py
--dry-run` and select the smallest meaningful check set.

- Documentation and human-facing text-only edits do not require unit tests. Do not
  add tests that merely freeze copy. Check documentation links; for UI copy run web
  type checking/build and inspect the affected screen, including mobile wrapping,
  overflow and accessible names. CLI copy requires compiling the affected executable
  and inspecting output. Review the entire diff before declaring `ui-copy` or
  `cli-copy`; machine-readable strings, identifiers, URLs and protocol/configuration
  values require checks for their actual behavior.
- CSS/layout changes require focused rendering checks. UI behavior changes require
  related model/browser tests. Rust changes require affected package/consumer tests
  and Clippy plus architecture/format checks. Authentication, authorization,
  storage/recovery, shared contracts, dependency/toolchain changes and uncertain
  scope require broader workspace coverage. Beta releases use the documented
  change-based set; RC/stable require full qualification of functional changes.
  Reuse unchanged evidence only with a recorded applicable full baseline. Report
  selected, failed and omitted checks honestly.
- Use the shared browser harness and supported launch context. On macOS, browser
  tests must run outside Codex's seatbelt execution context; the harness refuses
  that known-broken launch before starting Chrome. Use the permitted execution
  tool rather than repeatedly trying the failing context. Missing browser binaries
  and launch failures are infrastructure failures, never passed UI acceptance.
  Keep temporary state isolated and teardown bounded.
- Deploy release-mode executables only. Build each deployment target once, test
  the resulting artifact, and publish those same validated bytes. Debug builds
  are development/test tools, not deployment prerequisites. Rebuild a serving
  binary after changing its embedded web assets. Do not run duplicate standalone
  release builds when the container is the sole deliverable.

The full source set remains architecture checks, `cargo fmt --all -- --check`,
`cargo test --workspace --locked`, `cargo clippy --workspace --all-targets --locked
-- -D warnings`, web type checking/build, web model tests and the broad browser
suite. Run it with `python3 scripts/check_changes.py --level full --classification
infra`. `npm --prefix web run test:smoke` is short acceptance;
`npm --prefix web run test:integration` retains the broad provider/media/recovery
journey, including virtual WebAuthn. See the policy for installing test browsers,
explicit binary paths, feature journeys and release-image acceptance.
