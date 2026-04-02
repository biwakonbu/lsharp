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
#   - lsharp (CLI バイナリ)
#   - lsharp-lsp (LSP サーバーバイナリ)
#   - README.md
#   - LICENSE
#   - checksums.txt (SHA-256, AC-505)

set -euo pipefail

VERSION="${VERSION:-$(git describe --tags --always 2>/dev/null || echo "dev")}"
TARGET="${TARGET:-$(rustc -Vv 2>/dev/null | grep host | cut -d' ' -f2 || echo "unknown")}"
DIST_DIR="dist"
ARCHIVE_NAME="lsharp-${VERSION}-${TARGET}"

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

copy_required_binary() {
  local base_name="$1"
  local unix_path="target/release/${base_name}"
  local windows_path="target/release/${base_name}.exe"
  if [[ -f "$unix_path" ]]; then
    cp -f "$unix_path" "${DIST_DIR}/${ARCHIVE_NAME}/"
    return 0
  fi
  if [[ -f "$windows_path" ]]; then
    cp -f "$windows_path" "${DIST_DIR}/${ARCHIVE_NAME}/"
    return 0
  fi
  echo "ERROR: required release binary not found: ${base_name}" >&2
  exit 1
}

# バイナリのビルド
echo "Building binaries..."
cargo build --release

# 同梱物のコピー (AC-504)
echo "Assembling release artifacts..."
copy_required_file "README.md"
copy_required_file "LICENSE"

# バイナリのコピー
copy_required_binary "lsharp"
copy_required_binary "lsharp-lsp"

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
