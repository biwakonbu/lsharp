#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
VM_NAME="${LSHARP_NATIVE_LINUX_X86_VM_NAME:-lsharp-linux-x86}"
HOSTGEN_REPLAY_LOCK_DIR="${LSHARP_NATIVE_LINUX_X86_HOST_REPLAY_LOCK_DIR:-/tmp/lsharp-native-linux-x86-hostgen-vm-${VM_NAME}.lock}"
STAGE0_DIR_INPUT="${LSHARP_NATIVE_LINUX_X86_STAGE0_DIR:-}"
SOURCE_SMOKE_EVIDENCE_DIR_INPUT="${LSHARP_NATIVE_LINUX_X86_SOURCE_SMOKE_EVIDENCE_DIR:-}"
KEEP_WORK_DIR="${LSHARP_NATIVE_LINUX_X86_KEEP_NATIVE_STAGE0_SOURCE_SMOKE_WORK_DIR:-0}"
VM_MIN_FREE_BYTES="${LSHARP_NATIVE_LINUX_X86_VM_MIN_FREE_BYTES:-4294967296}"
TRANSPORT_CHUNK_SIZE="${LSHARP_NATIVE_LINUX_X86_TRANSPORT_CHUNK_SIZE:-64}"
TRANSPORT_TIMEOUT_SECONDS="${LSHARP_NATIVE_LINUX_X86_TRANSPORT_TIMEOUT_SECONDS:-900}"
VM_WORK_DIR="/tmp/lsharp-native-stage0-source-file-smoke-$$"
VM_WORK_DIR_CREATED=0
VM_STARTED_BY_SMOKE=0
SOURCE_SMOKE_EVIDENCE_DIR=""
VM_SOURCE_SMOKE_EVIDENCE_DIR=""

cleanup() {
  local exit_status=$?
  local cleanup_status=0
  if [[ "${VM_WORK_DIR_CREATED}" -eq 1 && "${KEEP_WORK_DIR}" != "1" ]] \
    && command -v limactl >/dev/null 2>&1; then
    if ! limactl shell "${VM_NAME}" -- rm -rf "${VM_WORK_DIR}" >/dev/null 2>&1; then
      cleanup_status=1
    fi
  fi
  if [[ "${VM_STARTED_BY_SMOKE}" -eq 1 ]] && command -v limactl >/dev/null 2>&1; then
    if ! limactl stop "${VM_NAME}" >/dev/null 2>&1; then
      cleanup_status=1
    fi
  fi
  if [[ "${cleanup_status}" -ne 0 ]]; then
    echo "ERROR: Linux native stage0 source-file smoke cleanup failed" >&2
  fi
  if [[ "${exit_status}" -ne 0 ]]; then
    exit "${exit_status}"
  fi
  if [[ "${cleanup_status}" -ne 0 ]]; then
    exit 1
  fi
  exit 0
}
trap cleanup EXIT

die() {
  echo "ERROR: $*" >&2
  exit 1
}

require_safe_lock_path() {
  local path="$1"
  case "${path}" in
    ""|/|*/../*|*/..)
      die "refusing unsafe Linux hostgen replay lock path: ${path}"
      ;;
  esac
  case "${path}" in
    /tmp/lsharp-*) ;;
    *) die "Linux hostgen replay lock must be under /tmp/lsharp-*: ${path}" ;;
  esac
}

preflight_hostgen_replay_lock() {
  if [[ ! -e "${HOSTGEN_REPLAY_LOCK_DIR}" && ! -L "${HOSTGEN_REPLAY_LOCK_DIR}" ]]; then
    return 0
  fi
  if [[ ! -d "${HOSTGEN_REPLAY_LOCK_DIR}" || -L "${HOSTGEN_REPLAY_LOCK_DIR}" ]]; then
    echo "ERROR: Linux hostgen replay lock has an unsafe shape: ${HOSTGEN_REPLAY_LOCK_DIR}" >&2
    exit 90
  fi

  local holder_pid
  local holder_artifact_dir
  local holder_vm_work_dir
  holder_pid="$(cat "${HOSTGEN_REPLAY_LOCK_DIR}/pid" 2>/dev/null || true)"
  holder_artifact_dir="$(cat "${HOSTGEN_REPLAY_LOCK_DIR}/artifact_dir" 2>/dev/null || true)"
  holder_vm_work_dir="$(cat "${HOSTGEN_REPLAY_LOCK_DIR}/vm_work_dir" 2>/dev/null || true)"
  if [[ "${holder_pid}" =~ ^[0-9]+$ ]] && kill -0 "${holder_pid}" 2>/dev/null; then
    echo "ERROR: Linux hostgen replay lock is held: holder_pid=${holder_pid} artifact_dir=${holder_artifact_dir} vm_work_dir=${holder_vm_work_dir} lock_dir=${HOSTGEN_REPLAY_LOCK_DIR}" >&2
    exit 90
  fi

  echo "ERROR: Linux hostgen replay lock exists without a live owner; refusing to remove it: lock_dir=${HOSTGEN_REPLAY_LOCK_DIR}" >&2
  exit 90
}

require_safe_lock_path "${HOSTGEN_REPLAY_LOCK_DIR}"

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
    VM_STARTED_BY_SMOKE=1
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

require_provenance_safe_stage0_dir() {
  local path="$1"
  [[ -d "${path}" && ! -L "${path}" ]] \
    || die "Linux native stage0 input must be a regular directory without symlinks: ${path}"

  local symlink
  if ! symlink="$(find -P "${path}" -type l -print -quit)"; then
    die "could not inspect Linux native stage0 directory for symlinks: ${path}"
  fi
  [[ -z "${symlink}" ]] \
    || die "Linux native stage0 input contains a symlink: ${symlink}"
}

