# V2-09: Wasm/native differential zero

## 概要

Wasm と native backend の観測差分ゼロは、Component Model pivot 後は Phase 11 の completion gate ではなく Deferred/v2 の研究課題として扱っていた。
現在は selfhosting completion track として、actual self-regenerated stage artifacts と aarch64 selfhost native gap 0 を完了 evidence に昇格している。

## 前提条件

- V2-08 の native self-regeneration 経路が representative input set で実行できる
- `tests/differential-allowlist.yaml` の `allowlist: []` を維持する
- 公式配布が host launcher + embedded guest component であることを維持する

## 完了 evidence

- `test_e2e_wasm_native_differential_five_observation_points`
- `test_e2e_differential_allowlist_empty`
- `test_e2e_wasm_native_differential_structural_parity`
- `test_e2e_zero_diff_const_0`
- `test_e2e_zero_diff_const_1`
- `test_e2e_zero_diff_const_42`
- `test_e2e_zero_diff_const_100`
- `test_e2e_zero_diff_sample_summary`
- `test_e2e_zero_diff_const_extended_corpus`
- `test_e2e_wasm_native_differential_uses_actual_self_regenerated_stage_artifacts`
- `test_e2e_native_actual_stage23_gap_report_has_zero_aarch64_selfhost_blockers`

これらにより、5 観測点 harness、empty allowlist、extended const corpus、actual self-regenerated stage artifacts、Darwin arm64 selfhost function-meta の native unsupported gap 0 を完了証跡として固定する。x86_64 selfhost runtime helper の full native execution は native-only RC の初期 target 外であり、今後の cross-platform native execution track で扱う。

## 設計

### 5 観測点

比較対象は次の 5 観測点で固定する。

1. exit code
2. stdout
3. stderr
4. generated file bytes
5. diagnostics JSON

### representative input categories

- normal
- parse-error
- type-error
- module-import
- file-io
- macro
- formatter

### gate の進め方

- representative input set を Wasm/native の両方で評価する
- tier1 artifact に対して generated file bytes と diagnostics JSON を比較する
- 差分は一時的に allowlist へ退避できるが、完了条件は allowlist 0 件へ戻すこと

## 正本参照

- plan 正本: [`../phase11-implementation-plan.md#v2-09-wasm-native-differential-zero`](../phase11-implementation-plan.md#v2-09-wasm-native-differential-zero)
- completion gate 境界: [`../completion-criteria.md`](../completion-criteria.md)
- native backend 契約: [`../../../language/native-backend-spec.md`](../../../language/native-backend-spec.md)

## ステータス

完了。Phase 11 完了判定には含めないが、selfhosting completion track として actual self-regenerated stage artifacts の differential input 化と aarch64 selfhost opcode gap 0 を固定済み。
