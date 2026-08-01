#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PACKAGE="$ROOT/scripts/ci/package-native-stage0.sh"
RELEASE_PACKAGE="$ROOT/scripts/ci/package-native-stage0-release.sh"

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

assert_eq() {
  [[ "$1" == "$2" ]] || fail "expected '$1', got '$2'"
}

[[ -x "$PACKAGE" ]] || fail "native stage0 package builder is missing or not executable: $PACKAGE"
[[ -x "$RELEASE_PACKAGE" ]] || fail "native stage0 release package builder is missing or not executable: $RELEASE_PACKAGE"

TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/lsharp-native-stage0-release-package.XXXXXX")"
trap 'rm -rf "$TMP_ROOT"' EXIT

INPUT_DIR="$TMP_ROOT/input"
STAGE0_DIR="$TMP_ROOT/stage0"
DIST_DIR="$TMP_ROOT/dist"
EXTRACT_DIR="$TMP_ROOT/extract"
HOST_BIN="$TMP_ROOT/host-bin"
HOST_TOOL_LOG="$TMP_ROOT/host-tools.log"
TARGET="x86_64-unknown-linux-gnu"
VERSION="v0.0.0-test"
SOURCE_COMMIT="0000000000000000000000000000000000000000"
ARCHIVE_NAME="lsharp-stage0-${VERSION}-${TARGET}"
ARCHIVE_PATH="$DIST_DIR/${ARCHIVE_NAME}.tar.gz"
IDENTITY_PATH="$INPUT_DIR/review-evidence-identity.json"
TRUST_STORE_PATH="$INPUT_DIR/trust-store.json"
LIFECYCLE_PATH="$INPUT_DIR/review-lifecycle.jsonl"

mkdir -p "$INPUT_DIR" "$HOST_BIN"
: >"$HOST_TOOL_LOG"

cat >"$INPUT_DIR/compiler.native" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
exit 0
SH
chmod +x "$INPUT_DIR/compiler.native"

cat >"$INPUT_DIR/transport-driver" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
exit 0
SH
chmod +x "$INPUT_DIR/transport-driver"

cat >"$INPUT_DIR/materializer.py" <<'PY'
raise SystemExit(0)
PY

printf '%s\n' '{"keys":["release-key"]}' >"$TRUST_STORE_PATH"
printf '%s\n' '{"review_id":"review:stage0/r1","state":"active"}' >"$LIFECYCLE_PATH"
python3 - "$IDENTITY_PATH" "$TRUST_STORE_PATH" "$LIFECYCLE_PATH" "$SOURCE_COMMIT" <<'PY'
import hashlib
import json
import pathlib
import sys

identity_path, trust_store_path, lifecycle_path, source_commit = map(pathlib.Path, sys.argv[1:])
compiler_path = identity_path.parent / "compiler.native"
identity = {
    "subject_digest": "sha256:" + "c" * 64,
    "source_commit": str(source_commit),
    "artifact_digest": "sha256:" + hashlib.sha256(compiler_path.read_bytes()).hexdigest(),
    "trust_store_digest": "sha256:" + hashlib.sha256(trust_store_path.read_bytes()).hexdigest(),
    "lifecycle_digest": "sha256:" + hashlib.sha256(lifecycle_path.read_bytes()).hexdigest(),
    "now": "2026-08-15T00:00:00Z",
}
identity_path.write_text(json.dumps(identity, separators=(",", ":")) + "\n", encoding="utf-8")
PY

for host_tool in cargo rustc lsharp; do
  cat >"$HOST_BIN/$host_tool" <<'SH'
#!/usr/bin/env bash
printf 'host-tool|%s\n' "$(basename "$0")" >>"${NATIVE_STAGE0_RELEASE_TEST_LOG:?}"
exit 99
SH
  chmod +x "$HOST_BIN/$host_tool"
done

PATH="$HOST_BIN:$PATH" "$PACKAGE" \
  --target "$TARGET" \
  --source-commit "$SOURCE_COMMIT" \
  --compiler "$INPUT_DIR/compiler.native" \
  --transport-driver "$INPUT_DIR/transport-driver" \
  --materializer "$INPUT_DIR/materializer.py" \
  --output-dir "$STAGE0_DIR"

