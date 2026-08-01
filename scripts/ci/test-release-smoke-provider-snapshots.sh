#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/lsharp-release-smoke-snapshot.XXXXXX")"
trap 'rm -rf "$TMP_ROOT"' EXIT

SOURCE_COMMIT="0123456789abcdef0123456789abcdef01234567"
VERSION="v0.0.0-test"
TARGET="x86_64-unknown-linux-gnu"
STABLE_NAME="lsharp-${VERSION}-${TARGET}"
ROLLBACK_NAME="${STABLE_NAME}-host-launcher"
STABLE_ROOT="$TMP_ROOT/$STABLE_NAME"
ROLLBACK_ROOT="$TMP_ROOT/$ROLLBACK_NAME"
TRUST_STORE="$TMP_ROOT/trust-store.json"
LIFECYCLE="$TMP_ROOT/review-lifecycle.jsonl"

mkdir -p "$STABLE_ROOT" "$ROLLBACK_ROOT"
printf '%s\n' '{"keys":["release-key"]}' >"$TRUST_STORE"
printf '%s\n' '{"review_id":"review:release-smoke/r1","state":"active"}' >"$LIFECYCLE"

cat >"$STABLE_ROOT/program.native" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
case "${1:-}" in
  --version) printf '%s\n' 'lsharp 0.0.0-test' ;;
  --help) printf '%s\n' 'Usage: lsharp <command> [options]' ;;
  *) printf 'unsupported command: %s\n' "${1:-}" >&2; exit 1 ;;
esac
SH
chmod +x "$STABLE_ROOT/program.native"
cp "$STABLE_ROOT/program.native" "$STABLE_ROOT/lsharp"

cat >"$ROLLBACK_ROOT/lsharp" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
cmd="${1:-}"
case "$cmd" in
  --version) printf '%s\n' 'lsharp 0.0.0-test' ;;
  check) printf '%s\n' 'type:Int' ;;
  fmt) cat "${2:?missing source path}" ;;
  test) printf '%s\n' 'examples:1 invariants:1 failures:0' ;;
  compile)
    out=""
    shift
    while [[ $# -gt 0 ]]; do
      if [[ "$1" == "-o" ]]; then out="$2"; shift 2; else shift; fi
    done
    if [[ -n "$out" ]]; then printf '\0asm' >"$out"; else printf '%s\n' 'wasm-size:42'; fi
    ;;
  doc)
    json=0
    out=""
    shift
    while [[ $# -gt 0 ]]; do
      case "$1" in
        --json) json=1; shift ;;
        -o|--output) out="$2"; shift 2 ;;
        *) shift ;;
      esac
    done
    if [[ "$json" == "1" ]]; then payload='{"package":"fixture"}'; else payload='<html><body>fixture doc</body></html>'; fi
    if [[ -n "$out" ]]; then printf '%s\n' "$payload" >"$out"; else printf '%s\n' "$payload"; fi
    ;;
  *) printf 'unsupported command: %s\n' "$cmd" >&2; exit 1 ;;
esac
SH
chmod +x "$ROLLBACK_ROOT/lsharp"
cp "$ROLLBACK_ROOT/lsharp" "$ROLLBACK_ROOT/lsharp-lsp"
printf '\0asm' >"$ROLLBACK_ROOT/lsharp.component.wasm"
printf '%s\n' '# fixture' >"$STABLE_ROOT/README.md"
printf '%s\n' 'fixture license' >"$STABLE_ROOT/LICENSE"
printf '%s\n' '# rollback fixture' >"$ROLLBACK_ROOT/README.md"
printf '%s\n' 'rollback fixture license' >"$ROLLBACK_ROOT/LICENSE"
printf '%s\n' "{\"schema_version\":1,\"archive_kind\":\"rollback compatibility\",\"target\":\"$TARGET\",\"version\":\"$VERSION\",\"source_commit\":\"$SOURCE_COMMIT\",\"entry_binary\":\"lsharp\",\"lsp_binary\":\"lsharp-lsp\",\"component\":\"lsharp.component.wasm\"}" >"$ROLLBACK_ROOT/manifest.json"

