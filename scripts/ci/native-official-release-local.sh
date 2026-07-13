#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${ROOT_DIR}"

VERSION="${VERSION:-}"
if [[ -z "${VERSION}" ]]; then
  command -v cargo >/dev/null 2>&1 \
    || { echo "ERROR: VERSION is required when cargo is unavailable" >&2; exit 1; }
  PACKAGE_VERSION="$(cargo metadata --no-deps --format-version 1 | python3 -c 'import json,sys; data=json.load(sys.stdin); print(next(p["version"] for p in data["packages"] if p["name"] == "lsharp-wasm"))')"
  VERSION="v${PACKAGE_VERSION}"
fi
SOURCE_COMMIT="${SOURCE_COMMIT:-$(git rev-parse HEAD)}"
DIST_DIR="${DIST_DIR:-${ROOT_DIR}/dist/native-official}"
MACOS_APP_CLI_ARTIFACT_DIR="${MACOS_APP_CLI_ARTIFACT_DIR:-}"
LINUX_APP_CLI_ARTIFACT_DIR="${LINUX_APP_CLI_ARTIFACT_DIR:-}"
MACOS_STAGE0_DIR="${MACOS_STAGE0_DIR:-}"
LINUX_STAGE0_DIR="${LINUX_STAGE0_DIR:-}"
MACOS_ROLLBACK_ARCHIVE="${MACOS_ROLLBACK_ARCHIVE:-}"
LINUX_ROLLBACK_ARCHIVE="${LINUX_ROLLBACK_ARCHIVE:-}"
SMOKE_ROOT="${LSHARP_NATIVE_RELEASE_SMOKE_ROOT:-/tmp/lsharp-native-official-release-smoke}"
MAX_DIST_KIB="${LSHARP_NATIVE_RELEASE_MAX_DIST_KIB:-1048576}"
VM_NAME="${LSHARP_NATIVE_LINUX_X86_VM_NAME:-lsharp-linux-x86}"

require_safe_cleanup_path() {
  local path="$1"
  local label="$2"
  case "${path}" in
    /tmp/lsharp-*) ;;
    *)
      echo "ERROR: refusing unsafe cleanup path for ${label}: ${path}" >&2
      exit 1
      ;;
  esac
}

require_safe_cleanup_path "${SMOKE_ROOT}" "release smoke root"

cleanup() {
  if [[ "${KEEP_WORK_DIR:-0}" != "1" ]]; then
    rm -rf "${SMOKE_ROOT}"
  fi
}
trap cleanup EXIT

require_file() {
  local path="$1"
  local description="$2"
  if [[ ! -s "${path}" ]]; then
    echo "ERROR: ${description} is required: ${path}" >&2
    exit 1
  fi
}

smoke_archive() {
  local target="$1"
  local archive_path="$2"
  local rollback_archive="$3"
  if [[ "${target}" != "x86_64-unknown-linux-gnu" ]]; then
    WORK_DIR="${SMOKE_ROOT}/${target}" \
      bash scripts/ci/release-smoke.sh "${archive_path}" "${rollback_archive}"
    return
  fi

  if ! command -v limactl >/dev/null 2>&1; then
    echo "ERROR: limactl is required for Linux x86 release smoke" >&2
    exit 1
  fi
  vm_status="$(limactl list "${VM_NAME}" --format '{{.Status}}' 2>/dev/null || true)"
  if [[ "${vm_status}" != "Running" ]]; then
    limactl start --tty=false "${VM_NAME}"
  fi
  vm_work_dir="/tmp/lsharp-native-official-release-smoke-$$"
  archive_name="$(basename "${archive_path}")"
  rollback_name="$(basename "${rollback_archive}")"
  limactl shell "${VM_NAME}" -- rm -rf "${vm_work_dir}"
  limactl shell "${VM_NAME}" -- mkdir -p "${vm_work_dir}"
  limactl copy "${archive_path}" "${VM_NAME}:${vm_work_dir}/${archive_name}"
  limactl copy "${rollback_archive}" "${VM_NAME}:${vm_work_dir}/${rollback_name}"
  limactl copy scripts/ci/release-smoke.sh "${VM_NAME}:${vm_work_dir}/release-smoke.sh"
  set +e
  limactl shell "${VM_NAME}" -- env \
    WORK_DIR="${vm_work_dir}/work" \
    bash "${vm_work_dir}/release-smoke.sh" \
      "${vm_work_dir}/${archive_name}" \
      "${vm_work_dir}/${rollback_name}"
  smoke_status=$?
  set -e
  limactl shell "${VM_NAME}" -- rm -rf "${vm_work_dir}"
  if [[ "${smoke_status}" -ne 0 ]]; then
    echo "ERROR: Linux x86 release smoke failed in Lima VM" >&2
    exit "${smoke_status}"
  fi
}

