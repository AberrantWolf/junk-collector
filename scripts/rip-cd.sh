#!/usr/bin/env bash
# Preserve a raw redumper CD package without generating playback/track copies.
set -euo pipefail

usage() {
    cat <<'EOF'
Usage: rip-cd.sh NAME DESTINATION [DRIVE]

Creates DESTINATION/NAME and captures a CD there as disc.*.
Example: rip-cd.sh "Album - Disc 1" /Volumes/Archive/incoming disk7

DRIVE is optional; redumper otherwise selects the first drive with a disc.
The selected drive is reused explicitly for subsequent stages.
On Linux use /dev/sgN; on macOS use diskN (see diskutil list).
Environment: REDUMPER=/path/to/redumper, RETRIES=100 (default).

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

name=$1
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
output="$destination/$name"
mkdir "$output" || die "Cannot create a fresh folder: $output"

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

printf '\nRaw capture saved to: %s\nReview disc.log for unresolved read errors. Import the entire folder into Collector using disc.toc as the analysis anchor.\n' "$output"
