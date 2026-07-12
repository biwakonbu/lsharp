#!/usr/bin/env bash
# release.sh - L# 配布物作成スクリプト
#
# P11-4 T4e-1/T4e-2: OS 別配布形式の固定 + release artifact の同梱物
#
# 使用法:
#   ./scripts/release.sh [--target <os-arch>] [--version <version>]
#
# Supported product/release targets:
#   - aarch64-apple-darwin (Mac Apple Silicon)
#   - x86_64-unknown-linux-gnu (Linux x86_64)
#
# 同梱物 (AC-504):
#   - program.native (native-only official CLI body)
#   - README.md
#   - LICENSE
#   - manifest.json (target / rollback anchor / smoke metadata)
#   - checksums.txt (SHA-256, AC-505)

set -euo pipefail

VERSION="${VERSION:-$(git describe --tags --always 2>/dev/null || echo "dev")}"
TARGET="${TARGET:-$(rustc -Vv 2>/dev/null | grep host | cut -d' ' -f2 || echo "unknown")}"
DIST_DIR="${DIST_DIR:-dist}"
BUILD_TARGET_DIR="${CARGO_TARGET_DIR:-target}"
NATIVE_ONLY_RELEASE="${NATIVE_ONLY_RELEASE:-1}"
NATIVE_ONLY_PROGRAM="${NATIVE_ONLY_PROGRAM:-}"
NATIVE_ONLY_PROGRAM_MANIFEST="${NATIVE_ONLY_PROGRAM_MANIFEST:-}"
ROLLBACK_COMPATIBILITY_ASSET_PATH="${ROLLBACK_COMPATIBILITY_ASSET_PATH:-}"
SOURCE_COMMIT="${SOURCE_COMMIT:-$(git rev-parse --verify HEAD 2>/dev/null || echo "unknown")}"
BASE_ARCHIVE_NAME="lsharp-${VERSION}-${TARGET}"
if [[ "${NATIVE_ONLY_RELEASE}" == "1" ]]; then
  ARCHIVE_NAME="${BASE_ARCHIVE_NAME}"
else
  ARCHIVE_NAME="${BASE_ARCHIVE_NAME}-host-launcher"
fi
ROLLBACK_COMPATIBILITY_ASSET=""
ROLLBACK_COMPATIBILITY_SHA256=""
NATIVE_ONLY_PROGRAM_INPUT_SHA256=""

case "$TARGET" in
  aarch64-apple-darwin|x86_64-unknown-linux-gnu) ;;
  *)
    echo "ERROR: unsupported release target: ${TARGET}. Supported release targets: aarch64-apple-darwin, x86_64-unknown-linux-gnu." >&2
    exit 1
    ;;
esac

echo "=== L# Release Build ==="
echo "Version: ${VERSION}"
echo "Target:  ${TARGET}"

# ビルドディレクトリの作成
mkdir -p "${DIST_DIR}/${ARCHIVE_NAME}"

copy_required_file() {
  local source_path="$1"
  if [[ ! -f "$source_path" ]]; then
    echo "ERROR: required release payload source not found: $source_path" >&2
    exit 1
  fi
  cp -f "$source_path" "${DIST_DIR}/${ARCHIVE_NAME}/"
}

resolve_required_binary_path() {
  local base_name="$1"
  local unix_path="${BUILD_TARGET_DIR}/release/${base_name}"
  local windows_path="${BUILD_TARGET_DIR}/release/${base_name}.exe"
  if [[ -f "$unix_path" ]]; then
    printf '%s\n' "$unix_path"
    return 0
  fi
  if [[ -f "$windows_path" ]]; then
    printf '%s\n' "$windows_path"
    return 0
  fi
  echo "ERROR: required release binary not found: ${base_name}" >&2
  exit 1
}

copy_required_binary() {
  local base_name="$1"
  local binary_path
  binary_path="$(resolve_required_binary_path "$base_name")"
  cp -f "$binary_path" "${DIST_DIR}/${ARCHIVE_NAME}/"
}

hash_file() {
  local path="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$path" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$path" | awk '{print $1}'
  else
    echo "ERROR: sha256sum or shasum not found" >&2
    exit 1
  fi
}

