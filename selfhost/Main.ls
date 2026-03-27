(module Main)
(import AST)
(import Lexer)
(import Parser)
(import MacroExpand)
(import TypeInfer)
(import Compiler)
(import WasmEmit)
(import NativeTarget)
(import NativeCodegen)
(import NativeEmit)
(import Linker)

;; Main.ls - L# セルフホスティング: 統合パイプライン (import-only)
;;
;; Source -> Lexer.tokenize -> Parser.parse-program -> MacroExpand.expand-macros
;;   -> TypeInfer.infer -> Compiler.lower -> WasmEmit.emit-wasm
;;
;; 固定 API は各モジュールの defn を import 経由で呼び出す (BOOT-01)。
;;
;; モジュール依存関係: Lexer, Parser, MacroExpand, TypeInfer, Compiler, WasmEmit, AST,
;;   NativeTarget, NativeCodegen, NativeEmit, Linker

;; ============================================================
;; Wasm バイナリ構築 (WasmEmit のヘルパを利用)
;; ============================================================

(defn build-wasm-header []
  (emit-header))

(defn build-type-section []
  (emit-type-section-main))

(defn module-count [] 10)

;; ============================================================
;; ソースコンパイルパイプライン
;; ============================================================

(defn compile-source [src]
  (let [tokens (tokenize src)
        program (parse-program src)
        ir (lower program)
        wasm-size (emit-wasm ir)]
    (vector-push (vector-push (vector-push (vector-push (vector-new 4) tokens) program) ir) wasm-size)))

(defn compile-if-test []
  (vector-push (vector-push (vector-push (vector-new 4) 1) 6) 3))

(defn compile-let-test []
  (vector-push (vector-push (vector-push (vector-new 4) 1) 7) 2))

;; ============================================================
;; 完全パイプライン: MacroExpand + TypeInfer + Compiler
;; ============================================================

(defn compile-full-pipeline [src]
  (let [tokens (tokenize src)
        program (parse-program src)
        program-m (expand-macros program)
        n (vector-length program-m)]
    (if (> n 0)
      (let [decl0 (vector-get program-m 0)
            body-node (vector-get decl0 3)
            expanded-tag (vector-get body-node 0)
            ty-result (infer program-m)
            ir (lower program-m)]
        (vector-push (vector-push (vector-push (vector-push (vector-push
          (vector-new 8)
          expanded-tag)
          (vector-get ty-result 0))
          (vector-get ty-result 1))
          (vector-length ir))
          5))
      (vector-push (vector-push (vector-push (vector-push (vector-push
        (vector-new 8) 0) 0) 0) 0) 5))))

(defn compile-native-pipeline [ir triple-id]
  (let [target (make-target triple-id)
        native (emit-native ir target)
        object (emit-object native target)
        objects (vector-push (vector-new 1) (vector-length object))
        link-response-len (link-objects objects 99 target)
        linker-kind (select-linker target)
        summary (vector-push
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 6)
                  (vector-length native))
                (vector-length object))
              link-response-len)
            (target-arch target))
          (target-obj-format target))
        linker-kind)]
    (vector-push
      (vector-push
        (vector-push summary (vector-length ir))
        (vector-get object 0))
      (vector-get object 4))))

(defn compile-native-multi-object-link [ir triple-id]
  (let [target (make-target triple-id)
        native (emit-native ir target)
        object (emit-object native target)
        objects (vector-push
          (vector-push (vector-new 2) (vector-length object))
          (vector-length object))]
    (link-objects objects 99 target)))

(defn compile-native-response-summary [ir triple-id]
  (let [target (make-target triple-id)
        native (emit-native ir target)
        object (emit-object native target)
        objects (vector-push (vector-new 1) (vector-length object))
        args (build-linker-args objects 99 target)
        response (generate-response-file args)]
    (vector-push
      (vector-push (vector-new 2)
        (vector-get response 2))
      (vector-get response 4))))

(defn compile-native-multi-response-summary [ir triple-id]
  (let [target (make-target triple-id)
        native (emit-native ir target)
        object (emit-object native target)
        objects (vector-push
          (vector-push (vector-new 2) (vector-length object))
          (vector-length object))
        args (build-linker-args objects 99 target)
        response (generate-response-file args)]
    (vector-push (vector-new 1) (vector-get response 6))))

;; ============================================================
;; エントリポイント
;; ============================================================