printf '%s\n' 'private trust-store snapshot must not ship' >"$STAGE0_DIR/review-trust-store.snapshot"
printf '%s\n' 'private lifecycle snapshot must not ship' >"$STAGE0_DIR/review-lifecycle.snapshot"

NATIVE_STAGE0_RELEASE_TEST_LOG="$HOST_TOOL_LOG" \
  PATH="$HOST_BIN:$PATH" \
  "$RELEASE_PACKAGE" \
    --target "$TARGET" \
    --version "$VERSION" \
    --stage0-dir "$STAGE0_DIR" \
    --source-commit "$SOURCE_COMMIT" \
    --review-evidence-identity "$IDENTITY_PATH" \
    --review-trust-store "$TRUST_STORE_PATH" \
    --review-lifecycle "$LIFECYCLE_PATH" \
    --output-dir "$DIST_DIR"

assert_eq "" "$(cat "$HOST_TOOL_LOG")"
[[ -s "$ARCHIVE_PATH" ]] || fail "native stage0 release archive was not created: $ARCHIVE_PATH"

archive_listing="$(tar -tzf "$ARCHIVE_PATH")"
for required in \
  "$ARCHIVE_NAME/manifest.json" \
  "$ARCHIVE_NAME/bin/compiler" \
  "$ARCHIVE_NAME/bin/transport-driver" \
  "$ARCHIVE_NAME/bin/materializer" \
  "$ARCHIVE_NAME/bin/materializer.py" \
  "$ARCHIVE_NAME/review-evidence-identity.json" \
  "$ARCHIVE_NAME/checksums.txt"; do
  grep -Fx "$required" <<<"$archive_listing" >/dev/null \
    || fail "archive payload is missing: $required"
done
! grep -E '(^|/)\._' <<<"$archive_listing" >/dev/null \
  || fail "archive contains macOS metadata files"
! grep -Fx "$ARCHIVE_NAME/review-trust-store.snapshot" <<<"$archive_listing" >/dev/null \
  || fail "archive leaked the raw review trust-store snapshot"
! grep -Fx "$ARCHIVE_NAME/review-lifecycle.snapshot" <<<"$archive_listing" >/dev/null \
  || fail "archive leaked the raw review lifecycle snapshot"

mkdir -p "$EXTRACT_DIR"
tar -xzf "$ARCHIVE_PATH" -C "$EXTRACT_DIR"
EXTRACTED_STAGE0="$EXTRACT_DIR/$ARCHIVE_NAME"
[[ -d "$EXTRACTED_STAGE0" ]] || fail "archive root is missing: $EXTRACTED_STAGE0"

python3 - "$EXTRACTED_STAGE0/manifest.json" "$TARGET" <<'PY'
import json
import os
import pathlib
import sys

manifest_path = pathlib.Path(sys.argv[1])
target = sys.argv[2]
manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
if manifest.get("kind") != "lsharp-native-selfhost-stage0":
    raise SystemExit(f"unexpected manifest kind: {manifest.get('kind')!r}")
if manifest.get("target") != target:
    raise SystemExit(f"unexpected manifest target: {manifest.get('target')!r}")
if manifest.get("source_commit") != "0000000000000000000000000000000000000000":
    raise SystemExit(f"unexpected manifest source commit: {manifest.get('source_commit')!r}")
identity = manifest.get("review_evidence_identity")
identity_path = manifest_path.parent / "review-evidence-identity.json"
if identity is None or identity != json.loads(identity_path.read_text(encoding="utf-8")):
    raise SystemExit("native stage0 release review evidence identity is missing or mismatched")
if list(identity) != [
    "subject_digest",
    "source_commit",
    "artifact_digest",
    "trust_store_digest",
    "lifecycle_digest",
    "now",
]:
    raise SystemExit("native stage0 release review evidence identity field order is not canonical")
for field in ("compiler", "transport_driver", "materializer"):
    value = manifest.get(field)
    path = pathlib.PurePosixPath(value or "")
    if not isinstance(value, str) or not value or path.is_absolute() or ".." in path.parts:
        raise SystemExit(f"unsafe manifest path: {field}={value!r}")
    candidate = manifest_path.parent.joinpath(*path.parts)
    if not candidate.is_file() or not os.access(candidate, os.X_OK):
        raise SystemExit(f"missing executable: {candidate}")
PY

