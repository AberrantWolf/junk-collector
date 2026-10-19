#!/bin/sh
# Fresh-install configuration transport only. Account setup and validation stay in the CLI.
set -eu
umask 077
root=${1:-/data}
config="$root/config/collector.toml"
for name in config archive state; do
    if [ ! -d "$root/$name" ] || [ -L "$root/$name" ]; then
        echo 'Initialize regular data directories before installing a proxy configuration.' >&2
        exit 1
    fi
done
if [ -L "$config" ] || { [ -e "$config" ] && [ ! -f "$config" ]; }; then
    echo 'Initialize the data directory before installing a proxy configuration.' >&2
    exit 1
fi
existing=$(find "$root/archive" "$root/state" -mindepth 1 -maxdepth 1 -print -quit)
if [ -n "$existing" ]; then
    echo 'Archive or private state already exists. Stop the server and deliberately edit the retained configuration; do not replace it with a fresh-install example.' >&2
    exit 1
fi
temporary=$(mktemp "$root/config/.proxy-config.XXXXXX")
trap 'rm -f "$temporary"' EXIT HUP INT TERM
cat >"$temporary"
if [ ! -s "$temporary" ]; then
    echo 'No configuration received on standard input. Existing configuration was not overwritten.' >&2
    exit 1
fi
mv "$temporary" "$config"
echo 'Proxy configuration saved. Run setup and doctor before starting the server.'