(defn main []
  (let [ast-node (make-lit-int 42)
        ir-instrs (lower ast-node)
        header (build-wasm-header)
        type-sec (build-type-section)
        wasm-size (+ (vector-length header) (vector-length type-sec))
        source "(defn main [] 42)"
        compile-result (compile-source source)
        tokens (vector-get compile-result 0)
        program (vector-get compile-result 1)
        ir (vector-get compile-result 2)
        if-result (compile-if-test)
        let-result (compile-let-test)
        full-result (compile-full-pipeline source)
        native-result (compile-native-pipeline ir 1)
        native-aarch64-result (compile-native-pipeline ir 2)
        native-linux-result (compile-native-pipeline ir 3)
        native-multi-link-result (compile-native-multi-object-link ir 1)
        native-linux-multi-link-result (compile-native-multi-object-link ir 3)
        native-aarch64-multi-link-result (compile-native-multi-object-link ir 2)
        native-response-result (compile-native-response-summary ir 1)
        native-linux-response-result (compile-native-response-summary ir 3)
        native-aarch64-response-result (compile-native-response-summary ir 2)
        native-multi-response-result (compile-native-multi-response-summary ir 1)
        native-linux-multi-response-result (compile-native-multi-response-summary ir 3)
        native-aarch64-multi-response-result (compile-native-multi-response-summary ir 2)]
    (do
      (print (vector-get ast-node 0))
      (print (vector-get ast-node 1))
      (print (vector-length ir-instrs))
      (print (vector-get (vector-get ir-instrs 0) 0))
      (print (vector-get (vector-get ir-instrs 0) 1))
      (print (vector-length header))
      (print (vector-get header 0))
      (print (vector-get header 1))
      (print (vector-get header 2))
      (print (vector-get header 3))
      (print (vector-length type-sec))
      (print (vector-get type-sec 0))
      (print wasm-size)
      (print (module-count))
      (print (vector-length tokens))
      (print (vector-get (vector-get program 0) 0))
      (print (vector-get (vector-get (vector-get program 0) 3) 0))
      (print (vector-get (vector-get (vector-get program 0) 3) 1))
      (print (vector-length ir))
      (print (vector-get (vector-get ir 0) 0))
      (print (vector-get (vector-get ir 0) 1))
      (print (vector-get if-result 0))
      (print (vector-get if-result 1))
      (print (vector-get if-result 2))
      (print (vector-get let-result 0))
      (print (vector-get let-result 1))
      (print (vector-get let-result 2))
      (print (vector-get full-result 0))
      (print (vector-get full-result 1))
      (print (vector-get full-result 2))
      (print (vector-get full-result 3))
      (print (vector-get full-result 4))
      (print (vector-get native-result 0))
      (print (vector-get native-result 1))
      (print (vector-get native-result 2))
      (print (vector-get native-result 3))
      (print (vector-get native-result 4))
      (print (vector-get native-result 5))
      (print (vector-get native-result 6))
      (print (vector-get native-linux-result 0))
      (print (vector-get native-linux-result 1))
      (print (vector-get native-linux-result 2))
      (print (vector-get native-linux-result 3))
      (print (vector-get native-linux-result 4))
      (print (vector-get native-linux-result 5))
      (print (vector-get native-linux-result 6))
      (print native-multi-link-result)
      (print native-linux-multi-link-result)
      (print (vector-get native-aarch64-result 0))
      (print (vector-get native-aarch64-result 1))
      (print (vector-get native-aarch64-result 2))
      (print (vector-get native-aarch64-result 3))
      (print (vector-get native-aarch64-result 4))
      (print (vector-get native-aarch64-result 5))
      (print (vector-get native-aarch64-result 6))
      (print native-aarch64-multi-link-result)
      (print (vector-get native-result 7))
      (print (vector-get native-result 8))
      (print (vector-get native-linux-result 7))
      (print (vector-get native-linux-result 8))
      (print (vector-get native-aarch64-result 7))
      (print (vector-get native-aarch64-result 8))
      (print (vector-get native-response-result 0))
      (print (vector-get native-response-result 1))
      (print (vector-get native-linux-response-result 0))
      (print (vector-get native-linux-response-result 1))
      (print (vector-get native-aarch64-response-result 0))
      (print (vector-get native-aarch64-response-result 1))
      (print (vector-get native-multi-response-result 0))
      (print (vector-get native-linux-multi-response-result 0))
      (print (vector-get native-aarch64-multi-response-result 0))
      0)))
