"""Real encoder interruption controls for a generated container qualification root.

The wrapper runs the shipped FLAC encoder. It can stall before consuming PCM or
fail after the real encoder emitted its output; neither mode invents success.
"""
import subprocess
import time
import urllib.error

DIRECTORY = "/data/state/qualification-encoder"
WRAPPER = b'''#!/bin/sh
set -eu
if [ "${1:-}" = "--version" ]; then exec /usr/bin/flac "$@"; fi
printf '%s' "$$" > /data/state/qualification-encoder/pid
mode=$(cat /data/state/qualification-encoder/mode)
if [ "$mode" = "block" ]; then exec /bin/sleep 900; fi
/usr/bin/flac "$@"
printf encoded > /data/state/qualification-encoder/output-complete
if [ "$mode" = "fail" ]; then
  printf 'qualification: encoder failed after output\\n' >&2
  exit 42
fi
'''


class PreparationFailureQualification:
    def __init__(self, run):
        self.run = run
        self.results = []

    def install(self):
        self.run("run", "--rm", "-T", "--entrypoint", "sh", "cli", "-c",
                 "mkdir -m 700 " + DIRECTORY + "; cat > " + DIRECTORY + "/flac; chmod 700 " + DIRECTORY + "/flac",
                 input=WRAPPER, capture_output=True)
        self.mode("pass")

    @staticmethod
    def configure(config):
        old = 'flac = "/usr/bin/flac"'
        if config.count(old) != 1:
            raise ValueError("Qualification expects the shipped FLAC configuration")
        return config.replace(old, 'flac = "' + DIRECTORY + '/flac"')

    def mode(self, value):
        if value not in ("pass", "block", "fail"):
            raise ValueError("Unknown encoder qualification mode")
        self.run("run", "--rm", "--entrypoint", "sh", "cli", "-c",
                 "rm -f " + DIRECTORY + "/pid " + DIRECTORY + "/output-complete; printf '%s' " + value + " > " + DIRECTORY + "/mode",
                 capture_output=True)

    @staticmethod
    def wait(read, accept, description, seconds=60):
        deadline = time.monotonic() + seconds
        while time.monotonic() < deadline:
            value = read()
            if accept(value):
                return value
            time.sleep(0.3)
        raise AssertionError(description + " did not settle")

    def entered(self):
        def read():
            try:
                return self.run("exec", "-T", "collector", "cat", DIRECTORY + "/pid",
                                capture_output=True, text=True).stdout.strip()
            except subprocess.CalledProcessError:
                return ""
        value = self.wait(read, lambda value: value.isdigit(), "Configured encoder process admission")
        return int(value)

    def no_process(self, identifier):
        # Direct encoder child must be terminated and reaped, not merely hidden by
        # a cancelled Job state. The retained container's /proc is authoritative.
        def reaped():
            try:
                self.run("exec", "-T", "collector", "sh", "-c", "test ! -e /proc/" + str(identifier), capture_output=True)
                return True
            except subprocess.CalledProcessError:
                return False
        self.wait(reaped, bool, "Cancelled encoder reaping", seconds=10)

    @staticmethod
    def derivatives(item):
        return [representation for capture in item["captures"] for representation in capture["representations"]
                if representation["role"] == "lossless_derivative"]

    def exercise(self, cookie, seed, request, verify, stop, up, ready, handoff):
        def batch(identifier):
            return request("/music-preparation-batches/" + identifier, cookie=cookie)

        def start(item):
            plan = request(f"/items/{item}/music-preparation-plan", cookie=cookie)
            sources = [source["source_representation_id"] for disc in plan["discs"]
                       for source in disc["sources"] if source["default_selected"]]
            assert len(sources) == 1 and plan["can_submit"], plan
            accepted = request(f"/items/{item}/music-preparation-batches", dict(source_representation_ids=sources), cookie)
            return accepted["batch_id"]

        def terminal(identifier, state):
            return self.wait(lambda: batch(identifier),
                             lambda value: len(value["members"]) == 1 and value[state] == 1,
                             "Preparation " + state)

        def success(item, identifier):
            settled = terminal(identifier, "succeeded")
            rows = request(f"/items/{item}/music-tracks", cookie=cookie)
            flac = [row for row in rows if (row.get("profile") or {}).get("codec") == "flac"]
            assert len(flac) == 1 and flac[0]["state"]["status"] == "available", rows
            assert len(self.derivatives(verify(cookie, item))) == 1, "Retry/restart duplicated published audio"
            self.results.append((item, identifier, flac[0]["state"]["representation_id"]))
            return settled

        # UI cancellation only starts after the actual blocking encoder exists.
        self.mode("block")
        item = seed("Cancelled preparation")
        identifier = start(item)
        process = self.entered()
        handoff(item, scenario="cancel-preparation", batch_id=identifier)
        cancelled = terminal(identifier, "cancelled")
        self.no_process(process)
        assert not self.derivatives(verify(cookie, item)), "Cancelled work published audio"
        child = cancelled["members"][0]["job_id"]
        stop()
        self.mode("pass")
        up()
        ready()
        assert batch(identifier)["cancelled_by_user"] and batch(identifier)["cancelled"] == 1
        assert not self.derivatives(verify(cookie, item)), "Restart resurrected cancelled work"
        try:
            request(f"/jobs/{child}/retry", {}, cookie)
        except urllib.error.HTTPError as error:
            assert error.code == 409, error.code
        else:
            raise AssertionError("Generic retry resurrected a user-cancelled batch child")
        handoff(item, scenario="retry-cancelled-preparation", batch_id=identifier)
        success(item, identifier)
        print("Encoder subprocess cancellation/reaping, durable cancellation, fenced generic retry and UI retry passed", flush=True)

        # This failure occurs after the actual FLAC encoder emitted valid bytes.
        self.mode("fail")
        item = seed("Failed encoder")
        identifier = start(item)
        failed = terminal(identifier, "failed")
        self.run("exec", "-T", "collector", "test", "-f", DIRECTORY + "/output-complete", capture_output=True)
        assert not self.derivatives(verify(cookie, item)), "Failed encoder published its output"
        child = failed["members"][0]["job_id"]
        before = request("/jobs/" + child, cookie=cookie)
        stop()
        self.mode("pass")
        up()
        ready()
        assert batch(identifier)["failed"] == 1
        handoff(item, scenario="retry-failed-preparation", batch_id=identifier)
        settled = success(item, identifier)
        assert settled["members"][0]["job_id"] == child
        after = request("/jobs/" + child, cookie=cookie)
        assert after["created_at"] == before["created_at"] and after["attempt"] > before["attempt"]
        print("Failure after real FLAC output stayed unpublished/visible across restart; UI retry preserved Job identity", flush=True)

        # Simulate a lost container during actual encoding, not an idle restart.
        self.mode("block")
        item = seed("Interrupted encoding")
        identifier = start(item)
        self.entered()
        active = batch(identifier)
        child = next(member["job_id"] for member in active["members"] if member["job_id"])
        before = request("/jobs/" + child, cookie=cookie)
        self.mode("pass")
        self.run("kill", "-s", "SIGKILL", "collector", capture_output=True)
        up()
        ready()
        settled = success(item, identifier)
        assert settled["members"][0]["job_id"] == child
        after = request("/jobs/" + child, cookie=cookie)
        # A reclaimed crash claim resumes the existing attempt; only explicit
        # retry adds an attempt. Preserve work identity without requiring a new one.
        assert after["created_at"] == before["created_at"]
        handoff(item, scenario="restarted-preparation", batch_id=identifier)
        print("SIGKILL during configured encoder execution recovered the original Job with one playable output", flush=True)

    def verify_restored(self, cookie, request, verify):
        for item, identifier, representation in self.results:
            assert len(self.derivatives(verify(cookie, item))) == 1
            assert request("/music-preparation-batches/" + identifier, cookie=cookie)["succeeded"] == 1
            rows = request(f"/items/{item}/music-tracks", cookie=cookie)
            assert any(row["state"].get("representation_id") == representation for row in rows)
        print("Recovered preparation outcomes, original bytes and immutable audio identities survived full restore", flush=True)
