#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SMOKE="$ROOT/scripts/ci/native-linux-x86-native-stage0-source-file-smoke.sh"
TMP_ROOT="$(mktemp -d "/tmp/lsharp-linux-source-replay-lock.XXXXXX")"
FAKE_BIN="$TMP_ROOT/bin"
LOG="$TMP_ROOT/limactl.log"
STAGE0="$TMP_ROOT/stage0"
LOCK_DIR="$TMP_ROOT/hostgen.lock"
SOURCE_COMMIT="$(git rev-parse --verify HEAD)"

cleanup() {
  rm -rf "$TMP_ROOT"
}
trap cleanup EXIT

mkdir -p "$FAKE_BIN" "$STAGE0/bin" "$LOCK_DIR"
printf '%s\n' '{"kind":"lsharp-native-selfhost-stage0","target":"x86_64-unknown-linux-gnu","source_commit":"'"$SOURCE_COMMIT"'","compiler":"bin/compiler","transport_driver":"bin/transport-driver","materializer":"bin/materializer"}' >"$STAGE0/manifest.json"
for executable in compiler transport-driver materializer; do
  printf '%s\n' '#!/usr/bin/env bash' 'exit 0' >"$STAGE0/bin/$executable"
  chmod +x "$STAGE0/bin/$executable"
done
printf '%s\n' "$$" >"$LOCK_DIR/pid"
printf '%s\n' "$TMP_ROOT/artifact" >"$LOCK_DIR/artifact_dir"
printf '%s\n' '/tmp/lsharp-linux-source-replay-lock-vm' >"$LOCK_DIR/vm_work_dir"

cat >"$FAKE_BIN/limactl" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"${FAKE_LIMACTL_LOG:?}"
exit 99
SH
chmod +x "$FAKE_BIN/limactl"

set +e
output="$({
  FAKE_LIMACTL_LOG="$LOG" \
  PATH="$FAKE_BIN:$PATH" \
  LSHARP_NATIVE_LINUX_X86_STAGE0_DIR="$STAGE0" \
  LSHARP_NATIVE_LINUX_X86_HOST_REPLAY_LOCK_DIR="$LOCK_DIR" \
    bash "$SMOKE"
} 2>&1)"
status=$?
set -e

[[ "$status" -eq 90 ]] \
  || { echo "expected Linux source smoke to refuse a live hostgen replay lock, got exit=$status" >&2; echo "$output" >&2; exit 1; }
grep -F 'Linux hostgen replay lock is held' <<<"$output" >/dev/null \
  || { echo 'Linux source smoke did not report the replay lock boundary' >&2; echo "$output" >&2; exit 1; }
grep -F "holder_pid=$$" <<<"$output" >/dev/null \
  || { echo 'Linux source smoke did not report the live lock owner' >&2; echo "$output" >&2; exit 1; }
[[ ! -s "$LOG" ]] \
  || { echo 'Linux source smoke invoked limactl before the replay lock preflight' >&2; cat "$LOG" >&2; exit 1; }

echo 'native Linux source smoke replay-lock preflight contract passed'
