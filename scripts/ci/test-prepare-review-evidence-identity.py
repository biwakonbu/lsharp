#!/usr/bin/env python3

import hashlib
import json
import pathlib
import subprocess
import sys
import tempfile
import unittest


SCRIPTS_DIR = pathlib.Path(__file__).resolve().parent
PREPARER = SCRIPTS_DIR / "prepare-review-evidence-identity.py"
VERIFIER = SCRIPTS_DIR / "verify-native-release-identity.py"
SOURCE_COMMIT = "0123456789abcdef0123456789abcdef01234567"
IDENTITY_KEYS = (
    "subject_digest",
    "source_commit",
    "artifact_digest",
    "trust_store_digest",
    "lifecycle_digest",
    "now",
)


class PrepareReviewEvidenceIdentityTest(unittest.TestCase):
    def run_preparer(self, *arguments):
        return subprocess.run(
            [sys.executable, str(PREPARER), *arguments],
            capture_output=True,
            text=True,
            check=False,
        )

    def test_projects_canonical_identity_from_explicit_snapshot_bytes(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            artifact = root / "program.native"
            artifact.write_bytes(b"native release program\n")
            trust_store = root / "trust-store.json"
            trust_store.write_bytes(b'{"keys":["key-1"]}\n')
            lifecycle = root / "review-lifecycle.jsonl"
            lifecycle.write_bytes(b'{"review_id":"review:checkout/r1","state":"active"}\n')
            output = root / "identity.json"
            subject_digest = "sha256:" + "c" * 64

            result = self.run_preparer(
                "--subject-digest",
                subject_digest,
                "--source-commit",
                SOURCE_COMMIT,
                "--artifact",
                str(artifact),
                "--trust-store",
                str(trust_store),
                "--review-lifecycle",
                str(lifecycle),
                "--now",
                "2026-08-15T00:00:00Z",
                "--output",
                str(output),
            )

            self.assertEqual(result.returncode, 0, result.stderr)
            identity = json.loads(result.stdout)
            self.assertEqual(list(identity), list(IDENTITY_KEYS))
            self.assertEqual(identity["subject_digest"], subject_digest)
            self.assertEqual(identity["source_commit"], SOURCE_COMMIT)
            self.assertEqual(
                identity["artifact_digest"],
                "sha256:" + hashlib.sha256(artifact.read_bytes()).hexdigest(),
            )
            self.assertEqual(
                identity["trust_store_digest"],
                "sha256:" + hashlib.sha256(trust_store.read_bytes()).hexdigest(),
            )
            self.assertEqual(
                identity["lifecycle_digest"],
                "sha256:" + hashlib.sha256(lifecycle.read_bytes()).hexdigest(),
            )
            self.assertEqual(output.read_text(encoding="utf-8"), result.stdout)

            verified = subprocess.run(
                [
                    sys.executable,
                    str(VERIFIER),
                    "--identity",
                    str(output),
                    "--artifact",
                    str(artifact),
                    "--source-commit",
                    SOURCE_COMMIT,
                    "--require-provider-input",
                ],
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertEqual(verified.returncode, 0, verified.stderr)
            self.assertEqual(json.loads(verified.stdout), identity)

    def test_without_provider_snapshots_is_explicitly_unverified_input(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            artifact = root / "program.native"
            artifact.write_bytes(b"native release program\n")

            result = self.run_preparer(
                "--subject-digest",
                "sha256:" + "c" * 64,
                "--source-commit",
                SOURCE_COMMIT,
                "--artifact",
                str(artifact),
                "--now",
                "2026-08-15T00:00:00Z",
            )

            self.assertEqual(result.returncode, 0, result.stderr)
            identity = json.loads(result.stdout)
            self.assertIsNone(identity["trust_store_digest"])
            self.assertIsNone(identity["lifecycle_digest"])

    def test_rejects_partial_provider_input_and_invalid_explicit_values(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            artifact = root / "program.native"
            artifact.write_bytes(b"native release program\n")
            trust_store = root / "trust-store.json"
            trust_store.write_bytes(b"{}\n")
            base = (
                "--subject-digest",
                "sha256:" + "c" * 64,
                "--source-commit",
                SOURCE_COMMIT,
                "--artifact",
                str(artifact),
                "--now",
                "2026-08-15T00:00:00Z",
            )

            partial = self.run_preparer(*base, "--trust-store", str(trust_store))
            self.assertNotEqual(partial.returncode, 0)
            self.assertIn("together", partial.stderr)

            invalid_subject = self.run_preparer(
                "--subject-digest",
                "sha256:not-a-digest",
                *base[2:],
            )
            self.assertNotEqual(invalid_subject.returncode, 0)
            self.assertIn("subject_digest", invalid_subject.stderr)

            invalid_clock = self.run_preparer(
                "--subject-digest",
                "sha256:" + "c" * 64,
                "--source-commit",
                SOURCE_COMMIT,
                "--artifact",
                str(artifact),
                "--now",
                "2026-08-15 00:00:00Z",
            )
            self.assertNotEqual(invalid_clock.returncode, 0)
            self.assertIn("now", invalid_clock.stderr)

    def test_missing_output_parent_fails_before_claiming_success(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            artifact = root / "program.native"
            artifact.write_bytes(b"native release program\n")
            output = root / "missing-parent" / "identity.json"

            result = self.run_preparer(
                "--subject-digest",
                "sha256:" + "c" * 64,
                "--source-commit",
                SOURCE_COMMIT,
                "--artifact",
                str(artifact),
                "--now",
                "2026-08-15T00:00:00Z",
                "--output",
                str(output),
            )

            self.assertNotEqual(result.returncode, 0)
            self.assertIn("parent directory", result.stderr)
            self.assertFalse(output.exists())


if __name__ == "__main__":
    unittest.main(verbosity=2)
