#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/lsharp-native-official-snapshot.XXXXXX")"
FAKE_ROOT="$TMP_ROOT/project"
LOG_PATH="$TMP_ROOT/invocations.log"
TRUST_STORE="$TMP_ROOT/trust-store.json"
LIFECYCLE="$TMP_ROOT/review-lifecycle.jsonl"
SOURCE_COMMIT=""
VERSION="v0.0.0-test"
PATH_PREFIX="$TMP_ROOT/bin"
SMOKE_ROOT="$(mktemp -d /tmp/lsharp-native-official-snapshot-smoke.XXXXXX)"
SOURCE_SMOKE_EVIDENCE_ROOT="$TMP_ROOT/source-smoke-evidence"
PARTIAL_SMOKE_ROOT=""
MISSING_IDENTITY_SMOKE_ROOT=""
VM_COPY_FAILURE_SMOKE_ROOT=""
RUNNING_VM_SMOKE_ROOT=""
CLEANUP_FAILURE_SMOKE_ROOT=""
TRAVERSAL_ROOT=""
TRAVERSAL_BASE=""
HOSTGEN_REPLAY_LOCK_PATH="/tmp/lsharp-native-official-snapshot-hostgen-lock.$$"
cleanup() {
  rm -rf "$TMP_ROOT" "$SMOKE_ROOT"
  rm -rf "$HOSTGEN_REPLAY_LOCK_PATH"
  [[ -z "$PARTIAL_SMOKE_ROOT" ]] || rm -rf "$PARTIAL_SMOKE_ROOT"
  [[ -z "$MISSING_IDENTITY_SMOKE_ROOT" ]] || rm -rf "$MISSING_IDENTITY_SMOKE_ROOT"
  [[ -z "$VM_COPY_FAILURE_SMOKE_ROOT" ]] || rm -rf "$VM_COPY_FAILURE_SMOKE_ROOT"
  [[ -z "$RUNNING_VM_SMOKE_ROOT" ]] || rm -rf "$RUNNING_VM_SMOKE_ROOT"
  [[ -z "$CLEANUP_FAILURE_SMOKE_ROOT" ]] || rm -rf "$CLEANUP_FAILURE_SMOKE_ROOT"
  [[ -z "$TRAVERSAL_ROOT" ]] || rm -rf "$TRAVERSAL_ROOT"
  [[ -z "$TRAVERSAL_BASE" ]] || rm -rf "$TRAVERSAL_BASE"
}
trap cleanup EXIT

export LSHARP_NATIVE_LINUX_X86_HOST_REPLAY_LOCK_DIR="$HOSTGEN_REPLAY_LOCK_PATH"

mkdir -p "$FAKE_ROOT/scripts/ci" "$PATH_PREFIX" "$FAKE_ROOT/dist"
FAKE_ROOT_CANONICAL="$(cd "$FAKE_ROOT" && pwd)"
printf '%s\n' '{"keys":["release-key"]}' >"$TRUST_STORE"
printf '%s\n' '{"review_id":"review:orchestrator/r1","state":"active"}' >"$LIFECYCLE"

cp "$ROOT/scripts/ci/native-official-release-local.sh" "$FAKE_ROOT/scripts/ci/"

cat >"$FAKE_ROOT/scripts/ci/native-selfhost-dev-source-file-smoke.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
evidence="${NATIVE_SELFHOST_SOURCE_SMOKE_EVIDENCE_DIR:-}"
if [[ -n "$evidence" ]]; then
  mkdir -p "$evidence"
  printf '%s\n' 'mac evidence' >"$evidence/manifest.json"
fi
printf '%s\n' "runtime mac stage0=${NATIVE_STAGE0_DIR:-} source=${NATIVE_SELFHOST_SOURCE_ROOT:-} evidence=$evidence" >>"$FAKE_LOG"
SH
chmod +x "$FAKE_ROOT/scripts/ci/native-selfhost-dev-source-file-smoke.sh"

cat >"$FAKE_ROOT/scripts/ci/native-linux-x86-native-stage0-source-file-smoke.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
evidence="${LSHARP_NATIVE_LINUX_X86_SOURCE_SMOKE_EVIDENCE_DIR:-}"
if [[ -n "$evidence" ]]; then
  mkdir -p "$evidence"
  printf '%s\n' 'linux evidence' >"$evidence/manifest.json"
