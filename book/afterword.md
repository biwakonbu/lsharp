# あとがき {.unnumbered}

## 振り返り {.unnumbered}

L# コンパイラの実装は、字句解析器の最初の一行から始まり、セルフホスティングコンパイラが自分自身の一部をコンパイルできるところまで到達した。その過程で実装した主要な機能を振り返る:

- **S 式パーサー**: 50 行に満たないパーサーから始まり、メタデータ、レコード構文、GADT 構文まで拡張した
- **Hindley-Milner 型推論**: 型変数と単一化から始まり、let 多相、レコード型、トレイト制約、高カインド型まで成長させた
- **WebAssembly コード生成**: `i64.const` と `i64.add` の2命令から始まり、WasmGC struct、クロージャ、文字列ヒープまで拡張した
- **セルフホスティング**: L# で書かれた字句解析器、パーサー、型推論器、コード生成器が動作するようになった

全体で約 18,000 行の Rust コードと 2,000 行の L# コードで構成されるコンパイラが完成した。

## コンパイラ実装から得られるもの {.unnumbered}

コンパイラを実装する中で、以下のような知見が得られた:

1. **抽象化の力**: AST、IR、Wasm という3層の中間表現を導入することで、各層が独立して進化できる
2. **型の価値**: Hindley-Milner 型推論は、プログラマの負担なしに型安全性を保証する強力な仕組みである
3. **テストの重要性**: 199 個の E2E テストがなければ、リファクタリングや機能追加は不可能だった
4. **段階的な設計**: 全てを最初から設計するのではなく、動くものを作りながら段階的に拡張していく方法が有効だった

## 今後の展望 {.unnumbered}

L# にはまだ多くの可能性が残されている:

- **完全なセルフホスティング**: stage2.wasm の生成と固定点検証
- **マクロシステム**: Quote/Unquote による衛生的マクロ
- **最適化パス**: 定数畳み込み、不要コード除去、末尾呼び出し最適化
- **WASI ファイル I/O**: 完全なシステムプログラミング対応
- **パッケージマネージャー**: 依存関係管理とビルドシステム

これらの機能は、読者自身が L# に貢献する出発点にもなり得る。コンパイラは決して完成しない——それが、この分野の面白さでもある。

## 参考文献 {.unnumbered}

- Benjamin C. Pierce. *Types and Programming Languages*. MIT Press, 2002.
- Robert Nystrom. *Crafting Interpreters*. Genever Benning, 2021.
- Yaron Minsky, Anil Madhavapeddy, Jason Hickey. *Real World OCaml*. O'Reilly Media, 2013.
- Simon Peyton Jones. *The Implementation of Functional Programming Languages*. Prentice Hall, 1987.
- Andrew W. Appel. *Modern Compiler Implementation in ML*. Cambridge University Press, 2004.
- WebAssembly Specification. https://webassembly.github.io/spec/
- WASI Specification. https://wasi.dev/
- WasmGC Proposal. https://github.com/WebAssembly/gc
- Rust Programming Language. https://www.rust-lang.org/
- wasm-encoder Documentation. https://docs.rs/wasm-encoder/
- wasmtime Documentation. https://docs.wasmtime.dev/

\newpage
