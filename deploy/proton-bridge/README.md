# Proton Bridge email intake

The optional purchase inbox reads receipts from Proton Mail through a local Proton
Bridge container. Collector images already include the mail connector; only Bridge
needs a local build. The Bridge image is Linux AMD64 only. Keep its credentials and
mailbox cache on private local storage.

## Bridge package

The Dockerfile installs a pinned Bridge release (`BRIDGE_VERSION`) and verifies the
DEB against a recorded SHA-256 and the vendored official signing key and policy.
No Proton credentials are bundled. To change versions, update the version and
digest together. References:

- [Proton CLI guide](https://proton.me/support/bridge-cli-guide)
- [Linux keychain requirements](https://proton.me/support/bridge-linux-login-error)
- [Package verification](https://proton.me/support/verifying-bridge-package)
- [Release manifest](https://proton.me/download/bridge/version_linux.json)

## Prepare

Install Collector first using the [container instructions](../container/README.md).
From the bundle's `proton-bridge` directory (`deploy/proton-bridge` in a source checkout):

```sh
docker build --platform linux/amd64 -t junk-collector-proton-bridge:3.18.0 .
```

Create private directories for the Bridge home and its status, owned by
10001:10001 with mode 0700 (apply matching ACLs on a NAS). Also create the profile
file Compose will bind: for setup through the web UI, a file containing `[]`,
owned by 10001:10001, mode 0600. (For [operator-file provisioning](#optional-operator-file-provisioning)
let the script create it instead.) Bridge may cache the whole mailbox, so size the
storage accordingly. Never enable IMAP/SMTP protocol logging or share Bridge logs.

Use `compose.yaml` with `compose.mail.yaml` and add to `.env`:

```text
PROTON_BRIDGE_IMAGE=junk-collector-proton-bridge:3.18.0
PROTON_BRIDGE_HOME=/srv/private-collector-mail/bridge-home
COLLECTOR_MAIL_PROFILES=/srv/private-collector-mail/profiles.json
PROTON_BRIDGE_STATUS=/srv/private-collector-mail/status
```

Collector gets only the sanitized status directory (read-only), never Bridge's
home. Web-managed profiles are stored in `/data/state/mail-profiles`.

Bridge and Collector both use host networking and talk over the host loopback;
IMAP and SMTP are not published. Anything else on the host is inside this trust
boundary. Do not publish the ports, add socat or router rules.

## Log in once

Run only one Bridge process against the home at a time:

```sh
docker compose -f compose.yaml -f compose.mail.yaml stop proton-bridge
docker compose -f compose.yaml -f compose.mail.yaml run --rm --no-deps proton-bridge mkdir -p /home/bridge/export
docker compose -f compose.yaml -f compose.mail.yaml run --rm --no-deps proton-bridge cli
```

At the Bridge prompt:

- `login`, and enter your password, 2FA and mailbox password (in a private terminal).
- `info` shows the **local IMAP** username and password (not your Proton password).
- `telemetry disable` and `updates autoupdates disable` (update by rebuilding the image).
- `cert export` to `/home/bridge/export`. Use only `cert.pem`; the exported
  private key is not needed.
- Confirm IMAP STARTTLS is on port 1143, then exit.

The entrypoint sets up GPG and `pass` with a passphrase-less key so Bridge can
restart unattended. Protection at rest therefore relies on disk encryption, file
permissions and encrypted backups: anyone with the complete Bridge home has access.

## Connect

Start Bridge and Collector. In **Purchase inbox**, the **Connect your email** card
shows Bridge status. Enter the local IMAP credentials and exact folder names and
choose **Connect Bridge**. Collector pins the certificate and checks folders
read-only before saving. New connections stay paused until you pick a collection
and enable receipt scanning.

INBOX alone does not include archived mail; add each archive folder by its exact
IMAP name, or use Bridge's All Mail folder. Connected profiles offer **Edit scanned
folders**, **Rescan configured folders** and **Revalidate Bridge connection**.
A changed Bridge certificate stops syncing until you revalidate. See
[email purchases](../../docs/email-purchases.md) for review behavior.

## Optional operator-file provisioning

Instead of the web flow, create a profile file before starting Collector. Get your
Collector `user_id` from `/api/v1/me`, then:

```sh
python3 scripts/configure_proton_mail.py \
  --output /srv/private-collector-mail/profiles.json \
  --certificate /srv/private-collector-mail/bridge-home/export/cert.pem \
  --owner YOUR_COLLECTOR_USER_ID --id proton-owner --folder INBOX
```

It prompts for the local IMAP credentials, creates the file with mode 0600 and
refuses to overwrite an existing file. Repeat `--folder` for more folders. Chown
the file to 10001:10001. Changes to this file need a Collector restart; to rotate,
write a new file and change the bind path.

## Portainer

Portainer does not build images, so build the Bridge image on the Docker host
first. Render the stack with `docker compose -f compose.yaml -f compose.mail.yaml config`
and deploy that. Keep passwords in the bind-mounted files, not in Portainer
environment variables.

## Checking it works

1. `dpkg-query -W protonmail-bridge` shows the expected version (Bridge has no `--version`).
2. Restart the Bridge container and confirm it stays logged in.
3. Confirm 1143/1025 listen only on loopback and are unreachable from the LAN.
4. Enable intake and check new and archived receipts appear, and unread flags are unchanged.
5. Accept, link and dismiss suggestions; confirm restarts do not duplicate them and
   that other accounts cannot see the inbox.

## Backup and rollback

Back up the Bridge home and any profile file separately from Collector's data. To
go back to a server version without the purchase inbox, restore a backup taken
before upgrading; do not open newer state with an older release.
