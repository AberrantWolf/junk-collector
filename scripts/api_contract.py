#!/usr/bin/env python3
"""Generate checked API artifacts or fail when checked files drift."""

from __future__ import annotations

import argparse
import subprocess
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
OPENAPI = ROOT / "api" / "openapi.v1.json"
TYPESCRIPT = ROOT / "web" / "src" / "generated" / "api.ts"
OPENAPI_TYPESCRIPT = ROOT / "web" / "node_modules" / ".bin" / "openapi-typescript"


def generate(openapi: Path, typescript: Path) -> None:
    document = subprocess.check_output(
        [
            "cargo",
            "run",
            "--quiet",
            "--locked",
            "-p",
            "collector-server",
            "--bin",
            "generate-openapi",
        ],
        cwd=ROOT,
    )
    openapi.parent.mkdir(parents=True, exist_ok=True)
    openapi.write_bytes(document)
    typescript.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(
        [str(OPENAPI_TYPESCRIPT), str(openapi), "--output", str(typescript)],
        cwd=ROOT,
        check=True,
        stdout=subprocess.DEVNULL,
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    if args.write:
        generate(OPENAPI, TYPESCRIPT)
        return 0

    with tempfile.TemporaryDirectory(prefix="junk-collector-api-") as directory:
        temporary = Path(directory)
        generated_openapi = temporary / "openapi.v1.json"
        generated_typescript = temporary / "api.ts"
        generate(generated_openapi, generated_typescript)
        drift = []
        for checked, generated in (
            (OPENAPI, generated_openapi),
            (TYPESCRIPT, generated_typescript),
        ):
            if not checked.exists() or checked.read_bytes() != generated.read_bytes():
                drift.append(checked.relative_to(ROOT))
        if drift:
            names = ", ".join(map(str, drift))
            raise SystemExit(
                f"generated API contract drift: {names}; run scripts/api_contract.py --write"
            )
    print("OpenAPI and TypeScript contracts match annotated routes and DTOs.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
