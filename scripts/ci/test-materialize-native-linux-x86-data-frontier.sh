#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
MATERIALIZER="$ROOT/scripts/ci/materialize-native-linux-x86-bundle.py"
WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/lsharp-native-linux-x86-data-frontier.XXXXXX")"
trap 'rm -rf "$WORK_DIR"' EXIT

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

python3 - "$WORK_DIR" <<'PY'
import pathlib
import sys

work_dir = pathlib.Path(sys.argv[1])
(work_dir / "stage-code.bin").write_bytes(b"\xc3")
(work_dir / "stage-data.bin").write_bytes(bytes(19523))
(work_dir / "entrypoint-offset.txt").write_text("0\n")
(work_dir / "fake-cc").write_text(
    "#!/usr/bin/env bash\n"
    "set -euo pipefail\n"
    "if [[ \"$1\" == \"@linker-response.txt\" ]]; then\n"
    "  : > program.native\n"
    "  exit 0\n"
    "fi\n"
    "for ((i=1; i<=$#; i++)); do\n"
    "  if [[ \"${!i}\" == \"-o\" ]]; then\n"
    "    j=$((i + 1))\n"
    "    : > \"${!j}\"\n"
    "    exit 0\n"
    "  fi\n"
    "done\n"
    "exit 1\n"
)
(work_dir / "fake-cc").chmod(0o755)
PY

PATH="$WORK_DIR:$PATH" ln -s fake-cc "$WORK_DIR/cc"
PATH="$WORK_DIR:$PATH" python3 "$MATERIALIZER" "$WORK_DIR" stage-code.bin entrypoint-offset.txt

expected_frontier=20552
grep -F -- "mov \$$expected_frontier, %rcx" "$WORK_DIR/program.s" >/dev/null \
  || fail "materializer did not place heap cursor after stage data"
grep -F -- "lea 1024(%r14), %rdi" "$WORK_DIR/program.s" >/dev/null \
  || fail "materializer did not copy stage data at the native data base"
if grep -F -- 'mov $8192, %rcx' "$WORK_DIR/program.s" >/dev/null; then
  fail "materializer still uses the fixed heap cursor"
fi

if LSHARP_NATIVE_LINUX_X86_ACTUAL_HEAP_BYTES="$expected_frontier" \
  PATH="$WORK_DIR:$PATH" \
  python3 "$MATERIALIZER" "$WORK_DIR" stage-code.bin entrypoint-offset.txt \
  >"$WORK_DIR/too-small.stdout" 2>"$WORK_DIR/too-small.stderr"; then
  fail "materializer accepted a heap that ends at the data frontier"
fi
grep -F -- "native heap is too small for embedded data" "$WORK_DIR/too-small.stderr" >/dev/null \
  || fail "materializer did not report the heap frontier capacity boundary"

echo "native Linux x86 data frontier materializer test passed"
