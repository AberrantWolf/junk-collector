# Routine beta updates on Fi

Use this procedure on the installed Fi/FatChocobo stack when only the Collector
and media image pins change. The direct upgrade floor is beta.22, catalog v15
and private schema v23. Use the extended qualification and
[paired recovery procedure](remote-archive.md#coordinated-recovery-points-and-rollback)
when release notes call for a conversion, durable work needs to be resumed, or
the update changes mounts, ingress, config, companion images or the service unit.
Run a targeted live workflow when the release affects that path.

Fi and the NAS have an existing Docker operator route for host administration.
Use SSH with the configured operator accounts. On each host, use an already
installed, trusted helper image pinned by its local image ID. The helper runs
as root in a short-lived privileged container and enters the host mount
namespace. Verify the route on both hosts before stopping writers.

```bash
# Run this function on each host with that host's trusted local image ID.
host_admin() {
  docker run --pull=never --rm -i --user 0:0 --privileged --pid=host \
    --network=none --entrypoint nsenter "$helper_image_id" \
    -t 1 -m -- "$@"
}
host_admin /usr/bin/id -u  # must return 0
```

Use the same wrapper on the NAS for the read-only snapshot. Run the interactive
checker from a terminal with `docker run -it` in the wrapper so it can prompt
for the session securely. For the checker's HTTPS probes, use the established
Fi helper with `--network=host` in place of `--network=none`. Keep the other
host administration commands on the isolated route.

For a separately requested tunnel update, verify the upstream release, pull its
exact AMD64 digest, and run its version and tunnel command checks before
downtime. Render the installed Compose configuration with the candidate pins
without replacing the live environment. Retain the old connector image and
environment in the same recovery point. During activation change the
`COLLECTOR_CONNECTOR_IMAGE` pin as well. Verify the new image, version, stable
container and public HTTPS through the tunnel after activation. The routine
checker expects companion containers to retain their identities. For this
extended update, compare the changed connector against its candidate pin and
compare the other companions against the baseline.

1. Read the target release notes. Run `sha256sum --check SHA256SUMS` with the
   downloaded release bundle and `release-image.json`. Pull the record's exact
   immutable `reference` on Fi with `docker pull --platform linux/amd64` before
   downtime. Keep the old image available
   locally by digest. Keep imports idle and pause the Collector receipt scan
   before recording the baseline if it was enabled. Bridge stays running and
   synced until the service stop.
2. Create a fresh private Fi pair directory with mode 0700. From the source
   checkout or unpacked release bundle, run the checker on Fi. Use the existing
   Compose project name and three trusted HTTPS origins. Pick one small,
   retained item that the session can read over the public origin. If the
   tunnel runs outside this Compose project, include it with `--companion`.

   ```bash
   # Use absolute host paths for source_dir, release_record, pair and lan_ca.
   check_args=(
     --project "$project" --release "$release_record" --pair-dir "$pair" \
     --public "$public_origin" --lan "$lan_origin" --media "$media_origin" \
     --item "$item_id"
   )
   # Add these only when applicable:
   # check_args+=(--companion "$tunnel_container")
   # check_args+=(--lan-ca "$retained_lan_certificate")
   host_admin /usr/bin/python3 -B "$source_dir/scripts/fi_update_check.py" before "${check_args[@]}"
   ```

   Use `--lan-ca` when Fi does not trust the retained LAN certificate through
   its system trust store. The checker prompts
   for the **value** of an existing `collector_session` browser cookie. Use the
   browser's cookie inspector for the selected public origin and a Viewer
   session with access to the sample item when available. The value stays out
   of arguments, the baseline and the output. The checker sends it only to the
   specified public HTTPS origin and refuses redirects. Keep that session
   available for the second check.
3. Run `host_admin systemctl stop junk-collector-remote.service` and verify all stack
   containers and every other writer on both hosts have stopped. Follow the
   [paired checkpoint steps](remote-archive.md#coordinated-recovery-points-and-rollback)
   while writers remain stopped. Sync the mounted archive and Fi local roots.
   Take the read-only Fi full-data snapshot at `$pair/fi-data` and the read-only
   NAS whole archive-parent snapshot under the same point ID. Retain the old
   image and exact env, Compose, systemd/mount configuration and media authority
   key. Check both snapshots cover their full roots and seal the pair record
   with both paths and snapshot IDs before editing the pins. An incomplete
   pair stops the update. With writers still stopped, pass the recorded NAS
   snapshot UUID to the cutoff check. It verifies the read-only Fi snapshot,
   checks both SQLite databases and requires idle Jobs and imports. It records
   the Fi snapshot UUID and fingerprints for the post-update comparison.

   ```bash
   host_admin /usr/bin/python3 -B "$source_dir/scripts/fi_update_check.py" cutoff "${check_args[@]}" \
     --nas-snapshot-uuid "$nas_snapshot_uuid"
   ```

   If the state directory has another name inside `fi-data`, supply
   `--state-in-snapshot RELATIVE_PATH` to this step. The NAS UUID comes from
   the operator's sealed pair record. Wait for the cutoff check to pass.
4. Edit only `COLLECTOR_IMAGE` and `COLLECTOR_MEDIA_IMAGE` in the installed
   EnvironmentFile to the verified immutable reference. Run
   `host_admin systemctl start junk-collector-remote.service` so the mount preflight runs.
   Keep normal traffic and receipt scanning quiet during acceptance. Run the
   post-update check with the same session.

   ```bash
   host_admin /usr/bin/python3 -B "$source_dir/scripts/fi_update_check.py" after "${check_args[@]}"
   ```

5. Record the pass beside the sealed pair, then resume ordinary use and restore
   the receipt scan setting if it changed. The checker compares the exact
   running image and its pinned reference, restart counts, mounts, retained
   companion identities, a fresh synced Bridge report, public/LAN health, all
   three ServerIds and the authenticated sample revision. It compares account
   and grant rows, mail connections, archive binding, TLS certificate, config,
   and Item, Job and Import counts with the stopped Fi snapshot. Staged imports
   and imports awaiting review keep their recorded identity and revision.
   A snapshot WAL is read using a temporary local database copy when needed.
   These checks read no media payloads. The after check waits up to two
   minutes for a fresh completed Bridge sync and reports a sync interruption
   separately from a timeout.

A failed check is an unresolved update. Preserve the pair and diagnose the
reported failure. If rollback is needed after new writes, restore the matching
Fi and NAS members and their old configuration and images together using
[paired recovery](remote-archive.md#coordinated-recovery-points-and-rollback).