validate_native_release_inputs() {
  if [[ -z "${NATIVE_ONLY_PROGRAM_MANIFEST}" || ! -s "${NATIVE_ONLY_PROGRAM_MANIFEST}" ]]; then
    echo "ERROR: native App.Cli release program manifest is required" >&2
    exit 1
  fi
  if [[ -z "${ROLLBACK_COMPATIBILITY_ASSET_PATH}" || ! -s "${ROLLBACK_COMPATIBILITY_ASSET_PATH}" ]]; then
    echo "ERROR: rollback compatibility asset is required" >&2
    exit 1
  fi

  NATIVE_ONLY_PROGRAM_INPUT_SHA256="$(hash_file "${NATIVE_ONLY_PROGRAM}")"
  python3 - \
    "${NATIVE_ONLY_PROGRAM_MANIFEST}" \
    "${TARGET}" \
    "${SOURCE_COMMIT}" \
    "${NATIVE_ONLY_PROGRAM_INPUT_SHA256}" <<'PY'
import json
import pathlib
import sys

manifest_path = pathlib.Path(sys.argv[1])
target = sys.argv[2]
source_commit = sys.argv[3]
program_sha256 = sys.argv[4]
manifest = json.loads(manifest_path.read_text())

if manifest.get("status") != "pass":
    raise SystemExit("native App.Cli release program manifest status must be pass")
if manifest.get("target") != target:
    raise SystemExit("native App.Cli release program target mismatch")
if manifest.get("entry_module") != "App.Cli":
    raise SystemExit("entry module must be App.Cli")
if manifest.get("source") != "src/App/Cli.ls":
    raise SystemExit("entry module must be App.Cli")
if manifest.get("selfhost_fixed_point") is not True:
    raise SystemExit("selfhost fixed-point evidence is required")
if manifest.get("scope") != "Linux x86_64 App.Cli native release bundle" and manifest.get("artifact_kind") != "native App.Cli release program":
    raise SystemExit("artifact kind must be native App.Cli release program")
if manifest.get("source_commit") != source_commit:
    raise SystemExit("native App.Cli release program source commit mismatch")
if manifest.get("program_sha256") != program_sha256:
    raise SystemExit("native program sha256 mismatch")
PY

  ROLLBACK_COMPATIBILITY_ASSET="$(basename "${ROLLBACK_COMPATIBILITY_ASSET_PATH}")"
  ROLLBACK_COMPATIBILITY_SHA256="$(hash_file "${ROLLBACK_COMPATIBILITY_ASSET_PATH}")"
}

generate_native_manifest() {
  local manifest_path="${DIST_DIR}/${ARCHIVE_NAME}/manifest.json"
  cat > "$manifest_path" <<EOF
{
  "schema_version": 1,
  "archive_kind": "native-only official archive",
  "target": "${TARGET}",
  "version": "${VERSION}",
  "source_commit": "${SOURCE_COMMIT}",
  "entry_binary": "program.native",
  "rollback_anchor": {
    "kind": "rollback compatibility",
    "asset": "${ROLLBACK_COMPATIBILITY_ASSET}",
    "rollback_sha256": "${ROLLBACK_COMPATIBILITY_SHA256}"
  },
  "native_program_input": {
    "manifest": "native-program-manifest.json",
    "input_sha256": "${NATIVE_ONLY_PROGRAM_INPUT_SHA256}"
  },
  "smoke": {
    "kind": "native-only release smoke",
    "binary": "program.native"
  }
}
EOF
}

generate_rollback_manifest() {
  local manifest_path="${DIST_DIR}/${ARCHIVE_NAME}/manifest.json"
  cat > "${manifest_path}" <<EOF
{
  "schema_version": 1,
  "archive_kind": "rollback compatibility",
  "target": "${TARGET}",
  "version": "${VERSION}",
  "source_commit": "${SOURCE_COMMIT}",
  "entry_binary": "lsharp",
  "lsp_binary": "lsharp-lsp",
  "component": "lsharp.component.wasm"
}
EOF
}

