"""Offline wrapper regressions; never opens an optical drive."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


SCRIPT = Path(__file__).with_name("rip-cd.sh")
FAKE = """#!/usr/bin/env python3
import os
from pathlib import Path
import sys

stage = sys.argv[1]
options = dict(arg[2:].split('=', 1) for arg in sys.argv[2:])
if stage == 'eject':
    assert options == {'drive': os.environ.get('TEST_EXPECTED_DRIVE', 'disk11')}, 'eject must use only the pinned drive'
    Path(os.environ['TEST_EJECT_MARKER']).write_text(options['drive'])
    if os.environ.get('TEST_EJECT_WARNING'):
        print('warning: failed to eject, SCSI (fixture refusal)')
    sys.exit(23 if os.environ.get('TEST_FAIL_STAGE') == stage else 0)
output = Path(options['image-path'])
with (output / 'calls').open('a') as calls:
    calls.write(stage + '\\n')
if stage == 'dump':
    for suffix in ['toc', 'scram', 'state', 'subcode', 'fulltoc']:
        (output / ('disc.' + suffix)).write_bytes(b'raw fixture')
    (output / 'disc.log').write_text(
        'drive information\\n  path: disk11\\n\\n'
        + os.environ['TEST_SUMMARY'] + '\\n\\n*** END\\n')
    if os.environ.get('TEST_MISSING_REQUIRED'):
        (output / 'disc.toc').unlink()
else:
    assert options['drive'] == os.environ.get('TEST_EXPECTED_DRIVE', 'disk11'), 'selected drive was not retained'
    if os.environ.get('TEST_FAIL_STAGE') == stage:
        sys.exit(23)
"""


class RipCdTests(unittest.TestCase):
    def run_capture(self, summary, fail_stage="", drive="", eject_warning=False, missing_required=False):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        fake = root / "fake redumper"
        fake.write_text(FAKE)
        fake.chmod(0o755)
        marker = root / "eject.called"
        result = subprocess.run(
            ["bash", str(SCRIPT), "Album - Disc 1", str(root / "dump path"), *([drive] if drive else [])],
            env=dict(os.environ, REDUMPER=str(fake), TEST_SUMMARY=summary,
                     TEST_FAIL_STAGE=fail_stage, TEST_EJECT_MARKER=str(marker),
                     TEST_EXPECTED_DRIVE=drive or 'disk11', TEST_EJECT_WARNING='1' if eject_warning else '',
                     TEST_MISSING_REQUIRED='1' if missing_required else ''),
            text=True, capture_output=True, timeout=30,
        )
        output = root / "dump path" / "Album - Disc 1"
        calls = (output / "calls").read_text().splitlines()
        self.assertEqual((output / "disc.scram").read_bytes(), b"raw fixture")
        self.assertTrue((output / "disc.fulltoc").exists())
        self.assertEqual(marker.exists(), result.returncode == 0, result.stderr)
        if marker.exists():
            self.assertEqual(marker.read_text(), drive or 'disk11')
        return result, calls

    def test_clean_and_q_only_dumps_do_not_start_refinement(self):
        for q_errors in (0, 113):
            with self.subTest(q_errors=q_errors):
                result, calls = self.run_capture(
                    f"media errors: \n  SCSI: 0 samples\n  C2: 0 samples\n  Q: {q_errors}")
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(calls, ["dump", "dump::extra"])
                self.assertIn("Skipping refinement", result.stdout)

    def test_each_sample_error_kind_starts_refinement(self):
        for scsi, c2 in ((1, 0), (0, 588), (12, 24)):
            with self.subTest(scsi=scsi, c2=c2):
                result, calls = self.run_capture(
                    f"media errors: \n  SCSI: {scsi} samples\n  C2: {c2} samples\n  Q: 0")
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(calls, ["dump", "dump::extra", "refine"])

    def test_unknown_summaries_stop_without_refinement(self):
        clean = "media errors:\n  SCSI: 0 samples\n  C2: 0 samples"
        for summary in ("", "media errors:\n  SCSI: 0 samples",
                        clean.replace("C2: 0", "C2: unknown"),
                        clean.replace("C2: 0 samples", "C2: 0 sectors"),
                        clean + "\n  C2: 0 samples", clean + "\n\n" + clean):
            with self.subTest(summary=summary):
                result, calls = self.run_capture(summary)
                self.assertNotEqual(result.returncode, 0)
                self.assertNotIn("refine", calls)
                self.assertIn("Cannot read initial SCSI/C2 error summary", result.stderr)

    def test_extra_failure_stops_before_refinement(self):
        result, calls = self.run_capture(
            "media errors:\n  SCSI: 1 samples\n  C2: 0 samples", "dump::extra")
        self.assertEqual(result.returncode, 23)
        self.assertEqual(calls, ["dump", "dump::extra"])

    def test_refinement_failure_remains_a_failure(self):
        result, calls = self.run_capture(
            "media errors:\n  SCSI: 1 samples\n  C2: 0 samples", "refine")
        self.assertEqual(result.returncode, 23)
        self.assertEqual(calls, ["dump", "dump::extra", "refine"])

    def test_explicit_drive_is_reused_for_eject(self):
        result, _ = self.run_capture("media errors:\n  SCSI: 0 samples\n  C2: 0 samples", drive="disk7")
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_incomplete_capture_is_not_ejected(self):
        result, _ = self.run_capture("media errors:\n  SCSI: 0 samples\n  C2: 0 samples", missing_required=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("disc.toc", result.stderr)

    def test_eject_failures_do_not_reclassify_a_saved_capture(self):
        for options in ({"fail_stage": "eject"}, {"eject_warning": True}):
            with self.subTest(options=options):
                result, _ = self.run_capture("media errors:\n  SCSI: 0 samples\n  C2: 0 samples", **options)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertIn("Capture saved, but", result.stderr)
                self.assertNotIn("Eject command completed", result.stdout)


if __name__ == "__main__":
    unittest.main()
