"""Offline naming acceptance, using the built identity tool and fake redumper/HTTP."""
import copy
import io
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch
import urllib.error

from test_rip_cd import FAKE, SCRIPT
import name_cd_capture as naming


DISC_ID = "49HHV7Eb8UKF3aQiNmu1GR8vKTY-"
RESPONSE = {
    "id": DISC_ID,
    "releases": [{
        "id": "00000000-0000-4000-8000-000000000001",
        "label-info": [{"catalog-number": "SQEX-11226"}],
        "media": [{"position": 1, "track-count": 6, "discs": [{"id": DISC_ID}]}],
    }],
}


def toc_bytes():
    # Published MusicBrainz six-track vector, encoded as MMC format-0 LBA TOC.
    body = b"".join(bytes([0, 0x10, number, 0]) + offset.to_bytes(4, "big")
                    for number, offset in zip((1, 2, 3, 4, 5, 6, 0xaa),
                                              (0, 15213, 32164, 46442, 63264, 80339, 95312)))
    return (len(body) + 2).to_bytes(2, "big") + bytes([1, 6]) + body


class CatalogPolicyTests(unittest.TestCase):
    def test_exact_single_disc_and_multi_disc_names(self):
        self.assertEqual(naming.catalog_folder(RESPONSE, DISC_ID, 6), "SQEX-11226")
        data = copy.deepcopy(RESPONSE)
        data["releases"][0]["media"].insert(0, {
            "position": 1, "track-count": 3, "discs": [{"id": "another-disc"}]})
        data["releases"][0]["media"][1]["position"] = 2
        self.assertEqual(naming.catalog_folder(data, DISC_ID, 6), "SQEX-11226 - Disc 2")

    def test_incomplete_conflicting_and_unsafe_candidates_are_inconclusive(self):
        candidates = []
        for labels in ([], [{"catalog-number": "../escape"}], [{"catalog-number": "NUL"}],
                       [{"catalog-number": "[none]"}], [{"catalog-number": "bad\nname"}],
                       [{"catalog-number": "A/B"}], [{"catalog-number": "x" * 200}],
                       [{"catalog-number": "one"}, {"catalog-number": "two"}]):
            data = copy.deepcopy(RESPONSE)
            data["releases"][0]["label-info"] = labels
            candidates.append(data)
        for field, value in (("position", 0), ("position", True), ("track-count", 7),
                             ("discs", [{"id": "unrelated"}])):
            data = copy.deepcopy(RESPONSE)
            data["releases"][0]["media"][0][field] = value
            candidates.append(data)
        for field, value in (("id", "other"), ("releases", []), ("release-count", 2)):
            data = copy.deepcopy(RESPONSE)
            data[field] = value
            candidates.append(data)
        data = copy.deepcopy(RESPONSE)
        data["releases"].append(copy.deepcopy(data["releases"][0]))
        candidates.append(data)  # Same catalog number still doesn't resolve the edition.
        data = copy.deepcopy(RESPONSE)
        data["releases"][0]["media"].append(copy.deepcopy(data["releases"][0]["media"][0]))
        candidates.append(data)
        candidates.extend((None, {}, {"id": DISC_ID, "releases": [None]}))
        for data in candidates:
            with self.subTest(data=data):
                self.assertIsNone(naming.catalog_folder(data, DISC_ID, 6))


