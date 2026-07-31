#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SMOKE="$ROOT/scripts/ci/native-linux-x86-native-stage0-source-file-smoke.sh"
SOURCE_SMOKE="$ROOT/scripts/ci/native-selfhost-dev-source-file-smoke.sh"
SOURCE_COMMIT="$(git rev-parse --verify HEAD)"

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

assert_script_contains() {
  local path="$1"
  local expected="$2"
  grep -F -- "$expected" "$path" >/dev/null \
    || fail "$path does not contain: $expected"
}

expect_reject() {
  local label="$1"
  shift

  local output
  local exit_code
  set +e
  output="$($@ 2>&1)"
  exit_code=$?
  set -e
  [[ "$exit_code" -ne 0 ]] || fail "$label unexpectedly succeeded"
  [[ -n "$output" ]] || fail "$label did not report an error"
}

[[ -x "$SMOKE" ]] || fail "Linux native stage0 source-file smoke is missing: $SMOKE"
[[ -x "$SOURCE_SMOKE" ]] || fail "native selfhost source-file smoke is missing: $SOURCE_SMOKE"

for required in \
  'run_expected_failure validation-text-unknown' \
  '--source "$VALIDATION_SOURCE"' \
  '--format text' \
  'require_exact_output validation-text-unknown' \
  'status: unknown' \
  'open-questions: 1' \
  'independent-reviews: 0' \
  'contradicting-observations: 0' \
  'stale-reviews: 0' \
  'stale-evidence: 0' \
  'run_expected_validation_error validation-manifest-write-failure' \
  'VALIDATION_WRITE_FAILURE_MANIFEST' \
  'missing-parent' \
  '--emit-manifest "$VALIDATION_WRITE_FAILURE_MANIFEST"' \
  'source validation manifest write failed' \
  '[[ ! -e "$VALIDATION_WRITE_FAILURE_MANIFEST" ]]' \
  'VALIDATION_PASS_SOURCE' \
  'validation-pass-json' \
  'validation-pass-text' \
  'status: pass' \
  'independent-reviews: 1' \
  'VALIDATION_FAILED_REVIEW_SOURCE' \
  'validation-failed-review-json' \
  'validation-failed-review-text' \
  'failed independent review must leave validation unknown' \
  'independent-reviews: 0' \
  'validation-manifest-roundtrip-json' \
  'validation manifest roundtrip must preserve source report' \
  'VALIDATION_FAIL_SOURCE' \
  'validation-fail-json' \
  'validation-fail-text' \
  'run_report_failure' \
  'status: fail' \
  'contradicting-observations: 1' \
  'VALIDATION_STALE_SOURCE' \
  'run_expected_failure validation-stale-json' \
  'validation-stale-json' \
  'run_expected_failure validation-stale-text' \
  'validation-stale-text' \
  'stale-reviews: 1' \
  'stale-evidence: 1' \
  'VALIDATION_ATTESTATION_SOURCE' \
  'ec-m3-review-attestation-source.ls' \
  'VALIDATION_ATTESTATION_MANIFEST' \
  'VALIDATION_ATTESTATION_NO_EXPIRY_SOURCE' \
  'VALIDATION_ATTESTATION_NO_EXPIRY_MANIFEST' \
  'run_expected_failure validation-attestation-json' \
  'validation-attestation-no-expiry-json' \
  'report.get("review_verifications")' \
  'report.get("review_attestations")' \
  'attestation.get("subject_digest")' \
  'attestation.get("source_commit")' \
  'attestation.get("provenance_digest")' \
  'attestation.get("provider")' \
  'attestation.get("key_id")' \
  'attestation.get("algorithm")' \
  'attestation.get("signature")' \
  'attestation.get("issued_at")' \
  'attestation.get("expires_at")' \
  'attestation.get("sequence")' \
  'attestation.get("state")' \
  'attestation.get("canonical_bytes")' \
  'attestation.get("span")' \
  'expected_attestation_fields' \
  'review.get("verification_state") != "unverified"' \
  'run_expected_failure validation-attestation-text' \
  'validation-attestation-no-expiry-text' \
  'review-verification: review:checkout/reviewer-001=unverified' \
  'VALIDATION_IDENTITY_MANIFEST' \
  'validation-identity-json' \
  'VALIDATION_IDENTITY_OPTIONAL_MANIFEST' \
  'validation-identity-optional-json' \
  'validation-identity-text' \
  'review_evidence_identity' \
  'review-evidence-identity: subject=sha256:graph source=commit-1 artifact=sha256:artifact trust-store=- lifecycle=- now=2026-08-15T00:00:00Z' \
  'validation-identity-partial' \
  'review identity requires --review-subject-digest --review-source-commit --review-artifact-digest --review-now' \
  'VALIDATION_INVALID_ATTESTATION_ALGORITHM_SOURCE' \
  'VALIDATION_INVALID_ATTESTATION_SIGNATURE_SOURCE' \
  'VALIDATION_INVALID_ATTESTATION_TIMESTAMP_SOURCE' \
  'VALIDATION_INVALID_ATTESTATION_WINDOW_SOURCE' \
  'run_invalid_attestation()' \
  'validation-invalid-attestation-algorithm' \
  'validation-invalid-attestation-signature' \
  'validation-invalid-attestation-timestamp' \
  'validation-invalid-attestation-window' \
  'VALIDATION_ORPHAN_SOURCE' \
  'VALIDATION_ORPHAN_MANIFEST' \
  'run_expected_validation_error validation-orphan' \
  'source validation error:5' \
  'VALIDATION_MALFORMED_SOURCE' \
  'VALIDATION_MALFORMED_MANIFEST' \
  'run_expected_validation_error validation-malformed-edge' \
  'source validation error:1' \
  'VALIDATION_INVALID_ID_SOURCE' \
  'VALIDATION_INVALID_ID_MANIFEST' \
  'run_expected_validation_error validation-invalid-id' \
  'source validation error:2' \
  'VALIDATION_KIND_MISMATCH_SOURCE' \
  'VALIDATION_KIND_MISMATCH_MANIFEST' \
  'run_expected_validation_error validation-kind-mismatch' \
  'source validation error:3' \
  'VALIDATION_EVIDENCE_REGISTRY_SOURCE' \
  'VALIDATION_EVIDENCE_REGISTRY_MANIFEST' \
  'run_expected_validation_error validation-evidence-registry' \
  'source validation error:6' \
  'VALIDATION_SUPPORTS_EVIDENCE_PRECEDENCE_SOURCE' \
  'VALIDATION_SUPPORTS_EVIDENCE_PRECEDENCE_MANIFEST' \
  'run_expected_validation_error validation-supports-evidence-precedence' \
  'unregistered supports evidence must win over its invalid wire ID' \
  'VALIDATION_CONTRADICTS_EVIDENCE_PRECEDENCE_SOURCE' \
  'VALIDATION_CONTRADICTS_EVIDENCE_PRECEDENCE_MANIFEST' \
  'run_expected_validation_error validation-contradicts-evidence-precedence' \
  'unregistered contradicts evidence must win over its invalid wire ID' \
  'VALIDATION_MALFORMED_EVIDENCE_SOURCE' \
  'VALIDATION_MALFORMED_EVIDENCE_MANIFEST' \
  'run_expected_validation_error validation-malformed-evidence' \
  'source validation error:1' \
  'VALIDATION_INVALID_EVIDENCE_SOURCE' \
  'VALIDATION_INVALID_EVIDENCE_MANIFEST' \
  'run_expected_validation_error validation-invalid-evidence' \
  'source validation error:8' \
  'VALIDATION_INVALID_EVIDENCE_OUTCOME_SOURCE' \
  'VALIDATION_INVALID_EVIDENCE_OUTCOME_MANIFEST' \
  'run_expected_validation_error validation-invalid-evidence-outcome' \
  'VALIDATION_INVALID_EVIDENCE_INDEPENDENCE_SOURCE' \
  'VALIDATION_INVALID_EVIDENCE_INDEPENDENCE_MANIFEST' \
  'run_expected_validation_error validation-invalid-evidence-independence' \
  'VALIDATION_INVALID_EVIDENCE_SUBJECT_SOURCE' \
  'VALIDATION_INVALID_EVIDENCE_SUBJECT_MANIFEST' \
  'run_expected_validation_error validation-invalid-evidence-subject' \
  'VALIDATION_EMPTY_EVIDENCE_METHOD_SOURCE' \
  'VALIDATION_EMPTY_EVIDENCE_METHOD_MANIFEST' \
  'run_expected_validation_error validation-empty-evidence-method' \
  'source validation error:8' \
  'VALIDATION_EMPTY_EVIDENCE_OUTCOME_SOURCE' \
  'VALIDATION_EMPTY_EVIDENCE_OUTCOME_MANIFEST' \
  'run_expected_validation_error validation-empty-evidence-outcome' \
  'VALIDATION_EMPTY_EVIDENCE_INDEPENDENCE_SOURCE' \
  'VALIDATION_EMPTY_EVIDENCE_INDEPENDENCE_MANIFEST' \
  'run_expected_validation_error validation-empty-evidence-independence' \
  'VALIDATION_DUPLICATE_EVIDENCE_SOURCE' \
  'VALIDATION_DUPLICATE_EVIDENCE_MANIFEST' \
  'run_expected_validation_error validation-duplicate-evidence' \
  'source validation error:3' \
  'VALIDATION_DUPLICATE_EVIDENCE_FIELD_SOURCE' \
  'VALIDATION_DUPLICATE_EVIDENCE_FIELD_MANIFEST' \
  'run_expected_validation_error validation-duplicate-evidence-field' \
  'source validation error:1' \
  'VALIDATION_EMPTY_EVIDENCE_GENERATOR_SOURCE' \
  'VALIDATION_EMPTY_EVIDENCE_GENERATOR_MANIFEST' \
  'run_expected_validation_error validation-empty-evidence-generator' \
  'source validation error:4' \
  'VALIDATION_EMPTY_EVIDENCE_RUNNER_SOURCE' \
  'VALIDATION_EMPTY_EVIDENCE_RUNNER_MANIFEST' \
  'run_expected_validation_error validation-empty-evidence-runner' \
  'source validation error:4' \
  'VALIDATION_EMPTY_EVIDENCE_TARGET_SOURCE' \
  'VALIDATION_EMPTY_EVIDENCE_TARGET_MANIFEST' \
  'run_expected_validation_error validation-empty-evidence-target' \
  'source validation error:4' \
  'VALIDATION_EMPTY_EVIDENCE_SOURCE_COMMIT_SOURCE' \
  'VALIDATION_EMPTY_EVIDENCE_SOURCE_COMMIT_MANIFEST' \
  'run_expected_validation_error validation-empty-evidence-source-commit' \
  'source validation error:4' \
  'VALIDATION_EMPTY_EVIDENCE_ARTIFACT_DIGEST_SOURCE' \
  'VALIDATION_EMPTY_EVIDENCE_ARTIFACT_DIGEST_MANIFEST' \
  'run_expected_validation_error validation-empty-evidence-artifact-digest' \
  'source validation error:4' \
  'VALIDATION_EMPTY_EVIDENCE_PRODUCER_SOURCE' \
  'VALIDATION_EMPTY_EVIDENCE_PRODUCER_MANIFEST' \
  'run_expected_validation_error validation-empty-evidence-producer' \
  'source validation error:4' \
  'VALIDATION_EMPTY_EVIDENCE_TOOL_VERSION_SOURCE' \
  'VALIDATION_EMPTY_EVIDENCE_TOOL_VERSION_MANIFEST' \
  'run_expected_validation_error validation-empty-evidence-tool-version' \
  'source validation error:4' \
  'VALIDATION_EMPTY_EVIDENCE_TIMESTAMP_SOURCE' \
  'VALIDATION_EMPTY_EVIDENCE_TIMESTAMP_MANIFEST' \
  'run_expected_validation_error validation-empty-evidence-timestamp' \
  'source validation error:4' \
  'VALIDATION_WHITESPACE_EVIDENCE_RUNNER_SOURCE' \
  'VALIDATION_WHITESPACE_EVIDENCE_RUNNER_MANIFEST' \
  'run_expected_validation_error validation-whitespace-evidence-runner' \
  'source validation error:4' \
  'VALIDATION_UNICODE_WHITESPACE_EVIDENCE_COVERAGE_SOURCE' \
  'VALIDATION_UNICODE_WHITESPACE_EVIDENCE_COVERAGE_MANIFEST' \
  'run_expected_validation_error validation-unicode-whitespace-evidence-coverage' \
  'source validation error:4' \
  'VALIDATION_NEGATIVE_EVIDENCE_COVERAGE_SOURCE' \
  'VALIDATION_NEGATIVE_EVIDENCE_COVERAGE_MANIFEST' \
  'run_expected_validation_error validation-negative-evidence-coverage' \
  'source validation error:11' \
  'VALIDATION_NEGATIVE_EVIDENCE_CASES_SOURCE' \
  'VALIDATION_NEGATIVE_EVIDENCE_CASES_MANIFEST' \
  'run_expected_validation_error validation-negative-evidence-cases' \
  'source validation error:11' \
  'VALIDATION_NEGATIVE_EVIDENCE_SEED_SOURCE' \
  'VALIDATION_NEGATIVE_EVIDENCE_SEED_MANIFEST' \
  'run_expected_validation_error validation-negative-evidence-seed' \
  'source validation error:11' \
  'VALIDATION_NEGATIVE_EVIDENCE_SHRINKS_SOURCE' \
  'VALIDATION_NEGATIVE_EVIDENCE_SHRINKS_MANIFEST' \
  'run_expected_validation_error validation-negative-evidence-shrinks' \
  'source validation error:11' \
  'VALIDATION_WHITESPACE_EVIDENCE_SUBJECT_SOURCE' \
  'VALIDATION_WHITESPACE_EVIDENCE_SUBJECT_MANIFEST' \
  'run_expected_validation_error validation-whitespace-evidence-subject' \
  'source validation error:2' \
  'VALIDATION_WHITESPACE_EVIDENCE_ID_SOURCE' \
  'VALIDATION_WHITESPACE_EVIDENCE_ID_MANIFEST' \
  'run_expected_validation_error validation-whitespace-evidence-id' \
  'source validation error:2' \
  'VALIDATION_EMPTY_EVIDENCE_ID_SOURCE' \
  'VALIDATION_EMPTY_EVIDENCE_ID_MANIFEST' \
  'run_expected_validation_error validation-empty-evidence-id' \
  'source validation error:2' \
  'VALIDATION_MISSING_NODE_ID_SOURCE' \
  'VALIDATION_MISSING_NODE_ID_MANIFEST' \
  'run_expected_validation_error validation-missing-node-id' \
  'source validation error:1' \
  'VALIDATION_MISSING_NODE_TEXT_SOURCE' \
  'VALIDATION_MISSING_NODE_TEXT_MANIFEST' \
  'run_expected_validation_error validation-missing-node-text' \
  'source validation error:1' \
  'VALIDATION_WHITESPACE_NODE_TEXT_SOURCE' \
  'VALIDATION_WHITESPACE_NODE_TEXT_MANIFEST' \
  'run_expected_validation_error validation-whitespace-node-text' \
  'source validation error:1' \
  'VALIDATION_MISSING_REVIEW_SOURCE' \
  'VALIDATION_MISSING_REVIEW_MANIFEST' \
  'run_expected_validation_error validation-missing-review' \
  'source validation error:10' \
  'VALIDATION_MISSING_REVIEW_SUBJECT_KIND_SOURCE' \
  'VALIDATION_MISSING_REVIEW_SUBJECT_KIND_MANIFEST' \
  'run_expected_validation_error validation-missing-review-before-subject-kind' \
  'missing review must win over invalid evaluates subject kind' \
  'VALIDATION_DUPLICATE_REVIEW_SOURCE' \
  'VALIDATION_DUPLICATE_REVIEW_MANIFEST' \
  'run_expected_validation_error validation-duplicate-review' \
  'source validation error:7' \
  'VALIDATION_INVALID_REVIEW_SOURCE' \
  'VALIDATION_INVALID_REVIEW_MANIFEST' \
  'run_expected_validation_error validation-invalid-review' \
  'source validation error:8' \
  'VALIDATION_INVALID_REVIEW_DIGEST_SOURCE' \
  'VALIDATION_INVALID_REVIEW_DIGEST_MANIFEST' \
  'run_expected_validation_error validation-invalid-review-digest' \
  'source validation error:8' \
  'VALIDATION_INVALID_REVIEW_ID_SOURCE' \
  'VALIDATION_INVALID_REVIEW_ID_MANIFEST' \
  'run_expected_validation_error validation-invalid-review-id' \
  'source validation error:2' \
  'VALIDATION_EMPTY_REVIEW_ID_SOURCE' \
  'VALIDATION_EMPTY_REVIEW_ID_MANIFEST' \
  'run_expected_validation_error validation-empty-review-id' \
  'source validation error:8' \
  'VALIDATION_MALFORMED_REVIEW_SOURCE' \
  'VALIDATION_MALFORMED_REVIEW_MANIFEST' \
  'run_expected_validation_error validation-malformed-review' \
  'source validation error:1' \
  'VALIDATION_MALFORMED_REVIEW_EXTRA_SOURCE' \
  'VALIDATION_MALFORMED_REVIEW_EXTRA_MANIFEST' \
  'run_expected_validation_error validation-malformed-review-extra' \
  'source validation error:1' \
  'VALIDATION_MALFORMED_REVIEW_EDGE_SOURCE' \
  'VALIDATION_MALFORMED_REVIEW_EDGE_MANIFEST' \
  'run_expected_validation_error validation-malformed-review-edge' \
  'source validation error:1' \
  'VALIDATION_MALFORMED_INVALIDATION_EDGE_SOURCE' \
  'VALIDATION_MALFORMED_INVALIDATION_EDGE_MANIFEST' \
  'run_expected_validation_error validation-malformed-invalidation-edge' \
  'source validation error:1' \
  'VALIDATION_MALFORMED_REVIEW_EDGE_EXTRA_SOURCE' \
  'VALIDATION_MALFORMED_REVIEW_EDGE_EXTRA_MANIFEST' \
  'run_expected_validation_error validation-malformed-review-edge-extra' \
  'source validation error:1' \
  'VALIDATION_MALFORMED_INVALIDATION_EDGE_EXTRA_SOURCE' \
  'VALIDATION_MALFORMED_INVALIDATION_EDGE_EXTRA_MANIFEST' \
  'run_expected_validation_error validation-malformed-invalidation-edge-extra' \
  'source validation error:1' \
  'VALIDATION_REVIEW_SUBJECT_KIND_SOURCE' \
  'VALIDATION_REVIEW_SUBJECT_KIND_MANIFEST' \
  'run_expected_validation_error validation-review-subject-kind' \
  'source validation error:9' \
  'VALIDATION_INVALIDATION_SUBJECT_KIND_SOURCE' \
  'VALIDATION_INVALIDATION_SUBJECT_KIND_MANIFEST' \
  'run_expected_validation_error validation-invalidation-subject-kind' \
  'source validation error:9' \
  'VALIDATION_INVALIDATION_MISSING_REVIEW_SOURCE' \
  'VALIDATION_INVALIDATION_MISSING_REVIEW_MANIFEST' \
  'run_expected_validation_error validation-invalidation-missing-review' \
  'source validation error:10' \
  'VALIDATION_REVIEW_EDGE_EVIDENCE_SOURCE' \
  'VALIDATION_REVIEW_EDGE_EVIDENCE_MANIFEST' \
  'run_expected_validation_error validation-review-edge-evidence' \
  'source validation error:6' \
  'VALIDATION_INVALIDATION_EDGE_EVIDENCE_SOURCE' \
  'VALIDATION_INVALIDATION_EDGE_EVIDENCE_MANIFEST' \
  'run_expected_validation_error validation-invalidation-edge-evidence' \
  'source validation error:6'; do
  assert_script_contains "$SOURCE_SMOKE" "$required"
