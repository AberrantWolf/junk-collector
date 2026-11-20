#!/usr/bin/env python3
"""Reject dependencies that bypass documented layer boundaries."""
import pathlib
import re
import tomllib

ROOT = pathlib.Path(__file__).resolve().parents[1]
ALLOWED = {
    "junk-libs-pcm": set(),
    "collector-cd-identity": set(),
    "collector-core": set(),
    "collector-api-types": {"collector-core"},
    "collector-media": {"collector-core", "collector-api-types"},
    "collector-modules": {"collector-core"},
    "collector-app": {"collector-core", "collector-modules", "collector-api-types"},
    "collector-providers": {"collector-core", "collector-api-types", "collector-app", "collector-modules"},
    "collector-store": {"collector-core", "collector-app"},
    "collector-client": {"collector-core", "collector-api-types"},
    "collector-server": {"collector-core", "collector-api-types", "collector-modules", "collector-app", "collector-providers", "collector-store", "collector-client", "collector-media"},
}
FORBIDDEN_IN_DOMAIN = {"tokio", "reqwest", "axum", "rusqlite", "sqlx", "slint", "egui", "eframe", "cpal"}
errors = []
for manifest in (ROOT / "crates").glob("*/Cargo.toml"):
    data = tomllib.loads(manifest.read_text())
    name = data["package"]["name"]
    if name not in ALLOWED:
        errors.append(f"{name}: add the new crate to ALLOWED in scripts/check_architecture.py")
        continue
    dependencies = data.get("dependencies", {})
    for dependency in dependencies:
        if dependency.startswith("collector-") and dependency not in ALLOWED[name]:
            errors.append(f"{name} -> {dependency}: dependency bypasses the architecture")
        if name in {"collector-core", "collector-modules"} and dependency in FORBIDDEN_IN_DOMAIN:
            errors.append(f"{name} -> {dependency}: domain code cannot own infrastructure")

for source in (ROOT / "crates/collector-server/src").rglob("*.rs"):
    if re.search(r"\bstate\.store\b", source.read_text()):
        errors.append(
            f"{source.relative_to(ROOT)}: HTTP policy must call application services, not Store"
        )

# Check relative links in Markdown docs.
for document in [ROOT / "README.md", ROOT / "AGENTS.md", *(ROOT / "docs").rglob("*.md")]:
    text = document.read_text()
    for target in re.findall(r"\[[^]]+\]\(([^)]+)\)", text):
        target = target.split("#", 1)[0]
        if not target or "://" in target or target.startswith(("#", "mailto:")):
            continue
        if not (document.parent / target).resolve().exists():
            errors.append(f"{document.relative_to(ROOT)}: missing link target {target}")

if errors:
    raise SystemExit("\n".join(errors))
print("Architecture and documentation link checks passed.")
