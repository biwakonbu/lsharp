#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
GATE="$ROOT/scripts/ci/native-official-release-local.sh"
TMP_ROOT="$(mktemp -d "/tmp/lsharp-native-official-replay-lock.XXXXXX")"
LOCK_DIR="$TMP_ROOT/hostgen.lock"
DIST_DIR="$TMP_ROOT/dist"
SMOKE_ROOT="$TMP_ROOT/smoke"
SOURCE_COMMIT="$(git rev-parse HEAD)"

cleanup() {
  rm -rf "$TMP_ROOT"
}
trap cleanup EXIT

mkdir "$LOCK_DIR"
printf '%s\n' "$$" >"$LOCK_DIR/pid"
printf '%s\n' "$TMP_ROOT/artifact" >"$LOCK_DIR/artifact_dir"
printf '%s\n' '/tmp/lsharp-native-official-replay-lock-vm' >"$LOCK_DIR/vm_work_dir"

set +e
output="$({
  VERSION=v0.1.0-test \
  SOURCE_COMMIT="$SOURCE_COMMIT" \
  TMPDIR=/tmp \
  DIST_DIR="$DIST_DIR" \
  LSHARP_NATIVE_RELEASE_SMOKE_ROOT="$SMOKE_ROOT" \
  LSHARP_NATIVE_LINUX_X86_HOST_REPLAY_LOCK_DIR="$LOCK_DIR" \
    bash "$GATE"
} 2>&1)"
status=$?
set -e

[[ "$status" -eq 90 ]] \
  || { echo "expected official gate to refuse a live hostgen replay lock, got exit=$status" >&2; echo "$output" >&2; exit 1; }
grep -F 'Linux hostgen replay lock is held' <<<"$output" >/dev/null \
  || { echo 'official gate did not report the replay lock boundary' >&2; echo "$output" >&2; exit 1; }
grep -F "holder_pid=$$" <<<"$output" >/dev/null \
  || { echo 'official gate did not report the live lock owner' >&2; echo "$output" >&2; exit 1; }
grep -F "artifact_dir=$TMP_ROOT/artifact" <<<"$output" >/dev/null \
  || { echo 'official gate did not report the replay artifact owner' >&2; echo "$output" >&2; exit 1; }
[[ ! -e "$DIST_DIR" ]] \
  || { echo 'official gate created release output before the replay lock preflight' >&2; exit 1; }

echo 'native official release replay-lock preflight contract passed'
