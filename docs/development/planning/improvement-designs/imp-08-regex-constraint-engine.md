# imp-08: 正規表現制約エンジン (WG-2 の実体化)

> 対象 issue: [D-05](../../../../ISSUES.md#d-05) (正規表現制約が簡易パターンのみ)
> ロードマップ: [improvement-roadmap.md](../improvement-roadmap.md) Phase B-3
>
> 注: コード内コメント (`crates/lsharp-types/src/constraints.rs:95`) が参照する「WG-2」には
> 対応する設計ドキュメントが存在しなかった (2026-06-12 `grep -rn "WG-2"` で確認)。
> 本書がその実体である。

## 現状の正確な把握 (2026-06-12 更新)

制約付き型の `matches` 制約は `crates/lsharp-types/src/regex/` の
`simple_pattern_match(text: &str, pattern: &str) -> bool` で評価される。
`constraints.rs` 側にあった重複 matcher は削除し、`eval_string_constraint` は shared engine を参照する。

サポート範囲:

- リテラル / `^` `$` アンカー / `.` / `*` `+` `?` / `{n}` / `{n,m}` / `{n,}` 量指定子
- `*?` / `+?` / `??` / `{...}?` 非貪欲サフィックス (boolean match の受理言語は通常量指定子と同じ)
- `[...]` 文字クラス (範囲対応) / `\d` `\w` `\s` / `\D` `\W` `\S`
- `(...)` キャプチャグループ / `(?:...)` 非キャプチャグループ / `|` 選択
- `\1`-`\9` 後方参照 / `(?=...)` 肯定先読み / `(?!...)` 否定先読み
- `\p{L}` / `\p{N}` / `\P{L}` / `\P{N}` Unicode letter/number class

## 設計

### 0. ギャップの確定 (完了)

当初ギャップ候補の扱い:

| item | status |
|---|---|
| `{n}` / `{n,m}` / `{n,}` | implemented |
| `\d` `\w` `\s` and `\D` `\W` `\S` | implemented |
| non-greedy suffix (`*?` / `+?` / `??` / `{...}?`) | accepted as same boolean language |
| `(?:...)` non-capturing group | implemented; does not shift backreference numbering |
| Unicode | char-based; `\p{L}` / `\p{N}` / negations supported |

対応は focused unit tests と `docs/guides/language-reference.md` の syntax table で固定する。

### 1. 実装方針: 自前実装の拡張 (regex クレートは採用しない)

理由:

1. **selfhost 制約**: `matches` 制約はコンパイル時評価 (`:example` 検証等) だけでなく
   実行時検証コードとして Wasm にも lower される。Rust の regex クレートに依存すると
   selfhost コンパイラ (L# 実装) 側で同一セマンティクスを再実装できなくなり、
   Rust 版と selfhost 版の判定が乖離する
2. **後方参照・先読みを既にサポート**しており、これらは regex クレート (RE2 系) では
   表現できない。乗り換えると後退する
3. 依存追加なしで差分実装が小さい

よって `simple_pattern_match` をバックトラッキング実装のまま拡張する。

### 2. 実装ステップ

1. §0 の確定リストに対する RED テストを `crates/lsharp-types/src/regex/mod.rs` と
   `constraints.rs` の shared-engine contract に追加
2. 既存の `RegexNode` parser / NFA fallback / DFA fast path を拡張し、
   `constraints.rs` の重複 matcher を削除
3. `docs/guides/language-reference.md` に `type-constrained` と `matches` syntax table を掲載
4. `ISSUES.md` / `TODO.md` / improvement roadmap を同期

selfhost 側 runtime parity と proptest / fuzz は I-06 / I-08 の検証基盤側で扱う。
この D-05 batch は Rust constraint evaluator と public docs の同期で閉じる。

### 3. 完了条件

- §0 の確定リスト全項目が focused tests 付きで動作する
- `matches` 制約が shared regex engine を使う
- ユーザー向けドキュメントにサポート構文表がある

## 影響範囲

- 既存の `matches` 制約の判定結果は拡張のみで不変 (既存パターンの挙動を変えない)。
  既存テストの全件 green を維持する
- ステップ数上限の導入により、これまで止まらなかった病的パターンがエラーになる
  (挙動変更だが安全側)

## ステータス

resolved (2026-06-12)。Evidence:
`test_regex_bounded_quantifiers`,
`test_regex_shorthand_negated_classes`,
`test_regex_non_capturing_group_does_not_shift_backreference`,
`test_regex_lazy_quantifier_suffix_is_accepted`,
`test_string_constraint_uses_shared_regex_extended_features`,
`test_dfa_unicode_letter`。
