# モジュールシステム -- コードの組織化

## なぜモジュールが必要か

プログラムが大きくなると、すべてのコードを 1 ファイルに書くことは現実的でなくなる。モジュールシステムは以下の問題を解決する:

1. **名前空間の分離**: 異なるモジュールで同じ名前を使える
2. **カプセル化**: 内部実装を隠蔽し、公開 API だけを提供する
3. **依存関係の明示**: どのモジュールが何に依存しているかが明確になる
4. **コンパイル順序の決定**: 依存先を先にコンパイルする

## 設計判断

### モジュール = ファイル

L# では 1 ファイルが 1 モジュールに対応する (F#/OCaml 寄りの設計)。ファイルパスからモジュール名が決まる:

```
src/geometry.ls  → Geometry モジュール
src/math/vec2.ls → Math.Vec2 モジュール
```

この設計を選んだ理由は明快さである。ファイルシステムの構造がそのままモジュール構造になるため、プロジェクトの構成が一目でわかる。

### 名前空間区切り

ドット `.` を使用する。これは多くの言語で馴染みがある:

```lisp
Math.Vec2.add
Geometry.Point
```

### 可視性

デフォルト公開、`(private ...)` で非公開にする方式を採用:

```lisp
(module Geometry)

;; 公開 (デフォルト)
(defn distance [p1 p2] ...)

;; 非公開
(private
  (defn helper [x] ...))
```

デフォルト公開にした理由は、L# が学習用言語としての側面を持つためである。明示的に `public` を書く必要がないことで、初学者の負担を減らす。

## 構文

### モジュール宣言

```lisp
;; ファイル先頭で宣言
(module Math.Vec2)
```

### インポート

4つのインポート形式を用意する:

```lisp
;; 完全修飾アクセス
(import Math.Vec2)
;; Math.Vec2.add で参照

;; モジュールエイリアス
(import Math.Vec2 :as V)
;; V.add で参照

;; 選択的インポート
(import Math.Vec2 :only [add sub])
;; add, sub が直接参照可能

;; 全公開 (F# の open に相当)
(import Math.Vec2 :open)
;; 全てのエクスポートが直接参照可能
```

### 使用例

```lisp
;; ファイル: src/geometry.ls
(module Geometry)

(type Point
  (record
    (: x Float)
    (: y Float)))

(defn distance [(: p1 Point) (: p2 Point)] : Float
  (let [dx (- (Point.x p1) (Point.x p2))
        dy (- (Point.y p1) (Point.y p2))]
    (sqrt (+ (* dx dx) (* dy dy)))))

;; ファイル: src/main.ls
(module Main)

(import Geometry :open)

(defn main [] : Unit
  (let [p1 {Point x 0.0 y 0.0}
        p2 {Point x 3.0 y 4.0}]
    (print (distance p1 p2))))
```

## モジュールグラフの実装

複数のファイルをコンパイルするには、モジュール間の依存関係を解析し、正しい順序で処理する必要がある。この処理は `crates/lsharp-ir/src/module_graph.rs` に実装されている。

### データ構造

```rust
/// モジュールグラフ
pub struct ModuleGraph {
    /// モジュール名 -> モジュール情報
    modules: HashMap<String, ModuleNode>,
    /// モジュール名 -> ファイルパス
    file_map: HashMap<String, String>,
}

/// モジュールノード
pub struct ModuleNode {
    pub name: String,
    pub imports: Vec<String>,
    pub file_path: Option<String>,
}
```

`ModuleGraph` は有向グラフとして機能する。各ノードはモジュール、各辺はインポート関係を表す。`add_module` でモジュールを登録し、`topological_sort` でコンパイル順序を決定する。

### 循環依存の検出

循環依存は深さ優先探索 (DFS) で検出する。探索中のノードを `in_stack` セットで追跡し、訪問済みかつスタック上にあるノードに再到達したら循環とする:

```rust
fn dfs_detect_cycle(
    &self,
    node: &str,
    visited: &mut HashSet<String>,
    in_stack: &mut HashSet<String>,
    path: &mut Vec<String>,
) -> Option<Vec<String>> {
    visited.insert(node.to_string());
    in_stack.insert(node.to_string());
    path.push(node.to_string());

    if let Some(module) = self.modules.get(node) {
        for import in &module.imports {
            if !visited.contains(import) {
                if let Some(cycle) = self.dfs_detect_cycle(
                    import, visited, in_stack, path
                ) {
                    return Some(cycle);
                }
            } else if in_stack.contains(import) {
                // 循環検出
                let start = path.iter()
                    .position(|n| n == import).unwrap_or(0);
                let mut cycle: Vec<String> = path[start..].to_vec();
                cycle.push(import.clone());
                return Some(cycle);
            }
        }
    }

    path.pop();
    in_stack.remove(node);
    None
}
```