fi
printf '%s\n' "runtime linux stage0=${LSHARP_NATIVE_LINUX_X86_STAGE0_DIR:-} evidence=$evidence" >>"$FAKE_LOG"
SH
chmod +x "$FAKE_ROOT/scripts/ci/native-linux-x86-native-stage0-source-file-smoke.sh"

cat >"$FAKE_ROOT/scripts/release.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "release target=$TARGET trust=${NATIVE_ONLY_REVIEW_TRUST_STORE:-} lifecycle=${NATIVE_ONLY_REVIEW_LIFECYCLE:-} identity=${NATIVE_ONLY_REVIEW_EVIDENCE_IDENTITY:-}" >>"$FAKE_LOG"
printf '%s\n' 'fake release archive' >"$DIST_DIR/lsharp-${VERSION}-${TARGET}.tar.gz"
SH
chmod +x "$FAKE_ROOT/scripts/release.sh"

cat >"$FAKE_ROOT/scripts/ci/package-native-stage0-release.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
target=''
version=''
output_dir=''
args="$*"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --target) target="$2"; shift 2 ;;
    --version) version="$2"; shift 2 ;;
    --output-dir) output_dir="$2"; shift 2 ;;
    *) shift ;;
  esac
done
printf '%s\n' "stage0 target=$target args=$args" >>"$FAKE_LOG"
printf '%s\n' 'fake stage0 archive' >"$output_dir/lsharp-stage0-${version}-${target}.tar.gz"
SH
chmod +x "$FAKE_ROOT/scripts/ci/package-native-stage0-release.sh"

cat >"$FAKE_ROOT/scripts/ci/release-smoke.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "mac smoke trust=${RELEASE_REVIEW_TRUST_STORE:-} lifecycle=${RELEASE_REVIEW_LIFECYCLE:-} archive=$1 rollback=$2" >>"$FAKE_LOG"
SH
chmod +x "$FAKE_ROOT/scripts/ci/release-smoke.sh"

cat >"$FAKE_ROOT/scripts/fetch-stage0.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
mkdir -p "$STAGE0_DIR"
printf '%s\n' "{\"target\":\"$STAGE0_TARGET\"}" >"$STAGE0_DIR/manifest.json"
printf '%s\n' "fetch target=$STAGE0_TARGET" >>"$FAKE_LOG"
SH
chmod +x "$FAKE_ROOT/scripts/fetch-stage0.sh"

cat >"$FAKE_ROOT/scripts/checksum.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' 'checksum fixture'
SH
chmod +x "$FAKE_ROOT/scripts/checksum.sh"

cat >"$PATH_PREFIX/limactl" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "limactl $*" >>"$FAKE_LOG"
if [[ "${FAIL_LIMACTL_COPY:-0}" == "1" && "${1:-}" == "copy" ]]; then
  exit 17
fi
if [[ "${FAIL_LIMACTL_STOP:-0}" == "1" && "${1:-}" == "stop" ]]; then
  exit 19
fi
case "${1:-}" in
  list)
    if [[ "${FAKE_LIMA_RUNNING:-0}" == "1" ]]; then
      printf '%s\n' 'Running'
    else
      printf '%s\n' 'Stopped'
    fi
    ;;
  start|stop|copy|shell) ;;
  *) exit 1 ;;
esac
SH
chmod +x "$PATH_PREFIX/limactl"

for target in aarch64-apple-darwin x86_64-unknown-linux-gnu; do
  artifact_dir="$TMP_ROOT/artifact-$target"
  stage0_dir="$TMP_ROOT/stage0-$target"
  mkdir -p "$artifact_dir" "$stage0_dir"
  printf '%s\n' 'fake native program' >"$artifact_dir/program.native"
  printf '%s\n' '{"status":"pass"}' >"$artifact_dir/manifest.json"
  printf '%s\n' 'identity' >"$artifact_dir/review-evidence-identity.json"
  printf '%s\n' 'stage0' >"$stage0_dir/manifest.json"
  printf '%s\n' 'stage0 identity' >"$stage0_dir/review-evidence-identity.json"
  printf '%s\n' 'rollback' >"$TMP_ROOT/rollback-$target.tar.gz"
