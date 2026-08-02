#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SOURCE_SMOKE="$ROOT/scripts/ci/native-selfhost-dev-source-file-smoke.sh"
WRITER="$ROOT/scripts/ci/write-native-source-smoke-evidence.py"

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

assert_file_contains() {
  local path="$1"
  local expected="$2"
  grep -F -- "$expected" "$path" >/dev/null || fail "$path does not contain: $expected"
}

[[ -f "$SOURCE_SMOKE" ]] || fail "source smoke is missing: $SOURCE_SMOKE"
[[ -f "$WRITER" ]] || fail "source smoke evidence writer is missing: $WRITER"
assert_file_contains "$SOURCE_SMOKE" 'NATIVE_SELFHOST_SOURCE_SMOKE_EVIDENCE_DIR'
assert_file_contains "$SOURCE_SMOKE" 'write-native-source-smoke-evidence.py'
assert_file_contains "$SOURCE_SMOKE" '--review-attestation-report'
python3 - "$SOURCE_SMOKE" <<'PY'
import pathlib
import sys

source = pathlib.Path(sys.argv[1]).read_text(encoding="utf-8")
broken = 'python3 \\\n  "$VALIDATION_ATTESTATION_SOURCE"'
if broken in source:
    raise SystemExit("attestation fixture generator must pass '-' to python3 stdin")
PY

TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/lsharp-native-source-smoke-evidence.XXXXXX")"
trap 'rm -rf "$TMP_ROOT"' EXIT

WORK_DIR="$TMP_ROOT/work"
EVIDENCE_DIR="$TMP_ROOT/evidence"
STAGE0_DIR="$TMP_ROOT/stage0"
STAGE0_MANIFEST="$STAGE0_DIR/manifest.json"
ATTESTATION_REPORT="$TMP_ROOT/validation-attestation-json.stdout"
mkdir -p "$WORK_DIR" "$STAGE0_DIR/bin"
printf 'decls:1\n' >"$WORK_DIR/parse.stdout"
printf 'compile-bytes' >"$WORK_DIR/compile.wasm"
printf 'build-bytes' >"$WORK_DIR/build.wasm"
printf 'compiler-bytes' >"$STAGE0_DIR/bin/compiler"
cat >"$ATTESTATION_REPORT" <<'JSON'
{"review_attestations":[{"review_id":"review:checkout/reviewer-001","subject_digest":"sha256:subject-001","source_commit":"0123456789abcdef","provenance_digest":"sha256:review-001","provider":"github","key_id":"org/reviews-2026","algorithm":"ed25519","signature":"AAECAw","issued_at":"2026-08-01T00:00:00Z","expires_at":"2026-09-01T00:00:00Z","sequence":3,"state":"unverified","canonical_bytes":[0,1,2],"span":{"start":12,"end":34}}]}
JSON
cat >"$STAGE0_MANIFEST" <<'JSON'
{
  "kind": "lsharp-native-selfhost-stage0",
  "target": "aarch64-apple-darwin",
  "source_commit": "0123456789012345678901234567890123456789",
  "compiler": "bin/compiler"
}
JSON

python3 "$WRITER" \
  --evidence-dir "$EVIDENCE_DIR" \
  --work-dir "$WORK_DIR" \
  --stage0-dir "$STAGE0_DIR" \
  --stage0-manifest "$STAGE0_MANIFEST" \
  --review-attestation-report "$ATTESTATION_REPORT" \
  --target aarch64-apple-darwin \
  --exit-code 7

[[ "$(cat "$EVIDENCE_DIR/exit.code")" == "7" ]] || fail "exit code evidence was not preserved"
[[ "$(cat "$EVIDENCE_DIR/work/parse.stdout")" == "decls:1" ]] || fail "stdout evidence was not preserved"
cmp -s "$WORK_DIR/compile.wasm" "$EVIDENCE_DIR/work/compile.wasm" \
  || fail "Wasm evidence was not preserved"
[[ -s "$EVIDENCE_DIR/stage0-manifest.json" ]] || fail "stage0 manifest evidence is missing"

python3 - "$EVIDENCE_DIR/manifest.json" "$STAGE0_DIR" <<'PY'
import hashlib
import json
import pathlib
import sys

manifest = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
stage0_dir = pathlib.Path(sys.argv[2])
records = []
for path in sorted(stage0_dir.rglob("*")):
    if path.is_file() and not path.is_symlink():
        records.append(
            path.relative_to(stage0_dir).as_posix().encode()
            + b"\0"
            + str(path.stat().st_size).encode()
            + b"\0"
            + hashlib.sha256(path.read_bytes()).hexdigest().encode()
            + b"\n"
        )
expected_payload_digest = hashlib.sha256(b"".join(records)).hexdigest()
assert manifest["kind"] == "lsharp-native-selfhost-source-smoke-evidence"
assert manifest["target"] == "aarch64-apple-darwin"
assert manifest["source_commit"] == "0123456789012345678901234567890123456789"
assert manifest["stage0_payload_sha256"] == expected_payload_digest
assert manifest["exit_code"] == 7
assert manifest["artifacts"]["compile.wasm"]["size"] == len(b"compile-bytes")
assert manifest["review_attestations"] == [{
    "review_id": "review:checkout/reviewer-001",
    "subject_digest": "sha256:subject-001",
    "source_commit": "0123456789abcdef",
    "provenance_digest": "sha256:review-001",
    "provider": "github",
    "key_id": "org/reviews-2026",
    "algorithm": "ed25519",
    "signature": "AAECAw",
    "issued_at": "2026-08-01T00:00:00Z",
    "expires_at": "2026-09-01T00:00:00Z",
    "sequence": 3,
    "state": "unverified",
    "canonical_bytes": [0, 1, 2],
    "span": {"start": 12, "end": 34},
}]
PY

