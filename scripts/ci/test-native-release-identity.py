#!/usr/bin/env python3

import hashlib
import json
import os
import pathlib
import subprocess
import sys
import tempfile
import unittest


SCRIPTS_DIR = pathlib.Path(__file__).resolve().parent
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


def identity_for(artifact_digest, trust="sha256:" + "a" * 64, lifecycle="sha256:" + "b" * 64):
    return {
        "subject_digest": "sha256:" + "c" * 64,
        "source_commit": SOURCE_COMMIT,
        "artifact_digest": artifact_digest,
        "trust_store_digest": trust,
        "lifecycle_digest": lifecycle,
        "now": "2026-08-15T00:00:00Z",
    }


class NativeReleaseIdentityTest(unittest.TestCase):
    def run_verifier(self, *arguments):
        return subprocess.run(
            [sys.executable, str(VERIFIER), *arguments],
            capture_output=True,
            text=True,
            check=False,
        )

    def write_identity(self, path, identity):
        path.write_text(json.dumps(identity, separators=(",", ":")) + "\n", encoding="utf-8")

    def test_projects_and_verifies_packaged_manifest_identity(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            artifact = root / "program.native"
            artifact.write_bytes(b"native release program\n")
            artifact_digest = "sha256:" + hashlib.sha256(artifact.read_bytes()).hexdigest()
            trust_store = root / "trust-store.json"
            trust_store.write_bytes(b'{"keys":["key-1"]}\n')
            lifecycle = root / "review-lifecycle.jsonl"
            lifecycle.write_bytes(
                b'{"review_id":"review:checkout/r1","sequence":1,"state":"active"}\n'
            )
            identity = identity_for(
                artifact_digest,
                trust="sha256:" + hashlib.sha256(trust_store.read_bytes()).hexdigest(),
                lifecycle="sha256:" + hashlib.sha256(lifecycle.read_bytes()).hexdigest(),
            )
            manifest = root / "manifest.json"
            manifest.write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "review_evidence_identity": identity,
                    },
                    separators=(",", ":"),
                )
                + "\n",
                encoding="utf-8",
            )

            result = self.run_verifier(
                "--manifest",
                str(manifest),
                "--artifact",
                str(artifact),
                "--source-commit",
                SOURCE_COMMIT,
                "--trust-store",
                str(trust_store),
                "--review-lifecycle",
                str(lifecycle),
                "--require-provider-input",
            )

            self.assertEqual(result.returncode, 0, result.stderr)
            projected = json.loads(result.stdout)
            self.assertEqual(list(projected), list(IDENTITY_KEYS))
            self.assertEqual(projected, identity)

    def test_rejects_artifact_digest_mismatch_and_provider_absence(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            artifact = root / "program.native"
            artifact.write_bytes(b"native release program\n")
            artifact_digest = "sha256:" + hashlib.sha256(artifact.read_bytes()).hexdigest()
            identity_path = root / "identity.json"
            trust_store = root / "trust-store.json"
            trust_store.write_bytes(b'{"keys":["key-1"]}\n')
            lifecycle = root / "review-lifecycle.jsonl"
            lifecycle.write_bytes(
                b'{"review_id":"review:checkout/r1","sequence":1,"state":"active"}\n'
            )
            identity = identity_for(
                artifact_digest,
                trust="sha256:" + hashlib.sha256(trust_store.read_bytes()).hexdigest(),
                lifecycle="sha256:" + hashlib.sha256(lifecycle.read_bytes()).hexdigest(),
            )
            self.write_identity(identity_path, identity)

            identity["artifact_digest"] = "sha256:" + "d" * 64
            self.write_identity(identity_path, identity)
            mismatch = self.run_verifier(
                "--identity",
                str(identity_path),
                "--artifact",
                str(artifact),
                "--source-commit",
                SOURCE_COMMIT,
                "--trust-store",
                str(trust_store),
                "--review-lifecycle",
                str(lifecycle),
                "--require-provider-input",
            )
            self.assertNotEqual(mismatch.returncode, 0)
            self.assertIn("artifact_digest", mismatch.stderr)

            identity = identity_for(artifact_digest, trust=None, lifecycle=None)
            self.write_identity(identity_path, identity)
            missing_provider = self.run_verifier(
                "--identity",
                str(identity_path),
                "--artifact",
                str(artifact),
                "--source-commit",
                SOURCE_COMMIT,
                "--require-provider-input",
            )
            self.assertEqual(missing_provider.returncode, 2)
            self.assertIn("provider", missing_provider.stderr)

    def test_rejects_symlinked_release_artifact_path(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            artifact = root / "program.native"
            outside_artifact = root / "outside-program.native"
            outside_artifact.write_bytes(b"native release program\n")
            artifact.symlink_to(outside_artifact)
            artifact_digest = "sha256:" + hashlib.sha256(
                outside_artifact.read_bytes()
            ).hexdigest()
            identity_path = root / "identity.json"
            self.write_identity(identity_path, identity_for(artifact_digest))

            result = self.run_verifier(
                "--identity",
                str(identity_path),
                "--artifact",
                str(artifact),
                "--source-commit",
                SOURCE_COMMIT,
            )

            self.assertNotEqual(result.returncode, 0)
            self.assertIn(
                "artifact must be a regular non-symlink file",
                result.stderr,
            )

    def test_rejects_provider_identity_without_auth_context_files(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            artifact = root / "program.native"
            artifact.write_bytes(b"native release program\n")
            identity_path = root / "identity.json"
            artifact_digest = "sha256:" + hashlib.sha256(artifact.read_bytes()).hexdigest()
            self.write_identity(identity_path, identity_for(artifact_digest))

            result = self.run_verifier(
                "--identity",
                str(identity_path),
                "--artifact",
                str(artifact),
                "--source-commit",
                SOURCE_COMMIT,
                "--require-provider-input",
            )

            self.assertEqual(result.returncode, 2)
            self.assertIn("provider auth context", result.stderr)

    def test_rejects_field_order_and_identity_conflict(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            identity_path = root / "identity.json"
            identity = identity_for("sha256:" + "e" * 64)
            reordered = {key: identity[key] for key in reversed(IDENTITY_KEYS)}
            self.write_identity(identity_path, reordered)

            result = self.run_verifier(
                "--identity",
                str(identity_path),
                "--source-commit",
                SOURCE_COMMIT,
            )

            self.assertNotEqual(result.returncode, 0)
            self.assertIn("field order", result.stderr)

    def test_rejects_non_object_manifest_without_traceback(self):
        for payload in ("[]", "null", "1"):
            with self.subTest(payload=payload), tempfile.TemporaryDirectory() as temporary_directory:
                manifest = pathlib.Path(temporary_directory) / "manifest.json"
                manifest.write_text(payload + "\n", encoding="utf-8")

                result = self.run_verifier("--manifest", str(manifest))

                self.assertEqual(result.returncode, 1)
                self.assertIn("review_evidence_identity", result.stderr)
                self.assertNotIn("Traceback", result.stderr)

    def test_matches_rust_calendar_timestamp_boundary(self):
        valid = identity_for("sha256:" + "e" * 64)
        valid["now"] = "2024-02-29T23:59:59Z"
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            identity_path = root / "identity.json"
            self.write_identity(identity_path, valid)
            accepted = self.run_verifier("--identity", str(identity_path))
            self.assertEqual(accepted.returncode, 0, accepted.stderr)

            for invalid_timestamp in (
                "0000-01-01T00:00:00Z",
                "2023-02-29T00:00:00Z",
                "2024-02-30T00:00:00Z",
                "2026-04-31T00:00:00Z",
                "2026-12-31T24:00:00Z",
                "2026-12-31T23:60:00Z",
                "2026-12-31T23:59:60Z",
            ):
                with self.subTest(invalid_timestamp=invalid_timestamp):
                    invalid = dict(valid)
                    invalid["now"] = invalid_timestamp
                    self.write_identity(identity_path, invalid)
                    result = self.run_verifier("--identity", str(identity_path))
                    self.assertNotEqual(result.returncode, 0)
                    self.assertIn("timestamp", result.stderr)

    def test_recomputes_explicit_provider_snapshot_digests(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            artifact = root / "program.native"
            artifact.write_bytes(b"native release program\n")
            trust_store = root / "trust-store.json"
            trust_store.write_bytes(b'{"keys":["key-1"]}\n')
            lifecycle = root / "review-lifecycle.jsonl"
            lifecycle.write_bytes(b'{"review_id":"review:checkout/r1","sequence":1,"state":"active"}\n')
            artifact_digest = "sha256:" + hashlib.sha256(artifact.read_bytes()).hexdigest()
            identity = identity_for(
                artifact_digest,
                trust="sha256:" + hashlib.sha256(trust_store.read_bytes()).hexdigest(),
                lifecycle="sha256:" + hashlib.sha256(lifecycle.read_bytes()).hexdigest(),
            )
            identity_path = root / "identity.json"
            self.write_identity(identity_path, identity)

            accepted = self.run_verifier(
                "--identity",
                str(identity_path),
                "--artifact",
                str(artifact),
                "--source-commit",
                SOURCE_COMMIT,
                "--trust-store",
                str(trust_store),
                "--review-lifecycle",
                str(lifecycle),
                "--require-provider-input",
            )
            self.assertEqual(accepted.returncode, 0, accepted.stderr)

            trust_store.write_bytes(b'{"keys":["key-2"]}\n')
            mismatch = self.run_verifier(
                "--identity",
                str(identity_path),
                "--trust-store",
                str(trust_store),
                "--review-lifecycle",
                str(lifecycle),
            )
            self.assertNotEqual(mismatch.returncode, 0)
            self.assertIn("trust_store_digest", mismatch.stderr)

            trust_store.write_bytes(b'{"keys":["key-1"]}\n')
            lifecycle.write_bytes(b'{"review_id":"review:checkout/r1","sequence":1,"state":"proposed"}\n')
            lifecycle_mismatch = self.run_verifier(
                "--identity",
                str(identity_path),
                "--trust-store",
                str(trust_store),
                "--review-lifecycle",
                str(lifecycle),
            )
            self.assertNotEqual(lifecycle_mismatch.returncode, 0)
            self.assertIn("lifecycle_digest", lifecycle_mismatch.stderr)

            partial = self.run_verifier(
                "--identity",
                str(identity_path),
                "--trust-store",
                str(trust_store),
            )
            self.assertNotEqual(partial.returncode, 0)
            self.assertIn("together", partial.stderr)

    def test_rejects_unknown_review_lifecycle_state(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            artifact = root / "program.native"
            artifact.write_bytes(b"native release program\n")
            trust_store = root / "trust-store.json"
            trust_store.write_bytes(b'{"keys":["key-1"]}\n')
            lifecycle = root / "review-lifecycle.jsonl"
            lifecycle.write_bytes(b'{"review_id":"review:checkout/r1","sequence":1,"state":"pending"}\n')
            identity_path = root / "identity.json"
            identity = identity_for(
                "sha256:" + hashlib.sha256(artifact.read_bytes()).hexdigest(),
                trust="sha256:" + hashlib.sha256(trust_store.read_bytes()).hexdigest(),
                lifecycle="sha256:" + hashlib.sha256(lifecycle.read_bytes()).hexdigest(),
            )
            self.write_identity(identity_path, identity)

            rejected = self.run_verifier(
                "--identity",
                str(identity_path),
                "--artifact",
                str(artifact),
                "--source-commit",
                SOURCE_COMMIT,
                "--trust-store",
                str(trust_store),
                "--review-lifecycle",
                str(lifecycle),
                "--require-provider-input",
            )

            self.assertNotEqual(rejected.returncode, 0)
            self.assertIn("review lifecycle state", rejected.stderr)

    def test_rejects_invalid_review_lifecycle_effective_at(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            artifact = root / "program.native"
            artifact.write_bytes(b"native release program\n")
            trust_store = root / "trust-store.json"
            trust_store.write_bytes(b'{"keys":["key-1"]}\n')
            lifecycle = root / "review-lifecycle.jsonl"
            lifecycle.write_text(
                json.dumps(
                    {
                        "review_id": "review:checkout/r1",
                        "sequence": 1,
                        "state": "active",
                        "effective_at": "2024-02-30T00:00:00Z",
                    }
                )
                + "\n",
                encoding="utf-8",
            )
            identity_path = root / "identity.json"
            identity = identity_for(
                "sha256:" + hashlib.sha256(artifact.read_bytes()).hexdigest(),
                trust="sha256:" + hashlib.sha256(trust_store.read_bytes()).hexdigest(),
                lifecycle="sha256:" + hashlib.sha256(lifecycle.read_bytes()).hexdigest(),
            )
            self.write_identity(identity_path, identity)

            rejected = self.run_verifier(
                "--identity",
                str(identity_path),
                "--artifact",
                str(artifact),
                "--source-commit",
                SOURCE_COMMIT,
                "--trust-store",
                str(trust_store),
                "--review-lifecycle",
                str(lifecycle),
                "--require-provider-input",
            )

            self.assertNotEqual(rejected.returncode, 0)
            self.assertIn("review lifecycle effective_at", rejected.stderr)

    def test_rejects_review_lifecycle_effective_at_rollback(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            artifact = root / "program.native"
            artifact.write_bytes(b"native release program\n")
            trust_store = root / "trust-store.json"
            trust_store.write_bytes(b'{"keys":["key-1"]}\n')
            lifecycle = root / "review-lifecycle.jsonl"
            lifecycle_records = [
                {
                    "review_id": "review:checkout/r1",
                    "sequence": 1,
                    "state": "proposed",
                    "effective_at": "2026-08-02T00:00:00Z",
                },
                {
                    "review_id": "review:checkout/r1",
                    "sequence": 2,
                    "state": "active",
                    "effective_at": "2026-08-01T23:59:59Z",
                },
            ]
            lifecycle.write_text(
                "".join(json.dumps(record) + "\n" for record in lifecycle_records),
                encoding="utf-8",
            )
            identity_path = root / "identity.json"
            identity = identity_for(
                "sha256:" + hashlib.sha256(artifact.read_bytes()).hexdigest(),
                trust="sha256:" + hashlib.sha256(trust_store.read_bytes()).hexdigest(),
                lifecycle="sha256:" + hashlib.sha256(lifecycle.read_bytes()).hexdigest(),
            )
            self.write_identity(identity_path, identity)

            rejected = self.run_verifier(
                "--identity",
                str(identity_path),
                "--artifact",
                str(artifact),
                "--source-commit",
                SOURCE_COMMIT,
                "--trust-store",
                str(trust_store),
                "--review-lifecycle",
                str(lifecycle),
                "--require-provider-input",
            )

            self.assertNotEqual(rejected.returncode, 0)
            self.assertIn("review lifecycle effective_at rollback", rejected.stderr)

    def test_rejects_review_lifecycle_effective_at_after_identity_now(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            artifact = root / "program.native"
            artifact.write_bytes(b"native release program\n")
            trust_store = root / "trust-store.json"
            trust_store.write_bytes(b'{"keys":["key-1"]}\n')
            lifecycle = root / "review-lifecycle.jsonl"
            lifecycle.write_text(
                json.dumps(
                    {
                        "review_id": "review:checkout/r1",
                        "sequence": 1,
                        "state": "active",
                        "effective_at": "2026-08-16T00:00:00Z",
                    }
                )
                + "\n",
                encoding="utf-8",
            )
            identity_path = root / "identity.json"
            identity = identity_for(
                "sha256:" + hashlib.sha256(artifact.read_bytes()).hexdigest(),
                trust="sha256:" + hashlib.sha256(trust_store.read_bytes()).hexdigest(),
                lifecycle="sha256:" + hashlib.sha256(lifecycle.read_bytes()).hexdigest(),
            )
            self.write_identity(identity_path, identity)

            rejected = self.run_verifier(
                "--identity",
                str(identity_path),
                "--artifact",
                str(artifact),
                "--source-commit",
                SOURCE_COMMIT,
                "--trust-store",
                str(trust_store),
                "--review-lifecycle",
                str(lifecycle),
                "--require-provider-input",
            )

            self.assertNotEqual(rejected.returncode, 0)
            self.assertIn("effective_at is after identity now", rejected.stderr)

    def test_rejects_non_positive_review_lifecycle_sequence(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            artifact = root / "program.native"
            artifact.write_bytes(b"native release program\n")
            trust_store = root / "trust-store.json"
            trust_store.write_bytes(b'{"keys":["key-1"]}\n')
            lifecycle = root / "review-lifecycle.jsonl"
            lifecycle.write_text(
                json.dumps(
                    {
                        "review_id": "review:checkout/r1",
                        "sequence": 0,
                        "state": "proposed",
                    }
                )
                + "\n",
                encoding="utf-8",
            )
            identity_path = root / "identity.json"
            identity = identity_for(
                "sha256:" + hashlib.sha256(artifact.read_bytes()).hexdigest(),
                trust="sha256:" + hashlib.sha256(trust_store.read_bytes()).hexdigest(),
                lifecycle="sha256:" + hashlib.sha256(lifecycle.read_bytes()).hexdigest(),
            )
            self.write_identity(identity_path, identity)

            rejected = self.run_verifier(
                "--identity",
                str(identity_path),
                "--artifact",
                str(artifact),
                "--source-commit",
                SOURCE_COMMIT,
                "--trust-store",
                str(trust_store),
                "--review-lifecycle",
                str(lifecycle),
                "--require-provider-input",
            )

            self.assertNotEqual(rejected.returncode, 0)
            self.assertIn("review lifecycle sequence must be a positive integer", rejected.stderr)

    def test_rejects_missing_review_lifecycle_sequence(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            artifact = root / "program.native"
            artifact.write_bytes(b"native release program\n")
            trust_store = root / "trust-store.json"
            trust_store.write_bytes(b'{"keys":["key-1"]}\n')
            lifecycle = root / "review-lifecycle.jsonl"
            lifecycle.write_text(
                json.dumps(
                    {
                        "review_id": "review:checkout/r1",
                        "state": "proposed",
                    }
                )
                + "\n",
                encoding="utf-8",
            )
            identity_path = root / "identity.json"
            identity = identity_for(
                "sha256:" + hashlib.sha256(artifact.read_bytes()).hexdigest(),
                trust="sha256:" + hashlib.sha256(trust_store.read_bytes()).hexdigest(),
                lifecycle="sha256:" + hashlib.sha256(lifecycle.read_bytes()).hexdigest(),
            )
            self.write_identity(identity_path, identity)

            rejected = self.run_verifier(
                "--identity",
                str(identity_path),
                "--artifact",
                str(artifact),
                "--source-commit",
                SOURCE_COMMIT,
                "--trust-store",
                str(trust_store),
                "--review-lifecycle",
                str(lifecycle),
                "--require-provider-input",
            )

            self.assertNotEqual(rejected.returncode, 0)
            self.assertIn("review lifecycle sequence is required", rejected.stderr)

    def test_rejects_missing_review_lifecycle_review_id(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            artifact = root / "program.native"
            artifact.write_bytes(b"native release program\n")
            trust_store = root / "trust-store.json"
            trust_store.write_bytes(b'{"keys":["key-1"]}\n')
            lifecycle = root / "review-lifecycle.jsonl"
            lifecycle.write_text(
                json.dumps(
                    {
                        "sequence": 1,
                        "state": "proposed",
                    }
                )
                + "\n",
                encoding="utf-8",
            )
            identity_path = root / "identity.json"
            identity = identity_for(
                "sha256:" + hashlib.sha256(artifact.read_bytes()).hexdigest(),
                trust="sha256:" + hashlib.sha256(trust_store.read_bytes()).hexdigest(),
                lifecycle="sha256:" + hashlib.sha256(lifecycle.read_bytes()).hexdigest(),
            )
            self.write_identity(identity_path, identity)

            rejected = self.run_verifier(
                "--identity",
                str(identity_path),
                "--artifact",
                str(artifact),
                "--source-commit",
                SOURCE_COMMIT,
                "--trust-store",
                str(trust_store),
                "--review-lifecycle",
                str(lifecycle),
                "--require-provider-input",
            )

            self.assertNotEqual(rejected.returncode, 0)
            self.assertIn("review lifecycle review_id is required", rejected.stderr)

    def test_rejects_invalid_review_lifecycle_review_id_wire_format(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            artifact = root / "program.native"
            artifact.write_bytes(b"native release program\n")
            trust_store = root / "trust-store.json"
            trust_store.write_bytes(b'{"keys":["key-1"]}\n')
            lifecycle = root / "review-lifecycle.jsonl"
            lifecycle.write_text(
                json.dumps(
                    {
                        "review_id": "review:checkout",
                        "sequence": 1,
                        "state": "proposed",
                    }
                )
                + "\n",
                encoding="utf-8",
            )
            identity_path = root / "identity.json"
            identity = identity_for(
                "sha256:" + hashlib.sha256(artifact.read_bytes()).hexdigest(),
                trust="sha256:" + hashlib.sha256(trust_store.read_bytes()).hexdigest(),
                lifecycle="sha256:" + hashlib.sha256(lifecycle.read_bytes()).hexdigest(),
            )
            self.write_identity(identity_path, identity)

            rejected = self.run_verifier(
                "--identity",
                str(identity_path),
                "--artifact",
                str(artifact),
                "--source-commit",
                SOURCE_COMMIT,
                "--trust-store",
                str(trust_store),
                "--review-lifecycle",
                str(lifecycle),
                "--require-provider-input",
            )

            self.assertNotEqual(rejected.returncode, 0)
            self.assertIn("review lifecycle review_id must use", rejected.stderr)

    def test_rejects_review_lifecycle_terminal_state_reactivation(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            artifact = root / "program.native"
            artifact.write_bytes(b"native release program\n")
            trust_store = root / "trust-store.json"
            trust_store.write_bytes(b'{"keys":["key-1"]}\n')
            lifecycle = root / "review-lifecycle.jsonl"
            lifecycle_records = [
                {
                    "review_id": "review:checkout/r1",
                    "sequence": 1,
                    "state": "proposed",
                },
                {
                    "review_id": "review:checkout/r1",
                    "sequence": 2,
                    "state": "active",
                },
                {
                    "review_id": "review:checkout/r1",
                    "sequence": 3,
                    "state": "revoked",
                },
                {
                    "review_id": "review:checkout/r1",
                    "sequence": 4,
                    "state": "active",
                },
            ]
            lifecycle.write_text(
                "".join(json.dumps(record) + "\n" for record in lifecycle_records),
                encoding="utf-8",
            )
            identity_path = root / "identity.json"
            identity = identity_for(
                "sha256:" + hashlib.sha256(artifact.read_bytes()).hexdigest(),
                trust="sha256:" + hashlib.sha256(trust_store.read_bytes()).hexdigest(),
                lifecycle="sha256:" + hashlib.sha256(lifecycle.read_bytes()).hexdigest(),
            )
            self.write_identity(identity_path, identity)

            rejected = self.run_verifier(
                "--identity",
                str(identity_path),
                "--artifact",
                str(artifact),
                "--source-commit",
                SOURCE_COMMIT,
                "--trust-store",
                str(trust_store),
                "--review-lifecycle",
                str(lifecycle),
                "--require-provider-input",
            )

            self.assertNotEqual(rejected.returncode, 0)
            self.assertIn("review lifecycle terminal state reactivation", rejected.stderr)

    def test_rejects_invalid_review_lifecycle_initial_state(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            artifact = root / "program.native"
            artifact.write_bytes(b"native release program\n")
            trust_store = root / "trust-store.json"
            trust_store.write_bytes(b'{"keys":["key-1"]}\n')
            lifecycle = root / "review-lifecycle.jsonl"
            lifecycle.write_text(
                json.dumps(
                    {
                        "review_id": "review:checkout/r1",
                        "sequence": 1,
                        "state": "revoked",
                    }
                )
                + "\n",
                encoding="utf-8",
            )
            identity_path = root / "identity.json"
            identity = identity_for(
                "sha256:" + hashlib.sha256(artifact.read_bytes()).hexdigest(),
                trust="sha256:" + hashlib.sha256(trust_store.read_bytes()).hexdigest(),
                lifecycle="sha256:" + hashlib.sha256(lifecycle.read_bytes()).hexdigest(),
            )
            self.write_identity(identity_path, identity)

            rejected = self.run_verifier(
                "--identity",
                str(identity_path),
                "--artifact",
                str(artifact),
                "--source-commit",
                SOURCE_COMMIT,
                "--trust-store",
                str(trust_store),
                "--review-lifecycle",
                str(lifecycle),
                "--require-provider-input",
            )

            self.assertNotEqual(rejected.returncode, 0)
            self.assertIn("review lifecycle initial state", rejected.stderr)

    def test_rejects_review_lifecycle_terminal_transition_before_active(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            artifact = root / "program.native"
            artifact.write_bytes(b"native release program\n")
            trust_store = root / "trust-store.json"
            trust_store.write_bytes(b'{"keys":["key-1"]}\n')
            lifecycle = root / "review-lifecycle.jsonl"
            lifecycle_records = [
                {
                    "review_id": "review:checkout/r1",
                    "sequence": 1,
                    "state": "proposed",
                },
                {
                    "review_id": "review:checkout/r1",
                    "sequence": 2,
                    "state": "revoked",
                },
            ]
            lifecycle.write_text(
                "".join(json.dumps(record) + "\n" for record in lifecycle_records),
                encoding="utf-8",
            )
            identity_path = root / "identity.json"
            identity = identity_for(
                "sha256:" + hashlib.sha256(artifact.read_bytes()).hexdigest(),
                trust="sha256:" + hashlib.sha256(trust_store.read_bytes()).hexdigest(),
                lifecycle="sha256:" + hashlib.sha256(lifecycle.read_bytes()).hexdigest(),
            )
            self.write_identity(identity_path, identity)

            rejected = self.run_verifier(
                "--identity",
                str(identity_path),
                "--artifact",
                str(artifact),
                "--source-commit",
                SOURCE_COMMIT,
                "--trust-store",
                str(trust_store),
                "--review-lifecycle",
                str(lifecycle),
                "--require-provider-input",
            )

            self.assertNotEqual(rejected.returncode, 0)
            self.assertIn("review lifecycle terminal transition requires active", rejected.stderr)

    def test_rejects_review_lifecycle_active_state_regression(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            artifact = root / "program.native"
            artifact.write_bytes(b"native release program\n")
            trust_store = root / "trust-store.json"
            trust_store.write_bytes(b'{"keys":["key-1"]}\n')
            lifecycle = root / "review-lifecycle.jsonl"
            lifecycle_records = [
                {
                    "review_id": "review:checkout/r1",
                    "sequence": 1,
                    "state": "proposed",
                },
                {
                    "review_id": "review:checkout/r1",
                    "sequence": 2,
                    "state": "active",
                },
                {
                    "review_id": "review:checkout/r1",
                    "sequence": 3,
                    "state": "proposed",
                },
            ]
            lifecycle.write_text(
                "".join(json.dumps(record) + "\n" for record in lifecycle_records),
                encoding="utf-8",
            )
            identity_path = root / "identity.json"
            identity = identity_for(
                "sha256:" + hashlib.sha256(artifact.read_bytes()).hexdigest(),
                trust="sha256:" + hashlib.sha256(trust_store.read_bytes()).hexdigest(),
                lifecycle="sha256:" + hashlib.sha256(lifecycle.read_bytes()).hexdigest(),
            )
            self.write_identity(identity_path, identity)

            rejected = self.run_verifier(
                "--identity",
                str(identity_path),
                "--artifact",
                str(artifact),
                "--source-commit",
                SOURCE_COMMIT,
                "--trust-store",
                str(trust_store),
                "--review-lifecycle",
                str(lifecycle),
                "--require-provider-input",
            )

            self.assertNotEqual(rejected.returncode, 0)
            self.assertIn("review lifecycle active state regression", rejected.stderr)

    def test_rejects_review_lifecycle_active_state_self_transition(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            artifact = root / "program.native"
            artifact.write_bytes(b"native release program\n")
            trust_store = root / "trust-store.json"
            trust_store.write_bytes(b'{"keys":["key-1"]}\n')
            lifecycle = root / "review-lifecycle.jsonl"
            lifecycle_records = [
                {
                    "review_id": "review:checkout/r1",
                    "sequence": 1,
                    "state": "proposed",
                },
                {
                    "review_id": "review:checkout/r1",
                    "sequence": 2,
                    "state": "active",
                },
                {
                    "review_id": "review:checkout/r1",
                    "sequence": 3,
                    "state": "active",
                },
            ]
            lifecycle.write_text(
                "".join(json.dumps(record) + "\n" for record in lifecycle_records),
                encoding="utf-8",
            )
            identity_path = root / "identity.json"
            identity = identity_for(
                "sha256:" + hashlib.sha256(artifact.read_bytes()).hexdigest(),
                trust="sha256:" + hashlib.sha256(trust_store.read_bytes()).hexdigest(),
                lifecycle="sha256:" + hashlib.sha256(lifecycle.read_bytes()).hexdigest(),
            )
            self.write_identity(identity_path, identity)

            rejected = self.run_verifier(
                "--identity",
                str(identity_path),
                "--artifact",
                str(artifact),
                "--source-commit",
                SOURCE_COMMIT,
                "--trust-store",
                str(trust_store),
                "--review-lifecycle",
                str(lifecycle),
                "--require-provider-input",
            )

            self.assertNotEqual(rejected.returncode, 0)
            self.assertIn("review lifecycle active state self-transition", rejected.stderr)

    def test_rejects_review_lifecycle_proposed_state_self_transition(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            artifact = root / "program.native"
            artifact.write_bytes(b"native release program\n")
            trust_store = root / "trust-store.json"
            trust_store.write_bytes(b'{"keys":["key-1"]}\n')
            lifecycle = root / "review-lifecycle.jsonl"
            lifecycle_records = [
                {
                    "review_id": "review:checkout/r1",
                    "sequence": 1,
                    "state": "proposed",
                },
                {
                    "review_id": "review:checkout/r1",
                    "sequence": 2,
                    "state": "proposed",
                },
            ]
            lifecycle.write_text(
                "".join(json.dumps(record) + "\n" for record in lifecycle_records),
                encoding="utf-8",
            )
            identity_path = root / "identity.json"
            identity = identity_for(
                "sha256:" + hashlib.sha256(artifact.read_bytes()).hexdigest(),
                trust="sha256:" + hashlib.sha256(trust_store.read_bytes()).hexdigest(),
                lifecycle="sha256:" + hashlib.sha256(lifecycle.read_bytes()).hexdigest(),
            )
            self.write_identity(identity_path, identity)

            rejected = self.run_verifier(
                "--identity",
                str(identity_path),
                "--artifact",
                str(artifact),
                "--source-commit",
                SOURCE_COMMIT,
                "--trust-store",
                str(trust_store),
                "--review-lifecycle",
                str(lifecycle),
                "--require-provider-input",
            )

            self.assertNotEqual(rejected.returncode, 0)
            self.assertIn("review lifecycle proposed state self-transition", rejected.stderr)

    def test_rejects_duplicate_review_lifecycle_sequence(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            artifact = root / "program.native"
            artifact.write_bytes(b"native release program\n")
            trust_store = root / "trust-store.json"
            trust_store.write_bytes(b'{"keys":["key-1"]}\n')
            lifecycle = root / "review-lifecycle.jsonl"
            lifecycle_records = [
                {
                    "review_id": "review:checkout/r1",
                    "sequence": 1,
                    "state": "proposed",
                },
                {
                    "review_id": "review:checkout/r1",
                    "sequence": 1,
                    "state": "active",
                },
            ]
            lifecycle.write_text(
                "".join(json.dumps(record) + "\n" for record in lifecycle_records),
                encoding="utf-8",
            )
            identity_path = root / "identity.json"
            identity = identity_for(
                "sha256:" + hashlib.sha256(artifact.read_bytes()).hexdigest(),
                trust="sha256:" + hashlib.sha256(trust_store.read_bytes()).hexdigest(),
                lifecycle="sha256:" + hashlib.sha256(lifecycle.read_bytes()).hexdigest(),
            )
            self.write_identity(identity_path, identity)

            rejected = self.run_verifier(
                "--identity",
                str(identity_path),
                "--artifact",
                str(artifact),
                "--source-commit",
                SOURCE_COMMIT,
                "--trust-store",
                str(trust_store),
                "--review-lifecycle",
                str(lifecycle),
                "--require-provider-input",
            )

            self.assertNotEqual(rejected.returncode, 0)
            self.assertIn("duplicate review lifecycle sequence", rejected.stderr)

    def test_rejects_review_lifecycle_sequence_rollback(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            artifact = root / "program.native"
            artifact.write_bytes(b"native release program\n")
            trust_store = root / "trust-store.json"
            trust_store.write_bytes(b'{"keys":["key-1"]}\n')
            lifecycle = root / "review-lifecycle.jsonl"
            lifecycle_records = [
                {
                    "review_id": "review:checkout/r1",
                    "sequence": 2,
                    "state": "active",
                },
                {
                    "review_id": "review:checkout/r1",
                    "sequence": 1,
                    "state": "revoked",
                },
            ]
            lifecycle.write_text(
                "".join(json.dumps(record) + "\n" for record in lifecycle_records),
                encoding="utf-8",
            )
            identity_path = root / "identity.json"
            identity = identity_for(
                "sha256:" + hashlib.sha256(artifact.read_bytes()).hexdigest(),
                trust="sha256:" + hashlib.sha256(trust_store.read_bytes()).hexdigest(),
                lifecycle="sha256:" + hashlib.sha256(lifecycle.read_bytes()).hexdigest(),
            )
            self.write_identity(identity_path, identity)

            rejected = self.run_verifier(
                "--identity",
                str(identity_path),
                "--artifact",
                str(artifact),
                "--source-commit",
                SOURCE_COMMIT,
                "--trust-store",
                str(trust_store),
                "--review-lifecycle",
                str(lifecycle),
                "--require-provider-input",
            )

            self.assertNotEqual(rejected.returncode, 0)
            self.assertIn("review lifecycle sequence rollback", rejected.stderr)

    def test_rejects_review_lifecycle_sequence_gap(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            artifact = root / "program.native"
            artifact.write_bytes(b"native release program\n")
            trust_store = root / "trust-store.json"
            trust_store.write_bytes(b'{"keys":["key-1"]}\n')
            lifecycle = root / "review-lifecycle.jsonl"
            lifecycle_records = [
                {
                    "review_id": "review:checkout/r1",
                    "sequence": 1,
                    "state": "proposed",
                },
                {
                    "review_id": "review:checkout/r1",
                    "sequence": 3,
                    "state": "active",
                },
            ]
            lifecycle.write_text(
                "".join(json.dumps(record) + "\n" for record in lifecycle_records),
                encoding="utf-8",
            )
            identity_path = root / "identity.json"
            identity = identity_for(
                "sha256:" + hashlib.sha256(artifact.read_bytes()).hexdigest(),
                trust="sha256:" + hashlib.sha256(trust_store.read_bytes()).hexdigest(),
                lifecycle="sha256:" + hashlib.sha256(lifecycle.read_bytes()).hexdigest(),
            )
            self.write_identity(identity_path, identity)

            rejected = self.run_verifier(
                "--identity",
                str(identity_path),
                "--artifact",
                str(artifact),
                "--source-commit",
                SOURCE_COMMIT,
                "--trust-store",
                str(trust_store),
                "--review-lifecycle",
                str(lifecycle),
                "--require-provider-input",
            )

            self.assertNotEqual(rejected.returncode, 0)
            self.assertIn("review lifecycle sequence gap", rejected.stderr)

    def test_rejects_empty_provider_snapshots(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            artifact = root / "program.native"
            artifact.write_bytes(b"native release program\n")
            trust_store = root / "trust-store.json"
            trust_store.write_bytes(b"")
            lifecycle = root / "review-lifecycle.jsonl"
            lifecycle.write_bytes(b"")
            identity_path = root / "identity.json"
            artifact_digest = "sha256:" + hashlib.sha256(artifact.read_bytes()).hexdigest()
            empty_digest = "sha256:" + hashlib.sha256(b"").hexdigest()
            self.write_identity(
                identity_path,
                identity_for(artifact_digest, trust=empty_digest, lifecycle=empty_digest),
            )

            result = self.run_verifier(
                "--identity",
                str(identity_path),
                "--source-commit",
                SOURCE_COMMIT,
                "--trust-store",
                str(trust_store),
                "--review-lifecycle",
                str(lifecycle),
                "--require-provider-input",
            )

            self.assertNotEqual(result.returncode, 0)
            self.assertIn("must be non-empty", result.stderr)

    def test_rejects_one_path_for_both_provider_roles(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            artifact = root / "program.native"
            artifact.write_bytes(b"native release program\n")
            snapshot = root / "provider-snapshot.json"
            snapshot.write_bytes(b'{"snapshot":"shared"}\n')
            identity_path = root / "identity.json"
            artifact_digest = "sha256:" + hashlib.sha256(artifact.read_bytes()).hexdigest()
            snapshot_digest = "sha256:" + hashlib.sha256(snapshot.read_bytes()).hexdigest()
            self.write_identity(
                identity_path,
                identity_for(artifact_digest, trust=snapshot_digest, lifecycle=snapshot_digest),
            )

            result = self.run_verifier(
                "--identity",
                str(identity_path),
                "--source-commit",
                SOURCE_COMMIT,
                "--trust-store",
                str(snapshot),
                "--review-lifecycle",
                str(snapshot),
                "--require-provider-input",
            )

            self.assertNotEqual(result.returncode, 0)
            self.assertIn("must be different files", result.stderr)

    def test_rejects_symlink_provider_snapshots(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            artifact = root / "program.native"
            artifact.write_bytes(b"native release program\n")
            trust_store = root / "trust-store.json"
            trust_store.write_bytes(b'{"keys":["key-1"]}\n')
            lifecycle = root / "review-lifecycle.jsonl"
            lifecycle.write_bytes(b'{"review_id":"review:checkout/r1","sequence":1,"state":"active"}\n')
            trust_store_link = root / "trust-store-link.json"
            lifecycle_link = root / "review-lifecycle-link.jsonl"
            trust_store_link.symlink_to(trust_store)
            lifecycle_link.symlink_to(lifecycle)
            artifact_digest = "sha256:" + hashlib.sha256(artifact.read_bytes()).hexdigest()
            identity_path = root / "identity.json"
            self.write_identity(
                identity_path,
                identity_for(
                    artifact_digest,
                    trust="sha256:" + hashlib.sha256(trust_store.read_bytes()).hexdigest(),
                    lifecycle="sha256:" + hashlib.sha256(lifecycle.read_bytes()).hexdigest(),
                ),
            )

            result = self.run_verifier(
                "--identity",
                str(identity_path),
                "--artifact",
                str(artifact),
                "--source-commit",
                SOURCE_COMMIT,
                "--trust-store",
                str(trust_store_link),
                "--review-lifecycle",
                str(lifecycle_link),
                "--require-provider-input",
            )

            self.assertNotEqual(result.returncode, 0)
            self.assertIn("regular non-symlink file", result.stderr)

    def test_release_surfaces_use_the_same_offline_identity_gate(self):
        project_root = SCRIPTS_DIR.parent.parent
        for relative_path in (
            "scripts/release.sh",
            "scripts/ci/release-smoke.sh",
            "scripts/ci/package-native-stage0-release.sh",
            "scripts/ci/native-official-release-local.sh",
        ):
            with self.subTest(relative_path=relative_path):
                content = (project_root / relative_path).read_text(encoding="utf-8")
                self.assertIn("verify-native-release-identity.py", content)
                self.assertIn("review_evidence_identity", content)
                if relative_path == "scripts/release.sh":
                    self.assertIn("NATIVE_ONLY_REVIEW_TRUST_STORE", content)
                    self.assertIn("NATIVE_ONLY_REVIEW_LIFECYCLE", content)
                if relative_path == "scripts/ci/package-native-stage0-release.sh":
                    self.assertIn("--review-trust-store", content)
                    self.assertIn("--review-lifecycle", content)
                if relative_path == "scripts/ci/release-smoke.sh":
                    self.assertIn("RELEASE_REVIEW_TRUST_STORE", content)
                    self.assertIn("RELEASE_REVIEW_LIFECYCLE", content)
                if relative_path == "scripts/ci/native-official-release-local.sh":
                    self.assertIn("NATIVE_OFFICIAL_REVIEW_TRUST_STORE", content)
                    self.assertIn("NATIVE_OFFICIAL_REVIEW_LIFECYCLE", content)
                    self.assertIn("--review-trust-store", content)
                    self.assertIn("--review-lifecycle", content)
                    self.assertIn("review_identity_timestamp.py", content)

    def test_native_release_packages_and_recomputes_provider_identity(self):
        project_root = SCRIPTS_DIR.parent.parent
        release_script = project_root / "scripts" / "release.sh"
        source_commit = SOURCE_COMMIT
        target = "aarch64-apple-darwin"
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            program = root / "program.native"
            program.write_bytes(b"native program fixture\n")
            program.chmod(0o755)
            program_digest = "sha256:" + hashlib.sha256(program.read_bytes()).hexdigest()
            input_manifest = root / "input-manifest.json"
            input_manifest.write_text(
                json.dumps(
                    {
                        "status": "pass",
                        "artifact_kind": "native App.Cli release program",
                        "target": target,
                        "entry_module": "App.Cli",
                        "source": "src/App/Cli.ls",
                        "source_commit": source_commit,
                        "selfhost_fixed_point": True,
                        "program_sha256": hashlib.sha256(program.read_bytes()).hexdigest(),
                    }
                )
                + "\n",
                encoding="utf-8",
            )
            identity_path = root / "identity.json"
            trust_store = root / "trust-store.json"
            trust_store.write_bytes(b'{"keys":["release-key"]}\n')
            lifecycle = root / "review-lifecycle.jsonl"
            lifecycle.write_bytes(b'{"review_id":"review:release/r1","sequence":1,"state":"active"}\n')
            identity = identity_for(
                program_digest,
                trust="sha256:" + hashlib.sha256(trust_store.read_bytes()).hexdigest(),
                lifecycle="sha256:" + hashlib.sha256(lifecycle.read_bytes()).hexdigest(),
            )
            self.write_identity(identity_path, identity)
            rollback = root / "rollback.tar.gz"
            rollback.write_bytes(b"rollback fixture\n")
            dist = root / "dist"

            environment = {
                **os.environ,
                "VERSION": "v0.0.0-identity-test",
                "TARGET": target,
                "SOURCE_COMMIT": source_commit,
                "DIST_DIR": str(dist),
                "NATIVE_ONLY_RELEASE": "1",
                "NATIVE_ONLY_PROGRAM": str(program),
                "NATIVE_ONLY_PROGRAM_MANIFEST": str(input_manifest),
                "NATIVE_ONLY_REVIEW_EVIDENCE_IDENTITY": str(identity_path),
                "NATIVE_ONLY_REVIEW_TRUST_STORE": str(trust_store),
                "NATIVE_ONLY_REVIEW_LIFECYCLE": str(lifecycle),
                "ROLLBACK_COMPATIBILITY_ASSET_PATH": str(rollback),
            }
            result = subprocess.run(
                ["bash", str(release_script)],
                cwd=project_root,
                env=environment,
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            archive_root = dist / "lsharp-v0.0.0-identity-test-aarch64-apple-darwin"
            packaged_manifest = json.loads(
                (archive_root / "manifest.json").read_text(encoding="utf-8")
            )
            self.assertEqual(
                packaged_manifest["review_evidence_identity"],
                identity,
            )
            self.assertEqual(
                json.loads(
                    (archive_root / "review-evidence-identity.json").read_text(
                        encoding="utf-8"
                    )
                ),
                identity,
            )

            trust_store.write_bytes(b'{"keys":["tampered-key"]}\n')
            rejected = subprocess.run(
                ["bash", str(release_script)],
                cwd=project_root,
                env={
                    **environment,
                    "VERSION": "v0.0.0-identity-mismatch-test",
                    "DIST_DIR": str(root / "mismatch-dist"),
                },
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertNotEqual(rejected.returncode, 0)
            self.assertIn("trust_store_digest", rejected.stderr)

            partial_identity = subprocess.run(
                ["bash", str(release_script)],
                cwd=project_root,
                env={
                    **environment,
                    "VERSION": "v0.0.0-identity-partial-test",
                    "DIST_DIR": str(root / "partial-dist"),
                    "NATIVE_ONLY_REVIEW_EVIDENCE_IDENTITY": "",
                },
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertNotEqual(partial_identity.returncode, 0)
            self.assertIn("require NATIVE_ONLY_REVIEW_EVIDENCE_IDENTITY", partial_identity.stderr)

    def test_rejects_unsafe_release_version_before_output_directory(self):
        project_root = SCRIPTS_DIR.parent.parent
        release_script = project_root / "scripts" / "release.sh"
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            dist = root / "dist"
            result = subprocess.run(
                ["bash", str(release_script)],
                cwd=project_root,
                env={
                    **os.environ,
                    "VERSION": "v1/unsafe",
                    "TARGET": "aarch64-apple-darwin",
                    "SOURCE_COMMIT": SOURCE_COMMIT,
                    "DIST_DIR": str(dist),
                    "NATIVE_ONLY_RELEASE": "1",
                },
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("version must contain only", result.stderr)
            self.assertFalse(dist.exists())


if __name__ == "__main__":
    unittest.main(verbosity=2)
