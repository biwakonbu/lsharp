#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SMOKE="$ROOT/scripts/ci/native-linux-x86-native-stage0-source-file-smoke.sh"
TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/lsharp-linux-source-evidence-copy.XXXXXX")"
FAKE_BIN="$TMP_ROOT/bin"
LOG="$TMP_ROOT/limactl.log"
STAGE0="$TMP_ROOT/stage0"
SOURCE_COMMIT="$(git rev-parse --verify HEAD)"
VM_NAME="lsharp-linux-x86"

cleanup() {
  rm -rf "$TMP_ROOT"
}
trap cleanup EXIT

mkdir -p "$FAKE_BIN" "$STAGE0/bin"
printf '%s\n' '{"kind":"lsharp-native-selfhost-stage0","target":"x86_64-unknown-linux-gnu","source_commit":"'"$SOURCE_COMMIT"'","compiler":"bin/compiler","transport_driver":"bin/transport-driver","materializer":"bin/materializer"}' >"$STAGE0/manifest.json"
for executable in compiler transport-driver materializer; do
  printf '%s\n' '#!/usr/bin/env bash' 'exit 0' >"$STAGE0/bin/$executable"
  chmod +x "$STAGE0/bin/$executable"
done

cat >"$FAKE_BIN/limactl" <<'SH'
#!/usr/bin/env bash
set -euo pipefail

printf '%s\n' "$*" >>"${FAKE_LIMACTL_LOG:?}"

case "${1:-}" in
  list)
    printf '%s\n' 'Running'
    ;;
  copy)
    args=("$@")
    source="${args[${#args[@]}-2]}"
    target="${args[${#args[@]}-1]}"
    if [[ "$source" == "${FAKE_VM_NAME}:"*"/source-smoke-evidence/." ]]; then
      mkdir -p "$target"
      printf '%s\n' 'copied evidence' >"$target/manifest.json"
    fi
    ;;
  shell)
    shift 3
    case "${1:-}" in
      sh)
        printf '%s\n' '4194304'
        ;;
      env)
        evidence=''
        for argument in "$@"; do
          case "$argument" in
            NATIVE_SELFHOST_SOURCE_SMOKE_EVIDENCE_DIR=*)
              evidence="${argument#*=}"
              ;;
          esac
        done
        if [[ -n "$evidence" ]]; then
          mkdir -p "$evidence"
          printf '%s\n' 'guest evidence' >"$evidence/manifest.json"
        fi
        exit "${FAKE_SMOKE_STATUS:-0}"
        ;;
      rm|mkdir)
        ;;
      *)
        printf 'unexpected fake shell command: %s\n' "$*" >&2
        exit 1
        ;;
    esac
    ;;
  *)
    printf 'unexpected fake limactl command: %s\n' "$*" >&2
    exit 1
    ;;
esac
SH
chmod +x "$FAKE_BIN/limactl"

SUCCESS_EVIDENCE="$TMP_ROOT/success-evidence"
success_output="$(
  FAKE_LIMACTL_LOG="$LOG" \
  FAKE_VM_NAME="$VM_NAME" \
  PATH="$FAKE_BIN:$PATH" \
  LSHARP_NATIVE_LINUX_X86_STAGE0_DIR="$STAGE0" \
  LSHARP_NATIVE_LINUX_X86_SOURCE_SMOKE_EVIDENCE_DIR="$SUCCESS_EVIDENCE" \
    bash "$SMOKE" 2>&1
)"
grep -F 'Linux x86_64 native stage0 source-file smoke passed' <<<"$success_output" >/dev/null \
  || { echo 'successful Linux source smoke was not reported' >&2; exit 1; }
[[ -s "$SUCCESS_EVIDENCE/manifest.json" ]] \
  || { echo 'successful Linux source smoke evidence was not copied to host' >&2; exit 1; }
grep -F -- '--recursive' "$LOG" >/dev/null \
  || { echo 'Linux source smoke evidence copy did not use recursive Lima copy' >&2; exit 1; }

FAILURE_EVIDENCE="$TMP_ROOT/failure-evidence"
set +e
failure_output="$(
  FAKE_LIMACTL_LOG="$LOG" \
  FAKE_VM_NAME="$VM_NAME" \
  FAKE_SMOKE_STATUS=23 \
  PATH="$FAKE_BIN:$PATH" \
  LSHARP_NATIVE_LINUX_X86_STAGE0_DIR="$STAGE0" \
  LSHARP_NATIVE_LINUX_X86_SOURCE_SMOKE_EVIDENCE_DIR="$FAILURE_EVIDENCE" \
    bash "$SMOKE" 2>&1
)"
failure_status=$?
set -e
[[ "$failure_status" -eq 23 ]] \
  || { echo "Linux source smoke status was not preserved: $failure_status" >&2; exit 1; }
[[ -s "$FAILURE_EVIDENCE/manifest.json" ]] \
  || { echo 'failed Linux source smoke evidence was not copied to host' >&2; exit 1; }
if grep -F 'source smoke evidence copy failed' <<<"$failure_output" >/dev/null; then
  echo 'fake Lima unexpectedly failed to copy source smoke evidence' >&2
  exit 1
fi

echo 'native Linux source smoke evidence copy contract passed'
