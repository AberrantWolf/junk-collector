#!/usr/bin/env python3
"""Offline checks for deploy/container/upgrade.py. Never runs docker, systemctl or snapper."""
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

HELPER = Path(__file__).resolve().parents[1] / "deploy/container/upgrade.py"
spec = importlib.util.spec_from_file_location("collector_upgrade", HELPER)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)

IMAGE = "ghcr.io/aberrantwolf/junk-collector"
OLD = IMAGE + "@sha256:" + "a" * 64
NEW = IMAGE + "@sha256:" + "b" * 64
OLD_COMMIT, NEW_COMMIT = "1" * 40, "2" * 40
API = "https://api.github.com/repos/AberrantWolf/junk-collector"


class FakeHost:
    """A machine with the old release running and the new one published."""

    def __init__(self, root):
        self.root = root
        self.calls = []
        self.running = True
        self.started_image = "sha256:old"
        self.healthy = True
        self.stop_leaves = []
        self.fail = {}
        self.compare = {"status": "ahead", "files": [{"filename": "crates/x.rs"}, {"filename": "deploy/container/remote-archive.md"}]}
        self.new_labels = {"org.opencontainers.image.revision": NEW_COMMIT, "org.opencontainers.image.version": "v0.1.0-beta.37"}
        (root / "data/state").mkdir(parents=True, exist_ok=True)
        self.snapshots = root / "data/.snapshots"
        self.signals_during_stop = None
        self.interrupt_wait = False
        self.volume = root / "volume"
        self.volume.mkdir(exist_ok=True)
        (self.volume / "service.key").write_text("key")
        self.clock = 0

    def run(self, *argv, check=True):
        self.calls.append(argv)
        for prefix, error in self.fail.items():
            if argv[:len(prefix)] == prefix:
                raise RuntimeError(error)
        match argv:
            case ("docker", "image", "inspect", ref):
                images = {OLD: ("sha256:old", {"org.opencontainers.image.revision": OLD_COMMIT, "org.opencontainers.image.version": "v0.1.0-beta.35"}, [OLD]),
                          f"{IMAGE}:v0.1.0-beta.37": ("sha256:new", self.new_labels, [NEW]),
                          NEW: ("sha256:new", self.new_labels, [NEW])}
                if ref not in images:
                    raise RuntimeError("no such image")
                identity, labels, digests = images[ref]
                return json.dumps([{"Id": identity, "Config": {"Labels": labels}, "RepoDigests": digests}])
            case ("docker", "pull", *_):
                return ""
            case ("systemctl", "stop", _):
                import signal
                self.signals_during_stop = signal.getsignal(signal.SIGTERM)
                self.running = False
                return ""
            case ("systemctl", "start", _):
                self.running = True
                self.started_image = "sha256:new" if NEW in (self.root / "remote.env").read_text() else "sha256:old"
                return ""
            case ("docker", "ps", *rest):
                if not self.running:
                    return "\n".join(self.stop_leaves)
                return "c1\n"
            case ("docker", "inspect", "c1"):
                if self.interrupt_wait and self.started_image == "sha256:new":
                    raise KeyboardInterrupt
                return json.dumps([{"Image": self.started_image, "State": {"Health": {"Status": "healthy" if self.healthy else "starting"}}}])
            case ("snapper", "-c", "junk-collector", "create", *_):
                (self.snapshots / "7/snapshot").mkdir(parents=True)
                return "7\n"
            case ("snapper", "--jsonout", "list-configs"):
                return json.dumps({"configs": [{"config": "junk-collector", "subvolume": str(self.root / "data")}]})
            case ("docker", "volume", "inspect", *_):
                return str(self.volume) + "\n"
            case ("cp", "-a", "--", source, target):
                import shutil
                shutil.copytree(source, target)
                return ""
        raise AssertionError(f"unexpected command {argv}")

    def http_json(self, url):
        self.calls.append(("GET", url))
        answers = {f"{API}/releases/tags/v0.1.0-beta.37": {"draft": False},
                   f"{API}/git/ref/tags/v0.1.0-beta.37": {"object": {"type": "commit", "sha": NEW_COMMIT}},
                   f"{API}/compare/{OLD_COMMIT}...v0.1.0-beta.37": self.compare}
        if url not in answers:
            raise module.Refused(f"GitHub has no {url}")
        return answers[url]

    def monotonic(self):
        return self.clock

    def sleep(self, seconds):
        self.clock += seconds

    def commands(self):
        return [call[:2] for call in self.calls if call[0] != "GET"]


