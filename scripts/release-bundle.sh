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
    Darwin:x86_64) printf '%s\n' "x86_64-apple-darwin" ;;
    Darwin:arm64|Darwin:aarch64) printf '%s\n' "aarch64-apple-darwin" ;;
    Linux:x86_64) printf '%s\n' "x86_64-unknown-linux-gnu" ;;
    Linux:arm64|Linux:aarch64) printf '%s\n' "aarch64-unknown-linux-gnu" ;;
    MINGW64_NT-*:x86_64|MSYS_NT-*:x86_64|CYGWIN_NT-*:x86_64) printf '%s\n' "x86_64-pc-windows-msvc" ;;
    *)
      echo "ERROR: unsupported host target: ${os}/${arch}. Set TARGET explicitly." >&2
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
case "$TARGET" in
  *windows*)
    (
      cd "$DIST_DIR"
      zip -rq "${ARCHIVE_NAME}.zip" "$ARCHIVE_NAME"
    )
    ;;
  *)
    (
      cd "$DIST_DIR"
      tar czf "${ARCHIVE_NAME}.tar.gz" "$ARCHIVE_NAME"
    )
    ;;
esac

echo "release-bundle: OK ($DIST_DIR)"
