//! metadata directive allowlist の parity test (`I-18` / `PARSER-PARITY-01`)
//!
//! `:` で始まる metadata directive を受理するかの判定表は 3 系統に手書きで存在する。
//!
//! | 系統 | 場所 | 役割 |
//! |---|---|---|
//! | `decl` | `parser/decl.rs` の `is_colon_directive` | directive として受理するか |
//! | `metadata` | `parser/metadata.rs` の `try_parse_metadata` | 受理したものをどう読むか |
//! | `selfhost` | `selfhost/src/Syntax/Parser.ls` の `directive-symbol-v3` (+ `source-` 版) | selfhost front end の受理判定 |
//!
//! 本 test は 3 系統の**完全一致**ではなく、**ペアごとの差分が既知の集合と一致すること**を
//! 検査する。3 者は正しく運用していても一致しないためで、判断の根拠は
//! `docs/adr/decisions-parser-directive-allowlist-parity.md` が正本。
//!
//! `selfhost/src` は compile-time ではなく実行時に読む (`include_str!` を使わない)。
//! compile-time 依存を作ると `.ls` の編集が Rust の再ビルドを誘発するため。

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

/// `decl` にだけあり `selfhost` に無い directive。
///
/// `roots-unbalanced` は 2026-08-18 に Rust 側だけへ追加した (`I-14` / `I-18`)。
/// **この 1 件が `I-18` が記録している唯一の意味論的 divergence** であり、
/// selfhost へ port したらこの集合を空にする。
const DECL_MINUS_SELFHOST: &[&str] = &["roots-unbalanced"];

/// `decl` にだけあり `metadata` に無い directive。
///
/// `where` / `constraints` は lexer が専用トークン (`TokenKind::Where` /
/// `TokenKind::Constraints`) へ落とすため、`is_colon_directive` で実際に効くのは
/// `Some(TokenKind::Where)` 側の腕であり、`matches!` 内の文字列腕は**到達しない死んだ枝**。
/// `try_parse_metadata` は `Some(TokenKind::Symbol(_))` しか見ないので、
/// この 2 つがそちらに無いのは正しい。**divergence ではない。**
const DECL_MINUS_METADATA: &[&str] = &["where", "constraints"];

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read_repo_file(relative: &str) -> String {
    let path = repo_root().join(relative);
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} を読めない: {e} (解決先 {})", relative, path.display()))
}

/// `haystack` から `start` 以降 `end` の手前までを切り出す。`end` が無ければ末尾まで。
fn slice_between<'a>(haystack: &'a str, what: &str, start: &str, end: &str) -> &'a str {
    let from = haystack
        .find(start)
        .unwrap_or_else(|| panic!("{what} の開始位置 {start:?} が見つからない"));
    let rest = &haystack[from + start.len()..];
    match rest.find(end) {
        Some(to) => &rest[..to],
        None => rest,
    }
}

/// 抽出が壊れて空集合を返したまま緑になるのを防ぐ。
///
/// 本 test は差分だけを見るので、抽出が全滅すると差分も空になり**静かに pass する**。
/// 各表の実測件数を大きく下回ったら「divergence が消えた」ではなく
/// 「抽出パターンが実装の書き換えに追随できていない」と読む。
fn assert_extraction_alive(what: &str, names: &BTreeSet<String>, minimum: usize) {
    assert!(
        names.len() >= minimum,
        "{what} の抽出結果が {} 件しかない (最低 {minimum} 件を期待)。\n\
         これは allowlist の divergence ではなく、抽出パターンが実装の書き換えに\n\
         追随できていない可能性が高い。test 側の抽出を実装に合わせて直すこと。\n\
         抽出できた名前: {names:?}",
        names.len()
    );
}

/// `parser/decl.rs` の `is_colon_directive`。
///
/// 関数本体に現れる文字列リテラルは directive 名だけなので、そのまま拾う。
fn decl_allowlist() -> BTreeSet<String> {
    let src = read_repo_file("crates/lsharp-syntax/src/parser/decl.rs");
    let body = slice_between(
        &src,
        "is_colon_directive",
        "fn is_colon_directive",
        "\n    }",
    );
    let names: BTreeSet<String> = body
        .split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect();
    assert_extraction_alive("decl.rs の is_colon_directive", &names, 25);
    names
}

/// `parser/metadata.rs` の `try_parse_metadata`。
///
/// 同関数には directive 名以外の文字列リテラル (エラーメッセージ) も現れるため、
/// match の腕そのもの (インデント 24 桁の `"name" =>`) だけを拾う。
/// `parse_property_form` 側の option 名 (インデント 16 桁) はこの表に含めない。
/// `try_parse_metadata` の match の腕のインデント幅。
const ARM_INDENT: &str = "                        ";

