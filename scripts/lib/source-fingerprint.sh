#!/usr/bin/env bash
# selfhost source tree の fingerprint を計算する共有実装。
#
# stage0 の再利用可否は「manifest が記録した source fingerprint」と「current checkout の
# source fingerprint」の一致で判定する。この計算が producer 側 (package-native-stage0.sh) と
# consumer 側 (native-selfhost-dev.sh) で 1 バイトでもずれると全 stage0 が使えなくなるため、
# 実装はこのファイルだけに置き、両者が source する。
#
# 使い方:
#   source "<root>/scripts/lib/source-fingerprint.sh"
#   fingerprint="$(lsharp_source_fingerprint "<source-root>/src")"
#
# 出力は 64 桁の小文字 16 進数 (sha256)。

lsharp_source_fingerprint_hash_stream() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 | awk '{print $1}'
  else
    echo "error: sha256sum or shasum is required for source fingerprint" >&2
    return 1
  fi
}

lsharp_source_fingerprint_hash_file() {
  local path="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$path" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$path" | awk '{print $1}'
  else
    echo "error: sha256sum or shasum is required for source fingerprint" >&2
    return 1
  fi
}

# ソート順は LC_ALL=C 固定。locale 依存で順序が変わると同一ソースが別 fingerprint になる。
lsharp_source_fingerprint() {
  local src_dir="$1"
  if [[ ! -d "$src_dir" ]]; then
    echo "error: source directory not found: $src_dir" >&2
    return 1
  fi
  (
    cd "$src_dir" || exit 1
    while IFS= read -r source_path; do
      printf '%s  %s\n' "$(lsharp_source_fingerprint_hash_file "$source_path")" "$source_path"
    done < <(find . -type f -print | LC_ALL=C sort)
  ) | lsharp_source_fingerprint_hash_stream
}
