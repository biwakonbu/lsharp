#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/lsharp-native-official-snapshot.XXXXXX")"
FAKE_ROOT="$TMP_ROOT/project"
LOG_PATH="$TMP_ROOT/invocations.log"
TRUST_STORE="$TMP_ROOT/trust-store.json"
LIFECYCLE="$TMP_ROOT/review-lifecycle.jsonl"
REVIEW_ATTESTATION_REPORT="$TMP_ROOT/review-attestation-report.json"
SOURCE_COMMIT=""
VERSION="v0.0.0-test"
PATH_PREFIX="$TMP_ROOT/bin"
SMOKE_ROOT="$(mktemp -d /tmp/lsharp-native-official-snapshot-smoke.XXXXXX)"
SOURCE_SMOKE_EVIDENCE_ROOT="$TMP_ROOT/source-smoke-evidence"
SOURCE_SMOKE_EVIDENCE_ROOT_CANONICAL="$(python3 - "$SOURCE_SMOKE_EVIDENCE_ROOT" <<'PY'
import pathlib
import sys

print(pathlib.Path(sys.argv[1]).resolve())
PY
)"
PARTIAL_SMOKE_ROOT=""
MISSING_IDENTITY_SMOKE_ROOT=""
PROVIDER_IDENTITY_SMOKE_ROOT=""
IDENTITY_SCHEMA_SMOKE_ROOT=""
PROVIDER_CLOCK_SMOKE_ROOT=""
PROVIDER_ARTIFACT_BINDING_SMOKE_ROOT=""
VM_COPY_FAILURE_SMOKE_ROOT=""
RUNNING_VM_SMOKE_ROOT=""
CLEANUP_FAILURE_SMOKE_ROOT=""
MISSING_REPORT_SMOKE_ROOT=""
PAYLOAD_MISMATCH_SMOKE_ROOT=""
IDENTITY_MISMATCH_SMOKE_ROOT=""
REPORT_FREE_SMOKE_ROOT=""
REPORT_FREE_EVIDENCE_ROOT=""
MISMATCH_SMOKE_ROOT=""
MISMATCH_EVIDENCE_ROOT=""
TRAVERSAL_ROOT=""
TRAVERSAL_BASE=""
HOSTGEN_REPLAY_LOCK_PATH="/tmp/lsharp-native-official-snapshot-hostgen-lock.$$"
cleanup() {
  rm -rf "$TMP_ROOT" "$SMOKE_ROOT"
  rm -rf "$HOSTGEN_REPLAY_LOCK_PATH"
  [[ -z "$PARTIAL_SMOKE_ROOT" ]] || rm -rf "$PARTIAL_SMOKE_ROOT"
  [[ -z "$MISSING_IDENTITY_SMOKE_ROOT" ]] || rm -rf "$MISSING_IDENTITY_SMOKE_ROOT"
  [[ -z "$PROVIDER_IDENTITY_SMOKE_ROOT" ]] || rm -rf "$PROVIDER_IDENTITY_SMOKE_ROOT"
  [[ -z "$IDENTITY_SCHEMA_SMOKE_ROOT" ]] || rm -rf "$IDENTITY_SCHEMA_SMOKE_ROOT"
  [[ -z "$PROVIDER_CLOCK_SMOKE_ROOT" ]] || rm -rf "$PROVIDER_CLOCK_SMOKE_ROOT"
  [[ -z "$PROVIDER_ARTIFACT_BINDING_SMOKE_ROOT" ]] || rm -rf "$PROVIDER_ARTIFACT_BINDING_SMOKE_ROOT"
  [[ -z "$VM_COPY_FAILURE_SMOKE_ROOT" ]] || rm -rf "$VM_COPY_FAILURE_SMOKE_ROOT"
  [[ -z "$RUNNING_VM_SMOKE_ROOT" ]] || rm -rf "$RUNNING_VM_SMOKE_ROOT"
  [[ -z "$CLEANUP_FAILURE_SMOKE_ROOT" ]] || rm -rf "$CLEANUP_FAILURE_SMOKE_ROOT"
  [[ -z "$MISSING_REPORT_SMOKE_ROOT" ]] || rm -rf "$MISSING_REPORT_SMOKE_ROOT"
  [[ -z "$PAYLOAD_MISMATCH_SMOKE_ROOT" ]] || rm -rf "$PAYLOAD_MISMATCH_SMOKE_ROOT"
  [[ -z "$IDENTITY_MISMATCH_SMOKE_ROOT" ]] || rm -rf "$IDENTITY_MISMATCH_SMOKE_ROOT"
  [[ -z "$REPORT_FREE_SMOKE_ROOT" ]] || rm -rf "$REPORT_FREE_SMOKE_ROOT"
  [[ -z "$REPORT_FREE_EVIDENCE_ROOT" ]] || rm -rf "$REPORT_FREE_EVIDENCE_ROOT"
  [[ -z "$MISMATCH_SMOKE_ROOT" ]] || rm -rf "$MISMATCH_SMOKE_ROOT"
  [[ -z "$MISMATCH_EVIDENCE_ROOT" ]] || rm -rf "$MISMATCH_EVIDENCE_ROOT"
  [[ -z "$TRAVERSAL_ROOT" ]] || rm -rf "$TRAVERSAL_ROOT"
  [[ -z "$TRAVERSAL_BASE" ]] || rm -rf "$TRAVERSAL_BASE"
}
trap cleanup EXIT

