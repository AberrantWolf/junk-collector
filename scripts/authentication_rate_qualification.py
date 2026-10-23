"""Bounded real-TCP login throttling on a generated Linux installation."""
import http.client
import json
import time
from urllib.parse import urlsplit


class AuthenticationRateQualification:
    def __init__(self, private, public, password):
        self.private, self.public, self.password = private, public, password
        self.deadline = 0

    def attempt(self, username, password, peer, alias, public=False):
        assert time.monotonic() < self.deadline, "Qualification exceeded the rate window"
        destination = urlsplit(self.public if public else self.private)
        connection = http.client.HTTPConnection(destination.hostname, destination.port,
                                                timeout=10, source_address=(peer, 0))
        try:
            connection.request("POST", "/api/v1/auth/login",
                               json.dumps(dict(username=username, password=password,
                                               device_name="Bounded rate qualification")),
                               {"Content-Type": "application/json", "Origin": self.private,
                                "X-Forwarded-For": f"203.0.113.{alias % 254 + 1}",
                                "Forwarded": f"for=198.51.100.{alias % 254 + 1}"})
            response = connection.getresponse()
            result = response.status, {key.lower(): value for key, value in response.getheaders()}, json.loads(response.read())
            assert time.monotonic() < self.deadline, "Qualification exceeded the rate window"
            return result
        finally:
            connection.close()

    @staticmethod
    def limited(result):
        status, headers, body = result
        assert status == 429 and headers["retry-after"] == "60"
        assert body["error"]["code"] == "rate_limited"

    def exercise(self, cookie, request, restart):
        owner = request("/me", cookie=cookie)["user_id"]

        def fresh():
            restart()
            assert request("/me", cookie=cookie)["user_id"] == owner
            self.deadline = time.monotonic() + 45

        fresh()
        # Ten attempts on one account from ten distinct actual TCP sources.
        for index in range(10):
            assert self.attempt("qualification-account", "wrong", f"127.0.0.{index+2}", index)[0] == 401
        self.limited(self.attempt("qualification-account", "wrong", "127.0.0.20", 10, public=True))
        assert self.attempt("different-account", "wrong", "127.0.0.21", 11)[0] == 401
        print("Account limit: ten real peers share the same account ceiling; an unrelated account reaches ordinary authentication failure", flush=True)

        fresh()
        assert self.attempt("owner", self.password, "127.0.0.2", 0)[0] == 200
        for index in range(1, 30):
            assert self.attempt(f"peer-account-{index}", "wrong", "127.0.0.2", index, public=bool(index % 2))[0] == 401
        for index in range(30, 62):
            self.limited(self.attempt(f"rejected-peer-account-{index}", "wrong", "127.0.0.2", index))
        assert self.attempt("owner", self.password, "127.0.0.3", 63, public=True)[0] == 200
        print("Peer limit: thirty admitted attempts share both listeners; account/forwarded-header rotation cannot bypass it; another actual peer works", flush=True)

        fresh()
        # Four true peers with 25 distinct accounts each stay below the peer and
        # account ceilings, so the next denial independently proves the global cap.
        for index in range(100):
            assert self.attempt(f"global-account-{index}", "wrong", f"127.0.0.{index//25+2}", index, public=bool(index % 2))[0] == 401
        for index in range(100, 200):
            self.limited(self.attempt(f"global-rejected-{index}", "wrong", f"127.0.0.{index-90}", index))
        assert request("/me", cookie=cookie)["user_id"] == owner
        print("Global limit: one hundred admitted attempts across four real peers; new real peers/accounts remain throttled while an existing session works", flush=True)

        fresh()
        assert self.attempt("owner", self.password, "127.0.0.2", 201)[0] == 200
        print("Process restart resets transient authentication limits and preserves the original session/account", flush=True)
