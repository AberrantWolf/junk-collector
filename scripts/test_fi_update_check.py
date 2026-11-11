"""Offline checks of retained application state and post-update refusal paths."""

from contextlib import closing, redirect_stderr
from http.server import BaseHTTPRequestHandler, HTTPServer
import io
import json
from pathlib import Path
import shutil
import sqlite3
import tempfile
import threading
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import fi_update_check as check


MAIL_CONNECTION = {
    "profile": {"id": "mail", "owner": "owner", "folders": ["INBOX"]},
    "collection_id": "music",
    "enabled": True,
    "revision": 2,
    "folders": ["INBOX"],
    "cursor": "old",
    "last_checked": "2026-11-11T12:00:00Z",
    "problem": None,
    "failure": None,
    "progress": [],
}


class FiUpdateTests(unittest.TestCase):
    def test_stopped_cutoff_reads_committed_wal_without_modifying_snapshot(self):
        with tempfile.TemporaryDirectory() as temporary, closing(sqlite3.connect(Path(temporary) / "private.sqlite")) as db:
            live = Path(temporary)
            db.executescript("""
                PRAGMA journal_mode=WAL;
                PRAGMA wal_autocheckpoint=0;
                CREATE TABLE jobs(state);
                CREATE TABLE imports(id, revision, phase);
                CREATE TABLE users(id, administrator, enabled);
                CREATE TABLE grants(collection_id, user_id, role);
                CREATE TABLE mail_connections(id, owner, record);
                INSERT INTO jobs VALUES ('completed');
                INSERT INTO imports VALUES ('saved', 2, 'published'), ('draft', 3, 'awaiting_review');
                INSERT INTO users VALUES ('owner', 1, 1);
                INSERT INTO grants VALUES ('music', 'owner', 'owner');
            """)
            db.execute("INSERT INTO mail_connections VALUES ('mail', 'owner', ?)", (json.dumps(MAIL_CONNECTION),))
            db.commit()
            with closing(sqlite3.connect(live / "catalog.sqlite")) as catalog:
                catalog.executescript("CREATE TABLE items(id); INSERT INTO items VALUES ('item');")
            (live / "archive-binding").write_bytes(b"binding")
            (live / "tls").mkdir()
            (live / "tls/cert.pem").write_bytes(b"certificate")
            cutoff = live / "snapshot"
            cutoff.mkdir()
            for name in ("private.sqlite", "private.sqlite-wal", "catalog.sqlite", "archive-binding"):
                shutil.copyfile(live / name, cutoff / name)
            shutil.copytree(live / "tls", cutoff / "tls")
            before = (cutoff / "private.sqlite-wal").read_bytes()
            old = check.state_facts(cutoff, stopped=True)
            self.assertEqual(old, check.state_facts(live))
            self.assertEqual(old["grants"], [("music", "owner", "owner")])
            self.assertEqual(old["staged_imports"], [("draft", 3, "awaiting_review")])
            self.assertEqual((cutoff / "private.sqlite-wal").read_bytes(), before)
            self.assertFalse((cutoff / "private.sqlite-shm").exists())
            config = live / "collector.toml"
            config.write_text("fixture")
            saved = check.fingerprints(old, config)
            advanced = dict(MAIL_CONNECTION, cursor="next", last_checked="2026-11-11T12:05:00Z",
                            progress=[{"folder": "INBOX", "scanned_through": 5}])
            db.execute("UPDATE mail_connections SET record=?", (json.dumps(advanced),))
            db.commit()
            self.assertEqual(check.fingerprints(check.state_facts(live), config), saved)
            for change in ({"enabled": False}, {"folders": ["Orders"]}, {"collection_id": "another"},
                           {"profile": dict(advanced["profile"], owner="another")}, {"revision": 3}):
                with self.subTest(change=change):
                    db.execute("UPDATE mail_connections SET record=?", (json.dumps(dict(advanced, **change)),))
                    db.commit()
                    self.assertNotEqual(check.fingerprints(check.state_facts(live), config), saved)
            db.execute("DELETE FROM grants")
            db.commit()
            self.assertNotEqual(old["grants"], check.state_facts(live)["grants"])
            db.execute("INSERT INTO imports VALUES ('running', 1, 'publishing')")
            db.commit()
            with self.assertRaisesRegex(ValueError, "Active imports"):
                check.state_facts(live)
            db.execute("DELETE FROM imports WHERE phase = 'publishing'")
            db.execute("INSERT INTO jobs VALUES ('cancel_requested')")
            db.commit()
            with self.assertRaisesRegex(ValueError, "Unfinished Jobs"):
                check.state_facts(live)

    def test_receipt_scanner_requires_a_fresh_success_for_each_enabled_connection(self):
        with tempfile.TemporaryDirectory() as temporary:
            state = Path(temporary)
            with closing(sqlite3.connect(state / "private.sqlite")) as db:
                db.execute("CREATE TABLE mail_connections(id, owner, record)")
                db.executemany("INSERT INTO mail_connections VALUES (?, 'owner', ?)", [
                    ("mail", json.dumps(MAIL_CONNECTION)),
                    ("other", json.dumps(dict(MAIL_CONNECTION, last_checked="2026-11-11T12:06:00Z"))),
                    ("disabled", json.dumps(dict(MAIL_CONNECTION, enabled=False, problem="old failure"))),
                ])
                db.commit()
                started = "2026-11-11T12:05:00.123456789Z"
                with self.assertRaisesRegex(ValueError, "waiting for a successful post-start check"):
                    check.wait_for_receipt_scanner(state, started, timeout=0)
                fresh = dict(MAIL_CONNECTION, last_checked="2026-11-11T12:06:00Z")
                broken = dict(fresh, problem="private diagnostic", failure={"kind": "transport"})
                db.execute("UPDATE mail_connections SET record=? WHERE id='mail'", (json.dumps(broken),))
                db.commit()
                with self.assertRaisesRegex(ValueError, "reported a problem") as error:
                    check.wait_for_receipt_scanner(state, started, timeout=0)
                self.assertNotIn("private diagnostic", str(error.exception))
                def complete_scan(_):
                    db.execute("UPDATE mail_connections SET record=? WHERE id='mail'", (json.dumps(fresh),))
                    db.commit()

                with patch.object(check.time, "sleep", side_effect=complete_scan):
                    self.assertIn("2 enabled", check.wait_for_receipt_scanner(state, started, timeout=10))
                db.execute("DELETE FROM mail_connections WHERE id != 'disabled'")
                db.commit()
                self.assertIn("no enabled", check.wait_for_receipt_scanner(state, started, timeout=0))

    def test_target_pin_and_companion_recreation_are_checked(self):
        old = {"id": "before", "image": "old", "reference": "old-ref", "mounts": []}
        image = {"id": "verified", "reference": "verified-digest"}
        baseline = {"collector": old, "media": old, "proton-bridge": old}
        updated = dict(old, id="replacement", image=image["id"], reference=image["reference"])
        live = {"collector": updated, "media": updated, "proton-bridge": old}
        check.compare_stack(baseline, live, image)
        with self.assertRaisesRegex(ValueError, "verified image by digest"):
            check.compare_stack(baseline, dict(live, media=dict(updated, reference="preview")), image)
        with self.assertRaisesRegex(ValueError, "mount bindings changed"):
            check.compare_stack(baseline, dict(live, collector=dict(updated, mounts=["different"])), image)
        with self.assertRaisesRegex(ValueError, "recreated"):
            check.compare_stack(baseline, dict(live, **{"proton-bridge": dict(old, id="replacement")}), image)

    def test_before_rejects_an_image_with_the_wrong_published_source(self):
        digest = "sha256:" + "a" * 64
        image = "ghcr.io/example/collector"
        record = dict(image=image, digest=digest, reference=image + "@" + digest, version="0.1.0-beta.23",
                      revision="b" * 40, source="https://github.com/example/collector",
                      channel="preview", anonymous_docker_pulls=True)
        labels = {"org.opencontainers.image." + field: record[field] for field in ("version", "revision", "source")}
        local = dict(Id="sha256:" + "c" * 64, RepoDigests=[record["reference"]],
                     Os="linux", Architecture="amd64", Config={"Labels": labels})
        with tempfile.TemporaryDirectory() as temporary, patch.object(check, "docker_json", return_value=[local]):
            path = Path(temporary) / "release.json"
            path.write_text(json.dumps(record))
            self.assertEqual(check.target(path)["id"], local["Id"])
            labels["org.opencontainers.image.revision"] = "d" * 40
            with self.assertRaisesRegex(ValueError, "does not match"):
                check.target(path)

    def test_http_client_refuses_to_forward_session_on_redirect(self):
        requests = []

        class Handler(BaseHTTPRequestHandler):
            def do_GET(self):
                requests.append((self.path, self.headers.get("Cookie")))
                self.send_response(302 if self.path == "/api/v1/me" else 200)
                self.send_header("Location", "/other")
                self.end_headers()

            def log_message(self, *_):
                pass

        # Exercise urllib's real redirect handling on a local fixture. The
        # operator CLI independently requires HTTPS for all production origins.
        with HTTPServer(("127.0.0.1", 0), Handler) as server:
            worker = threading.Thread(target=server.serve_forever)
            worker.start()
            try:
                with self.assertRaisesRegex(ValueError, "HTTP 302"):
                    check.get(f"http://127.0.0.1:{server.server_port}", "/api/v1/me", "fixture")
            finally:
                server.shutdown()
                worker.join(timeout=3)
        self.assertEqual(requests, [("/api/v1/me", "collector_session=fixture")])
        with self.assertRaisesRegex(ValueError, "HTTPS"):
            check.origin("http://127.0.0.1")

    def test_bridge_must_be_synced_and_fresh(self):
        with tempfile.TemporaryDirectory() as temporary:
            status = Path(temporary) / "status.json"
            report = {"schema": 1, "observed_at": 100, "event_at": 95, "phase": "syncing"}
            status.write_text(json.dumps(report))
            self.assertNotEqual(check.bridge_status(status, 100, require_synced=True)[1], 0)
            status.write_text(json.dumps(dict(report, phase="synced")))
            self.assertEqual(check.bridge_status(status, 100, require_synced=True)[1], 0)
            self.assertNotEqual(check.bridge_status(status, 121, require_synced=True)[1], 0)
            status.write_text(json.dumps(dict(report, phase="interrupted")))
            self.assertEqual(check.bridge_status(status, 100, require_synced=True)[1], check.SYNC_INTERRUPTED)

    def test_bridge_wait_allows_startup_but_distinguishes_timeout_and_failure(self):
        path = Path("report.json")
        with patch.object(check, "bridge_status", side_effect=[("syncing", 2), ("synced", 0)]), \
             patch.object(check.time, "monotonic", side_effect=[0, 1]), patch.object(check.time, "sleep"):
            self.assertEqual(check.wait_for_bridge(path, 10), "synced")
        with patch.object(check, "bridge_status", return_value=("syncing", 2)), \
             patch.object(check.time, "monotonic", side_effect=[0, 10]):
            with self.assertRaisesRegex(ValueError, "Timed out"):
                check.wait_for_bridge(path, 10)
        with patch.object(check, "bridge_status", return_value=("Interrupted", check.SYNC_INTERRUPTED)):
            with self.assertRaisesRegex(ValueError, "^Interrupted$"):
                check.wait_for_bridge(path, 10)

    def test_cutoff_requires_stopped_writers_and_clean_snapshot_before_sealing(self):
        with tempfile.TemporaryDirectory() as temporary:
            pair = Path(temporary).resolve()
            snapshot = pair / "fi-data"
            (snapshot / "state").mkdir(parents=True)
            (snapshot / "config").mkdir()
            (snapshot / "config/collector.toml").write_text("fixture")
            args = SimpleNamespace(nas_snapshot_uuid="nas", project="fi", pair_dir=pair, state_in_snapshot=Path("state"))
            service = {"Id": "old", "Image": "old-image", "Config": {"Image": "pin"},
                       "State": {"Running": True, "Restarting": False},
                       "Mounts": [{"Source": str(pair / "live"), "Destination": "/data/state", "Type": "bind", "RW": True}]}
            services = {"collector": service}
            before = {"services": check.container_facts(services)}
            with patch.object(check, "stack", return_value=services), \
                 patch.object(check.subprocess, "run", return_value=SimpleNamespace(stdout="inactive\n")), \
                 patch.object(check, "snapshot_identity", return_value=(snapshot, "fi")), \
                 patch.object(check, "state_facts", side_effect=ValueError("Unfinished Jobs")) as facts:
                with self.assertRaisesRegex(ValueError, "Stop all Compose services"):
                    check.seal_cutoff(args, {}, before)
                facts.assert_not_called()
                service["State"]["Running"] = False
                with self.assertRaisesRegex(ValueError, "Unfinished Jobs"):
                    check.seal_cutoff(args, {}, before)
                self.assertFalse((pair / "cutoff.json").exists())
                facts.side_effect = None
                facts.return_value = {"items": 1}
                check.seal_cutoff(args, {}, before)
                facts.assert_called_with(snapshot / "state", stopped=True)
                receipt = json.loads((pair / "cutoff.json").read_text())
                self.assertEqual(receipt["fi_snapshot_uuid"], "fi")
                self.assertEqual(receipt["nas_snapshot_uuid"], "nas")

    def test_after_refuses_a_changed_public_origin_before_asking_for_session(self):
        item = "00000000-0000-4000-8000-000000000001"
        image = {"reference": "verified"}
        origins = {"public": "https://before.example", "lan": "https://lan.example", "media": "https://media.example"}
        with tempfile.TemporaryDirectory() as temporary:
            pair = Path(temporary)
            baseline = dict(project="fi", target=image, origins=origins, item=item)
            (pair / "before.json").write_text(json.dumps({"request": baseline}))
            argv = ["fi_update_check.py", "after", "--project", "fi", "--release", "release.json",
                    "--pair-dir", str(pair), "--public", "https://changed.example",
                    "--lan", origins["lan"], "--media", origins["media"], "--item", item]
            with patch("sys.argv", argv), patch("sys.stdin.isatty", return_value=True), \
                 patch.object(check, "target", return_value=image), patch.object(check, "getpass") as password, \
                 redirect_stderr(io.StringIO()) as errors, self.assertRaises(SystemExit):
                check.main()
            self.assertIn("origins", errors.getvalue())
            password.assert_not_called()


if __name__ == "__main__":
    unittest.main()
