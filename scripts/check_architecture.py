#!/usr/bin/env python3
"""Reject dependencies that bypass documented layer boundaries."""
import pathlib
import hashlib
import re
import tomllib

ROOT = pathlib.Path(__file__).resolve().parents[1]
ALLOWED = {
    "collector-core": set(),
    "collector-modules": {"collector-core"},
    "collector-app": {"collector-core", "collector-modules"},
    "collector-store": {"collector-core", "collector-app"},
    "collector-client": {"collector-core"},
    "collector-server": {"collector-core", "collector-modules", "collector-app", "collector-store", "collector-client"},
}
FORBIDDEN_IN_DOMAIN = {"tokio", "reqwest", "axum", "rusqlite", "sqlx", "slint", "egui", "eframe", "cpal"}
errors = []
for manifest in (ROOT / "crates").glob("*/Cargo.toml"):
    data = tomllib.loads(manifest.read_text())
    name = data["package"]["name"]
    if name not in ALLOWED:
        errors.append(f"{name}: document and register new architectural layer before adding it")
        continue
    dependencies = data.get("dependencies", {})
    for dependency in dependencies:
        if dependency.startswith("collector-") and dependency not in ALLOWED[name]:
            errors.append(f"{name} -> {dependency}: dependency bypasses the architecture")
        if name in {"collector-core", "collector-modules"} and dependency in FORBIDDEN_IN_DOMAIN:
            errors.append(f"{name} -> {dependency}: domain code cannot own infrastructure")

for source in [
    ROOT / "crates/collector-server/src/lib.rs",
    ROOT / "crates/collector-server/src/passkeys.rs",
]:
    if re.search(r"\bstate\.store\b", source.read_text()):
        errors.append(
            f"{source.relative_to(ROOT)}: HTTP policy must call application services, not Store"
        )

# Check concrete document links. Prose coverage and security decisions require
# review; keyword matching cannot establish that a design is complete.
for document in [ROOT / "README.md", *(ROOT / "docs").rglob("*.md")]:
    text = document.read_text()
    for target in re.findall(r"\[[^]]+\]\(([^)]+)\)", text):
        target = target.split("#", 1)[0]
        if not target or "://" in target or target.startswith(("#", "mailto:")):
            continue
        if not (document.parent / target).resolve().exists():
            errors.append(f"{document.relative_to(ROOT)}: missing link target {target}")

fixture = ROOT / "crates/collector-store/tests/fixtures/schema-1"
for manifest in fixture.rglob("manifest.toml"):
    data = tomllib.loads(manifest.read_text())
    record = manifest.with_name("record.json")
    if not record.exists():
        errors.append(f"{manifest.relative_to(ROOT)}: fixture record.json is missing")
        continue
    actual = hashlib.sha256(record.read_bytes()).hexdigest()
    if data.get("record_sha256") != actual:
        errors.append(f"{manifest.relative_to(ROOT)}: fixture record digest mismatch")
if errors:
    raise SystemExit("\n".join(errors))
print("Architecture, documentation link, and fixture manifest checks passed.")