export LSHARP_NATIVE_LINUX_X86_HOST_REPLAY_LOCK_DIR="$HOSTGEN_REPLAY_LOCK_PATH"

mkdir -p "$FAKE_ROOT/scripts/ci" "$PATH_PREFIX" "$FAKE_ROOT/dist"
FAKE_ROOT_CANONICAL="$(cd "$FAKE_ROOT" && pwd)"
printf '%s\n' '{"keys":["release-key"]}' >"$TRUST_STORE"
printf '%s\n' '{"review_id":"review:orchestrator/r1","sequence":1,"state":"active"}' >"$LIFECYCLE"
cat >"$REVIEW_ATTESTATION_REPORT" <<'JSON'
{"review_attestations":[{"review_id":"review:checkout/reviewer-001","subject_digest":"sha256:subject-001","source_commit":"0123456789abcdef","provenance_digest":"sha256:review-001","provider":"github","key_id":"org/reviews-2026","algorithm":"ed25519","signature":"AAECAw","issued_at":"2026-08-01T00:00:00Z","expires_at":"2026-09-01T00:00:00Z","sequence":3,"state":"unverified","canonical_bytes":[0,1,2],"span":{"start":12,"end":34}}]}
JSON

cp "$ROOT/scripts/ci/native-official-release-local.sh" "$FAKE_ROOT/scripts/ci/"
cp "$ROOT/scripts/ci/verify-native-release-identity.py" "$FAKE_ROOT/scripts/ci/"
cp "$ROOT/scripts/ci/review_identity_timestamp.py" "$FAKE_ROOT/scripts/ci/"

cat >"$FAKE_ROOT/scripts/ci/native-selfhost-dev-source-file-smoke.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
evidence="${NATIVE_SELFHOST_SOURCE_SMOKE_EVIDENCE_DIR:-}"
if [[ -n "$evidence" ]]; then
  mkdir -p "$evidence"
  python3 - "${NATIVE_SELFHOST_REVIEW_ATTESTATION_REPORT:-}" "$evidence/manifest.json" <<'PY'
import json
import hashlib
import os
import pathlib
import sys

report_path = pathlib.Path(sys.argv[1]) if sys.argv[1] else None
if "FAKE_EXPECT_ATTESTATION_REPORT" in os.environ and not report_path:
    raise SystemExit("fake Mac source smoke did not receive review attestation report")
stage0_dir = pathlib.Path(os.environ["NATIVE_STAGE0_DIR"])
stage0_manifest_path = stage0_dir / "manifest.json"
stage0_manifest = json.loads(stage0_manifest_path.read_text(encoding="utf-8"))
records = []
for path in sorted(stage0_dir.rglob("*")):
    if path.is_file() and not path.is_symlink():
        records.append(
            path.relative_to(stage0_dir).as_posix().encode()
            + b"\0"
            + str(path.stat().st_size).encode()
            + b"\0"
            + hashlib.sha256(path.read_bytes()).hexdigest().encode()
            + b"\n"
        )
payload_digest = hashlib.sha256(b"".join(records)).hexdigest()
manifest = {
    "target": "aarch64-apple-darwin",
    "source_commit": stage0_manifest["source_commit"],
    "stage0_manifest_sha256": hashlib.sha256(stage0_manifest_path.read_bytes()).hexdigest(),
    "stage0_payload_sha256": payload_digest,
}
if os.environ.get("FAKE_IDENTITY_MODE") == "mac-mismatch":
    manifest["stage0_manifest_sha256"] = "wrong-mac-manifest"
if report_path:
    report = json.loads(report_path.read_text(encoding="utf-8"))
    attestations = report["review_attestations"]
    manifest["review_attestations"] = attestations
pathlib.Path(sys.argv[2]).write_text(json.dumps(manifest) + "\n", encoding="utf-8")
PY
fi
printf '%s\n' "runtime mac stage0=${NATIVE_STAGE0_DIR:-} source=${NATIVE_SELFHOST_SOURCE_ROOT:-} evidence=$evidence report=${NATIVE_SELFHOST_REVIEW_ATTESTATION_REPORT:-}" >>"$FAKE_LOG"
SH
chmod +x "$FAKE_ROOT/scripts/ci/native-selfhost-dev-source-file-smoke.sh"

cat >"$FAKE_ROOT/scripts/ci/native-linux-x86-native-stage0-source-file-smoke.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
evidence="${LSHARP_NATIVE_LINUX_X86_SOURCE_SMOKE_EVIDENCE_DIR:-}"
if [[ -n "$evidence" ]]; then
  mkdir -p "$evidence"
  python3 - "${LSHARP_NATIVE_LINUX_X86_REVIEW_ATTESTATION_REPORT:-}" "$evidence/manifest.json" <<'PY'
import json
import hashlib
import os
import pathlib
import sys

report_path = pathlib.Path(sys.argv[1]) if sys.argv[1] else None
if "FAKE_EXPECT_ATTESTATION_REPORT" in os.environ and not report_path:
    raise SystemExit("fake Linux source smoke did not receive review attestation report")
