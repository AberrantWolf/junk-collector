"""Disposable Portainer lifecycle adapter for the real container recovery journey.

Only the generated qualification project is managed. The operator's Portainer
database, credentials, stacks and registry configuration are never opened.
"""
import json
import os
from pathlib import Path
import re
import secrets
import subprocess
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request


class PortainerQualification:
    def __init__(self, image, compose, env, port, keep_companion=False, companion_image=None):
        self.image, self.compose, self.env = image, compose, env
        self.project = compose[compose.index("-p") + 1]
        if not re.fullmatch(r"collector-smoke-[0-9]+-[0-9a-f]{8}", self.project):
            raise ValueError("Portainer may only manage a generated qualification project")
        self.name = self.project + "-portainer"
        self.volume = self.name + "-data"
        self.origin = "http://127.0.0.1:" + str(port)
        self.port = port
        self.token = None
        self.endpoint = None
        self.stack = None
        self.volume_owned = False
        self.container_owned = False
        self.keep_companion = keep_companion
        self.companion_image = companion_image
        self.companion_id = None
        self.password_path = None

    @staticmethod
    def docker(*args):
        return subprocess.check_output(["docker", *args], timeout=120, text=True).strip()

    def request(self, path, body=None, method=None, form=False, stream=False):
        headers = {}
        if self.token:
            headers["Authorization"] = "Bearer " + self.token
        if body is not None:
            if form:
                body = urllib.parse.urlencode(body).encode()
                headers["Content-Type"] = "application/x-www-form-urlencoded"
            else:
                body = json.dumps(body).encode()
                headers["Content-Type"] = "application/json"
        request = urllib.request.Request(self.origin + "/api" + path, data=body, headers=headers, method=method)
        with urllib.request.urlopen(request, timeout=300) as response:
            if stream:
                # Docker image pulls can return HTTP 200 with an error in their JSON stream.
                for line in response:
                    if line.strip():
                        event = json.loads(line)
                        if event.get("error") or event.get("errorDetail"):
                            raise RuntimeError("Portainer image pull failed: " + str(event.get("error")))
                return None
            content = response.read()
            return json.loads(content) if content else None

    def start(self):
        # Fail on collisions, rather than adopting another test's resources.
        existing = subprocess.run(["docker", "volume", "inspect", self.volume], capture_output=True, timeout=30)
        if existing.returncode == 0:
            raise ValueError("Qualification volume already exists")
        self.docker("volume", "create", "--label", "org.junk.qualification=" + self.project, self.volume)
        self.volume_owned = True
        password = secrets.token_hex(24)
        descriptor, password_path = tempfile.mkstemp(prefix="collector-portainer-password-")
        self.password_path = Path(password_path)
        with os.fdopen(descriptor, "w") as output:
            output.write(password)
        self.docker("run", "-d", "--name", self.name,
                    "--label", "org.junk.qualification=" + self.project,
                    "-p", f"127.0.0.1:{self.port}:9000",
                    "-v", self.volume + ":/data",
                    "-v", str(self.password_path) + ":/run/qualification-password:ro",
                    "-v", "/var/run/docker.sock:/var/run/docker.sock",
                    self.image, "--http-enabled", "--admin-password-file", "/run/qualification-password")
        self.container_owned = True
        deadline = time.monotonic() + 45
        while True:
            try:
                status = self.request("/status")
                break
            except (OSError, urllib.error.URLError):
                if time.monotonic() >= deadline:
                    raise AssertionError("Disposable Portainer did not become ready")
                time.sleep(0.3)
        self.token = self.request("/auth", dict(Username="admin", Password=password))["jwt"]
        self.password_path.unlink()
        self.endpoint = self.request("/endpoints", dict(Name=self.project, EndpointCreationType=1), form=True)["Id"]
        if any(registry.get("Authentication") for registry in self.request("/registries")):
            raise AssertionError("Disposable Portainer unexpectedly has registry credentials")
        print("Disposable Portainer " + status["Version"] + " initialized on loopback; no registry credentials", flush=True)

    def pull(self, image):
        if not image.startswith("ghcr.io/aberrantwolf/junk-collector"):
            raise ValueError("Portainer publication qualification requires the actual public Collector image")
        query = urllib.parse.urlencode(dict(fromImage=image))
        self.request(f"/endpoints/{self.endpoint}/docker/images/create?{query}", method="POST", stream=True)
        print("Portainer anonymous image pull passed: " + image, flush=True)

    def deploy(self):
        rendered = json.loads(subprocess.check_output(self.compose + ["config", "--format", "json"], env=self.env, text=True, timeout=30))
        service = rendered["services"]["collector"]
        public = service["image"].startswith("ghcr.io/")
        # Existing local starting images must not be resolved against Docker Hub.
        service["pull_policy"] = "always" if public else "never"
        services = dict(collector=service)
        if self.keep_companion:
            # A control process, not a simulated proxy/tunnel. It has no data mounts
            # or network and proves that unrelated service identity/start time survive.
            companion_image = self.docker("image", "inspect", "--format", "{{.Id}}", self.companion_image)
            services["companion"] = dict(image=companion_image, entrypoint=["sleep", "infinity"],
                                         network_mode="none", user="10001:10001", read_only=True,
                                         cap_drop=["ALL"], security_opt=["no-new-privileges:true"], pull_policy="never")
        content = json.dumps(dict(services=services))
        repull = public and not self.keep_companion
        if self.stack is None:
            result = self.request(f"/stacks/create/standalone/string?endpointId={self.endpoint}",
                                  dict(Name=self.project, StackFileContent=content, Env=[]))
            self.stack = result["Id"]
        else:
            self.request(f"/stacks/{self.stack}?endpointId={self.endpoint}",
                         dict(StackFileContent=content, Env=[], RepullImageAndRedeploy=repull, Prune=False), method="PUT")
        self.wait(1)
        if self.keep_companion:
            identifier = self.docker("ps", "-q", "--no-trunc", "--filter", "label=com.docker.compose.project=" + self.project,
                                     "--filter", "label=com.docker.compose.service=companion")
            if not identifier:
                raise AssertionError("Unchanged companion is not running")
            state = json.loads(self.docker("inspect", identifier))[0]["State"]
            current = (identifier, state["StartedAt"])
            if self.companion_id is not None and current != self.companion_id:
                raise AssertionError("Service-targeted update restarted the unchanged companion")
            self.companion_id = current
            print("Unchanged companion container/start time retained", flush=True)
        action = "re-pull and redeploy" if repull else "service-targeted update" if public else "retained local image"
        print("Portainer stack deploy passed: " + service["image"] + " (" + action + ")", flush=True)

    def wait(self, expected):
        deadline = time.monotonic() + 180
        while time.monotonic() < deadline:
            result = self.request(f"/stacks/{self.stack}")
            if result["Status"] == expected:
                return
            # Portainer 2.45 deploys asynchronously. Do not mistake its HTTP 200 for success.
            if result["Status"] not in (1, 2, 3):
                history = result.get("DeploymentStatus", [])
                raise AssertionError("Portainer deployment failed: " + str(history[-1].get("Message", result["Status"]) if history else result["Status"]))
            time.sleep(0.5)
        raise AssertionError("Portainer stack operation did not settle")

    def stop(self):
        identifier = subprocess.check_output(self.compose + ["ps", "-aq", "collector"],
                                             env=self.env, text=True, timeout=30).strip()
        if not identifier:
            raise AssertionError("Qualification Collector container is missing")
        # Stop only Collector, as in Portainer's container view. Stack stop removes
        # containers and cannot supply the retained clean-exit state checked by the journey.
        self.request(f"/endpoints/{self.endpoint}/docker/containers/{identifier}/stop?t=10", method="POST")

    def cleanup(self):
        try:
            if self.stack is not None:
                self.request(f"/stacks/{self.stack}?endpointId={self.endpoint}", method="DELETE")
        finally:
            try:
                if self.container_owned:
                    subprocess.run(["docker", "rm", "-f", self.name], capture_output=True, timeout=30, check=True)
            finally:
                try:
                    if self.volume_owned:
                        subprocess.run(["docker", "volume", "rm", self.volume], capture_output=True, timeout=30, check=True)
                finally:
                    if self.password_path:
                        self.password_path.unlink(missing_ok=True)
