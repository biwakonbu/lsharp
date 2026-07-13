#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PACKAGE="$ROOT/scripts/ci/package-native-stage0.sh"
RELEASE_PACKAGE="$ROOT/scripts/ci/package-native-stage0-release.sh"

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

assert_eq() {
  [[ "$1" == "$2" ]] || fail "expected '$1', got '$2'"
}

[[ -x "$PACKAGE" ]] || fail "native stage0 package builder is missing or not executable: $PACKAGE"
[[ -x "$RELEASE_PACKAGE" ]] || fail "native stage0 release package builder is missing or not executable: $RELEASE_PACKAGE"

TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/lsharp-native-stage0-release-package.XXXXXX")"
trap 'rm -rf "$TMP_ROOT"' EXIT

INPUT_DIR="$TMP_ROOT/input"
STAGE0_DIR="$TMP_ROOT/stage0"
DIST_DIR="$TMP_ROOT/dist"
EXTRACT_DIR="$TMP_ROOT/extract"
HOST_BIN="$TMP_ROOT/host-bin"
HOST_TOOL_LOG="$TMP_ROOT/host-tools.log"
TARGET="x86_64-unknown-linux-gnu"
VERSION="v0.0.0-test"
ARCHIVE_NAME="lsharp-stage0-${VERSION}-${TARGET}"
ARCHIVE_PATH="$DIST_DIR/${ARCHIVE_NAME}.tar.gz"

mkdir -p "$INPUT_DIR" "$HOST_BIN"
: >"$HOST_TOOL_LOG"

cat >"$INPUT_DIR/compiler.native" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
exit 0
SH
chmod +x "$INPUT_DIR/compiler.native"

cat >"$INPUT_DIR/transport-driver" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
exit 0
SH
chmod +x "$INPUT_DIR/transport-driver"

cat >"$INPUT_DIR/materializer.py" <<'PY'
raise SystemExit(0)
PY

for host_tool in cargo rustc lsharp; do
  cat >"$HOST_BIN/$host_tool" <<'SH'
#!/usr/bin/env bash
printf 'host-tool|%s\n' "$(basename "$0")" >>"${NATIVE_STAGE0_RELEASE_TEST_LOG:?}"
exit 99
SH
  chmod +x "$HOST_BIN/$host_tool"
done

PATH="$HOST_BIN:$PATH" "$PACKAGE" \
  --target "$TARGET" \
  --compiler "$INPUT_DIR/compiler.native" \
  --transport-driver "$INPUT_DIR/transport-driver" \
  --materializer "$INPUT_DIR/materializer.py" \
  --output-dir "$STAGE0_DIR"

NATIVE_STAGE0_RELEASE_TEST_LOG="$HOST_TOOL_LOG" \
  PATH="$HOST_BIN:$PATH" \
  "$RELEASE_PACKAGE" \
    --target "$TARGET" \
    --version "$VERSION" \
    --stage0-dir "$STAGE0_DIR" \
    --output-dir "$DIST_DIR"

assert_eq "" "$(cat "$HOST_TOOL_LOG")"
[[ -s "$ARCHIVE_PATH" ]] || fail "native stage0 release archive was not created: $ARCHIVE_PATH"

archive_listing="$(tar -tzf "$ARCHIVE_PATH")"
for required in \
  "$ARCHIVE_NAME/manifest.json" \
  "$ARCHIVE_NAME/bin/compiler" \
  "$ARCHIVE_NAME/bin/transport-driver" \
  "$ARCHIVE_NAME/bin/materializer" \
  "$ARCHIVE_NAME/bin/materializer.py" \
  "$ARCHIVE_NAME/checksums.txt"; do
  grep -Fx "$required" <<<"$archive_listing" >/dev/null \
    || fail "archive payload is missing: $required"
done
! grep -E '(^|/)\._' <<<"$archive_listing" >/dev/null \
  || fail "archive contains macOS metadata files"

mkdir -p "$EXTRACT_DIR"
tar -xzf "$ARCHIVE_PATH" -C "$EXTRACT_DIR"
EXTRACTED_STAGE0="$EXTRACT_DIR/$ARCHIVE_NAME"
[[ -d "$EXTRACTED_STAGE0" ]] || fail "archive root is missing: $EXTRACTED_STAGE0"

python3 - "$EXTRACTED_STAGE0/manifest.json" "$TARGET" <<'PY'
import json
import os
import pathlib
import sys

manifest_path = pathlib.Path(sys.argv[1])
target = sys.argv[2]
manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
if manifest.get("kind") != "lsharp-native-selfhost-stage0":
    raise SystemExit(f"unexpected manifest kind: {manifest.get('kind')!r}")
if manifest.get("target") != target:
    raise SystemExit(f"unexpected manifest target: {manifest.get('target')!r}")
for field in ("compiler", "transport_driver", "materializer"):
    value = manifest.get(field)
    path = pathlib.PurePosixPath(value or "")
    if not isinstance(value, str) or not value or path.is_absolute() or ".." in path.parts:
        raise SystemExit(f"unsafe manifest path: {field}={value!r}")
    candidate = manifest_path.parent.joinpath(*path.parts)
    if not candidate.is_file() or not os.access(candidate, os.X_OK):
        raise SystemExit(f"missing executable: {candidate}")
PY

"$ROOT/scripts/checksum.sh" "$EXTRACTED_STAGE0" >"$TMP_ROOT/actual-checksums.txt"
cmp -s "$EXTRACTED_STAGE0/checksums.txt" "$TMP_ROOT/actual-checksums.txt" \
  || fail "archive package checksums do not match payload"

echo "native stage0 release package tests: OK"
