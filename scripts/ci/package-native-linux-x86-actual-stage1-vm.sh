#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ACTUAL_STAGE1_INPUT=""
OUTPUT_DIR=""
VM_NAME="${LSHARP_NATIVE_LINUX_X86_VM_NAME:-lsharp-linux-x86}"
MATERIALIZER="$ROOT/scripts/ci/materialize-native-linux-x86-bundle.py"
PACKAGE_BUILDER="$ROOT/scripts/ci/package-native-stage0.sh"
TRANSPORT_DRIVER="$ROOT/scripts/ci/native-stage0-transport-linux-x86.sh"
HOST_WORK_DIR=""
VM_WORK_DIR=""
VM_WORK_DIR_CREATED=0

usage() {
  cat <<'EOF'
usage: scripts/ci/package-native-linux-x86-actual-stage1-vm.sh --actual-stage1-dir DIR --output-dir DIR [--vm-name NAME]

options:
  --actual-stage1-dir DIR  validated Linux x86 actual stage1 bundle directory
  --output-dir DIR         new native stage0 package directory
  --vm-name NAME           Lima VM name (default: lsharp-linux-x86)
  --help                   show this help
EOF
}

die() {
  echo "ERROR: $*" >&2
  exit 1
}

SOURCE_COMMIT="$(git -C "$ROOT" rev-parse --verify HEAD 2>/dev/null || true)"
[[ "$SOURCE_COMMIT" =~ ^[0-9a-f]{40}$ ]] \
  || die "current checkout source commit is unavailable: $SOURCE_COMMIT"

