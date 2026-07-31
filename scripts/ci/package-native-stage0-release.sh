#!/usr/bin/env bash
set -euo pipefail

TARGET=""
VERSION=""
STAGE0_DIR=""
OUTPUT_DIR=""
SOURCE_COMMIT=""
REVIEW_EVIDENCE_IDENTITY=""
REVIEW_EVIDENCE_IDENTITY_JSON=""
REVIEW_IDENTITY_VERIFIER="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)/scripts/ci/verify-native-release-identity.py"

usage() {
  cat <<'EOF'
usage: scripts/ci/package-native-stage0-release.sh --target TARGET --version VERSION --stage0-dir DIR --output-dir DIR --source-commit COMMIT [--review-evidence-identity FILE]

options:
  --target TARGET      x86_64-unknown-linux-gnu or aarch64-apple-darwin
  --version VERSION    release version, for example v0.1.0
  --stage0-dir DIR     verified native stage0 package directory
  --output-dir DIR     directory that receives the release archive
  --source-commit COMMIT  40-hex source commit used to build the stage0 package
  --review-evidence-identity FILE
                       optional explicit review evidence identity JSON
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
  local expected_source_commit="${3:-}"

  python3 - "$package_dir" "$expected_target" "$expected_source_commit" <<'PY'
import json
import os
import pathlib
import sys

package_dir = pathlib.Path(sys.argv[1])
expected_target = sys.argv[2]
expected_source_commit = sys.argv[3]
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
if expected_source_commit and manifest.get("source_commit") != expected_source_commit:
    raise SystemExit(
        "native stage0 manifest source commit mismatch: "
        f"expected={expected_source_commit} actual={manifest.get('source_commit')!r}"
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
    --source-commit)
      require_option_value "$@"
      SOURCE_COMMIT="$2"
      shift 2
      ;;
    --review-evidence-identity)
      require_option_value "$@"
      REVIEW_EVIDENCE_IDENTITY="$2"
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
[[ -n "$SOURCE_COMMIT" ]] || die "--source-commit is required"

case "$TARGET" in
  x86_64-unknown-linux-gnu|aarch64-apple-darwin) ;;
  *) die "unsupported native target: $TARGET" ;;
esac
case "$VERSION" in
  *[!A-Za-z0-9._-]*|"") die "version must contain only ASCII letters, digits, dot, underscore, or hyphen: $VERSION" ;;
esac
[[ "$SOURCE_COMMIT" =~ ^[0-9a-f]{40}$ ]] \
  || die "source commit must be a 40-character lowercase hexadecimal commit: $SOURCE_COMMIT"
[[ "$OUTPUT_DIR" != "/" && "$OUTPUT_DIR" != "." ]] \
  || die "unsafe output directory: $OUTPUT_DIR"

validate_native_stage0_package "$STAGE0_DIR" "$TARGET"

if [[ -n "$REVIEW_EVIDENCE_IDENTITY" ]]; then
  [[ -s "$REVIEW_EVIDENCE_IDENTITY" ]] \
    || die "review evidence identity is not a non-empty file: $REVIEW_EVIDENCE_IDENTITY"
  REVIEW_EVIDENCE_IDENTITY_JSON="$(python3 "$REVIEW_IDENTITY_VERIFIER" \
    --identity "$REVIEW_EVIDENCE_IDENTITY" \
    --source-commit "$SOURCE_COMMIT" \
    --require-provider-input)"
fi

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
if [[ -n "$REVIEW_EVIDENCE_IDENTITY" ]]; then
  printf '%s\n' "$REVIEW_EVIDENCE_IDENTITY_JSON" >"$PACKAGE_DIR/review-evidence-identity.json"
fi
python3 - "$PACKAGE_DIR/manifest.json" "$SOURCE_COMMIT" <<'PY'
import json
import pathlib
import sys

manifest_path = pathlib.Path(sys.argv[1])
source_commit = sys.argv[2]
manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
existing = manifest.get("source_commit")
if existing is not None and existing != source_commit:
    raise SystemExit(
        "native stage0 source commit does not match the release input: "
        f"expected={source_commit} actual={existing}"
    )
manifest["source_commit"] = source_commit
identity_path = manifest_path.parent / "review-evidence-identity.json"
if identity_path.is_file():
    manifest["review_evidence_identity"] = json.loads(
        identity_path.read_text(encoding="utf-8")
    )
manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
PY
validate_native_stage0_package "$PACKAGE_DIR" "$TARGET" "$SOURCE_COMMIT"
"$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)/scripts/checksum.sh" "$PACKAGE_DIR" >"$PACKAGE_DIR/checksums.txt"

(
  cd "$WORK_DIR"
  COPYFILE_DISABLE=1 tar --no-xattrs --exclude '._*' --exclude '.DS_Store' --exclude '__MACOSX' \
    -czf "$ARCHIVE_TMP" "$ARCHIVE_ROOT_NAME"
)
[[ -s "$ARCHIVE_TMP" ]] || die "native stage0 release archive was not created: $ARCHIVE_TMP"
mv -f "$ARCHIVE_TMP" "$ARCHIVE_PATH"

echo "native stage0 release archive: $ARCHIVE_PATH"
