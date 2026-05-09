#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ARTIFACT_ID="${NATIVE_LINUX_X86_ARTIFACT_ID:-${GITHUB_SHA:-local}}"
ARTIFACT_DIR_INPUT="${LSHARP_NATIVE_LINUX_X86_ARTIFACT_DIR:-ci-artifacts/native-linux-x86/${ARTIFACT_ID}}"

if [[ "${ARTIFACT_DIR_INPUT}" = /* ]]; then
  if [[ "${ARTIFACT_DIR_INPUT}" != "${ROOT_DIR}"/* ]]; then
    echo "ERROR: LSHARP_NATIVE_LINUX_X86_ARTIFACT_DIR must be under repository root: ${ARTIFACT_DIR_INPUT}" >&2
    exit 1
  fi
  ARTIFACT_DIR="${ARTIFACT_DIR_INPUT}"
else
  ARTIFACT_DIR="${ROOT_DIR}/${ARTIFACT_DIR_INPUT}"
fi

cd "${ROOT_DIR}"

HOST_OS="$(uname -s)"
HOST_ARCH="$(uname -m)"
if [[ "${HOST_OS}" != "Linux" || "${HOST_ARCH}" != "x86_64" ]]; then
  echo "ERROR: native-linux-x86-smoke.sh requires Linux/x86_64; got ${HOST_OS}/${HOST_ARCH}" >&2
  exit 1
fi

rm -rf "${ARTIFACT_DIR}"
mkdir -p "${ARTIFACT_DIR}"

echo "=== native Linux x86_64 server target smoke ==="
echo "artifact dir: ${ARTIFACT_DIR}"
echo "scope: target descriptor / ELF emitter / x86_64 codegen smoke; actual self-regeneration remains V2-13a follow-up."

cargo test -q -p lsharp-wasm --test e2e \
  e2e::selfhost_gc_runtime_bootstrap::test_e2e_selfhost_native_target_descriptors \
  -- --exact
cargo test -q -p lsharp-wasm --test e2e \
  e2e::selfhost_gc_runtime_bootstrap::test_e2e_selfhost_native_object_emitter \
  -- --exact
cargo test -q -p lsharp-wasm --test e2e \
  e2e::selfhost_native_differential::test_native_codegen_emits_x86_i32_core_instruction_bytes \
  -- --exact --ignored

cat >"${ARTIFACT_DIR}/summary.json" <<JSON
{
  "target": "x86_64-unknown-linux-gnu",
  "host_os": "${HOST_OS}",
  "host_arch": "${HOST_ARCH}",
  "status": "pass",
  "scope": "target descriptor / ELF emitter / x86_64 codegen smoke",
  "actual_self_regeneration": "pending"
}
JSON

echo "native Linux x86_64 server target smoke evidence collected."
