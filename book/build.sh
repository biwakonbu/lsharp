#!/usr/bin/env bash
# L# 書籍ビルドスクリプト
set -euo pipefail

BOOK_DIR="$(cd "$(dirname "$0")" && pwd)"
OUTPUT_DIR="${BOOK_DIR}/output"
FORMAT="${1:-pdf}"

mkdir -p "${OUTPUT_DIR}"

# まえがき
PREFACE="${BOOK_DIR}/preface.md"

# 本文 (全16章)
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
  "${BOOK_DIR}/ch14-stdlib.md"
  "${BOOK_DIR}/ch15-selfhosting.md"
  "${BOOK_DIR}/ch16-lsp.md"
)

# 付録・あとがき
APPENDICES=(
  "${BOOK_DIR}/appendix-a-grammar.md"
  "${BOOK_DIR}/afterword.md"
)

# 存在するファイルだけを収集
ALL_FILES=()
for f in "${PREFACE}" "${CHAPTERS[@]}" "${APPENDICES[@]}"; do
  if [[ -f "$f" ]]; then
    ALL_FILES+=("$f")
  else
    echo "警告: $f が見つかりません (スキップ)"
  fi
done

case "${FORMAT}" in
  pdf)
    echo "PDF をビルドしています..."
    pandoc \
      --defaults="${BOOK_DIR}/metadata.yaml" \
      -o "${OUTPUT_DIR}/lsharp-book.pdf" \
      "${ALL_FILES[@]}"
    echo "完了: ${OUTPUT_DIR}/lsharp-book.pdf"
    ;;
  html)
    echo "HTML をビルドしています..."
    pandoc \
      --defaults="${BOOK_DIR}/metadata.yaml" \
      --standalone \
      -o "${OUTPUT_DIR}/lsharp-book.html" \
      "${ALL_FILES[@]}"
    echo "完了: ${OUTPUT_DIR}/lsharp-book.html"
    ;;
  epub)
    echo "EPUB をビルドしています..."
    pandoc \
      --defaults="${BOOK_DIR}/metadata.yaml" \
      -o "${OUTPUT_DIR}/lsharp-book.epub" \
      "${ALL_FILES[@]}"
    echo "完了: ${OUTPUT_DIR}/lsharp-book.epub"
    ;;
  count)
    echo "各章の行数:"
    for f in "${ALL_FILES[@]}"; do
      wc -l "$f"
    done
    echo "---"
    cat "${ALL_FILES[@]}" | wc -l | xargs echo "合計:"
    ;;
  *)
    echo "使い方: $0 [pdf|html|epub|count]"
    exit 1
    ;;
esac
