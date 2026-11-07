# Beta.23 publication and Fi update — 2026-11-07 JST

[Beta.23](https://github.com/AberrantWolf/junk-collector/releases/tag/v0.1.0-beta.23)
is a signed prerelease at source `ed65d66e2a27f219dca67f70ebf6b1305c980ead`.
It includes [direct Library audio batch submission](opus-preparation-2026-11-07.md#direct-batch-submission)
and the [Proton Bridge folder picker](mail-folder-editing-2026-10-31.md#folder-picker-follow-up--2026-11-07).

## Earlier checks and repairs

- The [original push](https://github.com/AberrantWolf/junk-collector/actions/runs/42364545369)
  failed the real Opus journey after a cancelled album batch left a running
  track. The repair settles claim-matched late worker outcomes and expired
  cancellations, and enables Retry after cancellation settles. The expanded
  archive regression, strict focused Clippy and the real local Chrome and
  opusenc journey passed. See the [cancellation record](opus-preparation-2026-11-07.md#cancellation-and-retry-qualification-follow-up).
- The [next Checks run](https://github.com/AberrantWolf/junk-collector/actions/runs/42366266515)
  passed the Opus journey and failed in the mail browser fixture when two
  status writers shared a temporary filename. The fixture now serializes
  writes, including during teardown. The mail journey passed locally with
  overlapping reports, as did the eight later journeys that the failure
  prevented from running. Both superseded container validations were cancelled.

## Exact-source qualification and publication

- [Checks 42368079325](https://github.com/AberrantWolf/junk-collector/actions/runs/42368079325)
  passed 641 Rust tests, strict workspace Clippy, architecture, links, format,
  tooling, web types/build/models and all 31 browser journeys. Five existing
  Rust tests remained ignored. The beta selector chose full source coverage
  from beta.22. The redundant tag-push Checks run was cancelled.
- [Qualification 42368084755](https://github.com/AberrantWolf/junk-collector/actions/runs/42368084755)
  reused the exact-source receipt. Both native image runtime gates and AMD64
  release-image browser acceptance passed. The workflow sealed artifact
  `13573122996`, attempt 1.
- [Publication 42370032068](https://github.com/AberrantWolf/junk-collector/actions/runs/42370032068)
  published the sealed OCI archive and verified anonymous pulls on both
  platforms. Independent registry reads confirmed both manifests and their
  source/version labels. The release bundle and image record match
  `SHA256SUMS`, and the downloaded validation artifact and OCI archive match
  their recorded digests. `preview` points to beta.23 and stable `latest`
  remains absent.

| Object | SHA-256 |
| --- | --- |
| Multi-platform index | `eb40bffb1ffd423a71d1de5e0a78361de90cde0a1d337607c11a2d8f5327eb13` |
| Linux AMD64 manifest | `85645661c033f776682628d5a918461e6563f433490ff106cba79a5a317a172c` |
| Linux ARM64 manifest | `73f2ec37110ae96f7055c91242afd6f697951d8029ba404f4abb0126c5c989cb` |
| Sealed artifact ZIP | `d707376b0c18f28b22e84c7079130c587b86c608742392ac4a088d358c001c1f` |
| Sealed OCI archive | `0a4f4c0567eb17b07608755d2b67c01d4795c58532ee5812f257398ba594382f` |

## Fi recovery point and live acceptance

Fi pulled the exact published index and checked the native image's source and
version before downtime. All five production services stopped with zero
unfinished Jobs. The full Fi private-state snapshot and NAS archive-parent
snapshot are read-only. Both SQLite databases passed `quick_check` against
the stopped snapshot. Private recovery metadata retains the old image,
configuration, systemd and Compose files, and media authority key.

- Fi: `/var/backups/collector-fi-pairs/before-beta23-20261107-ed65d66/fi-data`.
- FatChocobo: `/volume1/collector-recovery/before-beta23-20261107-ed65d66`,
  snapshot UUID `553973a2-9d56-d140-ba7d-ebdd56053a3c`.

Core and media run the published beta.23 digest with zero restarts. Core is
healthy and Bridge is synced with its existing certificate pin. The cutoff and
live counts match for 123 Items, 6,811 Jobs, two users and two grants. No Jobs
were unfinished. The enabled receipt connection, cursor, one receipt
suggestion, mail profiles, configuration and TLS digests matched the cutoff.
Bridge, Caddy, tunnel and seven unrelated containers retained their identities.

Public and LAN HTTPS returned healthy responses and the original ServerId.
Nine served assets on each origin match the tagged source. Anonymous requests
to playback, export, audio administration, saved mail folder discovery and
audio preparation revision routes returned 401. The separate media endpoint
reports the same ServerId. Live acceptance covered health, identity, assets and
access denials. Authenticated mailbox discovery, encoding and playback were
exercised in isolated browser journeys.
