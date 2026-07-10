#!/bin/bash
# Build and verify the static documentation site from docs/site.toml.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
OUTPUT_DIR="${1:-"$ROOT_DIR/_site"}"

cd "$ROOT_DIR"

rm -rf "$OUTPUT_DIR"
cargo run -q -p lsharp-driver -- doc-site --output "$OUTPUT_DIR"

required_files=(
  "index.html"
  ".nojekyll"
  "sitemap.xml"
  "docs-site-manifest.json"
  "guides/quick-start.html"
  "guides/language-reference.html"
  "book/introduction.html"
  "language/runtime-spec.html"
  "operations/documentation-site.html"
  "api/stdlib.json"
  "api/Core.html"
)

for file in "${required_files[@]}"; do
  if [[ ! -s "$OUTPUT_DIR/$file" && "$file" != ".nojekyll" ]]; then
    echo "ERROR: doc-site output missing or empty: $OUTPUT_DIR/$file" >&2
    exit 1
  fi
  if [[ "$file" == ".nojekyll" && ! -f "$OUTPUT_DIR/$file" ]]; then
    echo "ERROR: doc-site output missing: $OUTPUT_DIR/$file" >&2
    exit 1
  fi
done

if ! grep -q 'data-source="docs/site.toml"' "$OUTPUT_DIR/index.html"; then
  echo "ERROR: index.html does not expose docs/site.toml as the site source" >&2
  exit 1
fi

if ! grep -q '"source": "docs/site.toml"' "$OUTPUT_DIR/docs-site-manifest.json"; then
  echo "ERROR: docs-site-manifest.json does not mirror docs/site.toml source" >&2
  exit 1
fi

echo "doc-site output verified: $OUTPUT_DIR"
