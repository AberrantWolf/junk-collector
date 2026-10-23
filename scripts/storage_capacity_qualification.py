"""Real ENOSPC in a private 96 MiB tmpfs volume, never the host's payload disk."""
from contextlib import contextmanager
import json
from pathlib import Path
import secrets
import subprocess
import tempfile

from storage_failure_qualification import MetadataSaveFailureQualification


class StorageCapacityQualification(MetadataSaveFailureQualification):
    TITLE = "Recovered storage capacity test"
    SCENARIO = "full-archive"
    RECOVERY_SCENARIO = "recovered-full-archive"
    # Leave the normal 64 MiB preservation reserve intact during fixture setup.
    BYTES = 96 * 1024 * 1024

    def __init__(self, command, env, image, root_tool, restart):
        super().__init__()
        self.env, self.image, self.root_tool, self.restart = env, image, root_tool, restart
        self.resources = []
        # Initialization intentionally transfers /data to the service UID. Keep
        # Compose input outside that root so the host operator can still read it.
        self.configuration = tempfile.TemporaryDirectory(prefix="collector-capacity-compose-")
        override = Path(self.configuration.name) / "capacity.compose.json"
        mount = dict(type="volume", source="capacity_archive", target="/data/archive")
        override.write_text(json.dumps(dict(
            services={name: dict(volumes=[mount]) for name in ("collector", "cli", "initialize")},
            volumes=dict(capacity_archive=dict(external=True, name="${COLLECTOR_TEST_ARCHIVE_VOLUME}")))))
        command.extend(["-f", str(override)])

    def docker(self, *args):
        return subprocess.run(["docker", *args], check=True, capture_output=True, timeout=60)

    def new_archive(self):
        name = "collector-capacity-" + secrets.token_hex(8)
        keeper = name + "-keeper"
        self.docker("volume", "create", "--driver", "local", "--opt", "type=tmpfs",
                    "--opt", "device=tmpfs", "--opt", f"o=size={self.BYTES},nosuid,nodev",
                    "--label", "org.junk.qualification=storage-capacity", name)
        self.resources.append((name, keeper))
        # A retained, networkless sleeper keeps the tmpfs mounted across clean
        # Collector stops and CLI invocations. It performs no application work.
        self.docker("run", "-d", "--name", keeper, "--network", "none", "--read-only",
                    "--cap-drop", "ALL", "--security-opt", "no-new-privileges:true",
                    "--user", "10001:10001", "--mount", f"type=volume,src={name},dst=/hold",
                    "--entrypoint", "/bin/sleep", self.image, "1800")
        self.env["COLLECTOR_TEST_ARCHIVE_VOLUME"] = name

    @contextmanager
    def unavailable(self):
        geometry = self.root_tool("stat", "-f", "-c", "%T %S %b", "/data/archive").stdout.decode().split()
        assert len(geometry) == 3 and geometry[0] == "tmpfs", "Refusing capacity fault outside a tmpfs mount"
        assert int(geometry[1]) * int(geometry[2]) == self.BYTES, "Refusing a nonfixture filesystem size"
        try:
            try:
                # At most 104 MiB is attempted, with the mounted filesystem's
                # verified 96 MiB limit. This cannot fill the real NAS disk.
                self.root_tool("dd", "if=/dev/zero", "of=/data/archive/.qualification-full",
                               "bs=1M", "count=104", "status=none")
            except subprocess.CalledProcessError as error:
                assert b"No space left on device" in error.stderr, "Capacity fixture failed for another reason"
            else:
                raise AssertionError("Bounded capacity fixture did not reach ENOSPC")
            assert self.root_tool("stat", "-f", "-c", "%f", "/data/archive").stdout.strip() == b"0"
            yield
        finally:
            self.root_tool("rm", "-f", "/data/archive/.qualification-full")

    def recover(self):
        # Normal startup recognizes interrupted .json.tmp journal replacements.
        # Never delete application journals ourselves to make recovery succeed.
        self.restart()

    def cleanup(self):
        failures = []
        for name, keeper in reversed(self.resources):
            for args in (("rm", "-f", keeper), ("volume", "rm", name)):
                try:
                    self.docker(*args)
                except (subprocess.SubprocessError, OSError) as error:
                    failures.append(error)
        self.configuration.cleanup()
        if failures:
            raise RuntimeError("Generated storage-capacity resources need cleanup") from failures[0]
