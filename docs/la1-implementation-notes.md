# LA1 implementation notes

These notes settle only the immediate guided-ingest contracts used by LA1. The
typed production interfaces and tests remain authoritative.

## Durable intent and progress

- `ImportId` is the workflow identity. A guided intent is private state associated
  one-to-one with the existing provisional Import; it is not a second import
  lifecycle. It freezes the accepted plan fingerprint, automatic-enrichment
  choice, stable target `ItemId`, submitter and package decisions.
- Planning is read-only. It reports bounded package inventories, module-owned
  eligible anchors, typed blockers and provider availability. Acceptance
  reauthorizes and recomputes the fingerprint before atomically recording the
  intent and command result. New Item, Capture and Representation identities are
  deterministic from the accepted command and package position.
- The existing durable Job queue executes staging, analysis, publication and
  provider acquisition. A small application coordinator advances an Import only
  after reconciling the preceding durable effect. It never performs browser-owned
  chaining and never adds another scheduler.
- Preservation, identification and artwork have independent typed step states.
  Provider absence, no match and provider failure may produce a partial outcome,
  but cannot roll back a committed preservation master.
- Import list/detail views are submitter-scoped, collection-authorized and bounded.
  They expose safe retained attempts and actionable permissions, never stored
  requests, host paths, credentials, leases or claim tokens.

## Default-title and automatic-artwork authority

- A title derived from the accepted package name is a provisional display default,
  not a manual metadata override. Guided entry creation publishes an initial empty
  `MetadataAuthority` with the created inventory revision as its baseline. Until a
  candidate exists, resolution may display that inventory text; an automatic
  candidate may replace it.
- Editing inventory title/subtitle after that baseline is existing application
  policy for manual authority. Every automatic completion reloads the current Item
  and authority. A stale result is resolved around those edits or fails with a
  typed conflict; it never overwrites them.
- Provider artwork is first published by the existing bounded Asset effect. The
  automatic metadata refresh then creates a new immutable candidate version whose
  ranking inputs include that `AssetId`. It does not set the manual artwork choice.
  An existing explicit artwork choice therefore remains authoritative across
  completion, outage and retry.
- DAT names and MusicBrainz identity are descriptive metadata only. Integrity,
  structural inspection, catalog matching and audio verification remain distinct
  evidence and are never relabelled as metadata verification.

## Cancellation, retry and cleanup

- Cancellation requests fence future owned Jobs. Before the Capture commit, a
  discard may reclaim only private staging bytes; after the commit, originals and
  evidence remain and only optional downstream work is cancelled.
- Selective retry reuses the failed Job and its retained attempt history, or a
  stable step identity when a downstream step has not yet been admitted. It never
  allocates a second Item, Capture, Representation, observation, Asset or snapshot
  for the same accepted step.
- Published association correction is a normal optimistic Capture revision. It
  uses the current revision returned by ingest detail, changes only the association
  and preserves every Representation and evidence relationship.

## Independent review corrections — 2026-09-16

Acceptance resolves mutable inbox/provider configuration only inside a new command
receipt; replay does not re-run that preflight. The server dispatches validated
inbox paths best-effort after acceptance and periodically reconciles all pages of
unfinished work. An operational dispatch failure cannot undo accepted intent.
Upstream Jobs become terminal before downstream coordination, so a later failure
cannot turn an already committed publication into a failed publication Job.

Recovery repairs the Item-to-initial-authority crash gap, and provider refreshes
check the full observation history before replay. Manual and guided provider
refresh share their actual candidate construction/publication path. Artwork resumes
against the acquired release identity, independently of later display selection.

Discard receipts mean cleanup was accepted; they replay unchanged. Detail exposes
current reclaimed/pending bytes and completion, including empty packages. Deletion
holds the upload lock and waits for active workers. Cancellation and generic Job
retry/admission share transactional fences. Cancelling a provider Job through the
general job UI also settles the guided intent without cancelling its published Capture. Retry clears settled state atomically
with the Job retry receipt. Published association corrections report the current
Capture association rather than the original target.

History paginates and continues polling through post-publication enrichment. Requests
are aborted on close/selection changes, acceptance keys survive retries, and resume
uploads use a separate file selection with retained-prefix verification. Attempt
start times remain null where the existing Job ledger did not record them.

Current intake accepts up to 64 explicit packages atomically; the rejected-members
field is reserved, not partial admission implemented today. Browser selection is one
package; configured inbox selection supports several packages. Name-level readiness
cannot inspect CUE contents before upload. Ambiguous/malformed packages can be
preserved unidentified; guided anchor-selection and package-splitting editors are
not implemented. This is not all-format or full P04/U05 qualification.
