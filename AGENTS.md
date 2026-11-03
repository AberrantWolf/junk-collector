# Junk Collector development

Read [architecture](docs/architecture.md) before changing a subsystem. Use
[implementation status](docs/implementation-status.md) for capabilities and
[remaining work](docs/remaining-work.md) as the only open-work queue. Read
[release requirements](docs/release-requirements.md) for release work. Implement
one complete user-facing outcome across its layers, dependencies and acceptance.
Do not silently broaden scope or restart completed work from old plans.

## Invariants

- Domain/core code has no HTTP, filesystem, database, GUI or device dependencies.
  Application services own policy; adapters own mechanisms. Revise boundaries
  before adding a subsystem with no natural owner.
- Preserve exact originals, meaningful collection evidence, manual authority,
  accepted metadata/artwork sources and unknown module data. Existing records need
  explicit safe retention/conversion rules before compaction. Equivalent lookups
  do not require permanent duplicate history. Follow [scope](docs/decisions/home-lab-scope.md).
- Integrity hashing, structure, provisional metadata and MusicBrainz identity are
  distinct from catalog verification. Claims cannot exceed computed coverage.
- Public/private access is listener/router composition, never source IP or forwarded
  headers. Pages and backing APIs honor exposure; roles still govern access.
- Artwork belongs beside game/music text in browsing, search and offline views.
- Share parsing/audio algorithms through narrow `junk-libs` crates. Do not import
  old application orchestration. Extract shared code separately and check affected
  independent consumers. Phono is retired as a target, not evidence of passed acceptance.
- Keep queries, availability polling and admission free of full-payload hashing.
  Validate actual bytes in cancellable background processing where required.
- Never substitute a successful-looking mock for an incomplete capability.
- Keep tests tied to meaningful behavior, visibility, authority and recovery.
  Remove pointless assertions within affected work, explaining why their protection
  is unnecessary. Do not freeze incidental layout or wording.

Apply the relevant [review guidance](docs/development-review-guidance.md) for
shared Jobs, metadata authority, provider inputs, asynchronous UI and media lifecycle.
Follow [pre-release evolution](docs/decisions/pre-release-evolution.md): preserve
actual data/consumers, without checkpoint-only compatibility machinery.

## Documentation maintenance

Follow [documentation authority and maintenance](docs/index.md). Keep valid contracts
and open work in live docs; consolidate duplicates rather than discarding constraints.
Remove superseded instructions and use Git history for old prose, without creating
snapshot copies. Preserve existing dated validation/release evidence and its limits.
Update the owning contract, status and queue for their distinct purposes; record
validation once and link it. Use generic topology, paths and example domains. Never
name an individual operator's machines, SSH aliases, accounts or deployment endpoints.

## Validation and delivery

Follow [testing policy and commands](docs/testing.md). Inspect the complete staged,
unstaged and relevant untracked diff; preview `python3 scripts/check_changes.py --dry-run`.
Select the smallest meaningful set and report failures/omissions honestly. Prose-only
documentation needs link/reference review, not unit tests; policy changes need review
of their actual effects. UI/CLI changes require the policy's applicable build and
real screen/output acceptance, not merely text assertions.

On macOS, browser tests must run outside Codex's seatbelt context using the permitted
execution tool. Launch failures are infrastructure failures, not passed acceptance.
Deploy release-mode executables only. Build each target once, test and publish the
same bytes, and rebuild serving binaries after embedded asset changes. Container-only
delivery does not require discarded standalone release builds.
