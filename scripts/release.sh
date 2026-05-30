#!/usr/bin/env bash
# release.sh - L# 配布物作成スクリプト
#
# P11-4 T4e-1/T4e-2: OS 別配布形式の固定 + release artifact の同梱物
#
# 使用法:
#   ./scripts/release.sh [--target <os-arch>] [--version <version>]
#
# 対応ターゲット:
#   - x86_64-apple-darwin (macOS Intel)
#   - aarch64-apple-darwin (macOS ARM)
#   - x86_64-unknown-linux-gnu (Linux x86_64)
#   - aarch64-unknown-linux-gnu (Linux ARM)
#   - x86_64-pc-windows-msvc (Windows x86_64)
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
DIST_DIR="dist"
ARCHIVE_NAME="lsharp-${VERSION}-${TARGET}"
NATIVE_ONLY_RELEASE="${NATIVE_ONLY_RELEASE:-1}"
NATIVE_ONLY_PROGRAM="${NATIVE_ONLY_PROGRAM:-}"
ROLLBACK_COMPATIBILITY_ASSET="${ROLLBACK_COMPATIBILITY_ASSET:-lsharp-${VERSION}-${TARGET}-host-launcher.tar.gz}"
SOURCE_COMMIT="${SOURCE_COMMIT:-$(git rev-parse --verify HEAD 2>/dev/null || echo "unknown")}"

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
  local unix_path="target/release/${base_name}"
  local windows_path="target/release/${base_name}.exe"
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

generate_native_manifest() {
  local manifest_path="${DIST_DIR}/${ARCHIVE_NAME}/manifest.json"
  cat > "$manifest_path" <<EOF
{
  "schema_version": 1,
  "archive_kind": "native-only official archive",
  "target": "${TARGET}",
  "source_commit": "${SOURCE_COMMIT}",
  "entry_binary": "program.native",
  "rollback_anchor": {
    "kind": "rollback compatibility",
    "asset": "${ROLLBACK_COMPATIBILITY_ASSET}"
  },
  "smoke": {
    "kind": "native-only release smoke",
    "binary": "program.native"
  }
}
EOF
}

assemble_native_only_release() {
  local program_source="$NATIVE_ONLY_PROGRAM"
  local program_dest="${DIST_DIR}/${ARCHIVE_NAME}/program.native"
  local cli_dest="${DIST_DIR}/${ARCHIVE_NAME}/lsharp"

  if [[ -z "$program_source" ]]; then
    program_source="$(resolve_required_binary_path "lsharp")"
  fi
  if [[ ! -f "$program_source" ]]; then
    echo "ERROR: native-only program source not found: $program_source" >&2
    exit 1
  fi

  cp -f "$program_source" "$program_dest"
  chmod 755 "$program_dest"

  # install / fresh-clone tooling still resolves `lsharp`; keep it as a thin
  # filename alias for the native program while `program.native` remains canonical.
  cp -f "$program_dest" "$cli_dest"
  chmod 755 "$cli_dest"

  copy_required_file "README.md"
  copy_required_file "LICENSE"
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
  if [[ -z "$NATIVE_ONLY_PROGRAM" ]]; then
    echo "Building native-only release source binary..."
    cargo build --release -p lsharp-driver
  fi
  assemble_native_only_release
else
  echo "Building host launcher binaries..."
  cargo build --release
  copy_required_file "README.md"
  copy_required_file "LICENSE"
  copy_required_binary "lsharp"
  copy_required_binary "lsharp-lsp"
  echo "Generating rollback compatibility guest component sidecar..."
  generate_guest_component_sidecar
fi

# checksums.txt の生成 (AC-505: SHA-256)
echo "Generating checksums..."
./scripts/checksum.sh "${DIST_DIR}/${ARCHIVE_NAME}" > "${DIST_DIR}/${ARCHIVE_NAME}/checksums.txt"

# アーカイブ作成
echo "Creating archive..."
case "${TARGET}" in
  *windows*)
    # Windows: .zip 形式 (AC-502)
    cd "${DIST_DIR}" && zip -r "${ARCHIVE_NAME}.zip" "${ARCHIVE_NAME}/" 2>/dev/null || true
    ;;
  *)
    # macOS/Linux: .tar.gz 形式 (AC-500/AC-501)
    cd "${DIST_DIR}" && tar czf "${ARCHIVE_NAME}.tar.gz" "${ARCHIVE_NAME}/" 2>/dev/null || true
    ;;
esac

echo "=== Release complete: ${ARCHIVE_NAME} ==="
