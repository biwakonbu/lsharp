# トレイト -- アドホック多相とインタフェース

> **実装状態**: トレイト定義・impl の構文解析と型チェック、デフォルト実装のフォールバック、静的ディスパッチによる IR 変換は実装済み。WasmGC vtable による動的ディスパッチは未実装。

## 多相性の2つの形

第 4 章で見た HM 型推論は**パラメトリック多相** (parametric polymorphism) を提供する。`(defn id [x] x)` は**任意の**型に対して同じように動作する。

しかし、型ごとに**異なる**動作をしたい場合がある。たとえば「値を文字列に変換する」操作は、`Int` と `Point` で処理が異なる。これを**アドホック多相** (ad hoc polymorphism) と呼ぶ。

パラメトリック多相は「型を知らなくても動く」コードを書くための仕組みであり、アドホック多相は「型に応じて振る舞いを変える」仕組みである。L# のトレイトは後者を実現する。

## トレイトとは

トレイトは「ある型が満たすべきインタフェース」を定義する。Rust のトレイト、Haskell の型クラス、Swift のプロトコルに相当する:

```lisp
;; Show トレイト: 値を文字列表示できる型の集合
(trait (Show a)
  (defn show [(: self a)] : String))

;; Eq トレイト: 等値比較できる型の集合
(trait (Eq a)
  (defn eq [(: self a) (: other a)] : Bool)
  ;; デフォルト実装
  (defn ne [(: self a) (: other a)] : Bool
    (not (eq self other))))
```

トレイト宣言は以下の情報を型推論器に登録する:

- **トレイト名**: `Show`, `Eq`
- **型パラメータ**: `a` (トレイトが適用される型)
- **メソッドのシグネチャ**: 各メソッドの型スキーム
- **デフォルト実装**: メソッド本体が付与されたもの

## 型推論器での表現

トレイトに関連する情報は `crates/lsharp-types/src/types.rs` に定義されている:

```rust
/// トレイト定義情報
pub struct TraitInfo {
    pub name: String,
    pub type_param: TypeVarId,
    pub methods: Vec<(String, TypeScheme)>,
}

/// トレイト実装情報
pub struct ImplInfo {
    pub trait_name: String,
    pub type_name: String,
    pub methods: Vec<(String, Type)>,
}
```

`TraitInfo` はトレイト宣言時に登録される。`type_param` はトレイトの型パラメータを表す型変数 ID で、`methods` は各メソッドの名前と型スキームのペアである。

`ImplInfo` は `impl` ブロックの処理時に登録される。`type_name` は実装対象の具体型 (例: `"Point"`) で、`methods` は各メソッドの名前と具体的な型のペアである。

型推論器は `traits: HashMap<String, TraitInfo>` と `impls: Vec<ImplInfo>` をフィールドとして保持し、トレイト制約の解決に使用する。

## トレイト実装

特定の型に対してトレイトを実装する:

```lisp
(impl (Show Point)
  (defn show [(: self Point)] : String
    (str "Point(" (Point.x self) ", " (Point.y self) ")")))

(impl (Eq Point)
  (defn eq [(: self Point) (: other Point)] : Bool
    (and (== (Point.x self) (Point.x other))
         (== (Point.y self) (Point.y other)))))
```

impl ブロックを処理する際、型推論器は以下を検証する:

1. 指定されたトレイトが定義済みであること
2. 全ての必須メソッドが実装されていること
3. 各メソッドの型がトレイト宣言の型スキームと整合すること

## トレイト制約

関数が「Show を実装した任意の型」を受け取れるように制約を指定する:

```lisp
;; 単一制約
(defn to-string [(: x a)] : String
  :where [(Show a)]
  (show x))

;; 複数制約
(defn compare-and-show [(: x a) (: y a)] : String
  :where [(Eq a) (Show a)]
  (if (eq x y) (show x) "not equal"))
```

`:where` はメタデータキーワードとして構文解析される。型推論時には `TraitConstraint` として記録される:

```rust
/// トレイト制約
pub struct TraitConstraint {
    pub trait_name: String,
    pub type_var: TypeVarId,
}
```

制約付き関数を呼び出す際、型推論器は具体型がトレイトを実装しているかを `impls` から検索する。実装が見つからない場合は `TypeError::MissingImpl` エラーとなる。

## デフォルト実装

`Eq` トレイトの `ne` メソッドのように、デフォルト実装を持つメソッドは `impl` ブロックで省略できる。型推論器は `default_impls` キャッシュを保持し、impl に明示的な実装がなければデフォルト実装にフォールバックする:

```lisp
(impl (Eq Point)
  ;; eq のみ実装すれば、ne はデフォルト実装が使われる
  (defn eq [(: self Point) (: other Point)] : Bool
    (and (== (Point.x self) (Point.x other))
         (== (Point.y self) (Point.y other)))))
```

デフォルト実装のフォールバック処理は以下の手順で行われる:

1. impl ブロックのメソッドリストを走査
2. トレイト宣言の全メソッドと照合
3. 実装が欠けているメソッドがあれば、トレイト宣言のデフォルト実装を探す
4. デフォルト実装があればそれを使用、なければコンパイルエラー

## 静的ディスパッチ: IR レベルでの実現

