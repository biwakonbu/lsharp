#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
VM_NAME="${LSHARP_NATIVE_LINUX_X86_VM_NAME:-lsharp-linux-x86}"
BUNDLE_DIR_INPUT="${LSHARP_NATIVE_LINUX_X86_APP_CLI_BUNDLE_DIR:-}"
KEEP_WORK_DIR="${LSHARP_NATIVE_LINUX_X86_KEEP_APP_CLI_SOURCE_SMOKE_WORK_DIR:-0}"
VM_WORK_DIR="/tmp/lsharp-native-app-cli-source-file-smoke-$$"
VM_WORK_DIR_CREATED=0

cleanup() {
  if [[ "${VM_WORK_DIR_CREATED}" -eq 1 && "${KEEP_WORK_DIR}" != "1" ]] \
    && command -v limactl >/dev/null 2>&1; then
    limactl shell "${VM_NAME}" -- rm -rf "${VM_WORK_DIR}" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

require_file() {
  local path="$1"
  local description="$2"
  if [[ ! -s "${path}" ]]; then
    echo "ERROR: ${description} is required: ${path}" >&2
    exit 1
  fi
}

if [[ "$(uname -s)" != "Darwin" || "$(uname -m)" != "arm64" ]]; then
  echo "ERROR: this source-file smoke requires macOS arm64 with Lima; got $(uname -s)/$(uname -m)" >&2
  exit 1
fi
if ! command -v limactl >/dev/null 2>&1; then
  echo "ERROR: limactl is required for the Linux x86_64 source-file smoke" >&2
  exit 1
fi
if [[ -z "${BUNDLE_DIR_INPUT}" ]]; then
  echo "ERROR: LSHARP_NATIVE_LINUX_X86_APP_CLI_BUNDLE_DIR is required" >&2
  exit 1
fi

if [[ "${BUNDLE_DIR_INPUT}" = /* ]]; then
  BUNDLE_DIR="${BUNDLE_DIR_INPUT}"
else
  BUNDLE_DIR="${ROOT_DIR}/${BUNDLE_DIR_INPUT}"
fi

require_file "${BUNDLE_DIR}/stage-code.bin" "Linux App.Cli stage code"
require_file "${BUNDLE_DIR}/stage-data.bin" "Linux App.Cli stage data"
require_file "${BUNDLE_DIR}/entrypoint-offset.txt" "Linux App.Cli entrypoint offset"
require_file "${BUNDLE_DIR}/manifest.json" "Linux App.Cli manifest"

python3 - "${BUNDLE_DIR}" <<'PY'
import json
import pathlib
import sys

bundle_dir = pathlib.Path(sys.argv[1])
manifest = json.loads((bundle_dir / "manifest.json").read_text())
if manifest.get("target") != "x86_64-unknown-linux-gnu":
    raise SystemExit(f"unexpected bundle target: {manifest.get('target')!r}")
if (bundle_dir / "stage-code.bin").stat().st_size <= 0:
    raise SystemExit("stage-code.bin is empty")
if int((bundle_dir / "entrypoint-offset.txt").read_text().strip()) < 0:
    raise SystemExit("entrypoint offset must be non-negative")
PY

vm_status="$(limactl list "${VM_NAME}" --format '{{.Status}}' 2>/dev/null || true)"
if [[ "${vm_status}" != "Running" ]]; then
  limactl start --tty=false "${VM_NAME}"
fi

limactl shell "${VM_NAME}" -- rm -rf "${VM_WORK_DIR}"
limactl shell "${VM_NAME}" -- mkdir -p "${VM_WORK_DIR}"
VM_WORK_DIR_CREATED=1

limactl copy scripts/ci/materialize-native-linux-x86-bundle.py \
  "${VM_NAME}:${VM_WORK_DIR}/materialize-native-linux-x86-bundle.py"
limactl copy "${BUNDLE_DIR}/stage-code.bin" "${VM_NAME}:${VM_WORK_DIR}/stage-code.bin"
limactl copy "${BUNDLE_DIR}/stage-data.bin" "${VM_NAME}:${VM_WORK_DIR}/stage-data.bin"
limactl copy "${BUNDLE_DIR}/entrypoint-offset.txt" "${VM_NAME}:${VM_WORK_DIR}/entrypoint-offset.txt"

set +e
limactl shell "${VM_NAME}" -- bash -s -- "${VM_WORK_DIR}" <<'VM_SCRIPT'
set -euo pipefail

work_dir="$1"
cd "${work_dir}"

LSHARP_NATIVE_LINUX_X86_SKIP_ARGV0=1 \
  python3 materialize-native-linux-x86-bundle.py . stage-code.bin entrypoint-offset.txt

printf '%s\n' '(defn main [] 42)' > input.ls
cat > metadata.ls <<'LSHARP'
(defn abs [x]
  :example [(= (abs 5) 5) (= (abs (- 0 7)) 7)]
  :invariant (>= result 0)
  (if (< x 0) (- 0 x) x))
LSHARP

run_command() {
  local label="$1"
  shift
  set +e
  "$@" >"${label}.stdout" 2>"${label}.stderr"
  local exit_code=$?
  set -e
  if [[ "${exit_code}" -ne 0 ]]; then
    echo "ERROR: ${label} failed with exit=${exit_code}" >&2
    cat "${label}.stdout" >&2
    cat "${label}.stderr" >&2
    exit "${exit_code}"
  fi
  if [[ -s "${label}.stderr" ]]; then
    echo "ERROR: ${label} emitted stderr" >&2
    cat "${label}.stderr" >&2
    exit 1
  fi
}

require_line() {
  local label="$1"
  local expected="$2"
  if ! grep -Fx "${expected}" "${label}.stdout" >/dev/null; then
    echo "ERROR: ${label} stdout is missing ${expected@Q}" >&2
    cat "${label}.stdout" >&2
    exit 1
  fi
}

require_exact_output() {
  local label="$1"
  local expected="$2"
  if ! printf '%s' "${expected}" | cmp -s - "${label}.stdout"; then
    echo "ERROR: ${label} stdout does not match expected output" >&2
    cat "${label}.stdout" >&2
    exit 1
  fi
}

run_command parse ./program.native parse input.ls
for expected in decls:1 first-decl:defn first-body:int diagnostics:0; do
  require_line parse "${expected}"
done

run_command check ./program.native check input.ls
for expected in Int diagnostics:0; do
  require_line check "${expected}"
done

run_command fmt ./program.native fmt input.ls
require_line fmt '(defn main [] 42)'

run_command test ./program.native test input.ls
require_exact_output test $'examples:0\ninvariants:0\nfailures:0\n'

run_command metadata-test ./program.native test metadata.ls
require_exact_output metadata-test $'examples:2\ninvariants:1\nfailures:0\n'

run_command compile ./program.native compile input.ls -o compile.wasm
run_command build ./program.native build input.ls -o build.wasm
for output in compile.wasm build.wasm; do
  if [[ ! -s "${output}" ]]; then
    echo "ERROR: ${output} was not written" >&2
    exit 1
  fi
  if [[ "$(od -An -tx1 -N4 "${output}" | tr -d '[:space:]')" != "0061736d" ]]; then
    echo "ERROR: ${output} does not have a core Wasm header" >&2
    exit 1
  fi
done
if ! grep -Eq '^wasm-size:[1-9][0-9]*$' compile.stdout; then
  echo "ERROR: compile stdout is missing a positive wasm-size" >&2
  cat compile.stdout >&2
  exit 1
fi
if ! grep -Eq '^wasm-size:[1-9][0-9]*$' build.stdout; then
  echo "ERROR: build stdout is missing a positive wasm-size" >&2
  cat build.stdout >&2
  exit 1
fi

printf 'Linux x86_64 native App.Cli source-file smoke passed\n'
VM_SCRIPT
smoke_status=$?
set -e
if [[ "${smoke_status}" -ne 0 ]]; then
  echo "ERROR: Linux x86_64 native App.Cli source-file smoke failed with exit=${smoke_status}" >&2
  exit "${smoke_status}"
fi
