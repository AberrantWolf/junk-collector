#!/usr/bin/env bash
# Preserve a raw redumper CD package without generating playback/track copies.
set -euo pipefail
# Bash reads a script while running it, so editing or updating this file during
# a long rip could make the running copy execute mismatched pieces. The braces
# make bash read the whole script before starting; `exit` stops it reading on.
{

usage() {
    cat <<'EOF'
Usage: rip-cd.sh DESTINATION [DRIVE] NAME
       rip-cd.sh DESTINATION [DRIVE] --auto

NAME comes last so you can recall the previous command and change only the name.

NAME creates DESTINATION/NAME without a metadata lookup.
Example: rip-cd.sh /Volumes/Archive/incoming disk7 "Album - Disc 1"

--auto captures into a unique generated folder, then uses a catalog number only
when MusicBrainz returns one exact release, matching disc and usable number.
Naming waits at most 15 seconds after capture, never retries, and otherwise keeps
the generated name. Multi-disc releases include a Disc N suffix; collisions keep
the generated name. Naming performs no extra audio reads or drive commands.
Example: rip-cd.sh /Volumes/Archive/incoming disk7 --auto

DRIVE is optional; redumper otherwise selects the first drive with a disc.
The selected drive is reused explicitly for subsequent stages.
On Linux use /dev/sgN; on macOS use diskN (see diskutil list).
Environment: REDUMPER=/path/to/redumper, RETRIES=100 (default).
Automatic lookup uses Python 3 and collector-cd-identity from PATH or this repo's
target/release directory. Build once: cargo build --release --locked -p collector-cd-identity
COLLECTOR_CD_IDENTITY=/path/to/collector-cd-identity overrides that executable.
Missing naming tools keep the generated folder; ripping still works.
Never refines. If the dump reports SCSI or C2 errors, the last lines say to check
the disc and print a manual redumper refine command. Q-subchannel errors are not
reported there; see disc.log.
After successful capture and naming, ejects the selected drive using diskutil on
macOS and redumper on other platforms. macOS hides the disk while redumper holds
it, so the second stage and the eject wait up to MAC_DISK_WAIT seconds (default
30) for it to reappear.
Failed captures leave the disc inserted. Eject failures are reported separately.

Keeps raw SCRAM, STATE, SUBCODE, TOC and all additional disc sidecars/logs.
Does not generate BIN/CUE, skeletons, FLAC or other playback derivatives.
Existing destinations are refused. Failed/partial captures are kept for recovery.
Completion means the commands finished and required files exist, not that the
audio is error-free or catalog-verified. Review disc.log and verify in Collector.
EOF
}

die() { printf 'Error: %s\n' "$*" >&2; exit 1; }

if [[ ${1:-} == --help || ${1:-} == -h ]]; then
    usage
    exit 0
fi
if (( $# < 2 || $# > 3 )); then
    usage >&2
    exit 2
fi

automatic=0
destination=$1
drive=
(( $# == 3 )) && drive=$2
name=${!#}
if [[ $name == --auto ]]; then
    automatic=1
fi
redumper=${REDUMPER:-redumper}
retries=${RETRIES:-100}
# The argument order changed (NAME used to come first). These checks turn an
# old-style command into a clear error instead of folders in the wrong place.
order_hint='Arguments are DESTINATION [DRIVE] NAME (or --auto last); see --help.'
case "$name" in
    ''|.|..|*/*|*$'\n'*|*$'\r'*) die "NAME must be a single nonempty folder name. $order_hint" ;;
esac
# A forgotten NAME would otherwise rip from the first drive into a folder
# named after the drive.
if (( $# == 2 )) && [[ $name =~ ^(/dev/.+|r?disk[0-9]+)$ ]]; then
    die "NAME looks like a drive: $name. Put NAME (or --auto) after DRIVE. $order_hint"
fi
[[ -n $destination ]] || die 'DESTINATION must not be empty.'
[[ $destination == -* ]] && die "DESTINATION must not start with '-'. $order_hint"
if [[ -n $drive && ! $drive =~ ^(/dev/.+|r?disk[0-9]+)$ ]]; then
    die "DRIVE must be diskN on macOS or /dev/... on Linux, not: $drive. $order_hint"
fi
[[ $retries =~ ^[0-9]+$ ]] || die 'RETRIES must be a nonnegative integer.'
command -v "$redumper" >/dev/null 2>&1 || die "redumper executable not found: $redumper"

# Prefix relative paths so names starting with '-' cannot become mkdir options.
case "$destination" in /*) ;; *) destination="./$destination" ;; esac
mkdir -p "$destination"
destination=$(cd "$destination" && pwd -P)
if (( automatic )); then
    output=$(mktemp -d "$destination/cd-$(date -u +%Y%m%dT%H%M%SZ)-XXXXXXXX")
    # Match normal mkdir permissions so a shared inbox can read automatic dumps.
    chmod "$(printf '%03o' "$((0777 & ~$(umask)))")" "$output"
else
    output="$destination/$name"
    mkdir "$output" || die "Cannot create a fresh folder: $output"
fi

report_failure() {
    result=$?
    if (( result != 0 )); then
        printf '\nCapture workflow stopped at %s; files retained at: %s\nEarlier stages may have completed. Review disc.log before recovery/refinement.\n' "${stage:-setup}" "$output" >&2
    fi
}
trap report_failure EXIT

is_darwin=0
[[ $(uname -s) == Darwin ]] && is_darwin=1

# Map a macOS drive argument (disk7, /dev/disk7, /dev/rdisk7) to the whole-disk
# name diskutil understands. Prints nothing when it is not a whole disk.
mac_whole_disk() {
    local name=${1#/dev/}
    name=${name#r}
    [[ $name =~ ^disk[0-9]+$ ]] && printf '%s' "$name"
}

# While redumper holds a drive, macOS removes its /dev/diskN node, and only
# recreates it about a second after redumper exits. Commands that run sooner
# (diskutil eject, the next redumper stage) fail with "could not find disk".
# Wait up to MAC_DISK_WAIT seconds (default 30) for the node to come back.
wait_for_mac_disk() {
    local disk=$1 waited=0 limit=${MAC_DISK_WAIT:-30}
    until diskutil info "/dev/$disk" >/dev/null 2>&1; do
        (( waited >= limit * 2 )) && return 1
        sleep 0.5
        (( ++waited ))
    done
}

options=("--image-path=$output" --image-name=disc "--retries=$retries")
if [[ -n $drive ]]; then
    options+=("--drive=$drive")
fi

# Separate commands avoid the aggregate `disc` command's split/hash/info stages
# and its automatic refine. Keep every generated sidecar; never prune raw outputs.
for stage in dump dump::extra; do
    if (( is_darwin )) && [[ $stage != dump ]] && mac_disk=$(mac_whole_disk "$drive"); then
        wait_for_mac_disk "$mac_disk" ||
            die "Drive $drive did not reappear after the previous stage; supply DRIVE explicitly for recovery."
    fi
    printf '\nRunning redumper %s → %s\n' "$stage" "$output"
    "$redumper" "$stage" "${options[@]}"
    if [[ $stage == dump && -z $drive ]]; then
        # Reopening a drive can fail during autodetection after acquisition
        # (observed on macOS). Reuse the actual device from this fresh dump log;
        # never let a later stage silently choose a different inserted disc.
        drive=$(awk '/^drive information/ { in_drive = 1; next }
            in_drive && $1 == "path:" { print $2; exit }' "$output/disc.log")
        [[ -n $drive ]] || die 'Cannot determine the selected drive from disc.log; supply DRIVE explicitly for recovery.'
        options+=("--drive=$drive")
        printf 'Reusing selected drive: %s\n' "$drive"
    fi
done

# Otherwise a failure here would be reported as stopping at the last loop stage.
stage='required-file check'
for extension in scram state subcode toc log; do
    [[ -s "$output/disc.$extension" ]] || die "Missing or empty required output: disc.$extension"
done

# Never refine here. Refining a damaged disc can take hours and can leave the
# drive reading slowly until it is reconnected, so cleaning the disc or refining
# by hand is the owner's call. Only report what the dump found: "SCSI C2" counts,
# or "unknown" when the log doesn't hold exactly one well-formed summary.
# Q-subchannel errors are not counted; they don't affect the audio samples.
read_errors=$(awk '
    /^media errors:[[:space:]]*$/ { summaries++; in_summary = 1; next }
    in_summary && /^[[:space:]]*$/ { in_summary = 0 }
    in_summary && ($1 == "SCSI:" || $1 == "C2:") {
        if (NF != 3 || $2 !~ /^[0-9]+$/ || $3 != "samples") invalid = 1
        if ($1 == "SCSI:") { scsi++; scsi_count = $2 + 0 }
        else { c2++; c2_count = $2 + 0 }
    }
    END {
        if (summaries != 1 || scsi != 1 || c2 != 1 || invalid) print "unknown"
        else print scsi_count, c2_count
    }' "$output/disc.log")

if (( automatic )); then
    script_directory=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
    identity_binary=${COLLECTOR_CD_IDENTITY:-collector-cd-identity}
    if [[ -z ${COLLECTOR_CD_IDENTITY:-} ]] && ! command -v "$identity_binary" >/dev/null 2>&1; then
        identity_binary="$script_directory/../target/release/collector-cd-identity"
    fi
    if command -v python3 >/dev/null 2>&1 && command -v "$identity_binary" >/dev/null 2>&1; then
        stage=naming
        if named_output=$(python3 -B "$script_directory/name_cd_capture.py" "$output" "$identity_binary"); then
            output=$named_output
        else
            printf '\nAutomatic naming stopped; capture retained at: %s\n' "$output" >&2
        fi
    elif ! command -v python3 >/dev/null 2>&1; then
        printf '\nPython 3 is unavailable; keeping generated folder. See --help.\n' >&2
    else
        printf '\nIdentity executable unavailable: %s; keeping generated folder. See --help.\n' "$identity_binary" >&2
    fi
fi

printf '\nRaw capture saved to: %s\nReview disc.log for unresolved read errors. Import the entire folder into Collector using disc.toc as the analysis anchor.\n' "$output"

# Eject only the pinned drive, after every capture stage and required-file check.
printf '\nEjecting drive: %s\n' "$drive"
if (( is_darwin )); then
    # diskutil uses the block device even when redumper was given its raw alias.
    if eject_drive=$(mac_whole_disk "$drive"); then
        if ! wait_for_mac_disk "$eject_drive"; then
            printf 'Capture saved, but /dev/%s did not reappear after redumper finished; eject it manually.\n' "$eject_drive" >&2
        else
            # The reappeared disc is auto-mounted right away; an eject that
            # lands mid-mount can be refused as busy, so try a few times.
            for attempt in 1 2 3; do
                if eject_output=$(diskutil eject "/dev/$eject_drive" 2>&1); then
                    eject_status=0
                    break
                else
                    eject_status=$?
                fi
                (( attempt < 3 )) && sleep 2
            done
            if (( eject_status == 0 )); then
                printf '%s\nEject command completed for %s.\n' "$eject_output" "$drive"
            else
                printf '%s\nCapture saved, but diskutil eject failed for %s (exit %s).\n' "$eject_output" "$drive" "$eject_status" >&2
            fi
        fi
    else
        printf 'Capture saved, but cannot map macOS drive %s to a whole disk for eject.\n' "$drive" >&2
    fi
# Do not pass image options: eject must not open/append the preserved capture log.
# Some redumper versions return zero even when the SCSI eject command fails.
elif eject_output=$("$redumper" eject "--drive=$drive" 2>&1); then
    case "$eject_output" in
        *'failed to eject'*) printf '%s\nCapture saved, but the drive could not eject the disc.\n' "$eject_output" >&2 ;;
        *) printf '%s\nEject command completed for %s.\n' "$eject_output" "$drive" ;;
    esac
else
    eject_status=$?
    printf '%s\nCapture saved, but eject failed for %s (exit %s).\n' "$eject_output" "$drive" "$eject_status" >&2
fi

# Print this last so it is the final thing on screen.
if [[ $read_errors == unknown ]]; then
    printf '\nCHECK THIS DISC: could not read the SCSI/C2 error summary in disc.log.\nReview %s/disc.log before trusting this capture.\n' "$output" >&2
elif [[ $read_errors != '0 0' ]]; then
    read -r scsi_errors c2_errors <<<"$read_errors"
    printf '\nCHECK THIS DISC: the dump reported %s SCSI and %s C2 error samples. It was not refined.\n' "$scsi_errors" "$c2_errors" >&2
    printf 'Clean the disc and rip it again into a new folder, or refine this capture by hand:\n' >&2
    printf '  %q refine --image-path=%q --image-name=disc --retries=%s --drive=%q\n' "$redumper" "$output" "$retries" "$drive" >&2
else
    printf '\nThe dump reported no SCSI/C2 errors.\n'
fi
exit 0
}