stage0_dir = pathlib.Path(os.environ["LSHARP_NATIVE_LINUX_X86_STAGE0_DIR"])
stage0_manifest_path = stage0_dir / "manifest.json"
stage0_manifest = json.loads(stage0_manifest_path.read_text(encoding="utf-8"))
records = []
for path in sorted(stage0_dir.rglob("*")):
    if path.is_file() and not path.is_symlink():
        records.append(
            path.relative_to(stage0_dir).as_posix().encode()
            + b"\0"
            + str(path.stat().st_size).encode()
            + b"\0"
            + hashlib.sha256(path.read_bytes()).hexdigest().encode()
            + b"\n"
        )
payload_digest = hashlib.sha256(b"".join(records)).hexdigest()
manifest = {
    "target": "x86_64-unknown-linux-gnu",
    "source_commit": stage0_manifest["source_commit"],
    "stage0_manifest_sha256": hashlib.sha256(stage0_manifest_path.read_bytes()).hexdigest(),
    "stage0_payload_sha256": payload_digest,
}
if report_path:
    report = json.loads(report_path.read_text(encoding="utf-8"))
    attestations = report["review_attestations"]
    if os.environ.get("FAKE_ATTESTATION_MODE") == "linux-mismatch":
        attestations = [dict(attestations[0], state="verified")]
    manifest["review_attestations"] = attestations
if os.environ.get("FAKE_PAYLOAD_MODE") == "linux-mismatch":
    manifest["stage0_payload_sha256"] = "wrong-linux-payload"
if os.environ.get("FAKE_IDENTITY_MODE") == "linux-mismatch":
    manifest["stage0_manifest_sha256"] = "wrong-linux-manifest"
pathlib.Path(sys.argv[2]).write_text(json.dumps(manifest) + "\n", encoding="utf-8")
PY
fi
printf '%s\n' "runtime linux stage0=${LSHARP_NATIVE_LINUX_X86_STAGE0_DIR:-} evidence=$evidence report=${LSHARP_NATIVE_LINUX_X86_REVIEW_ATTESTATION_REPORT:-}" >>"$FAKE_LOG"
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
source_commit="$(git rev-parse HEAD)"
printf '%s\n' "{\"kind\":\"lsharp-native-selfhost-stage0\",\"target\":\"$STAGE0_TARGET\",\"source_commit\":\"$source_commit\"}" >"$STAGE0_DIR/manifest.json"
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
python3 - "$SOURCE_COMMIT" "$TRUST_STORE" "$LIFECYCLE" "$TMP_ROOT" <<'PY'
import hashlib
import json
import pathlib
import sys

source_commit, trust_store, lifecycle, tmp_root = map(pathlib.Path, sys.argv[1:])
artifact = tmp_root / "artifact-aarch64-apple-darwin" / "program.native"
identity = {
    "subject_digest": "sha256:" + "c" * 64,
    "source_commit": str(source_commit),
    "artifact_digest": "sha256:" + hashlib.sha256(artifact.read_bytes()).hexdigest(),
    "trust_store_digest": "sha256:" + hashlib.sha256(trust_store.read_bytes()).hexdigest(),
    "lifecycle_digest": "sha256:" + hashlib.sha256(lifecycle.read_bytes()).hexdigest(),
    "now": "2026-08-15T00:00:00Z",
}
for identity_path in (
    tmp_root / "artifact-aarch64-apple-darwin" / "review-evidence-identity.json",
    tmp_root / "artifact-x86_64-unknown-linux-gnu" / "review-evidence-identity.json",
    tmp_root / "stage0-aarch64-apple-darwin" / "review-evidence-identity.json",
    tmp_root / "stage0-x86_64-unknown-linux-gnu" / "review-evidence-identity.json",
):
    identity_path.write_text(json.dumps(identity, separators=(",", ":")) + "\n", encoding="utf-8")
PY

UNSAFE_EVIDENCE_DIST="$TMP_ROOT/unsafe-evidence-dist"
set +e
unsafe_evidence_output="$({
  FAKE_LOG="$LOG_PATH" \
  PATH="$PATH_PREFIX:$PATH" \
  VERSION="$VERSION" \
  SOURCE_COMMIT="$SOURCE_COMMIT" \
  DIST_DIR="$UNSAFE_EVIDENCE_DIST" \
  SMOKE_ROOT="$SMOKE_ROOT" \
  LSHARP_NATIVE_RELEASE_SMOKE_ROOT="$SMOKE_ROOT" \
  NATIVE_OFFICIAL_SOURCE_SMOKE_EVIDENCE_ROOT="$FAKE_ROOT" \
    bash "$FAKE_ROOT/scripts/ci/native-official-release-local.sh"
} 2>&1)"
unsafe_evidence_status=$?
set -e
[[ "$unsafe_evidence_status" -ne 0 ]] \
  || { echo 'repository-root source smoke evidence unexpectedly accepted' >&2; exit 1; }
grep -F 'source smoke evidence root' <<<"$unsafe_evidence_output" >/dev/null \
  || { echo 'unsafe source smoke evidence root diagnostic was missing' >&2; echo "$unsafe_evidence_output" >&2; exit 1; }
[[ ! -e "$UNSAFE_EVIDENCE_DIST" ]] \
  || { echo 'official gate created release output before evidence root preflight' >&2; exit 1; }

MISSING_REPORT_SMOKE_ROOT="$(mktemp -d /tmp/lsharp-native-official-missing-report.XXXXXX)"
before_missing_report_log_lines=0
if [[ -e "$LOG_PATH" ]]; then
  before_missing_report_log_lines="$(wc -l <"$LOG_PATH" | tr -d '[:space:]')"
