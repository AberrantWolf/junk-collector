"""Real permission-loss and recovery checks limited to the generated archive."""
from contextlib import contextmanager
from storage_failure_qualification import MetadataSaveFailureQualification


class StoragePermissionQualification(MetadataSaveFailureQualification):
    TITLE = "Recovered storage permission test"
    SCENARIO = "readonly-archive"
    RECOVERY_SCENARIO = "recovered-archive"

    def __init__(self, root_tool):
        super().__init__()
        self.root_tool = root_tool

    @contextmanager
    def unavailable(self):
        # The known generated package has no colon/newline filenames. Preserve
        # each exact mode rather than imposing new archive permissions afterward.
        self.root_tool("sh", "-eu", "-c",
                       "find /data/archive -exec stat -c '%a:%n' {} + > /data/qualification-archive-modes")
        try:
            self.root_tool("chmod", "-R", "a-w", "/data/archive")
            yield
        finally:
            self.root_tool("sh", "-eu", "-c",
                           "while IFS=: read -r mode path; do chmod \"$mode\" -- \"$path\"; done < /data/qualification-archive-modes")
