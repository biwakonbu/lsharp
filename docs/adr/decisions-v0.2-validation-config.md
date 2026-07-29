# ADR: v0.2 validation manifest の project config discovery

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-driver/src/config.rs`, `crates/lsharp-driver/src/main.rs`
- Related: `EC-M2-03`, `v0.2-milestone-02.md`, `v0.2-validation-model.md`,
  `intent-validation.md`

## Context

Rust の `lsharp validate <manifest>` は version 1 JSON manifest を検証できるが、project
command から毎回 path を渡す必要があった。入力を暗黙の `docs/intent-graph.json` へ寄せると、
どの project の evidence を検証したかが曖昧になる。一方で設定から外部 path を自由に参照
できると、validation provenance と再現性を壊す。

## Decision

- `lsharp.toml` に `[validation].manifest` を追加し、`lsharp validate` の manifest 引数を
  optional にする。明示した CLI path の既存導線は維持する。
- 引数省略時は current directory から祖先の `lsharp.toml` を探索し、その project root から
  manifest を解決する。
- 設定由来の path は非空の project-relative path に限定し、`..`、絶対 path、missing file、
  通常 file でない target、canonical project root 外へ出る symlink を fail-closed に拒否する。
- 設定も明示 path もない場合は既定 manifest を推測せず、入力診断を返す。

## Evidence

- RED: config field/resolver 未実装のため config unit tests が unresolved field/function で
  失敗し、CLI の config discovery、missing config、path traversal tests が失敗。
- GREEN: config parse、project-relative resolution、absolute/parent/empty/missing rejection、
  outside-root symlink rejection、root/nested-directory CLI discoveryを固定。
- CLI boundary follow-up: `validate` の project-config discoveryでも、absolute manifest path と
  project root外を指す manifest symlink を report生成前に拒否し、non-zero exit、空 stdout、path
  boundary診断を返すことを `validate_cli` に追加した。既存の `..` rejection と合わせ、設定由来の
  path safetyが resolver単体だけでなく公開 command surfaceへ伝播することを固定した。
- `cargo test -p lsharp-driver --bin lsharp config::tests`、
  `cargo test -p lsharp-driver --test validate_cli`、driver clippy、workspace check、targeted
  Rustfmt、`git diff --check`、`bash scripts/audit_docs.sh` をこの slice の gate とする。
- CLI follow-up gate: `validate_cli` 29 tests、driver clippy、対象Rustfmt、`git diff --check`、
  `bash scripts/audit_docs.sh`（0 errors/warnings）を passした。これは Rust-host CLI verified
  partial sliceであり、selfhost/native、current-source stage0、Mac/Linux target parityの証拠ではない。
- CLI completeness follow-up: project config の empty path、missing file、directory target も
  report生成前に拒否し、non-zero exit、空 stdout、個別の path boundary 診断を返す3ケースを追加した。
  resolver unit testに留まっていた non-empty/project-relative/regular-file 契約を公開 command surfaceへ
  接続し、mietteの行折返しを含む診断を安定して検証する。
- CLI completeness gate: `validate_cli` 32 tests、driver clippy、対象Rustfmt、`git diff --check`、
  `bash scripts/audit_docs.sh`（0 errors/warnings）を passした。これは Rust-host CLI verified
  partial sliceであり、selfhost/native、current-source stage0、Mac/Linux target parityの証拠ではない。

## Consequences

project command から再現可能に manifest を発見でき、validation input の project ownership と
path boundary が明示される。source syntax adapter、selfhost/native parity、EmbeddedCli/MCP、
Mac Apple Silicon / Linux x86_64 の artifact/runtime parityは未完了であり、M2 完了や
Rust-free 完了の証拠には拡大解釈しない。