fi
missing_report_output_path="$TMP_ROOT/missing-report-output.log"
set +e
FAKE_LOG="$LOG_PATH" \
PATH="$PATH_PREFIX:$PATH" \
VERSION="$VERSION" \
SOURCE_COMMIT="$SOURCE_COMMIT" \
DIST_DIR="$TMP_ROOT/missing-report-dist" \
SMOKE_ROOT="$MISSING_REPORT_SMOKE_ROOT" \
LSHARP_NATIVE_RELEASE_SMOKE_ROOT="$MISSING_REPORT_SMOKE_ROOT" \
NATIVE_OFFICIAL_SOURCE_SMOKE_EVIDENCE_ROOT="$SOURCE_SMOKE_EVIDENCE_ROOT/missing-report" \
NATIVE_OFFICIAL_REVIEW_ATTESTATION_REPORT="$TMP_ROOT/missing-review-attestation.json" \
  bash "$FAKE_ROOT/scripts/ci/native-official-release-local.sh" >"$missing_report_output_path" 2>&1
missing_report_status=$?
set -e
missing_report_output="$(<"$missing_report_output_path")"
[[ "$missing_report_status" -ne 0 ]] \
  || { echo 'missing review attestation report was unexpectedly accepted' >&2; exit 1; }
grep -F 'review attestation report' <<<"$missing_report_output" >/dev/null \
  || { echo 'missing review attestation report diagnostic was missing' >&2; echo "$missing_report_output" >&2; exit 1; }
after_missing_report_log_lines=0
if [[ -e "$LOG_PATH" ]]; then
  after_missing_report_log_lines="$(wc -l <"$LOG_PATH" | tr -d '[:space:]')"
fi
[[ "$after_missing_report_log_lines" == "$before_missing_report_log_lines" ]] \
  || { echo 'official gate invoked a target before missing report preflight' >&2; exit 1; }
[[ ! -e "$TMP_ROOT/missing-report-dist" ]] \
  || { echo 'official gate created release output before missing report preflight' >&2; exit 1; }

FAKE_LOG="$LOG_PATH" \
PATH="$PATH_PREFIX:$PATH" \
VERSION="$VERSION" \
SOURCE_COMMIT="$SOURCE_COMMIT" \
DIST_DIR="$FAKE_ROOT/dist" \
SMOKE_ROOT="$SMOKE_ROOT" \
LSHARP_NATIVE_RELEASE_SMOKE_ROOT="$SMOKE_ROOT" \
KEEP_WORK_DIR=1 \
NATIVE_OFFICIAL_SOURCE_SMOKE_EVIDENCE_ROOT="$SOURCE_SMOKE_EVIDENCE_ROOT" \
NATIVE_OFFICIAL_REVIEW_ATTESTATION_REPORT="$REVIEW_ATTESTATION_REPORT" \
FAKE_EXPECT_ATTESTATION_REPORT=1 \
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
grep -F "runtime mac stage0=$SMOKE_ROOT/stage0-aarch64-apple-darwin source=$FAKE_ROOT_CANONICAL/selfhost evidence=$SOURCE_SMOKE_EVIDENCE_ROOT_CANONICAL/aarch64-apple-darwin" "$LOG_PATH" >/dev/null
grep -F "runtime linux stage0=$SMOKE_ROOT/stage0-x86_64-unknown-linux-gnu evidence=$SOURCE_SMOKE_EVIDENCE_ROOT_CANONICAL/x86_64-unknown-linux-gnu" "$LOG_PATH" >/dev/null
[[ -s "$SOURCE_SMOKE_EVIDENCE_ROOT/aarch64-apple-darwin/manifest.json" ]] \
  || { echo 'Mac source smoke evidence was not retained' >&2; exit 1; }
[[ -s "$SOURCE_SMOKE_EVIDENCE_ROOT/x86_64-unknown-linux-gnu/manifest.json" ]] \
  || { echo 'Linux source smoke evidence was not retained' >&2; exit 1; }
python3 - "$REVIEW_ATTESTATION_REPORT" \
  "$SOURCE_SMOKE_EVIDENCE_ROOT/aarch64-apple-darwin/manifest.json" \
  "$SOURCE_SMOKE_EVIDENCE_ROOT/x86_64-unknown-linux-gnu/manifest.json" \
  "$SMOKE_ROOT/stage0-aarch64-apple-darwin" \
  "$SMOKE_ROOT/stage0-x86_64-unknown-linux-gnu" <<'PY'
import hashlib
import json
import pathlib
import sys

expected = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))["review_attestations"]
for manifest_path, stage0_dir_arg in zip(
    map(pathlib.Path, sys.argv[2:4]),
    sys.argv[4:6],
):
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    assert manifest.get("review_attestations") == expected, manifest_path
    stage0_dir = pathlib.Path(stage0_dir_arg)
    records = []
    for path in sorted(stage0_dir.rglob("*")):
        if path.is_file() and not path.is_symlink():
            records.append(
                path.relative_to(stage0_dir).as_posix().encode()
                + b"\0"
                + str(path.stat().st_size).encode()
                + b"\0"
                + hashlib.sha256(path.read_bytes()).hexdigest().encode()
                + b"\n"
    )
    expected_payload = hashlib.sha256(b"".join(records)).hexdigest()
    stage0_manifest_path = stage0_dir / "manifest.json"
    stage0_manifest = json.loads(stage0_manifest_path.read_text(encoding="utf-8"))
    assert manifest["target"] == stage0_manifest["target"]
    assert manifest["source_commit"] == stage0_manifest["source_commit"]
    assert manifest["stage0_manifest_sha256"] == hashlib.sha256(
        stage0_manifest_path.read_bytes()
    ).hexdigest()
    assert manifest["stage0_payload_sha256"] == expected_payload, manifest_path