package_target() {
  local target="$1"
  local artifact_dir="$2"
  local rollback_archive="$3"
  local program_path="${artifact_dir}/program.native"
  local manifest_path="${artifact_dir}/manifest.json"
  local archive_base="lsharp-${VERSION}-${target}"
  local archive_path="${DIST_DIR}/${archive_base}.tar.gz"

  require_file "${program_path}" "${target} actual App.Cli program"
  require_file "${manifest_path}" "${target} actual App.Cli manifest"
  require_file "${rollback_archive}" "${target} rollback compatibility archive"

  rm -rf "${DIST_DIR:?}/${archive_base}" "${archive_path}"
  TARGET="${target}" \
    VERSION="${VERSION}" \
    SOURCE_COMMIT="${SOURCE_COMMIT}" \
    DIST_DIR="${DIST_DIR}" \
    NATIVE_ONLY_RELEASE=1 \
    NATIVE_ONLY_PROGRAM="${program_path}" \
    NATIVE_ONLY_PROGRAM_MANIFEST="${manifest_path}" \
    ROLLBACK_COMPATIBILITY_ASSET_PATH="${rollback_archive}" \
    bash scripts/release.sh

  require_file "${archive_path}" "${target} native-only archive"
  smoke_archive "${target}" "${archive_path}" "${rollback_archive}"
}

package_stage0_target() {
  local target="$1"
  local stage0_dir="$2"
  local archive_path="${DIST_DIR}/lsharp-stage0-${VERSION}-${target}.tar.gz"

  [[ -d "${stage0_dir}" ]] \
    || { echo "ERROR: ${target} native stage0 directory is required: ${stage0_dir}" >&2; exit 1; }
  rm -f "${archive_path}"
  bash scripts/ci/package-native-stage0-release.sh \
    --target "${target}" \
    --version "${VERSION}" \
    --stage0-dir "${stage0_dir}" \
    --output-dir "${DIST_DIR}"
  require_file "${archive_path}" "${target} native stage0 archive"
}

release_base_url() {
  python3 - "${DIST_DIR}" <<'PY'
from pathlib import Path
import sys

print(Path(sys.argv[1]).resolve().as_uri())
PY
}

smoke_stage0_fetch() {
  local target="$1"
  local stage0_dir="${SMOKE_ROOT}/stage0-${target}"
  local base_url="$2"

  rm -rf "${stage0_dir}"
  STAGE0_VERSION="${VERSION}" \
    STAGE0_TARGET="${target}" \
    STAGE0_RELEASE_BASE_URL="${base_url}" \
    STAGE0_DIR="${stage0_dir}" \
    bash scripts/fetch-stage0.sh
  require_file "${stage0_dir}/manifest.json" "${target} fetched native stage0 manifest"
}

mkdir -p "${DIST_DIR}"
rm -rf "${SMOKE_ROOT}"

package_target \
  "aarch64-apple-darwin" \
  "${MACOS_APP_CLI_ARTIFACT_DIR}" \
  "${MACOS_ROLLBACK_ARCHIVE}"
package_target \
  "x86_64-unknown-linux-gnu" \
  "${LINUX_APP_CLI_ARTIFACT_DIR}" \
  "${LINUX_ROLLBACK_ARCHIVE}"
package_stage0_target "aarch64-apple-darwin" "${MACOS_STAGE0_DIR}"
package_stage0_target "x86_64-unknown-linux-gnu" "${LINUX_STAGE0_DIR}"

bash scripts/checksum.sh "${DIST_DIR}" > "${DIST_DIR}/checksums.txt"
RELEASE_BASE_URL="$(release_base_url)"
smoke_stage0_fetch "aarch64-apple-darwin" "${RELEASE_BASE_URL}"
smoke_stage0_fetch "x86_64-unknown-linux-gnu" "${RELEASE_BASE_URL}"

dist_kib="$(du -sk "${DIST_DIR}" | awk '{print $1}')"
if (( dist_kib > MAX_DIST_KIB )); then
  echo "ERROR: native official release output is too large: ${dist_kib} KiB > ${MAX_DIST_KIB} KiB" >&2
  exit 1
fi

echo "native official release local gate: OK (${dist_kib} KiB)"
