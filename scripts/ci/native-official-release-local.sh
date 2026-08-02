#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${ROOT_DIR}"

CURRENT_SOURCE_COMMIT="$(git rev-parse --verify HEAD 2>/dev/null || true)"
if [[ ! "${CURRENT_SOURCE_COMMIT}" =~ ^[0-9a-f]{40}$ ]]; then
  echo "ERROR: current checkout source_commit is unavailable: ${CURRENT_SOURCE_COMMIT}" >&2
  exit 1
fi
SOURCE_COMMIT="${SOURCE_COMMIT:-${CURRENT_SOURCE_COMMIT}}"
if [[ ! "${SOURCE_COMMIT}" =~ ^[0-9a-f]{40}$ ]]; then
  echo "ERROR: SOURCE_COMMIT must be a 40-character lowercase hexadecimal commit: ${SOURCE_COMMIT}" >&2
  exit 1
fi
if [[ "${SOURCE_COMMIT}" != "${CURRENT_SOURCE_COMMIT}" ]]; then
  echo "ERROR: SOURCE_COMMIT must match current checkout HEAD: expected=${CURRENT_SOURCE_COMMIT} actual=${SOURCE_COMMIT}" >&2
  exit 1
fi
VERSION="${VERSION:-}"
if [[ -z "${VERSION}" ]]; then
  command -v cargo >/dev/null 2>&1 \
    || { echo "ERROR: VERSION is required when cargo is unavailable" >&2; exit 1; }
  PACKAGE_VERSION="$(cargo metadata --no-deps --format-version 1 | python3 -c 'import json,sys; data=json.load(sys.stdin); print(next(p["version"] for p in data["packages"] if p["name"] == "lsharp-wasm"))')"
  VERSION="v${PACKAGE_VERSION}"
fi
DIST_DIR="${DIST_DIR:-${ROOT_DIR}/dist/native-official}"
MACOS_APP_CLI_ARTIFACT_DIR="${MACOS_APP_CLI_ARTIFACT_DIR:-}"
LINUX_APP_CLI_ARTIFACT_DIR="${LINUX_APP_CLI_ARTIFACT_DIR:-}"
MACOS_STAGE0_DIR="${MACOS_STAGE0_DIR:-}"
LINUX_STAGE0_DIR="${LINUX_STAGE0_DIR:-}"
MACOS_ROLLBACK_ARCHIVE="${MACOS_ROLLBACK_ARCHIVE:-}"
LINUX_ROLLBACK_ARCHIVE="${LINUX_ROLLBACK_ARCHIVE:-}"
NATIVE_OFFICIAL_REVIEW_TRUST_STORE="${NATIVE_OFFICIAL_REVIEW_TRUST_STORE:-}"
NATIVE_OFFICIAL_REVIEW_LIFECYCLE="${NATIVE_OFFICIAL_REVIEW_LIFECYCLE:-}"
NATIVE_OFFICIAL_REVIEW_VERIFICATION_NOW="${NATIVE_OFFICIAL_REVIEW_VERIFICATION_NOW:-}"
NATIVE_OFFICIAL_SOURCE_SMOKE_EVIDENCE_ROOT="${NATIVE_OFFICIAL_SOURCE_SMOKE_EVIDENCE_ROOT:-}"
NATIVE_OFFICIAL_REVIEW_ATTESTATION_REPORT="${NATIVE_OFFICIAL_REVIEW_ATTESTATION_REPORT:-}"
SMOKE_ROOT="${LSHARP_NATIVE_RELEASE_SMOKE_ROOT:-/tmp/lsharp-native-official-release-smoke}"
MAX_DIST_KIB="${LSHARP_NATIVE_RELEASE_MAX_DIST_KIB:-1048576}"
VM_NAME="${LSHARP_NATIVE_LINUX_X86_VM_NAME:-lsharp-linux-x86}"
HOSTGEN_REPLAY_LOCK_DIR="${LSHARP_NATIVE_LINUX_X86_HOST_REPLAY_LOCK_DIR:-/tmp/lsharp-native-linux-x86-hostgen-vm-${VM_NAME}.lock}"

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
  case "${path}" in
    */../*|*/..|*/./*|*/.)
      echo "ERROR: refusing unsafe cleanup path with traversal component for ${label}: ${path}" >&2
      exit 1
      ;;
  esac
}

require_safe_cleanup_path "${SMOKE_ROOT}" "release smoke root"
require_safe_cleanup_path "${HOSTGEN_REPLAY_LOCK_DIR}" "Linux hostgen replay lock"

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

