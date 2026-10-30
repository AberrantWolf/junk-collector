"""Bounded non-root Linux cleanup drill inside the generated Compose installation."""
import time


def qualify(cookie, imported, request, verify_originals,
            originals, env, root_tool, read_file, up, ready):
    deadline = time.monotonic() + 60
    while time.monotonic() < deadline:
        detail = request(f"/ingests/{imported}", cookie=cookie)
        if detail["summary"]["permissions"]["can_clean_inbox"]:
            break
        time.sleep(0.3)
    assert detail["summary"]["permissions"]["can_clean_inbox"], detail
    item = detail["summary"]["target_item_id"]
    # Real group write permissions exist underneath the read-only bind. This
    # changes only the generated fixture, never an operator inbox.
    root_tool("chmod", "-R", "g+rwX,o-rwx", "/data/inbox-fixture")

    def cleanup():
        request(f"/ingests/{imported}/inbox-cleanup", dict(source_is_idle=True), cookie)
        deadline = time.monotonic() + 60
        while time.monotonic() < deadline:
            value = request(f"/ingests/{imported}", cookie=cookie)["inbox_cleanup"]
            if value and value["state"] in ("completed", "failed", "cancelled"):
                return value
            time.sleep(0.3)
        raise AssertionError("Inbox cleanup did not settle")

    refused = cleanup()
    assert refused["state"] == "failed" and refused["removed_bytes"] == 0, refused
    assert "writable" in refused["error"].lower(), refused
    for name, content in originals.items():
        assert read_file("/inbox/Reference disc/" + name) == content
    verify_originals(cookie, item, originals)
    print("Read-only inbox cleanup refused; source files and archive originals retained", flush=True)
    env["COLLECTOR_INBOX_READ_ONLY"] = "false"
    up(recreate=True)
    ready()
    completed = cleanup()
    assert completed["completed"] and completed["removed_bytes"] == completed["total_bytes"], completed
    root_tool("test", "!", "-e", "/data/inbox-fixture/Reference disc")
    verify_originals(cookie, item, originals)
    print("Explicit read/write inbox cleanup passed as the normal server UID; preserved originals remain downloadable", flush=True)
