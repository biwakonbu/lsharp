#!/usr/bin/env sh
# Install the packaged L# release into ~/.local/bin by default.
set -eu

REPO="${LSHARP_REPO:-biwakonbu/lsharp}"
INSTALL_DIR="${LSHARP_INSTALL_DIR:-$HOME/.local/bin}"
TARGET="${LSHARP_TARGET:-}"
VERSION="${LSHARP_VERSION:-}"
ARCHIVE_URL="${LSHARP_ARCHIVE_URL:-}"
TMP_DIR="${TMPDIR:-/tmp}/lsharp-install.$$"

cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT INT TERM

fail() {
  echo "lsharp install: ERROR: $*" >&2
  exit 1
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || fail "required command not found: $1"
}

hash_file() {
  path="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$path" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$path" | awk '{print $1}'
  else
    fail "sha256sum or shasum is required"
  fi
}

detect_target() {
  os="$(uname -s)"
  arch="$(uname -m)"
  case "$os:$arch" in
    Darwin:arm64|Darwin:aarch64) echo "aarch64-apple-darwin" ;;
    Linux:x86_64) echo "x86_64-unknown-linux-gnu" ;;
    *) fail "unsupported host target: $os/$arch; supported release targets: aarch64-apple-darwin, x86_64-unknown-linux-gnu" ;;
  esac
}

validate_target() {
  case "$1" in
    aarch64-apple-darwin|x86_64-unknown-linux-gnu) ;;
    *) fail "unsupported release target: $1; supported release targets: aarch64-apple-darwin, x86_64-unknown-linux-gnu" ;;
  esac
}

resolve_latest_version() {
  curl -fsSL "https://api.github.com/repos/${REPO}/releases" \
    | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' \
    | head -n 1
}

download() {
  url="$1"
  output="$2"
  curl -fsSL "$url" -o "$output"
}

try_download() {
  url="$1"
  output="$2"
  if curl -fsSL "$url" -o "$output"; then
    return 0
  fi
  rm -f "$output"
  return 1
}

extract_archive() {
  archive="$1"
  dest="$2"
  case "$archive" in
    *.tar.gz|*.tgz) tar -xzf "$archive" -C "$dest" ;;
    *.zip) unzip -q "$archive" -d "$dest" ;;
    *) fail "unsupported archive format: $archive" ;;
  esac
}

find_release_root() {
  dir="$1"
  found="$(find "$dir" -mindepth 1 -maxdepth 3 -type f \( -name program.native -o -name lsharp -o -name lsharp.exe \) -print -quit)"
  [ -n "$found" ] || return 1
  dirname "$found"
}

verify_packaged_checksums() {
  root="$1"
  checksums="$root/checksums.txt"
  [ -f "$checksums" ] || fail "checksums.txt not found in release archive"
  while read -r expected relpath rest; do
    [ -n "${expected:-}" ] || continue
    case "$expected" in \#*) continue ;; esac
    target="$root/$relpath"
    [ -f "$target" ] || fail "checksum target missing: $relpath"
    actual="$(hash_file "$target")"
    [ "$actual" = "$expected" ] || fail "checksum mismatch for $relpath"
  done < "$checksums"
}

install_packaged_release() {
  root="$1"
  mkdir -p "$INSTALL_DIR"

  if [ -f "$root/program.native" ]; then
    lsharp_bin="$root/program.native"
    lsp_bin=""
  elif [ -f "$root/lsharp.exe" ]; then
    lsharp_bin="$root/lsharp.exe"
    lsp_bin="$root/lsharp-lsp.exe"
  else
    lsharp_bin="$root/lsharp"
    lsp_bin="$root/lsharp-lsp"
  fi

  [ -f "$lsharp_bin" ] || fail "lsharp binary not found in release archive"
  if [ -n "$lsp_bin" ]; then
    [ -f "$lsp_bin" ] || fail "lsharp-lsp binary not found in release archive"
  fi

  cp "$lsharp_bin" "$INSTALL_DIR/lsharp"
  chmod 755 "$INSTALL_DIR/lsharp"
  if [ -n "$lsp_bin" ]; then
    cp "$lsp_bin" "$INSTALL_DIR/lsharp-lsp"
    chmod 755 "$INSTALL_DIR/lsharp-lsp"
  fi
  if [ -f "$root/lsharp.component.wasm" ]; then
    cp "$root/lsharp.component.wasm" "$INSTALL_DIR/lsharp.component.wasm"
    chmod 644 "$INSTALL_DIR/lsharp.component.wasm"
  fi
}

install_experimental_native_rc() {
  extract_dir="$1"
  program="$(find "$extract_dir" -path '*/stage3-native/program.native' -type f -print -quit)"
  [ -n "$program" ] || fail "stage3-native/program.native not found in experimental native RC archive"

  bin_name="${LSHARP_EXPERIMENTAL_BIN_NAME:-lsharp-native-rc}"
  if [ "${LSHARP_EXPERIMENTAL_AS_LSHARP:-0}" = "1" ]; then
    bin_name="lsharp"
  fi

  mkdir -p "$INSTALL_DIR"
  cp "$program" "$INSTALL_DIR/$bin_name"
  chmod 755 "$INSTALL_DIR/$bin_name"
  echo "lsharp install: installed experimental native RC as $INSTALL_DIR/$bin_name" >&2
  echo "lsharp install: this artifact is not the stable CLI; set LSHARP_EXPERIMENTAL_AS_LSHARP=1 only for native artifact experiments" >&2
}

need_cmd curl
need_cmd awk
need_cmd sed
need_cmd find
need_cmd uname
need_cmd tar

[ -n "$TARGET" ] || TARGET="$(detect_target)"
validate_target "$TARGET"
EXT="tar.gz"

if [ -z "$VERSION" ]; then
  VERSION="$(resolve_latest_version)"
  [ -n "$VERSION" ] || fail "could not resolve latest release; set LSHARP_VERSION"
fi

mkdir -p "$TMP_DIR/extract"

if [ -z "$ARCHIVE_URL" ]; then
  asset="lsharp-${VERSION}-${TARGET}.${EXT}"
  ARCHIVE_URL="https://github.com/${REPO}/releases/download/${VERSION}/${asset}"
fi

archive_path="$TMP_DIR/archive.${EXT}"
echo "lsharp install: downloading $ARCHIVE_URL" >&2
if download "$ARCHIVE_URL" "$archive_path"; then
  extract_archive "$archive_path" "$TMP_DIR/extract"
  release_root="$(find_release_root "$TMP_DIR/extract")" || fail "release archive does not contain lsharp"
  verify_packaged_checksums "$release_root"
  install_packaged_release "$release_root"
  echo "lsharp install: installed lsharp to $INSTALL_DIR/lsharp" >&2
  echo "lsharp install: add $INSTALL_DIR to PATH if needed" >&2
  exit 0
fi

experimental_asset="experimental-native-rc-${VERSION}-${TARGET}.tar.gz"
experimental_url="https://github.com/${REPO}/releases/download/${VERSION}/${experimental_asset}"
archive_path="$TMP_DIR/experimental-native-rc.tar.gz"
echo "lsharp install: packaged CLI archive not found; trying experimental native RC $experimental_url" >&2
if try_download "$experimental_url" "$archive_path"; then
  extract_archive "$archive_path" "$TMP_DIR/extract"
  install_experimental_native_rc "$TMP_DIR/extract"
  exit 0
fi

fail "no installable archive found for ${VERSION}/${TARGET}"