program_sha256="$(sha256sum "$STABLE_ROOT/program.native" | awk '{print $1}')"
python3 - "$STABLE_ROOT" "$ROLLBACK_NAME.tar.gz" "$SOURCE_COMMIT" "$TARGET" "$VERSION" "$program_sha256" "$TRUST_STORE" "$LIFECYCLE" <<'PY'
import hashlib
import json
import pathlib
import sys

stable, rollback_name, source_commit, target, version, program_sha256, trust_store, lifecycle = map(pathlib.Path, sys.argv[1:])
identity = {
    "subject_digest": "sha256:" + "c" * 64,
    "source_commit": str(source_commit),
    "artifact_digest": "sha256:" + hashlib.sha256((stable / "program.native").read_bytes()).hexdigest(),
    "trust_store_digest": "sha256:" + hashlib.sha256(pathlib.Path(trust_store).read_bytes()).hexdigest(),
    "lifecycle_digest": "sha256:" + hashlib.sha256(pathlib.Path(lifecycle).read_bytes()).hexdigest(),
    "now": "2026-08-15T00:00:00Z",
}
(stable / "review-evidence-identity.json").write_text(json.dumps(identity, separators=(",", ":")) + "\n")
(stable / "native-program-manifest.json").write_text(json.dumps({
    "status": "pass",
    "target": str(target),
    "scope": "Linux x86_64 App.Cli native release bundle",
    "entry_module": "App.Cli",
    "source": "src/App/Cli.ls",
    "source_commit": str(source_commit),
    "selfhost_fixed_point": True,
    "program_sha256": str(program_sha256),
}) + "\n")
(stable / "manifest.json").write_text(json.dumps({
    "schema_version": 1,
    "archive_kind": "native-only official archive",
    "target": str(target),
    "version": str(version),
    "source_commit": str(source_commit),
    "entry_binary": "program.native",
    "rollback_anchor": {"kind": "rollback compatibility", "asset": str(rollback_name), "rollback_sha256": "pending"},
    "native_program_input": {"manifest": "native-program-manifest.json", "input_sha256": str(program_sha256)},
    "smoke": {"kind": "native-only release smoke", "binary": "program.native"},
    "review_evidence_identity": identity,
}) + "\n")
PY

bash "$ROOT/scripts/checksum.sh" "$ROLLBACK_ROOT" >"$ROLLBACK_ROOT/checksums.txt"
tar -czf "$TMP_ROOT/$ROLLBACK_NAME.tar.gz" -C "$TMP_ROOT" "$ROLLBACK_NAME"
rollback_sha256="$(sha256sum "$TMP_ROOT/$ROLLBACK_NAME.tar.gz" | awk '{print $1}')"
python3 - "$STABLE_ROOT/manifest.json" "$rollback_sha256" <<'PY'
import json
import pathlib
import sys

manifest_path = pathlib.Path(sys.argv[1])
manifest = json.loads(manifest_path.read_text())
manifest["rollback_anchor"]["rollback_sha256"] = sys.argv[2]
manifest_path.write_text(json.dumps(manifest) + "\n")
PY
bash "$ROOT/scripts/checksum.sh" "$STABLE_ROOT" >"$STABLE_ROOT/checksums.txt"
tar -czf "$TMP_ROOT/$STABLE_NAME.tar.gz" -C "$TMP_ROOT" "$STABLE_NAME"

set +e
preflight_output="$(
  RELEASE_REVIEW_TRUST_STORE="$TRUST_STORE" \
    RELEASE_REVIEW_LIFECYCLE="" \
    WORK_DIR="$TMP_ROOT/preflight-work" \
    bash "$ROOT/scripts/ci/release-smoke.sh" "$TMP_ROOT/missing-release.tar.gz" 2>&1
)"
preflight_status=$?
set -e
[[ "$preflight_status" -ne 0 ]] || { echo "incomplete provider input was accepted for a missing archive" >&2; exit 1; }
grep -F "must be supplied together" <<<"$preflight_output" >/dev/null \
  || { echo "provider preflight did not precede archive lookup" >&2; echo "$preflight_output" >&2; exit 1; }