require_option_value() {
  if [[ $# -lt 2 || -z "$2" ]]; then
    die "$1 requires a value"
  fi
}

cleanup() {
  if [[ -n "$HOST_WORK_DIR" ]]; then
    rm -rf "$HOST_WORK_DIR"
  fi
  if [[ "$VM_WORK_DIR_CREATED" -eq 1 ]] && command -v limactl >/dev/null 2>&1; then
    limactl shell "$VM_NAME" -- rm -rf "$VM_WORK_DIR" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

while [[ $# -gt 0 ]]; do
  case "$1" in
    --actual-stage1-dir)
      require_option_value "$@"
      ACTUAL_STAGE1_INPUT="$2"
      shift 2
      ;;
    --output-dir)
      require_option_value "$@"
      OUTPUT_DIR="$2"
      shift 2
      ;;
    --vm-name)
      require_option_value "$@"
      VM_NAME="$2"
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

[[ -n "$ACTUAL_STAGE1_INPUT" ]] || die "--actual-stage1-dir is required"
[[ -n "$OUTPUT_DIR" ]] || die "--output-dir is required"
command -v limactl >/dev/null 2>&1 || die "limactl is required"
[[ -f "$MATERIALIZER" && -s "$MATERIALIZER" ]] || die "materializer is unavailable: $MATERIALIZER"
[[ -x "$PACKAGE_BUILDER" ]] || die "stage0 package builder is unavailable: $PACKAGE_BUILDER"
[[ -x "$TRANSPORT_DRIVER" ]] || die "Linux x86 transport driver is unavailable: $TRANSPORT_DRIVER"
[[ ! -e "$OUTPUT_DIR" && ! -L "$OUTPUT_DIR" ]] || die "output directory already exists: $OUTPUT_DIR"

if [[ "$ACTUAL_STAGE1_INPUT" = /* ]]; then
  ACTUAL_STAGE1_DIR="$ACTUAL_STAGE1_INPUT"
else
  ACTUAL_STAGE1_DIR="$ROOT/$ACTUAL_STAGE1_INPUT"
fi

for file in stage1-code.bin stage1-data.bin entrypoint-offset.txt function-start-len.txt main-func-idx.txt manifest.json seed.ls; do
  [[ -s "$ACTUAL_STAGE1_DIR/$file" ]] || die "actual stage1 artifact is missing: $ACTUAL_STAGE1_DIR/$file"
done

python3 - "$ACTUAL_STAGE1_DIR" "$SOURCE_COMMIT" <<'PY'
import json
import pathlib
import sys

artifact_dir = pathlib.Path(sys.argv[1])
manifest = json.loads((artifact_dir / "manifest.json").read_text(encoding="utf-8"))
expected_source_commit = sys.argv[2]

def read_int(name: str) -> int:
    return int((artifact_dir / name).read_text(encoding="utf-8").strip())

code_len = (artifact_dir / "stage1-code.bin").stat().st_size
data_len = (artifact_dir / "stage1-data.bin").stat().st_size
entrypoint_offset = read_int("entrypoint-offset.txt")
function_start_len = read_int("function-start-len.txt")
main_func_idx = read_int("main-func-idx.txt")

checks = [
    (manifest.get("target") == "x86_64-unknown-linux-gnu", "target"),
    (manifest.get("source_commit") == expected_source_commit, "source_commit"),
    (manifest.get("code_len") == code_len, "code_len"),
    (manifest.get("data_len") == data_len, "data_len"),
    (manifest.get("entrypoint_offset") == entrypoint_offset, "entrypoint_offset"),
    (manifest.get("function_start_len") == function_start_len, "function_start_len"),
    (manifest.get("main_func_idx") == main_func_idx, "main_func_idx"),
    (0 <= entrypoint_offset < code_len, "entrypoint_offset_range"),
    (10 <= main_func_idx < 10 + function_start_len, "main_func_idx_range"),
]
for ok, label in checks:
    if not ok:
        raise SystemExit(f"invalid actual stage1 artifact manifest: {label}")
PY

vm_status="$(limactl list "$VM_NAME" --format '{{.Status}}' 2>/dev/null || true)"
if [[ "$vm_status" != "Running" ]]; then
  limactl start --tty=false "$VM_NAME"
fi

HOST_WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/lsharp-native-linux-x86-stage0.XXXXXX")"
VM_WORK_DIR="/tmp/lsharp-native-linux-x86-stage0-$$-${RANDOM}"
limactl shell "$VM_NAME" -- rm -rf "$VM_WORK_DIR"
limactl shell "$VM_NAME" -- mkdir -p "$VM_WORK_DIR"
VM_WORK_DIR_CREATED=1

limactl copy --recursive "$ACTUAL_STAGE1_DIR/." "$VM_NAME:$VM_WORK_DIR/actual-stage1"
limactl copy "$MATERIALIZER" "$VM_NAME:$VM_WORK_DIR/materialize.py"
limactl shell "$VM_NAME" -- env \
  LSHARP_NATIVE_LINUX_X86_ACTUAL_HEAP_BYTES="${LSHARP_NATIVE_LINUX_X86_ACTUAL_HEAP_BYTES:-4294967296}" \
  bash -lc "cd '$VM_WORK_DIR/actual-stage1' && python3 '$VM_WORK_DIR/materialize.py' . stage1-code.bin entrypoint-offset.txt && test -x program.native"

HOST_COMPILER="$HOST_WORK_DIR/program.native"
limactl copy "$VM_NAME:$VM_WORK_DIR/actual-stage1/program.native" "$HOST_COMPILER"
chmod 0755 "$HOST_COMPILER"
[[ -x "$HOST_COMPILER" ]] || die "VM materializer did not produce an executable program.native"

"$PACKAGE_BUILDER" \
  --target x86_64-unknown-linux-gnu \
  --source-commit "$SOURCE_COMMIT" \
  --compiler "$HOST_COMPILER" \
  --transport-driver "$TRANSPORT_DRIVER" \
  --materializer "$MATERIALIZER" \
  --output-dir "$OUTPUT_DIR"

echo "native Linux x86 actual-stage1 package: $OUTPUT_DIR"
