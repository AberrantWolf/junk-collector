#!/usr/bin/env python3
"""Plan/check the existing container-release workflow; publication stays in CI."""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import tomllib


VERSION = re.compile(r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?\Z")
DIGEST = re.compile(r"sha256:[0-9a-f]{64}\Z")


def version_key(version):
    match = VERSION.fullmatch(version)
    if not match or len(version) > 120:
        raise ValueError("Use a Docker-tag-compatible semantic version without build metadata")
    identifiers = (match[4] or "").split(".") if match[4] else []
    if any(value.isdigit() and len(value) > 1 and value.startswith("0") for value in identifiers):
        raise ValueError("Numeric prerelease identifiers cannot have leading zeroes")
    return (*map(int, match.group(1, 2, 3)), not bool(identifiers),
            tuple((0, int(value)) if value.isdigit() else (1, value) for value in identifiers))


def plan(repository, revision, version, event, release):
    key = version_key(version)
    if not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repository) or not re.fullmatch(r"[0-9a-f]{40}", revision):
        raise ValueError("Invalid repository or full source revision")
    if event not in ("release", "workflow_dispatch"):
        raise ValueError("Only release publication or a nonpublishing manual build is supported")
    tag, channel = "dry-run", None
    if event == "release":
        tag = release.get("tag_name")
        if tag != "v" + version or release.get("draft") is not False:
            raise ValueError("Published release tag must match the workspace version")
        if type(release.get("prerelease")) is not bool or release["prerelease"] != (not key[3]):
            raise ValueError("GitHub prerelease status must agree with the version")
        channel = "preview" if release["prerelease"] else "latest"
    image = "ghcr.io/" + repository.lower()
    return dict(image=image, reference=image + ":" + tag, tag=tag, channel=channel,
                version=version, revision=revision, source="https://github.com/" + repository)


def platforms(index):
    if index.get("schemaVersion") != 2 or index.get("mediaType") not in (
            "application/vnd.oci.image.index.v1+json", "application/vnd.docker.distribution.manifest.list.v2+json"):
        raise ValueError("Expected a multi-platform index")
    found, attestation_sources = {}, []
    for descriptor in index.get("manifests", []):
        digest = descriptor.get("digest", "")
        if not DIGEST.fullmatch(digest):
            raise ValueError("Invalid platform/attestation digest")
        platform = descriptor.get("platform", {})
        if (platform.get("os"), platform.get("architecture")) == ("unknown", "unknown") and descriptor.get("annotations", {}).get("vnd.docker.reference.type") == "attestation-manifest":
            attestation_sources.append(descriptor.get("annotations", {}).get("vnd.docker.reference.digest"))
            continue
        name = platform.get("os", "") + "/" + platform.get("architecture", "")
        if name not in ("linux/amd64", "linux/arm64") or name in found:
            raise ValueError("Unsupported or duplicate image platform")
        if platform.get("variant") not in (None, "", "v8" if name == "linux/arm64" else ""):
            raise ValueError("Unsupported image platform variant")
        found[name] = digest
    if set(found) != {"linux/amd64", "linux/arm64"}:
        raise ValueError("Both amd64 and arm64 images are required")
    if any(reference not in found.values() for reference in attestation_sources):
        raise ValueError("Attestation refers to an unknown platform manifest")
    return found


def check_labels(image, expected, platform):
    if image.get("Os", "") + "/" + image.get("Architecture", "") != platform:
        raise ValueError("Pulled image platform disagrees with its descriptor")
    labels = image.get("Labels", {})
    for name in ("version", "revision", "source"):
        if labels.get("org.opencontainers.image." + name) != expected[name]:
            raise ValueError("Image " + name + " label disagrees with the release")


def promote_current(expected, pages):
    if not expected["channel"]:
        return False
    candidates = []
    for page in pages:
        for release in page:
            if release.get("draft"):
                continue
            tag = release.get("tag_name", "")
            if not tag.startswith("v"):
                continue
            try:
                key = version_key(tag[1:])
            except ValueError:
                continue
            if type(release.get("prerelease")) is not bool or release["prerelease"] != (not key[3]):
                raise ValueError("Published release has inconsistent prerelease status")
            if ("preview" if release["prerelease"] else "latest") == expected["channel"]:
                candidates.append((key, tag))
    if expected["tag"] not in [tag for _, tag in candidates]:
        raise ValueError("Current release is missing from the complete release listing")
    return version_key(expected["version"]) == max(key for key, _ in candidates)


def command(*arguments):
    return subprocess.check_output(arguments, text=True).strip()