[[ ! -e "$TMP_ROOT/preflight-work" ]] \
  || { echo "provider preflight created release smoke work before archive access" >&2; exit 1; }

RELEASE_REVIEW_TRUST_STORE="$TRUST_STORE" \
RELEASE_REVIEW_LIFECYCLE="$LIFECYCLE" \
  WORK_DIR="$TMP_ROOT/smoke-work" \
  bash "$ROOT/scripts/ci/release-smoke.sh" "$TMP_ROOT/$STABLE_NAME.tar.gz" "$TMP_ROOT/$ROLLBACK_NAME.tar.gz" >/dev/null

set +e
partial_output="$(
  RELEASE_REVIEW_TRUST_STORE="$TRUST_STORE" \
    RELEASE_REVIEW_LIFECYCLE="" \
    WORK_DIR="$TMP_ROOT/partial-work" \
    bash "$ROOT/scripts/ci/release-smoke.sh" "$TMP_ROOT/$STABLE_NAME.tar.gz" "$TMP_ROOT/$ROLLBACK_NAME.tar.gz" 2>&1
)"
partial_status=$?
set -e
[[ "$partial_status" -ne 0 ]] || { echo "partial snapshot input was accepted" >&2; exit 1; }
grep -F "must be supplied together" <<<"$partial_output" >/dev/null \
  || { echo "partial snapshot input did not expose all-or-none diagnostic" >&2; exit 1; }

printf '%s\n' '{"keys":["tampered-key"]}' >"$TRUST_STORE"
set +e
tamper_output="$(
  RELEASE_REVIEW_TRUST_STORE="$TRUST_STORE" \
    RELEASE_REVIEW_LIFECYCLE="$LIFECYCLE" \
    WORK_DIR="$TMP_ROOT/tampered-work" \
    bash "$ROOT/scripts/ci/release-smoke.sh" "$TMP_ROOT/$STABLE_NAME.tar.gz" "$TMP_ROOT/$ROLLBACK_NAME.tar.gz" 2>&1
)"
tamper_status=$?
set -e
[[ "$tamper_status" -ne 0 ]] || { echo "tampered snapshot was accepted" >&2; exit 1; }
grep -F "trust_store_digest" <<<"$tamper_output" >/dev/null \
  || { echo "tampered snapshot did not expose digest mismatch" >&2; exit 1; }

BAD_ROLLBACK_NAME="${ROLLBACK_NAME}-bad-manifest"
BAD_ROLLBACK_ROOT="$TMP_ROOT/$BAD_ROLLBACK_NAME"
BAD_ROLLBACK_ARCHIVE="$TMP_ROOT/$BAD_ROLLBACK_NAME.tar.gz"
cp -R "$ROLLBACK_ROOT" "$BAD_ROLLBACK_ROOT"
python3 - "$BAD_ROLLBACK_ROOT/manifest.json" <<'PY'
import json
import pathlib
import sys

manifest_path = pathlib.Path(sys.argv[1])
manifest = json.loads(manifest_path.read_text())
manifest["entry_binary"] = "unexpected-entry"
manifest_path.write_text(json.dumps(manifest) + "\n")
PY
bash "$ROOT/scripts/checksum.sh" "$BAD_ROLLBACK_ROOT" >"$BAD_ROLLBACK_ROOT/checksums.txt"
tar -czf "$BAD_ROLLBACK_ARCHIVE" -C "$TMP_ROOT" "$BAD_ROLLBACK_NAME"
bad_rollback_sha256="$(sha256sum "$BAD_ROLLBACK_ARCHIVE" | awk '{print $1}')"
python3 - "$STABLE_ROOT/manifest.json" "$BAD_ROLLBACK_NAME.tar.gz" "$bad_rollback_sha256" <<'PY'
import json
import pathlib
import sys

manifest_path = pathlib.Path(sys.argv[1])
manifest = json.loads(manifest_path.read_text())
manifest["rollback_anchor"]["asset"] = sys.argv[2]
manifest["rollback_anchor"]["rollback_sha256"] = sys.argv[3]
manifest_path.write_text(json.dumps(manifest) + "\n")
PY
bash "$ROOT/scripts/checksum.sh" "$STABLE_ROOT" >"$STABLE_ROOT/checksums.txt"
tar -czf "$TMP_ROOT/$STABLE_NAME.tar.gz" -C "$TMP_ROOT" "$STABLE_NAME"