L# のトレイトは**静的ディスパッチ**で実現される。コンパイル時に呼び出し先の具体型が確定している場合、メソッド呼び出しを直接の関数呼び出しに変換する。

IR 降位 (`crates/lsharp-ir/src/lower/decl.rs`) での処理:

```rust
/// トレイトメソッド呼び出しを静的ディスパッチで解決
pub(crate) fn resolve_trait_dispatch(
    &self, method_name: &str, args: &[Expr]
) -> Option<u32> {
    // メソッド名がトレイトメソッドとして登録されているか確認
    let trait_names = self.trait_method_names.get(method_name)?;

    // 第一引数の型名を推定
    let first_arg_type = if let Some(arg) = args.first() {
        self.infer_expr_type_name(arg)
    } else {
        None
    };

    if let Some(type_name) = first_arg_type {
        // (trait_name, type_name, method_name) でマングル名を検索
        for trait_name in trait_names {
            let key = (trait_name.clone(), type_name.clone(),
                       method_name.to_string());
            if let Some(mangled) = self.trait_method_impls.get(&key) {
                return self.func_indices.get(mangled).copied();
            }
        }
    }

    // 型が不明な場合、実装が1つだけならそれを使う (一意解決)
    // ...
    None
}
```

マングル名は `TraitName_TypeName_method` の形式で生成される。例えば `(impl (Show Point) ...)` の `show` メソッドは `Show_Point_show` という名前で関数テーブルに登録される。

## 辞書パッシング: 動的ディスパッチの設計

静的に型が確定しない場合のために、**辞書パッシング (dictionary passing)** による動的ディスパッチも設計されている。各トレイト実装を「辞書」として WasmGC の構造体に格納し、関数の追加引数として渡す:

```wasm
;; Show トレイトの辞書型
(type $Show_dict (struct
  (field $show (ref $show_func_type))))
(type $show_func_type (func (param (ref eq)) (result (ref $String))))

;; Point 用 Show 辞書インスタンス
(global $show_Point_dict (ref $Show_dict)
  (struct.new $Show_dict (ref.func $show_Point)))
```

辞書パッシングでは、トレイト制約付き関数に暗黙の辞書引数が追加される:

```
;; 脱糖前
(defn to-string [(: x a)] : String :where [(Show a)] (show x))

;; 脱糖後 (概念的)
(defn to-string [dict x] (dict.show x))
```

呼び出し側は具体型に応じた辞書を渡す:

```
(to-string show_Point_dict my_point)
```

静的ディスパッチが可能な場合は**単相化 (monomorphization)** で最適化する。呼び出し時に具体型が確定していれば、辞書の間接呼び出しを直接呼び出しに変換できる。

## 他言語との比較

| 特徴 | L# トレイト | Haskell 型クラス | Rust トレイト |
|------|------------|-----------------|-------------|
| 解決タイミング | コンパイル時 | コンパイル時 | コンパイル時 |
| ディスパッチ方式 | 静的 (MVP) | 辞書パッシング | 単相化 + dyn |
| デフォルト実装 | あり | あり | あり |
| Orphan Rule | あり | あり (拡張で緩和) | あり |
| Associated Types | 将来計画 | あり (TypeFamilies) | あり |
| 多パラメータ | 未対応 | あり (MPTC) | 未対応 |
| Superclass | 未対応 | あり | あり (Supertrait) |

L# の現在の実装は Rust のトレイトに近い設計で、静的ディスパッチを優先する。Haskell の辞書パッシングは実行時オーバーヘッドがあるが、より柔軟な多相性を提供する。

## Orphan Rule

トレイトの実装は**orphan rule** に従う。ある型に対するトレイトの実装は、型またはトレイトが定義されたモジュールでのみ許可される。これにより、異なるモジュールで同じ型に対する矛盾した実装が作られることを防ぐ。

例えば、モジュール A で `Show` トレイトが定義され、モジュール B で `Point` 型が定義されている場合、`(impl (Show Point) ...)` は A または B のいずれかで行う必要がある。第三のモジュール C で行うことはできない。

## Kind チェックとトレイト

高カインド型 (HKT) のトレイトでは、実装型のカインドがトレイトの要求と一致するかを検証する:

```rust
fn kinds_compatible(trait_kind: &Kind, type_kind: &Kind) -> bool {
    match (trait_kind, type_kind) {
        (Kind::Star, Kind::Star) => true,
        (Kind::Arrow(_, _), Kind::Arrow(_, _)) => trait_kind == type_kind,
        _ => false,
    }
}
```

`(trait (Functor f) ...)` の `f` は `* -> *` カインドを持つため、`(impl (Functor Int) ...)` は `Int` のカインドが `*` であるためカインド不一致エラーとなる。`(impl (Functor Option) ...)` は `Option` のカインドが `* -> *` であるため成功する。

## Associated Types (将来拡張)

将来的には associated types (関連型) もサポートする予定:

```lisp
(trait (Collection c)
  (type-assoc Item)
  (defn get [(: self c) (: idx Int)] : (Option Item)))
```

associated types により、トレイトが「出力の型」も指定できるようになる。これは Rust の `type Output` や Haskell の `TypeFamilies` に相当する機能である。
