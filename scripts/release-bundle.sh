#!/usr/bin/env bash
# OPS-07: bootstrap 済み stage bundle から release-style archive を生成する
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
STAGE_DIR="${STAGE_DIR:-$ROOT/stage2}"
DIST_DIR="${DIST_DIR:-$ROOT/dist}"
VERSION="${VERSION:-$(git -C "$ROOT" describe --tags --always 2>/dev/null || echo "dev")}"
TARGET="${TARGET:-}"

detect_target() {
  if [[ -n "$TARGET" ]]; then
    printf '%s\n' "$TARGET"
    return 0
  fi
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"
  case "$os:$arch" in
    Darwin:arm64|Darwin:aarch64) printf '%s\n' "aarch64-apple-darwin" ;;
    Linux:x86_64) printf '%s\n' "x86_64-unknown-linux-gnu" ;;
    *)
      echo "ERROR: unsupported host target: ${os}/${arch}. Supported release targets: aarch64-apple-darwin, x86_64-unknown-linux-gnu." >&2
      exit 1
      ;;
  esac
}

validate_target() {
  case "$1" in
    aarch64-apple-darwin|x86_64-unknown-linux-gnu) ;;
    *)
      echo "ERROR: unsupported release target: $1. Supported release targets: aarch64-apple-darwin, x86_64-unknown-linux-gnu." >&2
      exit 1
      ;;
  esac
}

require_file() {
  local path="$1"
  if [[ ! -f "$path" ]]; then
    echo "ERROR: required file not found: $path" >&2
    exit 1
  fi
}

TARGET="$(detect_target)"
validate_target "$TARGET"
ARCHIVE_NAME="lsharp-${VERSION}-${TARGET}"
BUNDLE_DIR="$DIST_DIR/$ARCHIVE_NAME"

require_file "$STAGE_DIR/lsharp"
require_file "$STAGE_DIR/lsharp.component.wasm"
if [[ ! -f "$STAGE_DIR/lsharp-lsp" && ! -f "$STAGE_DIR/lsharp-lsp.exe" ]]; then
  echo "ERROR: stage bundle must include lsharp-lsp" >&2
  exit 1
fi

echo "=== release-bundle: assemble bundle ==="
rm -rf "$BUNDLE_DIR"
mkdir -p "$BUNDLE_DIR" "$DIST_DIR"
cp -f "$STAGE_DIR/lsharp" "$BUNDLE_DIR/lsharp"
if [[ -f "$STAGE_DIR/lsharp-lsp" ]]; then
  cp -f "$STAGE_DIR/lsharp-lsp" "$BUNDLE_DIR/lsharp-lsp"
else
  cp -f "$STAGE_DIR/lsharp-lsp.exe" "$BUNDLE_DIR/lsharp-lsp.exe"
fi
cp -f "$STAGE_DIR/lsharp.component.wasm" "$BUNDLE_DIR/lsharp.component.wasm"
cp -f "$ROOT/README.md" "$BUNDLE_DIR/README.md"
cp -f "$ROOT/LICENSE" "$BUNDLE_DIR/LICENSE"

echo "=== release-bundle: generate checksums ==="
bash "$ROOT/scripts/checksum.sh" "$BUNDLE_DIR" > "$BUNDLE_DIR/checksums.txt"

echo "=== release-bundle: create convenience outputs ==="
cp -f "$BUNDLE_DIR/lsharp" "$DIST_DIR/lsharp"
cp -f "$BUNDLE_DIR/lsharp.component.wasm" "$DIST_DIR/lsharp.component.wasm"
cp -f "$BUNDLE_DIR/lsharp.component.wasm" "$DIST_DIR/${ARCHIVE_NAME}.component.wasm"

echo "=== release-bundle: archive bundle ==="
(
  cd "$DIST_DIR"
  tar czf "${ARCHIVE_NAME}.tar.gz" "$ARCHIVE_NAME"
)

echo "release-bundle: OK ($DIST_DIR)"
