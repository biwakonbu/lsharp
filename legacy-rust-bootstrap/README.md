# legacy-rust-bootstrap

Rust 実装との比較・監査を行うためのスナップショット置き場。**現在は空**。

## 位置づけ

Rust ワークスペースは `crates/` に**現役として存在し続ける**。
`docs/development/operations/adr-rust-removal.md:55` の維持スコープ表が
`Cargo.toml` / `Cargo.lock` / `rust-toolchain.toml` / `.cargo/` を
「物理削除しない」と明記しているとおり、本ディレクトリへ Rust 実装を
「退避」する計画は存在しない。

同 ADR は、かつての前提

- Phase 11 の完了が Rust workspace の物理削除を含む
- Rust workspace の撤去が stable native-only archive の前提条件である

を **withdrawn** として列挙している。

## rollback との関係

本ディレクトリは rollback の正路では**ない**。
`adr-rust-removal.md:104` のとおり、ここにスナップショットが置かれていたとしても
それは比較・監査用であり、必要時に差分を確認するために使う。

rollback の手順は `docs/development/operations/rollback-procedure.md` と
`scripts/rollback.sh` を正本とする。

## 注意事項

- 本ディレクトリに置かれたコードは保守対象外
- バグ修正は selfhost コンパイラ側、ないし現役の `crates/` 側で行う
