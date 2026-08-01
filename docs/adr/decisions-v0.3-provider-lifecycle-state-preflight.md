# ADR: v0.3 provider review-lifecycle state preflight

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: `scripts/ci/verify-native-release-identity.py` の明示 review-lifecycle snapshot semantic preflight
- Related: [`decisions-v0.3-provider-snapshot-digest-verification.md`](decisions-v0.3-provider-snapshot-digest-verification.md)、
  [`decisions-v0.3-native-mcp-provider-semantic-boundary.md`](decisions-v0.3-native-mcp-provider-semantic-boundary.md)

## Context

release identity verifier は provider の trust-store / review-lifecycle snapshot を raw bytes の digest として
照合していたが、lifecycle record の未知の `state` を semantic input errorとして拒否していなかった。digestが一致しても
`pending` のような未定義 stateを含む snapshotがrelease identity boundaryへ到達できる状態だった。

## Decision

明示された `--review-lifecycle` snapshotは、UTF-8のJSON object、JSON object array、またはJSONL recordとして読み、各
recordの `state` を `proposed`、`active`、`superseded`、`revoked` のいずれかへ限定する。空record、非object、壊れたJSON、
未知stateは fail-closed に拒否する。既存のregular non-symlink file、non-empty、raw digest、trust-storeとのrole bindingは
維持し、trust-storeの内容検証やnetwork取得はこのsliceへ追加しない。

## Evidence

`test-native-release-identity.py` に、正しい artifact/source/provider digestを持つ `state: "pending"` snapshotを追加した。
実装前は verifier が exit 0 となる RED、実装後は `review lifecycle state must be one of ...` で拒否する GREENを確認した。
既存の release identity、official release snapshot、provider snapshot、stage0 package harnessは同じ focused batchで再実行する。

## Boundary and follow-up

これは lifecycle state allowlist の verified partial sliceである。署名/authentication、sequence ordering/reducer、live provider
API取得、MCP semantic parity、current-source Linux runtime、Mac/Linux両 targetの packaged provenance/rollback bytes parityは
未検証のため、M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。current-source manifest/expected replay lockが
現HEADに一致せず、別セッション所有のLima/cargo/replay processもあるため Linux replay・stage regeneration・full buildは実行しない。
blockerの再現 commandは次のとおり。

```bash
current_head="$(git rev-parse --verify HEAD)"
find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'
find /tmp /Users/biwakonbu/github/tmp -maxdepth 4 \( -name 'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)
```