printf '%s\n' '{"keys":["release-key"]}' >"$TRUST_STORE"
set +e
rollback_manifest_output="$(
  RELEASE_REVIEW_TRUST_STORE="$TRUST_STORE" \
    RELEASE_REVIEW_LIFECYCLE="$LIFECYCLE" \
    WORK_DIR="$TMP_ROOT/rollback-manifest-work" \
    bash "$ROOT/scripts/ci/release-smoke.sh" "$TMP_ROOT/$STABLE_NAME.tar.gz" "$BAD_ROLLBACK_ARCHIVE" 2>&1
)"
rollback_manifest_status=$?
set -e
[[ "$rollback_manifest_status" -ne 0 ]] || { echo "rollback manifest payload mismatch was accepted" >&2; exit 1; }
grep -F "entry_binary" <<<"$rollback_manifest_output" >/dev/null \
  || { echo "rollback manifest payload mismatch did not expose entry_binary diagnostic" >&2; exit 1; }

python3 - "$STABLE_ROOT/manifest.json" "$ROLLBACK_NAME.tar.gz" "$rollback_sha256" <<'PY'
import json
import pathlib
import sys

manifest_path = pathlib.Path(sys.argv[1])
manifest = json.loads(manifest_path.read_text())
manifest["rollback_anchor"] = {
    "kind": "unexpected rollback kind",
    "asset": sys.argv[2],
    "rollback_sha256": sys.argv[3],
}
manifest_path.write_text(json.dumps(manifest) + "\n")
PY
bash "$ROOT/scripts/checksum.sh" "$STABLE_ROOT" >"$STABLE_ROOT/checksums.txt"
tar -czf "$TMP_ROOT/$STABLE_NAME.tar.gz" -C "$TMP_ROOT" "$STABLE_NAME"

set +e
rollback_anchor_output="$(
  RELEASE_REVIEW_TRUST_STORE="$TRUST_STORE" \
    RELEASE_REVIEW_LIFECYCLE="$LIFECYCLE" \
    WORK_DIR="$TMP_ROOT/rollback-anchor-work" \
    bash "$ROOT/scripts/ci/release-smoke.sh" "$TMP_ROOT/$STABLE_NAME.tar.gz" "$TMP_ROOT/$ROLLBACK_NAME.tar.gz" 2>&1
)"
rollback_anchor_status=$?
set -e
[[ "$rollback_anchor_status" -ne 0 ]] || { echo "rollback anchor kind mismatch was accepted" >&2; exit 1; }
grep -F "anchor kind" <<<"$rollback_anchor_output" >/dev/null \
  || { echo "rollback anchor kind mismatch did not expose diagnostic" >&2; exit 1; }

python3 - "$STABLE_ROOT/manifest.json" "$ROLLBACK_NAME.tar.gz" "$rollback_sha256" <<'PY'
import json
import pathlib
import sys

manifest_path = pathlib.Path(sys.argv[1])
manifest = json.loads(manifest_path.read_text())
manifest["rollback_anchor"] = {
    "kind": "rollback compatibility",
    "asset": sys.argv[2],
    "rollback_sha256": sys.argv[3],
}
manifest_path.write_text(json.dumps(manifest) + "\n")
PY
bash "$ROOT/scripts/checksum.sh" "$STABLE_ROOT" >"$STABLE_ROOT/checksums.txt"

CHECKSUM_OUTSIDE="$TMP_ROOT/outside-checksum-target.txt"
printf '%s\n' 'outside archive root' >"$CHECKSUM_OUTSIDE"
outside_sha256="$(sha256sum "$CHECKSUM_OUTSIDE" | awk '{print $1}')"
printf '%s  ../../../outside-checksum-target.txt\n' "$outside_sha256" >>"$STABLE_ROOT/checksums.txt"
tar -czf "$TMP_ROOT/$STABLE_NAME.tar.gz" -C "$TMP_ROOT" "$STABLE_NAME"

