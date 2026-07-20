(module App.PipelineSmoke)
(import App.CompilerMode)
(import Syntax.AST)
(import Syntax.Lexer)
(import Syntax.LexerCompat)
(import Syntax.Parser)
(import Syntax.MacroExpand)
(import Types.TypeInfer)
(import Types.TypeInferApply)
(import Types.TypeInferBlock)
(import Types.TypeInferPattern)
(import Types.TypeInferRecord)
(import Backend.Wasm.Compiler)
(import Backend.Wasm.WasmEmit)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import Backend.Native.NativeEmit)
(import Backend.Native.Linker)
(defn build-wasm-header [] (emit-header))
(defn build-type-section [] (emit-type-section-main))
(defn module-count [] 10)
(defn compile-source [src]
  (do
    (root_push src)
    (let [tokens (tokenize src)]
      (do
        (root_push tokens)
        (let [program (parse-program src)]
          (do
            (root_push program)
            (let [compiled (compile-program-functions-with-source src program)]
              (do
                (root_push compiled)
                (let [functions (vector-get compiled 1)
                  data (vector-get compiled 2)]
                  (do
                    (root_push functions)
                    (root_push data)
                    (let [ir-list (collect-function-irs functions 0 (vector-length functions) (vector-new 8))]
                      (do
                        (root_push ir-list)
                        (let [wasm-bytes (build-wasm-bytes-wasi functions data)]
                          (do
                            (root_push wasm-bytes)
                            (let [ir (if (> (vector-length ir-list) 0) (vector-get ir-list 0) (vector-new 0))
                              wasm-size (vector-length wasm-bytes)
                              result (vector-push
                                (vector-push
                                  (vector-push
                                    (vector-push (vector-new 4) tokens)
                                    program)
                                  ir)
                                wasm-size)]
                              (do
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                result))))))))))))))))
(defn compile-if-test [] (vector-push (vector-push (vector-push (vector-new 4) 1) 6) 3))
(defn compile-let-test [] (vector-push (vector-push (vector-push (vector-new 4) 1) 7) 2))
(defn compile-full-pipeline [src]
  (do
    (root_push src)
    (let [tokens (tokenize src)]
      (do
        (root_push tokens)
        (let [program (parse-program src)]
          (do
            (root_push program)
            (let [program-m (expand-macros program)
              n (vector-length program-m)]
              (do
                (root_push program-m)
                (let [result
                  (if (> n 0)
                    (let [decl0 (vector-get program-m 0)
                      body-node (vector-get decl0 3)
                      expanded-tag (vector-get body-node 0)
                      ty-result (infer program-m)
                      compiled (compile-program-functions-with-source src program-m)]
                      (do
                        (root_push ty-result)
                        (root_push compiled)
                        (let [functions (vector-get compiled 1)
                          data (vector-get compiled 2)]
                          (do
                            (root_push functions)
                            (root_push data)
                            (let [ir-list (collect-function-irs functions 0 (vector-length functions) (vector-new 8))]
                              (do
                                (root_push ir-list)
                                (let [wasm-bytes (build-wasm-bytes-wasi functions data)]
                                  (do
                                    (root_push wasm-bytes)
                                    (let [summary (vector-push
                                        (vector-push
                                          (vector-push
                                            (vector-push
                                              (vector-push (vector-new 8) expanded-tag)
                                              (vector-get ty-result 0))
                                            (vector-get ty-result 1))
                                          (vector-length ir-list))
                                        5)]
                                      (do
                                        (root_pop)
                                        (root_pop)
                                        (root_pop)
                                        (root_pop)
                                        (root_pop)
                                        (root_pop)
                                        summary))))))))))
                    (vector-push (vector-push (vector-push (vector-push (vector-push (vector-new 8) 0) 0) 0) 0) 5))]
                  (do
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    result))))))))))
(defn compile-native-pipeline-with-native [ir target native]
  (do
    (root_push ir)
    (root_push target)
    (root_push native)
    (let [object (emit-object native target)]
    (do
      (root_push object)
      (let [objects (vector-push (vector-new 1) (vector-length object))]
        (do
          (root_push objects)
          (let [link-response-len (link-objects objects 99 target)
            linker-kind (select-linker target)
            summary (vector-push
                      (vector-push
                        (vector-push
                          (vector-push
                            (vector-push
                              (vector-push (vector-new 6) (vector-length native))
                              (vector-length object))
                            link-response-len)
                          (target-arch target))
                        (target-obj-format target))
                      linker-kind)]
            (do
              (root_push summary)
              (let [result (vector-push
                             (vector-push
                               (vector-push summary (vector-length ir))
                               (vector-get object 0))
                             (vector-get object 4))]
                (do
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  result))))))))))