"$ROOT/scripts/checksum.sh" "$EXTRACTED_STAGE0" >"$TMP_ROOT/actual-checksums.txt"
cmp -s "$EXTRACTED_STAGE0/checksums.txt" "$TMP_ROOT/actual-checksums.txt" \
  || fail "archive package checksums do not match payload"

EMBEDDED_STAGE0_DIR="$TMP_ROOT/embedded-identity-stage0"
EMBEDDED_DIST_DIR="$TMP_ROOT/embedded-identity-dist"
cp -pR "$STAGE0_DIR" "$EMBEDDED_STAGE0_DIR"
cp "$IDENTITY_PATH" "$EMBEDDED_STAGE0_DIR/review-evidence-identity.json"
set +e
embedded_identity_output="$(
  NATIVE_STAGE0_RELEASE_TEST_LOG="$HOST_TOOL_LOG" \
    PATH="$HOST_BIN:$PATH" \
    "$RELEASE_PACKAGE" \
      --target "$TARGET" \
      --version "${VERSION}-embedded-identity" \
      --stage0-dir "$EMBEDDED_STAGE0_DIR" \
      --source-commit "$SOURCE_COMMIT" \
      --output-dir "$EMBEDDED_DIST_DIR" 2>&1
)"
embedded_identity_status=$?
set -e
[[ "$embedded_identity_status" -ne 0 ]] \
  || fail "embedded review evidence identity was packaged without provider snapshots"
grep -F "embedded review evidence identity requires explicit provider snapshots" \
  <<<"$embedded_identity_output" >/dev/null \
  || fail "embedded identity rejection did not explain the provider snapshot requirement"
[[ ! -e "$EMBEDDED_DIST_DIR/lsharp-stage0-${VERSION}-embedded-identity-${TARGET}.tar.gz" ]] \
  || fail "embedded identity rejection left a release archive"

ARTIFACT_MISMATCH_DIST_DIR="$TMP_ROOT/artifact-mismatch-dist"
ARTIFACT_MISMATCH_IDENTITY_BACKUP="$TMP_ROOT/artifact-mismatch-identity.json"
cp "$IDENTITY_PATH" "$ARTIFACT_MISMATCH_IDENTITY_BACKUP"
python3 - "$IDENTITY_PATH" <<'PY'
import json
import pathlib
import sys

identity_path = pathlib.Path(sys.argv[1])
identity = json.loads(identity_path.read_text(encoding="utf-8"))
identity["artifact_digest"] = "sha256:" + "d" * 64
identity_path.write_text(json.dumps(identity, separators=(",", ":")) + "\n", encoding="utf-8")
PY
set +e
artifact_mismatch_output="$(
  NATIVE_STAGE0_RELEASE_TEST_LOG="$HOST_TOOL_LOG" \
    PATH="$HOST_BIN:$PATH" \
    "$RELEASE_PACKAGE" \
      --target "$TARGET" \
      --version "${VERSION}-artifact-mismatch" \
      --stage0-dir "$STAGE0_DIR" \
      --source-commit "$SOURCE_COMMIT" \
      --review-evidence-identity "$IDENTITY_PATH" \
      --review-trust-store "$TRUST_STORE_PATH" \
      --review-lifecycle "$LIFECYCLE_PATH" \
      --output-dir "$ARTIFACT_MISMATCH_DIST_DIR" 2>&1
)"
artifact_mismatch_status=$?
set -e
cp "$ARTIFACT_MISMATCH_IDENTITY_BACKUP" "$IDENTITY_PATH"
[[ "$artifact_mismatch_status" -ne 0 ]] \
  || fail "stage0 artifact digest mismatch was accepted"
grep -F "artifact_digest" <<<"$artifact_mismatch_output" >/dev/null \
  || fail "stage0 artifact digest mismatch did not expose the identity field"
[[ ! -e "$ARTIFACT_MISMATCH_DIST_DIR/lsharp-stage0-${VERSION}-artifact-mismatch-${TARGET}.tar.gz" ]] \
  || fail "stage0 artifact digest mismatch left a release archive"

