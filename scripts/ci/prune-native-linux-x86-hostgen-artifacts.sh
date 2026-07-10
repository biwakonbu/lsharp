#!/usr/bin/env bash
set -euo pipefail

if [[ "$#" -ne 5 ]]; then
  echo "usage: $0 <artifact-root> <current-artifact> <stage1-reuse-artifact> <stage2-reuse-artifact> <retention-count>" >&2
  exit 2
fi

ARTIFACT_ROOT="$1"
CURRENT_ARTIFACT="$2"
STAGE1_REUSE_ARTIFACT="$3"
STAGE2_REUSE_ARTIFACT="$4"
RETENTION_COUNT="$5"
DRY_RUN="${LSHARP_NATIVE_LINUX_X86_ARTIFACT_PRUNE_DRY_RUN:-0}"

python3 - \
  "${ARTIFACT_ROOT}" \
  "${CURRENT_ARTIFACT}" \
  "${STAGE1_REUSE_ARTIFACT}" \
  "${STAGE2_REUSE_ARTIFACT}" \
  "${RETENTION_COUNT}" \
  "${DRY_RUN}" <<'PY'
import pathlib
import shutil
import sys

artifact_root = pathlib.Path(sys.argv[1]).resolve()
protected_inputs = sys.argv[2:5]
try:
    retention_count = int(sys.argv[5])
except ValueError as error:
    raise SystemExit(f"retention count must be an integer: {sys.argv[5]}") from error
dry_run = sys.argv[6] == "1"

if retention_count < 0:
    raise SystemExit(f"retention count must be non-negative: {retention_count}")
if not artifact_root.exists():
    raise SystemExit(0)
if not artifact_root.is_dir():
    raise SystemExit(f"artifact root is not a directory: {artifact_root}")

protected = {
    pathlib.Path(path).resolve()
    for path in protected_inputs
    if path
}
candidates = sorted(
    (
        path
        for path in artifact_root.iterdir()
        if path.is_dir() and not path.is_symlink() and path.resolve() not in protected
    ),
    key=lambda path: (path.stat().st_mtime_ns, path.name),
    reverse=True,
)

for path in candidates[retention_count:]:
    if dry_run:
        print(f"would-remove {path}")
    else:
        shutil.rmtree(path)
        print(f"removed {path}")
PY
