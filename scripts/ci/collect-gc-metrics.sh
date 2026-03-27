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
payload = json.loads(path.read_text())
required = [
    "allocator_mode",
    "ci_level",
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
    raise SystemExit(f"missing GC metrics keys: {', '.join(missing)}")
print(f"gc-metrics-artifact:{path}")
print(json.dumps(payload, sort_keys=True))
PY
