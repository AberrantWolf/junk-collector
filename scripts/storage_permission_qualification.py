"""Real permission-loss and recovery checks limited to the generated archive."""


class StoragePermissionQualification:
    TITLE = "Recovered storage permission test"

    def __init__(self, root_tool):
        self.root_tool = root_tool
        self.item = None

    def exercise(self, cookie, item, request, verify, handoff):
        before = verify(cookie, item)
        metadata = request(f"/items/{item}/metadata", cookie=cookie)
        # The known generated package has no colon/newline filenames. Preserve
        # each exact mode rather than imposing new archive permissions afterward.
        self.root_tool("sh", "-eu", "-c",
                       "find /data/archive -exec stat -c '%a:%n' {} + > /data/qualification-archive-modes; "
                       "chmod -R a-w /data/archive")
        try:
            handoff(item, scenario="readonly-archive", desired_title=self.TITLE)
            assert request(f"/items/{item}", cookie=cookie)["title"] == before["title"]
            assert request(f"/items/{item}/metadata", cookie=cookie)["revision"] == metadata["revision"]
            verify(cookie, item)
        finally:
            self.root_tool("sh", "-eu", "-c",
                           "while IFS=: read -r mode path; do chmod \"$mode\" -- \"$path\"; done < /data/qualification-archive-modes")
        handoff(item, scenario="recovered-archive", desired_title=self.TITLE)
        assert request(f"/items/{item}", cookie=cookie)["title"] == self.TITLE
        updated = request(f"/items/{item}/metadata", cookie=cookie)
        assert updated["revision"] > metadata["revision"]
        verify(cookie, item)
        self.item = item
        print("Archive permission loss: visible failed save, unchanged committed metadata/originals, restored permissions and successful browser retry passed", flush=True)

    def verify_restored(self, cookie, request, verify):
        assert self.item
        assert request(f"/items/{self.item}", cookie=cookie)["title"] == self.TITLE
        metadata = request(f"/items/{self.item}/metadata", cookie=cookie)
        assert any(value["field"] == "title" and value["value"] == self.TITLE
                   for value in metadata["authority"]["field_overrides"])
        verify(cookie, self.item)
        print("Recovered manual title and exact originals survived full restore", flush=True)
