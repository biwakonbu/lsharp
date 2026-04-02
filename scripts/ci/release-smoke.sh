#!/usr/bin/env bash
# OPS-06/PKG-01: release artifact 展開ベースで packaged binary smoke を行う
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
ARCHIVE_PATH="${1:-}"
WORK_DIR="${WORK_DIR:-$ROOT/target/ci/release-smoke}"
EXTRACT_DIR="$WORK_DIR/extract"
SMOKE_DIR="$WORK_DIR/smoke"

cleanup() {
  local exit_code=$?
  if [[ $exit_code -eq 0 && "${KEEP_WORK_DIR:-0}" != "1" ]]; then
    rm -rf "$WORK_DIR"
  fi
}
trap cleanup EXIT

usage() {
  echo "Usage: $0 <release-archive>" >&2
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

if [[ -z "$ARCHIVE_PATH" ]]; then
  usage
  exit 1
fi

if [[ ! -f "$ARCHIVE_PATH" ]]; then
  echo "ERROR: archive not found: $ARCHIVE_PATH" >&2
  exit 1
fi

echo "=== release-smoke: unpack artifact ==="
rm -rf "$WORK_DIR"
mkdir -p "$EXTRACT_DIR" "$SMOKE_DIR"

case "$ARCHIVE_PATH" in
  *.tar.gz|*.tgz)
    tar -xzf "$ARCHIVE_PATH" -C "$EXTRACT_DIR"
    ;;
  *.zip)
    unzip -q "$ARCHIVE_PATH" -d "$EXTRACT_DIR"
    ;;
  *)
    echo "ERROR: unsupported archive format: $ARCHIVE_PATH" >&2
    exit 1
    ;;
esac

ARCHIVE_ROOT="$(find_archive_root "$EXTRACT_DIR")" || {
  echo "ERROR: extracted archive root containing lsharp binary not found" >&2
  exit 1
}

LSHARP_BIN="$ARCHIVE_ROOT/lsharp"
if [[ ! -e "$LSHARP_BIN" && -e "$ARCHIVE_ROOT/lsharp.exe" ]]; then
  LSHARP_BIN="$ARCHIVE_ROOT/lsharp.exe"
fi

LSHARP_LSP_BIN="$ARCHIVE_ROOT/lsharp-lsp"
if [[ ! -e "$LSHARP_LSP_BIN" && -e "$ARCHIVE_ROOT/lsharp-lsp.exe" ]]; then
  LSHARP_LSP_BIN="$ARCHIVE_ROOT/lsharp-lsp.exe"
fi

if [[ ! -e "$LSHARP_BIN" ]]; then
  echo "ERROR: packaged lsharp binary not found under $ARCHIVE_ROOT" >&2
  exit 1
fi

for required in README.md LICENSE checksums.txt; do
  if [[ ! -f "$ARCHIVE_ROOT/$required" ]]; then
    echo "ERROR: required release payload missing: $required" >&2
    exit 1
  fi
done

if [[ ! -e "$LSHARP_LSP_BIN" ]]; then
  echo "ERROR: packaged lsharp-lsp binary not found under $ARCHIVE_ROOT" >&2
  exit 1
fi

for optional in CHANGELOG.md; do
  if [[ -e "$ARCHIVE_ROOT/$optional" ]]; then
    echo "INFO: optional payload present: $optional"
  fi
done

echo "=== release-smoke: verify checksums ==="
while read -r expected relpath _; do
  [[ -n "${expected:-}" ]] || continue
  target="$ARCHIVE_ROOT/$relpath"
  if [[ ! -f "$target" ]]; then
    echo "ERROR: checksum target missing: $relpath" >&2
    exit 1
  fi
  actual="$(hash_file "$target")"
  if [[ "$actual" != "$expected" ]]; then
    echo "ERROR: checksum mismatch for $relpath" >&2
    echo "expected: $expected" >&2
    echo "actual:   $actual" >&2
    exit 1
  fi
done < "$ARCHIVE_ROOT/checksums.txt"

SMOKE_SOURCE="$SMOKE_DIR/quickstart.ls"
SMOKE_METADATA_SOURCE="$SMOKE_DIR/quickstart-metadata.ls"
SMOKE_WASM="$SMOKE_DIR/quickstart.wasm"
SMOKE_DOC_HTML="$SMOKE_DIR/quickstart.html"
SMOKE_DOC_JSON="$SMOKE_DIR/api.json"
cat > "$SMOKE_SOURCE" <<'EOF'
(defn main [] 42)
EOF
cat > "$SMOKE_METADATA_SOURCE" <<'EOF'
(defn abs
  [x]
  :doc "整数の絶対値を返す。"
  :params [(x "対象の整数")]
  :returns "x の絶対値"
  :example [(= (abs 5) 5)]
  :invariant (>= result 0)
  (if (< x 0) (- 0 x) x))
EOF

echo "=== release-smoke: packaged binary ==="
"$LSHARP_BIN" --version >/dev/null
"$LSHARP_LSP_BIN" --version >/dev/null
"$LSHARP_BIN" check "$SMOKE_SOURCE" >/dev/null
"$LSHARP_BIN" fmt "$SMOKE_SOURCE" >/dev/null
"$LSHARP_BIN" compile "$SMOKE_SOURCE" -o "$SMOKE_WASM" >/dev/null
"$LSHARP_BIN" test "$SMOKE_METADATA_SOURCE" >/dev/null
"$LSHARP_BIN" doc "$SMOKE_METADATA_SOURCE" -o "$SMOKE_DOC_HTML" >/dev/null
"$LSHARP_BIN" doc "$SMOKE_METADATA_SOURCE" --json -o "$SMOKE_DOC_JSON" >/dev/null

if [[ ! -s "$SMOKE_WASM" ]]; then
  echo "ERROR: compile output is empty: $SMOKE_WASM" >&2
  exit 1
fi
if ! xxd -p -l 4 "$SMOKE_WASM" | grep -qi '^0061736d$'; then
  echo "ERROR: compile output is not a Wasm binary: $SMOKE_WASM" >&2
  exit 1
fi

if [[ ! -s "$SMOKE_DOC_HTML" ]]; then
  echo "ERROR: doc HTML output is empty: $SMOKE_DOC_HTML" >&2
  exit 1
fi

if [[ ! -s "$SMOKE_DOC_JSON" ]]; then
  echo "ERROR: doc JSON output is empty: $SMOKE_DOC_JSON" >&2
  exit 1
fi

echo "release-smoke: OK"