done

git -C "$FAKE_ROOT" init -q
git -C "$FAKE_ROOT" config user.email fixture@example.invalid
git -C "$FAKE_ROOT" config user.name fixture
git -C "$FAKE_ROOT" add .
git -C "$FAKE_ROOT" -c commit.gpgSign=false commit -qm 'native official release fixture'
SOURCE_COMMIT="$(git -C "$FAKE_ROOT" rev-parse HEAD)"

FAKE_LOG="$LOG_PATH" \
PATH="$PATH_PREFIX:$PATH" \
VERSION="$VERSION" \
SOURCE_COMMIT="$SOURCE_COMMIT" \
DIST_DIR="$FAKE_ROOT/dist" \
SMOKE_ROOT="$SMOKE_ROOT" \
LSHARP_NATIVE_RELEASE_SMOKE_ROOT="$SMOKE_ROOT" \
NATIVE_OFFICIAL_SOURCE_SMOKE_EVIDENCE_ROOT="$SOURCE_SMOKE_EVIDENCE_ROOT" \
NATIVE_OFFICIAL_REVIEW_TRUST_STORE="$TRUST_STORE" \
NATIVE_OFFICIAL_REVIEW_LIFECYCLE="$LIFECYCLE" \
MACOS_APP_CLI_ARTIFACT_DIR="$TMP_ROOT/artifact-aarch64-apple-darwin" \
LINUX_APP_CLI_ARTIFACT_DIR="$TMP_ROOT/artifact-x86_64-unknown-linux-gnu" \
MACOS_STAGE0_DIR="$TMP_ROOT/stage0-aarch64-apple-darwin" \
LINUX_STAGE0_DIR="$TMP_ROOT/stage0-x86_64-unknown-linux-gnu" \
MACOS_ROLLBACK_ARCHIVE="$TMP_ROOT/rollback-aarch64-apple-darwin.tar.gz" \
LINUX_ROLLBACK_ARCHIVE="$TMP_ROOT/rollback-x86_64-unknown-linux-gnu.tar.gz" \
  bash "$FAKE_ROOT/scripts/ci/native-official-release-local.sh"

grep -F "release target=aarch64-apple-darwin trust=$TRUST_STORE lifecycle=$LIFECYCLE" "$LOG_PATH" >/dev/null
grep -F "release target=x86_64-unknown-linux-gnu trust=$TRUST_STORE lifecycle=$LIFECYCLE" "$LOG_PATH" >/dev/null
grep -F -- "--review-trust-store $TRUST_STORE --review-lifecycle $LIFECYCLE" "$LOG_PATH" >/dev/null
grep -F "mac smoke trust=$TRUST_STORE lifecycle=$LIFECYCLE" "$LOG_PATH" >/dev/null
grep -F "review-trust-store.snapshot" "$LOG_PATH" >/dev/null
grep -F "review-lifecycle.snapshot" "$LOG_PATH" >/dev/null
grep -F "review_identity_timestamp.py" "$LOG_PATH" >/dev/null
grep -F "fetch target=aarch64-apple-darwin" "$LOG_PATH" >/dev/null
grep -F "fetch target=x86_64-unknown-linux-gnu" "$LOG_PATH" >/dev/null
grep -F "runtime mac stage0=$SMOKE_ROOT/stage0-aarch64-apple-darwin source=$FAKE_ROOT_CANONICAL/selfhost evidence=$SOURCE_SMOKE_EVIDENCE_ROOT/aarch64-apple-darwin" "$LOG_PATH" >/dev/null
grep -F "runtime linux stage0=$SMOKE_ROOT/stage0-x86_64-unknown-linux-gnu evidence=$SOURCE_SMOKE_EVIDENCE_ROOT/x86_64-unknown-linux-gnu" "$LOG_PATH" >/dev/null
[[ -s "$SOURCE_SMOKE_EVIDENCE_ROOT/aarch64-apple-darwin/manifest.json" ]] \
  || { echo 'Mac source smoke evidence was not retained' >&2; exit 1; }
[[ -s "$SOURCE_SMOKE_EVIDENCE_ROOT/x86_64-unknown-linux-gnu/manifest.json" ]] \
  || { echo 'Linux source smoke evidence was not retained' >&2; exit 1; }