require_provenance_safe_stage0_dir "${STAGE0_DIR}"
require_file "${STAGE0_DIR}/manifest.json" "Linux native stage0 manifest"
require_file "${ROOT_DIR}/selfhost/src/App/Cli.ls" "native selfhost App.Cli source"
require_file "${ROOT_DIR}/scripts/native-selfhost-dev.sh" "native selfhost runner"
require_file "${ROOT_DIR}/scripts/ci/native-selfhost-dev-source-file-smoke.sh" "native source-file smoke"
require_file "${ROOT_DIR}/scripts/ci/decode-native-selfhost-transport.py" "native transport decoder"
if [[ -n "${SOURCE_SMOKE_EVIDENCE_DIR_INPUT}" ]]; then
  [[ "${SOURCE_SMOKE_EVIDENCE_DIR_INPUT}" = /* && "${SOURCE_SMOKE_EVIDENCE_DIR_INPUT}" != "/" ]] \
    || die "LSHARP_NATIVE_LINUX_X86_SOURCE_SMOKE_EVIDENCE_DIR must be an absolute non-root path"
  [[ ! -e "${SOURCE_SMOKE_EVIDENCE_DIR_INPUT}" && ! -L "${SOURCE_SMOKE_EVIDENCE_DIR_INPUT}" ]] \
    || die "Linux native source-file smoke evidence directory already exists: ${SOURCE_SMOKE_EVIDENCE_DIR_INPUT}"
  require_file "${ROOT_DIR}/scripts/ci/write-native-source-smoke-evidence.py" \
    "native source-file smoke evidence writer"
  SOURCE_SMOKE_EVIDENCE_DIR="${SOURCE_SMOKE_EVIDENCE_DIR_INPUT}"
fi
require_file "${ROOT_DIR}/tests/fixtures/validation/ec-m3-review-attestation-source.ls" \
  "EC-M3-04 review attestation source fixture"

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

preflight_hostgen_replay_lock
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
if [[ -n "${SOURCE_SMOKE_EVIDENCE_DIR}" ]]; then
  limactl copy "${ROOT_DIR}/scripts/ci/write-native-source-smoke-evidence.py" \
    "${VM_NAME}:${VM_WORK_DIR}/scripts/ci/write-native-source-smoke-evidence.py"
  VM_SOURCE_SMOKE_EVIDENCE_DIR="${VM_WORK_DIR}/source-smoke-evidence"
fi
limactl copy "${ROOT_DIR}/tests/fixtures/validation/ec-m3-canonical-source.ls" \
  "${VM_NAME}:${VM_WORK_DIR}/tests/fixtures/validation/ec-m3-canonical-source.ls"
limactl copy "${ROOT_DIR}/tests/fixtures/validation/ec-m3-canonical-manifest.json" \
  "${VM_NAME}:${VM_WORK_DIR}/tests/fixtures/validation/ec-m3-canonical-manifest.json"
limactl copy "${ROOT_DIR}/tests/fixtures/validation/ec-m3-duplicate-node-source.ls" \
  "${VM_NAME}:${VM_WORK_DIR}/tests/fixtures/validation/ec-m3-duplicate-node-source.ls"
limactl copy "${ROOT_DIR}/tests/fixtures/validation/ec-m2-project-duplicate-source.ls" \
  "${VM_NAME}:${VM_WORK_DIR}/tests/fixtures/validation/ec-m2-project-duplicate-source.ls"
limactl copy "${ROOT_DIR}/tests/fixtures/validation/ec-m3-review-attestation-source.ls" \
  "${VM_NAME}:${VM_WORK_DIR}/tests/fixtures/validation/ec-m3-review-attestation-source.ls"

set +e
limactl shell "${VM_NAME}" -- env \
  NATIVE_STAGE0_DIR="${VM_WORK_DIR}/stage0" \
  NATIVE_SELFHOST_SOURCE_ROOT="${VM_WORK_DIR}/selfhost" \
  NATIVE_SELFHOST_STAGE_DIR="${VM_WORK_DIR}/dev-stage" \
  NATIVE_STAGE0_TRANSPORT_CHUNK_SIZE="${TRANSPORT_CHUNK_SIZE}" \
  NATIVE_STAGE0_TRANSPORT_TIMEOUT_SECONDS="${TRANSPORT_TIMEOUT_SECONDS}" \
  NATIVE_SELFHOST_KEEP_STAGE_DIR=1 \
  NATIVE_SELFHOST_SOURCE_SMOKE_EVIDENCE_DIR="${VM_SOURCE_SMOKE_EVIDENCE_DIR}" \
  bash "${VM_WORK_DIR}/scripts/ci/native-selfhost-dev-source-file-smoke.sh"
smoke_status=$?
set -e

if [[ -n "${SOURCE_SMOKE_EVIDENCE_DIR}" ]]; then
  mkdir -p "${SOURCE_SMOKE_EVIDENCE_DIR}"
  if ! limactl copy --recursive \
    "${VM_NAME}:${VM_SOURCE_SMOKE_EVIDENCE_DIR}/." \
    "${SOURCE_SMOKE_EVIDENCE_DIR}"; then
    echo "ERROR: source smoke evidence copy failed" >&2
    [[ "${smoke_status}" -ne 0 ]] || smoke_status=1
  fi
fi

if [[ "${smoke_status}" -ne 0 ]]; then
  exit "${smoke_status}"
fi

printf 'Linux x86_64 native stage0 source-file smoke passed\n'
