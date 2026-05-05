#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: $0 <native-proxy-artifact-dir>" >&2
  echo "example: $0 ci-artifacts/native-proxy/local" >&2
  exit 2
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ARTIFACT_DIR_INPUT="$1"
if [[ "${ARTIFACT_DIR_INPUT}" = /* ]]; then
  ARTIFACT_DIR="${ARTIFACT_DIR_INPUT}"
else
  ARTIFACT_DIR="${ROOT_DIR}/${ARTIFACT_DIR_INPUT}"
fi

if [[ "${ARTIFACT_DIR}" != "${ROOT_DIR}"/* ]]; then
  echo "ERROR: experimental native-only RC artifact dir must be under repository root: ${ARTIFACT_DIR}" >&2
  exit 1
fi

echo "=== experimental native-only RC smoke ==="
echo "artifact dir: ${ARTIFACT_DIR}"

for required in manifest.json actual-stage23-gap.json; do
  if [[ ! -s "${ARTIFACT_DIR}/${required}" ]]; then
    echo "ERROR: missing ${required}" >&2
    exit 1
  fi
done

for stage in stage1-native stage2-native stage3-native; do
  stage_dir="${ARTIFACT_DIR}/${stage}"
  if [[ ! -d "${stage_dir}" ]]; then
    echo "ERROR: missing stage dir ${stage}" >&2
    exit 1
  fi
  for required in program.o runtime.o linker-response.txt program.native stdout.txt stderr.txt summary.json; do
    if [[ ! -s "${stage_dir}/${required}" ]]; then
      echo "ERROR: missing ${stage}/${required}" >&2
      exit 1
    fi
  done
  if [[ ! -x "${stage_dir}/program.native" ]]; then
    echo "ERROR: ${stage}/program.native is not executable" >&2
    exit 1
  fi
done

python3 - "${ARTIFACT_DIR}" <<'PY'
import json
import pathlib
import sys

root = pathlib.Path(sys.argv[1])
manifest = json.loads((root / "manifest.json").read_text())
stages = manifest.get("stages", {})
for label in ("stage1-native", "stage2-native", "stage3-native"):
    if label not in stages:
        raise SystemExit(f"ERROR: manifest.json missing {label}")
    summary = json.loads((root / label / "summary.json").read_text())
    for key in (
        "program_object_hash",
        "runtime_object_hash",
        "response_text_hash",
        "program_binary_hash",
        "exit_code",
    ):
        if key not in summary:
            raise SystemExit(f"ERROR: {label}/summary.json missing {key}")
    if summary["exit_code"] != 0:
        raise SystemExit(f"ERROR: {label} exit_code is not zero: {summary['exit_code']}")

if stages.get("stage2-native") != stages.get("stage3-native"):
    raise SystemExit("ERROR: stage2-native and stage3-native summaries differ")
PY

echo "experimental native-only RC smoke complete."