PY
grep -F "report=$REVIEW_ATTESTATION_REPORT" "$LOG_PATH" >/dev/null
grep -F "limactl stop lsharp-linux-x86" "$LOG_PATH" >/dev/null

REPORT_FREE_SMOKE_ROOT="$(mktemp -d /tmp/lsharp-native-official-report-free.XXXXXX)"
REPORT_FREE_EVIDENCE_ROOT="$TMP_ROOT/report-free-evidence"
FAKE_LOG="$LOG_PATH" \
PATH="$PATH_PREFIX:$PATH" \
VERSION="$VERSION" \
SOURCE_COMMIT="$SOURCE_COMMIT" \
DIST_DIR="$TMP_ROOT/report-free-dist" \
SMOKE_ROOT="$REPORT_FREE_SMOKE_ROOT" \
LSHARP_NATIVE_RELEASE_SMOKE_ROOT="$REPORT_FREE_SMOKE_ROOT" \
NATIVE_OFFICIAL_SOURCE_SMOKE_EVIDENCE_ROOT="$REPORT_FREE_EVIDENCE_ROOT" \
NATIVE_OFFICIAL_REVIEW_TRUST_STORE="$TRUST_STORE" \
NATIVE_OFFICIAL_REVIEW_LIFECYCLE="$LIFECYCLE" \
MACOS_APP_CLI_ARTIFACT_DIR="$TMP_ROOT/artifact-aarch64-apple-darwin" \
LINUX_APP_CLI_ARTIFACT_DIR="$TMP_ROOT/artifact-x86_64-unknown-linux-gnu" \
MACOS_STAGE0_DIR="$TMP_ROOT/stage0-aarch64-apple-darwin" \
LINUX_STAGE0_DIR="$TMP_ROOT/stage0-x86_64-unknown-linux-gnu" \
MACOS_ROLLBACK_ARCHIVE="$TMP_ROOT/rollback-aarch64-apple-darwin.tar.gz" \
LINUX_ROLLBACK_ARCHIVE="$TMP_ROOT/rollback-x86_64-unknown-linux-gnu.tar.gz" \
  bash "$FAKE_ROOT/scripts/ci/native-official-release-local.sh"
python3 \
  "$REPORT_FREE_EVIDENCE_ROOT/aarch64-apple-darwin/manifest.json" \
  "$REPORT_FREE_EVIDENCE_ROOT/x86_64-unknown-linux-gnu/manifest.json" <<'PY'
import json
import pathlib
import sys

for manifest_path in map(pathlib.Path, sys.argv[1:]):
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    assert "review_attestations" not in manifest, manifest_path
PY

MISMATCH_SMOKE_ROOT="$(mktemp -d /tmp/lsharp-native-official-mismatch.XXXXXX)"
MISMATCH_EVIDENCE_ROOT="$(mktemp -d /tmp/lsharp-native-official-mismatch-evidence.XXXXXX)"
mismatch_output_path="$TMP_ROOT/mismatch-output.log"
set +e
FAKE_LOG="$LOG_PATH" \
PATH="$PATH_PREFIX:$PATH" \
VERSION="$VERSION" \
SOURCE_COMMIT="$SOURCE_COMMIT" \
DIST_DIR="$TMP_ROOT/mismatch-dist" \
SMOKE_ROOT="$MISMATCH_SMOKE_ROOT" \
LSHARP_NATIVE_RELEASE_SMOKE_ROOT="$MISMATCH_SMOKE_ROOT" \
NATIVE_OFFICIAL_SOURCE_SMOKE_EVIDENCE_ROOT="$MISMATCH_EVIDENCE_ROOT" \
NATIVE_OFFICIAL_REVIEW_ATTESTATION_REPORT="$REVIEW_ATTESTATION_REPORT" \
FAKE_EXPECT_ATTESTATION_REPORT=1 \
FAKE_ATTESTATION_MODE=linux-mismatch \
NATIVE_OFFICIAL_REVIEW_TRUST_STORE="$TRUST_STORE" \
NATIVE_OFFICIAL_REVIEW_LIFECYCLE="$LIFECYCLE" \
MACOS_APP_CLI_ARTIFACT_DIR="$TMP_ROOT/artifact-aarch64-apple-darwin" \
LINUX_APP_CLI_ARTIFACT_DIR="$TMP_ROOT/artifact-x86_64-unknown-linux-gnu" \
MACOS_STAGE0_DIR="$TMP_ROOT/stage0-aarch64-apple-darwin" \
LINUX_STAGE0_DIR="$TMP_ROOT/stage0-x86_64-unknown-linux-gnu" \
MACOS_ROLLBACK_ARCHIVE="$TMP_ROOT/rollback-aarch64-apple-darwin.tar.gz" \
LINUX_ROLLBACK_ARCHIVE="$TMP_ROOT/rollback-x86_64-unknown-linux-gnu.tar.gz" \
  bash "$FAKE_ROOT/scripts/ci/native-official-release-local.sh" >"$mismatch_output_path" 2>&1
