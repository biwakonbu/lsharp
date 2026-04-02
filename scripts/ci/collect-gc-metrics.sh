#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ARTIFACT_SHA="${GITHUB_SHA:-local}"
ARTIFACT_DIR="${ROOT_DIR}/ci-artifacts/gc-metrics/${ARTIFACT_SHA}"
ARTIFACT_FILE="${LSHARP_GC_METRICS_INPUT:-${ARTIFACT_DIR}/summary.json}"

cd "${ROOT_DIR}"

if [[ -n "${LSHARP_GC_METRICS_INPUT:-}" ]]; then
    echo "gc-metrics-artifact: validate-only fixture ${ARTIFACT_FILE}"
else
    mkdir -p "${ARTIFACT_DIR}"
    export LSHARP_GC_METRICS_OUT="${ARTIFACT_FILE}"
    cargo test -p lsharp-wasm --test e2e test_e2e_alloc_metrics_ci_artifact_payload -- --nocapture
fi

python3 - "${ARTIFACT_FILE}" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])

# AR-02: JSON ファイルを読み取れるか
try:
    text = path.read_text()
except OSError as e:
    raise SystemExit(f"AR-02: cannot read artifact file {path}: {e}")

# AR-03: JSON として parse できるか
try:
    payload = json.loads(text)
except json.JSONDecodeError as e:
    raise SystemExit(f"AR-03: artifact is not valid JSON: {e}")

def evaluate_s14_status(payload):
    if payload["allocator_mode"] == "bump":
        return "n/a"

    series = payload["heap_bytes_series"]
    if not isinstance(series, list):
        raise SystemExit("AR-04: heap_bytes_series must be a JSON array")
    if len(series) < 2:
        return "blocked"

    tail_start = (len(series) * 9) // 10
    head = series[:tail_start]
    tail = series[tail_start:]
    if not head:
        return "blocked"

    running_max = max(head)
    for sample in tail:
        if not isinstance(sample, (int, float)):
            raise SystemExit("AR-04: heap_bytes_series entries must be numbers")
        if sample > running_max:
            running_max = sample
            continue
        return "pass"

    return "fail"

def validate_s15_proof(status, proof):
    if status in {"blocked", "n/a"}:
        if proof is not None:
            raise SystemExit(
                f"AR-04: s15_proof must be null when s15_status is '{status}'"
            )
        return

    if not isinstance(proof, dict):
        raise SystemExit(
            f"AR-04: s15_proof must be a JSON object when s15_status is '{status}'"
        )

    required = [
        "gc_mode",
        "stage_pair",
        "bytes_identical",
        "exports_identical",
        "data_sections_identical",
        "diagnostics_identical",
    ]
    missing = [key for key in required if key not in proof]
    if missing:
        raise SystemExit(f"AR-04: missing s15_proof keys: {', '.join(missing)}")

    if not isinstance(proof["gc_mode"], str) or not proof["gc_mode"]:
        raise SystemExit("AR-04: s15_proof.gc_mode must be a non-empty string")

    stage_pair = proof["stage_pair"]
    if (
        not isinstance(stage_pair, list)
        or len(stage_pair) != 2
        or not all(isinstance(item, str) and item for item in stage_pair)
    ):
        raise SystemExit(
            "AR-04: s15_proof.stage_pair must be a 2-element string array"
        )

    comparison_keys = [
        "bytes_identical",
        "exports_identical",
        "data_sections_identical",
        "diagnostics_identical",
    ]
    comparison_values = []
    for key in comparison_keys:
        value = proof[key]
        if not isinstance(value, bool):
            raise SystemExit(f"AR-04: s15_proof.{key} must be a boolean")
        comparison_values.append(value)

    if status == "pass" and not all(comparison_values):
        raise SystemExit(
            "AR-04: s15_proof comparisons must all be true when s15_status is 'pass'"
        )
    if status == "fail" and all(comparison_values):
        raise SystemExit(
            "AR-04: s15_proof must show at least one mismatch when s15_status is 'fail'"
        )

