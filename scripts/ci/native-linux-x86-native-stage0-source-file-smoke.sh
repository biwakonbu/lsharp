#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
VM_NAME="${LSHARP_NATIVE_LINUX_X86_VM_NAME:-lsharp-linux-x86}"
STAGE0_DIR_INPUT="${LSHARP_NATIVE_LINUX_X86_STAGE0_DIR:-}"
KEEP_WORK_DIR="${LSHARP_NATIVE_LINUX_X86_KEEP_NATIVE_STAGE0_SOURCE_SMOKE_WORK_DIR:-0}"
VM_MIN_FREE_BYTES="${LSHARP_NATIVE_LINUX_X86_VM_MIN_FREE_BYTES:-4294967296}"
TRANSPORT_CHUNK_SIZE="${LSHARP_NATIVE_LINUX_X86_TRANSPORT_CHUNK_SIZE:-64}"
TRANSPORT_TIMEOUT_SECONDS="${LSHARP_NATIVE_LINUX_X86_TRANSPORT_TIMEOUT_SECONDS:-900}"
VM_WORK_DIR="/tmp/lsharp-native-stage0-source-file-smoke-$$"
VM_WORK_DIR_CREATED=0

cleanup() {
  if [[ "${VM_WORK_DIR_CREATED}" -eq 1 && "${KEEP_WORK_DIR}" != "1" ]] \
    && command -v limactl >/dev/null 2>&1; then
    limactl shell "${VM_NAME}" -- rm -rf "${VM_WORK_DIR}" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

die() {
  echo "ERROR: $*" >&2
  exit 1
}

SOURCE_COMMIT="$(git -C "$ROOT_DIR" rev-parse --verify HEAD 2>/dev/null || true)"
[[ "$SOURCE_COMMIT" =~ ^[0-9a-f]{40}$ ]] \
  || die "current checkout source commit is unavailable: $SOURCE_COMMIT"

[[ "${TRANSPORT_CHUNK_SIZE}" =~ ^[1-9][0-9]*$ ]] \
  || die "LSHARP_NATIVE_LINUX_X86_TRANSPORT_CHUNK_SIZE must be a positive integer"
[[ "${TRANSPORT_TIMEOUT_SECONDS}" =~ ^[1-9][0-9]*$ ]] \
  || die "LSHARP_NATIVE_LINUX_X86_TRANSPORT_TIMEOUT_SECONDS must be a positive integer"

require_file() {
  local path="$1"
  local description="$2"
  [[ -f "${path}" && -s "${path}" ]] || die "${description} is required: ${path}"
}

ensure_vm_running() {
  local status
  status="$(limactl list "${VM_NAME}" --format '{{.Status}}' 2>/dev/null || true)"
  if [[ "${status}" != "Running" ]]; then
    limactl start --tty=false "${VM_NAME}"
  fi
}

require_vm_free_space() {
  local available_kib
  local available_bytes

  [[ "${VM_MIN_FREE_BYTES}" =~ ^[0-9]+$ ]] \
    || die "LSHARP_NATIVE_LINUX_X86_VM_MIN_FREE_BYTES must be a non-negative integer: ${VM_MIN_FREE_BYTES}"
  available_kib="$(limactl shell "${VM_NAME}" -- sh -lc "df -Pk /tmp | awk 'NR == 2 {print \$4}'" | tr -d '[:space:]')"
  [[ "${available_kib}" =~ ^[0-9]+$ ]] || die "could not read VM free space: ${available_kib}"
  available_bytes=$((available_kib * 1024))
  (( available_bytes >= VM_MIN_FREE_BYTES )) \
    || die "VM free space is below required minimum: available=${available_bytes} required=${VM_MIN_FREE_BYTES}"
  echo "VM free space gate: available=${available_bytes} required=${VM_MIN_FREE_BYTES}"
}

[[ "$(uname -s)" == "Darwin" && "$(uname -m)" == "arm64" ]] \
  || die "this native stage0 source-file smoke requires macOS arm64 with Lima; got $(uname -s)/$(uname -m)"
command -v limactl >/dev/null 2>&1 || die "limactl is required for the Linux x86_64 native stage0 source-file smoke"
[[ -n "${STAGE0_DIR_INPUT}" ]] || die "LSHARP_NATIVE_LINUX_X86_STAGE0_DIR is required"

if [[ "${STAGE0_DIR_INPUT}" = /* ]]; then
  STAGE0_DIR="${STAGE0_DIR_INPUT}"
else
  STAGE0_DIR="${ROOT_DIR}/${STAGE0_DIR_INPUT}"
fi

require_file "${STAGE0_DIR}/manifest.json" "Linux native stage0 manifest"
require_file "${ROOT_DIR}/selfhost/src/App/Cli.ls" "native selfhost App.Cli source"
require_file "${ROOT_DIR}/scripts/native-selfhost-dev.sh" "native selfhost runner"
require_file "${ROOT_DIR}/scripts/ci/native-selfhost-dev-source-file-smoke.sh" "native source-file smoke"
require_file "${ROOT_DIR}/scripts/ci/decode-native-selfhost-transport.py" "native transport decoder"

python3 - "${STAGE0_DIR}/manifest.json" "$SOURCE_COMMIT" <<'PY'
import json
import pathlib
import sys

manifest_path = pathlib.Path(sys.argv[1])
expected_source_commit = sys.argv[2]
try:
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
except (OSError, json.JSONDecodeError) as error:
    raise SystemExit(f"invalid native stage0 manifest: {error}")

if manifest.get("kind") != "lsharp-native-selfhost-stage0":
    raise SystemExit("native stage0 manifest kind is invalid")
if manifest.get("source_commit") != expected_source_commit:
    raise SystemExit(
        "native stage0 manifest source_commit does not match current checkout: "
        f"manifest={manifest.get('source_commit')!r} checkout={expected_source_commit}"
    )
if manifest.get("target") != "x86_64-unknown-linux-gnu":
    raise SystemExit(f"native stage0 target must be x86_64-unknown-linux-gnu: {manifest.get('target')!r}")
for field in ("compiler", "transport_driver", "materializer"):
    value = manifest.get(field)
    if not isinstance(value, str) or not value:
        raise SystemExit(f"native stage0 manifest {field} is invalid")
    path = manifest_path.parent / value
    if not path.is_file() or not path.stat().st_size:
        raise SystemExit(f"native stage0 executable is unavailable: {path}")
PY

ensure_vm_running
require_vm_free_space
limactl shell "${VM_NAME}" -- rm -rf "${VM_WORK_DIR}"
limactl shell "${VM_NAME}" -- mkdir -p \
  "${VM_WORK_DIR}/stage0" \
  "${VM_WORK_DIR}/selfhost" \
  "${VM_WORK_DIR}/scripts/ci" \
  "${VM_WORK_DIR}/tests/fixtures/validation"
VM_WORK_DIR_CREATED=1

limactl copy --recursive "${STAGE0_DIR}/." "${VM_NAME}:${VM_WORK_DIR}/stage0"
limactl copy --recursive "${ROOT_DIR}/selfhost/." "${VM_NAME}:${VM_WORK_DIR}/selfhost"
limactl copy "${ROOT_DIR}/scripts/native-selfhost-dev.sh" "${VM_NAME}:${VM_WORK_DIR}/scripts/native-selfhost-dev.sh"
limactl copy "${ROOT_DIR}/scripts/ci/native-selfhost-dev-source-file-smoke.sh" \
  "${VM_NAME}:${VM_WORK_DIR}/scripts/ci/native-selfhost-dev-source-file-smoke.sh"
limactl copy "${ROOT_DIR}/scripts/ci/decode-native-selfhost-transport.py" \
  "${VM_NAME}:${VM_WORK_DIR}/scripts/ci/decode-native-selfhost-transport.py"
limactl copy "${ROOT_DIR}/tests/fixtures/validation/ec-m3-canonical-source.ls" \
  "${VM_NAME}:${VM_WORK_DIR}/tests/fixtures/validation/ec-m3-canonical-source.ls"
limactl copy "${ROOT_DIR}/tests/fixtures/validation/ec-m3-canonical-manifest.json" \
  "${VM_NAME}:${VM_WORK_DIR}/tests/fixtures/validation/ec-m3-canonical-manifest.json"
limactl copy "${ROOT_DIR}/tests/fixtures/validation/ec-m3-duplicate-node-source.ls" \
  "${VM_NAME}:${VM_WORK_DIR}/tests/fixtures/validation/ec-m3-duplicate-node-source.ls"

limactl shell "${VM_NAME}" -- env \
  NATIVE_STAGE0_DIR="${VM_WORK_DIR}/stage0" \
  NATIVE_SELFHOST_SOURCE_ROOT="${VM_WORK_DIR}/selfhost" \
  NATIVE_SELFHOST_STAGE_DIR="${VM_WORK_DIR}/dev-stage" \
  NATIVE_STAGE0_TRANSPORT_CHUNK_SIZE="${TRANSPORT_CHUNK_SIZE}" \
  NATIVE_STAGE0_TRANSPORT_TIMEOUT_SECONDS="${TRANSPORT_TIMEOUT_SECONDS}" \
  NATIVE_SELFHOST_KEEP_STAGE_DIR=1 \
  bash "${VM_WORK_DIR}/scripts/ci/native-selfhost-dev-source-file-smoke.sh"

printf 'Linux x86_64 native stage0 source-file smoke passed\n'
