#!/bin/sh
# Run once as root using the existing initialize service and its /data mount.
set -eu
root=${1:-/data}
for name in media-authority media-http media-certificates media-proxy-config; do
    path="$root/$name"
    if [ -L "$path" ] || { [ -e "$path" ] && [ ! -d "$path" ]; }; then
        echo "Refusing non-directory or symlink: $path" >&2
        exit 1
    fi
    install -d -m 0750 -o 10001 -g 10001 "$path"
done
echo 'Media socket/certificate directories initialized. Existing archive/state permissions were not changed.'