grep -F "limactl stop lsharp-linux-x86" "$LOG_PATH" >/dev/null

RUNNING_VM_SMOKE_ROOT="$(mktemp -d /tmp/lsharp-native-official-running-vm.XXXXXX)"
RUNNING_VM_LOG="$TMP_ROOT/running-vm.log"
FAKE_LIMA_RUNNING=1 \
FAKE_LOG="$RUNNING_VM_LOG" \
PATH="$PATH_PREFIX:$PATH" \
VERSION="$VERSION" \
SOURCE_COMMIT="$SOURCE_COMMIT" \
DIST_DIR="$FAKE_ROOT/running-vm-dist" \
SMOKE_ROOT="$RUNNING_VM_SMOKE_ROOT" \
LSHARP_NATIVE_RELEASE_SMOKE_ROOT="$RUNNING_VM_SMOKE_ROOT" \
KEEP_WORK_DIR=1 \
NATIVE_OFFICIAL_REVIEW_TRUST_STORE="$TRUST_STORE" \
NATIVE_OFFICIAL_REVIEW_LIFECYCLE="$LIFECYCLE" \
MACOS_APP_CLI_ARTIFACT_DIR="$TMP_ROOT/artifact-aarch64-apple-darwin" \
LINUX_APP_CLI_ARTIFACT_DIR="$TMP_ROOT/artifact-x86_64-unknown-linux-gnu" \
MACOS_STAGE0_DIR="$TMP_ROOT/stage0-aarch64-apple-darwin" \
LINUX_STAGE0_DIR="$TMP_ROOT/stage0-x86_64-unknown-linux-gnu" \
MACOS_ROLLBACK_ARCHIVE="$TMP_ROOT/rollback-aarch64-apple-darwin.tar.gz" \
LINUX_ROLLBACK_ARCHIVE="$TMP_ROOT/rollback-x86_64-unknown-linux-gnu.tar.gz" \
  bash "$FAKE_ROOT/scripts/ci/native-official-release-local.sh"
if grep -F "limactl stop lsharp-linux-x86" "$RUNNING_VM_LOG" >/dev/null; then
  echo 'gate stopped a Lima VM it did not start' >&2
  exit 1
fi

CLEANUP_FAILURE_SMOKE_ROOT="$(mktemp -d /tmp/lsharp-native-official-cleanup-failure.XXXXXX)"
CLEANUP_FAILURE_LOG="$TMP_ROOT/cleanup-failure.log"
set +e
cleanup_failure_output="$(
  FAIL_LIMACTL_STOP=1 \
  FAKE_LOG="$CLEANUP_FAILURE_LOG" \
  PATH="$PATH_PREFIX:$PATH" \
  VERSION="$VERSION" \
  SOURCE_COMMIT="$SOURCE_COMMIT" \
  DIST_DIR="$FAKE_ROOT/cleanup-failure-dist" \
  SMOKE_ROOT="$CLEANUP_FAILURE_SMOKE_ROOT" \
  LSHARP_NATIVE_RELEASE_SMOKE_ROOT="$CLEANUP_FAILURE_SMOKE_ROOT" \
  KEEP_WORK_DIR=1 \
  NATIVE_OFFICIAL_REVIEW_TRUST_STORE="$TRUST_STORE" \
  NATIVE_OFFICIAL_REVIEW_LIFECYCLE="$LIFECYCLE" \
  MACOS_APP_CLI_ARTIFACT_DIR="$TMP_ROOT/artifact-aarch64-apple-darwin" \
  LINUX_APP_CLI_ARTIFACT_DIR="$TMP_ROOT/artifact-x86_64-unknown-linux-gnu" \
  MACOS_STAGE0_DIR="$TMP_ROOT/stage0-aarch64-apple-darwin" \
  LINUX_STAGE0_DIR="$TMP_ROOT/stage0-x86_64-unknown-linux-gnu" \
  MACOS_ROLLBACK_ARCHIVE="$TMP_ROOT/rollback-aarch64-apple-darwin.tar.gz" \
  LINUX_ROLLBACK_ARCHIVE="$TMP_ROOT/rollback-x86_64-unknown-linux-gnu.tar.gz" \
    bash "$FAKE_ROOT/scripts/ci/native-official-release-local.sh" 2>&1
)"
cleanup_failure_status=$?
set -e
[[ "$cleanup_failure_status" -ne 0 ]] \
  || { echo 'owned Lima VM cleanup failure was reported as success' >&2; exit 1; }
