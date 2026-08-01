#!/usr/bin/env python3

"""Contract tests for the V4-M1-01 one-command validation gate."""

from __future__ import annotations

from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[2]
RUNNER = ROOT / "scripts/ci/test-v4-m1-01-contracts.sh"
EXPECTED_COMMANDS = (
    "test-semantic-fixture-matrix.py",
    "test-semantic-fixture-diff.py",
    "test-semantic-fixture-rust-report.py",
    "test-semantic-fixture-native-report.py",
    "test-semantic-fixture-evidence-schema.py",
    "test-semantic-fixture-evidence-audit.py",
    "test-semantic-fixture-evidence-aggregate-schema.py",
    "test-semantic-fixture-evidence-aggregate.py",
    "test-semantic-fixture-producer-docs.py",
)


class V4M101ContractRunnerTest(unittest.TestCase):
    def test_runner_is_executable(self):
        self.assertTrue(RUNNER.is_file(), RUNNER)
        self.assertTrue(RUNNER.stat().st_mode & 0o111, f"{RUNNER} must be executable")

    def test_runner_keeps_contract_order_and_fail_fast_shell_options(self):
        source = RUNNER.read_text(encoding="utf-8")
        self.assertIn("set -euo pipefail", source)
        positions = [source.index(command) for command in EXPECTED_COMMANDS]
        self.assertEqual(positions, sorted(positions))
        self.assertIn("scripts/audit_docs.sh", source)
        self.assertIn("git diff --check", source)


if __name__ == "__main__":
    unittest.main()
