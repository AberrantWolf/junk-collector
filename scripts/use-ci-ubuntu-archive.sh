#!/usr/bin/env bash
# Keep the runner's suites and Ubuntu archive key. Replace only the AMD64 mirror.
set -euo pipefail
if [[ "$(dpkg --print-architecture)" != amd64 ]]; then
  exit 0
fi

sources=/etc/apt/sources.list.d/ubuntu.sources
grep -q '^URIs:' "$sources"
for archive in https://archive.ubuntu.com/ubuntu https://mirrors.kernel.org/ubuntu; do
  if curl --fail --silent --show-error --head --location --max-time 12 "$archive/dists/noble/InRelease" >/dev/null &&
     curl --fail --silent --show-error --head --location --max-time 12 "$archive/dists/noble-security/InRelease" >/dev/null; then
    sudo sed -i "s|^URIs:.*|URIs: $archive/|" "$sources"
    exit 0
  fi
done
echo "No reachable Ubuntu 24.04 archive for this runner." >&2
exit 1
