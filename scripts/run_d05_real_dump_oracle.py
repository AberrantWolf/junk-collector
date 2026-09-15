#!/usr/bin/env python3
"""Run the opt-in D05 oracle without inspecting or printing owned dump paths."""

import os
import pathlib
import subprocess
import sys

root = os.environ.get("JUNK_COLLECTOR_W4_CD_DUMP_ROOT")
if not root:
    print("D05 real-dump qualification unavailable: JUNK_COLLECTOR_W4_CD_DUMP_ROOT is unset.")
    sys.exit(2)
oracle = pathlib.Path(root) / "oracle.toml"
if not oracle.is_file():
    print("D05 real-dump qualification unavailable: oracle.toml is missing from the supplied root.")
    sys.exit(2)
result = subprocess.run(
    [
        "cargo",
        "test",
        "-p",
        "collector-modules",
        "--test",
        "d05_real_dump_oracle",
        "--",
        "--ignored",
        "--exact",
        "owned_cd_dump_matches_declared_layout_and_accuraterip_oracles",
    ],
    check=False,
)
sys.exit(result.returncode)
