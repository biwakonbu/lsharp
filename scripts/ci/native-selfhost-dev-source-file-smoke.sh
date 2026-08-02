#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
RUNNER="$ROOT/scripts/native-selfhost-dev.sh"
VALIDATION_SOURCE="$ROOT/tests/fixtures/validation/ec-m3-canonical-source.ls"
EXPECTED_VALIDATION_MANIFEST="$ROOT/tests/fixtures/validation/ec-m3-canonical-manifest.json"
VALIDATION_INVALID_SOURCE="$ROOT/tests/fixtures/validation/ec-m3-duplicate-node-source.ls"
VALIDATION_PROJECT_DUPLICATE_SOURCE="$ROOT/tests/fixtures/validation/ec-m2-project-duplicate-source.ls"
VALIDATION_ATTESTATION_FIXTURE="$ROOT/tests/fixtures/validation/ec-m3-review-attestation-source.ls"
STAGE0_DIR="${NATIVE_STAGE0_DIR:-}"
SOURCE_ROOT="${NATIVE_SELFHOST_SOURCE_ROOT:-$ROOT/selfhost}"
STAGE_DIR="${NATIVE_SELFHOST_STAGE_DIR:-}"
KEEP_STAGE_DIR="${NATIVE_SELFHOST_KEEP_STAGE_DIR:-0}"
SOURCE_SMOKE_EVIDENCE_DIR="${NATIVE_SELFHOST_SOURCE_SMOKE_EVIDENCE_DIR:-}"
REVIEW_ATTESTATION_REPORT_INPUT="${NATIVE_SELFHOST_REVIEW_ATTESTATION_REPORT:-}"
SOURCE_SMOKE_EVIDENCE_WRITER="$ROOT/scripts/ci/write-native-source-smoke-evidence.py"
WORK_DIR=""
STAGE_DIR_CREATED=0
SMOKE_TARGET=""

cleanup() {
  local status=$?
  if [[ -n "$SOURCE_SMOKE_EVIDENCE_DIR" && -d "$WORK_DIR" ]]; then
    local writer_args=()
    if [[ -n "$REVIEW_ATTESTATION_REPORT_INPUT" ]]; then
      writer_args+=(--review-attestation-report "$REVIEW_ATTESTATION_REPORT_INPUT")
    fi
    if ! python3 "$SOURCE_SMOKE_EVIDENCE_WRITER" \
      --evidence-dir "$SOURCE_SMOKE_EVIDENCE_DIR" \
      --work-dir "$WORK_DIR" \
      --stage0-manifest "$STAGE0_DIR/manifest.json" \
      --target "$SMOKE_TARGET" \
      --exit-code "$status" \
      "${writer_args[@]}"; then
      echo "ERROR: failed to persist native source-file smoke evidence" >&2
      [[ "$status" -ne 0 ]] || status=1
    fi
  fi
  [[ -z "$WORK_DIR" ]] || rm -rf "$WORK_DIR"
  if [[ "$STAGE_DIR_CREATED" -eq 1 && "$KEEP_STAGE_DIR" != "1" ]]; then
    rm -rf "$STAGE_DIR"
  fi
  exit "$status"
}
trap cleanup EXIT

die() {
  echo "ERROR: $*" >&2
  exit 1
}

require_file() {
  local path="$1"
  local description="$2"
  [[ -f "$path" && -s "$path" ]] || die "$description is required: $path"
}

[[ -n "$STAGE0_DIR" ]] || die "NATIVE_STAGE0_DIR is required"
[[ -d "$STAGE0_DIR" ]] || die "native stage0 directory is missing: $STAGE0_DIR"
[[ -d "$SOURCE_ROOT" ]] || die "native selfhost source root is missing: $SOURCE_ROOT"
[[ -x "$RUNNER" ]] || die "native selfhost runner is missing: $RUNNER"
require_file "$STAGE0_DIR/manifest.json" "native stage0 manifest"
require_file "$SOURCE_ROOT/src/App/Cli.ls" "native selfhost App.Cli source"
require_file "$VALIDATION_SOURCE" "EC-M3-01 validation source fixture"
require_file "$EXPECTED_VALIDATION_MANIFEST" "EC-M3-01 canonical manifest fixture"
require_file "$VALIDATION_INVALID_SOURCE" "EC-M3-01 duplicate node source fixture"
require_file "$VALIDATION_PROJECT_DUPLICATE_SOURCE" "EC-M2-01 project duplicate source fixture"
require_file "$VALIDATION_ATTESTATION_FIXTURE" "EC-M3-04 review attestation source fixture"

