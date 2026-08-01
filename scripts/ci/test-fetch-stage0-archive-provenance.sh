#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
FETCH="$ROOT/scripts/fetch-stage0.sh"
TARGET="x86_64-unknown-linux-gnu"
VERSION="v0.0.0-fetch-test"
ARCHIVE_ROOT="lsharp-stage0-${VERSION}-${TARGET}"
SOURCE_COMMIT="$(git rev-parse --verify HEAD)"
TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/lsharp-fetch-stage0-provenance.XXXXXX")"
trap 'rm -rf "$TMP_ROOT"' EXIT

ASSET_DIR="$TMP_ROOT/assets"
ARCHIVE_PATH="$ASSET_DIR/${ARCHIVE_ROOT}.tar.gz"
mkdir -p "$ASSET_DIR"

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

expect_reject_with_message() {
  local label="$1"
  local expected_message="$2"
  shift 2

  local output
  local exit_code
  set +e
  output="$($@ 2>&1)"
  exit_code=$?
  set -e
  [[ "$exit_code" -ne 0 ]] || fail "$label unexpectedly succeeded"
  grep -F -- "$expected_message" <<<"$output" >/dev/null \
    || fail "$label did not report: $expected_message"
}

write_archive() {
  local entry_type="$1"

  python3 - "$ARCHIVE_PATH" "$ARCHIVE_ROOT" "$SOURCE_COMMIT" "$entry_type" <<'PY'
import hashlib
import io
import json
import pathlib
import sys
import tarfile

archive_path, archive_root, source_commit, entry_type = sys.argv[1:]
files = {
    "manifest.json": json.dumps(
        {
            "kind": "lsharp-native-selfhost-stage0",
            "target": "x86_64-unknown-linux-gnu",
            "source_commit": source_commit,
            "compiler": "bin/compiler",
            "transport_driver": "bin/transport-driver",
            "materializer": "bin/materializer",
        },
        separators=(",", ":"),
    ).encode()
    + b"\n",
    "bin/compiler": b"#!/usr/bin/env bash\nexit 0\n",
    "bin/transport-driver": b"#!/usr/bin/env bash\nexit 0\n",
    "bin/materializer": b"#!/usr/bin/env bash\nexit 0\n",
}
checksums = "".join(
    f"{hashlib.sha256(payload).hexdigest()}  {name}\n"
    for name, payload in sorted(files.items())
)
files["checksums.txt"] = checksums.encode()

with tarfile.open(archive_path, "w:gz") as archive:
    root_info = tarfile.TarInfo(archive_root)
    root_info.type = tarfile.DIRTYPE
    root_info.mode = 0o755
    archive.addfile(root_info)
    bin_info = tarfile.TarInfo(f"{archive_root}/bin")
    bin_info.type = tarfile.DIRTYPE
    bin_info.mode = 0o755
    archive.addfile(bin_info)
    for name, payload in files.items():
        info = tarfile.TarInfo(f"{archive_root}/{name}")
        info.size = len(payload)
        info.mode = 0o755 if name.startswith("bin/") else 0o644
        archive.addfile(info, io.BytesIO(payload))
    if entry_type == "unknown":
        info = tarfile.TarInfo(f"{archive_root}/unsafe-unknown")
        info.type = b"?"
        info.mode = 0o644
        archive.addfile(info)

outer_checksum = hashlib.sha256(pathlib.Path(archive_path).read_bytes()).hexdigest()
pathlib.Path(archive_path).with_name("checksums.txt").write_text(
    f"{outer_checksum}  {pathlib.Path(archive_path).name}\n",
    encoding="utf-8",
)
PY
}

run_fetch() {
  local destination="$1"
  STAGE0_VERSION="$VERSION" \
    STAGE0_TARGET="$TARGET" \
    STAGE0_DIR="$destination" \
    STAGE0_RELEASE_BASE_URL="file://$ASSET_DIR" \
    bash "$FETCH"
}

[[ -x "$FETCH" ]] || fail "fetch-stage0.sh is missing or not executable: $FETCH"

write_archive regular
run_fetch "$TMP_ROOT/valid-stage0"
[[ -f "$TMP_ROOT/valid-stage0/manifest.json" ]] \
  || fail "valid fetched stage0 package was not installed"

write_archive unknown
expect_reject_with_message "unknown archive entry type" 'unsafe native stage0 archive entry type: lsharp-stage0-v0.0.0-fetch-test-x86_64-unknown-linux-gnu/unsafe-unknown' \
  run_fetch "$TMP_ROOT/unknown-stage0"
[[ ! -e "$TMP_ROOT/unknown-stage0" && ! -L "$TMP_ROOT/unknown-stage0" ]] \
  || fail "unsafe unknown archive entry created an installed stage0 directory"

echo "fetch-stage0 archive provenance tests: OK"
