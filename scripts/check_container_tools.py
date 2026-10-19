#!/usr/bin/env python3
"""Check the narrow GNU-tar cutoff adapter's preservation/refusal guarantees."""
import importlib.util
import os
from pathlib import Path
import tempfile
import unittest

helper = Path(__file__).resolve().parents[1] / "deploy/container/freeze-tar-time.py"
spec = importlib.util.spec_from_file_location("freeze_tar_time", helper)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class CutoffTests(unittest.TestCase):
    def test_preserves_directory_records_and_private_mode(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "point.snar.part"
            records = b"0\0unknown-directory-record\0\0"
            path.write_bytes(b"GNU tar-1.34-2\n200\0" + b"50\0" + records)
            module.freeze(path, 199_000_000_025)
            self.assertEqual(path.read_bytes(), b"GNU tar-1.34-2\n199\0" + b"25\0" + records)
            self.assertEqual(path.stat().st_mode & 0o777, 0o600)

    def test_refusal_keeps_previous_bytes(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "point.snar.part"
            for raw, cutoff in [(b"unknown-format\n200\0000\0data", 100),
                                (b"GNU tar-1.34-2\n200\0invalid\0data", 100),
                                (b"GNU tar-1.34-2\n200\0" + b"1000000000\0data", 100),
                                (b"GNU tar-1.34-2\n200\0" + b"0\0data", 201_000_000_000)]:
                with self.subTest(raw=raw):
                    path.write_bytes(raw)
                    with self.assertRaises(ValueError):
                        module.freeze(path, cutoff)
                    self.assertEqual(path.read_bytes(), raw)

    def test_refuses_published_or_aliased_snapshot(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "published.snar"
            raw = b"GNU tar-1.34-2\n200\0" + b"0\0data"
            path.write_bytes(raw)
            alias = Path(directory) / "alias.snar.part"
            alias.symlink_to(path)
            linked = Path(directory) / "linked.snar.part"
            os.link(path, linked)
            for target in (path, alias, linked):
                with self.assertRaises(ValueError):
                    module.freeze(target, 100)
                self.assertEqual(path.read_bytes(), raw)

    def test_refuses_oversized_working_file_without_mutation(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "large.snar.part"
            with path.open("wb") as target:
                target.truncate(64*1024*1024 + 1)
            with self.assertRaises(ValueError):
                module.freeze(path, 100)
            self.assertEqual(path.stat().st_size, 64*1024*1024 + 1)


if __name__ == "__main__":
    unittest.main()
