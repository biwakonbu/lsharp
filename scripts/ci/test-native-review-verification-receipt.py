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

    def run_verify_with_trust_store(
        self, value: dict, trust_store: dict
    ) -> subprocess.CompletedProcess[str]:
        with tempfile.TemporaryDirectory(prefix="lsharp-verification-receipt-trust-") as directory:
            root = pathlib.Path(directory)
            receipt_path = root / "receipt.json"
            trust_store_path = root / "trust-store.json"
            receipt_path.write_text(json.dumps(value), encoding="utf-8")
            trust_store_path.write_text(json.dumps(trust_store), encoding="utf-8")
            return subprocess.run(
                [
                    sys.executable,
                    str(VERIFY),
                    str(receipt_path),
                    "--trust-store",
                    str(trust_store_path),
                ],
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

    def test_active_trust_store_identity_is_required_for_handoff(self) -> None:
        trust_store = {
            "keys": [
                {
                    "provider": "github",
                    "key_id": "org/reviews-2026",
                    "algorithm": "ed25519",
                    "public_key": "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8",
                    "active": True,
                },
                {
                    "provider": "github",
                    "key_id": "org/reviews-2025",
                    "algorithm": "ed25519",
                    "public_key": "AQIDBAUGBwgJCgsMDQ4PEBESExQVFhcYGRobHB0eHyA",
                    "active": False,
                },
            ]
        }
        result = self.run_verify_with_trust_store(self.valid_receipt(), trust_store)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("active trust identity: github/ed25519=org/reviews-2026", result.stdout)

    def test_inactive_or_other_trust_store_identity_is_rejected(self) -> None:
        value = self.valid_receipt()
        trust_store = {
            "keys": [
                {
                    "provider": "github",
                    "key_id": "org/other-2026",
                    "algorithm": "ed25519",
                    "public_key": "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8",
                    "active": True,
                }
            ]
        }
        result = self.run_verify_with_trust_store(value, trust_store)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("active trust identity mismatch", result.stderr)

    def test_invalid_trust_store_is_rejected_before_handoff(self) -> None:
        result = self.run_verify_with_trust_store(
            self.valid_receipt(), {"keys": [{"provider": "github"}]}
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("trust store", result.stderr)

    def test_symlink_trust_store_is_rejected_before_handoff(self) -> None:
        with tempfile.TemporaryDirectory(prefix="lsharp-verification-receipt-link-") as directory:
            root = pathlib.Path(directory)
            receipt_path = root / "receipt.json"
            target = root / "trust-store-target.json"
            link = root / "trust-store.json"
            receipt_path.write_text(json.dumps(self.valid_receipt()), encoding="utf-8")
            target.write_text(json.dumps({"keys": []}), encoding="utf-8")
            link.symlink_to(target)
            result = subprocess.run(
                [
                    sys.executable,
                    str(VERIFY),
                    str(receipt_path),
                    "--trust-store",
                    str(link),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
                check=False,
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("regular non-symlink file", result.stderr)


if __name__ == "__main__":
    unittest.main()
