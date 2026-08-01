#!/usr/bin/env python3

"""Contract tests for semantic-fixture producer commands documented in docs/."""

from __future__ import annotations

from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[2]
DOCS = ROOT / "docs"
PRODUCER = re.compile(r"semantic_fixture_(?:rust|native)_report\.py")


def fenced_code_blocks(path: Path) -> list[tuple[int, str]]:
    blocks: list[tuple[int, str]] = []
    lines = path.read_text(encoding="utf-8").splitlines()
    inside = False
    start = 0
    body: list[str] = []
    for line_number, line in enumerate(lines, 1):
        if line.startswith("```"):
            if inside:
                blocks.append((start, "\n".join(body)))
                inside = False
                body = []
            else:
                inside = True
                start = line_number
            continue
        if inside:
            body.append(line)
    return blocks


class SemanticFixtureProducerDocsTest(unittest.TestCase):
    def test_fenced_producer_commands_declare_wasm_tools(self):
        violations = []
        for path in sorted(DOCS.rglob("*.md")):
            for start, body in fenced_code_blocks(path):
                if PRODUCER.search(body) and "--wasm-tools" not in body:
                    violations.append(f"{path.relative_to(ROOT)}:{start}")
        self.assertEqual(violations, [], "producer commands missing --wasm-tools: " + ", ".join(violations))

    def test_runbook_declares_and_passes_wasm_tools(self):
        runbook = (DOCS / "development/operations/v4-m1-semantic-fixture-evidence.md").read_text(
            encoding="utf-8"
        )
        self.assertIn('WASM_TOOLS="/absolute/path/to/wasm-tools"', runbook)
        self.assertGreaterEqual(runbook.count("--wasm-tools \"$WASM_TOOLS\""), 2)


if __name__ == "__main__":
    unittest.main()
