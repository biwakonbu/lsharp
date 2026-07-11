#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${ROOT_DIR}"

VM_NAME="${LSHARP_NATIVE_LINUX_X86_VM_NAME:-lsharp-linux-x86}"
PACKAGE_VERSION="$(cargo metadata --no-deps --format-version 1 | python3 -c 'import json,sys; data=json.load(sys.stdin); print(next(p["version"] for p in data["packages"] if p["name"] == "lsharp-wasm"))')"
ROLLBACK_VERSION="${ROLLBACK_VERSION:-v${PACKAGE_VERSION}}"
OUTPUT_DIR="${LSHARP_NATIVE_ROLLBACK_OUTPUT_DIR:-${ROOT_DIR}/ci-artifacts/native-release/rollback}"
HOST_WORK_DIR="${LSHARP_NATIVE_ROLLBACK_HOST_WORK_DIR:-/tmp/lsharp-native-rollback-compat-host-$$}"
VM_WORK_DIR="${LSHARP_NATIVE_ROLLBACK_VM_WORK_DIR:-/tmp/lsharp-native-rollback-compat-vm-$$}"
MAX_ROLLBACK_KIB="${LSHARP_NATIVE_RELEASE_MAX_ROLLBACK_KIB:-524288}"
SOURCE_COMMIT="$(git rev-parse HEAD)"

require_safe_cleanup_path() {
  local path="$1"
  local label="$2"
  case "${path}" in
    ""|/|"${ROOT_DIR}"|"${ROOT_DIR}/"|*/../*|*/..)
      echo "ERROR: refusing unsafe cleanup path for ${label}: ${path}" >&2
      exit 1
      ;;
  esac
  case "${path}" in
    "${ROOT_DIR}/ci-artifacts/native-release/"*|/tmp/lsharp-*) ;;
    *)
      echo "ERROR: refusing unsafe cleanup path for ${label}: ${path}" >&2
      exit 1
      ;;
  esac
}

require_safe_cleanup_path "${OUTPUT_DIR}" "rollback output"
require_safe_cleanup_path "${HOST_WORK_DIR}" "rollback host work"
require_safe_cleanup_path "${VM_WORK_DIR}" "rollback VM work"

if [[ -n "$(git status --porcelain --untracked-files=all)" ]]; then
  echo "ERROR: rollback compatibility provenance requires a clean worktree" >&2
  exit 1
fi

verify_rollback_manifest() {
  local archive="$1"
  local target="$2"
  python3 - "${archive}" "${target}" "${ROLLBACK_VERSION}" "${SOURCE_COMMIT}" <<'PY'
import json
import pathlib
import sys
import tarfile

archive = pathlib.Path(sys.argv[1])
target, version, source_commit = sys.argv[2:]
with tarfile.open(archive, "r:gz") as tar:
    members = [member for member in tar.getmembers() if member.name.endswith("/manifest.json")]
    if len(members) != 1 or not members[0].isfile():
        raise SystemExit("rollback archive must contain one regular manifest.json")
    extracted = tar.extractfile(members[0])
    if extracted is None:
        raise SystemExit("rollback manifest.json cannot be read")
    manifest = json.load(extracted)

expected = {
    "archive_kind": "rollback compatibility",
    "target": target,
    "version": version,
    "source_commit": source_commit,
}
for key, value in expected.items():
    if manifest.get(key) != value:
        raise SystemExit(f"rollback manifest {key} mismatch")
PY
}

cleanup() {
  rm -rf "${HOST_WORK_DIR}"
  if command -v limactl >/dev/null 2>&1; then
    limactl shell "${VM_NAME}" -- rm -rf "${VM_WORK_DIR}" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

if [[ "$(uname -s)" != "Darwin" || "$(uname -m)" != "arm64" ]]; then
  echo "ERROR: rollback compatibility local producer requires macOS arm64" >&2
  exit 1
fi
if ! command -v limactl >/dev/null 2>&1; then
  echo "ERROR: limactl is required for Linux x86 rollback build" >&2
  exit 1
fi

vm_status="$(limactl list "${VM_NAME}" --format '{{.Status}}' 2>/dev/null || true)"
if [[ "${vm_status}" != "Running" ]]; then
  limactl start --tty=false "${VM_NAME}"
fi

rm -rf "${HOST_WORK_DIR}" "${OUTPUT_DIR}"
mkdir -p "${HOST_WORK_DIR}/dist" "${OUTPUT_DIR}"

CARGO_TARGET_DIR="${HOST_WORK_DIR}/target" \
  DIST_DIR="${HOST_WORK_DIR}/dist" \
  TARGET="aarch64-apple-darwin" \
  VERSION="${ROLLBACK_VERSION}" \
  NATIVE_ONLY_RELEASE=0 \
  bash scripts/release.sh
mac_archive="${HOST_WORK_DIR}/dist/lsharp-${ROLLBACK_VERSION}-aarch64-apple-darwin-host-launcher.tar.gz"
verify_rollback_manifest "${mac_archive}" "aarch64-apple-darwin"
WORK_DIR="${HOST_WORK_DIR}/smoke-macos" \
  bash scripts/ci/release-smoke.sh "${mac_archive}"
cp "${mac_archive}" "${OUTPUT_DIR}/"

limactl shell "${VM_NAME}" -- rm -rf "${VM_WORK_DIR}"
limactl shell "${VM_NAME}" -- mkdir -p "${VM_WORK_DIR}/dist"
quoted_root="$(printf '%q' "${ROOT_DIR}")"
quoted_vm_work="$(printf '%q' "${VM_WORK_DIR}")"
quoted_version="$(printf '%q' "${ROLLBACK_VERSION}")"
limactl shell "${VM_NAME}" -- bash -lc \
  "cd ${quoted_root} && CARGO_TARGET_DIR=${quoted_vm_work}/target DIST_DIR=${quoted_vm_work}/dist TARGET=x86_64-unknown-linux-gnu VERSION=${quoted_version} NATIVE_ONLY_RELEASE=0 bash scripts/release.sh"

linux_archive_name="lsharp-${ROLLBACK_VERSION}-x86_64-unknown-linux-gnu-host-launcher.tar.gz"
limactl copy \
  "${VM_NAME}:${VM_WORK_DIR}/dist/${linux_archive_name}" \
  "${OUTPUT_DIR}/${linux_archive_name}"
verify_rollback_manifest \
  "${OUTPUT_DIR}/${linux_archive_name}" \
  "x86_64-unknown-linux-gnu"
limactl copy scripts/ci/release-smoke.sh "${VM_NAME}:${VM_WORK_DIR}/release-smoke.sh"
limactl shell "${VM_NAME}" -- env \
  WORK_DIR="${VM_WORK_DIR}/smoke-linux" \
  bash "${VM_WORK_DIR}/release-smoke.sh" \
    "${VM_WORK_DIR}/dist/${linux_archive_name}"

rollback_kib="$(du -sk "${OUTPUT_DIR}" | awk '{print $1}')"
if (( rollback_kib > MAX_ROLLBACK_KIB )); then
  echo "ERROR: rollback compatibility archives are too large: ${rollback_kib} KiB > ${MAX_ROLLBACK_KIB} KiB" >&2
  exit 1
fi

echo "rollback compatibility archives: ${OUTPUT_DIR} (${rollback_kib} KiB)"