grep -F 'Linux x86 release smoke cleanup failed in Lima VM' <<<"$cleanup_failure_output" >/dev/null

before_stale_log_lines="$(wc -l <"$LOG_PATH" | tr -d '[:space:]')"
set +e
stale_output="$(
  PATH="$PATH_PREFIX:$PATH" \
  VERSION="$VERSION" \
  SOURCE_COMMIT="0000000000000000000000000000000000000000" \
  DIST_DIR="$FAKE_ROOT/stale-dist" \
  SMOKE_ROOT="$TMP_ROOT/stale-smoke" \
  LSHARP_NATIVE_RELEASE_SMOKE_ROOT="$TMP_ROOT/stale-smoke" \
  KEEP_WORK_DIR=1 \
  NATIVE_OFFICIAL_REVIEW_TRUST_STORE="$TRUST_STORE" \
  NATIVE_OFFICIAL_REVIEW_LIFECYCLE="$LIFECYCLE" \
  MACOS_APP_CLI_ARTIFACT_DIR="$TMP_ROOT/artifact-aarch64-apple-darwin" \
  LINUX_APP_CLI_ARTIFACT_DIR="$TMP_ROOT/artifact-x86_64-unknown-linux-gnu" \
  MACOS_STAGE0_DIR="$TMP_ROOT/stage0-aarch64-apple-darwin" \
  LINUX_STAGE0_DIR="$TMP_ROOT/stage0-x86_64-unknown-linux-gnu" \
  MACOS_ROLLBACK_ARCHIVE="$TMP_ROOT/rollback-aarch64-apple-darwin.tar.gz" \
  LINUX_ROLLBACK_ARCHIVE="$TMP_ROOT/rollback-x86_64-unknown-linux-gnu.tar.gz" \
    bash "$FAKE_ROOT/scripts/ci/native-official-release-local.sh" 2>&1
)"
stale_status=$?
set -e
[[ "$stale_status" -ne 0 ]] || { echo 'stale source commit was accepted' >&2; exit 1; }
grep -F 'SOURCE_COMMIT must match current checkout HEAD' <<<"$stale_output" >/dev/null
after_stale_log_lines="$(wc -l <"$LOG_PATH" | tr -d '[:space:]')"
[[ "$after_stale_log_lines" == "$before_stale_log_lines" ]] \
  || { echo 'stale source commit reached a release or smoke boundary' >&2; exit 1; }

missing_identity_path="$TMP_ROOT/artifact-aarch64-apple-darwin/review-evidence-identity.json"
missing_identity_backup="$TMP_ROOT/missing-review-evidence-identity.json"
MISSING_IDENTITY_SMOKE_ROOT="$(mktemp -d /tmp/lsharp-native-official-missing-identity.XXXXXX)"
mv "$missing_identity_path" "$missing_identity_backup"
before_missing_identity_log_lines="$(wc -l <"$LOG_PATH" | tr -d '[:space:]')"
set +e
missing_identity_output="$(
  PATH="$PATH_PREFIX:$PATH" \
  VERSION="$VERSION" \
  SOURCE_COMMIT="$SOURCE_COMMIT" \
  DIST_DIR="$FAKE_ROOT/missing-identity-dist" \
  SMOKE_ROOT="$MISSING_IDENTITY_SMOKE_ROOT" \
  LSHARP_NATIVE_RELEASE_SMOKE_ROOT="$MISSING_IDENTITY_SMOKE_ROOT" \
  KEEP_WORK_DIR=1 \
  NATIVE_OFFICIAL_REVIEW_TRUST_STORE="$TRUST_STORE" \
  NATIVE_OFFICIAL_REVIEW_LIFECYCLE="$LIFECYCLE" \
  MACOS_APP_CLI_ARTIFACT_DIR="$TMP_ROOT/artifact-aarch64-apple-darwin" \
  LINUX_APP_CLI_ARTIFACT_DIR="$TMP_ROOT/artifact-x86_64-unknown-linux-gnu" \
  MACOS_STAGE0_DIR="$TMP_ROOT/stage0-aarch64-apple-darwin" \
  LINUX_STAGE0_DIR="$TMP_ROOT/stage0-x86_64-unknown-linux-gnu" \
  MACOS_ROLLBACK_ARCHIVE="$TMP_ROOT/rollback-aarch64-apple-darwin.tar.gz" \
  LINUX_ROLLBACK_ARCHIVE="$TMP_ROOT/rollback-x86_64-unknown-linux-gnu.tar.gz" \
    bash "$FAKE_ROOT/scripts/ci/native-official-release-local.sh" 2>&1
)"
missing_identity_status=$?
set -e
mv "$missing_identity_backup" "$missing_identity_path"
[[ "$missing_identity_status" -ne 0 ]] \
  || { echo 'provider snapshots without target identity were accepted' >&2; exit 1; }