mismatch_status=$?
set -e
mismatch_output="$(<"$mismatch_output_path")"
[[ "$mismatch_status" -ne 0 ]] \
  || { echo 'target review attestation mismatch was unexpectedly accepted' >&2; exit 1; }
grep -F 'source smoke evidence review_attestations mismatch' <<<"$mismatch_output" >/dev/null \
  || { echo 'target review attestation mismatch diagnostic was missing' >&2; echo "$mismatch_output" >&2; exit 1; }

PAYLOAD_MISMATCH_SMOKE_ROOT="$(mktemp -d /tmp/lsharp-native-official-payload-mismatch.XXXXXX)"
payload_mismatch_output_path="$TMP_ROOT/payload-mismatch-output.log"
set +e
FAKE_LOG="$LOG_PATH" \
PATH="$PATH_PREFIX:$PATH" \
VERSION="$VERSION" \
SOURCE_COMMIT="$SOURCE_COMMIT" \
DIST_DIR="$TMP_ROOT/payload-mismatch-dist" \
SMOKE_ROOT="$PAYLOAD_MISMATCH_SMOKE_ROOT" \
LSHARP_NATIVE_RELEASE_SMOKE_ROOT="$PAYLOAD_MISMATCH_SMOKE_ROOT" \
NATIVE_OFFICIAL_SOURCE_SMOKE_EVIDENCE_ROOT="$TMP_ROOT/payload-mismatch-evidence" \
NATIVE_OFFICIAL_REVIEW_ATTESTATION_REPORT="$REVIEW_ATTESTATION_REPORT" \
FAKE_EXPECT_ATTESTATION_REPORT=1 \
FAKE_PAYLOAD_MODE=linux-mismatch \
NATIVE_OFFICIAL_REVIEW_TRUST_STORE="$TRUST_STORE" \
NATIVE_OFFICIAL_REVIEW_LIFECYCLE="$LIFECYCLE" \
MACOS_APP_CLI_ARTIFACT_DIR="$TMP_ROOT/artifact-aarch64-apple-darwin" \
LINUX_APP_CLI_ARTIFACT_DIR="$TMP_ROOT/artifact-x86_64-unknown-linux-gnu" \
MACOS_STAGE0_DIR="$TMP_ROOT/stage0-aarch64-apple-darwin" \
LINUX_STAGE0_DIR="$TMP_ROOT/stage0-x86_64-unknown-linux-gnu" \
MACOS_ROLLBACK_ARCHIVE="$TMP_ROOT/rollback-aarch64-apple-darwin.tar.gz" \
LINUX_ROLLBACK_ARCHIVE="$TMP_ROOT/rollback-x86_64-unknown-linux-gnu.tar.gz" \
  bash "$FAKE_ROOT/scripts/ci/native-official-release-local.sh" >"$payload_mismatch_output_path" 2>&1
payload_mismatch_status=$?
set -e
payload_mismatch_output="$(<"$payload_mismatch_output_path")"
[[ "$payload_mismatch_status" -ne 0 ]] \
  || { echo 'target stage0 payload mismatch was unexpectedly accepted' >&2; exit 1; }
grep -F 'source smoke evidence stage0 identity mismatch' <<<"$payload_mismatch_output" >/dev/null \
  || { echo 'target stage0 payload mismatch diagnostic was missing' >&2; echo "$payload_mismatch_output" >&2; exit 1; }

IDENTITY_MISMATCH_SMOKE_ROOT="$(mktemp -d /tmp/lsharp-native-official-identity-mismatch.XXXXXX)"
identity_mismatch_output_path="$TMP_ROOT/identity-mismatch-output.log"
set +e
FAKE_LOG="$LOG_PATH" \
PATH="$PATH_PREFIX:$PATH" \
VERSION="$VERSION" \
SOURCE_COMMIT="$SOURCE_COMMIT" \
DIST_DIR="$TMP_ROOT/identity-mismatch-dist" \
SMOKE_ROOT="$IDENTITY_MISMATCH_SMOKE_ROOT" \
LSHARP_NATIVE_RELEASE_SMOKE_ROOT="$IDENTITY_MISMATCH_SMOKE_ROOT" \
NATIVE_OFFICIAL_SOURCE_SMOKE_EVIDENCE_ROOT="$TMP_ROOT/identity-mismatch-evidence" \
NATIVE_OFFICIAL_REVIEW_ATTESTATION_REPORT="$REVIEW_ATTESTATION_REPORT" \
FAKE_EXPECT_ATTESTATION_REPORT=1 \
FAKE_IDENTITY_MODE=linux-mismatch \
NATIVE_OFFICIAL_REVIEW_TRUST_STORE="$TRUST_STORE" \
NATIVE_OFFICIAL_REVIEW_LIFECYCLE="$LIFECYCLE" \
MACOS_APP_CLI_ARTIFACT_DIR="$TMP_ROOT/artifact-aarch64-apple-darwin" \
LINUX_APP_CLI_ARTIFACT_DIR="$TMP_ROOT/artifact-x86_64-unknown-linux-gnu" \
MACOS_STAGE0_DIR="$TMP_ROOT/stage0-aarch64-apple-darwin" \
LINUX_STAGE0_DIR="$TMP_ROOT/stage0-x86_64-unknown-linux-gnu" \
MACOS_ROLLBACK_ARCHIVE="$TMP_ROOT/rollback-aarch64-apple-darwin.tar.gz" \
LINUX_ROLLBACK_ARCHIVE="$TMP_ROOT/rollback-x86_64-unknown-linux-gnu.tar.gz" \
  bash "$FAKE_ROOT/scripts/ci/native-official-release-local.sh" >"$identity_mismatch_output_path" 2>&1