NO_REPORT_EVIDENCE_DIR="$TMP_ROOT/no-report-evidence"
python3 "$WRITER" \
  --evidence-dir "$NO_REPORT_EVIDENCE_DIR" \
  --work-dir "$WORK_DIR" \
  --stage0-dir "$STAGE0_DIR" \
  --stage0-manifest "$STAGE0_MANIFEST" \
  --target aarch64-apple-darwin \
  --exit-code 0
python3 - "$NO_REPORT_EVIDENCE_DIR/manifest.json" <<'PY'
import json
import pathlib
import sys

manifest = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
assert "review_attestations" not in manifest
PY

SYMLINK_TARGET="$TMP_ROOT/outside.txt"
SYMLINK_EVIDENCE_DIR="$TMP_ROOT/symlink-evidence"
printf 'outside-evidence\n' >"$SYMLINK_TARGET"
ln -s "$SYMLINK_TARGET" "$WORK_DIR/linked.stdout"
if python3 "$WRITER" \
  --evidence-dir "$SYMLINK_EVIDENCE_DIR" \
  --work-dir "$WORK_DIR" \
  --stage0-dir "$STAGE0_DIR" \
  --stage0-manifest "$STAGE0_MANIFEST" \
  --review-attestation-report "$ATTESTATION_REPORT" \
  --target aarch64-apple-darwin \
  --exit-code 0; then
  fail "evidence writer accepted a symlink inside the work directory"
fi
[[ ! -e "$SYMLINK_EVIDENCE_DIR" ]] || fail "symlink work input created evidence output"
rm "$WORK_DIR/linked.stdout"

STAGE0_SYMLINK_EVIDENCE_DIR="$TMP_ROOT/stage0-symlink-evidence"
ln -s "$STAGE0_DIR/bin/compiler" "$STAGE0_DIR/bin/linked-compiler"
if python3 "$WRITER" \
  --evidence-dir "$STAGE0_SYMLINK_EVIDENCE_DIR" \
  --work-dir "$WORK_DIR" \
  --stage0-dir "$STAGE0_DIR" \
  --stage0-manifest "$STAGE0_MANIFEST" \
  --target aarch64-apple-darwin \
  --exit-code 0; then
  fail "evidence writer accepted a symlink inside the stage0 directory"
fi
[[ ! -e "$STAGE0_SYMLINK_EVIDENCE_DIR" ]] \
  || fail "stage0 symlink input created evidence output"
rm "$STAGE0_DIR/bin/linked-compiler"

MISSING_ATTESTATION_REPORT="$TMP_ROOT/missing-attestation.stdout"
MISSING_ATTESTATION_EVIDENCE_DIR="$TMP_ROOT/missing-attestation-evidence"
if python3 "$WRITER" \
  --evidence-dir "$MISSING_ATTESTATION_EVIDENCE_DIR" \
  --work-dir "$WORK_DIR" \
  --stage0-dir "$STAGE0_DIR" \
  --stage0-manifest "$STAGE0_MANIFEST" \
  --review-attestation-report "$MISSING_ATTESTATION_REPORT" \
  --target aarch64-apple-darwin \
  --exit-code 0; then
  fail "evidence writer accepted a missing attestation report"
fi
[[ ! -e "$MISSING_ATTESTATION_EVIDENCE_DIR" ]] || fail "missing attestation report created evidence output"

UPPER_MANIFEST="$TMP_ROOT/stage0-manifest-uppercase.json"
UPPER_EVIDENCE_DIR="$TMP_ROOT/uppercase-evidence"
python3 - "$STAGE0_MANIFEST" "$UPPER_MANIFEST" <<'PY'
import json
import pathlib
import sys

source = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
source["source_commit"] = "ABCDEF0123456789ABCDEF0123456789ABCDEF01"
pathlib.Path(sys.argv[2]).write_text(json.dumps(source), encoding="utf-8")
PY
if python3 "$WRITER" \
  --evidence-dir "$UPPER_EVIDENCE_DIR" \
  --work-dir "$WORK_DIR" \
  --stage0-dir "$STAGE0_DIR" \
  --stage0-manifest "$UPPER_MANIFEST" \
  --target aarch64-apple-darwin \
  --exit-code 0; then
  fail "evidence writer accepted an uppercase source_commit"
fi
[[ ! -e "$UPPER_EVIDENCE_DIR" ]] || fail "uppercase source_commit created evidence output"

if python3 "$WRITER" \
  --evidence-dir "$EVIDENCE_DIR" \
  --work-dir "$WORK_DIR" \
  --stage0-dir "$STAGE0_DIR" \
  --stage0-manifest "$STAGE0_MANIFEST" \
  --target aarch64-apple-darwin \
  --exit-code 0; then
  fail "evidence writer overwrote an existing evidence directory"
fi

echo "native source smoke evidence contract test passed"
