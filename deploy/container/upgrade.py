#!/usr/bin/python3 -I
"""Upgrade a Junk Collector container install to a published release.

Install as /usr/local/sbin/junk-collector-upgrade (owned by root, mode 0755) and
run as root, normally through a single sudoers rule:

  junk-collector-upgrade check v0.1.0-beta.37     # verify and print the plan; no downtime
  junk-collector-upgrade upgrade v0.1.0-beta.37   # stop, snapshot, switch images, start

Settings come from /etc/junk-collector/upgrade.toml (see upgrade.toml.example).
The script only changes the image pins in the service's settings file. It never
edits Compose files, the systemd unit or collector config: when a release changes
deployment files it refuses until you have applied them by hand and pass
--deploy-files-updated. It never restores a snapshot; a failed upgrade stops and
prints the recovery steps instead, because the new version may already have
converted data the old version can't read.
"""
import argparse
import contextlib
import datetime
import fcntl
import json
import os
import re
import shutil
import signal
import stat
import subprocess
import sys
import tempfile
import time
import tomllib
import urllib.error
import urllib.request
from pathlib import Path

CONFIG_PATH = Path("/etc/junk-collector/upgrade.toml")
LOCK_PATH = Path("/run/junk-collector-upgrade.lock")
LOG_PATH = Path("/var/log/junk-collector-upgrade.log")
SAFE_PATH = "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
TAG = re.compile(r"v\d+\.\d+\.\d+(?:-[0-9A-Za-z]+(?:\.[0-9A-Za-z]+)*)?", re.ASCII)
SHA = re.compile(r"[0-9a-f]{40}")
# Local state the recovery point must hold (see remote.env.example).
STATE_KEYS = ("COLLECTOR_DATA", "COLLECTOR_CONFIG", "COLLECTOR_STATE", "COLLECTOR_BACKUPS",
              "PROTON_BRIDGE_HOME", "PROTON_BRIDGE_STATUS", "COLLECTOR_MAIL_PROFILES")
# Signals that would kill or pause the script while the server is down.
HELD_SIGNALS = (signal.SIGINT, signal.SIGQUIT, signal.SIGTSTP, signal.SIGTERM, signal.SIGHUP)
IMAGE_KEYS = ("COLLECTOR_IMAGE", "COLLECTOR_MEDIA_IMAGE")
GUIDE = "deploy/container/remote-archive.md (Recovery points and Routine updates)"

DEFAULTS = {
    "service": "junk-collector-remote.service",
    "env_file": "/etc/junk-collector/remote.env",
    "compose_project": "junk-collector",
    "image": "ghcr.io/aberrantwolf/junk-collector",
    "github_repository": "AberrantWolf/junk-collector",
    "snapper_config": "junk-collector",
    "media_key_volume": "junk-collector_media-authority",
    "health_timeout_seconds": 1800,
    "nas_snapshot_command": [],
}


class Refused(Exception):
    """A check failed before anything was changed."""


class Restarted(Exception):
    """A step failed after the service stopped; the old version was started again."""


class Stranded(Exception):
    """The server may be down or on the new image; a person has to look."""


def say(message=""):
    """Print and log. Output must not stop the upgrade if the SSH session drops."""
    with contextlib.suppress(OSError):
        print(message, flush=True)
    line = f"{datetime.datetime.now().isoformat(timespec='seconds')} {message}\n".encode()
    with contextlib.suppress(OSError):
        descriptor = os.open(LOG_PATH, os.O_WRONLY | os.O_APPEND | os.O_CREAT | os.O_NOFOLLOW, 0o600)
        try:
            os.write(descriptor, line)
        finally:
            os.close(descriptor)


