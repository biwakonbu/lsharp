# L# Language Guide

## 概要

L# は S 式構文と Hindley-Milner 型推論を持つ関数型言語です。WebAssembly (WASI) と Native をターゲットにします。

## 構文早見表

- 関数定義: `(defn name [params] body)`
- ラムダ: `(fn [params] body)`
- let 束縛: `(let [x 1] (+ x 1))`
- 条件分岐: `(if cond then else)`
- パターンマッチ: `(match expr [pattern body] ...)`
- ADT: `(type (Option a) (Some a) None)`
- Record: `(type Point (record (: x Int) (: y Int)))`
- Module: `(module Name)` / `(import Name)`

## 型システム

- プリミティブ: `Int`, `Float`, `String`, `Bool`, `Unit`
- 型推論: Hindley-Milner
- 関数型: `A -> B`
- 多相: `(Option a)`, `(Result a e)`

## stdlib

- `Core`, `List`, `Map`, `Set`, `Vector`
- `String`, `Char`, `IO`, `Json`, `Path`, `Debug`

## MCP ツール

- `lsharp_hover`
- `lsharp_completion`
- `lsharp_check`
- `lsharp_package_api`
- `lsharp_stdlib_api`