def validate_s16_proof(status, proof):
    if status in {"blocked", "n/a"}:
        if proof is not None:
            raise SystemExit(
                f"AR-04: s16_proof must be null when s16_status is '{status}'"
            )
        return

    if not isinstance(proof, dict):
        raise SystemExit(
            f"AR-04: s16_proof must be a JSON object when s16_status is '{status}'"
        )

    required = [
        "gc_mode",
        "completed_workloads",
        "all_workloads_completed",
        "sigsegv_count",
        "trap_count",
        "unreachable_count",
        "dangling_pointer_count",
    ]
    missing = [key for key in required if key not in proof]
    if missing:
        raise SystemExit(f"AR-04: missing s16_proof keys: {', '.join(missing)}")

    if not isinstance(proof["gc_mode"], str) or not proof["gc_mode"]:
        raise SystemExit("AR-04: s16_proof.gc_mode must be a non-empty string")

    completed_workloads = proof["completed_workloads"]
    if (
        not isinstance(completed_workloads, list)
        or not all(isinstance(item, str) and item for item in completed_workloads)
    ):
        raise SystemExit(
            "AR-04: s16_proof.completed_workloads must be a string array"
        )

    all_workloads_completed = proof["all_workloads_completed"]
    if not isinstance(all_workloads_completed, bool):
        raise SystemExit("AR-04: s16_proof.all_workloads_completed must be a boolean")

    counter_keys = [
        "sigsegv_count",
        "trap_count",
        "unreachable_count",
        "dangling_pointer_count",
    ]
    counter_values = []
    for key in counter_keys:
        value = proof[key]
        if not isinstance(value, int) or value < 0:
            raise SystemExit(f"AR-04: s16_proof.{key} must be a non-negative integer")
        counter_values.append(value)

    if status == "pass":
        if not all_workloads_completed:
            raise SystemExit(
                "AR-04: s16_proof.all_workloads_completed must be true when s16_status is 'pass'"
            )
        if any(counter_values):
            raise SystemExit(
                "AR-04: s16_proof crash counters must all be zero when s16_status is 'pass'"
            )
        if not completed_workloads:
            raise SystemExit(
                "AR-04: s16_proof.completed_workloads must be non-empty when s16_status is 'pass'"
            )
    if status == "fail" and all_workloads_completed and not any(counter_values):
        raise SystemExit(
            "AR-04: s16_proof must show an incomplete workload or non-zero crash counter when s16_status is 'fail'"
        )

# AR-04: 必須キーが揃っているか (gate_status / s14_status / s15_status / s16_status を含む)
required = [
    "allocator_mode",
    "ci_level",
    "gate_status",
    "s14_status",
    "s15_status",
    "s16_status",
    "s15_proof",
    "s16_proof",
    "heap_bytes_series",
    "proxy_workloads",
    "peak_alloc_bytes",
    "total_alloc_count",
    "live_alloc_count",
    "max_single_alloc",
    "alloc_span",
    "leak_growing_count",
    "leak_total",
    "leak_suspect",
]
missing = [key for key in required if key not in payload]
if missing:
    raise SystemExit(f"AR-04: missing GC metrics keys: {', '.join(missing)}")

proxy_workloads = payload["proxy_workloads"]
if not isinstance(proxy_workloads, dict):
    raise SystemExit("AR-04: proxy_workloads must be a JSON object")
required_proxy_workloads = [
    "compile_run_light_loop",
    "repl_soak_50_eval",
    "repl_stateful_long_session",
    "repl_stateful_single_session",
    "lsp_actual_stdio_repeated_sequence",
]
missing_proxy_workloads = [
    key for key in required_proxy_workloads if key not in proxy_workloads
]
if missing_proxy_workloads:
    raise SystemExit(
        "AR-04: missing proxy_workloads entries: "
        + ", ".join(missing_proxy_workloads)
    )
for name in required_proxy_workloads:
    workload = proxy_workloads[name]
    if not isinstance(workload, dict):
        raise SystemExit(f"AR-04: proxy_workloads.{name} must be a JSON object")
    if workload.get("status") != "pass":
        raise SystemExit(
            f"AR-01: proxy_workloads.{name}.status is '{workload.get('status')}' "
            "(expected 'pass')"
        )

# gate_status が "accepted" 以外ならジョブを失敗させる
gate_status = payload["gate_status"]
if gate_status != "accepted":
    raise SystemExit(f"AR-01: gate_status is '{gate_status}' (expected 'accepted')")

# S14-S16 の状態を出力して可視化する (blocking は CI 上では warning 扱い)
valid_states = {"pass", "fail", "blocked", "n/a"}
for gate in ("s14_status", "s15_status", "s16_status"):
    state = payload[gate]
    if state not in valid_states:
        raise SystemExit(f"AR-04: {gate} has invalid value '{state}'")
    if state == "fail":
        raise SystemExit(f"AR-01: {gate} is 'fail' -- runtime stability gate violation")
    print(f"  {gate}: {state}")

computed_s14_status = evaluate_s14_status(payload)
if payload["s14_status"] != computed_s14_status:
    raise SystemExit(
        f"AR-04: s14_status is '{payload['s14_status']}' but computed '{computed_s14_status}'"
    )

validate_s15_proof(payload["s15_status"], payload["s15_proof"])
validate_s16_proof(payload["s16_status"], payload["s16_proof"])

print(f"gc-metrics-artifact:{path}")
print(json.dumps(payload, sort_keys=True))
PY
