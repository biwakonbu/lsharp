#!/usr/bin/env bash
# OPS-06/PKG-01: release artifact 展開ベースで packaged binary smoke を行う
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
ARCHIVE_PATH="${1:-}"
ROLLBACK_ARCHIVE_PATH="${2:-}"
WORK_DIR="${WORK_DIR:-$ROOT/target/ci/release-smoke}"
MAX_ARCHIVE_BYTES="${LSHARP_RELEASE_SMOKE_MAX_ARCHIVE_BYTES:-536870912}"
RELEASE_IDENTITY_VERIFIER="${RELEASE_IDENTITY_VERIFIER:-$ROOT/scripts/ci/verify-native-release-identity.py}"
RELEASE_REVIEW_TRUST_STORE="${RELEASE_REVIEW_TRUST_STORE:-}"
RELEASE_REVIEW_LIFECYCLE="${RELEASE_REVIEW_LIFECYCLE:-}"
RELEASE_REVIEW_PROVIDER_ARGS=()

validate_work_dir() {
  WORK_DIR="$(python3 - "$WORK_DIR" <<'PY'
import pathlib
import sys

print(pathlib.Path(sys.argv[1]).resolve())
PY
)"
  python3 - "$ROOT" "$WORK_DIR" <<'PY'
import pathlib
import sys

root = pathlib.Path(sys.argv[1]).resolve()
work_dir = pathlib.Path(sys.argv[2])
unsafe_paths = {
    pathlib.Path("/"),
    pathlib.Path("/tmp"),
    pathlib.Path("/private/tmp"),
    root,
    root / "target",
    root / "target" / "ci",
}
if work_dir in unsafe_paths:
    raise SystemExit(f"ERROR: unsafe release smoke work directory: {work_dir}")
PY
}

validate_work_dir
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
  echo "Usage: $0 <release-archive> [rollback-archive]" >&2
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

validate_archive_input() {
  local kind="$1"
  local path="$2"
  if [[ ! -e "$path" ]]; then
    echo "ERROR: ${kind} archive not found: $path" >&2
    exit 1
  fi
  if [[ -L "$path" || ! -f "$path" ]]; then
    echo "ERROR: ${kind} archive must be a regular file without symlink: $path" >&2
    exit 1
  fi
}

validate_release_review_provider_inputs() {
  if [[ -z "$RELEASE_REVIEW_TRUST_STORE" && -z "$RELEASE_REVIEW_LIFECYCLE" ]]; then
    return 0
  fi
  if [[ -z "$RELEASE_REVIEW_TRUST_STORE" || -z "$RELEASE_REVIEW_LIFECYCLE" ]]; then
    echo "ERROR: RELEASE_REVIEW_TRUST_STORE and RELEASE_REVIEW_LIFECYCLE must be supplied together" >&2
    exit 1
  fi
  if [[ ! -s "$RELEASE_REVIEW_TRUST_STORE" ]]; then
    echo "ERROR: release review trust-store snapshot is not a non-empty file: $RELEASE_REVIEW_TRUST_STORE" >&2
    exit 1
  fi
  if [[ ! -s "$RELEASE_REVIEW_LIFECYCLE" ]]; then
    echo "ERROR: release review lifecycle snapshot is not a non-empty file: $RELEASE_REVIEW_LIFECYCLE" >&2
    exit 1
  fi
  RELEASE_REVIEW_PROVIDER_ARGS=(
    --trust-store "$RELEASE_REVIEW_TRUST_STORE"
    --review-lifecycle "$RELEASE_REVIEW_LIFECYCLE"
  )
}

validate_release_review_provider_inputs

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
  candidate="$(find "$extract_dir" -mindepth 1 -maxdepth 2 -type f \( -name 'program.native' -o -name 'lsharp' -o -name 'lsharp.exe' \) -print -quit)"
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

validate_archive_input "release" "$ARCHIVE_PATH"

echo "=== release-smoke: unpack artifact ==="
python3 - "$ARCHIVE_PATH" "$MAX_ARCHIVE_BYTES" <<'PY'
import pathlib
import stat
import sys
import tarfile
import zipfile

