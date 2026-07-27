# ADR: v0.2 EmbeddedCli validation text pass parity

- Status: Accepted (verified partial slice)
- Date: 2026-07-27
- Scope: `selfhost/src/App/EmbeddedCli.ls`、`validate --source --format text`
- Related: `EC-M2-03`、`docs/adr/decisions-v0.2-selfhost-validation-pass-status.md`

## Context

Cli と EmbeddedCli は同じ validation facts を公開する必要がある。trace gap の unknown と
contradiction の fail だけでは、complete graph の pass status・exit・text projection が両 surface
で一致することを証明できない。

## Decision

- EmbeddedCli の text report は Rust `ValidationReport::to_text()` と同じ固定順の行を返す。
- trace gap なし、open question なし、独立 review あり、contradiction/stale なしの graph は
  `status: pass` と exit `0` を返す。
- `verified` shortcut は出力しない。native stage0/native MCP/target runtime は別境界に残す。

## Evidence

- `cargo test -p lsharp-wasm --test e2e selfhost_embedded_cli_main_with_args_validate_source_text_pass -- --nocapture`
  は `1 passed`。actual EmbeddedCli Wasm で status、行順、件数、exit `0` を確認した。
- 既存の EmbeddedCli text trace-gap、JSON pass/fail/stale tests は同じ source/report boundary の
  回帰証跡として保持する。
- `bash scripts/audit_docs.sh` はエラー `0`、`git diff --check` は成功した。

## Boundary

Rust-host actual Wasm の EmbeddedCli source/report/exit parity までを verified slice とする。
native stage0 producer/runtime、native MCP、Mac/Linux の artifact/runtime evidence は未完了であり、
TODO の `EC-M2-03` `[~]` を維持する。
