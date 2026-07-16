#!/usr/bin/env bash
set -euo pipefail

TARGET=""
SOURCE_COMMIT="${NATIVE_STAGE0_SOURCE_COMMIT:-}"
COMPILER=""
TRANSPORT_DRIVER=""
MATERIALIZER=""
OUTPUT_DIR=""

COMPILER_PATH="bin/compiler"
TRANSPORT_DRIVER_PATH="bin/transport-driver"
MATERIALIZER_SCRIPT_PATH="bin/materializer.py"
MATERIALIZER_WRAPPER_PATH="bin/materializer"

usage() {
  cat <<'EOF'
usage: scripts/ci/package-native-stage0.sh --target TARGET --source-commit COMMIT --compiler PATH --transport-driver PATH --materializer PATH --output-dir DIR

options:
  --target TARGET             x86_64-unknown-linux-gnu or aarch64-apple-darwin
  --source-commit COMMIT      40-character lowercase source commit (defaults to current HEAD)
  --compiler PATH             executable native compiler
  --transport-driver PATH     executable native transport driver
  --materializer PATH         nonempty materializer Python script
  --output-dir DIR            new stage0 package directory
  --help                      show this help
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

require_executable_input() {
  local label="$1"
  local path="$2"
  [[ -f "$path" ]] || die "$label is not a regular file: $path"
  [[ ! -L "$path" ]] || die "$label must not be a symbolic link: $path"
  [[ -x "$path" ]] || die "$label is not executable: $path"
}

require_materializer_input() {
  local path="$1"
  [[ -f "$path" ]] || die "materializer is not a regular file: $path"
  [[ ! -L "$path" ]] || die "materializer must not be a symbolic link: $path"
  [[ -s "$path" ]] || die "materializer is empty: $path"
}

validate_manifest_path() {
  local field="$1"
  local path="$2"
  case "$path" in
    ""|/*|*//*|.|..|./*|../*|*/./*|*/../*|*/.|*/..)
      die "manifest $field must be a relative path without traversal: $path"
      ;;
  esac
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --target)
      require_option_value "$@"
      TARGET="$2"
      shift 2
      ;;
    --source-commit)
      require_option_value "$@"
      SOURCE_COMMIT="$2"
      shift 2
      ;;
    --compiler)
      require_option_value "$@"
      COMPILER="$2"
      shift 2
      ;;
    --transport-driver)
      require_option_value "$@"
      TRANSPORT_DRIVER="$2"
      shift 2
      ;;
    --materializer)
      require_option_value "$@"
      MATERIALIZER="$2"
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
[[ -n "$COMPILER" ]] || die "--compiler is required"
[[ -n "$TRANSPORT_DRIVER" ]] || die "--transport-driver is required"
[[ -n "$MATERIALIZER" ]] || die "--materializer is required"
[[ -n "$OUTPUT_DIR" ]] || die "--output-dir is required"

if [[ -z "$SOURCE_COMMIT" ]]; then
  SOURCE_COMMIT="$(cd "$ROOT" && git rev-parse --verify HEAD 2>/dev/null)" \
    || die "source commit is required when the repository HEAD cannot be resolved"
fi
[[ "$SOURCE_COMMIT" =~ ^[0-9a-f]{40}$ ]] \
  || die "source commit must be a 40-character lowercase hexadecimal commit: $SOURCE_COMMIT"

case "$TARGET" in
  x86_64-unknown-linux-gnu|aarch64-apple-darwin) ;;
  *) die "unsupported native target: $TARGET" ;;
esac

require_executable_input "compiler" "$COMPILER"
require_executable_input "transport driver" "$TRANSPORT_DRIVER"
require_materializer_input "$MATERIALIZER"

validate_manifest_path "compiler" "$COMPILER_PATH"
validate_manifest_path "transport_driver" "$TRANSPORT_DRIVER_PATH"
validate_manifest_path "materializer" "$MATERIALIZER_WRAPPER_PATH"
validate_manifest_path "materializer script" "$MATERIALIZER_SCRIPT_PATH"

[[ ! -e "$OUTPUT_DIR" && ! -L "$OUTPUT_DIR" ]] || die "output directory already exists: $OUTPUT_DIR"

OUTPUT_PARENT="$(dirname "$OUTPUT_DIR")"
mkdir -p "$OUTPUT_PARENT"
WORK_DIR="$(mktemp -d "$OUTPUT_PARENT/.native-stage0-package.XXXXXX")"
trap 'rm -rf "$WORK_DIR"' EXIT
PACKAGE_DIR="$WORK_DIR/package"

mkdir -p "$PACKAGE_DIR/bin"
cp -p "$COMPILER" "$PACKAGE_DIR/$COMPILER_PATH"
cp -p "$TRANSPORT_DRIVER" "$PACKAGE_DIR/$TRANSPORT_DRIVER_PATH"
cp -p "$MATERIALIZER" "$PACKAGE_DIR/$MATERIALIZER_SCRIPT_PATH"
chmod 0755 \
  "$PACKAGE_DIR/$COMPILER_PATH" \
  "$PACKAGE_DIR/$TRANSPORT_DRIVER_PATH"

cat >"$PACKAGE_DIR/$MATERIALIZER_WRAPPER_PATH" <<'SH'
#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec python3 "$SCRIPT_DIR/materializer.py" "$@"
SH
chmod 0755 "$PACKAGE_DIR/$MATERIALIZER_WRAPPER_PATH"

cat >"$PACKAGE_DIR/manifest.json" <<EOF
{
  "kind": "lsharp-native-selfhost-stage0",
  "target": "$TARGET",
  "source_commit": "$SOURCE_COMMIT",
  "compiler": "$COMPILER_PATH",
  "transport_driver": "$TRANSPORT_DRIVER_PATH",
  "materializer": "$MATERIALIZER_WRAPPER_PATH"
}
EOF

for package_file in \
  "$PACKAGE_DIR/$COMPILER_PATH" \
  "$PACKAGE_DIR/$TRANSPORT_DRIVER_PATH" \
  "$PACKAGE_DIR/$MATERIALIZER_SCRIPT_PATH" \
  "$PACKAGE_DIR/$MATERIALIZER_WRAPPER_PATH" \
  "$PACKAGE_DIR/manifest.json"; do
  [[ -f "$package_file" ]] || die "package file was not created: $package_file"
done

mv "$PACKAGE_DIR" "$OUTPUT_DIR"

echo "native stage0 package: $OUTPUT_DIR ($TARGET)"