fn metadata_allowlist() -> BTreeSet<String> {
    let src = read_repo_file("crates/lsharp-syntax/src/parser/metadata.rs");
    let body = slice_between(
        &src,
        "metadata.rs の try_parse_metadata",
        "fn try_parse_metadata",
        "\n    }",
    );
    let names: BTreeSet<String> = body
        .lines()
        .filter_map(|line| {
            let arm = line.strip_prefix(ARM_INDENT)?;
            if arm.starts_with(' ') {
                return None;
            }
            let name = arm.strip_prefix('"')?;
            let (name, rest) = name.split_once('"')?;
            rest.trim_start().starts_with("=>").then(|| name.to_owned())
        })
        .collect();
    assert_extraction_alive("metadata.rs の try_parse_metadata", &names, 25);
    names
}

/// `selfhost/src/Syntax/Parser.ls` の `directive-symbol-v3` と、
/// そこから fall through する `source-directive-symbol-v3` の和集合。
fn selfhost_allowlist() -> BTreeSet<String> {
    let src = read_repo_file("selfhost/src/Syntax/Parser.ls");
    let mut names = BTreeSet::new();
    for defn in [
        "(defn directive-symbol-v3 ",
        "(defn source-directive-symbol-v3 ",
    ] {
        let body = slice_between(&src, defn, defn, "\n(defn ");
        for chunk in body.split("(string-eq name \"").skip(1) {
            let name = chunk
                .split_once('"')
                .unwrap_or_else(|| panic!("{defn} の string-eq が閉じていない"))
                .0;
            names.insert(name.to_owned());
        }
    }
    assert_extraction_alive("selfhost の directive-symbol-v3", &names, 25);
    names
}

fn expected(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|s| (*s).to_owned()).collect()
}

fn difference(left: &BTreeSet<String>, right: &BTreeSet<String>) -> BTreeSet<String> {
    left.difference(right).cloned().collect()
}

/// Rust parser と selfhost parser の受理判定がずれていないこと。
///
/// 既知の divergence (`roots-unbalanced`) 以外の差が出たら、片側にだけ directive を
/// 足した状態である。両側に足すか、`DECL_MINUS_SELFHOST` を意図ごと更新すること。
#[test]
fn rust_and_selfhost_directive_allowlists_differ_only_by_known_divergence() {
    let decl = decl_allowlist();
    let selfhost = selfhost_allowlist();

    assert_eq!(
        difference(&decl, &selfhost),
        expected(DECL_MINUS_SELFHOST),
        "Rust parser にだけある directive が既知の divergence と一致しない (I-18)"
    );
    assert_eq!(
        difference(&selfhost, &decl),
        BTreeSet::new(),
        "selfhost parser にだけある directive がある。Rust parser 側が受理しないため \
         同じソースが front end によって落ちる (型注釈の parse error として現れる)"
    );
}

/// 受理判定 (`is_colon_directive`) と読み取り (`try_parse_metadata`) がずれていないこと。
///
/// 受理したのに読めない directive は、payload を黙って捨てる経路になる。
#[test]
fn rust_accept_and_parse_allowlists_differ_only_by_reserved_word_directives() {
    let decl = decl_allowlist();
    let metadata = metadata_allowlist();

    assert_eq!(
        difference(&decl, &metadata),
        expected(DECL_MINUS_METADATA),
        "受理はするが読み取り側に無い directive がある。予約語トークン経由の \
         where / constraints 以外は payload が黙って捨てられる"
    );
    assert_eq!(
        difference(&metadata, &decl),
        BTreeSet::new(),
        "読み取り側にあるのに is_colon_directive が受理しない directive がある。\
         この directive は到達しない"
    );
}

/// 上記 2 test が見ている差分の実測件数を固定する。
///
/// 差分だけを見る test は、3 系統が**揃って**同じ方向へずれた場合に緑のままになる。
/// 表の規模そのものを 1 箇所で pin して、その抜けを塞ぐ。
#[test]
fn directive_allowlist_sizes_are_pinned() {
    assert_eq!(decl_allowlist().len(), 29, "decl.rs の is_colon_directive");
    assert_eq!(
        metadata_allowlist().len(),
        27,
        "metadata.rs の try_parse_metadata"
    );
    assert_eq!(
        selfhost_allowlist().len(),
        28,
        "selfhost の directive-symbol-v3 (source- 版を含む)"
    );
}
