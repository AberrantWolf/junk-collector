# HL-05B — Btrfs checkpoints and incremental recovery, 2026-10-20

## Outcome

An isolated actual-host procedure is qualified: stop Collector briefly, capture its
whole Btrfs data subvolume read-only, return the service to availability, and export
GNU tar baseline/increments from immutable points. Transfer the complete chain to
independent storage and recover all archive/config/private state into empty roots.
This integrates existing filesystem/tar tools; Collector's accepted sealed full
backup/restore CLI and portable archive format are unchanged.

The [operator procedure](../../deploy/container/snapshot-recovery.md), narrow
`freeze-tar-time.py` adapter and disposable qualification mode are implemented.
Permanent adoption/destination/scheduling are **not** configured. The independent
destination used for acceptance was a temporary private directory on this Mac's
local disk, physically separate from the NAS. No operator reply selected a permanent
destination. Public registry/pull publication remains HL-07D.

## Host eligibility and failed alternative

Read-only inspection of the actual DS1522+ confirms `/volume1/junk-collector` is
Btrfs subvolume `@syno/junk-collector`, ID 1396, with no nested subvolumes. Its
config/archive/state are ordinary children on the same device. Production image,
start time, live roots, Portainer services and tunnels were unchanged. Snapshot
Replication and Hyper Backup were absent. No DSM package or host filesystem remount
was performed. Kernel: Linux 4.4.302+; Compose v2.20.1-6047-g6817716; Python 3.8.15.

Disposable qualification tools used Debian btrfs-progs 6.2 and GNU tar 1.34.
Both generic and DSM-native (v4.0) `btrfs receive` failed the incremental receive
through the isolated bind mount with the default-subvolume/root-mount error. The
base receive worked; that is not successful incremental recovery. All generated
subvolumes were removed after these experiments. Raw-device mounting, host remounts
and production changes were not used to bypass the failure. This route remains
unqualified; the accepted path uses Btrfs for local snapshots and GNU tar for exports.

## Actual NAS → Mac → NAS recovery drill

`check_container.py`'s snapshot mode reused normal Owner/login/import/media APIs
and the shipped Compose/config/inbox workflow from HL-05A. It generated its own
Btrfs subvolume, fixture credentials, 32 MiB opaque unknown-data payload, CUE/BIN
originals and partial browser upload. The two actual application images were:

- Before update: `sha256:4494180d637296938b090c33fc9cfae04a461184cda6231b81f19f487b14c3d5`.
- Candidate: `sha256:ede1f8e1949d9100f71e2f8f9312d074df2ac8df67ccde5649ea5c33cc3256a9`.

No automatic conversion runs for this delivery. Older builds may not understand
new automatic-group-title writes from HL-07A; these tests always restore the whole
older point with its older image, never assume image-only rollback is safe.

Acceptance passed:

1. The before-update cold snapshot took **0.67 s**, with no payload copy/hash.
   Candidate availability returned before exporting the baseline. The baseline
   GNU tar archive was **34,508,800 bytes**, including the opaque payload.
2. SIGKILL/restart of the updated service retained the partial upload's exact
   received offset and original authenticated session. A writable clone of the
   complete old cold point, with the old image, retained the same account,
   ServerId/session and pending offset. Only one identity clone ran at a time.
   The candidate's original roots then reopened and completed the upload normally.
3. Inbox originals/unknown note were downloaded byte-for-byte. Explicit FLAC
   preparation and HTTP 206 ranges worked. Absent provider configuration remained
   an honest optional metadata failure, not simulated identification/artwork success.
4. The second cold snapshot took **1.03 s**. The service reopened before export.
   The increment was **3,276,800 bytes** (9.5% of the baseline), and its member list
   excluded the unchanged opaque 32 MiB payload. It includes legitimately changed
   private state and newly saved originals. This is mechanism/fixture evidence,
   not a production 1,000-album performance measurement.