class Host:
    """Everything that touches the machine or the network; tests replace it."""

    def run(self, *argv, check=True):
        result = subprocess.run(argv, capture_output=True, text=True,
                                env={"PATH": SAFE_PATH, "LC_ALL": "C.UTF-8"})
        if check and result.returncode != 0:
            detail = (result.stderr or result.stdout).strip()
            raise RuntimeError(f"{' '.join(argv)} failed ({result.returncode}): {detail}")
        return result.stdout

    def http_json(self, url):
        request = urllib.request.Request(url, headers={
            "Accept": "application/vnd.github+json",
            "User-Agent": "junk-collector-upgrade",
        })
        try:
            with urllib.request.urlopen(request, timeout=30) as response:
                return json.load(response)
        except urllib.error.HTTPError as error:
            if error.code == 404:
                raise Refused(f"GitHub has no {url.split('/repos/', 1)[-1]}") from None
            raise Refused(f"GitHub answered {error.code} for {url}") from None
        except (urllib.error.URLError, TimeoutError, ValueError) as error:
            raise Refused(f"Couldn't read {url}: {error}") from None

    def monotonic(self):
        return time.monotonic()

    def sleep(self, seconds):
        time.sleep(seconds)


def load_config(text):
    raw = tomllib.loads(text)
    unknown = sorted(set(raw) - set(DEFAULTS))
    if unknown:
        raise Refused(f"Unknown setting(s) in {CONFIG_PATH}: {', '.join(unknown)}")
    config = {**DEFAULTS, **raw}
    for key, default in DEFAULTS.items():
        if type(config[key]) is not type(default):
            raise Refused(f"{key} in {CONFIG_PATH} must be a {type(default).__name__}")
    if not config["health_timeout_seconds"] > 0:
        raise Refused("health_timeout_seconds must be positive")
    if not Path(config["env_file"]).is_absolute():
        raise Refused("env_file must be an absolute path")
    if not re.fullmatch(r"[a-z0-9][a-z0-9._/:-]*", config["image"]) or "@" in config["image"]:
        raise Refused("image must be a lowercase repository name without a tag or digest")
    if not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", config["github_repository"]):
        raise Refused("github_repository must look like owner/name")
    command = config["nas_snapshot_command"]
    if command and (not all(isinstance(part, str) for part in command) or not Path(command[0]).is_absolute()):
        raise Refused("nas_snapshot_command must be a list of strings starting with an absolute path")
    return config


def require_private(path, owner=0, top=Path("/")):
    """Refuse a file that someone other than root could change, or that sits in such a folder."""
    path = Path(path)
    parents = list(path.parents)
    for part in [path, *parents[:parents.index(top) + 1 if top in parents else None]]:
        info = part.lstat()
        if stat.S_ISLNK(info.st_mode):
            raise Refused(f"{part} is a symbolic link; use the real path")
        if info.st_uid != owner:
            raise Refused(f"{part} must be owned by root")
        if info.st_mode & 0o022:
            raise Refused(f"{part} must not be writable by group or others")


def rewrite_images(text, image, new_ref):
    """Point COLLECTOR_IMAGE (and COLLECTOR_MEDIA_IMAGE, if set) at new_ref."""
    found, lines = {}, []
    for line in text.splitlines(keepends=True):
        match = re.fullmatch(r"(COLLECTOR_IMAGE|COLLECTOR_MEDIA_IMAGE)=(.*?)(\r?\n)?", line)
        if not match and re.match(r"\s*(export\s+)?COLLECTOR_(MEDIA_)?IMAGE\s*=", line):
            raise Refused(f"Can't safely rewrite this settings line; write it as KEY=value: {line.strip()}")
        if match:
            key, value, newline = match.group(1), match.group(2), match.group(3) or ""
            if key in found:
                raise Refused(f"{key} is set more than once in the settings file")
            if not re.fullmatch(re.escape(image) + r"(@sha256:[0-9a-f]{64}|:[A-Za-z0-9_.-]+)", value):
                raise Refused(f"{key} is {value!r}, not an image from {image}; update it by hand")
            found[key] = value
            line = f"{key}={new_ref}{newline}"
        lines.append(line)
    if "COLLECTOR_IMAGE" not in found:
        raise Refused("COLLECTOR_IMAGE isn't set in the settings file")
    if found.get("COLLECTOR_MEDIA_IMAGE", found["COLLECTOR_IMAGE"]) != found["COLLECTOR_IMAGE"]:
        raise Refused("COLLECTOR_MEDIA_IMAGE differs from COLLECTOR_IMAGE; make them match by hand first")
    return "".join(lines), found["COLLECTOR_IMAGE"]


