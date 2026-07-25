# ADR: v0.2 lowering コンテキストの再利用境界

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-ir/src/lower/mod.rs`, `crates/lsharp-ir/src/lower/tests.rs`
- Related: `docs/development/planning/rust-parity-spec.md` (IR lowering)

## Context

`Lower` は backend を保持しながら、プログラム単位の関数 index、型 index、GC 型、文字列
データ、lambda lifting 状態を蓄積する。複数の compilation unit を同じ lowering コンテキスト
で処理する呼び出し経路があるため、前のプログラムの state が次の IR に漏れないことが
`lower_program_with_expr_types` の observable contract になる。これまでは `reset_state` の
実装は存在したが、WasmGC の型定義と文字列を含むプログラムから別のプログラムへ再利用する
回帰テストがなかった。

## Decision

- `lower_program_with_expr_types` の各呼び出しは、同じ `Lower` を再利用しても fresh context で
  同じプログラムを lowering した結果と同じ module dump、文字列データ、GC 型数を返すことを
  契約とする。
- backend の選択はコンテキスト生成時に固定し、プログラム単位の state は既存の
  `prepare_program_state` / `reset_state` 境界で初期化する。
- この slice では lowering の意味論や state の所有構造を変更せず、今後の `lower/mod.rs`
  分割時にも再利用契約を維持する回帰テストを正本とする。

## Evidence

- Contract test: `lower_context_reuse_matches_a_fresh_context` は、WasmGC record/string を
  lowering した後に単純な別プログラムを同じ context で lowering し、fresh context と比較する。
- RED/GREEN: 新規テストを先に追加し、既存実装で pass することを確認した。これは欠陥修正ではなく、
  未固定だった state-reset contract を回帰防止として追加したものである。

## Boundary

これは Rust `lsharp-ir` の lowering context 再利用に限定した verified slice である。source
native stage0、selfhost parity、Wasm runtime、両対応 target、公開 command、M2 aggregate の
完了を意味しない。
