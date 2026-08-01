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

    def test_module_import_fixture_runtime_matches_source_contract(self):
        result = self.run_validator()
        self.assertEqual(result.returncode, 0, result.stderr)
        projected = json.loads(result.stdout)
        fixture = next(
            fixture
            for fixture in projected["fixtures"]
            if fixture["id"] == "valid/module-import"
        )
        source = (ROOT / fixture["source"]).read_text(encoding="utf-8")
        self.assertIn("(print (add (mul 3 4) 5))", source)
        self.assertEqual(fixture["expected"]["runtime"]["stdout"], "17\n")
        self.assertEqual(fixture["expected"]["runtime"]["exit_code"], 0)

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

    def test_r4_file_fixture_declares_explicit_runtime_input_snapshot(self):
        result = self.run_validator()
        self.assertEqual(result.returncode, 0, result.stderr)
        projected = json.loads(result.stdout)
        fixture = next(
            fixture
            for fixture in projected["fixtures"]
            if fixture["id"] == "valid/io-read-file"
        )
        self.assertEqual(fixture["runtime_inputs"], {"input.txt": "payload"})

    def test_r4_empty_file_fixture_declares_explicit_empty_snapshot(self):
        result = self.run_validator()
        self.assertEqual(result.returncode, 0, result.stderr)
        projected = json.loads(result.stdout)
        fixture = next(
            fixture
            for fixture in projected["fixtures"]
            if fixture["id"] == "valid/io-read-file-empty"
        )
        self.assertEqual(fixture["runtime_inputs"], {"input.txt": ""})
        self.assertEqual(fixture["expected"]["runtime"]["stdout"], "")

    def test_r4_missing_file_fixture_declares_explicit_empty_runtime_directory(self):
        result = self.run_validator()
        self.assertEqual(result.returncode, 0, result.stderr)
        projected = json.loads(result.stdout)
        fixture = next(
            fixture
            for fixture in projected["fixtures"]
            if fixture["id"] == "valid/io-read-file-missing"
        )
        self.assertEqual(fixture["runtime_inputs"], {})
        self.assertEqual(fixture["expected"]["runtime"]["stdout"], "")

    def test_r4_stdin_fixture_declares_explicit_runtime_stdin_snapshot(self):
        result = self.run_validator()
        self.assertEqual(result.returncode, 0, result.stderr)
        projected = json.loads(result.stdout)
        fixture = next(
            fixture
            for fixture in projected["fixtures"]
            if fixture["id"] == "valid/io-read-stdin"
        )
        self.assertEqual(fixture["runtime_stdin"], "payload")

    def test_rejects_unsafe_or_non_string_runtime_input_snapshot(self):
        original = json.loads(MANIFEST.read_text(encoding="utf-8"))
        cases = (
            ("unsafe path", {"runtime_inputs": {"../outside.txt": "payload"}}),
            ("non-string content", {"runtime_inputs": {"input.txt": 42}}),
            ("non-string stdin", {"runtime_stdin": 42}),
        )
        fixture_index = next(
            index for index, fixture in enumerate(original["fixtures"])
            if fixture["id"] == "valid/io-read-file"
        )
        for label, update in cases:
            with self.subTest(label=label), tempfile.TemporaryDirectory() as directory:
                manifest = pathlib.Path(directory) / "manifest.json"
                payload = json.loads(json.dumps(original))
                payload["fixtures"][fixture_index].update(update)
                manifest.write_text(json.dumps(payload), encoding="utf-8")
                result = self.run_validator(manifest)
                self.assertNotEqual(result.returncode, 0)
                expected_field = "runtime_stdin" if "runtime_stdin" in update else "runtime_inputs"
                self.assertIn(expected_field, result.stderr)

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

    def test_rejects_source_fixture_symlink_traversal(self):
        original = json.loads(MANIFEST.read_text(encoding="utf-8"))
        with tempfile.TemporaryDirectory() as root_directory, tempfile.TemporaryDirectory() as outside_directory:
            root = pathlib.Path(root_directory)
            outside = pathlib.Path(outside_directory) / "outside.ls"
            outside.write_text("(defn outside [] true)\n", encoding="utf-8")
            fixture_directory = root / "fixtures"
            fixture_directory.mkdir()
            (fixture_directory / "link.ls").symlink_to(outside)

            payload = json.loads(json.dumps(original))
            payload["fixtures"] = [payload["fixtures"][0]]
            payload["fixtures"][0]["source"] = "fixtures/link.ls"
            manifest = root / "manifest.json"
            manifest.write_text(json.dumps(payload), encoding="utf-8")

            result = self.run_validator(manifest, root)

            self.assertNotEqual(result.returncode, 0)
            self.assertIn("symlink", result.stderr.lower())

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

    def test_rejects_runtime_fixture_without_artifact_command(self):
        original = json.loads(MANIFEST.read_text(encoding="utf-8"))
        fixture_index = next(
            index
            for index, fixture in enumerate(original["fixtures"])
            if fixture["kind"] == "valid" and "runtime" in fixture["observables"]
        )
        with tempfile.TemporaryDirectory() as directory:
            manifest = pathlib.Path(directory) / "manifest.json"
            payload = json.loads(json.dumps(original))
            payload["fixtures"][fixture_index]["commands"] = ["check"]
            manifest.write_text(json.dumps(payload), encoding="utf-8")

            result = self.run_validator(manifest)

            self.assertNotEqual(result.returncode, 0)
            self.assertIn("artifact command", result.stderr.lower())

    def test_rejects_artifact_runtime_scope_without_matching_layer_or_observable(self):
        original = json.loads(MANIFEST.read_text(encoding="utf-8"))
        fixture_index = next(
            index
            for index, fixture in enumerate(original["fixtures"])
            if fixture["kind"] == "valid"
        )
        cases = (
            ("missing wasm observable", lambda fixture: fixture["observables"].remove("wasm"), "wasm"),
            ("missing runtime layer", lambda fixture: fixture["layers"].remove("runtime"), "must include runtime"),
        )
        for label, mutate, expected_word in cases:
            with self.subTest(label=label), tempfile.TemporaryDirectory() as directory:
                manifest = pathlib.Path(directory) / "manifest.json"
                payload = json.loads(json.dumps(original))
                mutate(payload["fixtures"][fixture_index])
                manifest.write_text(json.dumps(payload), encoding="utf-8")

                result = self.run_validator(manifest)

                self.assertNotEqual(result.returncode, 0)
                self.assertIn(expected_word, result.stderr.lower())

    def test_rejects_artifact_runtime_observables_without_expected_scope(self):
        original = json.loads(MANIFEST.read_text(encoding="utf-8"))
        fixture_index = next(
            index
            for index, fixture in enumerate(original["fixtures"])
            if fixture["kind"] == "invalid"
        )
        cases = (
            ("wasm without artifact", "observables", "wasm", "required artifact"),
            ("runtime without expected result", "observables", "runtime", "runtime is not expected"),
            ("runtime layer without expected result", "layers", "runtime", "runtime is not expected"),
        )
        for label, field, value, expected_word in cases:
            with self.subTest(label=label), tempfile.TemporaryDirectory() as directory:
                manifest = pathlib.Path(directory) / "manifest.json"
                payload = json.loads(json.dumps(original))
                payload["fixtures"][fixture_index][field].append(value)
                manifest.write_text(json.dumps(payload), encoding="utf-8")

                result = self.run_validator(manifest)

                self.assertNotEqual(result.returncode, 0)
                self.assertIn(expected_word, result.stderr.lower())


if __name__ == "__main__":
    raise SystemExit(unittest.main())
