#!/usr/bin/env python3
"""Focused native verification-receipt contract tests."""

from __future__ import annotations

import json
import pathlib
import subprocess
import sys
import tempfile
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[2]
VERIFY = ROOT / "scripts/ci/verify-native-review-verification-receipt.py"
RECEIPT_HEX = "6c73686172702e7265766965772d766572696669636174696f6e2d726563656970742e763100000000000000001a7265766965773a6f72646572732f72657669657765722d30303100000000000000087665726966696564000000000000000667697468756200000000000000106f72672f726576696577732d3230323600000000000000076564323535313900000000000000477368613235363a6161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616100000000000000477368613235363a626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262620000000000000014323032362d30382d30325430303a30303a30305a"


class NativeVerificationReceiptTests(unittest.TestCase):
    def run_verify(self, value: dict) -> subprocess.CompletedProcess[str]:
        with tempfile.TemporaryDirectory(prefix="lsharp-verification-receipt-") as directory:
            path = pathlib.Path(directory) / "receipt.json"
            path.write_text(json.dumps(value), encoding="utf-8")
            return subprocess.run(
                [sys.executable, str(VERIFY), str(path)],
                cwd=ROOT,
                text=True,
                capture_output=True,
                check=False,
            )

    def valid_receipt(self) -> dict:
        return {
            "review_id": "review:orders/reviewer-001",
            "state": "verified",
            "provider": "github",
            "key_id": "org/reviews-2026",
            "algorithm": "ed25519",
            "attestation_digest": "sha256:" + "a" * 64,
            "trust_store_digest": "sha256:" + "b" * 64,
            "verification_now": "2026-08-02T00:00:00Z",
        }

    def test_receipt_shape_and_canonical_bytes_match_rust_fixture(self) -> None:
        result = self.run_verify(self.valid_receipt())
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn(f"canonical receipt bytes: {RECEIPT_HEX}", result.stdout)

    def test_non_verified_receipt_is_rejected(self) -> None:
        value = self.valid_receipt()
        value["state"] = "unverified"
        result = self.run_verify(value)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("state must be verified", result.stderr)

    def test_nonexistent_verification_date_is_rejected(self) -> None:
        value = self.valid_receipt()
        value["verification_now"] = "2026-02-30T00:00:00Z"
        result = self.run_verify(value)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("valid UTC date", result.stderr)


if __name__ == "__main__":
    unittest.main()