preflight_linux_hostgen_replay_lock() {
  [[ ! -e "${HOSTGEN_REPLAY_LOCK_DIR}" ]] && return 0

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

preflight_linux_hostgen_replay_lock

validate_review_snapshots() {
  if [[ -z "${NATIVE_OFFICIAL_REVIEW_TRUST_STORE}" && -z "${NATIVE_OFFICIAL_REVIEW_LIFECYCLE}" ]]; then
    [[ -z "${NATIVE_OFFICIAL_REVIEW_VERIFICATION_NOW}" ]] \
      || { echo "ERROR: NATIVE_OFFICIAL_REVIEW_VERIFICATION_NOW requires provider snapshots" >&2; exit 1; }
    return 0
  fi
  if [[ -z "${NATIVE_OFFICIAL_REVIEW_TRUST_STORE}" || -z "${NATIVE_OFFICIAL_REVIEW_LIFECYCLE}" ]]; then
    echo "ERROR: NATIVE_OFFICIAL_REVIEW_TRUST_STORE and NATIVE_OFFICIAL_REVIEW_LIFECYCLE must be supplied together" >&2
    exit 1
  fi
  if [[ ! -f "${NATIVE_OFFICIAL_REVIEW_TRUST_STORE}" || ! -s "${NATIVE_OFFICIAL_REVIEW_TRUST_STORE}" ]]; then
    echo "ERROR: native official review trust-store snapshot is not a non-empty file: ${NATIVE_OFFICIAL_REVIEW_TRUST_STORE}" >&2
    exit 1
  fi
  if [[ ! -f "${NATIVE_OFFICIAL_REVIEW_LIFECYCLE}" || ! -s "${NATIVE_OFFICIAL_REVIEW_LIFECYCLE}" ]]; then
    echo "ERROR: native official review lifecycle snapshot is not a non-empty file: ${NATIVE_OFFICIAL_REVIEW_LIFECYCLE}" >&2
    exit 1
  fi
}

validate_review_identity_inputs() {
  if [[ -z "${NATIVE_OFFICIAL_REVIEW_TRUST_STORE}" && -z "${NATIVE_OFFICIAL_REVIEW_LIFECYCLE}" ]]; then
    return 0
  fi

  local identity_path
  for identity_path in \
    "${MACOS_APP_CLI_ARTIFACT_DIR}/review-evidence-identity.json" \
    "${LINUX_APP_CLI_ARTIFACT_DIR}/review-evidence-identity.json" \
    "${MACOS_STAGE0_DIR}/review-evidence-identity.json" \
    "${LINUX_STAGE0_DIR}/review-evidence-identity.json"; do
    if [[ ! -f "${identity_path}" || ! -s "${identity_path}" ]]; then
      echo "ERROR: review evidence identity is required when provider snapshots are supplied: ${identity_path}" >&2
      exit 1
    fi
    python3 - "${identity_path}" "${CURRENT_SOURCE_COMMIT}" <<'PY'
import json
import pathlib
import sys

identity_path = pathlib.Path(sys.argv[1])
expected_source_commit = sys.argv[2]
try:
    identity = json.loads(identity_path.read_text(encoding="utf-8"))
except (OSError, json.JSONDecodeError) as error:
    raise SystemExit(
        f"ERROR: review evidence identity is invalid JSON: {identity_path}: {error}"
    )
if not isinstance(identity, dict):
    raise SystemExit(
        f"ERROR: review evidence identity must be a JSON object: {identity_path}"
    )
actual_source_commit = identity.get("source_commit")
if actual_source_commit != expected_source_commit:
    raise SystemExit(
        "ERROR: review evidence identity source_commit mismatch: "
        f"expected={expected_source_commit} actual={actual_source_commit!r}: {identity_path}"
    )
PY
    verifier_args=(
      --identity "${identity_path}"
      --source-commit "${CURRENT_SOURCE_COMMIT}"
      --trust-store "${NATIVE_OFFICIAL_REVIEW_TRUST_STORE}"
      --review-lifecycle "${NATIVE_OFFICIAL_REVIEW_LIFECYCLE}"
      --require-provider-input
    )
    artifact_path="$(dirname "${identity_path}")/program.native"
    if [[ -e "${artifact_path}" ]]; then
      verifier_args+=(--artifact "${artifact_path}")
    fi
    if [[ -n "${NATIVE_OFFICIAL_REVIEW_VERIFICATION_NOW}" ]]; then
      verifier_args+=(
        --verification-now "${NATIVE_OFFICIAL_REVIEW_VERIFICATION_NOW}"
      )
    fi
    python3 "${ROOT_DIR}/scripts/ci/verify-native-release-identity.py" \
      "${verifier_args[@]}" \
      >/dev/null
  done
}

validate_source_smoke_evidence_root() {
  [[ -z "${NATIVE_OFFICIAL_SOURCE_SMOKE_EVIDENCE_ROOT}" ]] && return 0
  [[ "${NATIVE_OFFICIAL_SOURCE_SMOKE_EVIDENCE_ROOT}" = /* && "${NATIVE_OFFICIAL_SOURCE_SMOKE_EVIDENCE_ROOT}" != "/" ]] \
    || { echo "ERROR: NATIVE_OFFICIAL_SOURCE_SMOKE_EVIDENCE_ROOT must be an absolute non-root path" >&2; exit 1; }
  [[ ! -L "${NATIVE_OFFICIAL_SOURCE_SMOKE_EVIDENCE_ROOT}" ]] \
    || { echo "ERROR: source smoke evidence root must not be a symlink: ${NATIVE_OFFICIAL_SOURCE_SMOKE_EVIDENCE_ROOT}" >&2; exit 1; }
  NATIVE_OFFICIAL_SOURCE_SMOKE_EVIDENCE_ROOT="$(python3 - \
    "${ROOT_DIR}" \
    "${SMOKE_ROOT}" \
    "${NATIVE_OFFICIAL_SOURCE_SMOKE_EVIDENCE_ROOT}" <<'PY'
from pathlib import Path
import sys

root = Path(sys.argv[1]).resolve()
smoke_root = Path(sys.argv[2]).resolve()
raw_evidence_root = sys.argv[3]
evidence_root = Path(raw_evidence_root).resolve()
protected_paths = {
    Path("/"),
    Path("/tmp"),
    Path("/private/tmp"),
    root,
    root / "target",
    root / "target" / "ci",
    root / "ci-artifacts",
    root / "dist",
    root / "stage0",
}
if evidence_root in protected_paths:
    raise SystemExit(
        "ERROR: source smoke evidence root is a protected shared path: "
        f"{raw_evidence_root}"
    )
try:
    evidence_root.relative_to(smoke_root)
except ValueError:
    pass
else:
    raise SystemExit(
        "ERROR: source smoke evidence root must not be inside the cleaned release smoke root: "
        f"{raw_evidence_root}"
    )
print(evidence_root)
PY
)"
  if [[ -e "${NATIVE_OFFICIAL_SOURCE_SMOKE_EVIDENCE_ROOT}" && ! -d "${NATIVE_OFFICIAL_SOURCE_SMOKE_EVIDENCE_ROOT}" ]]; then
    echo "ERROR: source smoke evidence root is not a directory: ${NATIVE_OFFICIAL_SOURCE_SMOKE_EVIDENCE_ROOT}" >&2
    exit 1
  fi
}

validate_review_attestation_report() {
  [[ -z "${NATIVE_OFFICIAL_REVIEW_ATTESTATION_REPORT}" ]] && return 0
  [[ -n "${NATIVE_OFFICIAL_SOURCE_SMOKE_EVIDENCE_ROOT}" ]] \
    || { echo "ERROR: explicit review attestation report requires source smoke evidence root" >&2; exit 1; }
  [[ -f "${NATIVE_OFFICIAL_REVIEW_ATTESTATION_REPORT}" \
    && ! -L "${NATIVE_OFFICIAL_REVIEW_ATTESTATION_REPORT}" \
    && -s "${NATIVE_OFFICIAL_REVIEW_ATTESTATION_REPORT}" ]] \
    || { echo "ERROR: review attestation report must be a non-empty regular file: ${NATIVE_OFFICIAL_REVIEW_ATTESTATION_REPORT}" >&2; exit 1; }
  python3 - "${NATIVE_OFFICIAL_REVIEW_ATTESTATION_REPORT}" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
try:
    report = json.loads(path.read_text(encoding="utf-8"))
except (OSError, json.JSONDecodeError) as error:
    raise SystemExit(f"ERROR: review attestation report is invalid JSON: {path}: {error}")
if not isinstance(report, dict) or not isinstance(report.get("review_attestations"), list):
    raise SystemExit(
        "ERROR: review attestation report must be a JSON object with review_attestations list: "
        f"{path}"
    )
PY
}

validate_source_smoke_evidence_projection() {
  local target="$1"
  local evidence_dir="$2"
  local manifest_path="${evidence_dir}/manifest.json"
  [[ -n "${evidence_dir}" ]] || return 0
  [[ -d "${evidence_dir}" && ! -L "${evidence_dir}" ]] \
    || { echo "ERROR: source smoke evidence directory is unavailable for ${target}: ${evidence_dir}" >&2; exit 1; }
  [[ -f "${manifest_path}" && ! -L "${manifest_path}" && -s "${manifest_path}" ]] \
    || { echo "ERROR: source smoke evidence manifest is unavailable for ${target}: ${manifest_path}" >&2; exit 1; }
  python3 - "${manifest_path}" "${NATIVE_OFFICIAL_REVIEW_ATTESTATION_REPORT}" "${target}" <<'PY'
import json
import pathlib
import sys

manifest_path = pathlib.Path(sys.argv[1])
report_path = pathlib.Path(sys.argv[2]) if sys.argv[2] else None
target = sys.argv[3]
try:
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
except (OSError, json.JSONDecodeError) as error:
    raise SystemExit(f"ERROR: source smoke evidence manifest is invalid JSON for {target}: {manifest_path}: {error}")
if not isinstance(manifest, dict):
    raise SystemExit(f"ERROR: source smoke evidence manifest must be a JSON object for {target}: {manifest_path}")
if report_path:
    try:
        report = json.loads(report_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise SystemExit(f"ERROR: review attestation report is unreadable during postflight: {report_path}: {error}")
    expected = report.get("review_attestations") if isinstance(report, dict) else None
    if manifest.get("review_attestations") != expected:
        raise SystemExit(
            "ERROR: source smoke evidence review_attestations mismatch: "
            f"target={target} manifest={manifest_path} report={report_path}"
        )
elif "review_attestations" in manifest:
    raise SystemExit(
        "ERROR: source smoke evidence contains implicit review_attestations without explicit report: "
        f"target={target} manifest={manifest_path}"
    )
PY
}

NATIVE_OFFICIAL_REVIEW_ENV=()
validate_review_snapshots
validate_review_identity_inputs
validate_source_smoke_evidence_root
validate_review_attestation_report
if [[ -n "${NATIVE_OFFICIAL_REVIEW_TRUST_STORE}" ]]; then
  NATIVE_OFFICIAL_REVIEW_ENV=(
    "NATIVE_ONLY_REVIEW_TRUST_STORE=${NATIVE_OFFICIAL_REVIEW_TRUST_STORE}"
    "NATIVE_ONLY_REVIEW_LIFECYCLE=${NATIVE_OFFICIAL_REVIEW_LIFECYCLE}"
  )
fi

smoke_archive() {
  local target="$1"
  local archive_path="$2"
  local rollback_archive="$3"
  if [[ "${target}" != "x86_64-unknown-linux-gnu" ]]; then
    local smoke_env=("WORK_DIR=${SMOKE_ROOT}/${target}")
    if [[ -n "${NATIVE_OFFICIAL_REVIEW_TRUST_STORE}" ]]; then
      smoke_env+=(
        "RELEASE_REVIEW_TRUST_STORE=${NATIVE_OFFICIAL_REVIEW_TRUST_STORE}"
        "RELEASE_REVIEW_LIFECYCLE=${NATIVE_OFFICIAL_REVIEW_LIFECYCLE}"
      )
    fi
    env "${smoke_env[@]}" \
      bash scripts/ci/release-smoke.sh "${archive_path}" "${rollback_archive}"
    return
  fi

  if ! command -v limactl >/dev/null 2>&1; then
    echo "ERROR: limactl is required for Linux x86 release smoke" >&2
    exit 1
  fi
  vm_status="$(limactl list "${VM_NAME}" --format '{{.Status}}' 2>/dev/null || true)"
  local vm_started_by_gate=0
  if [[ "${vm_status}" != "Running" ]]; then
    limactl start --tty=false "${VM_NAME}"
    vm_started_by_gate=1
  fi
  local vm_work_dir="/tmp/lsharp-native-official-release-smoke-$$"
  local archive_name="$(basename "${archive_path}")"
  local rollback_name="$(basename "${rollback_archive}")"
  set +e
  (
    set -euo pipefail
    cleanup_vm_work_dir() {
      limactl shell "${VM_NAME}" -- rm -rf "${vm_work_dir}" >/dev/null 2>&1
    }
    cleanup_vm_resources() {
      local exit_status=$?
      local cleanup_status=0
      if ! cleanup_vm_work_dir; then
        cleanup_status=1
      fi
      if [[ "${vm_started_by_gate}" == "1" ]]; then
        if ! limactl stop "${VM_NAME}" >/dev/null 2>&1; then
          cleanup_status=1
        fi
      fi
      if [[ "${cleanup_status}" -ne 0 ]]; then
        echo "ERROR: Linux x86 release smoke cleanup failed in Lima VM" >&2
      fi
      if [[ "${exit_status}" -ne 0 ]]; then
        exit "${exit_status}"
      fi
      if [[ "${cleanup_status}" -ne 0 ]]; then
        exit 1
      fi
      exit 0
    }
    trap cleanup_vm_resources EXIT

    limactl shell "${VM_NAME}" -- rm -rf "${vm_work_dir}"
    limactl shell "${VM_NAME}" -- mkdir -p "${vm_work_dir}"
    limactl copy "${archive_path}" "${VM_NAME}:${vm_work_dir}/${archive_name}"
    limactl copy "${rollback_archive}" "${VM_NAME}:${vm_work_dir}/${rollback_name}"
    limactl copy scripts/ci/release-smoke.sh "${VM_NAME}:${vm_work_dir}/release-smoke.sh"
    limactl copy scripts/ci/verify-native-release-identity.py \
      "${VM_NAME}:${vm_work_dir}/verify-native-release-identity.py"
    limactl copy scripts/ci/review_identity_timestamp.py \
      "${VM_NAME}:${vm_work_dir}/review_identity_timestamp.py"
    linux_smoke_env=(
      "WORK_DIR=${vm_work_dir}/work"
      "RELEASE_IDENTITY_VERIFIER=${vm_work_dir}/verify-native-release-identity.py"
    )
    if [[ -n "${NATIVE_OFFICIAL_REVIEW_TRUST_STORE}" ]]; then
      limactl copy "${NATIVE_OFFICIAL_REVIEW_TRUST_STORE}" \
        "${VM_NAME}:${vm_work_dir}/review-trust-store.snapshot"
      limactl copy "${NATIVE_OFFICIAL_REVIEW_LIFECYCLE}" \
        "${VM_NAME}:${vm_work_dir}/review-lifecycle.snapshot"
      linux_smoke_env+=(
        "RELEASE_REVIEW_TRUST_STORE=${vm_work_dir}/review-trust-store.snapshot"
        "RELEASE_REVIEW_LIFECYCLE=${vm_work_dir}/review-lifecycle.snapshot"
      )
    fi
    set +e
    limactl shell "${VM_NAME}" -- env "${linux_smoke_env[@]}" \
      bash "${vm_work_dir}/release-smoke.sh" \
        "${vm_work_dir}/${archive_name}" \
        "${vm_work_dir}/${rollback_name}"
    smoke_status=$?
    set -e
    exit "${smoke_status}"
  )
  smoke_status=$?
  set -e
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
  # review_evidence_identity は producer の explicit input がある場合だけ伝播する。
  local identity_path="${artifact_dir}/review-evidence-identity.json"
  local archive_base="lsharp-${VERSION}-${target}"
  local archive_path="${DIST_DIR}/${archive_base}.tar.gz"

  require_file "${program_path}" "${target} actual App.Cli program"
  require_file "${manifest_path}" "${target} actual App.Cli manifest"
  require_file "${rollback_archive}" "${target} rollback compatibility archive"

  rm -rf "${DIST_DIR:?}/${archive_base}" "${archive_path}"
  if [[ -s "${identity_path}" ]]; then
    env "${NATIVE_OFFICIAL_REVIEW_ENV[@]}" \
      NATIVE_ONLY_REVIEW_EVIDENCE_IDENTITY="${identity_path}" \
      TARGET="${target}" \
      VERSION="${VERSION}" \
      SOURCE_COMMIT="${SOURCE_COMMIT}" \
      DIST_DIR="${DIST_DIR}" \
      NATIVE_ONLY_RELEASE=1 \
      NATIVE_ONLY_PROGRAM="${program_path}" \
      NATIVE_ONLY_PROGRAM_MANIFEST="${manifest_path}" \
      ROLLBACK_COMPATIBILITY_ASSET_PATH="${rollback_archive}" \
      bash scripts/release.sh
  else
    env "${NATIVE_OFFICIAL_REVIEW_ENV[@]}" \
      TARGET="${target}" \
      VERSION="${VERSION}" \
      SOURCE_COMMIT="${SOURCE_COMMIT}" \
      DIST_DIR="${DIST_DIR}" \
      NATIVE_ONLY_RELEASE=1 \
      NATIVE_ONLY_PROGRAM="${program_path}" \
      NATIVE_ONLY_PROGRAM_MANIFEST="${manifest_path}" \
      ROLLBACK_COMPATIBILITY_ASSET_PATH="${rollback_archive}" \
      bash scripts/release.sh
  fi

  require_file "${archive_path}" "${target} native-only archive"
  smoke_archive "${target}" "${archive_path}" "${rollback_archive}"
}

package_stage0_target() {
  local target="$1"
  local stage0_dir="$2"
  local archive_path="${DIST_DIR}/lsharp-stage0-${VERSION}-${target}.tar.gz"
  local identity_path="${stage0_dir}/review-evidence-identity.json"
  local identity_args=()

  [[ -d "${stage0_dir}" ]] \
    || { echo "ERROR: ${target} native stage0 directory is required: ${stage0_dir}" >&2; exit 1; }
  if [[ -s "${identity_path}" ]]; then
    identity_args+=(--review-evidence-identity "${identity_path}")
  fi
  if [[ -n "${NATIVE_OFFICIAL_REVIEW_TRUST_STORE}" ]]; then
    identity_args+=(
      --review-trust-store "${NATIVE_OFFICIAL_REVIEW_TRUST_STORE}"
      --review-lifecycle "${NATIVE_OFFICIAL_REVIEW_LIFECYCLE}"
    )
  fi
  rm -f "${archive_path}"
  bash scripts/ci/package-native-stage0-release.sh \
    --target "${target}" \
    --version "${VERSION}" \
    --stage0-dir "${stage0_dir}" \
    --source-commit "${SOURCE_COMMIT}" \
    --output-dir "${DIST_DIR}" \
    "${identity_args[@]}"
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

smoke_stage0_runtime() {
  local target="$1"
  local stage0_dir="${SMOKE_ROOT}/stage0-${target}"
  local evidence_dir=""
  if [[ -n "${NATIVE_OFFICIAL_SOURCE_SMOKE_EVIDENCE_ROOT}" ]]; then
    evidence_dir="${NATIVE_OFFICIAL_SOURCE_SMOKE_EVIDENCE_ROOT}/${target}"
    [[ ! -e "${evidence_dir}" && ! -L "${evidence_dir}" ]] \
      || { echo "ERROR: source smoke evidence directory already exists: ${evidence_dir}" >&2; exit 1; }
  fi

  case "${target}" in
    aarch64-apple-darwin)
      NATIVE_SELFHOST_SOURCE_SMOKE_EVIDENCE_DIR="${evidence_dir}" \
        NATIVE_SELFHOST_REVIEW_ATTESTATION_REPORT="${NATIVE_OFFICIAL_REVIEW_ATTESTATION_REPORT}" \
        NATIVE_STAGE0_DIR="${stage0_dir}" \
        NATIVE_SELFHOST_SOURCE_ROOT="${ROOT_DIR}/selfhost" \
        bash scripts/ci/native-selfhost-dev-source-file-smoke.sh
      ;;
    x86_64-unknown-linux-gnu)
      LSHARP_NATIVE_LINUX_X86_SOURCE_SMOKE_EVIDENCE_DIR="${evidence_dir}" \
        LSHARP_NATIVE_LINUX_X86_REVIEW_ATTESTATION_REPORT="${NATIVE_OFFICIAL_REVIEW_ATTESTATION_REPORT}" \
        LSHARP_NATIVE_LINUX_X86_STAGE0_DIR="${stage0_dir}" \
        bash scripts/ci/native-linux-x86-native-stage0-source-file-smoke.sh
      ;;
    *)
      echo "ERROR: unsupported native stage0 runtime target: ${target}" >&2
      exit 1
      ;;
  esac
  validate_source_smoke_evidence_projection "${target}" "${evidence_dir}"
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
smoke_stage0_runtime "aarch64-apple-darwin"
smoke_stage0_fetch "x86_64-unknown-linux-gnu" "${RELEASE_BASE_URL}"
smoke_stage0_runtime "x86_64-unknown-linux-gnu"

dist_kib="$(du -sk "${DIST_DIR}" | awk '{print $1}')"
if (( dist_kib > MAX_DIST_KIB )); then
  echo "ERROR: native official release output is too large: ${dist_kib} KiB > ${MAX_DIST_KIB} KiB" >&2
  exit 1
fi

echo "native official release local gate: OK (${dist_kib} KiB)"
