#!/usr/bin/env python3
"""Offline host-admission fixtures. Never mount or contact a NAS."""
import importlib.util
from pathlib import Path
import tempfile
import unittest
import uuid
from unittest.mock import patch
import contextlib
import io

HELPER = Path(__file__).resolve().parents[1] / "deploy/container/remote-preflight.py"
spec = importlib.util.spec_from_file_location("remote_preflight", HELPER)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class AdmissionTests(unittest.TestCase):
    def setUp(self):
        self.root = tempfile.TemporaryDirectory()
        self.addCleanup(self.root.cleanup)
        root = Path(self.root.name).resolve()
        self.env = {}
        for name in ("CONFIG", "STATE", "BACKUPS", "ARCHIVE", "INBOX"):
            path = root / name.lower()
            path.mkdir()
            self.env[f"COLLECTOR_{name}"] = str(path)
        data_root = root / "local"
        data_root.mkdir()
        (data_root / "media-certificates").mkdir()
        (data_root / "media-proxy-config").mkdir()
        self.env["COLLECTOR_DATA"] = str(data_root)
        self.env.update(COLLECTOR_ARCHIVE_SOURCE="nas:/archive", COLLECTOR_INBOX_SOURCE="nas:/inbox", COLLECTOR_ARCHIVE_ID=str(uuid.uuid4()))
        value = self.env["COLLECTOR_ARCHIVE_ID"] + "\n"
        (root / "archive/.archive-identity").write_text(value)
        (root / "state/archive-binding").write_text(value)
        (root / "config/collector.toml").write_text('archive="/data/archive"\nstate="/data/state"\n[archive_mount]\nnfs_source="nas:/archive"\n[remote_media]\nsocket="/run/collector-authority/socket"\n')
        self.local = "1 0 0:1 / / rw - ext4 /dev/local rw\n"
        self.archive = f"2 1 0:2 / {root}/archive rw - nfs4 nas:/archive rw,hard,vers=4.1\n"
        self.inbox = f"3 1 0:3 / {root}/inbox ro - nfs4 nas:/inbox ro,hard,vers=4.1\n"
        self.table = self.local + self.archive + self.inbox

    def test_correct_split_is_admitted_without_mutation(self):
        root = Path(self.root.name).resolve()
        before = {str(path): path.read_bytes() for path in root.rglob("*") if path.is_file()}
        module.check(self.env, self.table)
        self.assertEqual(before, {str(path): path.read_bytes() for path in root.rglob("*") if path.is_file()})

    def test_absent_wrong_soft_readonly_or_nested_archive_refused(self):
        cases = [self.local + self.inbox,
                 self.table.replace("nas:/archive", "other:/archive"),
                 self.table.replace("rw,hard", "rw,soft"),
                 self.table.replace("nfs4", "cifs"),
                 self.table.replace("vers=4.1", "vers=3"),
                 self.table.replace(f'{self.env["COLLECTOR_ARCHIVE"]} rw', f'{self.env["COLLECTOR_ARCHIVE"]} ro'),
                 self.table.replace(f'{self.env["COLLECTOR_ARCHIVE"]} rw', f'{self.env["COLLECTOR_ARCHIVE"]} ro').replace("rw,hard", "ro,hard"),
                 self.table + f'4 2 0:4 / {self.env["COLLECTOR_ARCHIVE"]}/.staging rw - ext4 /dev/b rw\n']
        for table in cases:
            with self.subTest(table=table), self.assertRaises(ValueError):
                module.check(self.env, table)

    def test_inbox_is_independently_required_and_sqlite_must_be_local(self):
        for table in [self.local + self.archive, self.table + f'4 1 0:4 / {self.env["COLLECTOR_STATE"]} rw - nfs4 nas:/state rw,hard\n']:
            with self.assertRaises(ValueError):
                module.check(self.env, table)

    def test_remote_caddy_private_data_is_refused(self):
        remote_caddy = self.table + f'4 1 0:4 / {self.env["COLLECTOR_DATA"]}/media-certificates rw - nfs4 nas:/private rw,hard,vers=4.1\n'
        with self.assertRaises(ValueError):
            module.check(self.env, remote_caddy)

    def test_missing_media_authority_configuration_is_refused_before_startup(self):
        config = Path(self.env["COLLECTOR_CONFIG"]) / "collector.toml"
        config.write_text('archive="/data/archive"\nstate="/data/state"\n[archive_mount]\nnfs_source="nas:/archive"\n')
        with self.assertRaises(ValueError):
            module.check(self.env, self.table)

    def test_wrong_identity_missing_binding_and_missing_remote_policy_refused(self):
        env = dict(self.env, COLLECTOR_ARCHIVE_ID=str(uuid.uuid4()))
        with self.assertRaises(ValueError):
            module.check(env, self.table)
        binding = Path(self.env["COLLECTOR_STATE"]) / "archive-binding"
        binding.unlink()
        with self.assertRaises(OSError):
            module.check(self.env, self.table)
        binding.write_text(self.env["COLLECTOR_ARCHIVE_ID"] + "\n")
        config = Path(self.env["COLLECTOR_CONFIG"]) / "collector.toml"
        config.write_text('archive="/data/archive"\nstate="/data/state"\n')
        with self.assertRaises(ValueError):
            module.check(self.env, self.table)


probe_spec = importlib.util.spec_from_file_location("collector_filesystem_probe", HELPER.parents[1].parent / "scripts/qualify_archive_filesystem.py")
probe = importlib.util.module_from_spec(probe_spec)
probe_spec.loader.exec_module(probe)


class ProbeTests(unittest.TestCase):
    def test_unmarked_or_existing_archive_is_refused_without_writes(self):
        with tempfile.TemporaryDirectory() as temporary, patch.object(probe.sys, "platform", "linux"):
            root = Path(temporary)
            (root / "original").write_bytes(b"preserve")
            with self.assertRaises(ValueError):
                probe.qualify(root, "nas:/fixture")
            self.assertEqual({path.name for path in root.iterdir()}, {"original"})
            self.assertEqual((root / "original").read_bytes(), b"preserve")

    def test_owned_probe_exercises_local_syscalls_and_removes_only_its_child(self):
        # Synthetic mount table: qualifies the fixture probe implementation only,
        # never NFS behavior on this local test filesystem.
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            marker = root / probe.SENTINEL
            expected = b"DISPOSABLE COLLECTOR NFS QUALIFICATION ONLY\n"
            marker.write_bytes(expected)
            entry = dict(kind="nfs4", source="nas:/fixture", options={"hard", "rw", "vers=4.1"}, mount_options={"rw"})
            with patch.object(probe.sys, "platform", "linux"), patch.object(probe.Path, "read_text", return_value="fixture mountinfo"), patch.object(probe.module, "mounts", return_value=[]), patch.object(probe.module, "filesystem", return_value=entry), contextlib.redirect_stdout(io.StringIO()):
                probe.qualify(root, "nas:/fixture")
            self.assertEqual({path.name for path in root.iterdir()}, {probe.SENTINEL})
            self.assertEqual(marker.read_bytes(), expected)


if __name__ == "__main__":
    unittest.main()