archive = pathlib.Path(sys.argv[1])
limit = int(sys.argv[2])
if archive.stat().st_size > limit:
    raise SystemExit("archive compressed size exceeds limit")

def validate_name(name: str) -> None:
    path = pathlib.PurePosixPath(name)
    if path.is_absolute() or ".." in path.parts:
        raise SystemExit(f"unsafe archive entry: {name}")

expanded = 0
entries = 0
if archive.name.endswith((".tar.gz", ".tgz")):
    with tarfile.open(archive, "r:gz") as bundle:
        for member in bundle.getmembers():
            entries += 1
            validate_name(member.name)
            if not (member.isfile() or member.isdir()):
                raise SystemExit(f"unsafe archive entry: {member.name}")
            if member.isfile():
                expanded += member.size
elif archive.name.endswith(".zip"):
    with zipfile.ZipFile(archive) as bundle:
        for member in bundle.infolist():
            entries += 1
            validate_name(member.filename)
            mode = member.external_attr >> 16
            if stat.S_ISLNK(mode):
                raise SystemExit(f"unsafe archive entry: {member.filename}")
            expanded += member.file_size
else:
    raise SystemExit(f"unsupported archive format: {archive}")
if entries > 256:
    raise SystemExit("archive entry count exceeds limit")
if expanded > limit:
    raise SystemExit("archive expanded size exceeds limit")
PY
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
  echo "ERROR: extracted archive root containing program.native or lsharp binary not found" >&2
  exit 1
}

PROGRAM_NATIVE="$ARCHIVE_ROOT/program.native"
LSHARP_BIN="$ARCHIVE_ROOT/lsharp"
if [[ ! -e "$LSHARP_BIN" && -e "$ARCHIVE_ROOT/lsharp.exe" ]]; then
  LSHARP_BIN="$ARCHIVE_ROOT/lsharp.exe"
fi
if [[ ! -e "$LSHARP_BIN" && -e "$PROGRAM_NATIVE" ]]; then
  LSHARP_BIN="$PROGRAM_NATIVE"
fi

LSHARP_LSP_BIN="$ARCHIVE_ROOT/lsharp-lsp"
if [[ ! -e "$LSHARP_LSP_BIN" && -e "$ARCHIVE_ROOT/lsharp-lsp.exe" ]]; then
  LSHARP_LSP_BIN="$ARCHIVE_ROOT/lsharp-lsp.exe"
fi

if [[ ! -e "$LSHARP_BIN" ]]; then
  echo "ERROR: packaged program.native or lsharp binary not found under $ARCHIVE_ROOT" >&2
  exit 1
fi

for required in README.md LICENSE checksums.txt; do
  if [[ ! -f "$ARCHIVE_ROOT/$required" ]]; then
    echo "ERROR: required release payload missing: $required" >&2
    exit 1
  fi
done

NATIVE_ONLY=0
if [[ -f "$PROGRAM_NATIVE" ]]; then
  NATIVE_ONLY=1
  if [[ ! -x "$PROGRAM_NATIVE" ]]; then
    echo "ERROR: native-only program.native is not executable: $PROGRAM_NATIVE" >&2
    exit 1
  fi
  if [[ ! -f "$ARCHIVE_ROOT/manifest.json" ]]; then
    echo "ERROR: native-only manifest.json not found under $ARCHIVE_ROOT" >&2
    exit 1
  fi
  if ! grep -q '"entry_binary"[[:space:]]*:[[:space:]]*"program.native"' "$ARCHIVE_ROOT/manifest.json"; then
    echo "ERROR: native-only manifest.json missing entry_binary program.native" >&2
    exit 1
  fi
  if ! grep -q 'rollback' "$ARCHIVE_ROOT/manifest.json"; then
    echo "ERROR: native-only manifest.json missing rollback anchor" >&2
    exit 1
  fi
  identity_path="$ARCHIVE_ROOT/review-evidence-identity.json"
  if [[ -f "$identity_path" ]]; then
    native_source_commit="$(python3 - "$ARCHIVE_ROOT/manifest.json" <<'PY'
import json
import pathlib
import sys