done

TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/lsharp-linux-stage0-source-smoke-test.XXXXXX")"
trap 'rm -rf "$TMP_ROOT"' EXIT
LOG_PATH="$TMP_ROOT/running-vm.log"
STOPPED_LOG="$TMP_ROOT/stopped-vm.log"
STOP_FAILURE_LOG="$TMP_ROOT/stop-failure.log"

STAGE0_DIR="$TMP_ROOT/stage0"
BIN_DIR="$TMP_ROOT/bin"
mkdir -p "$STAGE0_DIR/bin" "$BIN_DIR"

for executable in compiler transport-driver materializer; do
  printf '%s\n' '#!/usr/bin/env bash' 'exit 0' >"$STAGE0_DIR/bin/$executable"
  chmod 0755 "$STAGE0_DIR/bin/$executable"
done

cat >"$STAGE0_DIR/manifest.json" <<JSON
{
  "kind": "lsharp-native-selfhost-stage0",
  "target": "x86_64-unknown-linux-gnu",
  "source_commit": "$SOURCE_COMMIT",
  "compiler": "bin/compiler",
  "transport_driver": "bin/transport-driver",
  "materializer": "bin/materializer"
}
JSON

cat >"$BIN_DIR/uname" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
case "${1:-}" in
  -s) printf 'Darwin\n' ;;
  -m) printf 'arm64\n' ;;
  *) exit 1 ;;