循環が検出された場合、`ModuleGraphError::CyclicDependency` が報告される。エラーメッセージには循環パスが含まれる (例: `A -> B -> C -> A`)。

### トポロジカルソート

循環がないことを確認した後、DFS ベースのトポロジカルソートでコンパイル順序を決定する:

```rust
pub fn topological_sort(&self) -> Result<Vec<String>, ModuleGraphError> {
    // まず循環依存を検出
    if let Some(cycle) = self.detect_cycles() {
        return Err(ModuleGraphError::CyclicDependency {
            cycle: cycle.join(" -> "),
        });
    }

    let mut visited = HashSet::new();
    let mut order = Vec::new();

    for name in self.modules.keys() {
        if !visited.contains(name) {
            self.topo_dfs(name, &mut visited, &mut order);
        }
    }

    Ok(order)
}
```

結果のリストでは依存先が先に来る。例えば `Main` が `Geometry` をインポートしている場合、`[Geometry, Main]` の順序が返される。

### エラーの種類

モジュールグラフには3種類のエラーがある:

```rust
pub enum ModuleGraphError {
    /// 循環依存 (A -> B -> C -> A)
    CyclicDependency { cycle: String },
    /// 存在しないモジュールのインポート
    ModuleNotFound { name: String, from: String },
    /// 同名モジュールの重複登録
    DuplicateModule { name: String },
}
```

`check_imports` メソッドで未解決のインポートを一括検出できる。これにより、全てのエラーをまとめて報告する。

### モジュール名の変換

ファイルシステムとモジュール名の間には命名規則の変換が必要である。ファイル名は snake_case (`math_utils.ls`) だが、モジュール名は PascalCase (`MathUtils`) である:

```
ファイル: src/math_utils.ls → モジュール: MathUtils
ファイル: src/io/file.ls    → モジュール: Io.File
```

`parent_module` メソッドはドット区切りのモジュール名から親モジュールを取得する:

```rust
pub fn parent_module(name: &str) -> Option<&str> {
    name.rfind('.').map(|pos| &name[..pos])
}
// "A.B.C" -> Some("A.B")
// "A"     -> None
```

## モジュール環境 (ModuleEnv)

型推論器に `ModuleEnv` が追加されている (`crates/lsharp-types/src/infer.rs`)。各モジュールの型環境を分離し、インポート情報に基づいて名前解決を行う:

```rust
pub struct ModuleEnv {
    pub name: Option<String>,
    pub exports: Option<Vec<String>>,
    pub privates: Vec<String>,
    pub imports: Vec<ModuleImport>,
}

pub struct ModuleImport {
    pub module: String,
    pub alias: Option<String>,
    pub only: Option<Vec<String>>,
    pub open: bool,
}
```

名前解決の規則:

- **完全修飾アクセス**: `Math.Vec2.add` → `Math.Vec2` モジュールから `add` を検索
- **エイリアス**: `:as V` → `V.add` で参照可能
- **選択的インポート**: `:only [add sub]` → `add`, `sub` のみ直接参照
- **全公開**: `:open` → 全エクスポートを直接参照

可視性制御は `(private ...)` で包まれた宣言を `privates` リストに記録し、他モジュールからのアクセスを禁止する。

## IR リンクと Wasm 出力

複数のモジュールは IR レベルで結合される (`link_modules`)。この処理は以下の3ステップで行われる:

### 1. 関数インデックスのリベース

各モジュールの関数は独立にインデックス付けされている。リンク時には、先行モジュールの関数数を加算してインデックスをリベースする:

```
モジュール A: func 0, 1, 2 (3個)
モジュール B: func 0, 1     (2個)
リンク後:
  A: func 0, 1, 2
  B: func 3, 4        (0+3, 1+3)
```

### 2. GC 型インデックスのリベース

WasmGC の struct/array 型定義も同様にリベースが必要。各モジュールの型インデックスをグローバルな連番に変換する。

### 3. Import 関数の重複除去

WASI の `fd_write` や `proc_exit` など、複数モジュールが同じ関数をインポートする場合がある。リンカーはこれらの重複を検出し、単一のインポートにマージする。

最終的に、全モジュールのコードが**単一の Wasm モジュール**にフラット化され、1つの `.wasm` ファイルとして出力される。
