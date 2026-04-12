#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ARTIFACT_ID="${NATIVE_PROXY_ARTIFACT_ID:-${GITHUB_SHA:-local}}"
ARTIFACT_DIR_INPUT="${LSHARP_NATIVE_PROXY_ARTIFACT_DIR:-ci-artifacts/native-proxy/${ARTIFACT_ID}}"

if [[ "${ARTIFACT_DIR_INPUT}" = /* ]]; then
  if [[ "${ARTIFACT_DIR_INPUT}" != "${ROOT_DIR}"/* ]]; then
    echo "ERROR: LSHARP_NATIVE_PROXY_ARTIFACT_DIR must be under repository root: ${ARTIFACT_DIR_INPUT}" >&2
    exit 1
  fi
  ARTIFACT_DIR="${ARTIFACT_DIR_INPUT}"
else
  ARTIFACT_DIR="${ROOT_DIR}/${ARTIFACT_DIR_INPUT}"
fi

cd "${ROOT_DIR}"

HOST_OS="$(uname -s)"
HOST_ARCH="$(uname -m)"
if [[ "${HOST_OS}" != "Darwin" || "${HOST_ARCH}" != "arm64" ]]; then
  echo "ERROR: build-native.sh currently supports Darwin arm64 only; got ${HOST_OS}/${HOST_ARCH}" >&2
  exit 1
fi

rm -rf "${ARTIFACT_DIR}"
mkdir -p "${ARTIFACT_DIR}"

export LSHARP_NATIVE_PROXY_ARTIFACT_DIR="${ARTIFACT_DIR}"

echo "=== native proxy bundle gate ==="
echo "artifact dir: ${ARTIFACT_DIR}"

cargo test -p lsharp-wasm --test e2e \
  e2e::selfhost_native_stage_chain::test_e2e_native_host_bundle_uses_canonical_artifact_contract \
  -- --exact --nocapture
cargo test -p lsharp-wasm --test e2e \
  e2e::selfhost_native_stage_chain::test_e2e_stage23_native_host_bundle_proxy_observations_match \
  -- --exact --nocapture

for stage in stage1-native stage2-native stage3-native; do
  if [[ ! -s "${ARTIFACT_DIR}/${stage}/summary.json" ]]; then
    echo "ERROR: missing native proxy artifact summary for ${stage}" >&2
    exit 1
  fi
done

python3 - "${ARTIFACT_DIR}" <<'PY'
import json
import pathlib
import sys

root = pathlib.Path(sys.argv[1])
labels = ["stage1-native", "stage2-native", "stage3-native"]
manifest = {"stages": {}}

for label in labels:
    summary_path = root / label / "summary.json"
    manifest["stages"][label] = json.loads(summary_path.read_text())

(root / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
PY

echo "native proxy bundle gate complete."
