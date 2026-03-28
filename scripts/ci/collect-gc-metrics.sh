#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ARTIFACT_SHA="${GITHUB_SHA:-local}"
ARTIFACT_DIR="${ROOT_DIR}/ci-artifacts/gc-metrics/${ARTIFACT_SHA}"
ARTIFACT_FILE="${ARTIFACT_DIR}/summary.json"

mkdir -p "${ARTIFACT_DIR}"

cd "${ROOT_DIR}"
export LSHARP_GC_METRICS_OUT="${ARTIFACT_FILE}"

cargo test -p lsharp-wasm --test e2e test_e2e_alloc_metrics_ci_artifact_payload -- --nocapture

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

# AR-04: 必須キーが揃っているか (gate_status / s14_status / s15_status / s16_status を含む)
required = [
    "allocator_mode",
    "ci_level",
    "gate_status",
    "s14_status",
    "s15_status",
    "s16_status",
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

print(f"gc-metrics-artifact:{path}")
print(json.dumps(payload, sort_keys=True))
PY