def verify(expected, source, digest, output):
    if not DIGEST.fullmatch(digest):
        raise ValueError("Missing build-output digest")
    public = source.startswith("docker://")
    if public and source != "docker://" + expected["reference"]:
        raise ValueError("Published source must be the planned version tag")
    if not public and not source.startswith("oci-archive:"):
        raise ValueError("Dry runs verify a local OCI archive")
    with tempfile.TemporaryDirectory(prefix="collector-release-check-") as temporary:
        root = Path(temporary)
        raw = root / "index.json"
        with raw.open("w") as target:
            subprocess.run(["skopeo", "inspect", "--raw", "--no-creds", source], stdout=target, check=True)
        if command("skopeo", "manifest-digest", str(raw)) != digest:
            raise ValueError("Version tag/export differs from the build-output digest")
        children = platforms(json.loads(raw.read_text()))
        destination = "oci:" + str(root / "verified") + ":release"
        # Freeze the already-checked public tag before downloading any mutable reference.
        frozen = "docker://" + expected["image"] + "@" + digest if public else source
        subprocess.run(["skopeo", "copy", "--all", "--preserve-digests", "--src-no-creds", frozen, destination], check=True)
        copied = root / "copied-index.json"
        with copied.open("w") as target:
            subprocess.run(["skopeo", "inspect", "--raw", destination], stdout=target, check=True)
        if command("skopeo", "manifest-digest", str(copied)) != digest:
            raise ValueError("Downloaded image bytes do not retain the checked index digest")
        for platform in children:
            architecture = platform.split("/")[1]
            image = json.loads(command("skopeo", "--override-os", "linux", "--override-arch", architecture, "inspect", destination))
            check_labels(image, expected, platform)
        if public:
            # Empty credentials, without logging out the operator or changing saved Docker settings.
            auth = root / "docker"
            auth.mkdir()
            (auth / "config.json").write_text('{"auths":{}}')
            endpoint = os.environ.get("DOCKER_HOST") or command("docker", "context", "inspect", "--format", "{{.Endpoints.docker.Host}}")
            for platform in children:
                subprocess.run(["docker", "--config", str(auth), "--host", endpoint, "pull", "--platform", platform, expected["image"] + "@" + digest], check=True)
        record = dict(expected, digest=digest, reference=expected["image"] + "@" + digest,
                      platforms=children, anonymous_docker_pulls=public)
        output.write_text(json.dumps(record, indent=2) + "\n")


def bundle(root, expected, record, destination):
    import shutil
    if expected["channel"] and record.get("anonymous_docker_pulls") is not True:
        raise ValueError("A published bundle requires verified anonymous Docker pulls")
    if any(record.get(name) != expected[name] for name in ("version", "revision", "source")):
        raise ValueError("Bundle evidence disagrees with the release")
    if not DIGEST.fullmatch(record.get("digest", "")) or record.get("reference") != expected["image"] + "@" + record["digest"]:
        raise ValueError("Bundle requires the exact checked manifest digest")
    destination.mkdir(parents=True, exist_ok=False)
    names = ("compose.yaml", "compose.inbox.yaml", "compose.media.yaml", "Caddyfile.media", "media.md", "initialize-media.sh", ".env.example", "collector.proxy.toml.example",
             "README.md", "recovery.md", "snapshot-recovery.md", "freeze-tar-time.py",
             "compose.remote.yaml", "remote.env.example", "remote-preflight.py",
             "junk-collector-remote.service", "remote-archive.md")
    for name in names:
        shutil.copyfile(root / "deploy/container" / name, destination / name)
    pinned = record["reference"] if expected["channel"] else "junk-collector:local"
    env = destination / ".env.example"
    env.write_text(env.read_text().replace("COLLECTOR_IMAGE=junk-collector:local", "COLLECTOR_IMAGE=" + pinned))
    for name in (name for name in names if name.endswith(".md")):
        path = destination / name
        text = path.read_text()
        if name == "README.md" and expected["channel"]:
            start, end = "<!-- release-image:start -->", "<!-- release-image:end -->"
            if text.count(start) != 1 or text.count(end) != 1:
                raise ValueError("Installation image instructions are missing their release boundary")
            before, rest = text.split(start)
            _, after = rest.split(end)
            published = (f"- **Image:** `{expected['reference']}` (linux/amd64 and linux/arm64).\n"
                         f"- **Pinned install:** `.env.example` uses `{pinned}`.\n"
                         "- **Evidence:** `release-image.json` records source and platform digests.\n"
                         f"- **Moving channel:** `{expected['image']}:{expected['channel']}` may point to a newer release.\n"
                         "- **Before updating:** read that release's recovery notes and keep the previous digest.\n")
            text = before + published + after
            text = text.replace("Registry publication has not yet\nbeen exercised.",
                                "This bundle's image passed anonymous pulls and both-platform metadata checks;\nactual Portainer pull/redeploy acceptance is a separate deployment check.")
        text = text.replace("../../docs/", expected["source"] + "/blob/" + expected["revision"] + "/docs/")
        path.write_text(text)
    (destination / "release-image.json").write_text(json.dumps(record, indent=2) + "\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    subcommands = parser.add_subparsers(dest="operation", required=True)
    subcommands.add_parser("plan")
    check = subcommands.add_parser("verify")
    check.add_argument("source")
    check.add_argument("digest")
    check.add_argument("--output", type=Path, default=Path("release-image.json"))
    subcommands.add_parser("promote")
    package = subcommands.add_parser("bundle")
    package.add_argument("destination", type=Path)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    if args.operation == "plan":
        version = tomllib.loads((root / "Cargo.toml").read_text())["workspace"]["package"]["version"]
        event = json.loads(Path(os.environ["GITHUB_EVENT_PATH"]).read_text())
        result = plan(os.environ["GITHUB_REPOSITORY"], os.environ["GITHUB_SHA"], version,
                      os.environ["GITHUB_EVENT_NAME"], event.get("release", {}))
        Path("release-plan.json").write_text(json.dumps(result, indent=2) + "\n")
        with open(os.environ["GITHUB_OUTPUT"], "a") as output:
            for key in ("reference", "version", "revision", "source", "channel"):
                output.write(key + "=" + (result[key] or "") + "\n")
        return
    expected = json.loads(Path("release-plan.json").read_text())
    if args.operation == "verify":
        verify(expected, args.source, args.digest, args.output)
    elif args.operation == "promote":
        pages = json.loads(Path("release-pages.json").read_text())
        with open(os.environ["GITHUB_OUTPUT"], "a") as output:
            output.write("promote=" + str(promote_current(expected, pages)).lower() + "\n")
    else:
        record = json.loads(Path("release-image.json").read_text())
        bundle(root, expected, record, args.destination)


if __name__ == "__main__":
    main()