esac
SH
chmod 0755 "$BIN_DIR/uname"

cat >"$BIN_DIR/limactl" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "limactl $*" >>"${FAKE_LOG:-/dev/null}"
if [[ "${FAIL_LIMACTL_STOP:-0}" == "1" && "${1:-}" == "stop" ]]; then
  exit 19
fi
case "${1:-}" in
  list)
    if [[ "${FAKE_LIMA_RUNNING:-1}" == "1" ]]; then
      printf 'Running\n'
    else
      printf 'Stopped\n'
    fi
    ;;
  start|stop)
    ;;
  shell)
    if [[ "$*" == *"df -Pk /tmp"* ]]; then
      printf '10485760\n'
    fi
    ;;
  copy)
    ;;
  *)
    printf 'unexpected limactl invocation: %s\n' "$*" >&2
    exit 1
    ;;
esac
SH
chmod 0755 "$BIN_DIR/limactl"

run_smoke() {
  PATH="$BIN_DIR:$PATH" \
    FAKE_LIMA_RUNNING=1 \
    FAKE_LOG="$LOG_PATH" \
    LSHARP_NATIVE_LINUX_X86_STAGE0_DIR="$STAGE0_DIR" \
    LSHARP_NATIVE_LINUX_X86_KEEP_NATIVE_STAGE0_SOURCE_SMOKE_WORK_DIR=1 \
    "$SMOKE"
}

