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

# guest component sidecar の生成
echo "Generating guest component sidecar..."
generate_guest_component_sidecar

# checksums.txt の生成 (AC-505: SHA-256)
echo "Generating checksums..."
./scripts/checksum.sh "${DIST_DIR}/${ARCHIVE_NAME}" > "${DIST_DIR}/${ARCHIVE_NAME}/checksums.txt"

# アーカイブ作成
echo "Creating archive..."
# Mac Apple Silicon / Linux x86_64: .tar.gz 形式
cd "${DIST_DIR}" && tar czf "${ARCHIVE_NAME}.tar.gz" "${ARCHIVE_NAME}/" 2>/dev/null || true

echo "=== Release complete: ${ARCHIVE_NAME} ==="
