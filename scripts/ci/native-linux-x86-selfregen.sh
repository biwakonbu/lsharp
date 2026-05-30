#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ARTIFACT_ID="${NATIVE_LINUX_X86_HOSTGEN_VM_ARTIFACT_ID:-${GITHUB_SHA:-local}}"
ARTIFACT_DIR_INPUT="${LSHARP_NATIVE_LINUX_X86_HOSTGEN_VM_ARTIFACT_DIR:-ci-artifacts/native-linux-x86-hostgen-vm/${ARTIFACT_ID}}"
DEFAULT_REJECT_DIRTY_STAGE1_SEED=1

if [[ "${ARTIFACT_DIR_INPUT}" = /* ]]; then
  if [[ "${ARTIFACT_DIR_INPUT}" != "${ROOT_DIR}"/* ]]; then
    echo "ERROR: LSHARP_NATIVE_LINUX_X86_HOSTGEN_VM_ARTIFACT_DIR must be under repository root: ${ARTIFACT_DIR_INPUT}" >&2
    exit 1
  fi
  ARTIFACT_DIR="${ARTIFACT_DIR_INPUT}"
else
  ARTIFACT_DIR="${ROOT_DIR}/${ARTIFACT_DIR_INPUT}"
fi

cd "${ROOT_DIR}"

HOST_OS="$(uname -s)"
HOST_ARCH="$(uname -m)"
if [[ "${HOST_OS}" != "Linux" || "${HOST_ARCH}" != "x86_64" ]]; then
  echo "ERROR: native-linux-x86-selfregen.sh requires Linux/x86_64; got ${HOST_OS}/${HOST_ARCH}" >&2
  exit 1
fi

if ! command -v limactl >/dev/null 2>&1; then
  echo "ERROR: limactl is required for the Linux x86_64 selfregen gate" >&2
  exit 1
fi

echo "=== native Linux x86_64 actual self-regeneration gate ==="
echo "artifact id: ${ARTIFACT_ID}"
echo "artifact dir: ${ARTIFACT_DIR}"

NATIVE_LINUX_X86_HOSTGEN_VM_ARTIFACT_ID="${ARTIFACT_ID}" \
LSHARP_NATIVE_LINUX_X86_REJECT_DIRTY_STAGE1_SEED="${LSHARP_NATIVE_LINUX_X86_REJECT_DIRTY_STAGE1_SEED:-${DEFAULT_REJECT_DIRTY_STAGE1_SEED}}" \
LSHARP_NATIVE_LINUX_X86_ACTUAL_TIMEOUT="${LSHARP_NATIVE_LINUX_X86_ACTUAL_TIMEOUT:-1200}" \
  bash scripts/ci/native-linux-x86-hostgen-vm-exec.sh

SUMMARY_PATH="${ARTIFACT_DIR}/actual-selfregen-summary.json"
if [[ ! -s "${SUMMARY_PATH}" ]]; then
  echo "ERROR: actual-selfregen-summary.json was not generated: ${SUMMARY_PATH}" >&2
  exit 1
fi

python3 - "${SUMMARY_PATH}" <<'PY'
import json
import pathlib
import sys

summary_path = pathlib.Path(sys.argv[1])
summary = json.loads(summary_path.read_text())
expected_status = {"status": "pass"}
required = [
    "target",
    "host_os",
    "host_arch",
    "status",
    "stage2_stdout_sha256",
    "stage3_stdout_sha256",
    "stage2_code_len",
    "stage3_code_len",
]
missing = [key for key in required if key not in summary]
if missing:
    raise SystemExit(f"missing selfregen summary fields: {missing}")
if summary.get("target") != "x86_64-unknown-linux-gnu":
    raise SystemExit(f"unexpected target: {summary.get('target')}")
if summary.get("host_os") != "Linux" or summary.get("host_arch") != "x86_64":
    raise SystemExit(f"unexpected host: {summary.get('host_os')}/{summary.get('host_arch')}")
if summary.get("status") != expected_status["status"]:
    raise SystemExit(f"selfregen did not pass: {summary.get('status')}")
if summary["stage2_stdout_sha256"] != summary["stage3_stdout_sha256"]:
    raise SystemExit("stage2/stage3 stdout hashes differ")
if int(summary["stage2_code_len"]) <= 0 or int(summary["stage3_code_len"]) <= 0:
    raise SystemExit("stage2/stage3 code lengths must be positive")
if int(summary["stage2_code_len"]) != int(summary["stage3_code_len"]):
    raise SystemExit("stage2/stage3 code lengths differ")
print("native-linux-x86-selfregen: summary validated")
PY
