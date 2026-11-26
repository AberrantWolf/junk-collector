#!/usr/bin/env bash
# Prints a folder for test scratch data that lives in memory, for TMPDIR.
# On macOS every durable write the store makes is a full flush to the physical
# disk, so the store and browser tests spend most of their time waiting on it.
# A RAM disk makes those flushes cheap without changing what the code does.
#
#   TMPDIR="$(scripts/test-tmpdir.sh)" cargo test --workspace
#   TMPDIR="$(scripts/test-tmpdir.sh)" npm --prefix web test
#
# macOS: creates an APFS RAM disk once (it lasts until restart or eject;
# COLLECTOR_TEST_RAMDISK_MB sets its size, default 4096). Its memory is used as
# data is written and freed when you eject it:
#   diskutil eject JunkCollectorTests
# Linux: uses /dev/shm when it is writable. Elsewhere: prints the usual TMPDIR.
set -euo pipefail
name=JunkCollectorTests
case "$(uname -s)" in
Darwin)
 if [[ ! -d "/Volumes/$name" ]]; then
  megabytes=${COLLECTOR_TEST_RAMDISK_MB:-4096}
  device=$(hdiutil attach -nomount "ram://$((megabytes * 2048))" | awk '{print $1}')
  diskutil erasevolume APFS "$name" "$device" >/dev/null
 fi
 echo "/Volumes/$name/"
 ;;
Linux)
 if [[ -w /dev/shm ]]; then
  mkdir -p /dev/shm/junk-collector-tests
  echo /dev/shm/junk-collector-tests/
 else
  echo "${TMPDIR:-/tmp}"
 fi
 ;;
*) echo "${TMPDIR:-/tmp}" ;;
esac
