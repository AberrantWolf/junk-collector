# Beta.21 qualification, publication and Fi update — 2026-11-04 JST

The user authorized review, fixes, pushing, a new release and Fi installation.
GitHub failures must be diagnosed and repaired before another attempt unless the
cause is demonstrably external. This record distinguishes completed outcomes from
pending work; earlier source-only acceptance remains dated history.

## Candidate and review

Beta.21 includes normal authenticated website audio by default, a revision-fenced
persistent admin route preference and the unreleased cancelled-publication cleanup
from `71eaf64`. The [audio record](audio-connection-policy-2026-11-04.md) owns the
cross-layer contract, prior local coverage and initial beta.20 configuration change.

Review found a fresh-volume deployment failure in the initial optional-authority
preflight: this remote-archive profile always starts media, whose first startup
requires its service key. The repaired guard requires the provisioned authority
while permitting normal audio. Core HTTP readiness follows authority startup.
Offline coverage checks normal mode, missing authority and incorrect sockets.
Fi will regain its retained endpoint configuration during the stopped update,
with normal audio still selected, enabling future admin-only switching.
No remaining actionable authorization, revision/receipt, byte-read or client-route
finding was identified. Five existing Rust ignores and the local rootless-skopeo
skip remain unqualified rather than silently counted as passes.

The official Rust stable manifest was rechecked on 2026-11-04. It remains dated
2026-10-15 and selects Rust 1.99.1 (`58cf8f9c3`, 2026-10-13); repository, CI and
Docker pins remain unchanged. No external dependency or toolchain upgrade is part
of this release; workspace package versions alone advance to beta.21.

## Qualification and installation status

Exact-source GitHub source checks, both native image/runtime gates, AMD64
release-image browser acceptance and sealed-artifact publication are pending.
Fi remains healthy on beta.20 with normal audio enabled. Before installation,
verify pending work and retain a matched stopped-writer Fi/NAS recovery pair.
Only qualified bytes may be installed. Preserve identity, originals, private state,
manual authority, meaningful evidence, credentials, Bridge/Caddy/tunnel topology
and unrelated services. No production Jobs will be cancelled to make the update
convenient. Final outcomes, digests and any failed attempt will be recorded here.

## Previous main-branch GitHub failure and correction

Checks [41623865565](https://github.com/AberrantWolf/junk-collector/actions/runs/41623865565)
at `71eaf64` failed in integration at the immediate metadata-search row count.
The assertion slept 350 ms and sampled cleared results before asynchronous query
completion. It now waits for the exact successful Library search response and its
single matching rendered row. A one-use 600 ms delay forwards the actual request
rather than inventing successful results, defeating the old fixed-delay shortcut.
The adjacent manual-title lookup relies on its existing actionable-row wait.
This was diagnosed and repaired before pushing another candidate, not retried as
an unexplained hosted-runner failure. Final qualification remains pending.

## Reviewed local checks before push

The repaired integration journey passes in Chrome with its deliberately delayed
metadata search (151.66 seconds for the complete journey); the remote-media
workflow also passes, including default signed-in playback and immediate admin
route switching. Focused store settings, native/private TLS and isolated-media
Rust tests pass. Preflight/container-tools, web type checking/build, generated
contracts, architecture/document links, format and diff checks pass. Broader prior
local coverage is retained in the dated audio record; clean exact-source release
coverage must still come from the candidate workflow.

Fi preflight reports 123 Items, one user, two grants and 6,811 Jobs, with no unfinished
Jobs, paused mail connection revision 2 and a fresh synced Bridge. Local Btrfs has
about 458 GB free, and the archive remains the intended hard NFSv4.1 mount. These
read-only observations do not constitute deployment or a recovery point.
