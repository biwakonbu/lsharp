#!/usr/bin/env python3
"""クレートごとの Rust test distribution を deterministic に集計する。"""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Any

TEST_ATTRIBUTE = re.compile(r"^\s*#\s*\[(?P<attribute>[^]]+)\]\s*$")
FUNCTION_DECLARATION = re.compile(r"\bfn\s+[A-Za-z_][A-Za-z0-9_]*\s*\(")
PROPTEST_MACRO = re.compile(r"\bproptest!\s*\{")


def is_test_attribute(attribute: str) -> bool:
    head = attribute.split("(", 1)[0].strip()
    return (
        head == "test"
        or head.endswith("::test")
        or head == "test_case"
        or head.endswith("::test_case")
    )


def count_test_functions(lines: list[str]) -> tuple[int, int, int, int]:
    test_attributes = 0
    test_functions = 0
    ignored_tests = 0
    proptest_macros = 0

    for index, line in enumerate(lines):
        attribute_match = TEST_ATTRIBUTE.match(line)
        if attribute_match:
            attribute = attribute_match.group("attribute")
            if is_test_attribute(attribute):
                test_attributes += 1
                if any(
                    FUNCTION_DECLARATION.search(candidate)
                    for candidate in lines[index + 1 : index + 16]
                ):
                    test_functions += 1
            if attribute.split("(", 1)[0].strip() == "ignore":
                ignored_tests += 1

        proptest_macros += len(PROPTEST_MACRO.findall(line))

    return test_attributes, test_functions, proptest_macros, ignored_tests


def crate_name(cargo_toml: Path) -> str:
    match = re.search(r'^name\s*=\s*"([^"]+)"\s*$', cargo_toml.read_text(encoding="utf-8"), re.MULTILINE)
    if not match:
        raise ValueError(f"package name is missing: {cargo_toml}")
    return match.group(1)


def collect_crate(crate_dir: Path, cargo_toml: Path) -> dict[str, Any]:
    test_attributes = 0
    test_functions = 0
    proptest_macros = 0
    ignored_tests = 0
    rust_files = 0

    for rust_file in sorted(crate_dir.rglob("*.rs")):
        if any(part in {"target", ".git"} for part in rust_file.parts):
            continue
        rust_files += 1
        lines = rust_file.read_text(encoding="utf-8").splitlines()
        attributes, functions, macros, ignored = count_test_functions(lines)
        test_attributes += attributes
        test_functions += functions
        proptest_macros += macros
        ignored_tests += ignored

    return {
        "name": crate_name(cargo_toml),
        "rust_files": rust_files,
        "test_attributes": test_attributes,
        "test_functions": test_functions,
        "proptest_macros": proptest_macros,
        "ignored_tests": ignored_tests,
    }


def collect(root: Path) -> dict[str, Any]:
    crate_entries = [
        collect_crate(cargo_toml.parent, cargo_toml)
        for cargo_toml in sorted((root / "crates").glob("*/Cargo.toml"))
    ]
    totals = {
        key: sum(entry[key] for entry in crate_entries)
        for key in (
            "rust_files",
            "test_attributes",
            "test_functions",
            "proptest_macros",
            "ignored_tests",
        )
    }
    return {"schema_version": 1, "crates": crate_entries, "totals": totals}


def print_text(payload: dict[str, Any]) -> None:
    headers = (
        "crate",
        "rust_files",
        "test_attributes",
        "test_functions",
        "proptest_macros",
        "ignored_tests",
    )
    print("\t".join(headers))
    for entry in payload["crates"]:
        print(
            "\t".join(
                [entry["name"]]
                + [str(entry[key]) for key in headers[1:]]
            )
        )
    print("TOTAL\t" + "\t".join(str(payload["totals"][key]) for key in headers[1:]))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--json", action="store_true", dest="as_json")
    args = parser.parse_args()

    root = args.root.resolve()
    if not (root / "crates").is_dir():
        parser.error(f"workspace crates directory is missing: {root / 'crates'}")
    payload = collect(root)
    if args.as_json:
        print(json.dumps(payload, ensure_ascii=False, indent=2, sort_keys=True))
    else:
        print_text(payload)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
