#!/usr/bin/env bash
# OPS-07: GitHub Release の stage0 package を取得して展開する
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VERSION="${1:-${STAGE0_VERSION:-$(git -C "$ROOT" describe --tags --abbrev=0 2>/dev/null || echo "")}}"
STAGE0_DIR="${STAGE0_DIR:-$ROOT/stage0}"
RELEASE_BASE_URL="${STAGE0_RELEASE_BASE_URL:-}"

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

detect_target() {
  if [[ -n "${STAGE0_TARGET:-}" ]]; then
    printf '%s\n' "$STAGE0_TARGET"
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

detect_archive_ext() {
  local target="$1"
  case "$target" in
    aarch64-apple-darwin|x86_64-unknown-linux-gnu) printf '%s\n' "tar.gz" ;;
    *)
      echo "ERROR: unsupported release target: $target. Supported release targets: aarch64-apple-darwin, x86_64-unknown-linux-gnu." >&2
      exit 1
      ;;
  esac
}

extract_archive() {
  local archive_path="$1"
  local extract_dir="$2"
  case "$archive_path" in
    *.tar.gz|*.tgz)
      tar -xzf "$archive_path" -C "$extract_dir"
      ;;
    *.zip)
      unzip -q "$archive_path" -d "$extract_dir"
      ;;
    *)
      echo "ERROR: unsupported archive format: $archive_path" >&2
      exit 1
      ;;
  esac
}

find_archive_root() {
  local extract_dir="$1"
  local direct_children=()
  shopt -s nullglob
  direct_children=("$extract_dir"/*)
  shopt -u nullglob
  if [[ ${#direct_children[@]} -eq 1 && -d "${direct_children[0]}" ]]; then
    printf '%s\n' "${direct_children[0]}"
    return 0
  fi
  local candidate
  candidate="$(find "$extract_dir" -mindepth 1 -maxdepth 2 -type f \( -name 'lsharp' -o -name 'lsharp.exe' \) -print -quit)"
  if [[ -n "$candidate" ]]; then
    dirname "$candidate"
    return 0
  fi
  return 1
}

verify_checksum_entry() {
  local checksums_path="$1"
  local target_dir="$2"
  local target_name="$3"
  local expected
  expected="$(awk -v name="$target_name" '$2 == name { print $1; exit }' "$checksums_path")"
  if [[ -z "$expected" ]]; then
    echo "ERROR: checksum entry not found for $target_name in $checksums_path" >&2
    exit 1
  fi
  local actual
  actual="$(hash_file "$target_dir/$target_name")"
  if [[ "$expected" != "$actual" ]]; then
    echo "ERROR: checksum mismatch for $target_name" >&2
    echo "expected: $expected" >&2
    echo "actual:   $actual" >&2
    exit 1
  fi
}

verify_package_checksums() {
  local package_dir="$1"
  local checksums_path="$package_dir/checksums.txt"
  if [[ ! -f "$checksums_path" ]]; then
    echo "ERROR: package checksums.txt not found under $package_dir" >&2
    exit 1
  fi
  while read -r expected relpath _; do
    [[ -n "${expected:-}" ]] || continue
    local actual
    actual="$(hash_file "$package_dir/$relpath")"
    if [[ "$expected" != "$actual" ]]; then
      echo "ERROR: package checksum mismatch for $relpath" >&2
      exit 1
    fi
  done < "$checksums_path"
}

if [[ -z "$VERSION" ]]; then
  echo "ERROR: stage0 version is required. Pass it as the first arg or STAGE0_VERSION." >&2
  exit 1
fi

TARGET="$(detect_target)"
ARCHIVE_EXT="$(detect_archive_ext "$TARGET")"
ARCHIVE_NAME="lsharp-${VERSION}-${TARGET}.${ARCHIVE_EXT}"
if [[ -z "$RELEASE_BASE_URL" ]]; then
  RELEASE_BASE_URL="https://github.com/biwakonbu/lsharp/releases/download/${VERSION}"
fi

WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/lsharp-stage0-fetch.XXXXXX")"
cleanup() {
  rm -rf "$WORK_DIR"
}
trap cleanup EXIT

ARCHIVE_PATH="$WORK_DIR/$ARCHIVE_NAME"
CHECKSUMS_PATH="$WORK_DIR/checksums.txt"

echo "=== fetch-stage0: download release assets ==="
curl -fsSL "$RELEASE_BASE_URL/$ARCHIVE_NAME" -o "$ARCHIVE_PATH"
curl -fsSL "$RELEASE_BASE_URL/checksums.txt" -o "$CHECKSUMS_PATH"

echo "=== fetch-stage0: verify release checksum ==="
verify_checksum_entry "$CHECKSUMS_PATH" "$WORK_DIR" "$ARCHIVE_NAME"

echo "=== fetch-stage0: extract package ==="
EXTRACT_DIR="$WORK_DIR/extract"
mkdir -p "$EXTRACT_DIR"
extract_archive "$ARCHIVE_PATH" "$EXTRACT_DIR"
ARCHIVE_ROOT="$(find_archive_root "$EXTRACT_DIR")" || {
  echo "ERROR: extracted package root not found" >&2
  exit 1
}

echo "=== fetch-stage0: verify package checksum ==="
verify_package_checksums "$ARCHIVE_ROOT"

echo "=== fetch-stage0: install stage0 package ==="
rm -rf "$STAGE0_DIR"
mkdir -p "$STAGE0_DIR"
cp -fR "$ARCHIVE_ROOT"/. "$STAGE0_DIR"/

echo "fetch-stage0: OK ($STAGE0_DIR)"
