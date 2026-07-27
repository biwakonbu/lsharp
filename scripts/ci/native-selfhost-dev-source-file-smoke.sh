#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
RUNNER="$ROOT/scripts/native-selfhost-dev.sh"
VALIDATION_SOURCE="$ROOT/tests/fixtures/validation/ec-m3-canonical-source.ls"
EXPECTED_VALIDATION_MANIFEST="$ROOT/tests/fixtures/validation/ec-m3-canonical-manifest.json"
VALIDATION_INVALID_SOURCE="$ROOT/tests/fixtures/validation/ec-m3-duplicate-node-source.ls"
STAGE0_DIR="${NATIVE_STAGE0_DIR:-}"
SOURCE_ROOT="${NATIVE_SELFHOST_SOURCE_ROOT:-$ROOT/selfhost}"
STAGE_DIR="${NATIVE_SELFHOST_STAGE_DIR:-}"
KEEP_STAGE_DIR="${NATIVE_SELFHOST_KEEP_STAGE_DIR:-0}"
WORK_DIR=""
STAGE_DIR_CREATED=0

cleanup() {
  [[ -z "$WORK_DIR" ]] || rm -rf "$WORK_DIR"
  if [[ "$STAGE_DIR_CREATED" -eq 1 && "$KEEP_STAGE_DIR" != "1" ]]; then
    rm -rf "$STAGE_DIR"
  fi
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
VALIDATION_ORPHAN_MANIFEST="$WORK_DIR/ec-m3-orphan-node-manifest.json"
VALIDATION_MALFORMED_MANIFEST="$WORK_DIR/ec-m3-malformed-edge-manifest.json"
VALIDATION_INVALID_ID_MANIFEST="$WORK_DIR/ec-m3-invalid-id-manifest.json"
VALIDATION_KIND_MISMATCH_MANIFEST="$WORK_DIR/ec-m3-kind-mismatch-manifest.json"
VALIDATION_EVIDENCE_REGISTRY_MANIFEST="$WORK_DIR/ec-m3-evidence-registry-manifest.json"
VALIDATION_MISSING_REVIEW_MANIFEST="$WORK_DIR/ec-m3-missing-review-manifest.json"
VALIDATION_DUPLICATE_REVIEW_MANIFEST="$WORK_DIR/ec-m3-duplicate-review-manifest.json"
VALIDATION_INVALID_REVIEW_MANIFEST="$WORK_DIR/ec-m3-invalid-review-manifest.json"
VALIDATION_REVIEW_SUBJECT_KIND_MANIFEST="$WORK_DIR/ec-m3-review-subject-kind-manifest.json"
VALIDATION_INVALIDATION_SUBJECT_KIND_MANIFEST="$WORK_DIR/ec-m3-invalidation-subject-kind-manifest.json"
VALIDATION_INVALIDATION_MISSING_REVIEW_MANIFEST="$WORK_DIR/ec-m3-invalidation-missing-review-manifest.json"
VALIDATION_WRITE_FAILURE_MANIFEST="$WORK_DIR/missing-parent/intent-graph.json"
VALIDATION_PASS_SOURCE="$WORK_DIR/ec-m3-complete-source.ls"
VALIDATION_FAIL_SOURCE="$WORK_DIR/ec-m3-contradiction-source.ls"
VALIDATION_STALE_SOURCE="$WORK_DIR/ec-m3-stale-source.ls"
VALIDATION_ORPHAN_SOURCE="$WORK_DIR/ec-m3-orphan-node-source.ls"
VALIDATION_MALFORMED_SOURCE="$WORK_DIR/ec-m3-malformed-edge-source.ls"
VALIDATION_INVALID_ID_SOURCE="$WORK_DIR/ec-m3-invalid-id-source.ls"
VALIDATION_KIND_MISMATCH_SOURCE="$WORK_DIR/ec-m3-kind-mismatch-source.ls"
VALIDATION_EVIDENCE_REGISTRY_SOURCE="$WORK_DIR/ec-m3-evidence-registry-source.ls"
VALIDATION_MISSING_REVIEW_SOURCE="$WORK_DIR/ec-m3-missing-review-source.ls"
VALIDATION_DUPLICATE_REVIEW_SOURCE="$WORK_DIR/ec-m3-duplicate-review-source.ls"
VALIDATION_INVALID_REVIEW_SOURCE="$WORK_DIR/ec-m3-invalid-review-source.ls"
VALIDATION_REVIEW_SUBJECT_KIND_SOURCE="$WORK_DIR/ec-m3-review-subject-kind-source.ls"
VALIDATION_INVALIDATION_SUBJECT_KIND_SOURCE="$WORK_DIR/ec-m3-invalidation-subject-kind-source.ls"
VALIDATION_INVALIDATION_MISSING_REVIEW_SOURCE="$WORK_DIR/ec-m3-invalidation-missing-review-source.ls"

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
cat >"$VALIDATION_MISSING_REVIEW_SOURCE" <<'LSHARP'
(defn missing-review-edge []
  :claim "claim:checkout/rejects" "The API rejects shipped orders"
  :review "review:checkout/registered" "sha256:review-provenance" "redacted"
  :evaluates "review:checkout/missing" "claim:checkout/rejects"
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
if report.get("independent_reviews") != 1:
    raise SystemExit(f"validation fail review metric is invalid: {report!r}")
if report.get("contradicting_observations") != 1:
    raise SystemExit(f"validation fail contradiction metric is invalid: {report!r}")
if report.get("stale_reviews") != 0 or report.get("stale_evidence") != 0:
    raise SystemExit(f"validation fail stale metrics are invalid: {report!r}")
PY

run_report_failure validation-fail-text 0 validate \
  --source "$VALIDATION_FAIL_SOURCE" \
  --format text
require_exact_output validation-fail-text $'status: fail\nopen-questions: 0\nindependent-reviews: 1\ncontradicting-observations: 1\nstale-reviews: 0\nstale-evidence: 0\n'

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

run_expected_validation_error validation-missing-review \
  validate \
  --source "$VALIDATION_MISSING_REVIEW_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_MISSING_REVIEW_MANIFEST"
grep -F "source validation error:10" "$WORK_DIR/validation-missing-review.stderr" >/dev/null \
  || die "missing review validation must expose the missing-review error code"
[[ ! -e "$VALIDATION_MISSING_REVIEW_MANIFEST" ]] \
  || die "missing review validation must produce no report or manifest"

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
