#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
FETCH_SCRIPT="${ROOT_DIR}/scripts/fetch-stage0.sh"
CHECKSUM_SCRIPT="${ROOT_DIR}/scripts/checksum.sh"
SOURCE_COMMIT="$(git rev-parse HEAD)"
VERSION="v0.0.0-atomic-install-test"
TARGET="x86_64-unknown-linux-gnu"
ARCHIVE_ROOT_NAME="lsharp-stage0-${VERSION}-${TARGET}"
TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/lsharp-fetch-stage0-atomic.XXXXXX")"
RELEASE_DIR="${TMP_ROOT}/release"
ARCHIVE_ROOT="${RELEASE_DIR}/${ARCHIVE_ROOT_NAME}"
STAGE0_DIR="${TMP_ROOT}/stage0"
HOST_BIN="${TMP_ROOT}/host-bin"
MOVE_FAILURE_MARKER="${TMP_ROOT}/move-failure-injected"
RESTORE_FAILURE_MARKER="${TMP_ROOT}/restore-failure-injected"

cleanup() {
  rm -rf "${TMP_ROOT}"
}
trap cleanup EXIT

mkdir -p "${ARCHIVE_ROOT}/bin" "${HOST_BIN}" "${STAGE0_DIR}"
for executable in compiler transport-driver materializer; do
  printf '%s\n' '#!/usr/bin/env bash' 'exit 0' >"${ARCHIVE_ROOT}/bin/${executable}"
  chmod 0755 "${ARCHIVE_ROOT}/bin/${executable}"
done
printf '%s\n' '#!/usr/bin/env python3' >"${ARCHIVE_ROOT}/bin/materializer.py"
cat >"${ARCHIVE_ROOT}/manifest.json" <<JSON
{
  "kind": "lsharp-native-selfhost-stage0",
  "target": "${TARGET}",
  "source_commit": "${SOURCE_COMMIT}",
  "compiler": "bin/compiler",
  "transport_driver": "bin/transport-driver",
  "materializer": "bin/materializer"
}
JSON

bash "${CHECKSUM_SCRIPT}" "${ARCHIVE_ROOT}" >"${ARCHIVE_ROOT}/checksums.txt"
COPYFILE_DISABLE=1 tar -czf "${RELEASE_DIR}/${ARCHIVE_ROOT_NAME}.tar.gz" \
  -C "${RELEASE_DIR}" "${ARCHIVE_ROOT_NAME}"
bash "${CHECKSUM_SCRIPT}" "${RELEASE_DIR}" >"${RELEASE_DIR}/checksums.txt"

SUCCESS_STAGE0="${TMP_ROOT}/success-stage0"
STAGE0_RELEASE_BASE_URL="file://${RELEASE_DIR}" \
  STAGE0_VERSION="${VERSION}" \
  STAGE0_TARGET="${TARGET}" \
  STAGE0_DIR="${SUCCESS_STAGE0}" \
  bash "${FETCH_SCRIPT}" >"${TMP_ROOT}/success.stdout" 2>"${TMP_ROOT}/success.stderr"
[[ -s "${SUCCESS_STAGE0}/manifest.json" ]] || {
  echo "fetch-stage0 did not install a valid package" >&2
  exit 1
}

printf '%s\n' 'keep existing stage0' >"${STAGE0_DIR}/keep.txt"
cat >"${HOST_BIN}/mv" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${2:-}" == "${LSHARP_FETCH_STAGE0_FAIL_DEST:?}" && ! -e "${LSHARP_FETCH_STAGE0_MOVE_FAILURE_MARKER:?}" ]]; then
  : >"${LSHARP_FETCH_STAGE0_MOVE_FAILURE_MARKER}"
  printf 'injected final install move failure\n' >&2
  exit 77
fi
exec /bin/mv "$@"
SH
chmod 0755 "${HOST_BIN}/mv"

set +e
PATH="${HOST_BIN}:${PATH}" \
  STAGE0_RELEASE_BASE_URL="file://${RELEASE_DIR}" \
  STAGE0_VERSION="${VERSION}" \
  STAGE0_TARGET="${TARGET}" \
  STAGE0_DIR="${STAGE0_DIR}" \
  LSHARP_FETCH_STAGE0_FAIL_DEST="${STAGE0_DIR}" \
  LSHARP_FETCH_STAGE0_MOVE_FAILURE_MARKER="${MOVE_FAILURE_MARKER}" \
  bash "${FETCH_SCRIPT}" >"${TMP_ROOT}/fetch.stdout" 2>"${TMP_ROOT}/fetch.stderr"
fetch_status=$?
set -e

[[ "${fetch_status}" -ne 0 ]] || {
  echo "fetch-stage0 unexpectedly succeeded during injected final install failure" >&2
  exit 1
}
grep -F 'injected final install move failure' "${TMP_ROOT}/fetch.stderr" >/dev/null \
  || { echo "fetch-stage0 did not report the injected install failure" >&2; cat "${TMP_ROOT}/fetch.stderr" >&2; exit 1; }
[[ "$(<"${STAGE0_DIR}/keep.txt")" == 'keep existing stage0' ]] \
  || { echo "fetch-stage0 did not preserve the previous stage0 after install failure" >&2; exit 1; }