identity_mismatch_status=$?
set -e
identity_mismatch_output="$(<"$identity_mismatch_output_path")"
[[ "$identity_mismatch_status" -ne 0 ]] \
  || { echo 'target stage0 manifest identity mismatch was unexpectedly accepted' >&2; exit 1; }
grep -F 'source smoke evidence stage0 identity mismatch' <<<"$identity_mismatch_output" >/dev/null \
  || { echo 'target stage0 manifest identity mismatch diagnostic was missing' >&2; echo "$identity_mismatch_output" >&2; exit 1; }

before_provider_clock_log_lines="$(wc -l <"$LOG_PATH" | tr -d '[:space:]')"
PROVIDER_CLOCK_SMOKE_ROOT="$(mktemp -d /tmp/lsharp-native-official-provider-clock.XXXXXX)"
set +e
provider_clock_output="$(
  FAKE_LOG="$LOG_PATH" \
  PATH="$PATH_PREFIX:$PATH" \
  VERSION="$VERSION" \
  SOURCE_COMMIT="$SOURCE_COMMIT" \
  DIST_DIR="$FAKE_ROOT/provider-clock-dist" \
  SMOKE_ROOT="$PROVIDER_CLOCK_SMOKE_ROOT" \
  LSHARP_NATIVE_RELEASE_SMOKE_ROOT="$PROVIDER_CLOCK_SMOKE_ROOT" \
  KEEP_WORK_DIR=1 \
  NATIVE_OFFICIAL_REVIEW_TRUST_STORE="$TRUST_STORE" \
  NATIVE_OFFICIAL_REVIEW_LIFECYCLE="$LIFECYCLE" \
  NATIVE_OFFICIAL_REVIEW_VERIFICATION_NOW="2026-08-14T23:59:59Z" \
  MACOS_APP_CLI_ARTIFACT_DIR="$TMP_ROOT/artifact-aarch64-apple-darwin" \
  LINUX_APP_CLI_ARTIFACT_DIR="$TMP_ROOT/artifact-x86_64-unknown-linux-gnu" \
  MACOS_STAGE0_DIR="$TMP_ROOT/stage0-aarch64-apple-darwin" \
  LINUX_STAGE0_DIR="$TMP_ROOT/stage0-x86_64-unknown-linux-gnu" \
  MACOS_ROLLBACK_ARCHIVE="$TMP_ROOT/rollback-aarch64-apple-darwin.tar.gz" \
  LINUX_ROLLBACK_ARCHIVE="$TMP_ROOT/rollback-x86_64-unknown-linux-gnu.tar.gz" \
    bash "$FAKE_ROOT/scripts/ci/native-official-release-local.sh" 2>&1
)"
provider_clock_status=$?
set -e
[[ "$provider_clock_status" -ne 0 ]] \
  || { echo 'future provider identity now was accepted by official release gate' >&2; exit 1; }
grep -F 'identity now is after verification now' <<<"$provider_clock_output" >/dev/null \
  || { echo 'provider identity freshness diagnostic was missing' >&2; echo "$provider_clock_output" >&2; exit 1; }
after_provider_clock_log_lines="$(wc -l <"$LOG_PATH" | tr -d '[:space:]')"
[[ "$after_provider_clock_log_lines" == "$before_provider_clock_log_lines" ]] \
  || { echo 'provider identity freshness failure reached a release or smoke boundary' >&2; exit 1; }

ARTIFACT_BINDING_PROGRAM="$TMP_ROOT/artifact-aarch64-apple-darwin/program.native"
ARTIFACT_BINDING_BACKUP="$TMP_ROOT/provider-artifact-binding-program.native"
cp "$ARTIFACT_BINDING_PROGRAM" "$ARTIFACT_BINDING_BACKUP"
printf '%s\n' 'tampered artifact bytes' >>"$ARTIFACT_BINDING_PROGRAM"
before_artifact_binding_log_lines="$(wc -l <"$LOG_PATH" | tr -d '[:space:]')"
PROVIDER_ARTIFACT_BINDING_SMOKE_ROOT="$(mktemp -d /tmp/lsharp-native-official-provider-artifact.XXXXXX)"
set +e
artifact_binding_output="$(
  FAKE_LOG="$LOG_PATH" \
  PATH="$PATH_PREFIX:$PATH" \
  VERSION="$VERSION" \
  SOURCE_COMMIT="$SOURCE_COMMIT" \
  DIST_DIR="$FAKE_ROOT/provider-artifact-binding-dist" \
  SMOKE_ROOT="$PROVIDER_ARTIFACT_BINDING_SMOKE_ROOT" \
  LSHARP_NATIVE_RELEASE_SMOKE_ROOT="$PROVIDER_ARTIFACT_BINDING_SMOKE_ROOT" \
  KEEP_WORK_DIR=1 \
  NATIVE_OFFICIAL_REVIEW_TRUST_STORE="$TRUST_STORE" \
  NATIVE_OFFICIAL_REVIEW_LIFECYCLE="$LIFECYCLE" \
  NATIVE_OFFICIAL_REVIEW_VERIFICATION_NOW="2026-08-15T00:00:00Z" \
  MACOS_APP_CLI_ARTIFACT_DIR="$TMP_ROOT/artifact-aarch64-apple-darwin" \
  LINUX_APP_CLI_ARTIFACT_DIR="$TMP_ROOT/artifact-x86_64-unknown-linux-gnu" \
  MACOS_STAGE0_DIR="$TMP_ROOT/stage0-aarch64-apple-darwin" \
  LINUX_STAGE0_DIR="$TMP_ROOT/stage0-x86_64-unknown-linux-gnu" \
  MACOS_ROLLBACK_ARCHIVE="$TMP_ROOT/rollback-aarch64-apple-darwin.tar.gz" \
  LINUX_ROLLBACK_ARCHIVE="$TMP_ROOT/rollback-x86_64-unknown-linux-gnu.tar.gz" \
    bash "$FAKE_ROOT/scripts/ci/native-official-release-local.sh" 2>&1
)"
artifact_binding_status=$?
set -e
cp "$ARTIFACT_BINDING_BACKUP" "$ARTIFACT_BINDING_PROGRAM"
[[ "$artifact_binding_status" -ne 0 ]] \
  || { echo 'provider identity artifact mismatch was accepted by official release gate' >&2; exit 1; }
