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

# AR-04: 必須キーが揃っているか (gate_status / s14_status / s15_status / s16_status を含む)
required = [
    "allocator_mode",
    "ci_level",
    "gate_status",
    "s14_status",
    "s15_status",
    "s16_status",
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

print(f"gc-metrics-artifact:{path}")
print(json.dumps(payload, sort_keys=True))
PY