run_smoke

python3 - "$STAGE0_DIR/manifest.json" <<'PY'
import json
import sys

path = sys.argv[1]
manifest = json.load(open(path, encoding="utf-8"))
manifest["source_commit"] = "0" * 40
with open(path, "w", encoding="utf-8") as handle:
    json.dump(manifest, handle)
    handle.write("\n")
PY

expect_reject "stale stage0 source commit" run_smoke

python3 - "$STAGE0_DIR/manifest.json" "$SOURCE_COMMIT" <<'PY'
import json
import sys

path, source_commit = sys.argv[1:]
manifest = json.load(open(path, encoding="utf-8"))
manifest["source_commit"] = source_commit
with open(path, "w", encoding="utf-8") as handle:
    json.dump(manifest, handle)
    handle.write("\n")
PY

run_stopped_vm_smoke() {
  PATH="$BIN_DIR:$PATH" \
    FAKE_LIMA_RUNNING=0 \
    FAKE_LOG="$STOPPED_LOG" \
    LSHARP_NATIVE_LINUX_X86_STAGE0_DIR="$STAGE0_DIR" \
    LSHARP_NATIVE_LINUX_X86_KEEP_NATIVE_STAGE0_SOURCE_SMOKE_WORK_DIR=1 \
    "$SMOKE"
}