manifest = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
print(manifest.get("source_commit", ""))
PY
    )"
    python3 "$RELEASE_IDENTITY_VERIFIER" \
      --manifest "$ARCHIVE_ROOT/manifest.json" \
      --expected-identity "$identity_path" \
      --artifact "$PROGRAM_NATIVE" \
      --source-commit "$native_source_commit" \
      "${RELEASE_REVIEW_PROVIDER_ARGS[@]}" \
      --require-provider-input >/dev/null
  elif [[ -n "$RELEASE_REVIEW_TRUST_STORE" || -n "$RELEASE_REVIEW_LIFECYCLE" ]]; then
    echo "ERROR: release identity snapshots require review-evidence-identity" >&2
    exit 1
  elif grep -q 'review_evidence_identity' "$ARCHIVE_ROOT/manifest.json"; then
    echo "ERROR: native-only manifest declares review_evidence_identity without its payload" >&2
    exit 1
  elif [[ "${NATIVE_ONLY_REQUIRE_REVIEW_EVIDENCE_IDENTITY:-0}" == "1" ]]; then
    echo "ERROR: native-only release identity is required but missing" >&2
    exit 1
  fi
else
  if [[ ! -e "$LSHARP_LSP_BIN" ]]; then
    echo "ERROR: packaged lsharp-lsp binary not found under $ARCHIVE_ROOT" >&2
    exit 1
  fi

  COMPONENT_SIDECAR="$ARCHIVE_ROOT/lsharp.component.wasm"
  if [[ ! -f "$COMPONENT_SIDECAR" ]]; then
    echo "ERROR: rollback compatibility guest component sidecar not found under $ARCHIVE_ROOT" >&2
    exit 1
  fi

  if ! xxd -p -l 4 "$COMPONENT_SIDECAR" | grep -qi '^0061736d$'; then
    echo "ERROR: rollback compatibility guest component sidecar is not a Wasm binary: $COMPONENT_SIDECAR" >&2
    exit 1
  fi
  if [[ ! -f "$ARCHIVE_ROOT/manifest.json" ]]; then
    echo "ERROR: rollback compatibility manifest.json not found under $ARCHIVE_ROOT" >&2
    exit 1
  fi
  python3 - "$ARCHIVE_ROOT/manifest.json" \
    "${EXPECTED_ROLLBACK_TARGET:-}" \
    "${EXPECTED_ROLLBACK_SOURCE_COMMIT:-}" \
    "${EXPECTED_ROLLBACK_VERSION:-}" <<'PY'
import json
import pathlib
import sys

manifest = json.loads(pathlib.Path(sys.argv[1]).read_text())
expected_target = sys.argv[2]
expected_source_commit = sys.argv[3]
expected_version = sys.argv[4]
if manifest.get("archive_kind") != "rollback compatibility":
    raise SystemExit("rollback compatibility manifest kind mismatch")
if expected_target and manifest.get("target") != expected_target:
    raise SystemExit("rollback compatibility manifest target mismatch")
if expected_version and manifest.get("version") != expected_version:
    raise SystemExit("rollback compatibility manifest version mismatch")
if expected_source_commit and manifest.get("source_commit") != expected_source_commit:
    raise SystemExit("rollback compatibility manifest source commit mismatch")
for required in ("target", "version", "source_commit"):
    if not manifest.get(required):
        raise SystemExit(f"rollback compatibility manifest missing {required}")
expected_payloads = {
    "entry_binary": "lsharp",
    "lsp_binary": "lsharp-lsp",
    "component": "lsharp.component.wasm",
}
for field, expected in expected_payloads.items():
    if manifest.get(field) != expected:
        raise SystemExit(f"rollback compatibility manifest {field} mismatch")
PY
  for required_checksum in README.md LICENSE lsharp lsharp-lsp lsharp.component.wasm manifest.json; do
    if ! awk '{print $2}' "$ARCHIVE_ROOT/checksums.txt" | grep -Fxq "$required_checksum"; then
      echo "ERROR: rollback compatibility checksums.txt missing required entry: $required_checksum" >&2
      exit 1
    fi
  done
fi