EMBEDDED_CONFLICT_STAGE0="$TMP_ROOT/embedded-conflict-stage0"
EMBEDDED_CONFLICT_DIST_DIR="$TMP_ROOT/embedded-conflict-dist"
CONFLICT_IDENTITY_PATH="$TMP_ROOT/conflicting-identity.json"
cp -pR "$STAGE0_DIR" "$EMBEDDED_CONFLICT_STAGE0"
cp "$IDENTITY_PATH" "$EMBEDDED_CONFLICT_STAGE0/review-evidence-identity.json"
cp "$IDENTITY_PATH" "$CONFLICT_IDENTITY_PATH"
python3 - "$CONFLICT_IDENTITY_PATH" <<'PY'
import json
import pathlib
import sys

identity_path = pathlib.Path(sys.argv[1])
identity = json.loads(identity_path.read_text(encoding="utf-8"))
identity["subject_digest"] = "sha256:" + "e" * 64
identity_path.write_text(json.dumps(identity, separators=(",", ":")) + "\n", encoding="utf-8")
PY
set +e
embedded_conflict_output="$(
  NATIVE_STAGE0_RELEASE_TEST_LOG="$HOST_TOOL_LOG" \
    PATH="$HOST_BIN:$PATH" \
    "$RELEASE_PACKAGE" \
      --target "$TARGET" \
      --version "${VERSION}-embedded-conflict" \
      --stage0-dir "$EMBEDDED_CONFLICT_STAGE0" \
      --source-commit "$SOURCE_COMMIT" \
      --review-evidence-identity "$CONFLICT_IDENTITY_PATH" \
      --review-trust-store "$TRUST_STORE_PATH" \
      --review-lifecycle "$LIFECYCLE_PATH" \
      --output-dir "$EMBEDDED_CONFLICT_DIST_DIR" 2>&1
)"
embedded_conflict_status=$?
set -e
[[ "$embedded_conflict_status" -ne 0 ]] \
  || fail "embedded and explicit stage0 identities were silently replaced"
grep -F "embedded review evidence identity conflicts with explicit input" \
  <<<"$embedded_conflict_output" >/dev/null \
  || fail "embedded identity conflict did not expose a stable diagnostic"
[[ ! -e "$EMBEDDED_CONFLICT_DIST_DIR/lsharp-stage0-${VERSION}-embedded-conflict-${TARGET}.tar.gz" ]] \
  || fail "embedded identity conflict left a release archive"

RELATIVE_ROOT="$TMP_ROOT/relative-output"
mkdir -p "$RELATIVE_ROOT"
cp -pR "$STAGE0_DIR" "$RELATIVE_ROOT/stage0"
RELATIVE_ARCHIVE="$RELATIVE_ROOT/relative-dist/lsharp-stage0-${VERSION}-relative-${TARGET}.tar.gz"
(
  cd "$RELATIVE_ROOT"
  NATIVE_STAGE0_RELEASE_TEST_LOG="$HOST_TOOL_LOG" \
    PATH="$HOST_BIN:$PATH" \
    "$RELEASE_PACKAGE" \
      --target "$TARGET" \
      --version "$VERSION-relative" \
      --stage0-dir "$RELATIVE_ROOT/stage0" \
      --source-commit "$SOURCE_COMMIT" \
      --output-dir relative-dist
)
[[ -s "$RELATIVE_ARCHIVE" ]] \
  || fail "native stage0 release package did not support a relative output directory: $RELATIVE_ARCHIVE"

printf '%s\n' '{"keys":["tampered-key"]}' >"$TRUST_STORE_PATH"
TAMPER_DIST_DIR="$TMP_ROOT/tampered-dist"
set +e
tamper_output="$(
  NATIVE_STAGE0_RELEASE_TEST_LOG="$HOST_TOOL_LOG" \
    PATH="$HOST_BIN:$PATH" \
    "$RELEASE_PACKAGE" \
      --target "$TARGET" \
      --version "${VERSION}-tampered" \
      --stage0-dir "$STAGE0_DIR" \
      --source-commit "$SOURCE_COMMIT" \
      --review-evidence-identity "$IDENTITY_PATH" \
      --review-trust-store "$TRUST_STORE_PATH" \
      --review-lifecycle "$LIFECYCLE_PATH" \
      --output-dir "$TAMPER_DIST_DIR" 2>&1
)"
tamper_status=$?
set -e
[[ "$tamper_status" -ne 0 ]] || fail "tampered provider snapshot was accepted"
grep -F "trust_store_digest" <<<"$tamper_output" >/dev/null \
  || fail "tampered provider snapshot did not expose digest mismatch"

echo "native stage0 release package tests: OK"