run_stopped_vm_smoke
grep -F 'limactl start --tty=false lsharp-linux-x86' "$STOPPED_LOG" >/dev/null \
  || fail 'stopped VM smoke did not record gate-owned start'
grep -F 'limactl stop lsharp-linux-x86' "$STOPPED_LOG" >/dev/null \
  || fail 'gate-owned VM smoke did not stop its VM'
if grep -F 'limactl stop lsharp-linux-x86' "$LOG_PATH" >/dev/null; then
  fail 'already-running VM smoke was stopped by the gate'
fi

set +e
stop_failure_output="$(
  PATH="$BIN_DIR:$PATH" \
    FAIL_LIMACTL_STOP=1 \
    FAKE_LIMA_RUNNING=0 \
    FAKE_LOG="$STOP_FAILURE_LOG" \
    LSHARP_NATIVE_LINUX_X86_STAGE0_DIR="$STAGE0_DIR" \
    LSHARP_NATIVE_LINUX_X86_KEEP_NATIVE_STAGE0_SOURCE_SMOKE_WORK_DIR=1 \
    "$SMOKE" 2>&1
)"
stop_failure_status=$?
set -e
[[ "$stop_failure_status" -ne 0 ]] || fail 'owned VM stop failure was reported as success'
grep -F 'Linux native stage0 source-file smoke cleanup failed' <<<"$stop_failure_output" >/dev/null \
  || fail 'owned VM stop failure did not report cleanup failure'

echo "Linux native stage0 source-file provenance tests: OK"