grep -F 'review evidence identity is required when provider snapshots are supplied' \
  <<<"$missing_identity_output" >/dev/null
after_missing_identity_log_lines="$(wc -l <"$LOG_PATH" | tr -d '[:space:]')"
[[ "$after_missing_identity_log_lines" == "$before_missing_identity_log_lines" ]] \
  || { echo 'missing provider identity reached a release or smoke boundary' >&2; exit 1; }

VM_COPY_FAILURE_SMOKE_ROOT="$(mktemp -d /tmp/lsharp-native-official-vm-copy-failure.XXXXXX)"
VM_COPY_FAILURE_LOG="$TMP_ROOT/vm-copy-failure.log"
set +e
vm_copy_failure_output="$(
  FAIL_LIMACTL_COPY=1 \
  FAKE_LOG="$VM_COPY_FAILURE_LOG" \
  PATH="$PATH_PREFIX:$PATH" \
  VERSION="$VERSION" \
  SOURCE_COMMIT="$SOURCE_COMMIT" \
  DIST_DIR="$FAKE_ROOT/vm-copy-failure-dist" \
  SMOKE_ROOT="$VM_COPY_FAILURE_SMOKE_ROOT" \
  LSHARP_NATIVE_RELEASE_SMOKE_ROOT="$VM_COPY_FAILURE_SMOKE_ROOT" \
  KEEP_WORK_DIR=1 \
  NATIVE_OFFICIAL_REVIEW_TRUST_STORE="$TRUST_STORE" \
  NATIVE_OFFICIAL_REVIEW_LIFECYCLE="$LIFECYCLE" \
  MACOS_APP_CLI_ARTIFACT_DIR="$TMP_ROOT/artifact-aarch64-apple-darwin" \
  LINUX_APP_CLI_ARTIFACT_DIR="$TMP_ROOT/artifact-x86_64-unknown-linux-gnu" \
  MACOS_STAGE0_DIR="$TMP_ROOT/stage0-aarch64-apple-darwin" \
  LINUX_STAGE0_DIR="$TMP_ROOT/stage0-x86_64-unknown-linux-gnu" \
  MACOS_ROLLBACK_ARCHIVE="$TMP_ROOT/rollback-aarch64-apple-darwin.tar.gz" \
  LINUX_ROLLBACK_ARCHIVE="$TMP_ROOT/rollback-x86_64-unknown-linux-gnu.tar.gz" \
    bash "$FAKE_ROOT/scripts/ci/native-official-release-local.sh" 2>&1
)"
vm_copy_failure_status=$?
set -e
[[ "$vm_copy_failure_status" -ne 0 ]] \
  || { echo 'Linux VM archive copy failure was accepted' >&2; exit 1; }
vm_copy_failure_vm_dir="$(sed -n 's#^limactl copy .*lsharp-linux-x86:\(/tmp/lsharp-native-official-release-smoke-[^/]*/\).*#\1#p' "$VM_COPY_FAILURE_LOG" | head -n 1 | sed 's#/$##')"
[[ -n "$vm_copy_failure_vm_dir" ]] \
  || { echo 'Linux VM copy failure did not record a VM work directory' >&2; exit 1; }
awk -v work_dir="$vm_copy_failure_vm_dir" '
  /^limactl copy .*lsharp-linux-x86:/ { seen_copy=1 }
  seen_copy && $0 == "limactl shell lsharp-linux-x86 -- rm -rf " work_dir { found=1 }
  END { exit(found ? 0 : 1) }