if [[ -e "$ARCHIVE_ROOT/CHANGELOG.md" ]]; then
  echo "INFO: optional payload present: CHANGELOG.md"
fi

echo "=== release-smoke: verify checksums ==="
python3 - "$ARCHIVE_ROOT/checksums.txt" <<'PY'
import pathlib
import sys

checksums_path = pathlib.Path(sys.argv[1])
for line in checksums_path.read_text().splitlines():
    fields = line.split()
    if len(fields) < 2:
        continue
    relpath = pathlib.PurePosixPath(fields[1])
    if relpath.is_absolute() or ".." in relpath.parts:
        raise SystemExit(f"unsafe checksum target: {fields[1]}")
PY
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

if [[ "$NATIVE_ONLY" == "1" ]]; then
  if [[ -z "$ROLLBACK_ARCHIVE_PATH" || ! -s "$ROLLBACK_ARCHIVE_PATH" ]]; then
    echo "ERROR: rollback compatibility archive is required" >&2
    exit 1
  fi
  validate_archive_input "rollback compatibility" "$ROLLBACK_ARCHIVE_PATH"
  rollback_name="$(basename "$ROLLBACK_ARCHIVE_PATH")"
  rollback_sha256="$(hash_file "$ROLLBACK_ARCHIVE_PATH")"
  python3 - \
    "$ARCHIVE_ROOT/manifest.json" \
    "$rollback_name" \
    "$rollback_sha256" \
    "$ARCHIVE_ROOT/native-program-manifest.json" \
    "$PROGRAM_NATIVE" \
    "${VERSION:-}" <<'PY'
import hashlib
import json
import pathlib
import sys

manifest = json.loads(pathlib.Path(sys.argv[1]).read_text())
rollback_name = sys.argv[2]
rollback_sha256 = sys.argv[3]
native_manifest = json.loads(pathlib.Path(sys.argv[4]).read_text())
program_sha256 = hashlib.sha256(pathlib.Path(sys.argv[5]).read_bytes()).hexdigest()
expected_version = sys.argv[6]
anchor = manifest.get("rollback_anchor", {})
if expected_version and manifest.get("version") != expected_version:
    raise SystemExit("native-only manifest version mismatch")
if anchor.get("kind") != "rollback compatibility":
    raise SystemExit("rollback compatibility anchor kind mismatch")
if anchor.get("asset") != rollback_name:
    raise SystemExit("rollback compatibility asset name mismatch")
if anchor.get("rollback_sha256") != rollback_sha256:
    raise SystemExit("rollback compatibility asset checksum mismatch")
native_input = manifest.get("native_program_input", {})
input_manifest = native_input.get("manifest")
if not input_manifest or not (pathlib.Path(sys.argv[1]).parent / input_manifest).is_file():
    raise SystemExit("native App.Cli input manifest is missing from archive")
if native_manifest.get("target") != manifest.get("target"):
    raise SystemExit("native App.Cli input target mismatch")
if native_manifest.get("entry_module") != "App.Cli":
    raise SystemExit("native App.Cli input entry_module must be App.Cli")
if native_manifest.get("source") != "src/App/Cli.ls":
    raise SystemExit("native App.Cli input source must be src/App/Cli.ls")
if native_manifest.get("source_commit") != manifest.get("source_commit"):
    raise SystemExit("native App.Cli input source commit mismatch")
if native_manifest.get("selfhost_fixed_point") is not True:
    raise SystemExit("native App.Cli input selfhost fixed-point evidence is required")
if native_manifest.get("program_sha256") != program_sha256:
    raise SystemExit("native App.Cli input program sha256 mismatch")
if native_input.get("input_sha256") != program_sha256:
    raise SystemExit("native App.Cli archive input sha256 mismatch")
PY
  for required_checksum in program.native lsharp manifest.json native-program-manifest.json; do
    if ! awk '{print $2}' "$ARCHIVE_ROOT/checksums.txt" | grep -Fxq "$required_checksum"; then
      echo "ERROR: native-only checksums.txt missing required entry: $required_checksum" >&2
      exit 1
    fi
  done
fi