set +e
checksum_path_output="$(
  RELEASE_REVIEW_TRUST_STORE="$TRUST_STORE" \
    RELEASE_REVIEW_LIFECYCLE="$LIFECYCLE" \
    WORK_DIR="$TMP_ROOT/checksum-path-work" \
    bash "$ROOT/scripts/ci/release-smoke.sh" "$TMP_ROOT/$STABLE_NAME.tar.gz" "$TMP_ROOT/$ROLLBACK_NAME.tar.gz" 2>&1
)"
checksum_path_status=$?
set -e
[[ "$checksum_path_status" -ne 0 ]] || { echo "checksum target outside archive root was accepted" >&2; exit 1; }
grep -F "unsafe checksum target" <<<"$checksum_path_output" >/dev/null \
  || { echo "checksum target escape did not expose diagnostic" >&2; exit 1; }

BAD_CHECKSUM_ROLLBACK_NAME="${ROLLBACK_NAME}-missing-checksum"
BAD_CHECKSUM_ROLLBACK_ROOT="$TMP_ROOT/$BAD_CHECKSUM_ROLLBACK_NAME"
BAD_CHECKSUM_ROLLBACK_ARCHIVE="$TMP_ROOT/$BAD_CHECKSUM_ROLLBACK_NAME.tar.gz"
cp -R "$ROLLBACK_ROOT" "$BAD_CHECKSUM_ROLLBACK_ROOT"
bash "$ROOT/scripts/checksum.sh" "$BAD_CHECKSUM_ROLLBACK_ROOT" \
  | awk '$2 != "lsharp"' >"$TMP_ROOT/bad-rollback-checksums.txt"
mv "$TMP_ROOT/bad-rollback-checksums.txt" "$BAD_CHECKSUM_ROLLBACK_ROOT/checksums.txt"
tar -czf "$BAD_CHECKSUM_ROLLBACK_ARCHIVE" -C "$TMP_ROOT" "$BAD_CHECKSUM_ROLLBACK_NAME"
bad_checksum_rollback_sha256="$(sha256sum "$BAD_CHECKSUM_ROLLBACK_ARCHIVE" | awk '{print $1}')"
python3 - "$STABLE_ROOT/manifest.json" "$BAD_CHECKSUM_ROLLBACK_NAME.tar.gz" "$bad_checksum_rollback_sha256" <<'PY'
import json
import pathlib
import sys

manifest_path = pathlib.Path(sys.argv[1])
manifest = json.loads(manifest_path.read_text())
manifest["rollback_anchor"] = {
    "kind": "rollback compatibility",
    "asset": sys.argv[2],
    "rollback_sha256": sys.argv[3],
}
manifest_path.write_text(json.dumps(manifest) + "\n")
PY
bash "$ROOT/scripts/checksum.sh" "$STABLE_ROOT" >"$STABLE_ROOT/checksums.txt"
tar -czf "$TMP_ROOT/$STABLE_NAME.tar.gz" -C "$TMP_ROOT" "$STABLE_NAME"

set +e
rollback_checksum_output="$(
  RELEASE_REVIEW_TRUST_STORE="$TRUST_STORE" \
    RELEASE_REVIEW_LIFECYCLE="$LIFECYCLE" \
    WORK_DIR="$TMP_ROOT/rollback-checksum-work" \
    bash "$ROOT/scripts/ci/release-smoke.sh" "$TMP_ROOT/$STABLE_NAME.tar.gz" "$BAD_CHECKSUM_ROLLBACK_ARCHIVE" 2>&1
)"
rollback_checksum_status=$?
set -e
[[ "$rollback_checksum_status" -ne 0 ]] || { echo "rollback payload without checksum coverage was accepted" >&2; exit 1; }
grep -F "checksums.txt missing required entry: lsharp" <<<"$rollback_checksum_output" >/dev/null \
  || { echo "rollback checksum coverage mismatch did not expose diagnostic" >&2; exit 1; }

echo "release-smoke provider snapshot tests: OK"