def image_info(host, ref):
    try:
        return json.loads(host.run("docker", "image", "inspect", ref))[0]
    except (RuntimeError, ValueError, IndexError):
        raise Refused(f"Docker has no image {ref}") from None


def release_commit(host, repo, tag):
    api = f"https://api.github.com/repos/{repo}"
    release = host.http_json(f"{api}/releases/tags/{tag}")
    if release.get("draft"):
        raise Refused(f"{tag} is still a draft release")
    target = host.http_json(f"{api}/git/ref/tags/{tag}").get("object", {})
    for _ in range(5):
        if target.get("type") != "tag":
            break
        target = host.http_json(f"{api}/git/tags/{target.get('sha')}").get("object", {})
    if target.get("type") != "commit" or not SHA.fullmatch(str(target.get("sha"))):
        raise Refused(f"Couldn't find the commit for {tag}")
    return target["sha"]


def deploy_changes(host, repo, old_commit, new_tag):
    """Deployment files that changed between the running release and new_tag."""
    comparison = host.http_json(f"https://api.github.com/repos/{repo}/compare/{old_commit}...{new_tag}")
    status = comparison.get("status")
    if status == "identical":
        raise Refused(f"{new_tag} is the version already running")
    if status != "ahead":
        raise Refused(f"{new_tag} is not newer than the running version ({status}); "
                      "going back means restoring a recovery point")
    files = comparison.get("files")
    if not isinstance(files, list) or len(files) >= 300:
        return None  # GitHub truncates large comparisons; can't tell.
    changed = set()
    for entry in files:
        for name in (entry.get("filename"), entry.get("previous_filename")):
            if name and name.startswith("deploy/container/") and not name.endswith(".md"):
                changed.add(name)
    return sorted(changed)


def env_values(text):
    values = {}
    for line in text.splitlines():
        match = re.fullmatch(r"([A-Z_][A-Z0-9_]*)=(.*)", line)
        if match:
            values[match.group(1)] = match.group(2)
    return values


def snapper_subvolume(host, config):
    configs = json.loads(host.run("snapper", "--jsonout", "list-configs")).get("configs", [])
    subvolume = next((c.get("subvolume") for c in configs if c.get("config") == config["snapper_config"]), None)
    if not subvolume:
        raise Refused(f"snapper has no config named {config['snapper_config']}")
    return Path(subvolume)


def require_covered(subvolume, env):
    """Refuse unless the snapshot would hold every local state folder that is set."""
    device = subvolume.stat().st_dev
    for key in STATE_KEYS:
        value = env.get(key)
        if not value:
            continue
        path = Path(value)
        if not path.is_absolute() or not path.resolve().is_relative_to(subvolume.resolve()):
            raise Refused(f"{key}={value} is outside the snapshot subvolume {subvolume}")
        existing = path
        while not existing.exists():
            existing = existing.parent
        # A different device means another subvolume or mount, which the snapshot skips.
        if existing.stat().st_dev != device:
            raise Refused(f"{key}={value} is on a different subvolume or mount than {subvolume}, "
                          "so the snapshot wouldn't include it")


def plan(host, config, tag, deploy_files_updated=False):
    """Every check that needs no downtime. Returns what upgrade() needs."""
    try:
        return checked_plan(host, config, tag, deploy_files_updated)
    except Refused:
        raise
    except Exception as error:
        raise Refused(f"{type(error).__name__}: {error}") from None


