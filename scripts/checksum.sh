#!/usr/bin/env bash
# checksum.sh - SHA-256 チェックサム生成スクリプト
#
# P11-4 T4e-2 AC-505: SHA-256 ハッシュ
#
# 使用法:
#   ./scripts/checksum.sh <directory>
#   ./scripts/checksum.sh dist/lsharp-v1.0.0-x86_64-apple-darwin
#
# 出力形式:
#   <sha256hash>  <filename>

set -euo pipefail

TARGET_DIR="${1:-.}"

if [ ! -d "${TARGET_DIR}" ]; then
  echo "Error: directory '${TARGET_DIR}' not found" >&2
  exit 1
fi

# SHA-256 チェックサムの生成
# macOS は shasum、Linux は sha256sum を使用
if command -v sha256sum &>/dev/null; then
  HASH_CMD="sha256sum"
elif command -v shasum &>/dev/null; then
  HASH_CMD="shasum -a 256"
else
  echo "Error: sha256sum or shasum not found" >&2
  exit 1
fi

# ディレクトリ内の全ファイルのチェックサムを生成
find "${TARGET_DIR}" -type f -not -name "checksums.txt" | sort | while read -r file; do
  ${HASH_CMD} "${file}" | sed "s|${TARGET_DIR}/||"
done