SMOKE_SOURCE="$SMOKE_DIR/quickstart.ls"
SMOKE_METADATA_SOURCE="$SMOKE_DIR/quickstart-metadata.ls"
SMOKE_WASM="$SMOKE_DIR/quickstart.wasm"
SMOKE_DOC_HTML="$SMOKE_DIR/quickstart.html"
SMOKE_DOC_JSON="$SMOKE_DIR/api.json"
SMOKE_SOURCE_NAME="$(basename "$SMOKE_SOURCE")"
SMOKE_METADATA_SOURCE_NAME="$(basename "$SMOKE_METADATA_SOURCE")"
SMOKE_WASM_NAME="$(basename "$SMOKE_WASM")"

# embedded component は current directory だけを filesystem として利用する。
run_smoke_cli() {
  (
    cd "$SMOKE_DIR"
    "$LSHARP_BIN" "$@"
  )
}

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
version_output="$("$LSHARP_BIN" --version)"
if [[ -n "${VERSION:-}" ]]; then
  expected_cli_version="lsharp ${VERSION#v}"
  if [[ "${version_output}" != "${expected_cli_version}" ]]; then
    echo "ERROR: packaged CLI version mismatch: expected=${expected_cli_version} actual=${version_output}" >&2
    exit 1
  fi
fi
if [[ "$NATIVE_ONLY" == "1" ]]; then
  # native-only App.Cli smoke is limited to --version and --help.
  # The producer certifies the boot surface; host-launcher archives retain the
  # broader source-file command smoke below.
  native_help_output="$("$LSHARP_BIN" --help)"
  if [[ "$native_help_output" != *"Usage: lsharp"* ]]; then
    echo "ERROR: native-only App.Cli help output is invalid" >&2
    exit 1
  fi
else
  (
    cd "$SMOKE_DIR"
    lsp_version_output="$("$LSHARP_LSP_BIN" --version)"
    if [[ -n "${VERSION:-}" ]]; then
      expected_lsp_version="lsharp ${VERSION#v}"
      if [[ "$lsp_version_output" != "$expected_lsp_version" ]]; then
        echo "ERROR: packaged LSP version mismatch: expected=${expected_lsp_version} actual=${lsp_version_output}" >&2
        exit 1
      fi
    fi
  )
  run_smoke_cli check "$SMOKE_SOURCE_NAME" >/dev/null
  run_smoke_cli fmt "$SMOKE_SOURCE_NAME" >/dev/null
  run_smoke_cli test "$SMOKE_METADATA_SOURCE_NAME" >/dev/null
  run_smoke_cli compile "$SMOKE_SOURCE_NAME" -o "$SMOKE_WASM_NAME" >/dev/null
  run_smoke_cli doc "$SMOKE_METADATA_SOURCE_NAME" -o "$(basename "$SMOKE_DOC_HTML")" >/dev/null
  run_smoke_cli doc "$SMOKE_METADATA_SOURCE_NAME" --json -o "$(basename "$SMOKE_DOC_JSON")" >/dev/null
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
fi

if [[ "$NATIVE_ONLY" == "1" ]]; then
  rollback_work_dir="${WORK_DIR}-rollback"
  read -r rollback_target rollback_source_commit rollback_version < <(
    python3 - "$ARCHIVE_ROOT/manifest.json" <<'PY'
import json
import pathlib
import sys
manifest = json.loads(pathlib.Path(sys.argv[1]).read_text())
print(manifest.get("target", ""), manifest.get("source_commit", ""), manifest.get("version", ""))
PY
  )
  WORK_DIR="$rollback_work_dir" \
    VERSION="$rollback_version" \
    RELEASE_REVIEW_TRUST_STORE="" \
    RELEASE_REVIEW_LIFECYCLE="" \
    EXPECTED_ROLLBACK_TARGET="$rollback_target" \
    EXPECTED_ROLLBACK_SOURCE_COMMIT="$rollback_source_commit" \
    EXPECTED_ROLLBACK_VERSION="$rollback_version" \
    bash "$0" "$ROLLBACK_ARCHIVE_PATH"
fi

echo "release-smoke: OK"