def checked_plan(host, config, tag, deploy_files_updated):
    if not TAG.fullmatch(tag):
        raise Refused(f"{tag!r} is not a release tag like v0.1.0-beta.37")
    env_file = Path(config["env_file"])
    try:
        env_text = env_file.read_text(encoding="utf-8")
    except OSError as error:
        raise Refused(f"Couldn't read {env_file}: {error}") from None
    _, current_ref = rewrite_images(env_text, config["image"], config["image"] + "@sha256:" + "0" * 64)
    current = image_info(host, current_ref)
    labels = current.get("Config", {}).get("Labels") or {}
    old_version = labels.get("org.opencontainers.image.version", "")
    old_commit = labels.get("org.opencontainers.image.revision", "")
    if not TAG.fullmatch(old_version) or not SHA.fullmatch(old_commit):
        raise Refused(f"The running image {current_ref} has no release labels; upgrade it by hand")
    running = project_containers(host, config["compose_project"], "collector")
    if not running:
        raise Refused(f"No collector container of Compose project {config['compose_project']} is running; "
                      f"start {config['service']} first, or check compose_project")
    if json.loads(host.run("docker", "inspect", running[0]))[0].get("Image") != current.get("Id"):
        raise Refused(f"The running collector isn't {current_ref}; is {config['env_file']} the file the service uses?")
    require_covered(snapper_subvolume(host, config), env_values(env_text))

    commit = release_commit(host, config["github_repository"], tag)
    changed = deploy_changes(host, config["github_repository"], old_commit, tag)
    if changed is None and not deploy_files_updated:
        raise Refused(f"GitHub listed too many changes to check deployment files between {old_version} "
                      f"and {tag}. Compare deploy/container by hand, then pass --deploy-files-updated.")
    if changed and not deploy_files_updated:
        raise Refused("This release changes deployment files:\n  " + "\n  ".join(changed) +
                      "\nApply the changes your install uses (see the release notes), "
                      "then run again with --deploy-files-updated.")

    say(f"Pulling {config['image']}:{tag} (the server keeps running)…")
    host.run("docker", "pull", "--quiet", f"{config['image']}:{tag}")
    new = image_info(host, f"{config['image']}:{tag}")
    new_labels = new.get("Config", {}).get("Labels") or {}
    if new_labels.get("org.opencontainers.image.revision") != commit:
        raise Refused(f"The pulled image was built from {new_labels.get('org.opencontainers.image.revision')}, "
                      f"not {tag}'s commit {commit}")
    if new_labels.get("org.opencontainers.image.version") != tag:
        raise Refused(f"The pulled image says it is {new_labels.get('org.opencontainers.image.version')}, not {tag}")
    digests = sorted(d for d in new.get("RepoDigests") or [] if d.startswith(config["image"] + "@sha256:"))
    if not digests:
        raise Refused(f"Docker reported no digest for {config['image']}:{tag}")
    new_ref = digests[0]
    if image_info(host, new_ref).get("Id") != new.get("Id"):
        raise Refused(f"{new_ref} isn't the image just pulled")
    new_env, _ = rewrite_images(env_text, config["image"], new_ref)
    return {"tag": tag, "old_version": old_version, "old_ref": current_ref, "old_id": current["Id"], "new_ref": new_ref,
            "new_id": new["Id"], "env_text": env_text, "new_env": new_env, "deploy_changes": changed or []}


def project_containers(host, project, service=None):
    argv = ["docker", "ps", "--quiet", "--filter", f"label=com.docker.compose.project={project}"]
    if service:
        argv += ["--filter", f"label=com.docker.compose.service={service}"]
    return host.run(*argv).split()


def snapshot_dir(host, config, number):
    path = snapper_subvolume(host, config) / ".snapshots" / str(number)
    if not (path / "snapshot").is_dir():
        raise RuntimeError(f"snapshot {number} isn't at {path}")
    return path


def write_private(path, text, like):
    """Replace path atomically, keeping the owner and mode of `like`."""
    info = like.stat()
    descriptor, temporary = tempfile.mkstemp(dir=path.parent, prefix=f".{path.name}.")
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8") as handle:
            handle.write(text)
            handle.flush()
            os.fsync(handle.fileno())
        os.chmod(temporary, stat.S_IMODE(info.st_mode))
        os.chown(temporary, info.st_uid, info.st_gid)
        os.replace(temporary, path)
    except BaseException:
        with contextlib.suppress(OSError):
            os.unlink(temporary)
        raise
    directory = os.open(path.parent, os.O_DIRECTORY)
    try:
        os.fsync(directory)
    finally:
        os.close(directory)