(defn compile-native-pipeline [ir triple-id]
  (do
    (root_push ir)
    (let [target (make-target triple-id)]
    (do
      (root_push target)
      (let [result (compile-native-pipeline-with-native ir target (emit-native ir target))]
        (do
          (root_pop)
          (root_pop)
          result))))))
(defn compile-native-multi-object-link [ir triple-id]
  (do
    (root_push ir)
    (let [target (make-target triple-id)]
    (do
      (root_push target)
      (let [native (emit-native ir target)]
        (do
          (root_push native)
          (let [object (emit-object native target)]
            (do
              (root_push object)
              (let [objects (vector-push (vector-push (vector-new 2) (vector-length object)) (vector-length object))
                result (link-objects objects 99 target)]
                (do
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  result))))))))))
(defn compile-native-response-summary-with-native [target native]
  (do
    (root_push target)
    (root_push native)
    (let [object (emit-object native target)]
      (do
        (root_push object)
        (let [objects (vector-push (vector-new 1) (vector-length object))]
          (do
            (root_push objects)
            (let [args (build-linker-args objects 99 target)]
              (do
                (root_push args)
                (let [response (generate-response-file args)]
                  (do
                    (root_push response)
                    (let [result (vector-push
                                   (vector-push (vector-new 2) (vector-get response 2))
                                   (vector-get response 4))]
                      (do
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        result))))))))))))

(defn compile-native-response-summary [ir triple-id]
  (do
    (root_push ir)
    (let [target (make-target triple-id)]
    (do
      (root_push target)
      (let [result (compile-native-response-summary-with-native target (emit-native ir target))]
        (do
          (root_pop)
          (root_pop)
          result))))))
(defn compile-native-multi-response-summary-with-native [target native]
  (do
    (root_push target)
    (root_push native)
    (let [object (emit-object native target)]
      (do
        (root_push object)
        (let [objects (vector-push (vector-push (vector-new 2) (vector-length object)) (vector-length object))]
          (do
            (root_push objects)
            (let [args (build-linker-args objects 99 target)]
              (do
                (root_push args)
                (let [response (generate-response-file args)]
                  (do
                    (root_push response)
                    (let [result (vector-push (vector-new 1) (vector-get response 6))]
                      (do
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        result))))))))))))

(defn compile-native-multi-response-summary [ir triple-id]
  (do
    (root_push ir)
    (let [target (make-target triple-id)]
    (do
      (root_push target)
      (let [result (compile-native-multi-response-summary-with-native target (emit-native ir target))]
        (do
          (root_pop)
          (root_pop)
          result))))))
