#!/usr/bin/env bash
# L# 書籍ビルドスクリプト
set -euo pipefail

BOOK_DIR="$(cd "$(dirname "$0")" && pwd)"
OUTPUT_DIR="${BOOK_DIR}/output"
FORMAT="${1:-pdf}"

mkdir -p "${OUTPUT_DIR}"

CHAPTERS=(
  "${BOOK_DIR}/ch01-introduction.md"
  "${BOOK_DIR}/ch02-lexer.md"
  "${BOOK_DIR}/ch03-parser.md"
  "${BOOK_DIR}/ch04-type-inference.md"
  "${BOOK_DIR}/ch05-ir.md"
  "${BOOK_DIR}/ch06-codegen.md"
  "${BOOK_DIR}/ch07-record-types.md"
  "${BOOK_DIR}/ch08-type-aliases.md"
  "${BOOK_DIR}/ch09-modules.md"
  "${BOOK_DIR}/ch10-traits.md"
  "${BOOK_DIR}/ch11-advanced-types.md"
  "${BOOK_DIR}/ch12-error-reporting.md"
  "${BOOK_DIR}/ch13-testing.md"
)

case "${FORMAT}" in
  pdf)
    echo "PDF をビルドしています..."
    pandoc \
      --defaults="${BOOK_DIR}/metadata.yaml" \
      -o "${OUTPUT_DIR}/lsharp-book.pdf" \
      "${CHAPTERS[@]}"
    echo "完了: ${OUTPUT_DIR}/lsharp-book.pdf"
    ;;
  html)
    echo "HTML をビルドしています..."
    pandoc \
      --defaults="${BOOK_DIR}/metadata.yaml" \
      --standalone \
      -o "${OUTPUT_DIR}/lsharp-book.html" \
      "${CHAPTERS[@]}"
    echo "完了: ${OUTPUT_DIR}/lsharp-book.html"
    ;;
  *)
    echo "使い方: $0 [pdf|html]"
    exit 1
    ;;
esac