def cleanup_command(config, steps):
    """Delete a recovery point: its extra copies first, so snapper can remove the folder."""
    extras = " ".join(str(steps["private"] / name) for name in steps.get("copies", []))
    delete = f"snapper -c {config['snapper_config']} delete {steps['snapshot']}"
    return f"rm -r -- {extras} && {delete}" if extras else delete


def recovery_steps(config, steps):
    env_file = Path(config["env_file"])
    lines = []
    if "snapshot" in steps:
        lines.append(f"Recovery point: snapper config {config['snapper_config']}, snapshot {steps['snapshot']} "
                     f"({steps['private']}).")
    if "backup" in steps:
        lines.append(f"The previous settings are in {steps['backup']}.")
        lines.append(f"If the new version never ran, going back is: systemctl stop {config['service']}; "
                     f"cp -p {steps['backup']} {env_file}; systemctl start {config['service']}.")
    lines.append(f"If it did run, it may have converted data: restore the recovery point as {GUIDE} describes.")
    return "\n".join(lines)


def wait_healthy(host, config, image_id):
    """True once the collector container runs image_id and reports healthy."""
    deadline = host.monotonic() + config["health_timeout_seconds"]
    while True:
        ids = project_containers(host, config["compose_project"], "collector")
        if ids:
            container = json.loads(host.run("docker", "inspect", ids[0]))[0]
            health = (container.get("State", {}).get("Health") or {}).get("Status")
            if container.get("Image") == image_id and health == "healthy":
                return True
        if host.monotonic() >= deadline:
            return False
        host.sleep(5)


def upgrade(host, config, prepared):
    steps = {}
    # Stopping or pausing the script between stop and start would leave the server down.
    held = {number: signal.signal(number, signal.SIG_IGN) for number in HELD_SIGNALS}
    try:
        return switch(host, config, prepared, steps, held)
    finally:
        for number, handler in held.items():
            signal.signal(number, handler)


def restart_old(host, config, prepared, steps, error):
    service = config["service"]
    say(f"Stopped before switching images: {error}")
    say(f"Starting {prepared['old_version']} again…")
    kept = f" Snapshot {steps['snapshot']} was kept; delete it with: {cleanup_command(config, steps)}" if "snapshot" in steps else ""
    try:
        host.run("systemctl", "start", service)
        healthy = wait_healthy(host, config, prepared["old_id"])
    except Exception as start_error:
        say(f"Starting it failed: {start_error}")
        healthy = False
    if not healthy:
        raise Stranded(f"{error}\nTHE SERVER IS DOWN: {prepared['old_version']} didn't start again "
                       f"(settings unchanged). Check `systemctl status {service}`.{kept}")
    raise Restarted(f"{error}\n{prepared['old_version']} is running again; nothing was switched.{kept}")


def switch(host, config, prepared, steps, held):
    service, env_file = config["service"], Path(config["env_file"])
    say(f"Stopping {service} ({prepared['old_version']})…")
    try:
        host.run("systemctl", "stop", service)
        left = project_containers(host, config["compose_project"])
        if left:
            raise RuntimeError(f"containers still running after stop: {' '.join(left)}")
        if config["nas_snapshot_command"]:
            say("Taking the NAS snapshot…")
            host.run(*config["nas_snapshot_command"], f"before {prepared['tag']}")
        say("Taking the local snapshot…")
        number = int(host.run("snapper", "-c", config["snapper_config"], "create", "--description",
                              f"before {prepared['tag']}", "--userdata", "junk-collector-upgrade=yes",
                              "--print-number").strip())
        steps["snapshot"] = number
        steps["private"] = private = snapshot_dir(host, config, number)
        steps["copies"] = []
        if config["media_key_volume"]:
            volume = host.run("docker", "volume", "inspect", "--format", "{{.Mountpoint}}",
                              config["media_key_volume"]).strip()
            steps["copies"].append("media-authority")
            host.run("cp", "-a", "--", volume, str(private / "media-authority"))
        steps["copies"].append(env_file.name)
        shutil.copy2(env_file, private / env_file.name)
        os.chmod(private / env_file.name, 0o600)
        backup = env_file.with_name(f"{env_file.name}.{prepared['old_version']}")
        if backup.exists():
            backup = env_file.with_name(f"{backup.name}.{number}")
        shutil.copy2(env_file, backup)
        steps["backup"] = backup
        if env_file.read_text(encoding="utf-8") != prepared["env_text"]:
            raise RuntimeError(f"{env_file} changed during the upgrade")
        write_private(env_file, prepared["new_env"], env_file)  # Atomic: old or new, never half.
    except Exception as error:
        restart_old(host, config, prepared, steps, error)

    try:
        say(f"Switched {env_file} to {prepared['new_ref']}.")
        say(f"Starting {prepared['tag']}…")
        host.run("systemctl", "start", service)
        signal.signal(signal.SIGINT, held[signal.SIGINT])  # Waiting can be interrupted.
        if not wait_healthy(host, config, prepared["new_id"]):
            raise RuntimeError(f"the server wasn't healthy after {config['health_timeout_seconds']} seconds "
                               f"(a large catalog rebuild can take longer; check `systemctl status {service}`)")
    except (Exception, KeyboardInterrupt) as error:
        raise Stranded(f"{error or 'Interrupted'}\n{recovery_steps(config, steps)}") from None
    return steps


