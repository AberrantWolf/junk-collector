# HL-05A — actual-host NAS operations, 2026-10-20

## Outcome and boundaries

The shipped install/update/restore path is accepted on isolated data on the actual
Synology DS1522+ (`fatchocobo.local`, x86-64 Linux, Btrfs `/volume1`). Python 3.8.15
and Compose v2.20.1-6047-g6817716 ran the checked-in qualification script. No live
archive/config/state, account, Portainer service, tunnel or share ACL was changed.
The existing production container remained running with its original start time
and image `sha256:4494180d637296938b090c33fc9cfae04a461184cda6231b81f19f487b14c3d5`.

The candidate was built as linux/amd64, transferred and loaded under a separate
qualification tag, immutable image ID
`sha256:ede1f8e1949d9100f71e2f8f9312d074df2ac8df67ccde5649ea5c33cc3256a9`.
This qualifies loaded-image operations; it does **not** qualify anonymous registry
pulls, moving channels or Portainer redeployment. HL-07D owns that publication path.
No automatic storage conversion runs in this delivery. Newer intake review can
write automatic group titles that older builds do not understand. Recover the complete
old point with its old image; do not infer image-only rollback safety after new writes.

## Fixes and self-review

- Create the exact DSM bind root before initialization **and** before restore.
- The fresh proxy-config helper requires initialized regular roots, refuses any
  archive/private-state contents or empty stdin, and atomically installs a private
  config file. Compare exact transferred bytes before setup. Setup normalizes the
  config and saves a previous-file backup; doctor checks actual origins afterward.
  The cause of the earlier unverified copy failure is not asserted as established.
- The optional inbox override uses an existing bind source, read-only mount and
  matching supplemental group for server/CLI. Host traversal/read ACLs are separate
  from mount writability. Importing does not clean up sources.
- Portainer/SSH guidance compares immutable local image IDs, retains groups/mounts
  and connector topology, and explains CLI-only orphan warnings without removing
  tunnels. Do not use standalone stack replacement/down on the live Portainer stack.
- Startup/doctor explain monitored provider contact and distinguish configuration
  from network availability or a matching release. No MusicBrainz account is needed.

Self-review caught an invalid pre-setup formatting comparison in the test harness,
a missing DSM restore bind root, and insufficient assertions for old sessions,
image identity and resumed offsets. These were corrected before the final runs.
The harness compares every original and requires a complete capture; it does not
accept a vacuous/partial inventory check. All cleanup is restricted to its generated
project/data/handoff files. Existing production sources were inspected read-only.

## Reproducible actual-host acceptance

Run `scripts/check_container.py CANDIDATE --update-from OLD_IMAGE --proxy --inbox
--data-parent WRITABLE_BTRFS_PARENT --port-base FREE_BASE` on the Docker host.
Use isolated generated data only. With `--provider-user-agent-file PRIVATE_FILE`,
the file supplies an existing monitored contact without printing it. The final
configured and unconfigured runs both passed:

1. Initialization reentry retained config. Exact stdin transfer passed; empty
   input, archive contents, unknown private-state data and established accounts
   prevented replacement without changing those files.
2. Setup created an Owner and TLS identity. UID 10001 with supplemental GID 65538
   could traverse/read fixture files owned by the NAS user/group without changing
   production share permissions. The fixture used standard CUE/BIN PCM and an
   unknown hidden note, not user originals.
3. A browser upload was stopped halfway through its first file. The old image
   backed up and validated complete roots before candidate recreation. The new
   running image ID was checked. The same Owner, original session cookie,
   ServerId, config bytes, TLS certificate and exact received offset survived.
   Remaining bytes resumed through normal APIs, producing one complete capture.
4. An inbox import saved byte-identical originals and explicitly prepared FLAC.
   Typed readiness reflected provider configuration. Without contact, automatic
   metadata failed visibly while originals and FLAC remained usable. With the real
   configured contact, a provider Job completed successfully. No matching release
   or automatic artwork is claimed for this synthetic TOC. Media range responses
   returned the requested bytes with HTTP 206.
5. Clean SIGTERM stops returned exit 0 in 0.22–0.31 seconds in the final runs.
   Full candidate backup/validation and empty-root restore retained original
   sessions/accounts/ServerId/TLS and all original bytes. Restored prepared audio
   matched its recorded digest. The old instance stayed stopped during restore.

The configured contact came from the existing configuration via a private temporary
file; no credentials were printed. There was no repeated blocked-provider retry.
Temporary stacks/data were removed after success.

## Actual remote browser before and after restore

Chrome 153.0.8010.37 on the Mac connected to the isolated NAS private HTTPS listener.
The harness first validated the actual certificate PEM, hostname and ServerId with
Node TLS verification, then used test-only self-signed acceptance in Chrome.
A known-private-IP resolver mapping was used after the initial mDNS request timed
out. Neither OS/browser trust installation nor DNS/discovery is claimed.

Both the updated instance and restored instance passed:

- Fresh UI sign-in, Play album, real audio decoding/time advancement and seek.
- Original downloads compared by length and SHA-256 for every fixture file.
- Individual-track Play and continued connected/unpaused player after closing details.
- Desktop and 390×844 mobile layout without horizontal overflow; no page errors.

Use `--browser-host NAME --private-ip IP --browser-handoff PRIVATE_JSON` on the
host, securely copy the generated fixture JSON to the browser machine, run
`node web/tests/container-host.mjs PRIVATE_JSON`, then write `passed` to the host's
`PRIVATE_JSON.done` only after success. Repeat for the `-restored.json` handoff.
Each handoff expires after ten minutes. The file contains generated test credentials;
keep it private and remove transferred copies afterward.

## Gates and retained limits

Architecture/link checks, Rust formatting, workspace tests, all-target Clippy with
warnings denied, web type check/build, rebuilt embedded server and full real-browser
smoke passed. Docker amd64 build, the two actual NAS profiles and before/after
restore browser journeys passed. Screenshots were retained in temporary browser
artifact directories; source tests and this record are the durable evidence.

This is a complete isolated actual-host operations delivery, not acceptance of the
permanent collection's final grouping/enrichment, public Cloudflare ingress,
system-trusted second-device/native behavior, storage faults or power loss.
HL-05B remains open: the host is Btrfs, but Snapshot Replication and Hyper Backup
were not installed, Collector's eligible shared-folder boundary was not established,
and no independent destination was selected. Full-copy backups used here do not
supply a scalable routine-update policy. Preserve earlier dated NAS observations.
