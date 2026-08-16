#!/usr/bin/env bash
# selfhost/src を編集したときに cargo を起動せずに反映するための dev loop。
#
# lsharp driver は実行ファイルの隣に `<stem>.component.wasm` があればそれを embedded
# component の代わりに読む (crates/lsharp-driver/src/main.rs の
# resolve_default_component_bytes)。この仕組みを使い、`.lsharp-dev/bin/` に driver binary の
# コピーと再生成した component を並べる。cargo build も build.rs も走らない。
#
# `target/debug/` には置かない。あそこに sidecar を置くと `target/debug/lsharp` を exec する
# driver の integration test の挙動が黙って変わる。
#
# 使い方:
#   scripts/dev-loop.sh                 # 必要なら再生成して .lsharp-dev/bin/lsharp を用意
#   .lsharp-dev/bin/lsharp check foo.ls # 以後はこの binary を使う
#
# 環境変数:
#   LSHARP_DEV_COMPILER  再生成に使う compiler binary (既定: target/debug/lsharp)
#   LSHARP_DEV_DIR       生成先ディレクトリ (既定: <root>/.lsharp-dev)
#   LSHARP_DEV_ENTRY     entry file (既定: selfhost/src/App/EmbeddedCli.ls)
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SOURCE_DIR="$ROOT/selfhost/src"
ENTRY="${LSHARP_DEV_ENTRY:-$SOURCE_DIR/App/EmbeddedCli.ls}"
COMPILER="${LSHARP_DEV_COMPILER:-$ROOT/target/debug/lsharp}"
DEV_DIR="${LSHARP_DEV_DIR:-$ROOT/.lsharp-dev}"
BIN_DIR="$DEV_DIR/bin"
DEV_BIN="$BIN_DIR/lsharp"
COMPONENT="$BIN_DIR/lsharp.component.wasm"
FINGERPRINT_FILE="$DEV_DIR/.component-fingerprint.sha256"

die() {
  echo "dev-loop: $*" >&2
  exit 1
}

[[ -d "$SOURCE_DIR" ]] || die "selfhost source directory がありません: $SOURCE_DIR"
[[ -f "$ENTRY" ]] || die "entry file がありません: $ENTRY"
[[ -x "$COMPILER" ]] || die "compiler binary がありません。先に cargo build してください: $COMPILER"

# selfhost/src 配下全ファイルの内容 fingerprint。
# 実装は scripts/lib/source-fingerprint.sh に一本化してある。
# native stage0 の manifest や scripts/native-selfhost-dev.sh と同じ値でなければ、
# 「同じ source なのに別 fingerprint」になって dev lane が壊れる。
# shellcheck source=lib/source-fingerprint.sh
source "$ROOT/scripts/lib/source-fingerprint.sh"

CURRENT_FINGERPRINT="$(lsharp_source_fingerprint "$SOURCE_DIR")"
STORED_FINGERPRINT=""
if [[ -f "$FINGERPRINT_FILE" ]]; then
  STORED_FINGERPRINT="$(cat "$FINGERPRINT_FILE")"
fi

mkdir -p "$BIN_DIR"

# driver binary の同期は component 再生成とは独立に判定する。
# Rust 側だけ変えて cargo build した場合、fingerprint は一致したままなので
# ここを分けないと古い binary を使い続けてしまう。
if [[ ! -f "$DEV_BIN" || "$COMPILER" -nt "$DEV_BIN" ]]; then
  cp "$COMPILER" "$DEV_BIN"
  chmod +x "$DEV_BIN"
  echo "dev-loop: driver binary を更新しました ($DEV_BIN)" >&2
fi

if [[ "$CURRENT_FINGERPRINT" == "$STORED_FINGERPRINT" && -f "$COMPONENT" ]]; then
  echo "dev-loop: selfhost/src は変更なし。component 再生成をスキップします" >&2
  echo "$DEV_BIN"
  exit 0
fi

echo "dev-loop: selfhost/src の変更を検出しました。component を再生成します" >&2

# `lsharp compile` は入力 file を canonical 整形して in-place で書き戻す (実測済み)。
# dev loop がそれを素通しすると毎回 selfhost/src が dirty になり、build.rs の
# rerun-if-changed=selfhost/src が発火して次の cargo build がフル再コンパイルになる。
# 待ち時間を減らすための script が待ち時間を作ってしまうので、compile 前に entry を退避する。
ENTRY_BACKUP="$DEV_DIR/.entry-backup"
cp "$ENTRY" "$ENTRY_BACKUP"

# 再生成には Rust パイプラインを使う。embedded component へ委譲させると
# 古い component 自身で新しい source をコンパイルすることになる。
#
# 整形の書き戻しは compile の「前」に起きる (prepare_source_for_compile)。つまり
# 「整形差分あり + 型エラー」という編集中に最も多い組み合わせでは、entry が書き換わった
# 直後に compile が落ちる。ここで即 die すると書き戻しが残るので、rc を持ち回して
# 復元を先に済ませる。
COMPILE_RC=0
LSHARP_DISABLE_EMBEDDED_COMPONENT=1 "$COMPILER" compile "$ENTRY" \
  --target wasi-component -o "$COMPONENT" \
  || COMPILE_RC=$?

if ! cmp -s "$ENTRY_BACKUP" "$ENTRY"; then
  cp "$ENTRY_BACKUP" "$ENTRY"
  echo "dev-loop: compiler が entry file を書き換えました。復元しました ($ENTRY)" >&2
fi
rm -f "$ENTRY_BACKUP"

[[ "$COMPILE_RC" -eq 0 ]] || die "component の再生成に失敗しました (rc=$COMPILE_RC)"

# entry 以外まで書き換えられていた場合は復元手段が無い。fingerprint を記録せず fail-closed
# にして、source tree が黙って変わったまま先へ進むことを防ぐ。
RESTORED_FINGERPRINT="$(lsharp_source_fingerprint "$SOURCE_DIR")"
if [[ "$RESTORED_FINGERPRINT" != "$CURRENT_FINGERPRINT" ]]; then
  die "selfhost/src が予期せず変更されました (compile 前: $CURRENT_FINGERPRINT / compile 後: $RESTORED_FINGERPRINT)。git status で差分を確認してください"
fi

[[ -f "$COMPONENT" ]] || die "component が生成されませんでした: $COMPONENT"

# 成功したときだけ fingerprint を確定する。失敗を素通りさせない。
printf '%s\n' "$CURRENT_FINGERPRINT" >"$FINGERPRINT_FILE"
echo "dev-loop: component を更新しました ($COMPONENT)" >&2
echo "$DEV_BIN"
