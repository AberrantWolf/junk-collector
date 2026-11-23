"""Offline wrapper regressions; never opens an optical drive."""
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


SCRIPT = Path(__file__).with_name("rip-cd.sh")


def install_eject_tools(root):
    """Every test shadows host detection and diskutil; never eject real media."""
    uname = root / "uname"
    uname.write_text('#!/bin/sh\nprintf "%s\\n" "${TEST_PLATFORM:-Darwin}"\n')
    uname.chmod(0o755)
    diskutil = root / "diskutil"
    # `info` mimics macOS hiding the disk node after redumper exits: it reports
    # the disk missing for the first TEST_DISK_HIDDEN_POLLS calls, then present.
    # TEST_DISK_GONE_AFTER makes every call after that many report it missing.
    diskutil.write_text(
        f'#!{sys.executable}\nimport os, sys\nfrom pathlib import Path\n'
        "if sys.argv[1] == 'info':\n"
        "    assert len(sys.argv) == 3 and sys.argv[2].startswith('/dev/disk')\n"
        "    counter = Path(os.environ['TEST_EJECT_MARKER'] + '.polls')\n"
        "    polls = int(counter.read_text()) + 1 if counter.exists() else 1\n"
        "    counter.write_text(str(polls))\n"
        "    hidden = int(os.environ.get('TEST_DISK_HIDDEN_POLLS') or 0)\n"
        "    gone_after = int(os.environ.get('TEST_DISK_GONE_AFTER') or 0)\n"
        "    sys.exit(1 if polls <= hidden or 0 < gone_after < polls else 0)\n"
        "assert sys.argv[1] == 'eject' and len(sys.argv) == 3\n"
        "Path(os.environ['TEST_EJECT_MARKER']).write_text(sys.argv[2])\n"
        "print('fixture diskutil eject')\n"
        "sys.exit(23 if os.environ.get('TEST_FAIL_STAGE') == 'eject' else 0)\n")
    diskutil.chmod(0o755)


