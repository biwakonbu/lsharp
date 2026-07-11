#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${ROOT_DIR}"

TARGET="aarch64-apple-darwin"
ARTIFACT_DIR="${LSHARP_NATIVE_MACOS_AARCH64_RELEASE_ARTIFACT_DIR:-${ROOT_DIR}/ci-artifacts/native-release/${TARGET}}"
NATIVE_RELEASE_CARGO_TARGET_DIR="${LSHARP_NATIVE_MACOS_AARCH64_CARGO_TARGET_DIR:-/tmp/lsharp-native-macos-aarch64-release-cargo-target}"
KEEP_CARGO_TARGET="${LSHARP_NATIVE_MACOS_AARCH64_KEEP_CARGO_TARGET:-0}"
MAX_ARTIFACT_KIB="${LSHARP_NATIVE_RELEASE_MAX_ARTIFACT_KIB:-524288}"
LOCK_DIR="${LSHARP_NATIVE_MACOS_AARCH64_RELEASE_LOCK_DIR:-/tmp/lsharp-native-macos-aarch64-release.lock}"
PACKAGE_VERSION="$(cargo metadata --no-deps --format-version 1 | python3 -c 'import json,sys; data=json.load(sys.stdin); print(next(p["version"] for p in data["packages"] if p["name"] == "lsharp-wasm"))')"
EXPECTED_CLI_VERSION="lsharp ${PACKAGE_VERSION}"

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

require_safe_cleanup_path "${ARTIFACT_DIR}" "Mac release artifact"
require_safe_cleanup_path "${NATIVE_RELEASE_CARGO_TARGET_DIR}" "Mac release Cargo target"

cleanup_native_release_target() {
  rmdir "${LOCK_DIR}" 2>/dev/null || true
  if [[ "${KEEP_CARGO_TARGET}" != "1" ]]; then
    rm -rf "${NATIVE_RELEASE_CARGO_TARGET_DIR}"
  fi
}
trap cleanup_native_release_target EXIT

if [[ "$(uname -s)" != "Darwin" || "$(uname -m)" != "arm64" ]]; then
  echo "ERROR: ${TARGET} release producer requires macOS arm64" >&2
  exit 1
fi

if ! mkdir "${LOCK_DIR}" 2>/dev/null; then
  echo "ERROR: another Mac actual App.Cli release producer is running: ${LOCK_DIR}" >&2
  exit 1
fi

rm -rf "${ARTIFACT_DIR}"
if ! git diff --quiet --ignore-submodules -- ||
   ! git diff --cached --quiet --ignore-submodules -- ||
   [[ -n "$(git status --porcelain --untracked-files=normal)" ]]; then
  echo "ERROR: actual App.Cli release provenance requires a clean worktree" >&2
  exit 1
fi

rm -rf "${NATIVE_RELEASE_CARGO_TARGET_DIR}"
mkdir -p "${ARTIFACT_DIR}"

LSHARP_NATIVE_MACOS_AARCH64_APP_CLI_ARTIFACT_DIR="${ARTIFACT_DIR}" \
  CARGO_TARGET_DIR="${NATIVE_RELEASE_CARGO_TARGET_DIR}" cargo test \
    -p lsharp-wasm --test e2e \
    e2e::selfhost_native_stage_chain::test_e2e_native_macos_aarch64_actual_app_cli_release_program \
    -- --exact --ignored --nocapture

PROGRAM_PATH="${ARTIFACT_DIR}/program.native"
MANIFEST_PATH="${ARTIFACT_DIR}/manifest.json"
if [[ ! -x "${PROGRAM_PATH}" || ! -s "${MANIFEST_PATH}" ]]; then
  echo "ERROR: actual App.Cli producer did not create program.native and manifest.json" >&2
  exit 1
fi

version_output="$(cd selfhost && "${PROGRAM_PATH}" --version)"
if [[ "${version_output}" != "${EXPECTED_CLI_VERSION}" ]]; then
  echo "ERROR: actual App.Cli --version mismatch: ${version_output}" >&2
  exit 1
fi

python3 - "${MANIFEST_PATH}" "${PROGRAM_PATH}" "${TARGET}" "$(git rev-parse HEAD)" <<'PY'
import hashlib
import json
import pathlib
import sys

manifest_path = pathlib.Path(sys.argv[1])
program_path = pathlib.Path(sys.argv[2])
target = sys.argv[3]
source_commit = sys.argv[4]
manifest = json.loads(manifest_path.read_text())
program_sha256 = hashlib.sha256(program_path.read_bytes()).hexdigest()

expected = {
    "status": "pass",
    "artifact_kind": "native App.Cli release program",
    "target": target,
    "entry_module": "App.Cli",
    "source": "src/App/Cli.ls",
    "source_commit": source_commit,
    "selfhost_fixed_point": True,
    "program_sha256": program_sha256,
}
for key, value in expected.items():
    if manifest.get(key) != value:
        raise SystemExit(f"native App.Cli manifest mismatch: {key}")
PY

tar -czf "${ARTIFACT_DIR}/native-input-bundle.tar.gz" \
  -C "${ARTIFACT_DIR}" \
  program.native manifest.json smoke-stdout.txt smoke-stderr.txt

artifact_kib="$(du -sk "${ARTIFACT_DIR}" | awk '{print $1}')"
if (( artifact_kib > MAX_ARTIFACT_KIB )); then
  echo "ERROR: native release artifact is too large: ${artifact_kib} KiB > ${MAX_ARTIFACT_KIB} KiB" >&2
  exit 1
fi

echo "Mac actual App.Cli release artifact: ${ARTIFACT_DIR} (${artifact_kib} KiB)"