5. The full baseline/increment and their `.snar` metadata were copied over SSH to
   a mode-0700 private directory on the Mac, fsynced, compared by length/SHA-256,
   then returned over SSH into a separate NAS fixture directory. Returned digests
   matched. Source export files were removed before the restore consumed the
   returned files; same-NAS snapshot access was not a restore dependency.
6. Ordered GNU incremental extraction into empty roots recovered all files.
   A separate cold extraction comparison matched **every** archive/config/private
   path, file SHA-256, type, Unix mode, UID/GID and symlink target to the immutable
   after-update point, including private-spool deletions and unknown data. This
   full validation belongs to restore acceptance, not routine snapshot capture.
7. The recovered server retained original account/session/ServerId/TLS, exact
   originals and the prepared FLAC's digest. Chrome 153.0.8010.37 on the Mac passed
   actual NAS login, album/track Play, seek, original length/SHA-256 downloads,
   persistent playback and mobile layout both before backup and after restore.
   As in HL-05A, Node independently checked certificate PEM/name/identity; Chrome
   used test-only self-signed acceptance and a known-IP DNS mapping. System-browser
   trust and DNS discovery were not qualified.
8. All generated services, fixture roots and snapshots were removed. Production
   remained running on its original image/start time afterward.

## Self-review repairs and refusal guarantees

A tar export starts **after** filesystem snapshot capture. Using tar's later clock
as its next cutoff could lose an existing file changed between capture and export.
The adapter rewinds only the two documented format-2 cutoff fields to the time
recorded just before snapshot creation, retaining all directory records. The drill
overwrote an existing file with equal-length bytes in that gap; its `after!` bytes
survived the final independent restore. A new file alone would not defeat this
shortcut. See [GNU tar's snapshot format](https://www.gnu.org/software/tar/manual/html_node/Snapshot-Files.html).

The helper refuses unsupported/malformed headers, invalid or later cutoffs,
published filenames, symlinks/hardlinks, nonregular/oversized input and concurrent
file replacement. Reads have an allocation bound; correction uses a private atomic
working replacement. Four tests passed on both macOS and NAS Python 3.8, covering
preserved unknown record bytes/private mode, non-mutating refusals, aliases and
oversized input. CI runs them. The source runbook requires a fresh baseline after
clock rollback/unreliable timestamps; timestamp-based incremental selection is not
an integrity check.

Self-review also added complete cold file-set/permission comparison instead of
checking only visible originals, retained the old whole-point rollback rather than
image downgrade, and creates recovered outer-root/backups permissions before doctor.
Unsuccessful/incomplete exports retain the accepted prior `.snar`/backup point;
working directories are published only as complete pairs with digests/parent/image
notes. The qualification helper is explicitly restricted to its generated fixture;
it is not a second production backup API.

## Reproduction and remaining limits

Build disposable tools with `docker build -f deploy/container/Dockerfile.qualification
-t YOUR_TOOLS .`. On the NAS, run the ordinary checker with two distinct loaded
images, `--proxy --inbox --data-parent WRITABLE_BTRFS_PARENT`, plus
`--snapshot-tool-image YOUR_TOOLS --snapshot-handoff PRIVATE_JSON`.
The private handoff lists the four files and return directory. Copy/verify them on
another physical system, return their checked bytes, then write `passed` to its
`.done` marker. Optional `--browser-host/--private-ip/--browser-handoff` add the two
real-browser pauses described in HL-05A. Never point this fixture at production.

Architecture/link checks, helper tests on both hosts and the actual NAS workflow,
independent roundtrip/cold comparison and Chrome journeys passed. Rust/web source
is unchanged from HL-05A's passing workspace/Clippy/fmt/type/build/full smoke gates.
No automatic backup schedule, production destination, destructive restore, public
pull/redeploy, storage-fault or power-loss certification is claimed. Local snapshots
remain rollback points, not independent backups. Periodic independent full integrity
validation and operator retention of complete parent chains remain required.
