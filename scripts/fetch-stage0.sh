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

validate_release_base_url() {
  local base_url="$1"
  python3 - "$base_url" <<'PY'
from urllib.parse import urlsplit
import sys

raw_url = sys.argv[1]
try:
    parsed = urlsplit(raw_url)
except ValueError:
    raise SystemExit("ERROR: native stage0 release URL is invalid")

if parsed.username is not None or parsed.password is not None:
    raise SystemExit("ERROR: native stage0 release URL must not include credentials")
if parsed.query or parsed.fragment:
    raise SystemExit(
        "ERROR: native stage0 release URL must not include a query or fragment"
    )

scheme = parsed.scheme.lower()
if scheme == "https":
    if not parsed.netloc:
        raise SystemExit("ERROR: native stage0 release URL must include an HTTPS host")
elif scheme == "file":
    if parsed.netloc not in ("", "localhost") or not parsed.path:
        raise SystemExit(
            "ERROR: native stage0 release URL must use https:// or local file://"
        )
else:
    raise SystemExit(
        "ERROR: native stage0 release URL must use https:// or local file://"
    )
PY
}

validate_archive() {
  local archive_path="$1"
  local expected_root="$2"

  python3 - "$archive_path" "$expected_root" <<'PY'
import pathlib
import tarfile
import sys

archive_path = pathlib.Path(sys.argv[1])
expected_root = sys.argv[2]
try:
    with tarfile.open(archive_path, "r:gz") as archive:
        members = archive.getmembers()
except (OSError, tarfile.TarError) as error:
    raise SystemExit(f"native stage0 archive is invalid: {error}")

if not members:
    raise SystemExit("native stage0 archive is empty")
for member in members:
    path = pathlib.PurePosixPath(member.name)
    if path.is_absolute() or ".." in path.parts or not path.parts or path.parts[0] != expected_root:
        raise SystemExit(f"unsafe native stage0 archive entry: {member.name}")
    if not member.isdir() and not member.isfile():
        raise SystemExit(f"unsafe native stage0 archive entry type: {member.name}")
PY
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
    case "${relpath:-}" in
      ""|/*|..|../*|*/../*|*/..)
        echo "ERROR: unsafe package checksum path: ${relpath:-missing}" >&2
        exit 1
        ;;
    esac
    local actual
    actual="$(hash_file "$package_dir/$relpath")"
    if [[ "$expected" != "$actual" ]]; then
      echo "ERROR: package checksum mismatch for $relpath" >&2
      exit 1
    fi
  done < "$checksums_path"

  python3 - "$package_dir" "$checksums_path" <<'PY'
import pathlib
import sys

package_dir = pathlib.Path(sys.argv[1])
checksums_path = pathlib.Path(sys.argv[2])
listed = set()
for line in checksums_path.read_text(encoding="utf-8").splitlines():
    if not line.strip():
        continue
    fields = line.split(None, 1)
    if len(fields) != 2:
        raise SystemExit("invalid package checksum entry")
    relative = fields[1].strip()
    if relative in listed:
        raise SystemExit(f"duplicate package checksum entry: {relative}")
    listed.add(relative)

actual = set()
for path in package_dir.rglob("*"):
    if path.is_symlink():
        raise SystemExit(f"package payload symlink is not allowed: {path.relative_to(package_dir)}")
    if path.is_file() and path.name != "checksums.txt":
        actual.add(path.relative_to(package_dir).as_posix())

missing = sorted(actual - listed)
if missing:
    raise SystemExit(
        "package files not listed in package checksums: " + ", ".join(missing)
    )
PY
}

