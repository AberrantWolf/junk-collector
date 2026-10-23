#!/bin/sh
# One-shot initialization, run as root; the server itself always runs as UID 10001.
set -eu
umask 077
root=${1:-/data}
for name in config archive state backups; do
    path="$root/$name"
    if [ -L "$path" ] || { [ -e "$path" ] && [ ! -d "$path" ]; }; then
        echo "Refusing non-directory or symlink: $path" >&2
        exit 1
    fi
done
config="$root/config/collector.toml"
if [ -L "$config" ] || { [ -e "$config" ] && [ ! -f "$config" ]; }; then
    echo "Refusing non-regular configuration: $config" >&2
    exit 1
fi
if [ ! -f "$config" ]; then
    : "${COLLECTOR_HOST:?Set COLLECTOR_HOST to the stable private browser hostname or IPv4 address}"
    : "${COLLECTOR_PRIVATE_IP:?Set COLLECTOR_PRIVATE_IP to the host private IPv4 address}"
    case "$COLLECTOR_HOST" in
        *[!a-zA-Z0-9.-]*|'') echo 'COLLECTOR_HOST must be a plain hostname or IPv4 address' >&2; exit 1 ;;
    esac
    case "$COLLECTOR_PRIVATE_IP" in
        *[!0-9.]*|'') echo 'COLLECTOR_PRIVATE_IP must be an IPv4 address' >&2; exit 1 ;;
    esac
fi
# Never recursively chown an archive or overwrite existing operator configuration.
install -d -m 0700 -o 10001 -g 10001 "$root"
for name in config archive state backups; do
    install -d -m 0700 -o 10001 -g 10001 "$root/$name"
done
if [ -f "$config" ]; then
    echo 'Existing configuration retained. Run setup only while the server is stopped.'
    exit 0
fi
# Publish the initial file without replacing a configuration created concurrently.
temporary=$(mktemp "$root/config/.initialize.XXXXXX")
trap 'rm -f "$temporary"' EXIT HUP INT TERM
cat >"$temporary" <<EOF
archive = "/data/archive"
state = "/data/state"
public_bind = "127.0.0.1:9841"
private_bind = "127.0.0.1:9842"
lan_bind = "$COLLECTOR_PRIVATE_IP:9843"
private_tls_bind = "$COLLECTOR_PRIVATE_IP:9844"
tls_names = ["localhost", "127.0.0.1", "$COLLECTOR_HOST", "$COLLECTOR_PRIVATE_IP"]
allowed_origins = ["http://localhost:9842", "https://$COLLECTOR_HOST:9844", "https://$COLLECTOR_HOST:9843"]
authentication_origin = "https://$COLLECTOR_HOST:9844"
endpoints = ["https://$COLLECTOR_HOST:9843"]

[exposure]
public_curation = false
public_administration = false

[encoders]
flac = "/usr/bin/flac"
opus = "/usr/bin/opusenc"
EOF
chown 10001:10001 "$temporary"
ln "$temporary" "$config"
echo 'Initial configuration created. Run the setup service to create your owner account.'
