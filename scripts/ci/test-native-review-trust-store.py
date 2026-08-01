#!/usr/bin/env python3
"""Focused native trust-store rotation contract tests."""

from __future__ import annotations

import json
import pathlib
import subprocess
import sys
import tempfile
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[2]
VERIFY = ROOT / "scripts/ci/verify-native-review-trust-store.py"


class NativeReviewTrustStoreTests(unittest.TestCase):
    def run_verify(self, value: object) -> subprocess.CompletedProcess[str]:
        with tempfile.TemporaryDirectory(prefix="lsharp-trust-store-") as directory:
            path = pathlib.Path(directory) / "trust-store.json"
            path.write_text(json.dumps(value), encoding="utf-8")
            return subprocess.run(
                [sys.executable, str(VERIFY), str(path)],
                cwd=ROOT,
                text=True,
                capture_output=True,
                check=False,
            )

    def test_rotation_selects_single_active_key(self) -> None:
        result = self.run_verify(
            {
                "keys": [
                    {
                        "provider": "github",
                        "key_id": "org/reviews-2025",
                        "algorithm": "ed25519",
                        "public_key": "BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc",
                        "active": False,
                    },
                    {
                        "provider": "github",
                        "key_id": "org/reviews-2026",
                        "algorithm": "ed25519",
                        "public_key": "CAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAg",
                        "active": True,
                    },
                ]
            }
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("active key: github/ed25519=org/reviews-2026", result.stdout)

    def test_ambiguous_active_keys_fail_closed(self) -> None:
        result = self.run_verify(
            {
                "keys": [
                    {
                        "provider": "github",
                        "key_id": "org/reviews-2025",
                        "algorithm": "ed25519",
                        "public_key": "BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc",
                        "active": True,
                    },
                    {
                        "provider": "github",
                        "key_id": "org/reviews-2026",
                        "algorithm": "ed25519",
                        "public_key": "CAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAg",
                        "active": True,
                    },
                ]
            }
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("multiple active keys", result.stderr)


if __name__ == "__main__":
    unittest.main()
