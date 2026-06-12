# Metadata-Driven Development

L# の metadata は、関数の近くにドキュメント、実行例、invariant を置き、`lsharp test` と `lsharp doc` の入力にするための仕組みです。通常の実行サンプルとは別に、metadata の検証用 source として扱います。

## Metadata Forms

`defn` の引数リストの後、関数 body の前に metadata を並べます。

```lisp
(defn abs
  [x]
  :doc "整数の絶対値を返す。"
  :params [(x "対象の整数")]
  :returns "x の絶対値"
  :example [(= (abs 5) 5)]
  :invariant (>= result 0)
  (if (< x 0) (- 0 x) x))
```

- `:doc` は関数の短い説明です。
- `:params` は `(name "description")` の列です。関数引数名と合わせます。
- `:returns` は戻り値の意味を説明します。
- `:example` は検証したい式の列です。複数式を書けます。
- `:invariant` は関数実行後に成り立つ条件です。戻り値は `result` として参照します。
- `:transitions` は `(From -> To)` の列で、状態遷移を説明する metadata です。

## Workflow

metadata を書いたら、まず metadata test と doc generation を近い範囲で確認します。

```bash
lsharp test examples/metadata.ls
lsharp doc examples/metadata.ls -o metadata.html
lsharp review examples/metadata.ls
```

- `lsharp test` は `:example` と `:invariant` から検証を生成します。
- `lsharp doc` は `:doc` / `:params` / `:returns` / `:example` を HTML 出力へ反映します。
- `lsharp review` は metadata と doc freshness の確認に使います。

## Authoring Rules

- `:params` の名前は関数引数名と一致させます。
- `:example` と `:invariant` では未定義の関数や変数を参照しないようにします。
- metadata 用の source は、通常の executable sample と分けて扱います。
- public API にする関数ほど `:doc`, `:params`, `:returns` を揃えます。

## Related Pages

- [Language Reference](./language-reference.md)
- [Examples Matrix](./examples.md)
- [Stdlib Guide](./stdlib-guide.md)