class LookupTests(unittest.TestCase):
    def test_refusal_no_match_and_malformed_or_oversized_body_are_not_retried(self):
        identity = subprocess.CompletedProcess([], 0, json.dumps({
            "disc_id": DISC_ID, "track_count": 6}).encode())
        for body in (b"not json", b"x" * (naming.MAX_RESPONSE + 1),
                     json.dumps({"id": DISC_ID, "releases": []}).encode(),
                     urllib.error.HTTPError("https://musicbrainz.org", 503, "busy", {}, None)):
            with self.subTest(body=type(body)):
                class Opener:
                    calls = 0

                    def open(self, request, timeout):
                        self.calls += 1
                        self_test.assertIn("inc=releases%2Blabels%2Bdiscids", request.full_url)
                        if isinstance(body, Exception):
                            raise body
                        return io.BytesIO(body)

                self_test = self
                opener = Opener()
                with patch.object(naming.subprocess, "run", return_value=identity), \
                     patch.object(naming.urllib.request, "build_opener", return_value=opener):
                    try:
                        self.assertIsNone(naming.lookup(Path("unused"), "identity"))
                    except (ValueError, urllib.error.HTTPError):
                        pass
                self.assertEqual(opener.calls, 1)

    def test_redirects_are_refused(self):
        self.assertIsNone(naming.NoRedirect().redirect_request(
            None, None, 302, "redirect", {}, "https://other.example"))

    def test_wall_deadline_kills_worker_and_its_identity_child(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            worker = root / "worker.py"
            pid_file = root / "child.pid"
            worker.write_text(
                "import subprocess, sys, time\n"
                f"child = subprocess.Popen([sys.executable, '-c', "
                f"{('from pathlib import Path; import os, time; Path(' + repr(str(pid_file)) + ').write_text(str(os.getpid())); time.sleep(30)')!r}])\n"
                "time.sleep(30)\n")
            started = time.monotonic()
            with patch.object(naming, "__file__", str(worker)), \
                 patch.object(naming, "LOOKUP_SECONDS", 0.5):
                with self.assertRaises(subprocess.TimeoutExpired):
                    naming.bounded_lookup(root, "unused")
            self.assertLess(time.monotonic() - started, 3)
            self.assertTrue(pid_file.exists(), "fixture child never started")
            pid = int(pid_file.read_text())
            # A killed child may briefly remain a zombie awaiting the OS reaper.
            status = subprocess.run(["ps", "-o", "stat=", "-p", str(pid)],
                                    capture_output=True, text=True, timeout=2).stdout.strip()
            self.assertTrue(not status or status.startswith("Z"), status)

    def test_exclusive_rename_preserves_empty_nonempty_and_symlink_destinations(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "generated"
            source.mkdir()
            (source / "disc.scram").write_bytes(b"master")
            for kind in ("empty", "nonempty", "symlink"):
                destination = root / kind
                if kind == "symlink":
                    destination.symlink_to(source, target_is_directory=True)
                else:
                    destination.mkdir()
                    if kind == "nonempty":
                        (destination / "keep").write_text("original")
                with self.assertRaises(OSError):
                    naming.rename_without_replacement(source, destination)
                self.assertEqual((source / "disc.scram").read_bytes(), b"master")
                self.assertFalse((destination / "generated").exists())
            naming.rename_without_replacement(source, root / "catalog")
            self.assertFalse(source.exists())
            self.assertEqual((root / "catalog" / "disc.scram").read_bytes(), b"master")

    def test_termination_cleans_up_supervised_lookup(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            worker = root / "worker.py"
            pid_file = root / "worker.pid"
            worker.write_text(
                "import os, time\nfrom pathlib import Path\n"
                f"Path({str(pid_file)!r}).write_text(str(os.getpid()))\n"
                "time.sleep(30)\n")
            code = (f"import sys; sys.path.insert(0, {str(SCRIPT.parent)!r}); "
                    f"import name_cd_capture as naming; naming.__file__ = {str(worker)!r}; "
                    f"sys.argv = ['naming', {str(root)!r}, 'unused']; naming.main()")
            parent = subprocess.Popen([sys.executable, "-B", "-c", code],
                                      stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            try:
                deadline = time.monotonic() + 3
                while not pid_file.exists() and time.monotonic() < deadline:
                    time.sleep(0.02)
                self.assertTrue(pid_file.exists(), "lookup worker did not start")
                parent.terminate()
                parent.communicate(timeout=3)
                self.assertEqual(parent.returncode, 128 + 15)
                status = subprocess.run(["ps", "-o", "stat=", "-p", pid_file.read_text()],
                                        capture_output=True, text=True, timeout=2).stdout.strip()
                self.assertTrue(not status or status.startswith("Z"), status)
            finally:
                if parent.poll() is None:
                    parent.kill()
                    parent.communicate(timeout=3)


@unittest.skipUnless(os.environ.get("COLLECTOR_CD_IDENTITY"),
                     "set COLLECTOR_CD_IDENTITY to the release utility for wrapper acceptance")
class AutomaticCaptureTests(unittest.TestCase):
    def run_capture(self, payload=RESPONSE, slow=False, fail_stage="", occupied=False,
                    invalid_toc=False, missing_tool=False):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        destination = root / "dump path"
        destination.mkdir()
        if occupied:
            (destination / "SQEX-11226").mkdir()
            (destination / "SQEX-11226" / "keep").write_bytes(b"existing")
        fake = root / "fake redumper"
        fake.write_text(FAKE + "\nif stage == 'dump':\n"
                        f"    (output / 'disc.toc').write_bytes({b'invalid' if invalid_toc else toc_bytes()!r})\n"
                        "    (output / 'disc.fulltoc').write_bytes(bytes([0, 2, 1, 1]))\n")
        fake.chmod(0o755)
        worker = root / "http_fixture.py"
        worker.write_text(
            "import sys, os, json, io, time\nfrom pathlib import Path\n"
            f"sys.path.insert(0, {str(SCRIPT.parent)!r})\nimport name_cd_capture as naming\n"
            "class Opener:\n"
            "    def open(self, request, timeout):\n"
            "        Path(os.environ['TEST_HTTP_MARKER']).write_text(request.full_url)\n"
            f"        time.sleep({30 if slow else 0})\n"
            f"        return io.BytesIO({json.dumps(payload).encode()!r})\n"
            "naming.urllib.request.build_opener = lambda *args: Opener()\n"
            "print(json.dumps(naming.lookup(Path(sys.argv[2]), sys.argv[3])))\n")
        # Intercept only the naming helper, retaining the real identity binary
        # and its shared parser. Provider I/O happens in the supervised worker.
        python = root / "python3"
        python.write_text(
            f"#!{sys.executable}\nimport os, sys\n"
            "if len(sys.argv) < 2 or sys.argv[1] != '-B':\n"
            "    os.execv(sys.executable, [sys.executable, *sys.argv[1:]])\n"
            f"sys.path.insert(0, {str(SCRIPT.parent)!r})\nimport name_cd_capture as naming\n"
            f"naming.__file__ = {str(worker)!r}\n"
            f"naming.LOOKUP_SECONDS = {0.5 if slow else 15}\n"
            "sys.argv = sys.argv[2:]\nnaming.main()\n")
        python.chmod(0o755)
        marker = root / "http.called"
        environment = dict(os.environ)
        if missing_tool:
            environment["COLLECTOR_CD_IDENTITY"] = str(root / "missing-identity")
        process = subprocess.Popen(
            ["bash", str(SCRIPT), "--auto", str(destination)],
            env=dict(environment, PATH=str(root) + os.pathsep + os.environ["PATH"],
                     REDUMPER=str(fake), TEST_SUMMARY="media errors:\n  SCSI: 0 samples\n  C2: 0 samples",
                     TEST_FAIL_STAGE=fail_stage, TEST_HTTP_MARKER=str(marker)),
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, start_new_session=True,
        )
        try:
            stdout, stderr = process.communicate(timeout=30)
        except BaseException:
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            process.communicate(timeout=3)
            raise
        result = subprocess.CompletedProcess(process.args, process.returncode, stdout, stderr)
        captures = [p for p in destination.iterdir() if (p / "disc.scram").exists()]
        self.assertEqual(len(captures), 1, result.stderr)
        output = captures[0]
        self.assertEqual((output / "disc.scram").read_bytes(), b"raw fixture")
        self.assertEqual((output / "disc.toc").read_bytes(), b"invalid" if invalid_toc else toc_bytes())
        self.assertTrue((output / "disc.state").exists())
        self.assertEqual(output.stat().st_mode & 0o777, destination.stat().st_mode & 0o777)
        return result, output, marker

    def test_completed_capture_uses_unambiguous_catalog_number(self):
        result, output, marker = self.run_capture()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(output.name, "SQEX-11226")
        self.assertIn(str(output), result.stdout)
        self.assertTrue(marker.exists())
        self.assertEqual((output / "calls").read_text().splitlines(), ["dump", "dump::extra"])

    def test_no_match_ambiguity_timeout_and_collision_keep_generated_capture(self):
        ambiguous = copy.deepcopy(RESPONSE)
        ambiguous["releases"].append(copy.deepcopy(ambiguous["releases"][0]))
        for options in ({"payload": {"id": DISC_ID, "releases": []}},
                        {"payload": ambiguous}, {"slow": True}, {"occupied": True}):
            with self.subTest(options=options):
                result, output, marker = self.run_capture(**options)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertTrue(output.name.startswith("cd-"), output)
                self.assertTrue(marker.exists())
                self.assertIn(str(output), result.stdout)
                if options.get("occupied"):
                    self.assertEqual((output.parent / "SQEX-11226" / "keep").read_bytes(), b"existing")

    def test_capture_failure_never_starts_naming_and_retains_generated_files(self):
        result, output, marker = self.run_capture(fail_stage="dump::extra")
        self.assertEqual(result.returncode, 23)
        self.assertTrue(output.name.startswith("cd-"))
        self.assertFalse(marker.exists())

    def test_unavailable_identity_never_contacts_provider(self):
        for options in ({"invalid_toc": True}, {"missing_tool": True}):
            with self.subTest(options=options):
                result, output, marker = self.run_capture(**options)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertTrue(output.name.startswith("cd-"))
                self.assertFalse(marker.exists())


if __name__ == "__main__":
    unittest.main()
