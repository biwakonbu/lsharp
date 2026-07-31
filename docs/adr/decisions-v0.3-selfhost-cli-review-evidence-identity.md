# ADR: v0.3 selfhost App.Cli の review evidence identity wiring

- Status: Accepted (verified partial slice)
- Date: 2026-07-31
- Scope: `selfhost/src/App/Cli.ls` の `validate --source` CLI
- Related: [`decisions-v0.3-selfhost-embedded-cli-review-evidence-identity.md`](decisions-v0.3-selfhost-embedded-cli-review-evidence-identity.md)、
  [`decisions-v0.3-review-evidence-identity.md`](decisions-v0.3-review-evidence-identity.md)、
  `EC-M3-05` / `M3-05-N1`

## Context

`EmbeddedCli` には caller が明示した review evidence identity を report と manifest へ投影する
経路がある一方、通常の selfhost `App.Cli` は同じ `validate --source` を受けても six flags を
option error として拒否していた。二つの selfhost CLI surface が異なる identity provenance を
出すと、日常開発の入口と embedded/native producer の observable contract が分岐する。

identity は source の内容や環境から推測せず、caller が渡した subject、source commit、artifact、
optional trust/lifecycle、clock だけから構築する。未検証 review に identity を付けても、trust
provider が接続されていない状態を `verified` へ昇格させない。

## Decision

- `App.Cli` の validate option parser は次の flags を保持する。
  `--review-subject-digest`、`--review-source-commit`、`--review-artifact-digest`、
  `--review-trust-store-digest`、`--review-lifecycle-digest`、`--review-now`。
- subject/source/artifact/now は all-or-none とし、partial または空値を report/manifest 生成前に
  option error として拒否する。trust/lifecycle は省略可能で、JSON では `null`、text では `-` を
  出力する。
- valid context は `source-review-evidence-identity-result` で canonical timestamp を検証し、
  `source-evidence-graph-attach-review-identity` で graph へ attach する。identity の検証または
  既存 manifest identity との conflict が失敗した場合は report/manifest を生成しない。
- JSON report と emitted manifest は同じ `review_evidence_identity` object を同じ field order で
  出力する。text report は
  `review-evidence-identity: subject=... source=... artifact=... trust-store=... lifecycle=... now=...`
  の deterministic line を追加する。identity がない既存出力は変更しない。
- identity wiring は review verification を変更しない。attestation/provider がない旧 `:review`
  は `unverified` のまま、validation status は `unknown`（exit `2`）とする。

## Evidence

- RED: `test_e2e_selfhost_cli_main_validate_projects_explicit_review_evidence_identity` を実装前に
  実行し、App.Cli が `--review-*` を受理せず manifest を生成しないことを確認した。
- GREEN: 同テストで current source の `selfhost_cli_runtime_bundle` を Rust host actual Wasm へ
  compile/run し、explicit identity の JSON report、manifest object、text line が一致した。未検証
  review の exit `2` も確認した（`cargo test -p lsharp-wasm --test e2e
  selfhost_cli_main_validate_projects_explicit_review_evidence_identity -- --nocapture`、1 passed、
  309.75s）。
- `git diff --check` は通過した。既存の EmbeddedCli/native source-file smoke の同一 identity
  contract は related ADR と `scripts/ci/native-selfhost-dev-source-file-smoke.sh` で保持する。

## Boundary

これは macOS 上の Rust host が生成した selfhost App.Cli Wasm の verified partial slice である。
current-source の native stage0、packaged artifact provenance、Mac Apple Silicon / Linux x86_64
runtime parity、selfhost/native MCP、provider の実取得、`verified` / `stale` / `revoked` 判定は
未完了であり、`TODO.md` の `EC-M3-05` は `[~]` のまま維持する。
