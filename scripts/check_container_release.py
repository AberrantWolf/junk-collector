#!/usr/bin/env python3
"""Release-policy and publication-evidence regressions (no registry writes)."""
import copy
import json
from pathlib import Path
import tempfile
import unittest
import container_release as release

SHA = "a"*40
AMD = "sha256:" + "1"*64
ARM = "sha256:" + "2"*64
ATTEST = "sha256:" + "3"*64
ROOT = Path(__file__).resolve().parents[1]


def planned(version="0.1.0-alpha.2", prerelease=True):
    return release.plan("AberrantWolf/junk-collector", SHA, version, "release",
                        dict(tag_name="v"+version, prerelease=prerelease, draft=False))


def index():
    return dict(schemaVersion=2, mediaType="application/vnd.oci.image.index.v1+json", manifests=[
        dict(digest=AMD, platform=dict(os="linux", architecture="amd64")),
        dict(digest=ARM, platform=dict(os="linux", architecture="arm64", variant="v8")),
        dict(digest=ATTEST, platform=dict(os="unknown", architecture="unknown"),
             annotations={"vnd.docker.reference.type":"attestation-manifest", "vnd.docker.reference.digest":AMD})])


class ReleaseTests(unittest.TestCase):
    def test_stable_and_preview_are_disjoint(self):
        self.assertEqual(planned()["channel"], "preview")
        self.assertEqual(planned("0.1.0", False)["channel"], "latest")
        for version, flag in (("0.1.0", True), ("0.1.0-alpha.2", False)):
            with self.assertRaises(ValueError):
                planned(version, flag)
        with self.assertRaises(ValueError):
            release.plan("AberrantWolf/junk-collector", SHA, "0.1.0", "release", dict(tag_name="v0.0.9", prerelease=False, draft=False))
        manual = release.plan("AberrantWolf/junk-collector", SHA, "0.1.0-alpha.2", "workflow_dispatch", {})
        self.assertIsNone(manual["channel"])

    def test_semantic_order_is_numeric_and_rejects_ambiguous_tags(self):
        values = ["0.1.0-alpha.2", "0.1.0-alpha.10", "0.1.0-beta.1", "0.1.0", "0.1.1", "0.2.0"]
        self.assertEqual(sorted(reversed(values), key=release.version_key), values)
        for invalid in ("01.1.0", "0.1.0-alpha.01", "0.1.0-alpha..1", "0.1.0+metadata", "0.1.0\nlatest"):
            with self.assertRaises(ValueError):
                release.version_key(invalid)

    def test_channel_guard_reads_later_pages_and_ignores_other_channel(self):
        current = planned()
        first = dict(tag_name=current["tag"], prerelease=True, draft=False)
        newer = dict(tag_name="v0.1.0-alpha.10", prerelease=True, draft=False)
        stable = dict(tag_name="v1.0.0", prerelease=False, draft=False)
        self.assertTrue(release.promote_current(current, [[first], [stable]]))
        self.assertFalse(release.promote_current(current, [[first], [newer, stable]]))
        with self.assertRaises(ValueError):
            release.promote_current(current, [[newer]])

    def test_both_platforms_and_bound_attestations_are_required(self):
        self.assertEqual(release.platforms(index()), {"linux/amd64":AMD, "linux/arm64":ARM})
        for change in ("missing", "duplicate", "foreign", "unbound"):
            value = index()
            if change == "missing":
                value["manifests"].pop(1)
            elif change == "duplicate":
                value["manifests"].append(copy.deepcopy(value["manifests"][0]))
            elif change == "foreign":
                value["manifests"][1]["platform"]["architecture"] = "arm"
            else:
                value["manifests"][2]["annotations"]["vnd.docker.reference.digest"] = ATTEST
            with self.subTest(change=change), self.assertRaises(ValueError):
                release.platforms(value)

    def test_actual_platform_and_all_source_labels_must_match(self):
        expected = planned()
        image = dict(Os="linux", Architecture="amd64", Labels={"org.opencontainers.image."+name:expected[name] for name in ("version","revision","source")})
        release.check_labels(image, expected, "linux/amd64")
        for name in ("version", "revision", "source"):
            changed = copy.deepcopy(image)
            changed["Labels"]["org.opencontainers.image."+name] = "wrong"
            with self.assertRaises(ValueError):
                release.check_labels(changed, expected, "linux/amd64")
        with self.assertRaises(ValueError):
            release.check_labels(image, expected, "linux/arm64")

    def test_published_bundle_is_pinned_and_self_contained(self):
        expected = planned()
        record = dict(expected, digest=AMD, reference=expected["image"]+"@"+AMD, anonymous_docker_pulls=True, platforms={"linux/amd64":AMD,"linux/arm64":ARM})
        with tempfile.TemporaryDirectory() as directory:
            destination = Path(directory)/"bundle"
            release.bundle(ROOT, expected, record, destination)
            self.assertIn("COLLECTOR_IMAGE="+record["reference"], (destination/".env.example").read_text())
            text = (destination/"README.md").read_text()
            self.assertNotIn("Registry images have not yet been published", text)
            self.assertNotIn("Registry publication has not yet", text)
            self.assertNotIn("../../docs/", text)
            for name in ("compose.inbox.yaml", "snapshot-recovery.md", "freeze-tar-time.py", "release-image.json",
                         "compose.remote.yaml", "remote.env.example", "remote-preflight.py",
                         "junk-collector-remote.service", "remote-archive.md"):
                self.assertTrue((destination/name).is_file())
            # The migration runbook must ship with its executable/template dependencies.
            for name in ("compose.remote.yaml", "remote.env.example", "remote-preflight.py", "junk-collector-remote.service"):
                self.assertEqual((destination/name).read_bytes(), (ROOT/"deploy/container"/name).read_bytes())
            for document in destination.glob("*.md"):
                self.assertNotIn("../../docs/", document.read_text(), document.name)
            self.assertIn(expected["source"]+"/blob/"+expected["revision"]+"/docs/validation/remote-archive-safety-2026-10-30.md",
                          (destination/"remote-archive.md").read_text())
            self.assertEqual(json.loads((destination/"release-image.json").read_text()), record)
            record["anonymous_docker_pulls"] = False
            with self.assertRaises(ValueError):
                release.bundle(ROOT, expected, record, Path(directory)/"unverified")
            self.assertFalse((Path(directory)/"unverified").exists())
            manual = release.plan("AberrantWolf/junk-collector", SHA, expected["version"], "workflow_dispatch", {})
            release.bundle(ROOT, manual, record, Path(directory)/"dry-run")
            self.assertIn("COLLECTOR_IMAGE=junk-collector:local", (Path(directory)/"dry-run/.env.example").read_text())


if __name__ == "__main__":
    unittest.main()
