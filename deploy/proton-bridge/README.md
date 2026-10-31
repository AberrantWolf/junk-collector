# Official Proton Bridge with Collector email intake

## Current Fi installation — 2026-10-31

The merged implementation is installed on **Fi**, alongside Proton Bridge 3.18.0
in the existing `junk-collector` Compose stack. Collector private state, Bridge
home and the mail profile are on Fi's local SSD; the archive remains on FatChocobo
over NFS. See [the deployment and recovery record](../../docs/validation/fi-email-deployment-2026-10-31.md).
This is an exact-source local release-mode image, not a new published release.

Use your private terminal on the Mac to authenticate:

```sh
ssh -t fi-home /home/link/bin/collector-mail login
```

At the Bridge prompt, run `login` and complete its password/2FA prompts. Use `info`
to view the **local IMAP** credentials, then `exit`. Keep these credentials in your
private terminal. Telemetry and automatic updates were disabled during setup;
the helper stops only Bridge for CLI use and restarts its daemon on exit.
This follows the [official CLI workflow](https://proton.me/support/bridge-cli-guide).

In Collector's **Purchase inbox**, the visible **Connect your email** card shows
Bridge status, last-reported sync progress and estimated remaining time. Enter
the local IMAP username/password and exact folder names, then choose **Connect
Bridge**. The background reporter automatically exports the certificate inside
Bridge's private home and supplies its public fingerprint; no manual certificate
copy or owner ID is needed for this web flow. A failed credential/TLS/folder check
saves no profile. A successful connection stays paused until you select Games and
choose **Enable receipt scanning**. INBOX alone does not cover archived history.
Proton account passwords/2FA are never entered into this form.

Query progress without stopping Bridge or keeping the terminal open:

```sh
ssh fi-home /home/link/bin/collector-mail status
```

Progress may initially be unknown after a restart until Bridge emits a fresh
event. A missing/stale reporter is an explicit unavailable state. The status
snapshot contains no account names, credentials, raw logs or message identifiers.
It does not alter Bridge's account-wide encrypted cache.

For management access through SSH:

```sh
ssh -N -L 9842:127.0.0.1:9842 fi-home
```

Open [Purchase inbox](http://localhost:9842/purchase-inbox) and sign in. The
initial `[]` operator profile file deliberately supplies
no account before provisioning. Bridge IMAP/SMTP bind only `127.0.0.1:1143/1025`.
Both were inaccessible from the Mac over Fi's LAN address.

Fi's guarded systemd unit owns lifecycle, including Bridge. Mail data is under
`/srv/junk-collector/mail` (Bridge home, status directory and mode-0600 legacy
`profiles.json`). Web-managed profiles and their receipts live under
`/srv/junk-collector/state/mail-profiles`; both are inside the local data subvolume
for complete paired recovery points. The installed
operator helper is `/opt/junk-collector-beta14/scripts/fi-mail.py`; its wrapper is
`/home/link/bin/collector-mail`. Check state with its `status` command over `ssh -t`.
Account login, authenticated restart persistence and actual mailbox/read-only
history acceptance still need your local test. The original isolated build/setup
procedure below remains useful for another installation.

## Verified feasibility and remaining qualification

On 2026-10-30 Proton's official release manifest returned stable **3.18.0**,
commit `0800aeea50ee8a474e64355a4b7df124c0bcee91`. Its unchanged AMD64 DEB has
SHA-256 `4a4b4f3efe5502d2fc1280a2abd3d3bae5091275b466d1148c1b7df27cd571b7`.
The official signing key fingerprint is
`D51E64D3E63EDC3EEF7864CEE2C75D68E6234B07`. This build checks both digest and DEB
signature against the vendored official key/policy. These are build inputs, not
credentials. Base OS/package updates remain administrator-managed.

The release source implements `--cli` and `--noninteractive`; its Linux helper
selects `pass` when usable and its mail listener host is `127.0.0.1`. Official
package inspection and signature verification can be performed without logging
in. Running the CLI, credential persistence and IMAP sync on your NAS still need
the isolated acceptance steps below. Source inspection is not a successful NAS
login or Docker runtime test. No Proton account credentials are bundled.

Primary references:

- [Proton CLI guide](https://proton.me/support/bridge-cli-guide)
- [Linux keychain requirements](https://proton.me/support/bridge-linux-login-error)
- [Official package verification](https://proton.me/support/verifying-bridge-package)
- [Loopback and TLS design](https://proton.me/support/bridge-ssl-connection-issue)
- [Supported systems](https://proton.me/support/operating-systems-supported-bridge)
- [Pinned release source](https://github.com/ProtonMail/proton-bridge/tree/v3.18.0)
- [Release manifest](https://proton.me/download/bridge/version_linux.json)

## Prepare an isolated installation

Use a disposable candidate Collector installation and its own config/state/archive,
following the existing [container instructions](../container/README.md). The
candidate Collector image must be built from this branch's complete, checked
source/web assets; this branch adds Python 3 to that image. It is not available
under an existing published release tag.

On the AMD64 Docker host, build the candidate Collector image from the repository
root (the checked `web/dist` assets are already included in this branch):

```sh
docker build --platform linux/amd64 -t junk-collector-email:candidate .
```

Set `COLLECTOR_IMAGE=junk-collector-email:candidate` for the isolated stack. Build
the Bridge image from `deploy/proton-bridge`:


```sh
docker build --platform linux/amd64 -t junk-collector-proton-bridge:3.18.0 .
```

Create an encrypted/private NAS directory for `bridge-home` and a private mail
profile file. Give directories UID:GID 10001:10001 and mode 0700; restrict DSM ACLs
as well as Unix modes. Bridge can cache the entire mailbox, so size storage for the
account, not just purchase emails. Protect Bridge logs too; never enable IMAP/SMTP
protocol logging or attach logs containing credentials/mail to chat.

Use base `deploy/container/compose.yaml` plus `compose.mail.yaml`. Set the existing
Collector environment variables to the isolated installation, and additionally:

```text
PROTON_BRIDGE_IMAGE=junk-collector-proton-bridge:3.18.0
PROTON_BRIDGE_HOME=/volume1/private-collector-mail/bridge-home
COLLECTOR_MAIL_PROFILES=/volume1/private-collector-mail/profiles.json
PROTON_BRIDGE_STATUS=/volume1/private-collector-mail/status
```

Create the status directory as UID:GID 10001:10001 with mode 0700 too. The mail
override mounts only this sanitized directory read-only into Collector; it never
mounts Bridge's home into Collector. Its managed-profile directory defaults to
`/data/state/mail-profiles`, covered by the private-state bind.

The standard Collector stack uses host networking. This override does too, so
Bridge and Collector reach the same host loopback; neither IMAP nor SMTP is
published to LAN/WAN. Other host processes and administrators are inside this
trust boundary. Do not add socat, port publishing, host bind patches, router rules
or a public mail endpoint. If Collector uses a separate network namespace, both
processes must share that namespace instead; this override is specifically for
the documented host-network installation.

## Authenticate locally, once

First run only the Bridge service interactively; do not run two Bridge processes
against the same persistent home. From the base/override directory on the NAS:

```sh
docker compose -f compose.yaml -f compose.mail.yaml stop proton-bridge
docker compose -f compose.yaml -f compose.mail.yaml run --rm --no-deps proton-bridge mkdir -p /home/bridge/export
docker compose -f compose.yaml -f compose.mail.yaml run --rm --no-deps proton-bridge cli
```

At the official Bridge prompt, enter `help`, then `login` and follow its prompts
for your account password, 2FA and any separate mailbox password. Enter these only
in your private SSH terminal. Use `info` to view the **local IMAP** username/password;
these are distinct from your Proton account password. Disable optional usage
telemetry with `telemetry disable` and pin updates with
`updates autoupdates disable`. Update via a newly verified container image.

Run `cert export` and enter `/home/bridge/export` (create this directory first via
an operator container command if needed). Bridge exports `cert.pem` and `key.pem`.
Use only `cert.pem` for provisioning; keep/remove the exported private key locally
as appropriate. Confirm the CLI reports IMAP STARTTLS on 1143, changing the profile
port if you deliberately configured another port. Exit the CLI before starting
the daemon. Check the current release's `help` if command names differ.

The entrypoint initializes GPG + `pass`, checks an encrypt/decrypt round trip, and
persists the complete Bridge home. Its GPG key has no passphrase so it can restart
unattended. Protection at rest therefore comes from NAS encryption, file/ACL
permissions and encrypted backups; possession of the complete home grants access.
Do not describe a co-located key and encrypted store as independent disk protection.

## Provision the connector without chat credentials

Get your Collector `user_id` from its authenticated `/api/v1/me` response. Run on
the NAS (Python 3 required):

```sh
python3 scripts/configure_proton_mail.py \
  --output /volume1/private-collector-mail/profiles.json \
  --certificate /volume1/private-collector-mail/bridge-home/export/cert.pem \
  --owner YOUR_COLLECTOR_USER_ID --id proton-owner --folder INBOX
```

The script prompts with hidden input for the local Bridge IMAP credentials and
creates a new file exclusively with mode 0600. It does not print them. Assign file
ownership to 10001:10001 so Collector can read it. To include archived history,
repeat `--folder` with each exact IMAP folder name, or configure Bridge's All Mail
visibility and its exact folder name. INBOX alone does not include archived mail.
No folder name is guessed, and no folders are modified.

The profile pins Bridge's exported certificate digest. TLS encryption is required
before LOGIN; a changed pin stops syncing instead of accepting a replacement.
Re-export/re-provision locally after an intentional certificate reset. Profile
changes to the operator file require a Collector restart. Web profile creation
loads immediately. The provisioning script refuses to overwrite
an existing file; for rotation, create a new private file and update the bind
mount path before restarting. Do not use your Proton account password here.

For Portainer, merge the base and override with `docker compose ... config` after
setting the non-secret path variables, then paste/upload the rendered stack in an
isolated stack. Inspect paths and networking before deploying. Compose resolves
files on the Docker host, not on your Mac. Avoid Portainer environment variables
for passwords; only private bind-mounted files contain them. Images must already
exist on the selected Docker engine; Portainer stack deployment does not build
the local Dockerfile. Do not redeploy production with this candidate.

Start Bridge and Collector, then open Collector's private management address,
choose **Purchase inbox**, select Games, and enable **My Proton Mail**. The first
poll begins a resumable history import. Review a few messages from each retailer
locally, especially cancellations, collector editions and multi-item orders.

## Isolated acceptance before production

1. Verify the package version with `dpkg-query -W protonmail-bridge`, CLI startup
   and the `pass` round trip. Bridge 3.18.0 does not support `--version`.
2. Authenticate, exit, restart the Bridge daemon, then restart its container again.
   Confirm the account remains logged in without a password prompt.
3. Confirm the host has only loopback listeners for 1143/1025 and another LAN
   machine cannot connect to either. Never broaden binds to make a failed test pass.
4. Enable intake and verify new and historical suggestions, including archived
   folders. Check the mailbox's unread flags before/after extraction.
5. Accept one new game and link another receipt to it; dismiss a non-game purchase.
   Apply an explicit cancellation update. Verify retry/restart does not duplicate
   accepted records and that Viewer/another account cannot read the inbox.
6. Pause during a sync, restart, re-enable and confirm safe resumption. Change the
   configured certificate digest deliberately in a disposable profile and confirm
   no login/sync proceeds until the correct pin is restored.
7. Back up the stopped candidate's collection/private state plus external Bridge
   home/profile, restore to isolated storage, and confirm orders and cursor resume.

Rollback stops/removes only the candidate containers and restores its complete
paired backup if needed. No production migration or release publication is part
of this branch. The new private schema is 22; use a pre-feature paired backup to
return to an older server. Do not open newer state with an older release.