assemble_native_only_release() {
  local program_source="$NATIVE_ONLY_PROGRAM"
  local program_dest="${DIST_DIR}/${ARCHIVE_NAME}/program.native"
  local cli_dest="${DIST_DIR}/${ARCHIVE_NAME}/lsharp"

  if [[ -z "$program_source" ]]; then
    echo "ERROR: NATIVE_ONLY_PROGRAM is required for native-only official archive" >&2
    echo "ERROR: refusing to package the Rust host launcher as program.native" >&2
    exit 1
  fi
  if [[ ! -s "$program_source" ]]; then
    echo "ERROR: native-only program source not found: $program_source" >&2
    exit 1
  fi
  validate_native_release_inputs

  cp -f "$program_source" "$program_dest"
  chmod 755 "$program_dest"

  # install / fresh-clone tooling still resolves `lsharp`; keep it as a thin
  # filename alias for the native program while `program.native` remains canonical.
  cp -f "$program_dest" "$cli_dest"
  chmod 755 "$cli_dest"

  copy_required_file "README.md"
  copy_required_file "LICENSE"
  cp -f "${NATIVE_ONLY_PROGRAM_MANIFEST}" "${DIST_DIR}/${ARCHIVE_NAME}/native-program-manifest.json"
  generate_native_manifest
}

generate_guest_component_sidecar() {
  local lsharp_bin
  local entry_path="selfhost/src/App/EmbeddedCli.ls"
  local release_sidecar_path="${DIST_DIR}/${ARCHIVE_NAME}.component.wasm"
  local packaged_sidecar_path="${DIST_DIR}/${ARCHIVE_NAME}/lsharp.component.wasm"

  if [[ ! -f "$entry_path" ]]; then
    echo "ERROR: embedded guest component source not found: $entry_path" >&2
    exit 1
  fi

  lsharp_bin="$(resolve_required_binary_path "lsharp")"
  LSHARP_DISABLE_EMBEDDED_COMPONENT=1 "$lsharp_bin" compile "$entry_path" -o "$release_sidecar_path" >/dev/null

  if [[ ! -s "$release_sidecar_path" ]]; then
    echo "ERROR: guest component sidecar generation failed: $release_sidecar_path" >&2
    exit 1
  fi

  cp -f "$release_sidecar_path" "$packaged_sidecar_path"
}

echo "Assembling release artifacts..."
if [[ "$NATIVE_ONLY_RELEASE" == "1" ]]; then
  assemble_native_only_release
  rollback_dest="${DIST_DIR}/${ROLLBACK_COMPATIBILITY_ASSET}"
  if [[ "${ROLLBACK_COMPATIBILITY_ASSET_PATH}" != "${rollback_dest}" ]]; then
    cp -f "${ROLLBACK_COMPATIBILITY_ASSET_PATH}" "${rollback_dest}"
  fi
else
  echo "Building host launcher binaries..."
  cargo build --release
  copy_required_file "README.md"
  copy_required_file "LICENSE"
  copy_required_binary "lsharp"
  copy_required_binary "lsharp-lsp"
  echo "Generating rollback compatibility guest component sidecar..."
  generate_guest_component_sidecar
  generate_rollback_manifest
fi

# checksums.txt の生成 (AC-505: SHA-256)
echo "Generating checksums..."
./scripts/checksum.sh "${DIST_DIR}/${ARCHIVE_NAME}" > "${DIST_DIR}/${ARCHIVE_NAME}/checksums.txt"

# アーカイブ作成
echo "Creating archive..."
# Mac Apple Silicon / Linux x86_64: .tar.gz 形式
(
  cd "${DIST_DIR}"
  COPYFILE_DISABLE=1 tar --no-xattrs --exclude '._*' -czf "${ARCHIVE_NAME}.tar.gz" "${ARCHIVE_NAME}/"
)
if [[ ! -s "${DIST_DIR}/${ARCHIVE_NAME}.tar.gz" ]]; then
  echo "ERROR: release archive was not created: ${DIST_DIR}/${ARCHIVE_NAME}.tar.gz" >&2
  exit 1
fi

echo "=== Release complete: ${ARCHIVE_NAME} ==="
