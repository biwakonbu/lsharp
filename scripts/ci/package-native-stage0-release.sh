#!/usr/bin/env bash
set -euo pipefail

TARGET=""
VERSION=""
STAGE0_DIR=""
OUTPUT_DIR=""

usage() {
  cat <<'EOF'
usage: scripts/ci/package-native-stage0-release.sh --target TARGET --version VERSION --stage0-dir DIR --output-dir DIR

options:
  --target TARGET      x86_64-unknown-linux-gnu or aarch64-apple-darwin
  --version VERSION    release version, for example v0.1.0
  --stage0-dir DIR     verified native stage0 package directory
  --output-dir DIR     directory that receives the release archive
  --help               show this help
EOF
}

die() {
  echo "ERROR: $*" >&2
  exit 1
}

require_option_value() {
  if [[ $# -lt 2 || -z "$2" ]]; then
    die "$1 requires a value"
  fi
}

validate_native_stage0_package() {
  local package_dir="$1"
  local expected_target="$2"

  python3 - "$package_dir" "$expected_target" <<'PY'
import json
import os
import pathlib
import sys

package_dir = pathlib.Path(sys.argv[1])
expected_target = sys.argv[2]
if not package_dir.is_dir() or package_dir.is_symlink():
    raise SystemExit(f"native stage0 package directory is invalid: {package_dir}")

for path in package_dir.rglob("*"):
    relative = path.relative_to(package_dir)
    if path.is_symlink() or not (path.is_file() or path.is_dir()):
        raise SystemExit(f"native stage0 package has an unsafe entry: {relative}")
    if path.name == ".DS_Store" or path.name.startswith("._") or "__MACOSX" in relative.parts:
        raise SystemExit(f"native stage0 package has unsupported macOS metadata: {relative}")

manifest_path = package_dir / "manifest.json"
if not manifest_path.is_file() or manifest_path.is_symlink():
    raise SystemExit(f"native stage0 manifest is required: {manifest_path}")
try:
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
except (OSError, json.JSONDecodeError) as error:
    raise SystemExit(f"native stage0 manifest is invalid: {error}")

if manifest.get("kind") != "lsharp-native-selfhost-stage0":
    raise SystemExit("native stage0 manifest kind is invalid")
if manifest.get("target") != expected_target:
    raise SystemExit(
        "native stage0 manifest target mismatch: "
        f"expected={expected_target} actual={manifest.get('target')!r}"
    )

for field in ("compiler", "transport_driver", "materializer"):
    value = manifest.get(field)
    if not isinstance(value, str) or not value:
        raise SystemExit(f"native stage0 manifest {field} is invalid")
    path = pathlib.PurePosixPath(value)
    if path.is_absolute() or ".." in path.parts or "." in path.parts:
        raise SystemExit(f"native stage0 manifest {field} must be a relative path")
    candidate = package_dir.joinpath(*path.parts)
    if not candidate.is_file() or candidate.is_symlink() or not os.access(candidate, os.X_OK):
        raise SystemExit(f"native stage0 executable is unavailable: {candidate}")
PY
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --target)
      require_option_value "$@"
      TARGET="$2"
      shift 2
      ;;
    --version)
      require_option_value "$@"
      VERSION="$2"
      shift 2
      ;;
    --stage0-dir)
      require_option_value "$@"
      STAGE0_DIR="$2"
      shift 2
      ;;
    --output-dir)
      require_option_value "$@"
      OUTPUT_DIR="$2"
      shift 2
      ;;
    --help)
      usage
      exit 0
      ;;
    *)
      die "unknown option: $1"
      ;;
  esac
done

[[ -n "$TARGET" ]] || die "--target is required"
[[ -n "$VERSION" ]] || die "--version is required"
[[ -n "$STAGE0_DIR" ]] || die "--stage0-dir is required"
[[ -n "$OUTPUT_DIR" ]] || die "--output-dir is required"

case "$TARGET" in
  x86_64-unknown-linux-gnu|aarch64-apple-darwin) ;;
  *) die "unsupported native target: $TARGET" ;;
esac
case "$VERSION" in
  *[!A-Za-z0-9._-]*|"") die "version must contain only ASCII letters, digits, dot, underscore, or hyphen: $VERSION" ;;
esac
[[ "$OUTPUT_DIR" != "/" && "$OUTPUT_DIR" != "." ]] \
  || die "unsafe output directory: $OUTPUT_DIR"

validate_native_stage0_package "$STAGE0_DIR" "$TARGET"

ARCHIVE_ROOT_NAME="lsharp-stage0-${VERSION}-${TARGET}"
ARCHIVE_NAME="${ARCHIVE_ROOT_NAME}.tar.gz"
mkdir -p "$OUTPUT_DIR"
WORK_DIR="$(mktemp -d "$OUTPUT_DIR/.native-stage0-release.XXXXXX")"
trap 'rm -rf "$WORK_DIR"' EXIT
PACKAGE_DIR="$WORK_DIR/$ARCHIVE_ROOT_NAME"
ARCHIVE_TMP="$WORK_DIR/$ARCHIVE_NAME"
ARCHIVE_PATH="$OUTPUT_DIR/$ARCHIVE_NAME"

cp -pR "$STAGE0_DIR/." "$PACKAGE_DIR"
rm -f "$PACKAGE_DIR/checksums.txt"
validate_native_stage0_package "$PACKAGE_DIR" "$TARGET"
"$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)/scripts/checksum.sh" "$PACKAGE_DIR" >"$PACKAGE_DIR/checksums.txt"

(
  cd "$WORK_DIR"
  COPYFILE_DISABLE=1 tar --no-xattrs --exclude '._*' --exclude '.DS_Store' --exclude '__MACOSX' \
    -czf "$ARCHIVE_TMP" "$ARCHIVE_ROOT_NAME"
)
[[ -s "$ARCHIVE_TMP" ]] || die "native stage0 release archive was not created: $ARCHIVE_TMP"
mv -f "$ARCHIVE_TMP" "$ARCHIVE_PATH"

echo "native stage0 release archive: $ARCHIVE_PATH"
