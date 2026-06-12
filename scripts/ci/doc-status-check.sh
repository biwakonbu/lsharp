#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
STATUS_FILE="$ROOT/.lsharp-doc-status"
DOC_SOURCE="$ROOT/examples/metadata.ls"
LSHARP_BIN="${LSHARP_BIN:-}"

if [[ ! -s "$STATUS_FILE" ]]; then
  echo "ERROR: .lsharp-doc-status is missing or empty" >&2
  exit 1
fi

if [[ ! -f "$DOC_SOURCE" ]]; then
  echo "ERROR: doc-status fixture missing: $DOC_SOURCE" >&2
  exit 1
fi

if [[ -n "$LSHARP_BIN" ]]; then
  LSHARP_CMD=("$LSHARP_BIN")
else
  LSHARP_CMD=(cargo run -q -p lsharp-driver --)
fi

trailers_output="$(mktemp)"
trap 'rm -f "$trailers_output"' EXIT

(
  cd "$ROOT"
  "${LSHARP_CMD[@]}" doc-check "$DOC_SOURCE" --emit-trailers > "$trailers_output"
)

if ! grep -q 'Doc-Review-Status: Passed' "$trailers_output"; then
  echo "ERROR: doc-check trailer status was not Passed" >&2
  cat "$trailers_output" >&2
  exit 1
fi

if ! grep -q 'Doc-Reviewed-By: docs-maintainers' "$trailers_output"; then
  echo "ERROR: doc-check trailer did not include docs-maintainers" >&2
  cat "$trailers_output" >&2
  exit 1
fi

echo "doc-status check ok: $DOC_SOURCE"