FAKE = """#!/usr/bin/env python3
import os
from pathlib import Path
import sys

stage = sys.argv[1]
options = dict(arg[2:].split('=', 1) for arg in sys.argv[2:])
if stage == 'eject':
    assert os.environ.get('TEST_PLATFORM') == 'Linux', 'macOS must not use redumper eject'
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
    def run_capture(self, summary, fail_stage="", drive="", eject_warning=False, missing_required=False,
                    platform="Darwin", invalid_eject=False, hidden_polls=0, gone_after=0):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        install_eject_tools(root)
        fake = root / "fake redumper"
        fake.write_text(FAKE)
        fake.chmod(0o755)
        marker = root / "eject.called"
        result = subprocess.run(
            ["bash", str(SCRIPT), str(root / "dump path"), *([drive] if drive else []), "Album - Disc 1"],
            env=dict(os.environ, REDUMPER=str(fake), TEST_SUMMARY=summary,
                     PATH=str(root) + os.pathsep + os.environ['PATH'], TEST_PLATFORM=platform,
                     TEST_FAIL_STAGE=fail_stage, TEST_EJECT_MARKER=str(marker),
                     TEST_EXPECTED_DRIVE=drive or 'disk11', TEST_EJECT_WARNING='1' if eject_warning else '',
                     TEST_MISSING_REQUIRED='1' if missing_required else '',
                     TEST_DISK_HIDDEN_POLLS=str(hidden_polls), TEST_DISK_GONE_AFTER=str(gone_after),
                     MAC_DISK_WAIT='2'),
            text=True, capture_output=True, timeout=30,
        )
        output = root / "dump path" / "Album - Disc 1"
        calls = (output / "calls").read_text().splitlines()
        self.assertEqual((output / "disc.scram").read_bytes(), b"raw fixture")
        self.assertTrue((output / "disc.fulltoc").exists())
        self.assertEqual(marker.exists(), result.returncode == 0 and not invalid_eject, result.stderr)
        if marker.exists():
            expected = drive or 'disk11'
            if platform == 'Darwin':
                expected = '/dev/' + expected.removeprefix('/dev/').removeprefix('r')
            self.assertEqual(marker.read_text(), expected)
        return result, calls

    def test_clean_and_q_only_dumps_report_no_errors(self):
        for q_errors in (0, 113):
            with self.subTest(q_errors=q_errors):
                result, calls = self.run_capture(
                    f"media errors: \n  SCSI: 0 samples\n  C2: 0 samples\n  Q: {q_errors}")
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(calls, ["dump", "dump::extra"])
                self.assertNotIn("CHECK THIS DISC", result.stderr)

    def test_sample_errors_are_reported_but_never_refined(self):
        for scsi, c2 in ((1, 0), (0, 588), (12, 24)):
            with self.subTest(scsi=scsi, c2=c2):
                result, calls = self.run_capture(
                    f"media errors: \n  SCSI: {scsi} samples\n  C2: {c2} samples\n  Q: 0")
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(calls, ["dump", "dump::extra"])
                self.assertIn("CHECK THIS DISC", result.stderr)
                self.assertIn(f"{scsi} SCSI and {c2} C2", result.stderr)
                self.assertIn(" refine --image-path=", result.stderr)

    def test_unknown_summaries_are_reported_but_never_refined(self):
        clean = "media errors:\n  SCSI: 0 samples\n  C2: 0 samples"
        for summary in ("", "media errors:\n  SCSI: 0 samples",
                        clean.replace("C2: 0", "C2: unknown"),
                        clean.replace("C2: 0 samples", "C2: 0 sectors"),
                        clean + "\n  C2: 0 samples", clean + "\n\n" + clean):
            with self.subTest(summary=summary):
                result, calls = self.run_capture(summary)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(calls, ["dump", "dump::extra"])
                self.assertIn("CHECK THIS DISC", result.stderr)
                self.assertIn("could not read the SCSI/C2 error summary", result.stderr)

    def test_extra_failure_stops_the_capture(self):
        result, calls = self.run_capture(
            "media errors:\n  SCSI: 1 samples\n  C2: 0 samples", "dump::extra")
        self.assertEqual(result.returncode, 23)
        self.assertEqual(calls, ["dump", "dump::extra"])

    def test_explicit_drive_is_reused_for_eject(self):
        for drive in ('disk7', '/dev/disk7', '/dev/rdisk7'):
            with self.subTest(drive=drive):
                result, _ = self.run_capture("media errors:\n  SCSI: 0 samples\n  C2: 0 samples", drive=drive)
                self.assertEqual(result.returncode, 0, result.stderr)

    def test_mac_waits_for_the_hidden_disk_to_reappear(self):
        result, calls = self.run_capture("media errors:\n  SCSI: 0 samples\n  C2: 0 samples", hidden_polls=2)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(calls, ["dump", "dump::extra"])
        self.assertIn("Eject command completed", result.stdout)

    def test_mac_disk_that_never_reappears_stops_before_the_next_stage(self):
        result, calls = self.run_capture("media errors:\n  SCSI: 0 samples\n  C2: 0 samples",
                                         hidden_polls=1000)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(calls, ["dump"])
        self.assertIn("did not reappear", result.stderr)

    def test_mac_disk_missing_at_eject_is_reported_not_ejected(self):
        result, _ = self.run_capture("media errors:\n  SCSI: 0 samples\n  C2: 0 samples",
                                     gone_after=1, invalid_eject=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("did not reappear", result.stderr)

    def test_unrecognized_mac_device_is_not_ejected(self):
        result, _ = self.run_capture("media errors:\n  SCSI: 0 samples\n  C2: 0 samples",
                                     drive='/dev/sr0', invalid_eject=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('cannot map macOS drive', result.stderr)

    def test_linux_uses_redumper_for_the_pinned_drive(self):
        result, _ = self.run_capture("media errors:\n  SCSI: 0 samples\n  C2: 0 samples",
                                     platform='Linux', drive='/dev/sg2')
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_incomplete_capture_is_not_ejected(self):
        result, _ = self.run_capture("media errors:\n  SCSI: 0 samples\n  C2: 0 samples", missing_required=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("disc.toc", result.stderr)

    def test_eject_failures_do_not_reclassify_a_saved_capture(self):
        for options in ({"fail_stage": "eject"},
                        {"platform": "Linux", "fail_stage": "eject"},
                        {"platform": "Linux", "eject_warning": True}):
            with self.subTest(options=options):
                result, _ = self.run_capture("media errors:\n  SCSI: 0 samples\n  C2: 0 samples", **options)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertIn("Capture saved, but", result.stderr)
                self.assertNotIn("Eject command completed", result.stdout)


class ArgumentOrderTests(unittest.TestCase):
    def test_old_argument_order_is_refused_before_creating_folders(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        dumps = str(root / "dumps")
        for old_style in (["Album", dumps, "disk7"], ["Album", dumps], ["--auto", dumps, "disk7"],
                          ["--auto", dumps], [dumps, "disk7s1", "Album"]):
            with self.subTest(args=old_style):
                result = subprocess.run(["bash", str(SCRIPT), *old_style], cwd=root,
                                        env=dict(os.environ, REDUMPER="true"),
                                        text=True, capture_output=True, timeout=30)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("DESTINATION [DRIVE] NAME", result.stderr)
                self.assertEqual(sorted(p.name for p in root.iterdir()), [])


if __name__ == "__main__":
    unittest.main()