class UpgradeTests(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name)
        self.env = self.root / "remote.env"
        self.original = f"# settings\nCOLLECTOR_DATA={self.root}/data\nCOLLECTOR_STATE={self.root}/data/state\nCOLLECTOR_BACKUPS={self.root}/data/backups\nCOLLECTOR_IMAGE={OLD}\nCOLLECTOR_MEDIA_IMAGE={OLD}\nOTHER=1\n"
        self.env.write_text(self.original)
        self.env.chmod(0o600)
        self.config = self.root / "upgrade.toml"
        self.config.write_text(f'env_file = "{self.env}"\nhealth_timeout_seconds = 60\n')
        self.host = FakeHost(self.root)
        for name, value in (("CONFIG_PATH", self.config), ("LOCK_PATH", self.root / "lock"), ("LOG_PATH", self.root / "log")):
            patcher = patch.object(module, name, value)
            patcher.start()
            self.addCleanup(patcher.stop)
        for patcher in (patch.object(module.os, "geteuid", return_value=0), patch.object(module, "require_private"),
                        patch("sys.stdout", io.StringIO())):
            patcher.start()
            self.addCleanup(patcher.stop)

    def main(self, *argv):
        return module.main(list(argv), host=self.host)

    def test_upgrade_snapshots_then_switches_both_pins_and_waits_for_health(self):
        self.assertEqual(self.main("upgrade", "v0.1.0-beta.37"), 0)
        self.assertEqual(self.env.read_text(), self.original.replace(OLD, NEW))
        self.assertEqual(self.env.stat().st_mode & 0o777, 0o600)
        self.assertEqual((self.root / "remote.env.v0.1.0-beta.35").read_text(), self.original)
        private = self.root / "data/.snapshots/7"
        self.assertEqual((private / "remote.env").read_text(), self.original)
        self.assertEqual((private / "media-authority/service.key").read_text(), "key")
        order = self.host.commands()
        self.assertLess(order.index(("docker", "pull")), order.index(("systemctl", "stop")))
        self.assertLess(order.index(("systemctl", "stop")), order.index(("snapper", "-c")))
        self.assertLess(order.index(("snapper", "-c")), order.index(("systemctl", "start")))

    def test_check_changes_nothing_and_never_stops(self):
        self.assertEqual(self.main("check", "v0.1.0-beta.37"), 0)
        self.assertEqual(self.env.read_text(), self.original)
        self.assertNotIn(("systemctl", "stop"), self.host.commands())

    def test_bad_tags_and_settings_are_refused_before_any_command(self):
        for tag in ("latest", "v1", "v0.1.0;id", "../v0.1.0", "v0.1.0-beta.37 "):
            with self.subTest(tag=tag):
                self.host.calls.clear()
                self.assertEqual(self.main("upgrade", tag), 2)
                self.assertEqual(self.host.calls, [])
        for text in (self.original.replace(f"COLLECTOR_IMAGE={OLD}", "COLLECTOR_IMAGE=example.com/other@sha256:" + "c" * 64),
                     self.original + f"COLLECTOR_IMAGE={OLD}\n",
                     self.original.replace(f"COLLECTOR_MEDIA_IMAGE={OLD}", f"COLLECTOR_MEDIA_IMAGE={NEW}"),
                     "OTHER=1\n"):
            with self.subTest(text=text):
                self.env.write_text(text)
                self.assertEqual(self.main("upgrade", "v0.1.0-beta.37"), 2)
                self.assertEqual(self.env.read_text(), text)
        self.config.write_text('servise = "typo"\n')
        self.assertEqual(self.main("check", "v0.1.0-beta.37"), 2)

    def test_deployment_changes_need_acknowledging(self):
        self.host.compare["files"].append({"filename": "deploy/container/compose.yaml"})
        self.assertEqual(self.main("upgrade", "v0.1.0-beta.37"), 2)
        self.assertNotIn(("systemctl", "stop"), self.host.commands())
        self.assertEqual(self.main("upgrade", "v0.1.0-beta.37", "--deploy-files-updated"), 0)
        self.host.compare["files"] = [{"filename": f"f{n}"} for n in range(300)]
        self.env.write_text(self.original)
        self.assertEqual(self.main("check", "v0.1.0-beta.37"), 2)

    def test_wrong_image_older_release_or_draft_is_refused(self):
        self.host.new_labels = {**self.host.new_labels, "org.opencontainers.image.revision": "3" * 40}
        self.assertEqual(self.main("upgrade", "v0.1.0-beta.37"), 2)
        self.host.new_labels = {**self.host.new_labels, "org.opencontainers.image.revision": NEW_COMMIT}
        for status in ("behind", "diverged", "identical"):
            with self.subTest(status=status):
                self.host.compare = {"status": status, "files": []}
                self.assertEqual(self.main("upgrade", "v0.1.0-beta.37"), 2)
        self.assertNotIn(("systemctl", "stop"), self.host.commands())
        self.assertEqual(self.env.read_text(), self.original)

    def test_failure_before_the_switch_starts_the_old_version_again(self):
        for case in ("containers", "snapshot", "volume"):
            with self.subTest(case=case):
                self.host = FakeHost(self.root)
                if case == "containers":
                    self.host.stop_leaves = ["stuck"]
                elif case == "snapshot":
                    self.host.fail = {("snapper", "-c"): "no space"}
                else:
                    self.host.fail = {("docker", "volume"): "no such volume"}
                self.assertEqual(self.main("upgrade", "v0.1.0-beta.37"), 3)
                self.assertEqual(self.env.read_text(), self.original)
                self.assertTrue(self.host.running)
                self.assertEqual(self.host.started_image, "sha256:old")
                for path in (self.root / "data").glob(".snapshots/*"):
                    import shutil
                    shutil.rmtree(path)
                for path in self.root.glob("remote.env.*"):
                    path.unlink()

    def test_unhealthy_new_version_is_left_for_a_person_with_recovery_steps(self):
        self.host.healthy = False
        self.assertEqual(self.main("upgrade", "v0.1.0-beta.37"), 4)
        self.assertEqual(self.env.read_text(), self.original.replace(OLD, NEW))
        log = (self.root / "log").read_text()
        self.assertIn("snapshot 7", log)
        self.assertIn(str(self.root / "remote.env.v0.1.0-beta.35"), log)

    def test_stop_or_settings_write_failure_starts_the_old_version_again(self):
        self.host.fail = {("systemctl", "stop"): "job failed"}
        self.assertEqual(self.main("upgrade", "v0.1.0-beta.37"), 3)
        self.assertTrue(self.host.running)
        self.assertEqual(self.host.started_image, "sha256:old")
        self.host = FakeHost(self.root)
        with patch.object(module, "write_private", side_effect=OSError("disk full")):
            self.assertEqual(self.main("upgrade", "v0.1.0-beta.37", "--deploy-files-updated"), 3)
        self.assertEqual(self.env.read_text(), self.original)
        self.assertEqual(self.host.started_image, "sha256:old")
        self.assertIn("rm -r -- ", (self.root / "log").read_text())

    def test_old_version_that_fails_to_return_is_reported_as_down(self):
        self.host.fail = {("snapper", "-c"): "no space"}
        self.host.healthy = False
        self.assertEqual(self.main("upgrade", "v0.1.0-beta.37"), 4)
        self.assertIn("THE SERVER IS DOWN", (self.root / "log").read_text())
        self.assertEqual(self.env.read_text(), self.original)

    def test_termination_is_held_while_down_and_an_interrupted_wait_prints_recovery(self):
        import signal
        self.host.interrupt_wait = True
        self.assertEqual(self.main("upgrade", "v0.1.0-beta.37"), 4)
        self.assertEqual(self.host.signals_during_stop, signal.SIG_IGN)
        self.assertNotEqual(signal.getsignal(signal.SIGTERM), signal.SIG_IGN)
        self.assertIn("snapshot 7", (self.root / "log").read_text())

    def test_state_outside_the_snapshot_or_another_service_is_refused(self):
        for text in (self.original + "COLLECTOR_CONFIG=/elsewhere/config\n",
                     self.original.replace("COLLECTOR_IMAGE=", "export COLLECTOR_IMAGE="),
                     self.original + f'COLLECTOR_IMAGE="{OLD}"\n'):
            with self.subTest(text=text):
                self.env.write_text(text)
                self.assertEqual(self.main("check", "v0.1.0-beta.37"), 2)
        self.env.write_text(self.original)
        self.host.started_image = "sha256:other"
        self.assertEqual(self.main("check", "v0.1.0-beta.37"), 2)
        self.host.running = False
        self.assertEqual(self.main("check", "v0.1.0-beta.37"), 2)
        self.host.running = True
        self.host.started_image = "sha256:old"
        self.host.fail = {("docker", "pull"): "network down"}
        self.assertEqual(self.main("check", "v0.1.0-beta.37"), 2)
        self.assertIn("Refused, nothing changed", (self.root / "log").read_text())

    def test_rewrite_keeps_other_lines_and_line_endings(self):
        text, current = module.rewrite_images("A=1\r\nCOLLECTOR_IMAGE=" + OLD + "\r\n", IMAGE, NEW)
        self.assertEqual((text, current), ("A=1\r\nCOLLECTOR_IMAGE=" + NEW + "\r\n", OLD))

    def test_files_others_could_change_are_refused(self):
        real = importlib.util.module_from_spec(spec)  # setUp replaced require_private on `module`.
        spec.loader.exec_module(real)
        folder = self.root / "private"
        folder.mkdir()
        folder.chmod(0o700)
        target = folder / "upgrade.toml"
        target.write_text("")
        target.chmod(0o600)
        owner = target.stat().st_uid
        real.require_private(target, owner=owner, top=folder)
        for mode, folder_mode in ((0o666, 0o700), (0o620, 0o700), (0o600, 0o777)):
            with self.subTest(mode=oct(mode), folder=oct(folder_mode)):
                target.chmod(mode)
                folder.chmod(folder_mode)
                with self.assertRaises(real.Refused):
                    real.require_private(target, owner=owner, top=folder)
        target.chmod(0o600)
        folder.chmod(0o700)
        with self.assertRaises(real.Refused):
            real.require_private(target, owner=owner + 1, top=folder)
        link = folder / "link"
        link.symlink_to(target)
        with self.assertRaises(real.Refused):
            real.require_private(link, owner=owner, top=folder)


if __name__ == "__main__":
    unittest.main()
