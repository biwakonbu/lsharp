#!/usr/bin/env python3

"""Contract tests for the two-target evidence aggregate schema."""

from __future__ import annotations

import json
import pathlib
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[2]
SCHEMA = ROOT / "docs/schemas/v4-m1-06-evidence-aggregate.schema.json"


class SemanticFixtureEvidenceAggregateSchemaTest(unittest.TestCase):
    def load_schema(self):
        return json.loads(SCHEMA.read_text(encoding="utf-8"))

    def test_declares_two_target_aggregate_shape(self):
        schema = self.load_schema()
        self.assertEqual(schema["properties"]["schema_version"], {"const": 1})
        self.assertEqual(schema["properties"]["suite"], {"const": "v4-m1-06-aggregate"})
        self.assertEqual(schema["properties"]["task"], {"const": "V4-M1-01"})
        self.assertEqual(schema["properties"]["status"]["enum"], ["pass", "pending", "mismatch"])
        self.assertEqual(schema["properties"]["indexes"]["minItems"], 2)
        self.assertEqual(schema["properties"]["indexes"]["maxItems"], 2)

    def test_declares_target_and_safe_index_reference(self):
        schema = self.load_schema()
        item = schema["properties"]["indexes"]["items"]
        self.assertEqual(
            item["properties"]["target"]["enum"],
            ["aarch64-apple-darwin", "x86_64-unknown-linux-gnu"],
        )
        self.assertEqual(
            item["properties"]["index"]["pattern"],
            r"^ci-artifacts/v4-m1-01/(?!.*(?:^|/)\.\.(?:/|$))(?!.*\\).+",
        )


if __name__ == "__main__":
    raise SystemExit(unittest.main())