if [[ -n "$SOURCE_SMOKE_EVIDENCE_DIR" ]]; then
  [[ "$SOURCE_SMOKE_EVIDENCE_DIR" = /* && "$SOURCE_SMOKE_EVIDENCE_DIR" != "/" ]] \
    || die "NATIVE_SELFHOST_SOURCE_SMOKE_EVIDENCE_DIR must be an absolute non-root path"
  [[ ! -e "$SOURCE_SMOKE_EVIDENCE_DIR" && ! -L "$SOURCE_SMOKE_EVIDENCE_DIR" ]] \
    || die "native source-file smoke evidence directory already exists: $SOURCE_SMOKE_EVIDENCE_DIR"
  require_file "$SOURCE_SMOKE_EVIDENCE_WRITER" "native source-file smoke evidence writer"
fi
if [[ -n "$REVIEW_ATTESTATION_REPORT_INPUT" ]]; then
  [[ -f "$REVIEW_ATTESTATION_REPORT_INPUT" && ! -L "$REVIEW_ATTESTATION_REPORT_INPUT" && -s "$REVIEW_ATTESTATION_REPORT_INPUT" ]] \
    || die "review attestation report must be a non-empty regular file: $REVIEW_ATTESTATION_REPORT_INPUT"
fi

stage0_target="$(python3 - "$STAGE0_DIR/manifest.json" <<'PY'
import json
import pathlib
import sys

manifest_path = pathlib.Path(sys.argv[1])
try:
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
except (OSError, json.JSONDecodeError) as error:
    raise SystemExit(f"invalid native stage0 manifest: {error}")

if manifest.get("kind") != "lsharp-native-selfhost-stage0":
    raise SystemExit("native stage0 manifest kind is invalid")

target = manifest.get("target")
if target not in ("aarch64-apple-darwin", "x86_64-unknown-linux-gnu"):
    raise SystemExit(f"native stage0 target is unsupported: {target!r}")
print(target)
PY
)"

case "$(uname -s)/$(uname -m)" in
  Darwin/arm64)
    expected_target="aarch64-apple-darwin"
    ;;
  Linux/x86_64)
    expected_target="x86_64-unknown-linux-gnu"
    ;;
  *)
    die "native selfhost source-file smoke requires macOS arm64 or Linux x86_64"
    ;;
esac
SMOKE_TARGET="$expected_target"
[[ "$stage0_target" == "$expected_target" ]] || die "stage0 target mismatch: expected $expected_target, got $stage0_target"

WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/lsharp-native-selfhost-source-smoke.XXXXXX")"
if [[ -z "$STAGE_DIR" ]]; then
  STAGE_DIR="$(mktemp -d "${TMPDIR:-/tmp}/lsharp-native-selfhost-stage.XXXXXX")"
  STAGE_DIR_CREATED=1
fi

BLOCKED_TOOL_DIR="$WORK_DIR/blocked-tools"
mkdir -p "$BLOCKED_TOOL_DIR"
for blocked_tool in cargo rustc lsharp; do
  printf '%s\n' '#!/usr/bin/env bash' 'exit 99' >"$BLOCKED_TOOL_DIR/$blocked_tool"
  chmod 755 "$BLOCKED_TOOL_DIR/$blocked_tool"
done

INPUT="$WORK_DIR/input.ls"
METADATA="$WORK_DIR/metadata.ls"
PROPERTY="$WORK_DIR/property.ls"
VACUOUS_PROPERTY="$WORK_DIR/vacuous-property.ls"
DYNAMIC_COMPLEMENT_PROPERTY="$WORK_DIR/dynamic-complement-property.ls"
COMPILE_OUTPUT="$WORK_DIR/compile.wasm"
BUILD_OUTPUT="$WORK_DIR/build.wasm"
VALIDATION_MANIFEST="$WORK_DIR/ec-m3-canonical-manifest.json"
VALIDATION_INVALID_MANIFEST="$WORK_DIR/ec-m3-duplicate-node-manifest.json"
VALIDATION_PROJECT_DUPLICATE_MANIFEST="$WORK_DIR/ec-m2-project-duplicate-manifest.json"
VALIDATION_ORPHAN_MANIFEST="$WORK_DIR/ec-m3-orphan-node-manifest.json"
VALIDATION_MALFORMED_MANIFEST="$WORK_DIR/ec-m3-malformed-edge-manifest.json"
VALIDATION_INVALID_ID_MANIFEST="$WORK_DIR/ec-m3-invalid-id-manifest.json"
VALIDATION_KIND_MISMATCH_MANIFEST="$WORK_DIR/ec-m3-kind-mismatch-manifest.json"
VALIDATION_EVIDENCE_REGISTRY_MANIFEST="$WORK_DIR/ec-m3-evidence-registry-manifest.json"
VALIDATION_SUPPORTS_EVIDENCE_PRECEDENCE_MANIFEST="$WORK_DIR/ec-m2-supports-evidence-precedence-manifest.json"
VALIDATION_CONTRADICTS_EVIDENCE_PRECEDENCE_MANIFEST="$WORK_DIR/ec-m2-contradicts-evidence-precedence-manifest.json"
VALIDATION_MALFORMED_EVIDENCE_MANIFEST="$WORK_DIR/ec-m3-malformed-evidence-manifest.json"
VALIDATION_DUPLICATE_EVIDENCE_COVERAGE_MANIFEST="$WORK_DIR/ec-m2-duplicate-evidence-coverage-manifest.json"
VALIDATION_INVALID_EVIDENCE_MANIFEST="$WORK_DIR/ec-m3-invalid-evidence-manifest.json"
VALIDATION_INVALID_EVIDENCE_OUTCOME_MANIFEST="$WORK_DIR/ec-m3-invalid-evidence-outcome-manifest.json"
VALIDATION_INVALID_EVIDENCE_INDEPENDENCE_MANIFEST="$WORK_DIR/ec-m3-invalid-evidence-independence-manifest.json"
VALIDATION_INVALID_EVIDENCE_SUBJECT_MANIFEST="$WORK_DIR/ec-m3-invalid-evidence-subject-manifest.json"
VALIDATION_EMPTY_EVIDENCE_METHOD_MANIFEST="$WORK_DIR/ec-m2-empty-evidence-method-manifest.json"
VALIDATION_EMPTY_EVIDENCE_OUTCOME_MANIFEST="$WORK_DIR/ec-m2-empty-evidence-outcome-manifest.json"
VALIDATION_EMPTY_EVIDENCE_INDEPENDENCE_MANIFEST="$WORK_DIR/ec-m2-empty-evidence-independence-manifest.json"
VALIDATION_EVIDENCE_REQUIRED_PRECEDENCE_MANIFEST="$WORK_DIR/ec-m2-evidence-required-precedence-manifest.json"
VALIDATION_DUPLICATE_EVIDENCE_MANIFEST="$WORK_DIR/ec-m3-duplicate-evidence-manifest.json"
VALIDATION_DUPLICATE_EVIDENCE_FIELD_MANIFEST="$WORK_DIR/ec-m3-duplicate-evidence-field-manifest.json"
VALIDATION_EMPTY_EVIDENCE_GENERATOR_MANIFEST="$WORK_DIR/ec-m2-empty-evidence-generator-manifest.json"
VALIDATION_WHITESPACE_EVIDENCE_COVERAGE_MANIFEST="$WORK_DIR/ec-m2-whitespace-evidence-coverage-manifest.json"
VALIDATION_UNICODE_WHITESPACE_EVIDENCE_COVERAGE_MANIFEST="$WORK_DIR/ec-m2-unicode-whitespace-evidence-coverage-manifest.json"
VALIDATION_NEGATIVE_EVIDENCE_COVERAGE_MANIFEST="$WORK_DIR/ec-m2-negative-evidence-coverage-manifest.json"
VALIDATION_NEGATIVE_EVIDENCE_CASES_MANIFEST="$WORK_DIR/ec-m2-negative-evidence-cases-manifest.json"
VALIDATION_NEGATIVE_EVIDENCE_SEED_MANIFEST="$WORK_DIR/ec-m2-negative-evidence-seed-manifest.json"
VALIDATION_NEGATIVE_EVIDENCE_SHRINKS_MANIFEST="$WORK_DIR/ec-m2-negative-evidence-shrinks-manifest.json"
VALIDATION_EMPTY_EVIDENCE_RUNNER_MANIFEST="$WORK_DIR/ec-m2-empty-evidence-runner-manifest.json"
VALIDATION_EMPTY_EVIDENCE_TARGET_MANIFEST="$WORK_DIR/ec-m2-empty-evidence-target-manifest.json"
VALIDATION_EMPTY_EVIDENCE_SOURCE_COMMIT_MANIFEST="$WORK_DIR/ec-m2-empty-evidence-source-commit-manifest.json"
VALIDATION_EMPTY_EVIDENCE_ARTIFACT_DIGEST_MANIFEST="$WORK_DIR/ec-m2-empty-evidence-artifact-digest-manifest.json"
VALIDATION_EMPTY_EVIDENCE_PRODUCER_MANIFEST="$WORK_DIR/ec-m2-empty-evidence-producer-manifest.json"
VALIDATION_EMPTY_EVIDENCE_TOOL_VERSION_MANIFEST="$WORK_DIR/ec-m2-empty-evidence-tool-version-manifest.json"
VALIDATION_EMPTY_EVIDENCE_TIMESTAMP_MANIFEST="$WORK_DIR/ec-m2-empty-evidence-timestamp-manifest.json"
VALIDATION_WHITESPACE_EVIDENCE_RUNNER_MANIFEST="$WORK_DIR/ec-m2-whitespace-evidence-runner-manifest.json"
VALIDATION_UNICODE_WHITESPACE_EVIDENCE_RUNNER_MANIFEST="$WORK_DIR/ec-m2-unicode-whitespace-evidence-runner-manifest.json"
VALIDATION_WHITESPACE_EVIDENCE_SUBJECT_MANIFEST="$WORK_DIR/ec-m2-whitespace-evidence-subject-manifest.json"
VALIDATION_WHITESPACE_EVIDENCE_ID_MANIFEST="$WORK_DIR/ec-m2-whitespace-evidence-id-manifest.json"
VALIDATION_EMPTY_EVIDENCE_ID_MANIFEST="$WORK_DIR/ec-m2-empty-evidence-id-manifest.json"
VALIDATION_MISSING_NODE_ID_MANIFEST="$WORK_DIR/ec-m2-missing-node-id-manifest.json"
VALIDATION_MISSING_NODE_TEXT_MANIFEST="$WORK_DIR/ec-m2-missing-node-text-manifest.json"
VALIDATION_WHITESPACE_NODE_TEXT_MANIFEST="$WORK_DIR/ec-m2-whitespace-node-text-manifest.json"
VALIDATION_UNICODE_WHITESPACE_NODE_TEXT_MANIFEST="$WORK_DIR/ec-m2-unicode-whitespace-node-text-manifest.json"
VALIDATION_NODE_TEXT_PRECEDENCE_MANIFEST="$WORK_DIR/ec-m2-node-text-precedence-manifest.json"
VALIDATION_MISSING_REVIEW_MANIFEST="$WORK_DIR/ec-m3-missing-review-manifest.json"
VALIDATION_MISSING_REVIEW_SUBJECT_KIND_MANIFEST="$WORK_DIR/ec-m3-missing-review-subject-kind-manifest.json"
VALIDATION_DUPLICATE_REVIEW_MANIFEST="$WORK_DIR/ec-m3-duplicate-review-manifest.json"
VALIDATION_INVALID_REVIEW_MANIFEST="$WORK_DIR/ec-m3-invalid-review-manifest.json"
VALIDATION_INVALID_REVIEW_DIGEST_MANIFEST="$WORK_DIR/ec-m3-invalid-review-digest-manifest.json"
VALIDATION_UNICODE_WHITESPACE_REVIEW_DIGEST_MANIFEST="$WORK_DIR/ec-m3-unicode-whitespace-review-digest-manifest.json"
VALIDATION_REVIEW_REQUIRED_PRECEDENCE_MANIFEST="$WORK_DIR/ec-m3-review-required-precedence-manifest.json"
VALIDATION_INVALID_REVIEW_ID_MANIFEST="$WORK_DIR/ec-m3-invalid-review-id-manifest.json"
VALIDATION_EMPTY_REVIEW_ID_MANIFEST="$WORK_DIR/ec-m3-empty-review-id-manifest.json"
VALIDATION_MALFORMED_REVIEW_MANIFEST="$WORK_DIR/ec-m3-malformed-review-manifest.json"
VALIDATION_MALFORMED_REVIEW_EXTRA_MANIFEST="$WORK_DIR/ec-m3-malformed-review-extra-manifest.json"
VALIDATION_MALFORMED_REVIEW_EDGE_MANIFEST="$WORK_DIR/ec-m3-malformed-review-edge-manifest.json"
VALIDATION_MALFORMED_INVALIDATION_EDGE_MANIFEST="$WORK_DIR/ec-m3-malformed-invalidation-edge-manifest.json"
VALIDATION_MALFORMED_REVIEW_EDGE_EXTRA_MANIFEST="$WORK_DIR/ec-m3-malformed-review-edge-extra-manifest.json"
VALIDATION_MALFORMED_INVALIDATION_EDGE_EXTRA_MANIFEST="$WORK_DIR/ec-m3-malformed-invalidation-edge-extra-manifest.json"
VALIDATION_REVIEW_SUBJECT_KIND_MANIFEST="$WORK_DIR/ec-m3-review-subject-kind-manifest.json"
VALIDATION_INVALIDATION_SUBJECT_KIND_MANIFEST="$WORK_DIR/ec-m3-invalidation-subject-kind-manifest.json"
VALIDATION_INVALIDATION_MISSING_REVIEW_MANIFEST="$WORK_DIR/ec-m3-invalidation-missing-review-manifest.json"
VALIDATION_REVIEW_EDGE_EVIDENCE_MANIFEST="$WORK_DIR/ec-m3-review-edge-evidence-manifest.json"
VALIDATION_INVALIDATION_EDGE_EVIDENCE_MANIFEST="$WORK_DIR/ec-m3-invalidation-edge-evidence-manifest.json"
VALIDATION_WRITE_FAILURE_MANIFEST="$WORK_DIR/missing-parent/intent-graph.json"
VALIDATION_ATTESTATION_MANIFEST="$WORK_DIR/ec-m3-review-attestation-manifest.json"
VALIDATION_ATTESTATION_NO_EXPIRY_MANIFEST="$WORK_DIR/ec-m3-review-attestation-no-expiry-manifest.json"
VALIDATION_IDENTITY_MANIFEST="$WORK_DIR/ec-m3-review-identity-manifest.json"
VALIDATION_IDENTITY_OPTIONAL_MANIFEST="$WORK_DIR/ec-m3-review-identity-optional-manifest.json"
VALIDATION_IDENTITY_PARTIAL_MANIFEST="$WORK_DIR/ec-m3-review-identity-partial-manifest.json"
VALIDATION_IDENTITY_REATTACH_MANIFEST="$WORK_DIR/ec-m3-review-identity-reattach-manifest.json"
VALIDATION_IDENTITY_CONFLICT_MANIFEST="$WORK_DIR/ec-m3-review-identity-conflict-manifest.json"
VALIDATION_IDENTITY_CONFLICT_OUTPUT_MANIFEST="$WORK_DIR/ec-m3-review-identity-conflict-output-manifest.json"
VALIDATION_INVALID_ATTESTATION_ALGORITHM_MANIFEST="$WORK_DIR/ec-m3-invalid-attestation-algorithm-manifest.json"
VALIDATION_INVALID_ATTESTATION_SIGNATURE_MANIFEST="$WORK_DIR/ec-m3-invalid-attestation-signature-manifest.json"
VALIDATION_INVALID_ATTESTATION_TIMESTAMP_MANIFEST="$WORK_DIR/ec-m3-invalid-attestation-timestamp-manifest.json"
VALIDATION_INVALID_ATTESTATION_WINDOW_MANIFEST="$WORK_DIR/ec-m3-invalid-attestation-window-manifest.json"
VALIDATION_PASS_SOURCE="$WORK_DIR/ec-m3-complete-source.ls"
VALIDATION_FAILED_REVIEW_SOURCE="$WORK_DIR/ec-m3-failed-review-source.ls"
VALIDATION_FAIL_SOURCE="$WORK_DIR/ec-m3-contradiction-source.ls"
VALIDATION_STALE_SOURCE="$WORK_DIR/ec-m3-stale-source.ls"
VALIDATION_ATTESTATION_SOURCE="$WORK_DIR/ec-m3-review-attestation-source.ls"
VALIDATION_ATTESTATION_NO_EXPIRY_SOURCE="$WORK_DIR/ec-m3-review-attestation-no-expiry-source.ls"
VALIDATION_IDENTITY_SOURCE="$WORK_DIR/ec-m3-review-identity-source.ls"
VALIDATION_INVALID_ATTESTATION_ALGORITHM_SOURCE="$WORK_DIR/ec-m3-invalid-attestation-algorithm-source.ls"
VALIDATION_INVALID_ATTESTATION_SIGNATURE_SOURCE="$WORK_DIR/ec-m3-invalid-attestation-signature-source.ls"
VALIDATION_INVALID_ATTESTATION_TIMESTAMP_SOURCE="$WORK_DIR/ec-m3-invalid-attestation-timestamp-source.ls"
VALIDATION_INVALID_ATTESTATION_WINDOW_SOURCE="$WORK_DIR/ec-m3-invalid-attestation-window-source.ls"
VALIDATION_ORPHAN_SOURCE="$WORK_DIR/ec-m3-orphan-node-source.ls"
VALIDATION_MALFORMED_SOURCE="$WORK_DIR/ec-m3-malformed-edge-source.ls"
VALIDATION_INVALID_ID_SOURCE="$WORK_DIR/ec-m3-invalid-id-source.ls"
VALIDATION_KIND_MISMATCH_SOURCE="$WORK_DIR/ec-m3-kind-mismatch-source.ls"
VALIDATION_EVIDENCE_REGISTRY_SOURCE="$WORK_DIR/ec-m3-evidence-registry-source.ls"
VALIDATION_SUPPORTS_EVIDENCE_PRECEDENCE_SOURCE="$WORK_DIR/ec-m2-supports-evidence-precedence-source.ls"
VALIDATION_CONTRADICTS_EVIDENCE_PRECEDENCE_SOURCE="$WORK_DIR/ec-m2-contradicts-evidence-precedence-source.ls"
VALIDATION_MALFORMED_EVIDENCE_SOURCE="$WORK_DIR/ec-m3-malformed-evidence-source.ls"
VALIDATION_DUPLICATE_EVIDENCE_COVERAGE_SOURCE="$WORK_DIR/ec-m2-duplicate-evidence-coverage-source.ls"
VALIDATION_INVALID_EVIDENCE_SOURCE="$WORK_DIR/ec-m3-invalid-evidence-source.ls"
VALIDATION_INVALID_EVIDENCE_OUTCOME_SOURCE="$WORK_DIR/ec-m3-invalid-evidence-outcome-source.ls"
VALIDATION_INVALID_EVIDENCE_INDEPENDENCE_SOURCE="$WORK_DIR/ec-m3-invalid-evidence-independence-source.ls"
VALIDATION_INVALID_EVIDENCE_SUBJECT_SOURCE="$WORK_DIR/ec-m3-invalid-evidence-subject-source.ls"
VALIDATION_EMPTY_EVIDENCE_METHOD_SOURCE="$WORK_DIR/ec-m2-empty-evidence-method-source.ls"
VALIDATION_EMPTY_EVIDENCE_OUTCOME_SOURCE="$WORK_DIR/ec-m2-empty-evidence-outcome-source.ls"
VALIDATION_EMPTY_EVIDENCE_INDEPENDENCE_SOURCE="$WORK_DIR/ec-m2-empty-evidence-independence-source.ls"
VALIDATION_EVIDENCE_REQUIRED_PRECEDENCE_SOURCE="$WORK_DIR/ec-m2-evidence-required-precedence-source.ls"
VALIDATION_DUPLICATE_EVIDENCE_SOURCE="$WORK_DIR/ec-m3-duplicate-evidence-source.ls"
VALIDATION_DUPLICATE_EVIDENCE_FIELD_SOURCE="$WORK_DIR/ec-m3-duplicate-evidence-field-source.ls"
VALIDATION_EMPTY_EVIDENCE_GENERATOR_SOURCE="$WORK_DIR/ec-m2-empty-evidence-generator-source.ls"
VALIDATION_WHITESPACE_EVIDENCE_COVERAGE_SOURCE="$WORK_DIR/ec-m2-whitespace-evidence-coverage-source.ls"
VALIDATION_UNICODE_WHITESPACE_EVIDENCE_COVERAGE_SOURCE="$WORK_DIR/ec-m2-unicode-whitespace-evidence-coverage-source.ls"
VALIDATION_NEGATIVE_EVIDENCE_COVERAGE_SOURCE="$WORK_DIR/ec-m2-negative-evidence-coverage-source.ls"
VALIDATION_NEGATIVE_EVIDENCE_CASES_SOURCE="$WORK_DIR/ec-m2-negative-evidence-cases-source.ls"
VALIDATION_NEGATIVE_EVIDENCE_SEED_SOURCE="$WORK_DIR/ec-m2-negative-evidence-seed-source.ls"
VALIDATION_NEGATIVE_EVIDENCE_SHRINKS_SOURCE="$WORK_DIR/ec-m2-negative-evidence-shrinks-source.ls"
VALIDATION_EMPTY_EVIDENCE_RUNNER_SOURCE="$WORK_DIR/ec-m2-empty-evidence-runner-source.ls"
VALIDATION_EMPTY_EVIDENCE_TARGET_SOURCE="$WORK_DIR/ec-m2-empty-evidence-target-source.ls"
VALIDATION_EMPTY_EVIDENCE_SOURCE_COMMIT_SOURCE="$WORK_DIR/ec-m2-empty-evidence-source-commit-source.ls"
VALIDATION_EMPTY_EVIDENCE_ARTIFACT_DIGEST_SOURCE="$WORK_DIR/ec-m2-empty-evidence-artifact-digest-source.ls"
VALIDATION_EMPTY_EVIDENCE_PRODUCER_SOURCE="$WORK_DIR/ec-m2-empty-evidence-producer-source.ls"
VALIDATION_EMPTY_EVIDENCE_TOOL_VERSION_SOURCE="$WORK_DIR/ec-m2-empty-evidence-tool-version-source.ls"
VALIDATION_EMPTY_EVIDENCE_TIMESTAMP_SOURCE="$WORK_DIR/ec-m2-empty-evidence-timestamp-source.ls"
VALIDATION_WHITESPACE_EVIDENCE_RUNNER_SOURCE="$WORK_DIR/ec-m2-whitespace-evidence-runner-source.ls"
VALIDATION_UNICODE_WHITESPACE_EVIDENCE_RUNNER_SOURCE="$WORK_DIR/ec-m2-unicode-whitespace-evidence-runner-source.ls"
VALIDATION_WHITESPACE_EVIDENCE_SUBJECT_SOURCE="$WORK_DIR/ec-m2-whitespace-evidence-subject-source.ls"
VALIDATION_WHITESPACE_EVIDENCE_ID_SOURCE="$WORK_DIR/ec-m2-whitespace-evidence-id-source.ls"
VALIDATION_EMPTY_EVIDENCE_ID_SOURCE="$WORK_DIR/ec-m2-empty-evidence-id-source.ls"
VALIDATION_MISSING_NODE_ID_SOURCE="$WORK_DIR/ec-m2-missing-node-id-source.ls"
VALIDATION_MISSING_NODE_TEXT_SOURCE="$WORK_DIR/ec-m2-missing-node-text-source.ls"
VALIDATION_WHITESPACE_NODE_TEXT_SOURCE="$WORK_DIR/ec-m2-whitespace-node-text-source.ls"
VALIDATION_UNICODE_WHITESPACE_NODE_TEXT_SOURCE="$WORK_DIR/ec-m2-unicode-whitespace-node-text-source.ls"
VALIDATION_NODE_TEXT_PRECEDENCE_SOURCE="$WORK_DIR/ec-m2-node-text-precedence-source.ls"
VALIDATION_MISSING_REVIEW_SOURCE="$WORK_DIR/ec-m3-missing-review-source.ls"
VALIDATION_MISSING_REVIEW_SUBJECT_KIND_SOURCE="$WORK_DIR/ec-m3-missing-review-subject-kind-source.ls"
VALIDATION_DUPLICATE_REVIEW_SOURCE="$WORK_DIR/ec-m3-duplicate-review-source.ls"
VALIDATION_INVALID_REVIEW_SOURCE="$WORK_DIR/ec-m3-invalid-review-source.ls"
VALIDATION_INVALID_REVIEW_DIGEST_SOURCE="$WORK_DIR/ec-m3-invalid-review-digest-source.ls"
VALIDATION_UNICODE_WHITESPACE_REVIEW_DIGEST_SOURCE="$WORK_DIR/ec-m3-unicode-whitespace-review-digest-source.ls"
VALIDATION_REVIEW_REQUIRED_PRECEDENCE_SOURCE="$WORK_DIR/ec-m3-review-required-precedence-source.ls"
VALIDATION_INVALID_REVIEW_ID_SOURCE="$WORK_DIR/ec-m3-invalid-review-id-source.ls"
VALIDATION_EMPTY_REVIEW_ID_SOURCE="$WORK_DIR/ec-m3-empty-review-id-source.ls"
VALIDATION_MALFORMED_REVIEW_SOURCE="$WORK_DIR/ec-m3-malformed-review-source.ls"
VALIDATION_MALFORMED_REVIEW_EXTRA_SOURCE="$WORK_DIR/ec-m3-malformed-review-extra-source.ls"
VALIDATION_MALFORMED_REVIEW_EDGE_SOURCE="$WORK_DIR/ec-m3-malformed-review-edge-source.ls"
VALIDATION_MALFORMED_INVALIDATION_EDGE_SOURCE="$WORK_DIR/ec-m3-malformed-invalidation-edge-source.ls"
VALIDATION_MALFORMED_REVIEW_EDGE_EXTRA_SOURCE="$WORK_DIR/ec-m3-malformed-review-edge-extra-source.ls"
VALIDATION_MALFORMED_INVALIDATION_EDGE_EXTRA_SOURCE="$WORK_DIR/ec-m3-malformed-invalidation-edge-extra-source.ls"
VALIDATION_REVIEW_SUBJECT_KIND_SOURCE="$WORK_DIR/ec-m3-review-subject-kind-source.ls"
VALIDATION_INVALIDATION_SUBJECT_KIND_SOURCE="$WORK_DIR/ec-m3-invalidation-subject-kind-source.ls"
VALIDATION_INVALIDATION_MISSING_REVIEW_SOURCE="$WORK_DIR/ec-m3-invalidation-missing-review-source.ls"
VALIDATION_REVIEW_EDGE_EVIDENCE_SOURCE="$WORK_DIR/ec-m3-review-edge-evidence-source.ls"
VALIDATION_INVALIDATION_EDGE_EVIDENCE_SOURCE="$WORK_DIR/ec-m3-invalidation-edge-evidence-source.ls"

printf '%s\n' '(defn main [] 42)' >"$INPUT"
cat >"$METADATA" <<'LSHARP'
(defn abs [x]
  :example [(= (abs 5) 5) (= (abs (- 0 7)) 7)]
  :invariant (>= result 0)
  (if (< x 0) (- 0 x) x))
LSHARP
cat >"$PROPERTY" <<'LSHARP'
(defn identity [x]
  :property [(for-all [sample String] :cases 5 :postcondition (string-eq result sample))]
  x)
LSHARP
cat >"$VACUOUS_PROPERTY" <<'LSHARP'
(defn identity [x]
  :property [(for-all [sample Int] :cases 1 :postcondition (or true (= sample 0)))]
  x)
LSHARP
cat >"$DYNAMIC_COMPLEMENT_PROPERTY" <<'LSHARP'
(defn identity [x]
  :property [(for-all [value Int] :cases 1 :postcondition (or (= value 0) (not (= value 0))))]
  x)
LSHARP
cat >"$VALIDATION_PASS_SOURCE" <<'LSHARP'
(defn verify []
  :intent "intent:checkout/safe-cancel" "Users can cancel an order"
  :claim "claim:checkout/rejects" "Shipped orders are rejected"
  :motivates "intent:checkout/safe-cancel" "claim:checkout/rejects"
  :tested-by "claim:checkout/rejects" "contract:checkout/review"
  :evidence "evidence:checkout/review"
    :subject "claim:checkout/rejects"
    :method "review"
    :outcome "pass"
    :runner "reviewer"
    :target "aarch64-apple-darwin"
    :source-commit "deadbeef"
    :artifact-digest "sha256:abc"
    :cases 1
    :seed 42
    :generator "checkout-review"
    :producer "lsharp-test"
    :tool-version "0.2"
    :timestamp "2026-07-25T00:00:00Z"
    :independence "independent-review"
  :supports "evidence:checkout/review" "claim:checkout/rejects"
  true)
LSHARP
cat >"$VALIDATION_FAILED_REVIEW_SOURCE" <<'LSHARP'
(defn failed-review []
  :intent "intent:checkout/safe-cancel" "Users can cancel an order"
  :claim "claim:checkout/rejects" "Shipped orders are rejected"
  :motivates "intent:checkout/safe-cancel" "claim:checkout/rejects"
  :tested-by "claim:checkout/rejects" "contract:checkout/review"
  :evidence "evidence:checkout/review"
    :subject "claim:checkout/rejects"
    :method "review"
    :outcome "fail"
    :runner "reviewer"
    :target "aarch64-apple-darwin"
    :source-commit "deadbeef"
    :artifact-digest "sha256:abc"
    :cases 1
    :seed 42
    :generator "checkout-review"
    :producer "lsharp-test"
    :tool-version "0.2"
    :timestamp "2026-07-25T00:00:00Z"
    :independence "independent-review"
  :supports "evidence:checkout/review" "claim:checkout/rejects"
  true)
LSHARP
cat >"$VALIDATION_FAIL_SOURCE" <<'LSHARP'
(defn verify []
  :intent "intent:checkout/safe-cancel" "Users can cancel an order"
  :claim "claim:checkout/rejects" "Shipped orders are rejected"
  :motivates "intent:checkout/safe-cancel" "claim:checkout/rejects"
  :tested-by "claim:checkout/rejects" "contract:checkout/review"
  :evidence "evidence:checkout/review"
    :subject "claim:checkout/rejects"
    :method "review"
    :outcome "contradicted"
    :runner "reviewer"
    :target "aarch64-apple-darwin"
    :source-commit "deadbeef"
    :artifact-digest "sha256:abc"
    :cases 1
    :seed 42
    :generator "checkout-review"
    :producer "lsharp-test"
    :tool-version "0.2"
    :timestamp "2026-07-25T00:00:00Z"
    :independence "independent-review"
  :contradicts "evidence:checkout/review" "claim:checkout/rejects"
  true)
LSHARP
cat >"$VALIDATION_STALE_SOURCE" <<'LSHARP'
(defn stale-review []
  :intent "intent:checkout/safe-cancel" "Users can cancel an order"
  :claim "claim:checkout/rejects" "Shipped orders are rejected"
  :motivates "intent:checkout/safe-cancel" "claim:checkout/rejects"
  :tested-by "claim:checkout/rejects" "contract:checkout/review"
  :evidence "evidence:checkout/review-001"
    :subject "claim:checkout/rejects"
    :method "review"
    :outcome "pass"
    :runner "reviewer"
    :target "aarch64-apple-darwin"
    :source-commit "deadbeef"
    :artifact-digest "sha256:abc"
    :cases 1
    :seed 42
    :generator "checkout-review"
    :producer "lsharp-test"
    :tool-version "0.2"
    :timestamp "2026-07-27T00:00:00Z"
    :independence "independent-review"
  :review "review:checkout/reviewer-001" "sha256:review-provenance-001" "redacted"
  :evaluates "review:checkout/reviewer-001" "evidence:checkout/review-001"
  :invalidates "change:checkout/api-v2" "review:checkout/reviewer-001"
  true)
LSHARP
cp "$VALIDATION_ATTESTATION_FIXTURE" "$VALIDATION_ATTESTATION_SOURCE"
cp "$VALIDATION_ATTESTATION_FIXTURE" "$VALIDATION_IDENTITY_SOURCE"
cat >>"$VALIDATION_IDENTITY_SOURCE" <<'LSHARP'
(defn review-identity-trace []
  :intent "intent:checkout/safe-cancel" "Users can cancel an order"
  :claim "claim:checkout/rejects" "Shipped orders are rejected"
  :motivates "intent:checkout/safe-cancel" "claim:checkout/rejects"
  :tested-by "claim:checkout/rejects" "contract:checkout/review"
  true)
LSHARP
python3 - \
  "$VALIDATION_ATTESTATION_SOURCE" \
  "$VALIDATION_INVALID_ATTESTATION_ALGORITHM_SOURCE" \
  "$VALIDATION_INVALID_ATTESTATION_SIGNATURE_SOURCE" \
  "$VALIDATION_INVALID_ATTESTATION_TIMESTAMP_SOURCE" \
  "$VALIDATION_INVALID_ATTESTATION_WINDOW_SOURCE" \
  "$VALIDATION_ATTESTATION_NO_EXPIRY_SOURCE" <<'PY'
import pathlib
import sys

source = pathlib.Path(sys.argv[1]).read_text(encoding="utf-8")
variants = [
    (sys.argv[2], ':algorithm "ed25519"', ':algorithm "rsa-sha256"'),
    (sys.argv[3], ':signature "AAECAw"', ':signature "A==="'),
    (
        sys.argv[4],
        ':issued-at "2026-08-01T00:00:00Z"',
        ':issued-at "2026-02-30T00:00:00Z"',
    ),
    (
        sys.argv[5],
        ':expires-at "2026-09-01T00:00:00Z"',
        ':expires-at "2026-07-01T00:00:00Z"',
    ),
    (
        sys.argv[6],
        ':expires-at "2026-09-01T00:00:00Z"\n',
        '',
    ),
]
for output, old, new in variants:
    if old not in source:
        raise SystemExit(f"attestation fixture replacement is missing: {old}")
    pathlib.Path(output).write_text(source.replace(old, new), encoding="utf-8")
PY
cat >"$VALIDATION_ORPHAN_SOURCE" <<'LSHARP'
(defn orphan-edge []
  :motivates "intent:checkout/missing" "claim:checkout/rejects"
  true)
LSHARP
cat >"$VALIDATION_MALFORMED_SOURCE" <<'LSHARP'
(defn malformed-edge []
  :motivates "intent:checkout/safe-cancel"
  true)
LSHARP
cat >"$VALIDATION_INVALID_ID_SOURCE" <<'LSHARP'
(defn invalid-id-edge []
  :motivates "intent:checkout/safe-cancel" "claim:checkout"
  true)
LSHARP
cat >"$VALIDATION_KIND_MISMATCH_SOURCE" <<'LSHARP'
(defn kind-mismatch-node []
  :claim "intent:checkout/wrong-kind" "Claim metadata must use a claim ID"
  true)
LSHARP
cat >"$VALIDATION_EVIDENCE_REGISTRY_SOURCE" <<'LSHARP'
(defn missing-evidence-edge []
  :claim "claim:checkout/rejects" "The API rejects shipped orders"
  :supports "evidence:checkout/missing" "claim:checkout/rejects"
  true)
LSHARP
cat >"$VALIDATION_SUPPORTS_EVIDENCE_PRECEDENCE_SOURCE" <<'LSHARP'
(defn supports-invalid-evidence-id []
  :claim "claim:checkout/rejects" "The API rejects shipped orders"
  :supports "evidence:checkout" "claim:checkout/rejects"
  true)
LSHARP
cat >"$VALIDATION_CONTRADICTS_EVIDENCE_PRECEDENCE_SOURCE" <<'LSHARP'
(defn contradicts-invalid-evidence-id []
  :claim "claim:checkout/rejects" "The API rejects shipped orders"
  :contradicts "evidence:checkout" "claim:checkout/rejects"
  true)
LSHARP
cat >"$VALIDATION_MALFORMED_EVIDENCE_SOURCE" <<'LSHARP'
(defn malformed-evidence []
  :evidence "evidence:checkout/malformed"
    :subject "claim:checkout/rejects"
  true)
LSHARP
cat >"$VALIDATION_DUPLICATE_EVIDENCE_COVERAGE_SOURCE" <<'LSHARP'
(defn duplicate-evidence-coverage []
  :claim "claim:checkout/rejects" "The API rejects shipped orders"
  :evidence "evidence:checkout/duplicate-coverage"
    :subject "claim:checkout/rejects"
    :method "property"
    :outcome "pass"
    :runner "duplicate-evidence-coverage-runner"
    :target "aarch64-apple-darwin"
    :source-commit "source-duplicate-evidence-coverage"
    :artifact-digest "sha256:duplicate-evidence-coverage"
    :cases 1
    :seed 0
    :generator "duplicate-evidence-coverage-generator"
    :coverage [("smoke" 2) ("smoke" 1)]
    :producer "duplicate-evidence-coverage-producer"
    :tool-version "0.2.0-dev"
    :timestamp "2026-07-29T00:00:00Z"
    :independence "same-author"
  true)
LSHARP
cat >"$VALIDATION_INVALID_EVIDENCE_SOURCE" <<'LSHARP'
(defn invalid-evidence-method []
  :claim "claim:checkout/rejects" "The API rejects shipped orders"
  :evidence "evidence:checkout/invalid-method"
    :subject "claim:checkout/rejects"
    :method "not-a-method"
    :outcome "pass"
    :runner "invalid-evidence-method-runner"
    :target "aarch64-apple-darwin"
    :source-commit "source-invalid-evidence-method"
    :artifact-digest "sha256:invalid-evidence-method"
    :cases 1
    :seed 0
    :generator "invalid-evidence-method-generator"
    :producer "invalid-evidence-method-producer"
    :tool-version "0.2.0-dev"
    :timestamp "2026-07-28T00:00:00Z"
    :independence "same-author"
  true)
LSHARP
cat >"$VALIDATION_INVALID_EVIDENCE_OUTCOME_SOURCE" <<'LSHARP'
(defn invalid-evidence-outcome []
  :claim "claim:checkout/rejects" "The API rejects shipped orders"
  :evidence "evidence:checkout/invalid-outcome"
    :subject "claim:checkout/rejects"
    :method "case"
    :outcome "not-an-outcome"
    :runner "invalid-evidence-outcome-runner"
    :target "aarch64-apple-darwin"
    :source-commit "source-invalid-evidence-outcome"
    :artifact-digest "sha256:invalid-evidence-outcome"
    :cases 1
    :seed 0
    :generator "invalid-evidence-outcome-generator"
    :producer "invalid-evidence-outcome-producer"
    :tool-version "0.2.0-dev"
    :timestamp "2026-07-28T00:00:00Z"
    :independence "same-author"
  true)
LSHARP
cat >"$VALIDATION_INVALID_EVIDENCE_INDEPENDENCE_SOURCE" <<'LSHARP'
(defn invalid-evidence-independence []
  :claim "claim:checkout/rejects" "The API rejects shipped orders"
  :evidence "evidence:checkout/invalid-independence"
    :subject "claim:checkout/rejects"
    :method "case"
    :outcome "pass"
    :runner "invalid-evidence-independence-runner"
    :target "aarch64-apple-darwin"
    :source-commit "source-invalid-evidence-independence"
    :artifact-digest "sha256:invalid-evidence-independence"
    :cases 1
    :seed 0
    :generator "invalid-evidence-independence-generator"
    :producer "invalid-evidence-independence-producer"
    :tool-version "0.2.0-dev"
    :timestamp "2026-07-28T00:00:00Z"
    :independence "not-an-independence"
  true)
LSHARP
cat >"$VALIDATION_INVALID_EVIDENCE_SUBJECT_SOURCE" <<'LSHARP'
(defn invalid-evidence-subject []
  :claim "claim:checkout/rejects" "The API rejects shipped orders"
  :evidence "evidence:checkout/invalid-subject"
    :subject "evidence:checkout/wrong-kind"
    :method "case"
    :outcome "pass"
    :runner "invalid-evidence-subject-runner"
    :target "aarch64-apple-darwin"
    :source-commit "source-invalid-evidence-subject"
    :artifact-digest "sha256:invalid-evidence-subject"
    :cases 1
    :seed 0
    :generator "invalid-evidence-subject-generator"
    :producer "invalid-evidence-subject-producer"
    :tool-version "0.2.0-dev"
    :timestamp "2026-07-28T00:00:00Z"
    :independence "same-author"
  true)
LSHARP
cat >"$VALIDATION_EMPTY_EVIDENCE_METHOD_SOURCE" <<'LSHARP'
(defn empty-evidence-method []
  :claim "claim:checkout/rejects" "The API rejects shipped orders"
  :evidence "evidence:checkout/empty-method"
    :subject "claim:checkout/rejects"
    :method ""
    :outcome "pass"
    :runner "empty-evidence-method-runner"
    :target "aarch64-apple-darwin"
    :source-commit "source-empty-evidence-method"
    :artifact-digest "sha256:empty-evidence-method"
    :cases 1
    :seed 0
    :generator "empty-evidence-method-generator"
    :producer "empty-evidence-method-producer"
    :tool-version "0.2.0-dev"
    :timestamp "2026-07-28T00:00:00Z"
    :independence "same-author"
  true)
LSHARP
cat >"$VALIDATION_EMPTY_EVIDENCE_OUTCOME_SOURCE" <<'LSHARP'
(defn empty-evidence-outcome []
  :claim "claim:checkout/rejects" "The API rejects shipped orders"
  :evidence "evidence:checkout/empty-outcome"
    :subject "claim:checkout/rejects"
    :method "case"
    :outcome ""
    :runner "empty-evidence-outcome-runner"
    :target "aarch64-apple-darwin"
    :source-commit "source-empty-evidence-outcome"
    :artifact-digest "sha256:empty-evidence-outcome"
    :cases 1
    :seed 0
    :generator "empty-evidence-outcome-generator"
    :producer "empty-evidence-outcome-producer"
    :tool-version "0.2.0-dev"
    :timestamp "2026-07-28T00:00:00Z"
    :independence "same-author"
  true)
LSHARP
cat >"$VALIDATION_EMPTY_EVIDENCE_INDEPENDENCE_SOURCE" <<'LSHARP'
(defn empty-evidence-independence []
  :claim "claim:checkout/rejects" "The API rejects shipped orders"
  :evidence "evidence:checkout/empty-independence"
    :subject "claim:checkout/rejects"
    :method "case"
    :outcome "pass"
    :runner "empty-evidence-independence-runner"
    :target "aarch64-apple-darwin"
    :source-commit "source-empty-evidence-independence"
    :artifact-digest "sha256:empty-evidence-independence"
    :cases 1
    :seed 0
    :generator "empty-evidence-independence-generator"
    :producer "empty-evidence-independence-producer"
    :tool-version "0.2.0-dev"
    :timestamp "2026-07-28T00:00:00Z"
    :independence ""
  true)
LSHARP
cat >"$VALIDATION_EVIDENCE_REQUIRED_PRECEDENCE_SOURCE" <<'LSHARP'
(defn evidence-required-precedence []
  :claim "claim:checkout/rejects" "The API rejects shipped orders"
  :evidence "evidence:checkout"
    :subject "claim:checkout/rejects"
    :method "case"
    :outcome "pass"
    :runner ""
    :target "aarch64-apple-darwin"
    :source-commit "source-required-precedence"
    :artifact-digest "sha256:required-precedence"
    :cases 1
    :seed 0
    :generator "required-precedence-generator"
    :producer "required-precedence-producer"
    :tool-version "0.2.0-dev"
    :timestamp "2026-07-28T00:00:00Z"
    :independence "same-author"
  true)
LSHARP
cat >"$VALIDATION_DUPLICATE_EVIDENCE_SOURCE" <<'LSHARP'
(defn duplicate-evidence []
  :claim "claim:checkout/rejects" "The API rejects shipped orders"
  :evidence "evidence:checkout/duplicate"
    :subject "claim:checkout/rejects"
    :method "case"
    :outcome "pass"
    :runner "duplicate-evidence-first-runner"
    :target "aarch64-apple-darwin"
    :source-commit "source-duplicate-evidence"
    :artifact-digest "sha256:duplicate-evidence-first"
    :cases 1
    :seed 0
    :generator "duplicate-evidence-generator"
    :producer "duplicate-evidence-producer"
    :tool-version "0.2.0-dev"
    :timestamp "2026-07-28T00:00:00Z"
    :independence "same-author"
  :evidence "evidence:checkout/duplicate"
    :subject "claim:checkout/rejects"
    :method "case"
    :outcome "pass"
    :runner "duplicate-evidence-second-runner"
    :target "aarch64-apple-darwin"
    :source-commit "source-duplicate-evidence"
    :artifact-digest "sha256:duplicate-evidence-second"
    :cases 1
    :seed 0
    :generator "duplicate-evidence-generator"
    :producer "duplicate-evidence-producer"
    :tool-version "0.2.0-dev"
    :timestamp "2026-07-28T00:00:00Z"
    :independence "same-author"
  true)
LSHARP
cat >"$VALIDATION_DUPLICATE_EVIDENCE_FIELD_SOURCE" <<'LSHARP'
(defn duplicate-evidence-field []
  :claim "claim:checkout/rejects" "The API rejects shipped orders"
  :evidence "evidence:checkout/duplicate-field"
    :subject "claim:checkout/rejects"
    :subject "claim:checkout/rejects"
    :method "case"
    :outcome "pass"
    :runner "duplicate-evidence-field-runner"
    :target "aarch64-apple-darwin"
    :source-commit "source-duplicate-evidence-field"
    :artifact-digest "sha256:duplicate-evidence-field"
    :cases 1
    :seed 0
    :generator "duplicate-evidence-field-generator"
    :producer "duplicate-evidence-field-producer"
    :tool-version "0.2.0-dev"
    :timestamp "2026-07-28T00:00:00Z"
    :independence "same-author"
  true)
LSHARP
cat >"$VALIDATION_EMPTY_EVIDENCE_GENERATOR_SOURCE" <<'LSHARP'
(defn empty-evidence-generator []
  :claim "claim:checkout/rejects" "The API rejects shipped orders"
  :evidence "evidence:checkout/empty-generator"
    :subject "claim:checkout/rejects"
    :method "case"
    :outcome "pass"
    :runner "empty-evidence-generator-runner"
    :target "aarch64-apple-darwin"
    :source-commit "source-empty-evidence-generator"
    :artifact-digest "sha256:empty-evidence-generator"
    :cases 1
    :seed 0
    :generator ""
    :producer "empty-evidence-generator-producer"
    :tool-version "0.2.0-dev"
    :timestamp "2026-07-28T00:00:00Z"
    :independence "same-author"
  true)
LSHARP
cat >"$VALIDATION_WHITESPACE_EVIDENCE_COVERAGE_SOURCE" <<'LSHARP'
(defn whitespace-evidence-coverage []
  :claim "claim:checkout/rejects" "The API rejects shipped orders"
  :evidence "evidence:checkout/whitespace-coverage"
    :subject "claim:checkout/rejects"
    :method "property"
    :outcome "pass"
    :runner "whitespace-evidence-coverage-runner"
    :target "aarch64-apple-darwin"
    :source-commit "source-whitespace-evidence-coverage"
    :artifact-digest "sha256:whitespace-evidence-coverage"
    :cases 1
    :seed 0
    :generator "whitespace-evidence-coverage-generator"
    :coverage [("  " 1)]
    :producer "whitespace-evidence-coverage-producer"
    :tool-version "0.2.0-dev"
    :timestamp "2026-07-29T00:00:00Z"
    :independence "same-author"
  true)
LSHARP
cat >"$VALIDATION_UNICODE_WHITESPACE_EVIDENCE_COVERAGE_SOURCE" <<'LSHARP'
(defn unicode-whitespace-evidence-coverage []
  :claim "claim:checkout/rejects" "The API rejects shipped orders"
  :evidence "evidence:checkout/unicode-whitespace-coverage"
    :subject "claim:checkout/rejects"
    :method "property"
    :outcome "pass"
    :runner "unicode-whitespace-evidence-coverage-runner"
    :target "aarch64-apple-darwin"
    :source-commit "source-unicode-whitespace-evidence-coverage"
    :artifact-digest "sha256:unicode-whitespace-evidence-coverage"
    :cases 1
    :seed 0
    :generator "unicode-whitespace-evidence-coverage-generator"
    :coverage [(" " 1)]
    :producer "unicode-whitespace-evidence-coverage-producer"
    :tool-version "0.2.0-dev"
    :timestamp "2026-07-29T00:00:00Z"
    :independence "same-author"
  true)
LSHARP
cat >"$VALIDATION_NEGATIVE_EVIDENCE_COVERAGE_SOURCE" <<'LSHARP'
(defn negative-evidence-coverage-count []
  :claim "claim:checkout/rejects" "The API rejects shipped orders"
  :evidence "evidence:checkout/negative-coverage-count"
    :subject "claim:checkout/rejects"
    :method "property"
    :outcome "pass"
    :runner "negative-evidence-coverage-count-runner"
    :target "aarch64-apple-darwin"
    :source-commit "source-negative-evidence-coverage-count"
    :artifact-digest "sha256:negative-evidence-coverage-count"
    :cases 1
    :seed 0
    :generator "negative-evidence-coverage-count-generator"
    :coverage [("negative" -1)]
    :producer "negative-evidence-coverage-count-producer"
    :tool-version "0.2.0-dev"
    :timestamp "2026-07-29T00:00:00Z"
    :independence "same-author"
  true)
LSHARP
cat >"$VALIDATION_NEGATIVE_EVIDENCE_CASES_SOURCE" <<'LSHARP'
(defn negative-evidence-cases []
  :claim "claim:checkout/rejects" "The API rejects shipped orders"
  :evidence "evidence:checkout/negative-cases"
    :subject "claim:checkout/rejects"
    :method "property"
    :outcome "pass"
    :runner "negative-evidence-cases-runner"
    :target "aarch64-apple-darwin"
    :source-commit "source-negative-evidence-cases"
    :artifact-digest "sha256:negative-evidence-cases"
    :cases -1
    :seed 0
    :generator "negative-evidence-cases-generator"
    :producer "negative-evidence-cases-producer"
    :tool-version "0.2.0-dev"
    :timestamp "2026-07-29T00:00:00Z"
    :independence "same-author"
  true)
LSHARP
cat >"$VALIDATION_NEGATIVE_EVIDENCE_SEED_SOURCE" <<'LSHARP'
(defn negative-evidence-seed []
  :claim "claim:checkout/rejects" "The API rejects shipped orders"
  :evidence "evidence:checkout/negative-seed"
    :subject "claim:checkout/rejects"
    :method "property"
    :outcome "pass"
    :runner "negative-evidence-seed-runner"
    :target "aarch64-apple-darwin"
    :source-commit "source-negative-evidence-seed"
    :artifact-digest "sha256:negative-evidence-seed"
    :cases 1
    :seed -1
    :generator "negative-evidence-seed-generator"
    :producer "negative-evidence-seed-producer"
    :tool-version "0.2.0-dev"
    :timestamp "2026-07-29T00:00:00Z"
    :independence "same-author"
  true)
LSHARP
cat >"$VALIDATION_NEGATIVE_EVIDENCE_SHRINKS_SOURCE" <<'LSHARP'
(defn negative-evidence-shrinks []
  :claim "claim:checkout/rejects" "The API rejects shipped orders"
  :evidence "evidence:checkout/negative-shrinks"
    :subject "claim:checkout/rejects"
    :method "property"
    :outcome "pass"
    :runner "negative-evidence-shrinks-runner"
    :target "aarch64-apple-darwin"
    :source-commit "source-negative-evidence-shrinks"
    :artifact-digest "sha256:negative-evidence-shrinks"
    :cases 1
    :seed 0
    :generator "negative-evidence-shrinks-generator"
    :shrinks [-1]
    :producer "negative-evidence-shrinks-producer"
    :tool-version "0.2.0-dev"
    :timestamp "2026-07-29T00:00:00Z"
    :independence "same-author"
  true)
LSHARP
cat >"$VALIDATION_EMPTY_EVIDENCE_RUNNER_SOURCE" <<'LSHARP'
(defn empty-evidence-runner []
  :claim "claim:checkout/rejects" "The API rejects shipped orders"
  :evidence "evidence:checkout/empty-runner"
    :subject "claim:checkout/rejects"
    :method "case"
    :outcome "pass"
    :runner ""
    :target "aarch64-apple-darwin"
    :source-commit "source-empty-evidence-runner"
    :artifact-digest "sha256:empty-evidence-runner"
    :cases 1
    :seed 0
    :generator "empty-evidence-runner-generator"
    :producer "empty-evidence-runner-producer"
    :tool-version "0.2.0-dev"
    :timestamp "2026-07-28T00:00:00Z"
    :independence "same-author"
  true)
LSHARP
cat >"$VALIDATION_EMPTY_EVIDENCE_TARGET_SOURCE" <<'LSHARP'
(defn empty-evidence-target []
  :claim "claim:checkout/rejects" "The API rejects shipped orders"
  :evidence "evidence:checkout/empty-target"
    :subject "claim:checkout/rejects"
    :method "case"
    :outcome "pass"
    :runner "empty-evidence-target-runner"
    :target ""
    :source-commit "source-empty-evidence-target"
    :artifact-digest "sha256:empty-evidence-target"
    :cases 1
    :seed 0
    :generator "empty-evidence-target-generator"
    :producer "empty-evidence-target-producer"
    :tool-version "0.2.0-dev"
    :timestamp "2026-07-28T00:00:00Z"
    :independence "same-author"
  true)
LSHARP
cat >"$VALIDATION_EMPTY_EVIDENCE_SOURCE_COMMIT_SOURCE" <<'LSHARP'
(defn empty-evidence-source-commit []
  :claim "claim:checkout/rejects" "The API rejects shipped orders"
  :evidence "evidence:checkout/empty-source-commit"
    :subject "claim:checkout/rejects"
    :method "case"
    :outcome "pass"
    :runner "empty-evidence-source-commit-runner"
    :target "aarch64-apple-darwin"
    :source-commit ""
    :artifact-digest "sha256:empty-evidence-source-commit"
    :cases 1
    :seed 0
    :generator "empty-evidence-source-commit-generator"
    :producer "empty-evidence-source-commit-producer"
    :tool-version "0.2.0-dev"
    :timestamp "2026-07-28T00:00:00Z"
    :independence "same-author"
  true)
LSHARP
cat >"$VALIDATION_EMPTY_EVIDENCE_ARTIFACT_DIGEST_SOURCE" <<'LSHARP'
(defn empty-evidence-artifact-digest []
  :claim "claim:checkout/rejects" "The API rejects shipped orders"
  :evidence "evidence:checkout/empty-artifact-digest"
    :subject "claim:checkout/rejects"
    :method "case"
    :outcome "pass"
    :runner "empty-evidence-artifact-digest-runner"
    :target "aarch64-apple-darwin"
    :source-commit "source-empty-evidence-artifact-digest"
    :artifact-digest ""
    :cases 1
    :seed 0
    :generator "empty-evidence-artifact-digest-generator"
    :producer "empty-evidence-artifact-digest-producer"
    :tool-version "0.2.0-dev"
    :timestamp "2026-07-28T00:00:00Z"
    :independence "same-author"
  true)
LSHARP
cat >"$VALIDATION_EMPTY_EVIDENCE_PRODUCER_SOURCE" <<'LSHARP'
(defn empty-evidence-producer []
  :claim "claim:checkout/rejects" "The API rejects shipped orders"
  :evidence "evidence:checkout/empty-producer"
    :subject "claim:checkout/rejects"
    :method "case"
    :outcome "pass"
    :runner "empty-evidence-producer-runner"
    :target "aarch64-apple-darwin"
    :source-commit "source-empty-evidence-producer"
    :artifact-digest "sha256:empty-evidence-producer"
    :cases 1
    :seed 0
    :generator "empty-evidence-producer-generator"
    :producer ""
    :tool-version "0.2.0-dev"
    :timestamp "2026-07-28T00:00:00Z"
    :independence "same-author"
  true)
LSHARP
cat >"$VALIDATION_EMPTY_EVIDENCE_TOOL_VERSION_SOURCE" <<'LSHARP'
(defn empty-evidence-tool-version []
  :claim "claim:checkout/rejects" "The API rejects shipped orders"
  :evidence "evidence:checkout/empty-tool-version"
    :subject "claim:checkout/rejects"
    :method "case"
    :outcome "pass"
    :runner "empty-evidence-tool-version-runner"
    :target "aarch64-apple-darwin"
    :source-commit "source-empty-evidence-tool-version"
    :artifact-digest "sha256:empty-evidence-tool-version"
    :cases 1
    :seed 0
    :generator "empty-evidence-tool-version-generator"
    :producer "empty-evidence-tool-version-producer"
    :tool-version ""
    :timestamp "2026-07-28T00:00:00Z"
    :independence "same-author"
  true)
LSHARP
cat >"$VALIDATION_EMPTY_EVIDENCE_TIMESTAMP_SOURCE" <<'LSHARP'
(defn empty-evidence-timestamp []
  :claim "claim:checkout/rejects" "The API rejects shipped orders"
  :evidence "evidence:checkout/empty-timestamp"
    :subject "claim:checkout/rejects"
    :method "case"
    :outcome "pass"
    :runner "empty-evidence-timestamp-runner"
    :target "aarch64-apple-darwin"
    :source-commit "source-empty-evidence-timestamp"
    :artifact-digest "sha256:empty-evidence-timestamp"
    :cases 1
    :seed 0
    :generator "empty-evidence-timestamp-generator"
    :producer "empty-evidence-timestamp-producer"
    :tool-version "0.2.0-dev"
    :timestamp ""
    :independence "same-author"
  true)
LSHARP
cat >"$VALIDATION_WHITESPACE_EVIDENCE_RUNNER_SOURCE" <<'LSHARP'
(defn whitespace-evidence-runner []
  :claim "claim:checkout/rejects" "The API rejects shipped orders"
  :evidence "evidence:checkout/whitespace-runner"
    :subject "claim:checkout/rejects"
    :method "case"
    :outcome "pass"
    :runner "  "
    :target "aarch64-apple-darwin"
    :source-commit "source-whitespace-evidence-runner"
    :artifact-digest "sha256:whitespace-evidence-runner"
    :cases 1
    :seed 0
    :generator "whitespace-evidence-runner-generator"
    :producer "whitespace-evidence-runner-producer"
    :tool-version "0.2.0-dev"
    :timestamp "2026-07-28T00:00:00Z"
    :independence "same-author"
  true)
LSHARP
cat >"$VALIDATION_UNICODE_WHITESPACE_EVIDENCE_RUNNER_SOURCE" <<'LSHARP'
(defn unicode-whitespace-evidence-runner []
  :claim "claim:checkout/rejects" "The API rejects shipped orders"
  :evidence "evidence:checkout/unicode-whitespace-runner"
    :subject "claim:checkout/rejects"
    :method "case"
    :outcome "pass"
    :runner " "
    :target "aarch64-apple-darwin"
    :source-commit "source-unicode-whitespace-runner"
    :artifact-digest "sha256:unicode-whitespace-runner"
    :cases 1
    :seed 0
    :generator "unicode-whitespace-runner-generator"
    :producer "unicode-whitespace-runner-producer"
    :tool-version "0.2.0-dev"
    :timestamp "2026-07-29T00:00:00Z"
    :independence "same-author"
  true)
LSHARP
cat >"$VALIDATION_WHITESPACE_EVIDENCE_SUBJECT_SOURCE" <<'LSHARP'
(defn whitespace-evidence-subject []
  :claim "claim:checkout/rejects" "The API rejects shipped orders"
  :evidence "evidence:checkout/whitespace-subject"
    :subject "  "
    :method "case"
    :outcome "pass"
    :runner "whitespace-subject-runner"
    :target "aarch64-apple-darwin"
    :source-commit "source-whitespace-subject"
    :artifact-digest "sha256:whitespace-subject"
    :cases 1
    :seed 0
    :generator "whitespace-subject-generator"
    :producer "whitespace-subject-producer"
    :tool-version "0.2.0-dev"
    :timestamp "2026-07-28T00:00:00Z"
    :independence "same-author"
  true)
LSHARP
cat >"$VALIDATION_WHITESPACE_EVIDENCE_ID_SOURCE" <<'LSHARP'
(defn whitespace-evidence-id []
  :claim "claim:checkout/rejects" "The API rejects shipped orders"
  :evidence "  "
    :subject "claim:checkout/rejects"
    :method "case"
    :outcome "pass"
    :runner "whitespace-id-runner"
    :target "aarch64-apple-darwin"
    :source-commit "source-whitespace-id"
    :artifact-digest "sha256:whitespace-id"
    :cases 1
    :seed 0
    :generator "whitespace-id-generator"
    :producer "whitespace-id-producer"
    :tool-version "0.2.0-dev"
    :timestamp "2026-07-28T00:00:00Z"
    :independence "same-author"
  true)
LSHARP
cat >"$VALIDATION_EMPTY_EVIDENCE_ID_SOURCE" <<'LSHARP'
(defn empty-evidence-id []
  :claim "claim:checkout/rejects" "The API rejects shipped orders"
  :evidence ""
    :subject "claim:checkout/rejects"
    :method "case"
    :outcome "pass"
    :runner "empty-id-runner"
    :target "aarch64-apple-darwin"
    :source-commit "source-empty-id"
    :artifact-digest "sha256:empty-id"
    :cases 1
    :seed 0
    :generator "empty-id-generator"
    :producer "empty-id-producer"
    :tool-version "0.2.0-dev"
    :timestamp "2026-07-28T00:00:00Z"
    :independence "same-author"
  true)
LSHARP
cat >"$VALIDATION_MISSING_NODE_ID_SOURCE" <<'LSHARP'
(defn missing-node-id []
  :intent "Users can cancel an order"
  true)
LSHARP
cat >"$VALIDATION_MISSING_NODE_TEXT_SOURCE" <<'LSHARP'
(defn missing-node-text []
  :claim "claim:checkout/missing-text"
  true)
LSHARP
cat >"$VALIDATION_WHITESPACE_NODE_TEXT_SOURCE" <<'LSHARP'
(defn whitespace-node-text []
  :claim "claim:checkout/whitespace-text" "  "
  true)
LSHARP
cat >"$VALIDATION_UNICODE_WHITESPACE_NODE_TEXT_SOURCE" <<'LSHARP'
(defn unicode-whitespace-node-text []
  :claim "claim:checkout/unicode-whitespace-text" " "
  true)
LSHARP
cat >"$VALIDATION_NODE_TEXT_PRECEDENCE_SOURCE" <<'LSHARP'
(defn node-text-precedence []
  :claim "claim:checkout/bad/key" "  "
  true)
LSHARP
cat >"$VALIDATION_MISSING_REVIEW_SOURCE" <<'LSHARP'
(defn missing-review-edge []
  :claim "claim:checkout/rejects" "The API rejects shipped orders"
  :review "review:checkout/registered" "sha256:review-provenance" "redacted"
  :evaluates "review:checkout/missing" "claim:checkout/rejects"
  true)
LSHARP
cat >"$VALIDATION_MISSING_REVIEW_SUBJECT_KIND_SOURCE" <<'LSHARP'
(defn missing-review-before-subject-kind []
  :review "review:checkout/registered" "sha256:review-provenance" "public"
  :evaluates "review:checkout/missing" "review:checkout/registered"
  true)
LSHARP
cat >"$VALIDATION_DUPLICATE_REVIEW_SOURCE" <<'LSHARP'
(defn duplicate-review []
  :claim "claim:checkout/rejects" "The API rejects shipped orders"
  :review "review:checkout/duplicate" "sha256:review-one" "redacted"
  :review "review:checkout/duplicate" "sha256:review-two" "redacted"
  true)
LSHARP
cat >"$VALIDATION_INVALID_REVIEW_SOURCE" <<'LSHARP'
(defn invalid-review []
  :claim "claim:checkout/rejects" "The API rejects shipped orders"
  :review "review:checkout/invalid" "sha256:review-provenance" "private"
  true)
LSHARP
cat >"$VALIDATION_INVALID_REVIEW_DIGEST_SOURCE" <<'LSHARP'
(defn invalid-review-digest []
  :claim "claim:checkout/rejects" "The API rejects shipped orders"
  :review "review:checkout/blank-digest" "   " "redacted"
  true)
LSHARP
cat >"$VALIDATION_UNICODE_WHITESPACE_REVIEW_DIGEST_SOURCE" <<'LSHARP'
(defn unicode-whitespace-review-digest []
  :claim "claim:checkout/rejects" "The API rejects shipped orders"
  :review "review:checkout/unicode-whitespace-digest" " " "redacted"
  true)
LSHARP
cat >"$VALIDATION_REVIEW_REQUIRED_PRECEDENCE_SOURCE" <<'LSHARP'
(defn review-required-precedence []
  :claim "claim:checkout/rejects" "The API rejects shipped orders"
  :review "review:checkout" "   " "redacted"
  true)
LSHARP
cat >"$VALIDATION_INVALID_REVIEW_ID_SOURCE" <<'LSHARP'
(defn invalid-review-id []
  :claim "claim:checkout/rejects" "The API rejects shipped orders"
  :review "review:checkout" "sha256:review-provenance" "redacted"
  true)
LSHARP
cat >"$VALIDATION_EMPTY_REVIEW_ID_SOURCE" <<'LSHARP'
(defn empty-review-id []
  :claim "claim:checkout/rejects" "The API rejects shipped orders"
  :review "" "sha256:review-provenance" "redacted"
  true)
LSHARP
cat >"$VALIDATION_MALFORMED_REVIEW_SOURCE" <<'LSHARP'
(defn malformed-review []
  :claim "claim:checkout/rejects" "The API rejects shipped orders"
  :review "review:checkout/malformed" "sha256:review-provenance"
  true)
LSHARP
cat >"$VALIDATION_MALFORMED_REVIEW_EXTRA_SOURCE" <<'LSHARP'
(defn malformed-review-extra []
  :claim "claim:checkout/rejects" "The API rejects shipped orders"
  :review "review:checkout/malformed" "sha256:review-provenance" "redacted" "extra"
  true)
LSHARP
cat >"$VALIDATION_MALFORMED_REVIEW_EDGE_SOURCE" <<'LSHARP'
(defn malformed-review-edge []
  :review "review:checkout/registered" "sha256:review-provenance" "redacted"
  :evaluates "review:checkout/registered"
  true)
LSHARP
cat >"$VALIDATION_MALFORMED_INVALIDATION_EDGE_SOURCE" <<'LSHARP'
(defn malformed-invalidation-edge []
  :review "review:checkout/registered" "sha256:review-provenance" "redacted"
  :invalidates "change:checkout/api-v2"
  true)
LSHARP
cat >"$VALIDATION_MALFORMED_REVIEW_EDGE_EXTRA_SOURCE" <<'LSHARP'
(defn malformed-review-edge-extra []
  :review "review:checkout/registered" "sha256:review-provenance" "redacted"
  :evaluates "review:checkout/registered" "claim:checkout/rejects" "extra"
  true)
LSHARP
cat >"$VALIDATION_MALFORMED_INVALIDATION_EDGE_EXTRA_SOURCE" <<'LSHARP'
(defn malformed-invalidation-edge-extra []
  :review "review:checkout/registered" "sha256:review-provenance" "redacted"
  :invalidates "change:checkout/api-v2" "evidence:checkout/review" "extra"
  true)
LSHARP
cat >"$VALIDATION_REVIEW_SUBJECT_KIND_SOURCE" <<'LSHARP'
(defn review-subject-kind-mismatch []
  :claim "claim:checkout/rejects" "The API rejects shipped orders"
  :review "review:checkout/registered" "sha256:review-provenance" "redacted"
  :evaluates "review:checkout/registered" "review:checkout/registered"
  true)
LSHARP
cat >"$VALIDATION_INVALIDATION_SUBJECT_KIND_SOURCE" <<'LSHARP'
(defn invalidation-subject-kind-mismatch []
  :review "review:checkout/registered" "sha256:review-provenance" "redacted"
  :invalidates "change:checkout/api-v2" "claim:checkout/rejects"
  true)
LSHARP
cat >"$VALIDATION_INVALIDATION_MISSING_REVIEW_SOURCE" <<'LSHARP'
(defn invalidation-missing-review []
  :review "review:checkout/registered" "sha256:review-provenance" "redacted"
  :invalidates "change:checkout/api-v2" "review:checkout/missing"
  true)
LSHARP
cat >"$VALIDATION_REVIEW_EDGE_EVIDENCE_SOURCE" <<'LSHARP'
(defn review-edge-evidence-registry []
  :review "review:checkout/registered" "sha256:review-provenance" "redacted"
  :evaluates "review:checkout/registered" "evidence:checkout/missing"
  true)
LSHARP
cat >"$VALIDATION_INVALIDATION_EDGE_EVIDENCE_SOURCE" <<'LSHARP'
(defn invalidation-edge-evidence-registry []
  :review "review:checkout/registered" "sha256:review-provenance" "redacted"
  :invalidates "change:checkout/api-v2" "evidence:checkout/missing"
  true)
LSHARP

run_command() {
  local label="$1"
  local bootstrap="$2"
  shift 2

  local status=0
  set +e
  if [[ "$bootstrap" == "1" ]]; then
    PATH="$BLOCKED_TOOL_DIR:$PATH" \
      "$RUNNER" \
        --stage0-dir "$STAGE0_DIR" \
        --source-root "$SOURCE_ROOT" \
        --stage-dir "$STAGE_DIR" \
        --bootstrap \
        "$@" >"$WORK_DIR/$label.stdout" 2>"$WORK_DIR/$label.stderr"
    status=$?
  else
    PATH="$BLOCKED_TOOL_DIR:$PATH" \
      "$RUNNER" \
        --stage0-dir "$STAGE0_DIR" \
        --source-root "$SOURCE_ROOT" \
        --stage-dir "$STAGE_DIR" \
        "$@" >"$WORK_DIR/$label.stdout" 2>"$WORK_DIR/$label.stderr"
    status=$?
  fi
  set -e

  if [[ "$status" -ne 0 ]]; then
    echo "ERROR: $label failed with exit=$status" >&2
    cat "$WORK_DIR/$label.stdout" >&2
    cat "$WORK_DIR/$label.stderr" >&2
    exit "$status"
  fi
  if [[ -s "$WORK_DIR/$label.stderr" ]]; then
    echo "ERROR: $label emitted stderr" >&2
    cat "$WORK_DIR/$label.stderr" >&2
    exit 1
  fi
}

require_line() {
  local label="$1"
  local expected="$2"
  grep -Fx "$expected" "$WORK_DIR/$label.stdout" >/dev/null || {
    echo "ERROR: $label stdout is missing $expected" >&2
    cat "$WORK_DIR/$label.stdout" >&2
    exit 1
  }
}

require_exact_output() {
  local label="$1"
  local expected="$2"
  if ! printf '%s' "$expected" | cmp -s - "$WORK_DIR/$label.stdout"; then
    echo "ERROR: $label stdout does not match expected output" >&2
    cat "$WORK_DIR/$label.stdout" >&2
    exit 1
  fi
}

run_expected_failure() {
  local label="$1"
  local bootstrap="$2"
  shift 2

  local status=0
  set +e
  if [[ "$bootstrap" == "1" ]]; then
    PATH="$BLOCKED_TOOL_DIR:$PATH" \
      "$RUNNER" \
        --stage0-dir "$STAGE0_DIR" \
        --source-root "$SOURCE_ROOT" \
        --stage-dir "$STAGE_DIR" \
        --bootstrap \
        "$@" >"$WORK_DIR/$label.stdout" 2>"$WORK_DIR/$label.stderr"
    status=$?
  else
    PATH="$BLOCKED_TOOL_DIR:$PATH" \
      "$RUNNER" \
        --stage0-dir "$STAGE0_DIR" \
        --source-root "$SOURCE_ROOT" \
        --stage-dir "$STAGE_DIR" \
        "$@" >"$WORK_DIR/$label.stdout" 2>"$WORK_DIR/$label.stderr"
    status=$?
  fi
  set -e

  [[ "$status" -eq 2 ]] || {
    echo "ERROR: $label expected exit 2, got $status" >&2
    cat "$WORK_DIR/$label.stdout" >&2
    cat "$WORK_DIR/$label.stderr" >&2
    exit 1
  }
  [[ ! -s "$WORK_DIR/$label.stderr" ]] || {
    echo "ERROR: $label emitted stderr" >&2
    cat "$WORK_DIR/$label.stderr" >&2
    exit 1
  }
}

run_expected_validation_error() {
  local label="$1"
  shift

  local status=0
  set +e
  PATH="$BLOCKED_TOOL_DIR:$PATH" \
    "$RUNNER" \
      --stage0-dir "$STAGE0_DIR" \
      --source-root "$SOURCE_ROOT" \
      --stage-dir "$STAGE_DIR" \
      "$@" >"$WORK_DIR/$label.stdout" 2>"$WORK_DIR/$label.stderr"
  status=$?
  set -e

  [[ "$status" -eq 1 ]] || {
    echo "ERROR: $label expected exit 1, got $status" >&2
    cat "$WORK_DIR/$label.stdout" >&2
    cat "$WORK_DIR/$label.stderr" >&2
    exit 1
  }
  [[ ! -s "$WORK_DIR/$label.stdout" ]] || {
    echo "ERROR: $label must not emit a report on validation error" >&2
    cat "$WORK_DIR/$label.stdout" >&2
    exit 1
  }
  [[ -s "$WORK_DIR/$label.stderr" ]] || {
    echo "ERROR: $label must emit a diagnostic on validation error" >&2
    exit 1
  }
}

run_report_failure() {
  local label="$1"
  local bootstrap="$2"
  shift 2

  local status=0
  set +e
  if [[ "$bootstrap" == "1" ]]; then
    PATH="$BLOCKED_TOOL_DIR:$PATH" \
      "$RUNNER" \
        --stage0-dir "$STAGE0_DIR" \
        --source-root "$SOURCE_ROOT" \
        --stage-dir "$STAGE_DIR" \
        --bootstrap \
        "$@" >"$WORK_DIR/$label.stdout" 2>"$WORK_DIR/$label.stderr"
    status=$?
  else
    PATH="$BLOCKED_TOOL_DIR:$PATH" \
      "$RUNNER" \
        --stage0-dir "$STAGE0_DIR" \
        --source-root "$SOURCE_ROOT" \
        --stage-dir "$STAGE_DIR" \
        "$@" >"$WORK_DIR/$label.stdout" 2>"$WORK_DIR/$label.stderr"
    status=$?
  fi
  set -e

  [[ "$status" -eq 1 ]] || {
    echo "ERROR: $label expected exit 1, got $status" >&2
    cat "$WORK_DIR/$label.stdout" >&2
    cat "$WORK_DIR/$label.stderr" >&2
    exit 1
  }
  [[ -s "$WORK_DIR/$label.stdout" ]] || {
    echo "ERROR: $label must emit a failure report" >&2
    exit 1
  }
  [[ ! -s "$WORK_DIR/$label.stderr" ]] || {
    echo "ERROR: $label emitted stderr" >&2
    cat "$WORK_DIR/$label.stderr" >&2
    exit 1
  }
}

run_command parse 1 parse "$INPUT"
for expected in decls:1 first-decl:defn first-body:int diagnostics:0; do
  require_line parse "$expected"
done

run_command check 0 check "$INPUT"
for expected in Int diagnostics:0; do
  require_line check "$expected"
done

run_command fmt 0 fmt "$INPUT"
require_line fmt '(defn main [] 42)'

run_command test 0 test "$INPUT"
require_exact_output test $'examples:0\ninvariants:0\nfailures:0\n'

run_command metadata-test 0 test "$METADATA"
require_exact_output metadata-test $'examples:2\ninvariants:1\nfailures:0\n'

run_command property-json 0 test "$PROPERTY" --format json
python3 - "$WORK_DIR/property-json.stdout" <<'PY'
import json
import pathlib
import sys

lines = pathlib.Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
if len(lines) != 1:
    raise SystemExit(f"property JSON must contain one report line: {lines!r}")
report = json.loads(lines[0])
conformance = report["implementation_conformance"]
if conformance["status"] != "pass":
    raise SystemExit(f"property JSON status is not pass: {report!r}")
if conformance["method"] != "sampled-property":
    raise SystemExit(f"property JSON method is invalid: {report!r}")
if conformance["cases"] != 5 or conformance["coverage"]["executed"] != 5:
    raise SystemExit(f"property JSON coverage is invalid: {report!r}")
if report["intent_validation"]["status"] != "unknown":
    raise SystemExit(f"property JSON intent status is invalid: {report!r}")
PY

run_expected_failure vacuous-property-json 0 test "$VACUOUS_PROPERTY" --format json
python3 - "$WORK_DIR/vacuous-property-json.stdout" <<'PY'
import json
import pathlib
import sys

lines = pathlib.Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
if len(lines) != 1:
    raise SystemExit(f"vacuous property JSON must contain one report line: {lines!r}")
report = json.loads(lines[0])
conformance = report["implementation_conformance"]
if conformance["status"] != "fail":
    raise SystemExit(f"vacuous property JSON status is not fail: {report!r}")
diagnostics = conformance["diagnostics"]
if diagnostics["count"] != 1 or diagnostics["firstErrorCode"] != 2005:
    raise SystemExit(f"vacuous property JSON diagnostic is invalid: {report!r}")
if report["intent_validation"]["status"] != "unknown":
    raise SystemExit(f"vacuous property JSON intent status is invalid: {report!r}")
PY

run_expected_failure dynamic-complement-property-json 0 test "$DYNAMIC_COMPLEMENT_PROPERTY" --format json
python3 - "$WORK_DIR/dynamic-complement-property-json.stdout" <<'PY'
import json
import pathlib
import sys

lines = pathlib.Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
if len(lines) != 1:
    raise SystemExit(f"dynamic complement property JSON must contain one report line: {lines!r}")
report = json.loads(lines[0])
conformance = report["implementation_conformance"]
if conformance["status"] != "fail":
    raise SystemExit(f"dynamic complement property JSON status is not fail: {report!r}")
diagnostics = conformance["diagnostics"]
if diagnostics["count"] != 1 or diagnostics["firstErrorCode"] != 2005:
    raise SystemExit(f"dynamic complement property JSON diagnostic is invalid: {report!r}")
if report["intent_validation"]["status"] != "unknown":
    raise SystemExit(f"dynamic complement property JSON intent status is invalid: {report!r}")
PY

run_expected_failure validation-manifest-unknown 0 validate \
  --source "$VALIDATION_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_MANIFEST"
cmp -s "$EXPECTED_VALIDATION_MANIFEST" "$VALIDATION_MANIFEST" \
  || die "EC-M3-01 native manifest bytes differ from the Rust canonical fixture"
python3 - "$WORK_DIR/validation-manifest-unknown.stdout" <<'PY'
import json
import pathlib
import sys

lines = pathlib.Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
if len(lines) != 1:
    raise SystemExit(f"validation JSON must contain one report line: {lines!r}")
report = json.loads(lines[0])
if report.get("status") != "unknown":
    raise SystemExit(f"validation status must be unknown: {report!r}")
if report.get("open_questions") != 1 or report.get("independent_reviews") != 0:
    raise SystemExit(f"validation unknown metrics are invalid: {report!r}")
if report.get("trace_gaps") != [] or report.get("contradicting_observations") != 0:
    raise SystemExit(f"validation trace/contradiction metrics are invalid: {report!r}")
PY

run_expected_failure validation-manifest-roundtrip-json 0 validate \
  "$VALIDATION_MANIFEST" \
  --format json
cmp -s "$WORK_DIR/validation-manifest-unknown.stdout" \
  "$WORK_DIR/validation-manifest-roundtrip-json.stdout" \
  || die "validation manifest roundtrip must preserve source report"

run_expected_failure validation-text-unknown 0 validate \
  --source "$VALIDATION_SOURCE" \
  --format text
require_exact_output validation-text-unknown $'status: unknown\nopen-questions: 1\nindependent-reviews: 0\ncontradicting-observations: 0\nstale-reviews: 0\nstale-evidence: 0\n'

run_expected_validation_error validation-manifest-write-failure \
  validate \
  --source "$VALIDATION_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_WRITE_FAILURE_MANIFEST"
grep -F "source validation manifest write failed" \
  "$WORK_DIR/validation-manifest-write-failure.stderr" >/dev/null \
  || die "manifest write failure must expose the stable diagnostic"
[[ ! -e "$VALIDATION_WRITE_FAILURE_MANIFEST" ]] \
  || die "manifest write failure must produce no manifest artifact"

run_command validation-pass-json 0 validate \
  --source "$VALIDATION_PASS_SOURCE" \
  --format json
python3 - "$WORK_DIR/validation-pass-json.stdout" <<'PY'
import json
import pathlib
import sys

lines = pathlib.Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
if len(lines) != 1:
    raise SystemExit(f"validation pass JSON must contain one report line: {lines!r}")
report = json.loads(lines[0])
if report.get("status") != "pass":
    raise SystemExit(f"validation pass status is invalid: {report!r}")
if report.get("trace_gaps") != [] or report.get("open_questions") != 0:
    raise SystemExit(f"validation pass graph metrics are invalid: {report!r}")
if report.get("independent_reviews") != 1:
    raise SystemExit(f"validation pass review metric is invalid: {report!r}")
if report.get("contradicting_observations") != 0:
    raise SystemExit(f"validation pass contradiction metric is invalid: {report!r}")
if report.get("stale_reviews") != 0 or report.get("stale_evidence") != 0:
    raise SystemExit(f"validation pass stale metrics are invalid: {report!r}")
PY

run_command validation-pass-text 0 validate \
  --source "$VALIDATION_PASS_SOURCE" \
  --format text
require_exact_output validation-pass-text $'status: pass\nopen-questions: 0\nindependent-reviews: 1\ncontradicting-observations: 0\nstale-reviews: 0\nstale-evidence: 0\n'

run_expected_failure validation-failed-review-json 0 validate \
  --source "$VALIDATION_FAILED_REVIEW_SOURCE" \
  --format json
python3 - "$WORK_DIR/validation-failed-review-json.stdout" <<'PY'
import json
import pathlib
import sys

lines = pathlib.Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
if len(lines) != 1:
    raise SystemExit(f"failed independent review JSON must contain one report line: {lines!r}")
report = json.loads(lines[0])
if report.get("status") != "unknown":
    raise SystemExit(f"failed independent review must leave validation unknown: {report!r}")
if report.get("trace_gaps") != [] or report.get("open_questions") != 0:
    raise SystemExit(f"failed independent review graph metrics are invalid: {report!r}")
if report.get("independent_reviews") != 0:
    raise SystemExit(f"failed independent review must not satisfy the gate: {report!r}")
if report.get("contradicting_observations") != 0:
    raise SystemExit(f"failed independent review contradiction metrics are invalid: {report!r}")
if report.get("stale_reviews") != 0 or report.get("stale_evidence") != 0:
    raise SystemExit(f"failed independent review stale metrics are invalid: {report!r}")
PY

run_expected_failure validation-failed-review-text 0 validate \
  --source "$VALIDATION_FAILED_REVIEW_SOURCE" \
  --format text
require_exact_output validation-failed-review-text $'status: unknown\nopen-questions: 0\nindependent-reviews: 0\ncontradicting-observations: 0\nstale-reviews: 0\nstale-evidence: 0\n'

run_report_failure validation-fail-json 0 validate \
  --source "$VALIDATION_FAIL_SOURCE" \
  --format json
python3 - "$WORK_DIR/validation-fail-json.stdout" <<'PY'
import json
import pathlib
import sys

lines = pathlib.Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
if len(lines) != 1:
    raise SystemExit(f"validation fail JSON must contain one report line: {lines!r}")
report = json.loads(lines[0])
if report.get("status") != "fail":
    raise SystemExit(f"validation fail status is invalid: {report!r}")
if report.get("trace_gaps") != [] or report.get("open_questions") != 0:
    raise SystemExit(f"validation fail graph metrics are invalid: {report!r}")
if report.get("independent_reviews") != 0:
    raise SystemExit(f"validation fail review metric is invalid: {report!r}")
if report.get("contradicting_observations") != 1:
    raise SystemExit(f"validation fail contradiction metric is invalid: {report!r}")
if report.get("stale_reviews") != 0 or report.get("stale_evidence") != 0:
    raise SystemExit(f"validation fail stale metrics are invalid: {report!r}")
PY

run_report_failure validation-fail-text 0 validate \
  --source "$VALIDATION_FAIL_SOURCE" \
  --format text
require_exact_output validation-fail-text $'status: fail\nopen-questions: 0\nindependent-reviews: 0\ncontradicting-observations: 1\nstale-reviews: 0\nstale-evidence: 0\n'

run_expected_failure validation-stale-json 0 validate \
  --source "$VALIDATION_STALE_SOURCE" \
  --format json
python3 - "$WORK_DIR/validation-stale-json.stdout" <<'PY'
import json
import pathlib
import sys

lines = pathlib.Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
if len(lines) != 1:
    raise SystemExit(f"validation stale JSON must contain one report line: {lines!r}")
report = json.loads(lines[0])
if report.get("status") != "unknown":
    raise SystemExit(f"validation stale status is invalid: {report!r}")
if report.get("trace_gaps") != [] or report.get("open_questions") != 0:
    raise SystemExit(f"validation stale graph metrics are invalid: {report!r}")
if report.get("independent_reviews") != 1 or report.get("contradicting_observations") != 0:
    raise SystemExit(f"validation stale review metrics are invalid: {report!r}")
if report.get("stale_reviews") != 1 or report.get("stale_evidence") != 1:
    raise SystemExit(f"validation stale propagation metrics are invalid: {report!r}")
PY

run_expected_failure validation-stale-text 0 validate \
  --source "$VALIDATION_STALE_SOURCE" \
  --format text
require_exact_output validation-stale-text $'status: unknown\nopen-questions: 0\nindependent-reviews: 1\ncontradicting-observations: 0\nstale-reviews: 1\nstale-evidence: 1\n'

run_expected_failure validation-attestation-json 0 validate \
  --source "$VALIDATION_ATTESTATION_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_ATTESTATION_MANIFEST"
python3 - "$WORK_DIR/validation-attestation-json.stdout" "$VALIDATION_ATTESTATION_MANIFEST" "$VALIDATION_ATTESTATION_SOURCE" <<'PY'
import json
import pathlib
import sys

report = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
if report.get("review_verifications") != [
    {"review_id": "review:checkout/reviewer-001", "state": "unverified"}
]:
    raise SystemExit(f"native source attestation report is invalid: {report!r}")
expected_attestation_fields = [
    "review_id",
    "subject_digest",
    "source_commit",
    "provenance_digest",
    "provider",
    "key_id",
    "algorithm",
    "signature",
    "issued_at",
    "expires_at",
    "sequence",
    "state",
    "canonical_bytes",
    "span",
]
attestations = report.get("review_attestations")
if not isinstance(attestations, list) or len(attestations) != 1:
    raise SystemExit(f"native source attestation projections are invalid: {report!r}")
attestation = attestations[0]
if list(attestation) != expected_attestation_fields:
    raise SystemExit(f"native source attestation field order is invalid: {attestation!r}")
expected_values = {
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
}
if (
    attestation.get("subject_digest") != expected_values["subject_digest"]
    or attestation.get("source_commit") != expected_values["source_commit"]
    or attestation.get("provenance_digest") != expected_values["provenance_digest"]
    or attestation.get("provider") != expected_values["provider"]
    or attestation.get("key_id") != expected_values["key_id"]
    or attestation.get("algorithm") != expected_values["algorithm"]
    or attestation.get("signature") != expected_values["signature"]
    or attestation.get("issued_at") != expected_values["issued_at"]
    or attestation.get("expires_at") != expected_values["expires_at"]
    or attestation.get("sequence") != expected_values["sequence"]
    or attestation.get("state") != expected_values["state"]
):
    raise SystemExit(f"native source attestation fields are invalid: {attestation!r}")
def append_field(out, value):
    encoded = value.encode("utf-8")
    return out + len(encoded).to_bytes(8, "big") + encoded
canonical = b"lsharp.review-attestation.v1\0"
for value in (
    expected_values["review_id"],
    expected_values["subject_digest"],
    expected_values["source_commit"],
    expected_values["provenance_digest"],
    expected_values["provider"],
    expected_values["key_id"],
    expected_values["algorithm"],
    expected_values["issued_at"],
    expected_values["expires_at"],
    str(expected_values["sequence"]),
):
    canonical = append_field(canonical, value)
if attestation.get("canonical_bytes") != list(canonical):
    raise SystemExit(f"native source attestation canonical bytes are invalid: {attestation!r}")
source = pathlib.Path(sys.argv[3]).read_text(encoding="utf-8")
expected_span = {"start": source.find(":review-attestation"), "end": source.rfind("\n  true")}
if attestation.get("span") != expected_span:
    raise SystemExit(f"native source attestation span is invalid: {attestation!r}")
manifest = json.loads(pathlib.Path(sys.argv[2]).read_text(encoding="utf-8"))
if "review_evidence_identity" in report or "review_evidence_identity" in manifest:
    raise SystemExit(
        "review identity must remain absent without explicit review context"
    )
reviews = manifest.get("reviews")
if not isinstance(reviews, list) or len(reviews) != 1:
    raise SystemExit(f"native source attestation manifest reviews are invalid: {reviews!r}")
for review in reviews:
    if review.get("verification_state") != "unverified":
        raise SystemExit(f"native source attestation manifest state is invalid: {review!r}")
PY

run_expected_failure validation-attestation-no-expiry-json 0 validate \
  --source "$VALIDATION_ATTESTATION_NO_EXPIRY_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_ATTESTATION_NO_EXPIRY_MANIFEST"
python3 - "$WORK_DIR/validation-attestation-no-expiry-json.stdout" "$VALIDATION_ATTESTATION_NO_EXPIRY_MANIFEST" "$VALIDATION_ATTESTATION_NO_EXPIRY_SOURCE" <<'PY'
import json
import pathlib
import sys

report = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
if report.get("review_verifications") != [
    {"review_id": "review:checkout/reviewer-001", "state": "unverified"}
]:
    raise SystemExit(f"native source attestation without expiry report is invalid: {report!r}")
expected_attestation_fields = [
    "review_id",
    "subject_digest",
    "source_commit",
    "provenance_digest",
    "provider",
    "key_id",
    "algorithm",
    "signature",
    "issued_at",
    "expires_at",
    "sequence",
    "state",
    "canonical_bytes",
    "span",
]
attestations = report.get("review_attestations")
if not isinstance(attestations, list) or len(attestations) != 1:
    raise SystemExit(f"native source attestation without expiry projections are invalid: {report!r}")
attestation = attestations[0]
if list(attestation) != expected_attestation_fields:
    raise SystemExit(f"native source attestation without expiry field order is invalid: {attestation!r}")
expected_values = {
    "review_id": "review:checkout/reviewer-001",
    "subject_digest": "sha256:subject-001",
    "source_commit": "0123456789abcdef",
    "provenance_digest": "sha256:review-001",
    "provider": "github",
    "key_id": "org/reviews-2026",
    "algorithm": "ed25519",
    "signature": "AAECAw",
    "issued_at": "2026-08-01T00:00:00Z",
    "expires_at": None,
    "sequence": 3,
    "state": "unverified",
}
if (
    attestation.get("subject_digest") != expected_values["subject_digest"]
    or attestation.get("source_commit") != expected_values["source_commit"]
    or attestation.get("provenance_digest") != expected_values["provenance_digest"]
    or attestation.get("provider") != expected_values["provider"]
    or attestation.get("key_id") != expected_values["key_id"]
    or attestation.get("algorithm") != expected_values["algorithm"]
    or attestation.get("signature") != expected_values["signature"]
    or attestation.get("issued_at") != expected_values["issued_at"]
    or attestation.get("expires_at") != expected_values["expires_at"]
    or attestation.get("sequence") != expected_values["sequence"]
    or attestation.get("state") != expected_values["state"]
):
    raise SystemExit(f"native source attestation without expiry fields are invalid: {attestation!r}")
def append_field(out, value):
    encoded = value.encode("utf-8")
    return out + len(encoded).to_bytes(8, "big") + encoded
canonical = b"lsharp.review-attestation.v1\0"
for value in (
    expected_values["review_id"],
    expected_values["subject_digest"],
    expected_values["source_commit"],
    expected_values["provenance_digest"],
    expected_values["provider"],
    expected_values["key_id"],
    expected_values["algorithm"],
    expected_values["issued_at"],
    "",
    str(expected_values["sequence"]),
):
    canonical = append_field(canonical, value)
if attestation.get("canonical_bytes") != list(canonical):
    raise SystemExit(f"native source attestation without expiry canonical bytes are invalid: {attestation!r}")
source = pathlib.Path(sys.argv[3]).read_text(encoding="utf-8")
expected_span = {"start": source.find(":review-attestation"), "end": source.rfind("\n  true")}
if attestation.get("span") != expected_span:
    raise SystemExit(f"native source attestation without expiry span is invalid: {attestation!r}")
reviews = json.loads(pathlib.Path(sys.argv[2]).read_text(encoding="utf-8")).get("reviews")
if not isinstance(reviews, list) or len(reviews) != 1:
    raise SystemExit(f"native source attestation without expiry reviews are invalid: {reviews!r}")
if reviews[0].get("verification_state") != "unverified":
    raise SystemExit(f"native source attestation without expiry state is invalid: {reviews[0]!r}")
PY

run_expected_failure validation-attestation-text 0 validate \
  --source "$VALIDATION_ATTESTATION_SOURCE" \
  --format text
require_exact_output validation-attestation-text $'status: unknown\nopen-questions: 0\nindependent-reviews: 0\ncontradicting-observations: 0\nstale-reviews: 0\nstale-evidence: 0\nreview-verification: review:checkout/reviewer-001=unverified\n'

run_expected_failure validation-attestation-no-expiry-text 0 validate \
  --source "$VALIDATION_ATTESTATION_NO_EXPIRY_SOURCE" \
  --format text
require_exact_output validation-attestation-no-expiry-text $'status: unknown\nopen-questions: 0\nindependent-reviews: 0\ncontradicting-observations: 0\nstale-reviews: 0\nstale-evidence: 0\nreview-verification: review:checkout/reviewer-001=unverified\n'

run_expected_failure validation-identity-json 0 validate \
  --source "$VALIDATION_IDENTITY_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_IDENTITY_MANIFEST" \
  --review-subject-digest "sha256:graph" \
  --review-source-commit "commit-1" \
  --review-artifact-digest "sha256:artifact" \
  --review-trust-store-digest "sha256:trust" \
  --review-lifecycle-digest "sha256:lifecycle" \
  --review-now "2026-08-15T00:00:00Z"
python3 - "$WORK_DIR/validation-identity-json.stdout" "$VALIDATION_IDENTITY_MANIFEST" <<'PY'
import json
import pathlib
import sys

expected = {
    "subject_digest": "sha256:graph",
    "source_commit": "commit-1",
    "artifact_digest": "sha256:artifact",
    "trust_store_digest": "sha256:trust",
    "lifecycle_digest": "sha256:lifecycle",
    "now": "2026-08-15T00:00:00Z",
}
report = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
identity = report.get("review_evidence_identity")
if identity != expected or list(identity) != list(expected):
    raise SystemExit(f"native review evidence identity report is invalid: {identity!r}")
manifest = json.loads(pathlib.Path(sys.argv[2]).read_text(encoding="utf-8"))
manifest_identity = manifest.get("review_evidence_identity")
if manifest_identity != expected or list(manifest_identity) != list(expected):
    raise SystemExit(f"native review evidence identity manifest is invalid: {manifest_identity!r}")
if manifest_identity != identity:
    raise SystemExit("native report and manifest review evidence identity differ")
PY

cp "$VALIDATION_IDENTITY_MANIFEST" "$VALIDATION_IDENTITY_REATTACH_MANIFEST"
cp "$VALIDATION_IDENTITY_MANIFEST" "$VALIDATION_IDENTITY_CONFLICT_MANIFEST"

run_expected_failure validation-identity-manifest-reattach-json 0 validate \
  "$VALIDATION_IDENTITY_REATTACH_MANIFEST" \
  --format json \
  --review-subject-digest "sha256:graph" \
  --review-source-commit "commit-1" \
  --review-artifact-digest "sha256:artifact" \
  --review-trust-store-digest "sha256:trust" \
  --review-lifecycle-digest "sha256:lifecycle" \
  --review-now "2026-08-15T00:00:00Z"
python3 - "$WORK_DIR/validation-identity-manifest-reattach-json.stdout" <<'PY'
import json
import pathlib
import sys

expected = {
    "subject_digest": "sha256:graph",
    "source_commit": "commit-1",
    "artifact_digest": "sha256:artifact",
    "trust_store_digest": "sha256:trust",
    "lifecycle_digest": "sha256:lifecycle",
    "now": "2026-08-15T00:00:00Z",
}
report = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
identity = report.get("review_evidence_identity")
if identity != expected or list(identity) != list(expected):
    raise SystemExit(f"manifest identity reattach report is invalid: {identity!r}")
PY

run_expected_validation_error validation-identity-manifest-conflict \
  validate \
  "$VALIDATION_IDENTITY_CONFLICT_MANIFEST" \
  --format json \
  --emit-manifest "$VALIDATION_IDENTITY_CONFLICT_OUTPUT_MANIFEST" \
  --review-subject-digest "sha256:graph" \
  --review-source-commit "commit-2" \
  --review-artifact-digest "sha256:artifact" \
  --review-trust-store-digest "sha256:trust" \
  --review-lifecycle-digest "sha256:lifecycle" \
  --review-now "2026-08-15T00:00:00Z"
grep -F "source validation error:14" \
  "$WORK_DIR/validation-identity-manifest-conflict.stderr" >/dev/null \
  || die "validation-identity-manifest-conflict must expose the stable identity conflict diagnostic"
[[ ! -s "$WORK_DIR/validation-identity-manifest-conflict.stdout" ]] \
  || die "validation-identity-manifest-conflict must produce no report or manifest"
[[ ! -e "$VALIDATION_IDENTITY_CONFLICT_OUTPUT_MANIFEST" ]] \
  || die "validation-identity-manifest-conflict must produce no report or manifest"

run_expected_failure validation-identity-optional-json 0 validate \
  --source "$VALIDATION_IDENTITY_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_IDENTITY_OPTIONAL_MANIFEST" \
  --review-subject-digest "sha256:graph" \
  --review-source-commit "commit-1" \
  --review-artifact-digest "sha256:artifact" \
  --review-now "2026-08-15T00:00:00Z"
python3 - "$WORK_DIR/validation-identity-optional-json.stdout" "$VALIDATION_IDENTITY_OPTIONAL_MANIFEST" <<'PY'
import json
import pathlib
import sys

expected = {
    "subject_digest": "sha256:graph",
    "source_commit": "commit-1",
    "artifact_digest": "sha256:artifact",
    "trust_store_digest": None,
    "lifecycle_digest": None,
    "now": "2026-08-15T00:00:00Z",
}
report = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
identity = report.get("review_evidence_identity")
if identity != expected or list(identity) != list(expected):
    raise SystemExit(f"native optional review identity report is invalid: {identity!r}")
manifest = json.loads(pathlib.Path(sys.argv[2]).read_text(encoding="utf-8"))
manifest_identity = manifest.get("review_evidence_identity")
if manifest_identity != expected or list(manifest_identity) != list(expected):
    raise SystemExit(f"native optional review identity manifest is invalid: {manifest_identity!r}")
PY

run_expected_failure validation-identity-text 0 validate \
  --source "$VALIDATION_IDENTITY_SOURCE" \
  --format text \
  --review-subject-digest "sha256:graph" \
  --review-source-commit "commit-1" \
  --review-artifact-digest "sha256:artifact" \
  --review-now "2026-08-15T00:00:00Z"
require_exact_output validation-identity-text $'status: unknown\nopen-questions: 0\nindependent-reviews: 0\ncontradicting-observations: 0\nstale-reviews: 0\nstale-evidence: 0\nreview-verification: review:checkout/reviewer-001=unverified\nreview-evidence-identity: subject=sha256:graph source=commit-1 artifact=sha256:artifact trust-store=- lifecycle=- now=2026-08-15T00:00:00Z\n'

run_expected_validation_error validation-identity-partial \
  validate \
  --source "$VALIDATION_IDENTITY_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_IDENTITY_PARTIAL_MANIFEST" \
  --review-subject-digest "sha256:graph"
grep -F "review identity requires --review-subject-digest --review-source-commit --review-artifact-digest --review-now" \
  "$WORK_DIR/validation-identity-partial.stderr" >/dev/null \
  || die "validation-identity-partial must expose the all-or-none identity boundary"
[[ ! -e "$VALIDATION_IDENTITY_PARTIAL_MANIFEST" ]] \
  || die "validation-identity-partial must not produce a manifest"

run_invalid_attestation() {
  local label="$1"
  local source="$2"
  local manifest="$3"

  run_expected_validation_error "$label" \
    validate \
    --source "$source" \
    --format json \
    --emit-manifest "$manifest"
  grep -F "source validation error:8" "$WORK_DIR/$label.stderr" >/dev/null \
    || die "$label must expose the invalid attestation error code"
  [[ ! -e "$manifest" ]] || die "$label must produce no report or manifest"
}

run_invalid_attestation \
  validation-invalid-attestation-algorithm \
  "$VALIDATION_INVALID_ATTESTATION_ALGORITHM_SOURCE" \
  "$VALIDATION_INVALID_ATTESTATION_ALGORITHM_MANIFEST"
run_invalid_attestation \
  validation-invalid-attestation-signature \
  "$VALIDATION_INVALID_ATTESTATION_SIGNATURE_SOURCE" \
  "$VALIDATION_INVALID_ATTESTATION_SIGNATURE_MANIFEST"
run_invalid_attestation \
  validation-invalid-attestation-timestamp \
  "$VALIDATION_INVALID_ATTESTATION_TIMESTAMP_SOURCE" \
  "$VALIDATION_INVALID_ATTESTATION_TIMESTAMP_MANIFEST"
run_invalid_attestation \
  validation-invalid-attestation-window \
  "$VALIDATION_INVALID_ATTESTATION_WINDOW_SOURCE" \
  "$VALIDATION_INVALID_ATTESTATION_WINDOW_MANIFEST"

run_expected_validation_error validation-malformed-edge \
  validate \
  --source "$VALIDATION_MALFORMED_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_MALFORMED_MANIFEST"
grep -F "source validation error:1" "$WORK_DIR/validation-malformed-edge.stderr" >/dev/null \
  || die "malformed edge validation must expose the malformed-source error code"
[[ ! -e "$VALIDATION_MALFORMED_MANIFEST" ]] \
  || die "malformed edge validation must produce no report or manifest"

run_expected_validation_error validation-invalid-id \
  validate \
  --source "$VALIDATION_INVALID_ID_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_INVALID_ID_MANIFEST"
grep -F "source validation error:2" "$WORK_DIR/validation-invalid-id.stderr" >/dev/null \
  || die "invalid ID validation must expose the invalid-wire error code"
[[ ! -e "$VALIDATION_INVALID_ID_MANIFEST" ]] \
  || die "invalid ID validation must produce no report or manifest"

run_expected_validation_error validation-kind-mismatch \
  validate \
  --source "$VALIDATION_KIND_MISMATCH_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_KIND_MISMATCH_MANIFEST"
grep -F "source validation error:3" "$WORK_DIR/validation-kind-mismatch.stderr" >/dev/null \
  || die "kind mismatch validation must expose the node-kind error code"
[[ ! -e "$VALIDATION_KIND_MISMATCH_MANIFEST" ]] \
  || die "kind mismatch validation must produce no report or manifest"

run_expected_validation_error validation-evidence-registry \
  validate \
  --source "$VALIDATION_EVIDENCE_REGISTRY_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_EVIDENCE_REGISTRY_MANIFEST"
grep -F "source validation error:6" "$WORK_DIR/validation-evidence-registry.stderr" >/dev/null \
  || die "unregistered evidence validation must expose the registry-required error code"
[[ ! -e "$VALIDATION_EVIDENCE_REGISTRY_MANIFEST" ]] \
  || die "unregistered evidence validation must produce no report or manifest"

run_expected_validation_error validation-supports-evidence-precedence \
  validate \
  --source "$VALIDATION_SUPPORTS_EVIDENCE_PRECEDENCE_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_SUPPORTS_EVIDENCE_PRECEDENCE_MANIFEST"
grep -F "source validation error:6" "$WORK_DIR/validation-supports-evidence-precedence.stderr" >/dev/null \
  || die "unregistered supports evidence must win over its invalid wire ID"
[[ ! -e "$VALIDATION_SUPPORTS_EVIDENCE_PRECEDENCE_MANIFEST" ]] \
  || die "supports evidence precedence validation must produce no report or manifest"

run_expected_validation_error validation-contradicts-evidence-precedence \
  validate \
  --source "$VALIDATION_CONTRADICTS_EVIDENCE_PRECEDENCE_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_CONTRADICTS_EVIDENCE_PRECEDENCE_MANIFEST"
grep -F "source validation error:6" "$WORK_DIR/validation-contradicts-evidence-precedence.stderr" >/dev/null \
  || die "unregistered contradicts evidence must win over its invalid wire ID"
[[ ! -e "$VALIDATION_CONTRADICTS_EVIDENCE_PRECEDENCE_MANIFEST" ]] \
  || die "contradicts evidence precedence validation must produce no report or manifest"

run_expected_validation_error validation-malformed-evidence \
  validate \
  --source "$VALIDATION_MALFORMED_EVIDENCE_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_MALFORMED_EVIDENCE_MANIFEST"
grep -F "source validation error:1" "$WORK_DIR/validation-malformed-evidence.stderr" >/dev/null \
  || die "malformed evidence validation must expose the malformed error code"
[[ ! -e "$VALIDATION_MALFORMED_EVIDENCE_MANIFEST" ]] \
  || die "malformed evidence validation must produce no report or manifest"

run_expected_validation_error validation-duplicate-evidence-coverage \
  validate \
  --source "$VALIDATION_DUPLICATE_EVIDENCE_COVERAGE_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_DUPLICATE_EVIDENCE_COVERAGE_MANIFEST"
grep -F "source validation error:1" "$WORK_DIR/validation-duplicate-evidence-coverage.stderr" >/dev/null \
  || die "duplicate evidence coverage validation must expose the parser error code"
[[ ! -e "$VALIDATION_DUPLICATE_EVIDENCE_COVERAGE_MANIFEST" ]] \
  || die "duplicate evidence coverage validation must produce no report or manifest"

run_expected_validation_error validation-invalid-evidence \
  validate \
  --source "$VALIDATION_INVALID_EVIDENCE_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_INVALID_EVIDENCE_MANIFEST"
grep -F "source validation error:8" "$WORK_DIR/validation-invalid-evidence.stderr" >/dev/null \
  || die "invalid evidence validation must expose the invalid-field error code"
[[ ! -e "$VALIDATION_INVALID_EVIDENCE_MANIFEST" ]] \
  || die "invalid evidence validation must produce no report or manifest"

run_expected_validation_error validation-invalid-evidence-outcome \
  validate \
  --source "$VALIDATION_INVALID_EVIDENCE_OUTCOME_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_INVALID_EVIDENCE_OUTCOME_MANIFEST"
grep -F "source validation error:8" "$WORK_DIR/validation-invalid-evidence-outcome.stderr" >/dev/null \
  || die "invalid evidence outcome validation must expose the invalid-field error code"
[[ ! -e "$VALIDATION_INVALID_EVIDENCE_OUTCOME_MANIFEST" ]] \
  || die "invalid evidence outcome validation must produce no report or manifest"

run_expected_validation_error validation-invalid-evidence-independence \
  validate \
  --source "$VALIDATION_INVALID_EVIDENCE_INDEPENDENCE_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_INVALID_EVIDENCE_INDEPENDENCE_MANIFEST"
grep -F "source validation error:8" "$WORK_DIR/validation-invalid-evidence-independence.stderr" >/dev/null \
  || die "invalid evidence independence validation must expose the invalid-field error code"
[[ ! -e "$VALIDATION_INVALID_EVIDENCE_INDEPENDENCE_MANIFEST" ]] \
  || die "invalid evidence independence validation must produce no report or manifest"

run_expected_validation_error validation-invalid-evidence-subject \
  validate \
  --source "$VALIDATION_INVALID_EVIDENCE_SUBJECT_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_INVALID_EVIDENCE_SUBJECT_MANIFEST"
grep -F "source validation error:8" "$WORK_DIR/validation-invalid-evidence-subject.stderr" >/dev/null \
  || die "invalid evidence subject validation must expose the invalid-field error code"
[[ ! -e "$VALIDATION_INVALID_EVIDENCE_SUBJECT_MANIFEST" ]] \
  || die "invalid evidence subject validation must produce no report or manifest"

run_expected_validation_error validation-empty-evidence-method \
  validate \
  --source "$VALIDATION_EMPTY_EVIDENCE_METHOD_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_EMPTY_EVIDENCE_METHOD_MANIFEST"
grep -F "source validation error:8" "$WORK_DIR/validation-empty-evidence-method.stderr" >/dev/null \
  || die "empty evidence method validation must expose the invalid-field error code"
[[ ! -e "$VALIDATION_EMPTY_EVIDENCE_METHOD_MANIFEST" ]] \
  || die "empty evidence method validation must produce no report or manifest"

run_expected_validation_error validation-empty-evidence-outcome \
  validate \
  --source "$VALIDATION_EMPTY_EVIDENCE_OUTCOME_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_EMPTY_EVIDENCE_OUTCOME_MANIFEST"
grep -F "source validation error:8" "$WORK_DIR/validation-empty-evidence-outcome.stderr" >/dev/null \
  || die "empty evidence outcome validation must expose the invalid-field error code"
[[ ! -e "$VALIDATION_EMPTY_EVIDENCE_OUTCOME_MANIFEST" ]] \
  || die "empty evidence outcome validation must produce no report or manifest"

run_expected_validation_error validation-empty-evidence-independence \
  validate \
  --source "$VALIDATION_EMPTY_EVIDENCE_INDEPENDENCE_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_EMPTY_EVIDENCE_INDEPENDENCE_MANIFEST"
grep -F "source validation error:8" "$WORK_DIR/validation-empty-evidence-independence.stderr" >/dev/null \
  || die "empty evidence independence validation must expose the invalid-field error code"
[[ ! -e "$VALIDATION_EMPTY_EVIDENCE_INDEPENDENCE_MANIFEST" ]] \
  || die "empty evidence independence validation must produce no report or manifest"

run_expected_validation_error validation-evidence-required-precedence \
  validate \
  --source "$VALIDATION_EVIDENCE_REQUIRED_PRECEDENCE_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_EVIDENCE_REQUIRED_PRECEDENCE_MANIFEST"
grep -F "source validation error:4" "$WORK_DIR/validation-evidence-required-precedence.stderr" >/dev/null \
  || die "empty runner must win over invalid evidence ID with the required-field error code"
[[ ! -e "$VALIDATION_EVIDENCE_REQUIRED_PRECEDENCE_MANIFEST" ]] \
  || die "evidence required-field precedence validation must produce no report or manifest"

run_expected_validation_error validation-duplicate-evidence \
  validate \
  --source "$VALIDATION_DUPLICATE_EVIDENCE_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_DUPLICATE_EVIDENCE_MANIFEST"
grep -F "source validation error:3" "$WORK_DIR/validation-duplicate-evidence.stderr" >/dev/null \
  || die "duplicate evidence validation must expose the duplicate-ID error code"
[[ ! -e "$VALIDATION_DUPLICATE_EVIDENCE_MANIFEST" ]] \
  || die "duplicate evidence validation must produce no report or manifest"

run_expected_validation_error validation-duplicate-evidence-field \
  validate \
  --source "$VALIDATION_DUPLICATE_EVIDENCE_FIELD_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_DUPLICATE_EVIDENCE_FIELD_MANIFEST"
grep -F "source validation error:1" "$WORK_DIR/validation-duplicate-evidence-field.stderr" >/dev/null \
  || die "duplicate evidence field validation must expose the malformed parser error code"
[[ ! -e "$VALIDATION_DUPLICATE_EVIDENCE_FIELD_MANIFEST" ]] \
  || die "duplicate evidence field validation must produce no report or manifest"

run_expected_validation_error validation-empty-evidence-generator \
  validate \
  --source "$VALIDATION_EMPTY_EVIDENCE_GENERATOR_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_EMPTY_EVIDENCE_GENERATOR_MANIFEST"
grep -F "source validation error:4" "$WORK_DIR/validation-empty-evidence-generator.stderr" >/dev/null \
  || die "empty evidence generator validation must expose the required-field error code"
[[ ! -e "$VALIDATION_EMPTY_EVIDENCE_GENERATOR_MANIFEST" ]] \
  || die "empty evidence generator validation must produce no report or manifest"

run_expected_validation_error validation-whitespace-evidence-coverage \
  validate \
  --source "$VALIDATION_WHITESPACE_EVIDENCE_COVERAGE_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_WHITESPACE_EVIDENCE_COVERAGE_MANIFEST"
grep -F "source validation error:4" "$WORK_DIR/validation-whitespace-evidence-coverage.stderr" >/dev/null \
  || die "whitespace evidence coverage validation must expose the empty-field error code"
[[ ! -e "$VALIDATION_WHITESPACE_EVIDENCE_COVERAGE_MANIFEST" ]] \
  || die "whitespace evidence coverage validation must produce no report or manifest"

run_expected_validation_error validation-unicode-whitespace-evidence-coverage \
  validate \
  --source "$VALIDATION_UNICODE_WHITESPACE_EVIDENCE_COVERAGE_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_UNICODE_WHITESPACE_EVIDENCE_COVERAGE_MANIFEST"
grep -F "source validation error:4" "$WORK_DIR/validation-unicode-whitespace-evidence-coverage.stderr" >/dev/null \
  || die "Unicode whitespace evidence coverage validation must expose the empty-field error code"
[[ ! -e "$VALIDATION_UNICODE_WHITESPACE_EVIDENCE_COVERAGE_MANIFEST" ]] \
  || die "Unicode whitespace evidence coverage validation must produce no report or manifest"

run_expected_validation_error validation-negative-evidence-coverage \
  validate \
  --source "$VALIDATION_NEGATIVE_EVIDENCE_COVERAGE_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_NEGATIVE_EVIDENCE_COVERAGE_MANIFEST"
grep -F "source validation error:11" "$WORK_DIR/validation-negative-evidence-coverage.stderr" >/dev/null \
  || die "negative evidence coverage count validation must expose the invalid-sampling error code"
[[ ! -e "$VALIDATION_NEGATIVE_EVIDENCE_COVERAGE_MANIFEST" ]] \
  || die "negative evidence coverage count validation must produce no report or manifest"

run_expected_validation_error validation-negative-evidence-cases \
  validate \
  --source "$VALIDATION_NEGATIVE_EVIDENCE_CASES_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_NEGATIVE_EVIDENCE_CASES_MANIFEST"
grep -F "source validation error:11" "$WORK_DIR/validation-negative-evidence-cases.stderr" >/dev/null \
  || die "negative evidence cases validation must expose the invalid-sampling error code"
[[ ! -e "$VALIDATION_NEGATIVE_EVIDENCE_CASES_MANIFEST" ]] \
  || die "negative evidence cases validation must produce no report or manifest"

run_expected_validation_error validation-negative-evidence-seed \
  validate \
  --source "$VALIDATION_NEGATIVE_EVIDENCE_SEED_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_NEGATIVE_EVIDENCE_SEED_MANIFEST"
grep -F "source validation error:11" "$WORK_DIR/validation-negative-evidence-seed.stderr" >/dev/null \
  || die "negative evidence seed validation must expose the invalid-sampling error code"
[[ ! -e "$VALIDATION_NEGATIVE_EVIDENCE_SEED_MANIFEST" ]] \
  || die "negative evidence seed validation must produce no report or manifest"

run_expected_validation_error validation-negative-evidence-shrinks \
  validate \
  --source "$VALIDATION_NEGATIVE_EVIDENCE_SHRINKS_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_NEGATIVE_EVIDENCE_SHRINKS_MANIFEST"
grep -F "source validation error:11" "$WORK_DIR/validation-negative-evidence-shrinks.stderr" >/dev/null \
  || die "negative evidence shrinks validation must expose the invalid-sampling error code"
[[ ! -e "$VALIDATION_NEGATIVE_EVIDENCE_SHRINKS_MANIFEST" ]] \
  || die "negative evidence shrinks validation must produce no report or manifest"

run_expected_validation_error validation-empty-evidence-runner \
  validate \
  --source "$VALIDATION_EMPTY_EVIDENCE_RUNNER_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_EMPTY_EVIDENCE_RUNNER_MANIFEST"
grep -F "source validation error:4" "$WORK_DIR/validation-empty-evidence-runner.stderr" >/dev/null \
  || die "empty evidence runner validation must expose the required-field error code"
[[ ! -e "$VALIDATION_EMPTY_EVIDENCE_RUNNER_MANIFEST" ]] \
  || die "empty evidence runner validation must produce no report or manifest"

run_expected_validation_error validation-empty-evidence-target \
  validate \
  --source "$VALIDATION_EMPTY_EVIDENCE_TARGET_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_EMPTY_EVIDENCE_TARGET_MANIFEST"
grep -F "source validation error:4" "$WORK_DIR/validation-empty-evidence-target.stderr" >/dev/null \
  || die "empty evidence target validation must expose the required-field error code"
[[ ! -e "$VALIDATION_EMPTY_EVIDENCE_TARGET_MANIFEST" ]] \
  || die "empty evidence target validation must produce no report or manifest"

run_expected_validation_error validation-empty-evidence-source-commit \
  validate \
  --source "$VALIDATION_EMPTY_EVIDENCE_SOURCE_COMMIT_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_EMPTY_EVIDENCE_SOURCE_COMMIT_MANIFEST"
grep -F "source validation error:4" "$WORK_DIR/validation-empty-evidence-source-commit.stderr" >/dev/null \
  || die "empty evidence source commit validation must expose the required-field error code"
[[ ! -e "$VALIDATION_EMPTY_EVIDENCE_SOURCE_COMMIT_MANIFEST" ]] \
  || die "empty evidence source commit validation must produce no report or manifest"

run_expected_validation_error validation-empty-evidence-artifact-digest \
  validate \
  --source "$VALIDATION_EMPTY_EVIDENCE_ARTIFACT_DIGEST_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_EMPTY_EVIDENCE_ARTIFACT_DIGEST_MANIFEST"
grep -F "source validation error:4" "$WORK_DIR/validation-empty-evidence-artifact-digest.stderr" >/dev/null \
  || die "empty evidence artifact digest validation must expose the required-field error code"
[[ ! -e "$VALIDATION_EMPTY_EVIDENCE_ARTIFACT_DIGEST_MANIFEST" ]] \
  || die "empty evidence artifact digest validation must produce no report or manifest"

run_expected_validation_error validation-empty-evidence-producer \
  validate \
  --source "$VALIDATION_EMPTY_EVIDENCE_PRODUCER_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_EMPTY_EVIDENCE_PRODUCER_MANIFEST"
grep -F "source validation error:4" "$WORK_DIR/validation-empty-evidence-producer.stderr" >/dev/null \
  || die "empty evidence producer validation must expose the required-field error code"
[[ ! -e "$VALIDATION_EMPTY_EVIDENCE_PRODUCER_MANIFEST" ]] \
  || die "empty evidence producer validation must produce no report or manifest"

run_expected_validation_error validation-empty-evidence-tool-version \
  validate \
  --source "$VALIDATION_EMPTY_EVIDENCE_TOOL_VERSION_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_EMPTY_EVIDENCE_TOOL_VERSION_MANIFEST"
grep -F "source validation error:4" "$WORK_DIR/validation-empty-evidence-tool-version.stderr" >/dev/null \
  || die "empty evidence tool version validation must expose the required-field error code"
[[ ! -e "$VALIDATION_EMPTY_EVIDENCE_TOOL_VERSION_MANIFEST" ]] \
  || die "empty evidence tool version validation must produce no report or manifest"

run_expected_validation_error validation-empty-evidence-timestamp \
  validate \
  --source "$VALIDATION_EMPTY_EVIDENCE_TIMESTAMP_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_EMPTY_EVIDENCE_TIMESTAMP_MANIFEST"
grep -F "source validation error:4" "$WORK_DIR/validation-empty-evidence-timestamp.stderr" >/dev/null \
  || die "empty evidence timestamp validation must expose the required-field error code"
[[ ! -e "$VALIDATION_EMPTY_EVIDENCE_TIMESTAMP_MANIFEST" ]] \
  || die "empty evidence timestamp validation must produce no report or manifest"

run_expected_validation_error validation-whitespace-evidence-runner \
  validate \
  --source "$VALIDATION_WHITESPACE_EVIDENCE_RUNNER_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_WHITESPACE_EVIDENCE_RUNNER_MANIFEST"
grep -F "source validation error:4" "$WORK_DIR/validation-whitespace-evidence-runner.stderr" >/dev/null \
  || die "whitespace evidence runner validation must expose the required-field error code"
[[ ! -e "$VALIDATION_WHITESPACE_EVIDENCE_RUNNER_MANIFEST" ]] \
  || die "whitespace evidence runner validation must produce no report or manifest"

run_expected_validation_error validation-unicode-whitespace-evidence-runner \
  validate \
  --source "$VALIDATION_UNICODE_WHITESPACE_EVIDENCE_RUNNER_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_UNICODE_WHITESPACE_EVIDENCE_RUNNER_MANIFEST"
grep -F "source validation error:4" "$WORK_DIR/validation-unicode-whitespace-evidence-runner.stderr" >/dev/null \
  || die "Unicode whitespace evidence runner validation must expose the required-field error code"
[[ ! -e "$VALIDATION_UNICODE_WHITESPACE_EVIDENCE_RUNNER_MANIFEST" ]] \
  || die "Unicode whitespace evidence runner validation must produce no report or manifest"

run_expected_validation_error validation-whitespace-evidence-subject \
  validate \
  --source "$VALIDATION_WHITESPACE_EVIDENCE_SUBJECT_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_WHITESPACE_EVIDENCE_SUBJECT_MANIFEST"
grep -F "source validation error:2" "$WORK_DIR/validation-whitespace-evidence-subject.stderr" >/dev/null \
  || die "whitespace evidence subject validation must expose the invalid-id error code"
[[ ! -e "$VALIDATION_WHITESPACE_EVIDENCE_SUBJECT_MANIFEST" ]] \
  || die "whitespace evidence subject validation must produce no report or manifest"

run_expected_validation_error validation-whitespace-evidence-id \
  validate \
  --source "$VALIDATION_WHITESPACE_EVIDENCE_ID_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_WHITESPACE_EVIDENCE_ID_MANIFEST"
grep -F "source validation error:2" "$WORK_DIR/validation-whitespace-evidence-id.stderr" >/dev/null \
  || die "whitespace evidence ID validation must expose the invalid-id error code"
[[ ! -e "$VALIDATION_WHITESPACE_EVIDENCE_ID_MANIFEST" ]] \
  || die "whitespace evidence ID validation must produce no report or manifest"

run_expected_validation_error validation-empty-evidence-id \
  validate \
  --source "$VALIDATION_EMPTY_EVIDENCE_ID_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_EMPTY_EVIDENCE_ID_MANIFEST"
grep -F "source validation error:2" "$WORK_DIR/validation-empty-evidence-id.stderr" >/dev/null \
  || die "empty evidence ID validation must expose the invalid-id error code"
[[ ! -e "$VALIDATION_EMPTY_EVIDENCE_ID_MANIFEST" ]] \
  || die "empty evidence ID validation must produce no report or manifest"

run_expected_validation_error validation-missing-node-id \
  validate \
  --source "$VALIDATION_MISSING_NODE_ID_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_MISSING_NODE_ID_MANIFEST"
grep -F "source validation error:1" "$WORK_DIR/validation-missing-node-id.stderr" >/dev/null \
  || die "missing node ID validation must expose the malformed parser error code"
[[ ! -e "$VALIDATION_MISSING_NODE_ID_MANIFEST" ]] \
  || die "missing node ID validation must produce no report or manifest"

run_expected_validation_error validation-missing-node-text \
  validate \
  --source "$VALIDATION_MISSING_NODE_TEXT_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_MISSING_NODE_TEXT_MANIFEST"
grep -F "source validation error:1" "$WORK_DIR/validation-missing-node-text.stderr" >/dev/null \
  || die "missing node text validation must expose the malformed parser error code"
[[ ! -e "$VALIDATION_MISSING_NODE_TEXT_MANIFEST" ]] \
  || die "missing node text validation must produce no report or manifest"

run_expected_validation_error validation-whitespace-node-text \
  validate \
  --source "$VALIDATION_WHITESPACE_NODE_TEXT_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_WHITESPACE_NODE_TEXT_MANIFEST"
grep -F "source validation error:1" "$WORK_DIR/validation-whitespace-node-text.stderr" >/dev/null \
  || die "whitespace node text validation must expose the malformed parser error code"
[[ ! -e "$VALIDATION_WHITESPACE_NODE_TEXT_MANIFEST" ]] \
  || die "whitespace node text validation must produce no report or manifest"

run_expected_validation_error validation-unicode-whitespace-node-text \
  validate \
  --source "$VALIDATION_UNICODE_WHITESPACE_NODE_TEXT_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_UNICODE_WHITESPACE_NODE_TEXT_MANIFEST"
grep -F "source validation error:1" "$WORK_DIR/validation-unicode-whitespace-node-text.stderr" >/dev/null \
  || die "Unicode whitespace node text validation must expose the malformed parser error code"
[[ ! -e "$VALIDATION_UNICODE_WHITESPACE_NODE_TEXT_MANIFEST" ]] \
  || die "Unicode whitespace node text validation must produce no report or manifest"

run_expected_validation_error validation-node-text-precedence \
  validate \
  --source "$VALIDATION_NODE_TEXT_PRECEDENCE_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_NODE_TEXT_PRECEDENCE_MANIFEST"
grep -F "source validation error:1" "$WORK_DIR/validation-node-text-precedence.stderr" >/dev/null \
  || die "whitespace node text must win over invalid stable ID with the malformed error code"
[[ ! -e "$VALIDATION_NODE_TEXT_PRECEDENCE_MANIFEST" ]] \
  || die "node text precedence validation must produce no report or manifest"

run_expected_validation_error validation-missing-review \
  validate \
  --source "$VALIDATION_MISSING_REVIEW_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_MISSING_REVIEW_MANIFEST"
grep -F "source validation error:10" "$WORK_DIR/validation-missing-review.stderr" >/dev/null \
  || die "missing review validation must expose the missing-review error code"
[[ ! -e "$VALIDATION_MISSING_REVIEW_MANIFEST" ]] \
  || die "missing review validation must produce no report or manifest"

run_expected_validation_error validation-missing-review-before-subject-kind \
  validate \
  --source "$VALIDATION_MISSING_REVIEW_SUBJECT_KIND_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_MISSING_REVIEW_SUBJECT_KIND_MANIFEST"
grep -F "source validation error:10" "$WORK_DIR/validation-missing-review-before-subject-kind.stderr" >/dev/null \
  || die "missing review must win over invalid evaluates subject kind"
[[ ! -e "$VALIDATION_MISSING_REVIEW_SUBJECT_KIND_MANIFEST" ]] \
  || die "missing review subject precedence validation must produce no report or manifest"

run_expected_validation_error validation-duplicate-review \
  validate \
  --source "$VALIDATION_DUPLICATE_REVIEW_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_DUPLICATE_REVIEW_MANIFEST"
grep -F "source validation error:7" "$WORK_DIR/validation-duplicate-review.stderr" >/dev/null \
  || die "duplicate review validation must expose the duplicate-review error code"
[[ ! -e "$VALIDATION_DUPLICATE_REVIEW_MANIFEST" ]] \
  || die "duplicate review validation must produce no report or manifest"

run_expected_validation_error validation-invalid-review \
  validate \
  --source "$VALIDATION_INVALID_REVIEW_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_INVALID_REVIEW_MANIFEST"
grep -F "source validation error:8" "$WORK_DIR/validation-invalid-review.stderr" >/dev/null \
  || die "invalid review validation must expose the invalid-review error code"
[[ ! -e "$VALIDATION_INVALID_REVIEW_MANIFEST" ]] \
  || die "invalid review validation must produce no report or manifest"

run_expected_validation_error validation-invalid-review-digest \
  validate \
  --source "$VALIDATION_INVALID_REVIEW_DIGEST_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_INVALID_REVIEW_DIGEST_MANIFEST"
grep -F "source validation error:8" "$WORK_DIR/validation-invalid-review-digest.stderr" >/dev/null \
  || die "blank review digest validation must expose the invalid-review error code"
[[ ! -e "$VALIDATION_INVALID_REVIEW_DIGEST_MANIFEST" ]] \
  || die "blank review digest validation must produce no report or manifest"

run_expected_validation_error validation-unicode-whitespace-review-digest \
  validate \
  --source "$VALIDATION_UNICODE_WHITESPACE_REVIEW_DIGEST_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_UNICODE_WHITESPACE_REVIEW_DIGEST_MANIFEST"
grep -F "source validation error:8" "$WORK_DIR/validation-unicode-whitespace-review-digest.stderr" >/dev/null \
  || die "Unicode whitespace review digest validation must expose the invalid-review error code"
[[ ! -e "$VALIDATION_UNICODE_WHITESPACE_REVIEW_DIGEST_MANIFEST" ]] \
  || die "Unicode whitespace review digest validation must produce no report or manifest"

run_expected_validation_error validation-review-required-precedence \
  validate \
  --source "$VALIDATION_REVIEW_REQUIRED_PRECEDENCE_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_REVIEW_REQUIRED_PRECEDENCE_MANIFEST"
grep -F "source validation error:8" "$WORK_DIR/validation-review-required-precedence.stderr" >/dev/null \
  || die "blank review digest must win over invalid review ID with the invalid-review error code"
[[ ! -e "$VALIDATION_REVIEW_REQUIRED_PRECEDENCE_MANIFEST" ]] \
  || die "review required-field precedence validation must produce no report or manifest"

run_expected_validation_error validation-invalid-review-id \
  validate \
  --source "$VALIDATION_INVALID_REVIEW_ID_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_INVALID_REVIEW_ID_MANIFEST"
grep -F "source validation error:2" "$WORK_DIR/validation-invalid-review-id.stderr" >/dev/null \
  || die "invalid review ID validation must expose the invalid-ID error code"
[[ ! -e "$VALIDATION_INVALID_REVIEW_ID_MANIFEST" ]] \
  || die "invalid review ID validation must produce no report or manifest"

run_expected_validation_error validation-empty-review-id \
  validate \
  --source "$VALIDATION_EMPTY_REVIEW_ID_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_EMPTY_REVIEW_ID_MANIFEST"
grep -F "source validation error:8" "$WORK_DIR/validation-empty-review-id.stderr" >/dev/null \
  || die "empty review ID validation must expose the invalid-review error code"
[[ ! -e "$VALIDATION_EMPTY_REVIEW_ID_MANIFEST" ]] \
  || die "empty review ID validation must produce no report or manifest"

run_expected_validation_error validation-malformed-review \
  validate \
  --source "$VALIDATION_MALFORMED_REVIEW_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_MALFORMED_REVIEW_MANIFEST"
grep -F "source validation error:1" "$WORK_DIR/validation-malformed-review.stderr" >/dev/null \
  || die "malformed review validation must expose the malformed error code"
[[ ! -e "$VALIDATION_MALFORMED_REVIEW_MANIFEST" ]] \
  || die "malformed review validation must produce no report or manifest"

run_expected_validation_error validation-malformed-review-extra \
  validate \
  --source "$VALIDATION_MALFORMED_REVIEW_EXTRA_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_MALFORMED_REVIEW_EXTRA_MANIFEST"
grep -F "source validation error:1" "$WORK_DIR/validation-malformed-review-extra.stderr" >/dev/null \
  || die "malformed review extra validation must expose the malformed error code"
[[ ! -e "$VALIDATION_MALFORMED_REVIEW_EXTRA_MANIFEST" ]] \
  || die "malformed review extra validation must produce no report or manifest"

run_expected_validation_error validation-malformed-review-edge \
  validate \
  --source "$VALIDATION_MALFORMED_REVIEW_EDGE_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_MALFORMED_REVIEW_EDGE_MANIFEST"
grep -F "source validation error:1" "$WORK_DIR/validation-malformed-review-edge.stderr" >/dev/null \
  || die "malformed review edge validation must expose the malformed error code"
[[ ! -e "$VALIDATION_MALFORMED_REVIEW_EDGE_MANIFEST" ]] \
  || die "malformed review edge validation must produce no report or manifest"

run_expected_validation_error validation-malformed-invalidation-edge \
  validate \
  --source "$VALIDATION_MALFORMED_INVALIDATION_EDGE_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_MALFORMED_INVALIDATION_EDGE_MANIFEST"
grep -F "source validation error:1" "$WORK_DIR/validation-malformed-invalidation-edge.stderr" >/dev/null \
  || die "malformed invalidation edge validation must expose the malformed error code"
[[ ! -e "$VALIDATION_MALFORMED_INVALIDATION_EDGE_MANIFEST" ]] \
  || die "malformed invalidation edge validation must produce no report or manifest"

run_expected_validation_error validation-malformed-review-edge-extra \
  validate \
  --source "$VALIDATION_MALFORMED_REVIEW_EDGE_EXTRA_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_MALFORMED_REVIEW_EDGE_EXTRA_MANIFEST"
grep -F "source validation error:1" "$WORK_DIR/validation-malformed-review-edge-extra.stderr" >/dev/null \
  || die "malformed review edge extra validation must expose the malformed error code"
[[ ! -e "$VALIDATION_MALFORMED_REVIEW_EDGE_EXTRA_MANIFEST" ]] \
  || die "malformed review edge extra validation must produce no report or manifest"

run_expected_validation_error validation-malformed-invalidation-edge-extra \
  validate \
  --source "$VALIDATION_MALFORMED_INVALIDATION_EDGE_EXTRA_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_MALFORMED_INVALIDATION_EDGE_EXTRA_MANIFEST"
grep -F "source validation error:1" "$WORK_DIR/validation-malformed-invalidation-edge-extra.stderr" >/dev/null \
  || die "malformed invalidation edge extra validation must expose the malformed error code"
[[ ! -e "$VALIDATION_MALFORMED_INVALIDATION_EDGE_EXTRA_MANIFEST" ]] \
  || die "malformed invalidation edge extra validation must produce no report or manifest"

run_expected_validation_error validation-review-subject-kind \
  validate \
  --source "$VALIDATION_REVIEW_SUBJECT_KIND_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_REVIEW_SUBJECT_KIND_MANIFEST"
grep -F "source validation error:9" "$WORK_DIR/validation-review-subject-kind.stderr" >/dev/null \
  || die "review subject kind validation must expose the subject-kind error code"
[[ ! -e "$VALIDATION_REVIEW_SUBJECT_KIND_MANIFEST" ]] \
  || die "review subject kind validation must produce no report or manifest"

run_expected_validation_error validation-invalidation-subject-kind \
  validate \
  --source "$VALIDATION_INVALIDATION_SUBJECT_KIND_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_INVALIDATION_SUBJECT_KIND_MANIFEST"
grep -F "source validation error:9" "$WORK_DIR/validation-invalidation-subject-kind.stderr" >/dev/null \
  || die "invalidation subject kind validation must expose the subject-kind error code"
[[ ! -e "$VALIDATION_INVALIDATION_SUBJECT_KIND_MANIFEST" ]] \
  || die "invalidation subject kind validation must produce no report or manifest"

run_expected_validation_error validation-invalidation-missing-review \
  validate \
  --source "$VALIDATION_INVALIDATION_MISSING_REVIEW_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_INVALIDATION_MISSING_REVIEW_MANIFEST"
grep -F "source validation error:10" "$WORK_DIR/validation-invalidation-missing-review.stderr" >/dev/null \
  || die "invalidation missing review validation must expose the missing-review error code"
[[ ! -e "$VALIDATION_INVALIDATION_MISSING_REVIEW_MANIFEST" ]] \
  || die "invalidation missing review validation must produce no report or manifest"

run_expected_validation_error validation-review-edge-evidence \
  validate \
  --source "$VALIDATION_REVIEW_EDGE_EVIDENCE_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_REVIEW_EDGE_EVIDENCE_MANIFEST"
grep -F "source validation error:6" "$WORK_DIR/validation-review-edge-evidence.stderr" >/dev/null \
  || die "review edge evidence validation must expose the registry-required error code"
[[ ! -e "$VALIDATION_REVIEW_EDGE_EVIDENCE_MANIFEST" ]] \
  || die "review edge evidence validation must produce no report or manifest"

run_expected_validation_error validation-invalidation-edge-evidence \
  validate \
  --source "$VALIDATION_INVALIDATION_EDGE_EVIDENCE_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_INVALIDATION_EDGE_EVIDENCE_MANIFEST"
grep -F "source validation error:6" "$WORK_DIR/validation-invalidation-edge-evidence.stderr" >/dev/null \
  || die "invalidation edge evidence validation must expose the registry-required error code"
[[ ! -e "$VALIDATION_INVALIDATION_EDGE_EVIDENCE_MANIFEST" ]] \
  || die "invalidation edge evidence validation must produce no report or manifest"

run_expected_validation_error validation-orphan \
  validate \
  --source "$VALIDATION_ORPHAN_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_ORPHAN_MANIFEST"
grep -F "source validation error:5" "$WORK_DIR/validation-orphan.stderr" >/dev/null \
  || die "orphan edge validation must expose the missing-node error code"
[[ ! -e "$VALIDATION_ORPHAN_MANIFEST" ]] \
  || die "orphan edge validation must produce no report or manifest"

run_expected_validation_error validation-duplicate-node \
  validate \
  --source "$VALIDATION_INVALID_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_INVALID_MANIFEST"
grep -F "source validation error" "$WORK_DIR/validation-duplicate-node.stderr" >/dev/null \
  || die "duplicate validation must expose a source validation error diagnostic"
grep -F "source validation error:2" "$WORK_DIR/validation-duplicate-node.stderr" >/dev/null \
  || die "duplicate validation diagnostic must expose the canonical duplicate-node code"
[[ ! -e "$VALIDATION_INVALID_MANIFEST" ]] \
  || die "duplicate validation must produce no report or manifest"

run_expected_validation_error validation-project-duplicate-node \
  validate \
  --source "$VALIDATION_PROJECT_DUPLICATE_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_PROJECT_DUPLICATE_MANIFEST"
grep -F "source validation error:2" "$WORK_DIR/validation-project-duplicate-node.stderr" >/dev/null \
  || die "project duplicate validation diagnostic must expose the canonical duplicate-node code"
[[ ! -e "$VALIDATION_PROJECT_DUPLICATE_MANIFEST" ]] \
  || die "project duplicate validation must produce no report or manifest"

run_command compile 0 compile "$INPUT" -o "$COMPILE_OUTPUT"
run_command build 0 build "$INPUT" -o "$BUILD_OUTPUT"
for output in "$COMPILE_OUTPUT" "$BUILD_OUTPUT"; do
  [[ -s "$output" ]] || die "native command did not write output: $output"
  [[ "$(od -An -tx1 -N4 "$output" | tr -d '[:space:]')" == "0061736d" ]] \
    || die "native command did not write core Wasm bytes: $output"
done
grep -Eq '^wasm-size:[1-9][0-9]*$' "$WORK_DIR/compile.stdout" \
  || die "compile stdout is missing a positive wasm-size"
grep -Eq '^wasm-size:[1-9][0-9]*$' "$WORK_DIR/build.stdout" \
  || die "build stdout is missing a positive wasm-size"

printf '%s native selfhost source-file smoke passed\n' "$expected_target"
