# ADR: syntax AST roundtrip と未知 Unicode escape の lexer 境界

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `crates/lsharp-syntax/src/test_gen.rs`, `crates/lsharp-syntax/src/lib.rs`, `crates/lsharp-syntax/src/lexer.rs`, `crates/lsharp-syntax/proptest-regressions/lib.txt`
- Related: `imp-07-test-verification-infrastructure.md`, `TODO.md` の `LEGACY-TEST-01`

## Context

syntax の parser/AST には個別の表示テストがあるが、複数の式形を組み合わせた pretty-print → re-parse の回帰を bounded に検出する
generator がなかった。また arbitrary-byte panic property を full crate gate で再実行したところ、未知 escape の直後に不正 UTF-8 が置換された
入力で lexer が UTF-8 char boundary の途中を再度 slice して panic することが分かった。

## Decision

- `test_gen.rs` を `cfg(test)` 専用 module とし、深さ 3・各 collection 最大 2 の `Expr` を生成する。literal/variable/if/let/lambda/application/do/
  annotation/record/quote に範囲を限定し、文字列は escape を含まない ASCII 小文字だけにする。
- `pretty_printed_ast_reparses_to_the_same_source` は生成式を `defn` に包み、`Program` の Display 結果が生成元 source と一致し、その結果を再 parse しても
  同じ Display になることを 64 cases で検証する。span の byte offset は property の契約に含めない。
- 文字列中の未知 escape は従来の「バックスラッシュを値に残す」挙動を維持しつつ、後続が非 ASCII の場合は `char::len_utf8` 分だけ消費する。
  これにより `source[pos..]` を常に char boundary から slice し、不正 UTF-8 を含む lossy source でも panic ではなく通常の lexer error を返す。
- proptest が見つけた `[34, 92, 128]` の seed は regression artifact として保持する。

## Evidence

- RED: roundtrip property を先に追加し、`test_gen` 未作成の module 解決エラーを確認した。
- RED: lexer regression test は修正前に `lexer.rs:226` の char boundary panic で失敗した。full crate property でも同じ入力を最小化して再現した。
- GREEN: lexer regression、parser panic-safety、roundtrip property が成功した。
- Regression: `cargo test -p lsharp-syntax -- --nocapture --test-threads=1`（unit 163、integration 11）、`cargo clippy -p lsharp-syntax --lib --tests -- -D warnings`、
  changed file の rustfmt check、`git diff --check` が成功した。

## Consequences

bounded AST の表示/parser regression と未知 Unicode escape の panic regression を通常の local test で検出できる。一方、AST の全 variant、
pretty-print の source span 保持、生成式全体の型推論、nightly 4096 cases、GC leak/limit、rooting stress は未完了であり、`LEGACY-TEST-01` を完了扱いにはしない。