validate_native_stage0_package() {
  local package_dir="$1"
  local expected_target="$2"
  local expected_source_commit="$3"

  python3 - "$package_dir" "$expected_target" "$expected_source_commit" <<'PY'
import json
import os
import pathlib
import re
import sys

package_dir = pathlib.Path(sys.argv[1])
expected_target = sys.argv[2]
expected_source_commit = sys.argv[3]
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
source_commit = manifest.get("source_commit")
if not isinstance(source_commit, str) or re.fullmatch(r"[0-9a-f]{40}", source_commit) is None:
    raise SystemExit("native stage0 manifest source_commit must be a 40-character lowercase hexadecimal commit")
if source_commit != expected_source_commit:
    raise SystemExit(
        "native stage0 source_commit does not match current checkout: "
        f"expected={expected_source_commit} actual={source_commit}"
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

install_stage0_package() {
  local package_dir="$1"
  local parent_dir
  local stage0_name
  local temporary_dir
  local backup_dir=""

  [[ "${STAGE0_DIR}" != "/" && "${STAGE0_DIR}" != "." ]] \
    || { echo "ERROR: unsafe STAGE0_DIR: ${STAGE0_DIR}" >&2; exit 1; }
  parent_dir="$(dirname "${STAGE0_DIR}")"
  stage0_name="$(basename "${STAGE0_DIR}")"
  mkdir -p "${parent_dir}"
  temporary_dir="$(mktemp -d "${parent_dir}/.${stage0_name}.new.XXXXXX")"
  cp -pR "${package_dir}/." "${temporary_dir}/"
  validate_native_stage0_package "${temporary_dir}" "${TARGET}" "${CURRENT_SOURCE_COMMIT}"

  if [[ -e "${STAGE0_DIR}" || -L "${STAGE0_DIR}" ]]; then
    backup_dir="$(mktemp -d "${parent_dir}/.${stage0_name}.previous.XXXXXX")"
    rmdir "${backup_dir}"
    if ! mv "${STAGE0_DIR}" "${backup_dir}"; then
      rm -rf "${backup_dir}" "${temporary_dir}"
      return 1
    fi
  fi
  local install_status=0
  if mv "${temporary_dir}" "${STAGE0_DIR}"; then
    temporary_dir=""
  else
    install_status=$?
    if [[ -n "${backup_dir}" ]]; then
      if ! mv "${backup_dir}" "${STAGE0_DIR}"; then
        echo "WARNING: stage0 restore move failed; attempting copy recovery: ${STAGE0_DIR}" >&2
        if mkdir -p "${STAGE0_DIR}" && cp -pR "${backup_dir}/." "${STAGE0_DIR}/"; then
          rm -rf "${backup_dir}"
          backup_dir=""
        else
          echo "ERROR: failed to restore previous stage0 package; backup retained at ${backup_dir}" >&2
        fi
      else
        backup_dir=""
      fi
    fi
    rm -rf "${temporary_dir}"
    return "${install_status}"
  fi
  if [[ -n "${backup_dir}" ]]; then
    rm -rf "${backup_dir}"
  fi
}

if [[ -z "$VERSION" ]]; then
  echo "ERROR: stage0 version is required. Pass it as the first arg or STAGE0_VERSION." >&2
  exit 1
fi

TARGET="$(detect_target)"
CURRENT_SOURCE_COMMIT="$(git -C "$ROOT" rev-parse --verify HEAD 2>/dev/null)" \
  || { echo "ERROR: current checkout source_commit could not be determined" >&2; exit 1; }
if [[ ! "$CURRENT_SOURCE_COMMIT" =~ ^[0-9a-f]{40}$ ]]; then
  echo "ERROR: current checkout source_commit is not a 40-character lowercase hexadecimal commit" >&2
  exit 1
fi
ARCHIVE_EXT="$(detect_archive_ext "$TARGET")"
ARCHIVE_ROOT_NAME="lsharp-stage0-${VERSION}-${TARGET}"
ARCHIVE_NAME="${ARCHIVE_ROOT_NAME}.${ARCHIVE_EXT}"
if [[ -z "$RELEASE_BASE_URL" ]]; then
  RELEASE_BASE_URL="https://github.com/biwakonbu/lsharp/releases/download/${VERSION}"
fi
validate_release_base_url "$RELEASE_BASE_URL"

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
validate_archive "$ARCHIVE_PATH" "$ARCHIVE_ROOT_NAME"
extract_archive "$ARCHIVE_PATH" "$EXTRACT_DIR"
ARCHIVE_ROOT="$EXTRACT_DIR/$ARCHIVE_ROOT_NAME"
[[ -d "$ARCHIVE_ROOT" ]] || { echo "ERROR: extracted native stage0 root not found: $ARCHIVE_ROOT" >&2; exit 1; }

echo "=== fetch-stage0: verify package checksum ==="
verify_package_checksums "$ARCHIVE_ROOT"
validate_native_stage0_package "$ARCHIVE_ROOT" "$TARGET" "$CURRENT_SOURCE_COMMIT"

echo "=== fetch-stage0: install stage0 package ==="
install_stage0_package "$ARCHIVE_ROOT"

echo "fetch-stage0: OK ($STAGE0_DIR)"
