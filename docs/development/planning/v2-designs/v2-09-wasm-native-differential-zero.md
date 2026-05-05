# V2-09: Wasm/native differential zero

## 概要

Wasm と native backend の観測差分ゼロは、Component Model pivot 後は Phase 11 の completion gate ではなく Deferred/v2 の研究課題として扱う。  
このページでは、既存の differential harness / allowlist / sample parity を future tier1 parity へ繋ぐための正本条件を整理する。

## 前提条件

- V2-08 の native self-regeneration 経路が representative input set で実行できる
- `tests/differential-allowlist.yaml` の `allowlist: []` を維持する
- 公式配布が host launcher + embedded guest component であることを維持する

## 現状 evidence

- `test_e2e_wasm_native_differential_five_observation_points`
- `test_e2e_differential_allowlist_empty`
- `test_e2e_wasm_native_differential_structural_parity`
- `test_e2e_zero_diff_const_0`
- `test_e2e_zero_diff_const_1`
- `test_e2e_zero_diff_const_42`
- `test_e2e_zero_diff_const_100`
- `test_e2e_zero_diff_sample_summary`
- `test_e2e_zero_diff_const_extended_corpus`

これらは differential harness と narrow zero-diff sample の partial evidence であり、tier1 artifact 全体の zero diff 完了証跡ではない。

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

Deferred。Phase 11 完了判定には含めない。現状の empty allowlist と sample parity は regression guard であり、tier1 differential zero の完了証跡ではない。
