#!/usr/bin/env python3

"""Contract tests for the machine-readable V4 evidence-index schema."""

from __future__ import annotations

import json
import pathlib
import unittest


SCRIPTS_DIR = pathlib.Path(__file__).resolve().parent
ROOT = SCRIPTS_DIR.parent.parent
SCHEMA = ROOT / "docs/schemas/v4-m1-06-evidence-index.schema.json"


class SemanticFixtureEvidenceSchemaTest(unittest.TestCase):
    def load_schema(self):
        return json.loads(SCHEMA.read_text(encoding="utf-8"))

    def test_declares_versioned_index_boundary(self):
        schema = self.load_schema()
        self.assertEqual(schema["$schema"], "https://json-schema.org/draft/2020-12/schema")
        self.assertEqual(schema["properties"]["schema_version"], {"const": 1})
        self.assertEqual(schema["properties"]["suite"], {"const": "v4-m1-06"})
        self.assertEqual(schema["properties"]["task"], {"const": "V4-M1-01"})
        self.assertTrue(schema["additionalProperties"] is False)
        self.assertEqual(
            schema["required"],
            [
                "schema_version",
                "suite",
                "task",
                "target",
                "source_commit",
                "status",
                "adr",
                "oracle_report",
                "native_report",
                "comparison",
                "fixtures",
            ],
        )

    def test_declares_status_target_and_safe_reference_shapes(self):
        schema = self.load_schema()
        properties = schema["properties"]
        self.assertEqual(properties["status"]["enum"], ["pass", "pending", "mismatch"])
        self.assertEqual(properties["target"]["enum"], ["aarch64-apple-darwin", "x86_64-unknown-linux-gnu"])
        self.assertEqual(properties["source_commit"]["pattern"], "^[0-9a-f]{40}$")
        self.assertEqual(properties["adr"]["type"], "string")
        self.assertEqual(
            properties["adr"]["pattern"],
            r"^docs/adr/(?!.*(?:^|/)\.\.(?:/|$))(?!.*\\).+\.md$",
        )
        for field in ("oracle_report", "native_report", "comparison"):
            self.assertEqual(properties[field]["type"], "string")
            self.assertIn("pattern", properties[field])
            self.assertEqual(
                properties[field]["pattern"],
                r"^ci-artifacts/(?!.*(?:^|/)\.\.(?:/|$))(?!.*\\).+$",
            )

    def test_declares_fixture_command_and_required_negative_gates(self):
        schema = self.load_schema()
        fixture = schema["properties"]["fixtures"]["items"]
        self.assertFalse(fixture["additionalProperties"])
        self.assertEqual(fixture["required"], ["id", "command", "negative_gates"])
        gates = fixture["properties"]["negative_gates"]
        self.assertFalse(gates["additionalProperties"])
        self.assertEqual(
            gates["required"],
            ["fallback-forbidden", "network-forbidden", "source-commit-bound", "target-declared"],
        )
        for gate in gates["required"]:
            self.assertEqual(gates["properties"][gate], {"const": "pass"})


if __name__ == "__main__":
    raise SystemExit(unittest.main())
