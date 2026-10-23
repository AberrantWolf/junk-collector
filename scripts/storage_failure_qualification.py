"""Shared actual metadata/browser assertions for bounded storage failures."""


class MetadataSaveFailureQualification:
    def __init__(self):
        self.item = None

    def exercise(self, cookie, item, request, verify, handoff):
        before = verify(cookie, item)
        metadata = request(f"/items/{item}/metadata", cookie=cookie)
        with self.unavailable():
            handoff(item, scenario=self.SCENARIO, desired_title=self.TITLE)
            assert request(f"/items/{item}", cookie=cookie)["title"] == before["title"]
            assert request(f"/items/{item}/metadata", cookie=cookie)["revision"] == metadata["revision"]
            verify(cookie, item)
        self.recover()
        handoff(item, scenario=self.RECOVERY_SCENARIO, desired_title=self.TITLE)
        assert request(f"/items/{item}", cookie=cookie)["title"] == self.TITLE
        updated = request(f"/items/{item}/metadata", cookie=cookie)
        assert updated["revision"] > metadata["revision"]
        verify(cookie, item)
        self.item = item
        print(f"{self.SCENARIO}: visible failed save, unchanged committed metadata/originals and successful browser retry passed", flush=True)

    def recover(self):
        # Permission recovery needs no restart; capacity recovery overrides this
        # to settle any interrupted journal replacement through normal startup.
        pass

    def verify_restored(self, cookie, request, verify):
        assert self.item
        assert request(f"/items/{self.item}", cookie=cookie)["title"] == self.TITLE
        metadata = request(f"/items/{self.item}/metadata", cookie=cookie)
        assert any(value["field"] == "title" and value["value"] == self.TITLE
                   for value in metadata["authority"]["field_overrides"])
        verify(cookie, self.item)
        print("Recovered manual title and exact originals survived full restore", flush=True)