[[ ! -e "${STAGE0_DIR}/manifest.json" ]] \
  || { echo "fetch-stage0 partially installed a new stage0 after install failure" >&2; exit 1; }
! compgen -G "${TMP_ROOT}/.stage0.previous.*" >/dev/null \
  || { echo "fetch-stage0 left a previous stage0 backup after install failure" >&2; exit 1; }

cat >"${HOST_BIN}/mv" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${2:-}" == "${LSHARP_FETCH_STAGE0_FAIL_DEST:?}" ]]; then
  if [[ ! -e "${LSHARP_FETCH_STAGE0_FINAL_FAILURE_MARKER:?}" ]]; then
    : >"${LSHARP_FETCH_STAGE0_FINAL_FAILURE_MARKER}"
    printf 'injected final install move failure before restore failure\n' >&2
    exit 77
  fi
  : >"${LSHARP_FETCH_STAGE0_RESTORE_FAILURE_MARKER:?}"
  printf 'injected rollback restore move failure\n' >&2
  exit 78
fi
exec /bin/mv "$@"
SH
chmod 0755 "${HOST_BIN}/mv"
rm -f "${MOVE_FAILURE_MARKER}"

set +e
PATH="${HOST_BIN}:${PATH}" \
  STAGE0_RELEASE_BASE_URL="file://${RELEASE_DIR}" \
  STAGE0_VERSION="${VERSION}" \
  STAGE0_TARGET="${TARGET}" \
  STAGE0_DIR="${STAGE0_DIR}" \
  LSHARP_FETCH_STAGE0_FAIL_DEST="${STAGE0_DIR}" \
  LSHARP_FETCH_STAGE0_FINAL_FAILURE_MARKER="${MOVE_FAILURE_MARKER}" \
  LSHARP_FETCH_STAGE0_RESTORE_FAILURE_MARKER="${RESTORE_FAILURE_MARKER}" \
  bash "${FETCH_SCRIPT}" >"${TMP_ROOT}/restore-failure.stdout" 2>"${TMP_ROOT}/restore-failure.stderr"
restore_failure_status=$?
set -e

[[ "${restore_failure_status}" -ne 0 ]] || {
  echo "fetch-stage0 unexpectedly succeeded during injected rollback restore failure" >&2
  exit 1
}
grep -F 'injected rollback restore move failure' "${TMP_ROOT}/restore-failure.stderr" >/dev/null \
  || { echo "fetch-stage0 did not report the injected rollback restore failure" >&2; cat "${TMP_ROOT}/restore-failure.stderr" >&2; exit 1; }
previous_stage0_contents="$(cat "${STAGE0_DIR}/keep.txt" 2>/dev/null || true)"
[[ "${previous_stage0_contents}" == 'keep existing stage0' ]] || {
  echo "fetch-stage0 did not keep the previous stage0 after rollback restore failure" >&2
  exit 1
}
! compgen -G "${TMP_ROOT}/.stage0.previous.*" >/dev/null || {
  echo "fetch-stage0 left a hidden previous stage0 after rollback restore failure" >&2
  exit 1
}

TAMPERED_BUILD="${TMP_ROOT}/tampered-build"
TAMPERED_RELEASE="${TMP_ROOT}/tampered-release"
mkdir -p "${TAMPERED_BUILD}" "${TAMPERED_RELEASE}"
tar -xzf "${RELEASE_DIR}/${ARCHIVE_ROOT_NAME}.tar.gz" -C "${TAMPERED_BUILD}"
printf '%s\n' 'unregistered payload' >"${TAMPERED_BUILD}/${ARCHIVE_ROOT_NAME}/bin/unlisted"
COPYFILE_DISABLE=1 tar -czf "${TAMPERED_RELEASE}/${ARCHIVE_ROOT_NAME}.tar.gz" \
  -C "${TAMPERED_BUILD}" "${ARCHIVE_ROOT_NAME}"
bash "${CHECKSUM_SCRIPT}" "${TAMPERED_RELEASE}" >"${TAMPERED_RELEASE}/checksums.txt"

set +e
STAGE0_RELEASE_BASE_URL="file://${TAMPERED_RELEASE}" \
  STAGE0_VERSION="${VERSION}" \
  STAGE0_TARGET="${TARGET}" \
  STAGE0_DIR="${STAGE0_DIR}" \
  bash "${FETCH_SCRIPT}" >"${TMP_ROOT}/tampered.stdout" 2>"${TMP_ROOT}/tampered.stderr"
tampered_status=$?
set -e

[[ "${tampered_status}" -ne 0 ]] || {
  echo "fetch-stage0 unexpectedly accepted an unregistered package payload" >&2
  exit 1
}
grep -F 'not listed in package checksums' "${TMP_ROOT}/tampered.stderr" >/dev/null \
  || { echo "fetch-stage0 did not report the unregistered payload" >&2; cat "${TMP_ROOT}/tampered.stderr" >&2; exit 1; }
[[ "$(<"${STAGE0_DIR}/keep.txt")" == 'keep existing stage0' ]] \
  || { echo "unregistered payload changed the existing stage0" >&2; exit 1; }

echo "fetch-stage0 atomic install rollback test passed"
