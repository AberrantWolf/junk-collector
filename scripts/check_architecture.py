#!/usr/bin/env python3
"""Reject dependencies that bypass documented layer boundaries."""
import pathlib
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
if errors:
    raise SystemExit("\n".join(errors))
print("Architecture dependency checks passed.")
