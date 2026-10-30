#!/bin/sh
set -eu
umask 077
mkdir -p "$GNUPGHOME" "$PASSWORD_STORE_DIR"
chmod 700 "$GNUPGHOME" "$PASSWORD_STORE_DIR"
# Never regenerate a missing private key over an existing password store.
if [ ! -f "$PASSWORD_STORE_DIR/.gpg-id" ]; then
    gpg --batch --pinentry-mode loopback --passphrase '' --quick-generate-key 'Collector Bridge local key' rsa3072 encr 0
    key=$(gpg --batch --with-colons --list-secret-keys | awk -F: '$1=="fpr" {print $10;exit}')
    test -n "$key"
    pass init "$key"
fi
# A pass round trip verifies unattended unlocking without printing the secret.
probe=$(head -c 32 /dev/urandom | base64)
printf '%s\n' "$probe" | pass insert --multiline --force collector-startup-check >/dev/null
[ "$(pass show collector-startup-check)" = "$probe" ]
pass rm --force collector-startup-check >/dev/null
unset probe
case "${1:-serve}" in
    cli) exec /usr/bin/protonmail-bridge --cli ;;
    serve) exec /usr/bin/protonmail-bridge --noninteractive ;;
    *) exec "$@" ;;
esac
