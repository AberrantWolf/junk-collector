# Cancelled publication conflict prevention — 2026-11-03

> Deployment identifiers are anonymized; dates, source identities, outcomes and qualification limits are unchanged. Example addresses are not deployment instructions.


The user requested an underlying fix for the cancelled publication conflict found
on application-host, regression coverage, review, commit and push. This source delivery does not
publish another release or change application-host's installed beta.20. Its earlier inspected
quarantine and snapshots remain untouched.

## Cause and boundary

`publish_v4` persisted a journal and staged revision bytes, then returned a normal
cancellation without removing them. The cancelled Job became terminal while its
journal remained active. Before restart, a different Job could publish the same
observation identity with a new `created_at`. Startup compared that new committed
record against the cancelled attempt's old digest and correctly failed closed.

The mechanism belongs in the existing store publisher, under its archive writer.
The wrapper now cleans its own still-uncommitted attempt for typed cancellation,
claim loss and catalog-contention yields, before another writer can publish. The
existing abort primitive refuses committed revisions before removing any bytes.
Committed state and other fault/recovery behavior retain their existing policies;
startup integrity checks are unchanged. The processing-observation lookup also
propagates every error except `NotFound`, so contention or a damaged projection
cannot be mistaken for permission to publish over an existing immutable identity.
No public API, archive schema, compatibility migration or generic journal-ignore
rule was added. Pre-existing conflicting journals still require inspected recovery.

## Regression and review

`cancelled_processing_publication_cannot_poison_a_later_observation_or_restart`
failed against the original code at reopening the Store, with the same
`publication artifact digest mismatch` as application-host. It now passes with:

- Actual cancellation after journal preparation and after bytes are staged.
- Typed catalog-busy and lost-claim interruption after staging, followed by owner
  cancellation and another Job's publication of the same ID at a later timestamp.
- Rejected cancellation after commit and rejected mutation of that immutable record.
- An invalid existing catalog record propagating failure without a second journal;
  startup/reindex restores the disposable projection from the valid archive.
- Successful restart and explicit reindex, the expected observation intact, no
  abandoned journal/staging, and unchanged capture records and original bytes.

Review checked writer serialization across cleanup, the rename/commit boundary,
late-cancellation fencing, preservation on abort refusal, snapshot callers'
existing error policy and propagation of non-missing lookup errors. The phase
fixture also remains shared with the existing derivative cancellation regression.

## Validation

The focused regression passes all five phase/interruption cases. The full storage/
recovery selection was run with:

```sh
python3 scripts/check_changes.py --level full --classification infra --report /private/tmp/cancelled-publication-check-report.json
```

Architecture/format, web types/build/models, tooling/release/mail checks, **634 Rust
tests with zero failures and five existing ignores**, strict workspace/all-target
Clippy and browser-server compilation pass. The full command stopped in the
game-catalog browser fixture because its polling helper threw immediately on a
typed transient `catalog_busy` 503. This is retained as a failed initial run, not
reported as a successful command.

That test-only reader now retries only GET + 503 + `catalog_busy`, for at most five
seconds. No mutation retry is added; other read failures still fail immediately.
A one-shot injected
response and an explicit consumed-response assertion cover recovery after the
second import. The failed journey plus the remaining nine journeys then passed:

```sh
node web/tests/run-suite.mjs game-catalog email-purchases digital-matching purchases intake-review game-audio documents backup-reminder remote-media storage
```

Together with the unchanged first 20 passing journeys, this completes all **30**
browser journeys against the fixed server. The final injected-response assertion
also passed a focused game-catalog rerun. Unchanged Rust checks and earlier isolated
browser journeys were not repeated after this test-only repair. Documentation was
updated afterward and architecture/link/diff checks rerun.

Logs/reports are retained in `/private/tmp/cancelled-publication-checks.log`,
`/private/tmp/cancelled-publication-check-report.json`,
`/private/tmp/cancelled-publication-browser-resume.log` and
`/private/tmp/cancelled-publication-game-regression.log`. No production data,
profiles, provider requests, release tags or deployment images are changed.

## Release installation — 2026-11-04

This delivery is included in beta.21 at signed source `280b04f`, with full exact-source
checks and native image qualification. Fi was updated from the exact published
image after retaining a matched stopped-writer recovery pair. See [the completed
release record](container-beta21-publication-2026-11-04.md). The earlier source-only
validation and its failures above remain dated evidence.
