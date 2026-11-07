# Beta release checks and upgrade baseline — 2026-11-07

Change checks v2 separates workspace Rust coverage from browser scope.
The beta.23 candidate diff against beta.22 selects workspace Rust tests and
Clippy with just the named Opus preparation and email purchase journeys.
Short browser smoke is covered by the AMD64 release image. An explicit full
plan retains all browser journeys, and a candidate given itself as its
release baseline falls back to full selection.

The native image receipt identifies smoke or extended qualification. Both
architectures must still pass their runtime checks, and AMD64 must pass
the browser gate before the exact OCI exports can be sealed. Release-tag
pushes skip branch source checks. Publication reuses the sealed image and
revalidates its identity and digests. The synthetic rootless OCI transport
test runs during candidate assembly.

The selector, release policy, container tools, validation receipts, mail
fixtures, architecture links, formatting, workspace Clippy and web
types/build/models passed locally. The workspace suite passed 640 Rust tests
with five existing ignores. Workflow YAML, shell and embedded Python
syntax checks passed, including local execution of the runtime scope decision
for ordinary beta, full, RC and missing-baseline cases. Short browser smoke
passed in 4.92 seconds against the debug server built in the worktree.
The full browser suite was only planned. The Docker daemon was unavailable,
so native release-image smoke, AMD64 release-image browser smoke and hosted
publication were unqualified locally. The rootless OCI transport test is
reserved for artifact assembly and was unqualified locally because skopeo
was unavailable.

The [supported direct upgrade baseline](../decisions/pre-release-evolution.md#compatibility-policy)
is beta.22, catalog schema v15 and private SQLite schema v23. This delivery
changes the support policy and qualification, with no format conversion or
production deployment.

## Review follow-up

Change checks v3 verifies the published release record and validation run for
the supplied version tag before using it as a baseline. Unverified inputs
select full source and runtime coverage. Local workflow checks exercised the
rejected ancestor and verified beta.22 tag, and the verifier accepted the
published beta.23 record. The selector now uses the browser runner's journey
map to include changed tests alongside any named journeys. Focused selector
and evidence tests passed. Web types, build, models, short browser smoke and
the email purchase journey passed in the merged worktree. Opus selection was
checked by the selector tests. The browser Opus journey requires opusenc,
which was unavailable locally.
