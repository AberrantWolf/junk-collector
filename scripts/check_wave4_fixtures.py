#!/usr/bin/env python3
"""Reproduce Wave 4 recipe bytes; this does not qualify format/provider semantics."""
import argparse
import hashlib
from pathlib import Path
import re
import tomllib
import zlib

ROOTS = {"D04", "D05", "D06", "D07", "P05", "C01", "S04", "C02"}
MAX_BYTES = 16 * 1024 * 1024


def require(condition, message):
    if not condition:
        raise ValueError(message)


def formula_bytes(formula, length):
    match = re.fullmatch(r"\(i \* (\d+) \+ (\d+)\) % 256", formula)
    require(match is not None, f"unsupported formula: {formula!r}")
    require(0 <= length <= MAX_BYTES, "recipe length exceeds fixture bound")
    multiplier, addend = map(int, match.groups())
    return bytes((i * multiplier + addend) % 256 for i in range(length))


def described_bytes(recipe):
    match = re.fullmatch(r"(\d+) bytes where byte i is (.+)", recipe)
    require(match is not None, f"unsupported composite recipe: {recipe!r}")
    return formula_bytes(match[2], int(match[1]))


def vector_bytes(vector):
    length = vector["length"]
    require(0 <= length <= MAX_BYTES, "recipe length exceeds fixture bound")
    match vector["recipe"]:
        case "hex":
            return bytes.fromhex(vector["hex"])
        case "utf8":
            return vector["utf8"].encode("utf-8")
        case "byte_formula":
            return formula_bytes(vector["formula"], length)
        case "repeat_byte":
            return bytes([vector["byte"]]) * length
        case _:
            raise ValueError(f"unsupported recipe: {vector['recipe']}")


def check_bytes(data, record, prefix=""):
    require(len(data) == record[prefix + "length"], f"{prefix}length mismatch")
    require(hashlib.sha256(data).hexdigest() == record[prefix + "sha256"],
            f"{prefix}SHA-256 mismatch")
    if "expected_sha1" in record:
        require(hashlib.sha1(data).hexdigest() == record["expected_sha1"], "SHA-1 mismatch")
    if "expected_crc32" in record:
        require(f"{zlib.crc32(data):08x}" == record["expected_crc32"], "CRC32 mismatch")


def check_manifest(path):
    manifest = tomllib.loads(path.read_text())
    require(manifest["schema"] == 1, "unsupported manifest schema")
    require(manifest["root"] == path.parent.name.upper(), "root/directory mismatch")
    require(bool(manifest["license"]), "missing provenance/license description")
    vectors = {}
    count = 0
    for vector in manifest.get("vectors", []):
        name = vector["id"]
        require(name not in vectors, f"duplicate vector: {name}")
        data = vector_bytes(vector)
        try:
            check_bytes(data, vector)
        except ValueError as error:
            raise ValueError(f"{name}: {error}") from error
        vectors[name] = data
        count += 1
    for name, record in manifest.get("composites", {}).items():
        if name == "ines":
            payload = vectors[record["payload_vector"]]
            check_bytes(bytes.fromhex(record["header_hex"]) + payload, record, "plain_file_")
            check_bytes(bytes.fromhex(record["trainer_header_hex"])
                        + described_bytes(record["trainer_recipe"]) + payload,
                        record, "trainer_file_")
            count += 2
        elif name in {"snes", "pce"}:
            check_bytes(described_bytes(record["header_recipe"])
                        + vectors[record["payload_vector"]], record, "headered_file_")
            count += 1
        elif name == "cue":
            check_bytes(b"".join(vectors[key] for key in record["ordered_vectors"]),
                        record, "combined_")
            count += 1
        else:
            raise ValueError(f"unsupported composite: {name}")
    return manifest["root"], count


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path,
                        default=Path(__file__).resolve().parents[1] / "fixtures/wave4")
    args = parser.parse_args()
    roots, count = set(), 0
    for path in sorted(args.root.glob("*/manifest.toml")):
        try:
            root, checked = check_manifest(path)
        except (ValueError, KeyError, TypeError) as error:
            raise SystemExit(f"{path}: {error}") from error
        require(root not in roots, f"duplicate root: {root}")
        roots.add(root)
        count += checked
    require(roots == ROOTS, f"expected roots {sorted(ROOTS)}, found {sorted(roots)}")
    print(f"Wave 4 recipes passed: {len(roots)} manifests, {count} vector/composite digests. "
          "Semantic oracles remain separate gates.")


if __name__ == "__main__":
    main()