def main(argv=None, host=None):
    parser = argparse.ArgumentParser(prog="junk-collector-upgrade", description=__doc__.split("\n\n")[0])
    parser.add_argument("action", choices=["check", "upgrade"])
    parser.add_argument("tag")
    parser.add_argument("--deploy-files-updated", action="store_true",
                        help="the release's deployment file changes have been applied by hand")
    args = parser.parse_args(argv)
    host = host or Host()
    try:
        if os.geteuid() != 0:
            raise Refused("Run this as root, for example: sudo junk-collector-upgrade check <tag>")
        require_private(Path(__file__).resolve())
        require_private(CONFIG_PATH)
        try:
            config = load_config(CONFIG_PATH.read_text(encoding="utf-8"))
        except (OSError, UnicodeDecodeError, tomllib.TOMLDecodeError) as error:
            raise Refused(f"Couldn't read {CONFIG_PATH}: {error}") from None
        require_private(config["env_file"])
        if config["nas_snapshot_command"]:
            require_private(config["nas_snapshot_command"][0])
        with open(LOCK_PATH, "w") as lock:
            try:
                fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
            except BlockingIOError:
                raise Refused("Another upgrade is already running") from None
            say(f"junk-collector-upgrade {args.action} {args.tag}")
            prepared = plan(host, config, args.tag, args.deploy_files_updated)
            say(f"Running: {prepared['old_version']} ({prepared['old_ref']})")
            say(f"New:     {prepared['tag']} ({prepared['new_ref']})")
            if prepared["deploy_changes"]:
                say("Deployment files changed (you said they're applied): " + ", ".join(prepared["deploy_changes"]))
            if not config["nas_snapshot_command"]:
                say("No nas_snapshot_command is set: take and lock a NAS snapshot yourself, or the "
                    "recovery point covers only local state.")
            if args.action == "check":
                say("Checks passed. Nothing was stopped or changed.")
                return 0
            steps = upgrade(host, config, prepared)
            say(f"{prepared['tag']} is running and healthy.")
            say("Sign in, play a track and compare the Library counts. Once it has proven itself, delete the "
                f"recovery point with: {cleanup_command(config, steps)}")
            return 0
    except Refused as error:
        say(f"Refused, nothing changed: {error}")
        return 2
    except Restarted as error:
        say(f"Upgrade stopped: {error}")
        return 3
    except Stranded as error:
        say(f"UPGRADE NEEDS ATTENTION: {error}")
        return 4
    except Exception as error:
        say(f"UNEXPECTED ERROR, check `systemctl status` and the log at {LOG_PATH}: {type(error).__name__}: {error}")
        return 5


if __name__ == "__main__":
    os.environ.clear()
    os.environ.update({"PATH": SAFE_PATH, "LC_ALL": "C.UTF-8"})
    os.umask(0o077)
    # Losing the SSH session must not stop an upgrade halfway.
    signal.signal(signal.SIGHUP, signal.SIG_IGN)
    sys.exit(main())