grep -F 'artifact_digest mismatch' <<<"$artifact_binding_output" >/dev/null \
  || { echo 'provider identity artifact binding diagnostic was missing' >&2; echo "$artifact_binding_output" >&2; exit 1; }
after_artifact_binding_log_lines="$(wc -l <"$LOG_PATH" | tr -d '[:space:]')"
[[ "$after_artifact_binding_log_lines" == "$before_artifact_binding_log_lines" ]] \
  || { echo 'provider identity artifact mismatch reached a release or smoke boundary' >&2; exit 1; }

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

provider_identity_path="$TMP_ROOT/artifact-aarch64-apple-darwin/review-evidence-identity.json"
provider_identity_backup="$TMP_ROOT/provider-identity-source-commit.json"
PROVIDER_IDENTITY_SMOKE_ROOT="$(mktemp -d /tmp/lsharp-native-official-provider-identity.XXXXXX)"
mv "$provider_identity_path" "$provider_identity_backup"
printf '%s\n' '{"source_commit":"0000000000000000000000000000000000000000"}' >"$provider_identity_path"
before_provider_identity_log_lines="$(wc -l <"$LOG_PATH" | tr -d '[:space:]')"
set +e
provider_identity_output="$(
  PATH="$PATH_PREFIX:$PATH" \
  VERSION="$VERSION" \
  SOURCE_COMMIT="$SOURCE_COMMIT" \
  DIST_DIR="$FAKE_ROOT/provider-identity-dist" \
  SMOKE_ROOT="$PROVIDER_IDENTITY_SMOKE_ROOT" \
  LSHARP_NATIVE_RELEASE_SMOKE_ROOT="$PROVIDER_IDENTITY_SMOKE_ROOT" \
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
provider_identity_status=$?
set -e
mv "$provider_identity_backup" "$provider_identity_path"
[[ "$provider_identity_status" -ne 0 ]] \
  || { echo 'provider identity source_commit mismatch was accepted' >&2; exit 1; }
grep -F 'review evidence identity source_commit mismatch' <<<"$provider_identity_output" >/dev/null
after_provider_identity_log_lines="$(wc -l <"$LOG_PATH" | tr -d '[:space:]')"
[[ "$after_provider_identity_log_lines" == "$before_provider_identity_log_lines" ]] \
  || { echo 'provider identity source_commit mismatch reached a release or smoke boundary' >&2; exit 1; }

identity_schema_path="$TMP_ROOT/artifact-aarch64-apple-darwin/review-evidence-identity.json"
identity_schema_backup="$TMP_ROOT/provider-identity-schema.json"
IDENTITY_SCHEMA_SMOKE_ROOT="$(mktemp -d /tmp/lsharp-native-official-provider-schema.XXXXXX)"
mv "$identity_schema_path" "$identity_schema_backup"
printf '{"source_commit":"%s"}\n' "$SOURCE_COMMIT" >"$identity_schema_path"
before_identity_schema_log_lines="$(wc -l <"$LOG_PATH" | tr -d '[:space:]')"
set +e
identity_schema_output="$(
  PATH="$PATH_PREFIX:$PATH" \
  VERSION="$VERSION" \
  SOURCE_COMMIT="$SOURCE_COMMIT" \
  DIST_DIR="$FAKE_ROOT/provider-schema-dist" \
  SMOKE_ROOT="$IDENTITY_SCHEMA_SMOKE_ROOT" \
  LSHARP_NATIVE_RELEASE_SMOKE_ROOT="$IDENTITY_SCHEMA_SMOKE_ROOT" \
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
identity_schema_status=$?
set -e
mv "$identity_schema_backup" "$identity_schema_path"
[[ "$identity_schema_status" -ne 0 ]] \
  || { echo 'provider identity schema mismatch was accepted' >&2; exit 1; }
grep -F 'review_evidence_identity fields must be exactly' <<<"$identity_schema_output" >/dev/null
after_identity_schema_log_lines="$(wc -l <"$LOG_PATH" | tr -d '[:space:]')"
[[ "$after_identity_schema_log_lines" == "$before_identity_schema_log_lines" ]] \
  || { echo 'provider identity schema mismatch reached a release or smoke boundary' >&2; exit 1; }

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
