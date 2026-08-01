#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
FETCH_SCRIPT="${ROOT_DIR}/scripts/fetch-stage0.sh"
VERSION="v0.0.0-provider-url-test"
TARGET="x86_64-unknown-linux-gnu"
TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/lsharp-fetch-stage0-url.XXXXXX")"
HOST_BIN="${TMP_ROOT}/bin"
CURL_LOG="${TMP_ROOT}/curl.log"

cleanup() {
  rm -rf "${TMP_ROOT}"
}
trap cleanup EXIT

mkdir -p "${HOST_BIN}"
cat >"${HOST_BIN}/curl" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"${LSHARP_TEST_CURL_LOG:?}"
exit 0
SH
chmod 0755 "${HOST_BIN}/curl"

expect_rejected_url() {
  local label="$1"
  local url="$2"
  local expected_message="$3"
  local stderr_path="${TMP_ROOT}/${label}.stderr"
  : >"${CURL_LOG}"

  set +e
  PATH="${HOST_BIN}:${PATH}" \
    LSHARP_TEST_CURL_LOG="${CURL_LOG}" \
    STAGE0_RELEASE_BASE_URL="${url}" \
    STAGE0_VERSION="${VERSION}" \
    STAGE0_TARGET="${TARGET}" \
    STAGE0_DIR="${TMP_ROOT}/${label}-stage0" \
    bash "${FETCH_SCRIPT}" >"${TMP_ROOT}/${label}.stdout" 2>"${stderr_path}"
  local status=$?
  set -e

  [[ "${status}" -ne 0 ]] || {
    echo "${label}: unsafe provider URL unexpectedly succeeded" >&2
    exit 1
  }
  grep -F "${expected_message}" "${stderr_path}" >/dev/null || {
    echo "${label}: expected provider URL diagnostic was missing" >&2
    cat "${stderr_path}" >&2
    exit 1
  }
  [[ ! -s "${CURL_LOG}" ]] || {
    echo "${label}: curl was invoked before provider URL validation" >&2
    cat "${CURL_LOG}" >&2
    exit 1
  }
}

expect_rejected_url \
  "insecure-scheme" \
  "http://mirror.example.invalid/lsharp" \
  "ERROR: native stage0 release URL must use https:// or local file://"
expect_rejected_url \
  "embedded-credentials" \
  "https://user:secret@mirror.example.invalid/lsharp" \
  "ERROR: native stage0 release URL must not include credentials"
expect_rejected_url \
  "query" \
  "https://mirror.example.invalid/lsharp?token=secret" \
  "ERROR: native stage0 release URL must not include a query or fragment"

echo "fetch-stage0 provider URL tests: OK"