' "$VM_COPY_FAILURE_LOG" \
  || { echo 'Linux VM copy failure left its work directory without cleanup' >&2; exit 1; }
[[ -s "$VM_COPY_FAILURE_LOG" ]] \
  || { echo 'Linux VM copy failure produced no invocation evidence' >&2; exit 1; }

set +e
PARTIAL_SMOKE_ROOT="$(mktemp -d /tmp/lsharp-native-official-snapshot-partial.XXXXXX)"
partial_output="$(
  PATH="$PATH_PREFIX:$PATH" \
  VERSION="$VERSION" \
  SOURCE_COMMIT="$SOURCE_COMMIT" \
  DIST_DIR="$FAKE_ROOT/partial-dist" \
  SMOKE_ROOT="$PARTIAL_SMOKE_ROOT" \
  LSHARP_NATIVE_RELEASE_SMOKE_ROOT="$PARTIAL_SMOKE_ROOT" \
  NATIVE_OFFICIAL_REVIEW_TRUST_STORE="$TRUST_STORE" \
  NATIVE_OFFICIAL_REVIEW_LIFECYCLE="" \
    bash "$FAKE_ROOT/scripts/ci/native-official-release-local.sh" 2>&1
)"
partial_status=$?
set -e
[[ "$partial_status" -ne 0 ]] || { echo 'partial snapshot input was accepted' >&2; exit 1; }
grep -F 'must be supplied together' <<<"$partial_output" >/dev/null

TRAVERSAL_BASE="$(mktemp -d /tmp/lsharp-native-official-snapshot-parent.XXXXXX)"
TRAVERSAL_ROOT="$(mktemp -d /tmp/outside-lsharp-native-official-snapshot.XXXXXX)"
printf '%s\n' 'sentinel' >"$TRAVERSAL_ROOT/sentinel"
set +e
traversal_output="$(
  FAKE_LOG="$LOG_PATH" \
  PATH="$PATH_PREFIX:$PATH" \
  VERSION="$VERSION" \
  SOURCE_COMMIT="$SOURCE_COMMIT" \
  DIST_DIR="$FAKE_ROOT/traversal-dist" \
  SMOKE_ROOT="$TRAVERSAL_BASE/../${TRAVERSAL_ROOT##*/}" \
  LSHARP_NATIVE_RELEASE_SMOKE_ROOT="$TRAVERSAL_BASE/../${TRAVERSAL_ROOT##*/}" \
  KEEP_WORK_DIR=1 \
  NATIVE_OFFICIAL_REVIEW_TRUST_STORE="$TRUST_STORE" \
  NATIVE_OFFICIAL_REVIEW_LIFECYCLE="$LIFECYCLE" \
  MACOS_APP_CLI_ARTIFACT_DIR="$TMP_ROOT/artifact-aarch64-apple-darwin" \
  LINUX_APP_CLI_ARTIFACT_DIR="$TMP_ROOT/artifact-x86_64-unknown-linux-gnu" \
  MACOS_STAGE0_DIR="$TMP_ROOT/stage0-aarch64-apple-darwin" \
  LINUX_STAGE0_DIR="$TMP_ROOT/stage0-x86_64-unknown-linux-gnu" \
  MACOS_ROLLBACK_ARCHIVE="$TMP_ROOT/rollback-aarch64-apple-darwin.tar.gz" \
  LINUX_ROLLBACK_ARCHIVE="$TMP_ROOT/rollback-x86_64-unknown-linux-gnu.tar.gz" \
    bash "$FAKE_ROOT/scripts/ci/native-official-release-local.sh" 2>&1
)"
traversal_status=$?
set -e
[[ "$traversal_status" -ne 0 ]] \
  || { echo 'cleanup path traversal was accepted' >&2; exit 1; }
grep -F 'unsafe cleanup path' <<<"$traversal_output" >/dev/null \
  || { echo 'cleanup path traversal did not expose a stable diagnostic' >&2; exit 1; }
[[ -s "$TRAVERSAL_ROOT/sentinel" ]] \
  || { echo 'cleanup path traversal removed an outside sentinel' >&2; exit 1; }

echo 'native official release snapshot tests: OK'
