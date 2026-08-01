#!/usr/bin/env python3

"""Contract tests for the v0.4 semantic fixture matrix manifest."""

from __future__ import annotations

import json
import pathlib
import subprocess
import sys
import tempfile
import unittest


SCRIPTS_DIR = pathlib.Path(__file__).resolve().parent
ROOT = SCRIPTS_DIR.parent.parent
VALIDATOR = SCRIPTS_DIR / "semantic_fixture_matrix.py"
MANIFEST = SCRIPTS_DIR / "semantic-fixture-matrix.json"
TARGETS = ["aarch64-apple-darwin", "x86_64-unknown-linux-gnu"]


class SemanticFixtureMatrixTest(unittest.TestCase):
    def run_validator(self, manifest: pathlib.Path = MANIFEST, root: pathlib.Path = ROOT):
        return subprocess.run(
            [
                sys.executable,
                str(VALIDATOR),
                "--manifest",
                str(manifest),
                "--root",
                str(root),
            ],
            capture_output=True,
            text=True,
            check=False,
        )

    def test_manifest_projects_deterministic_fixture_inventory(self):
        result = self.run_validator()
        self.assertEqual(result.returncode, 0, result.stderr)
        projected = json.loads(result.stdout)
        self.assertEqual(projected["schema_version"], 1)
        self.assertEqual(projected["suite"], "v4-m1-01")
        self.assertEqual(projected["targets"], TARGETS)
        self.assertGreaterEqual(projected["fixture_count"], 8)
        self.assertEqual(projected["fixture_count"], len(projected["fixtures"]))
        ids = [fixture["id"] for fixture in projected["fixtures"]]
        self.assertEqual(ids, sorted(ids))
        self.assertEqual(len(ids), len(set(ids)))
        self.assertIn("valid", {fixture["kind"] for fixture in projected["fixtures"]})
        self.assertIn("invalid", {fixture["kind"] for fixture in projected["fixtures"]})
        for fixture in projected["fixtures"]:
            self.assertEqual(fixture["targets"], TARGETS)
            self.assertIn("report", fixture["observables"])
            self.assertEqual(fixture["execution"]["fallback"], "forbidden")
            self.assertEqual(fixture["execution"]["stage0"], "current-source")
            self.assertEqual(fixture["execution"]["network"], "forbidden")

    def test_r1_nested_record_pattern_fixture_declares_end_to_end_observables(self):
        result = self.run_validator()
        self.assertEqual(result.returncode, 0, result.stderr)
        projected = json.loads(result.stdout)
        fixture = next(
            fixture
            for fixture in projected["fixtures"]
            if fixture["id"] == "valid/nested-record-pattern"
        )
        self.assertEqual(fixture["kind"], "valid")
        self.assertEqual(
            fixture["layers"],
            ["syntax", "types", "ir", "codegen", "runtime"],
        )
        self.assertEqual(
            fixture["observables"],
            ["ast", "type", "ir", "ftable", "imports", "wasm", "runtime", "report"],
        )
        self.assertEqual(fixture["commands"], ["check", "compile", "build"])
        self.assertEqual(fixture["expected"]["runtime"]["stdout"], "41\n1\n7\n")
        self.assertEqual(fixture["expected"]["runtime"]["exit_code"], 0)

    def test_r1_literal_record_pattern_is_explicit_unsupported_boundary(self):
        result = self.run_validator()
        self.assertEqual(result.returncode, 0, result.stderr)
        projected = json.loads(result.stdout)
        fixture = next(
            fixture
            for fixture in projected["fixtures"]
            if fixture["id"] == "invalid/record-field-pattern-literal"
        )
        self.assertEqual(fixture["kind"], "invalid")
        self.assertEqual(fixture["commands"], ["compile"])
        self.assertEqual(fixture["expected"]["exit_code"], 1)
        self.assertEqual(
            fixture["expected"]["diagnostics"],
            [
                {
                    "code": "LS3001",
                    "span": {
                        "start": {"line": 8, "column": 19},
                        "end": {"line": 8, "column": 21},
                    },
                }
            ],
        )
        self.assertEqual(
            fixture["expected"]["artifact"],
            {"required": False, "status": "not-applicable"},
        )

    def test_r1_map_collections_fixture_declares_runtime_contract(self):
        result = self.run_validator()
        self.assertEqual(result.returncode, 0, result.stderr)
        projected = json.loads(result.stdout)
        fixture = next(
            fixture
            for fixture in projected["fixtures"]
            if fixture["id"] == "valid/map-collections"
        )
        self.assertEqual(fixture["kind"], "valid")
        self.assertEqual(
            fixture["layers"],
            ["syntax", "types", "ir", "codegen", "runtime"],
        )
        self.assertEqual(
            fixture["observables"],
            ["ast", "type", "ir", "wasm", "runtime", "report"],
        )
        self.assertEqual(fixture["commands"], ["check", "compile", "build"])
        self.assertEqual(fixture["expected"]["runtime"]["stdout"], "3\n1\n0\n")
        self.assertEqual(fixture["expected"]["runtime"]["exit_code"], 0)

    def test_r2_closure_allocation_fixture_declares_runtime_contract(self):
        result = self.run_validator()
        self.assertEqual(result.returncode, 0, result.stderr)
        projected = json.loads(result.stdout)
        fixture = next(
            fixture
            for fixture in projected["fixtures"]
            if fixture["id"] == "valid/closure-allocation"
        )
        self.assertEqual(fixture["kind"], "valid")
        self.assertEqual(
            fixture["layers"],
            ["syntax", "types", "ir", "codegen", "runtime"],
        )
        self.assertEqual(
            fixture["observables"],
            ["ast", "type", "ir", "wasm", "runtime", "report"],
        )
        self.assertEqual(fixture["commands"], ["check", "compile", "build"])
        self.assertEqual(fixture["expected"]["runtime"]["stdout"], "5\n")
        self.assertEqual(fixture["expected"]["runtime"]["exit_code"], 0)

    def test_r3_free_list_growth_fixture_declares_runtime_contract(self):
        result = self.run_validator()
        self.assertEqual(result.returncode, 0, result.stderr)
        projected = json.loads(result.stdout)
        fixture = next(
            fixture
            for fixture in projected["fixtures"]
            if fixture["id"] == "valid/free-list-growth"
        )
        self.assertEqual(fixture["kind"], "valid")
        self.assertEqual(
            fixture["layers"],
            ["syntax", "types", "ir", "codegen", "runtime"],
        )
        self.assertEqual(
            fixture["observables"],
            ["ast", "type", "ir", "wasm", "runtime", "report"],
        )
        self.assertEqual(fixture["commands"], ["check", "compile", "build"])
        self.assertEqual(fixture["expected"]["runtime"]["stdout"], "4097\n")
        self.assertEqual(fixture["expected"]["runtime"]["exit_code"], 0)

    def test_r4_argv_program_only_fixture_declares_runtime_contract(self):
        result = self.run_validator()
        self.assertEqual(result.returncode, 0, result.stderr)
        projected = json.loads(result.stdout)
        fixture = next(
            fixture
            for fixture in projected["fixtures"]
            if fixture["id"] == "valid/argv-program-only"
        )
        self.assertEqual(fixture["kind"], "valid")
        self.assertEqual(
            fixture["layers"],
            ["syntax", "types", "ir", "codegen", "runtime"],
        )
        self.assertEqual(
            fixture["observables"],
            ["ast", "type", "ir", "wasm", "runtime", "report"],
        )
        self.assertEqual(fixture["commands"], ["check", "compile", "build"])
        self.assertEqual(fixture["expected"]["runtime"]["stdout"], "1\n")
        self.assertEqual(fixture["expected"]["runtime"]["exit_code"], 0)

    def test_rejects_unresolved_target_and_unsafe_source(self):
        original = json.loads(MANIFEST.read_text(encoding="utf-8"))
        cases = (
            ("unknown target", {"targets": ["aarch64-apple-darwin", "unknown"]}),
            ("unsafe source", {"source": "../outside.ls"}),
        )
        for label, update in cases:
            with self.subTest(label=label), tempfile.TemporaryDirectory() as directory:
                manifest = pathlib.Path(directory) / "manifest.json"
                payload = json.loads(json.dumps(original))
                if "targets" in update:
                    payload["fixtures"][0]["targets"] = update["targets"]
                else:
                    payload["fixtures"][0]["source"] = update["source"]
                manifest.write_text(json.dumps(payload), encoding="utf-8")
                result = self.run_validator(manifest)
                self.assertNotEqual(result.returncode, 0)
                expected_word = "target" if "target" in label else "source"
                self.assertIn(expected_word, result.stderr.lower())

    def test_rejects_valid_fixture_with_diagnostic_or_invalid_without_one(self):
        original = json.loads(MANIFEST.read_text(encoding="utf-8"))
        valid_index = next(
            index for index, fixture in enumerate(original["fixtures"])
            if fixture["kind"] == "valid"
        )
        invalid_index = next(
            index for index, fixture in enumerate(original["fixtures"])
            if fixture["kind"] == "invalid"
        )
        cases = (
            (valid_index, {"kind": "valid", "diagnostics": [{"code": "LS0101"}]}),
            (invalid_index, {"kind": "invalid", "diagnostics": []}),
        )
        for index, update in cases:
            with self.subTest(index=index), tempfile.TemporaryDirectory() as directory:
                manifest = pathlib.Path(directory) / "manifest.json"
                payload = json.loads(json.dumps(original))
                fixture = payload["fixtures"][index]
                fixture["kind"] = update["kind"]
                fixture["expected"]["diagnostics"] = update["diagnostics"]
                manifest.write_text(json.dumps(payload), encoding="utf-8")
                result = self.run_validator(manifest)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("diagnostic", result.stderr.lower())

    def test_rejects_stale_stage0_fallback_or_network_execution(self):
        original = json.loads(MANIFEST.read_text(encoding="utf-8"))
        for field, value in (
            ("stage0", "stale-artifact"),
            ("fallback", "host-rust"),
            ("network", "implicit"),
        ):
            with self.subTest(field=field), tempfile.TemporaryDirectory() as directory:
                manifest = pathlib.Path(directory) / "manifest.json"
                payload = json.loads(json.dumps(original))
                payload["execution"][field] = value
                manifest.write_text(json.dumps(payload), encoding="utf-8")
                result = self.run_validator(manifest)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn(field, result.stderr.lower())


if __name__ == "__main__":
    raise SystemExit(unittest.main())
