#!/usr/bin/env bash
# Preserve a raw redumper CD package without generating playback/track copies.
set -euo pipefail

usage() {
    cat <<'EOF'
Usage: rip-cd.sh --auto DESTINATION [DRIVE]
       rip-cd.sh NAME DESTINATION [DRIVE]

--auto captures into a unique generated folder, then uses a catalog number only
when MusicBrainz returns one exact release, matching disc and usable number.
Naming waits at most 15 seconds after capture, never retries, and otherwise keeps
the generated name. Multi-disc releases include a Disc N suffix; collisions keep
the generated name. No extra audio reads or drive commands are performed.
Example: rip-cd.sh --auto /Volumes/Archive/incoming disk7

Supplying NAME instead creates DESTINATION/NAME without a metadata lookup.
Example: rip-cd.sh "Album - Disc 1" /Volumes/Archive/incoming disk7

DRIVE is optional; redumper otherwise selects the first drive with a disc.
The selected drive is reused explicitly for subsequent stages.
On Linux use /dev/sgN; on macOS use diskN (see diskutil list).
Environment: REDUMPER=/path/to/redumper, RETRIES=100 (default).
Automatic lookup uses Python 3 and collector-cd-identity from PATH or this repo's
target/release directory. Build once: cargo build --release --locked -p collector-cd-identity
COLLECTOR_CD_IDENTITY=/path/to/collector-cd-identity overrides that executable.
Missing naming tools keep the generated folder; ripping still works.
Refinement runs only when the initial dump reports SCSI or C2 errors.
Q-subchannel errors alone do not trigger the normal SCSI/C2 refinement pass.

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
name=$1
if [[ $name == --auto ]]; then
    automatic=1
fi
destination=$2
drive=${3:-}
redumper=${REDUMPER:-redumper}
retries=${RETRIES:-100}
case "$name" in
    ''|.|..|*/*|*$'\n'*|*$'\r'*) die 'NAME must be a single nonempty folder name.' ;;
esac
[[ -n $destination ]] || die 'DESTINATION must not be empty.'
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

options=("--image-path=$output" --image-name=disc "--retries=$retries")
if [[ -n $drive ]]; then
    options+=("--drive=$drive")
fi

# Separate acquisition/refinement commands avoid the aggregate `disc` command's
# split/hash/info stages. Keep every generated sidecar; never prune raw outputs.
for stage in dump dump::extra refine; do
    if [[ $stage == refine ]]; then
        # Match normal redumper refinement eligibility. Q errors alone require
        # --refine-subchannel, which this helper deliberately does not enable.
        # Refuse unknown summaries rather than silently skipping needed repair
        # or invoking refinement unnecessarily on drive firmware sensitive to it.
        needs_refinement=$(awk '
            /^media errors:[[:space:]]*$/ { summaries++; in_summary = 1; next }
            in_summary && /^[[:space:]]*$/ { in_summary = 0 }
            in_summary && ($1 == "SCSI:" || $1 == "C2:") {
                if (NF != 3 || $2 !~ /^[0-9]+$/ || $3 != "samples") invalid = 1
                if ($1 == "SCSI:") scsi++
                else c2++
                if ($2 !~ /^0+$/) errors = 1
            }
            END {
                if (summaries != 1 || scsi != 1 || c2 != 1 || invalid) exit 2
                print errors ? 1 : 0
            }' "$output/disc.log") || die 'Cannot read initial SCSI/C2 error summary; refinement was not started. Review disc.log.'
        if [[ $needs_refinement == 0 ]]; then
            printf '\nSkipping refinement: initial dump reported no SCSI/C2 errors.\n'
            continue
        fi
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

for extension in scram state subcode toc log; do
    [[ -s "$output/disc.$extension" ]] || die "Missing or empty required output: disc.$extension"
done

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
    else
        printf '\nAutomatic naming tools unavailable; keeping generated folder. See --help.\n' >&2
    fi
fi

printf '\nRaw capture saved to: %s\nReview disc.log for unresolved read errors. Import the entire folder into Collector using disc.toc as the analysis anchor.\n' "$output"