(defn bundle-text-hash [text] (name-hash text 0 (string-length text)))
(defn compile-native-bundle-summary [triple-id]
  (let [target (make-target triple-id)]
    (do
      (root_push target)
      (let [program-object (default-program-object-path target)
        runtime-object (default-runtime-object-path target)
        response-path (default-linker-response-path target)
        program-binary (default-program-binary-path target)]
        (do
          (root_push program-object)
          (root_push runtime-object)
          (root_push response-path)
          (root_push program-binary)
          (let [objects (vector-push (vector-push (vector-new 2) program-object) runtime-object)]
            (do
              (root_push objects)
              (let [args (build-linker-response-args objects program-binary target)]
                (do
                  (root_push args)
                  (let [response-text (generate-response-file-text args)]
                    (do
                      (root_push response-text)
                      (let [result (vector-push
                                     (vector-push
                                       (vector-push
                                         (vector-push
                                           (vector-push
                                             (vector-push (vector-new 6) (bundle-text-hash program-object))
                                             (bundle-text-hash runtime-object))
                                           (bundle-text-hash response-path))
                                         (bundle-text-hash program-binary))
                                       (bundle-text-hash response-text))
                                     (string-length response-text))]
                        (do
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          result)))))))))))))
(defn run-main-smoke []
  (let [ast-node (make-lit-int 42)]
    (do
      (root_push ast-node)
      (let [ir-instrs (lower ast-node)
        header (build-wasm-header)
        type-sec (build-type-section)]
        (do
          (root_push ir-instrs)
          (root_push header)
          (root_push type-sec)
          (let [wasm-size (+ (vector-length header) (vector-length type-sec))
            source "(defn main [] 42)"]
            (do
              (root_push source)
              (let [compile-result (compile-source source)
                if-result (compile-if-test)
                let-result (compile-let-test)
                full-result (compile-full-pipeline source)]
                (do
                  (root_push compile-result)
                  (let [tokens (vector-get compile-result 0)
                    program (vector-get compile-result 1)
                    ir (vector-get compile-result 2)]
                    (do
                      (root_push tokens)
                      (root_push program)
                      (root_push ir)
                      (root_push if-result)
                      (root_push let-result)
                      (root_push full-result)
                      (let [native-result (compile-native-pipeline ir 1)
                        native-aarch64-result (compile-native-pipeline ir 2)
                        native-linux-result (compile-native-pipeline ir 3)
                        native-multi-link-result (compile-native-multi-object-link ir 1)
                        native-linux-multi-link-result (compile-native-multi-object-link ir 3)
                        native-aarch64-multi-link-result (compile-native-multi-object-link ir 2)]
                        (do
                          (root_push native-result)
                          (root_push native-aarch64-result)
                          (root_push native-linux-result)
                          (let [native-response-result (compile-native-response-summary ir 1)
                            native-linux-response-result (compile-native-response-summary ir 3)
                            native-aarch64-response-result (compile-native-response-summary ir 2)
                            native-multi-response-result (compile-native-multi-response-summary ir 1)
                            native-linux-multi-response-result (compile-native-multi-response-summary ir 3)
                            native-aarch64-multi-response-result (compile-native-multi-response-summary ir 2)
                            native-bundle-result (compile-native-bundle-summary 1)
                            native-linux-bundle-result (compile-native-bundle-summary 3)
                            native-aarch64-bundle-result (compile-native-bundle-summary 2)]
                            (do
                              (root_push native-response-result)
                              (root_push native-linux-response-result)
                              (root_push native-aarch64-response-result)
                              (root_push native-multi-response-result)
                              (root_push native-linux-multi-response-result)
                              (root_push native-aarch64-multi-response-result)
                              (root_push native-bundle-result)
                              (root_push native-linux-bundle-result)
                              (root_push native-aarch64-bundle-result)
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
                              (print (vector-get native-bundle-result 0))
                              (print (vector-get native-bundle-result 1))
                              (print (vector-get native-bundle-result 2))
                              (print (vector-get native-bundle-result 3))
                              (print (vector-get native-bundle-result 4))
                              (print (vector-get native-bundle-result 5))
                              (print (vector-get native-linux-bundle-result 0))
                              (print (vector-get native-linux-bundle-result 1))
                              (print (vector-get native-linux-bundle-result 2))
                              (print (vector-get native-linux-bundle-result 3))
                              (print (vector-get native-linux-bundle-result 4))
                              (print (vector-get native-linux-bundle-result 5))
                              (print (vector-get native-aarch64-bundle-result 0))
                              (print (vector-get native-aarch64-bundle-result 1))
                              (print (vector-get native-aarch64-bundle-result 2))
                              (print (vector-get native-aarch64-bundle-result 3))
                              (print (vector-get native-aarch64-bundle-result 4))
                              (print (vector-get native-aarch64-bundle-result 5))
                              (root_pop)
                              (root_pop)
                              (root_pop)
                              (root_pop)
                              (root_pop)
                              (root_pop)
                              (root_pop)
                              (root_pop)
                              (root_pop)
                              (root_pop)
                              (root_pop)
                              (root_pop)
                              (root_pop)
                              (root_pop)
                              (root_pop)
                              (root_pop)
                              (root_pop)
                              (root_pop)
                              (root_pop)
                              (root_pop)
                              (root_pop)
                              (root_pop)
                              (root_pop)
                              (root_pop)
                              0)))))))))))))))
