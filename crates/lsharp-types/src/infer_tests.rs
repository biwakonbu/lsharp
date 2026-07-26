#[cfg(test)]
mod tests {
    use super::*;

    fn infer(input: &str) -> Result<Vec<(String, TypeScheme)>, TypeError> {
        let program = lsharp_syntax::parse(input).unwrap();
        let mut infer = Infer::new();
        infer.infer_program(&program)
    }

    fn infer_one(input: &str) -> String {
        let results = infer(input).unwrap();
        let (_, scheme) = &results[0];
        scheme.to_string()
    }

    #[test]
    fn test_identity() {
        let result = infer_one("(defn id [x] x)");
        assert!(result.starts_with("forall"));
        assert!(result.contains("->"));
    }

    #[test]
    fn test_add() {
        let result = infer_one("(defn add [x y] (+ x y))");
        assert_eq!(result, "(Int, Int) -> Int");
    }

    #[test]
    fn test_builtin_env_keeps_core_operator_scheme() {
        let mut infer = Infer::new();
        let env = infer.builtin_env();
        assert_eq!(
            env.get("+").map(ToString::to_string),
            Some("(Int, Int) -> Int".to_string())
        );
    }

    #[test]
    fn test_bool_expr() {
        let result = infer_one("(defn is-zero [n] (== n 0))");
        assert_eq!(result, "(Int) -> Bool");
    }

    #[test]
    fn test_if_expr() {
        let result = infer_one("(defn abs [n] (if (< n 0) (- 0 n) n))");
        assert_eq!(result, "(Int) -> Int");
    }

    #[test]
    fn test_let_expr() {
        let result = infer_one("(defn f [] (let [x 42] x))");
        assert_eq!(result, "() -> Int");
    }

    #[test]
    fn test_lambda() {
        let result = infer_one("(defn apply [f x] (f x))");
        assert!(result.starts_with("forall"));
    }

    #[test]
    fn test_recursive() {
        let result = infer_one("(defn fib [n] (if (<= n 1) n (+ (fib (- n 1)) (fib (- n 2)))))");
        assert_eq!(result, "(Int) -> Int");
    }

    #[test]
    fn test_type_error_mismatch() {
        let result = infer("(defn bad [] (+ 1 true))");
        assert!(result.is_err());
    }

    #[test]
    fn test_undefined_var() {
        let result = infer("(defn bad [] x)");
        assert!(result.is_err());
    }

    #[test]
    fn test_str_is_not_treated_as_builtin() {
        let result = infer("(defn bad [] (str 42))");
        match result {
            Err(TypeError::UndefinedVar { name, .. }) => assert_eq!(name, "str"),
            other => panic!("expected UndefinedVar for str, got {other:?}"),
        }
    }

    #[test]
    fn test_adt_basic() {
        let results = infer(
            "(type (Option a) (Some a) None)
             (defn get-or-zero [opt] (match opt [(Some x) x] [None 0]))",
        )
        .unwrap();
        let (name, scheme) = &results[0];
        assert_eq!(name, "get-or-zero");
        assert!(scheme.to_string().contains("Int"));
    }

    #[test]
    fn test_do_expr() {
        let result = infer_one("(defn main [] (do (print 1) (print 2)))");
        assert_eq!(result, "() -> Unit");
    }

    #[test]
    fn test_ref_new_builtin_returns_ref_type() {
        let result = infer_one("(defn make-cell [] (ref-new 1))");
        assert_eq!(result, "() -> (Ref Int)");
    }

    #[test]
    fn test_ref_set_builtin_accepts_ref_type() {
        let result = infer_one("(defn set-cell [cell] (ref-set cell 1))");
        assert_eq!(result, "((Ref Int)) -> Unit");
    }

    #[test]
    fn test_vector_builtins_use_vector_type() {
        let make_vec = infer_one("(defn make-vec [] (vector-new 0))");
        assert_eq!(make_vec, "() -> Vector");

        let grow = infer_one("(defn grow [v] (vector-push v 1))");
        assert_eq!(grow, "(Vector) -> Vector");
    }

    #[test]
    fn test_write_file_bytes_builtin_accepts_vector_type() {
        let result = infer_one("(defn write-bytes [bytes] (write-file-bytes \"out.wasm\" bytes))");
        assert_eq!(result, "(Vector) -> Int");
    }

    #[test]
    fn test_map_builtins_use_map_type() {
        let make_map = infer_one("(defn make-map [] (map-new))");
        assert_eq!(make_map, "() -> Map");

        let size = infer_one("(defn size [m] (map-size m))");
        assert_eq!(size, "(Map) -> Int");
    }

    #[test]
    fn test_zero_sentinel_remains_compatible_with_vector_type() {
        let result = infer("(defn maybe-vec [] (if (= 0 0) (vector-new 0) 0))");
        assert!(result.is_ok());
    }

    #[test]
    fn test_zero_sentinel_remains_compatible_with_map_type() {
        let result = infer("(defn maybe-map [] (if (= 0 0) (map-new) 0))");
        assert!(result.is_ok());
    }

    #[test]
    fn test_zero_sentinel_remains_compatible_with_ref_type() {
        let result = infer("(defn maybe-ref [] (if (= 0 0) (ref-new 1) 0))");
        assert!(result.is_ok());
    }

    #[test]
    fn test_root_push_accepts_string_type() {
        let result = infer_one("(defn keep [] (root_push \"hello\"))");
        assert_eq!(result, "() -> Int");
    }

    #[test]
    fn test_root_set_accepts_string_type() {
        let result = infer_one("(defn refresh [slot] (root_set slot \"hello\"))");
        assert_eq!(result, "(Int) -> Int");
    }

    #[test]
    fn test_root_pop_can_be_constrained_to_string_type() {
        let result = infer_one("(defn read-head [] (string-length (root_pop)))");
        assert_eq!(result, "() -> Int");
    }

    #[test]
    fn test_type_annotation() {
        let result = infer_one("(defn add [(: x Int) (: y Int)] : Int (+ x y))");
        assert_eq!(result, "(Int, Int) -> Int");
    }

    #[test]
    fn test_multiple_scoped_type_vars_in_type_annotation() {
        let result = infer_one("(defn choose-first [(: x a) (: y b)] : a x)");
        assert!(
            result.starts_with("forall"),
            "expected polymorphic scheme: {result}"
        );
        assert!(result.contains("->"), "expected function type: {result}");
        let calls = infer(
            "(defn choose-first [(: x a) (: y b)] : a x)
             (defn main []
               (do (print (choose-first 42 true))
                   (print (choose-first true 42))
                   0))",
        );
        assert!(
            calls.is_ok(),
            "independent call-site instantiation should type-check: {calls:?}"
        );
    }

    // --- レコード型テスト ---

    #[test]
    fn test_record_type_inference() {
        let results = infer(
            "(type Point (record (: x Int) (: y Int)))
             (defn make-point [] {Point x 1 y 2})",
        )
        .unwrap();
        let (name, scheme) = &results[0];
        assert_eq!(name, "make-point");
        assert!(scheme.to_string().contains("Point"));
    }

    #[test]
    fn test_record_field_access() {
        let results = infer(
            "(type Point (record (: x Int) (: y Int)))
             (defn get-x [p] (Point.x p))",
        )
        .unwrap();
        let (name, _scheme) = &results[0];
        assert_eq!(name, "get-x");
    }

    #[test]
    fn test_nested_record_field_type_is_inferred() {
        let results = infer(
            "(type Inner (record (: x Int)))
             (type Outer (record (: inner Inner)))
             (defn read-inner [o]
               (match o
                 [{Outer inner {Inner x x}} x]
                 [_ 0]))",
        );
        assert!(
            results.is_ok(),
            "nested record field type should unify with its record pattern: {results:?}"
        );
    }

    #[test]
    fn test_type_alias() {
        let results = infer(
            "(type-alias Str String)
             (defn hello [] (: \"world\" Str))",
        );
        assert!(results.is_ok());
    }

    #[test]
    fn test_parametric_type_alias_expansion() {
        // (type-alias (Callback a b) (-> a b)) は 引数型 -> 戻り値型 の関数型
        let results = infer(
            "(type-alias (Pair a b) (-> a b))
             (defn apply-pair [f] (: f (Pair Int Bool)))",
        );
        assert!(results.is_ok());
    }

    #[test]
    fn test_simple_parametric_alias() {
        let results = infer(
            "(type-alias (Id a) a)
             (defn identity [x] (: x (Id Int)))",
        );
        assert!(results.is_ok());
    }

    // --- 再帰エイリアス検出テスト ---

    #[test]
    fn test_recursive_alias_detection() {
        let result = infer("(type-alias Rec Rec)");
        assert!(result.is_err());
        if let Err(TypeError::RecursiveAlias { name, .. }) = &result {
            assert_eq!(name, "Rec");
        }
    }

    // --- 制約付き型テスト ---

    #[test]
    fn test_type_constrained_registration() {
        let results = infer(
            "(type-constrained Natural Int :constraints [(>= 0)])
             (defn make-nat [] (Natural.new 42))",
        );
        assert!(results.is_ok());
    }

    #[test]
    fn test_type_constrained_valid() {
        let results = infer(
            "(type-constrained Natural Int :constraints [(>= 0)])
             (defn is-valid [] (Natural.valid? 42))",
        );
        assert!(results.is_ok());
        let (_, scheme) = &results.unwrap()[0];
        assert!(scheme.to_string().contains("Bool"));
    }

    // --- トレイトテスト ---

    #[test]
    fn test_trait_registration() {
        let results = infer(
            "(trait (Show a) (defn show [self] : String))
             (defn main [] (print 42))",
        );
        assert!(results.is_ok());
    }

    #[test]
    fn test_impl_registration() {
        let results = infer(
            "(trait (Show a) (defn show [self] : String))
             (impl (Show Int) (defn show [self] (int-to-string self)))
             (defn main [] (print 42))",
        );
        assert!(results.is_ok());
    }

    // --- モジュール環境テスト ---

    #[test]
    fn test_module_declaration() {
        let program = lsharp_syntax::parse("(module MyModule) (defn main [] 42)").unwrap();
        let mut infer_ctx = Infer::new();
        let results = infer_ctx.infer_program(&program);
        assert!(results.is_ok());
        assert_eq!(infer_ctx.module_env.name, Some("MyModule".to_string()));
    }

    #[test]
    fn test_import_declaration() {
        let program =
            lsharp_syntax::parse("(module Main) (import MyModule :as M) (defn main [] 42)")
                .unwrap();
        let mut infer_ctx = Infer::new();
        let results = infer_ctx.infer_program(&program);
        assert!(results.is_ok());
        assert_eq!(infer_ctx.module_env.imports.len(), 1);
        assert_eq!(infer_ctx.module_env.imports[0].module, "MyModule");
        assert_eq!(infer_ctx.module_env.imports[0].alias, Some("M".to_string()));
    }

    #[test]
    fn test_import_only() {
        let program =
            lsharp_syntax::parse("(module Main) (import Utils :only [helper]) (defn main [] 42)")
                .unwrap();
        let mut infer_ctx = Infer::new();
        let results = infer_ctx.infer_program(&program);
        assert!(results.is_ok());
        assert_eq!(infer_ctx.module_env.imports.len(), 1);
        assert_eq!(
            infer_ctx.module_env.imports[0].only,
            Some(vec!["helper".to_string()])
        );
    }

    #[test]
    fn test_import_open() {
        let program =
            lsharp_syntax::parse("(module Main) (import Utils :open) (defn main [] 42)").unwrap();
        let mut infer_ctx = Infer::new();
        let results = infer_ctx.infer_program(&program);
        assert!(results.is_ok());
        assert!(infer_ctx.module_env.imports[0].open);
    }
}

#[cfg(test)]
mod private_tests {
    use super::*;

    #[test]
    fn test_private_defn_type_inference() {
        // private 内の defn も正しく型推論される
        let program = lsharp_syntax::parse(
            "(module MyModule) (private (defn helper [x] (+ x 1))) (defn main [] (helper 42))",
        )
        .unwrap();
        let mut infer_ctx = Infer::new();
        let results = infer_ctx.infer_program(&program).unwrap();

        // helper も main も型推論結果に含まれる
        let helper_result = results.iter().find(|(n, _)| n == "helper");
        assert!(helper_result.is_some());

        let main_result = results.iter().find(|(n, _)| n == "main");
        assert!(main_result.is_some());

        // helper が privates に記録される
        assert!(
            infer_ctx
                .module_env
                .privates
                .contains(&"helper".to_string())
        );
    }

    #[test]
    fn test_private_not_in_public() {
        // private でない関数は privates に記録されない
        let program =
            lsharp_syntax::parse("(module MyModule) (defn public_fn [x] (+ x 1))").unwrap();
        let mut infer_ctx = Infer::new();
        let _results = infer_ctx.infer_program(&program).unwrap();
        assert!(
            !infer_ctx
                .module_env
                .privates
                .contains(&"public_fn".to_string())
        );
    }

    #[test]
    fn test_multiple_private_declarations() {
        let program = lsharp_syntax::parse(
            "(module M) (private (defn a [] 1)) (private (defn b [] 2)) (defn c [] (+ (a) (b)))",
        )
        .unwrap();
        let mut infer_ctx = Infer::new();
        let results = infer_ctx.infer_program(&program).unwrap();
        assert_eq!(results.len(), 3);
        assert!(infer_ctx.module_env.privates.contains(&"a".to_string()));
        assert!(infer_ctx.module_env.privates.contains(&"b".to_string()));
        assert!(!infer_ctx.module_env.privates.contains(&"c".to_string()));
    }
}

#[cfg(test)]
mod nested_module_infer_tests {
    use super::*;

    fn infer_nested(input: &str) -> (Vec<(String, TypeScheme)>, Infer) {
        let program = lsharp_syntax::parse(input).unwrap();
        let mut infer_ctx = Infer::new();
        let results = infer_ctx.infer_program(&program).unwrap();
        (results, infer_ctx)
    }

    #[test]
    fn test_nested_module_function_qualified_name() {
        let (results, infer_ctx) = infer_nested("(module Utils (defn helper [x] (+ x 1)))");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "Utils.helper");
        assert_eq!(infer_ctx.module_env.name, Some("Utils".to_string()));
    }

    #[test]
    fn test_nested_module_multiple_functions() {
        let (results, _) = infer_nested(
            "(module Math
              (defn add [x y] (+ x y))
              (defn sub [x y] (- x y)))",
        );
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, "Math.add");
        assert_eq!(results[1].0, "Math.sub");
    }

    #[test]
    fn test_deeply_nested_module() {
        let (results, _) = infer_nested(
            "(module App
              (module Sub
                (defn inner [] 42)))",
        );
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "App.Sub.inner");
    }

    #[test]
    fn test_nested_module_with_top_level() {
        let (results, _) = infer_nested(
            "(module Utils (defn helper [x] (+ x 1)))
             (defn main [] (Utils.helper 10))",
        );
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, "Utils.helper");
        assert_eq!(results[1].0, "main");
    }

    #[test]
    fn test_nested_module_private_tracking() {
        let (_, infer_ctx) = infer_nested(
            "(module Utils
              (private (defn secret [] 42))
              (defn public_fn [] 0))",
        );
        assert!(
            infer_ctx
                .module_env
                .privates
                .contains(&"Utils.secret".to_string())
        );
        assert!(
            !infer_ctx
                .module_env
                .privates
                .contains(&"Utils.public_fn".to_string())
        );
    }
}

#[cfg(test)]
mod trait_default_tests {
    use super::*;

    #[test]
    fn test_trait_with_default_impl() {
        // デフォルト実装を持つトレイトメソッド
        let program = lsharp_syntax::parse(
            "(trait (Describable a) (defn describe [self] 0))
             (impl (Describable Int) )
             (defn main [] 42)",
        )
        .unwrap();
        let mut infer_ctx = Infer::new();
        let results = infer_ctx.infer_program(&program);
        assert!(results.is_ok());
    }

    #[test]
    fn test_trait_default_impl_cached() {
        // デフォルト実装がキャッシュされていることを確認
        let program = lsharp_syntax::parse(
            "(trait (Describable a) (defn describe [self] 0))
             (defn main [] 42)",
        )
        .unwrap();
        let mut infer_ctx = Infer::new();
        let _results = infer_ctx.infer_program(&program).unwrap();

        assert!(
            infer_ctx
                .default_impls
                .contains_key(&("Describable".to_string(), "describe".to_string()))
        );
    }

    #[test]
    fn test_impl_with_override() {
        // impl でメソッドをオーバーライドした場合はデフォルト実装は使われない
        let program = lsharp_syntax::parse(
            "(trait (Show a) (defn show [self] 0))
             (impl (Show Int) (defn show [self] (+ self 1)))
             (defn main [] 42)",
        )
        .unwrap();
        let mut infer_ctx = Infer::new();
        let results = infer_ctx.infer_program(&program);
        assert!(results.is_ok());
    }

    #[test]
    fn test_trait_method_without_default_and_without_impl() {
        // デフォルト実装もimplメソッドもない場合（メソッドシグネチャのみ）
        let program = lsharp_syntax::parse(
            "(trait (Show a) (defn show [self] : Int))
             (impl (Show Int) )
             (defn main [] 42)",
        )
        .unwrap();
        let mut infer_ctx = Infer::new();
        // デフォルト実装がないのでメソッドは impl に追加されない
        // エラーにはならない（将来的にエラーにすべきだが現時点では許容）
        let results = infer_ctx.infer_program(&program);
        assert!(results.is_ok());
    }
}

#[cfg(test)]
mod parametric_record_tests {
    use super::*;

    fn infer(input: &str) -> Result<Vec<(String, TypeScheme)>, TypeError> {
        let program = lsharp_syntax::parse(input).unwrap();
        let mut infer_ctx = Infer::new();
        infer_ctx.infer_program(&program)
    }

    #[test]
    fn test_parametric_record_def() {
        // パラメトリックレコード型の定義と構築
        let results = infer(
            "(type (Pair a b) (record (: fst a) (: snd b)))
             (defn make-pair [] {Pair fst 1 snd 2})",
        );
        assert!(results.is_ok());
    }

    #[test]
    fn test_parametric_record_polymorphic_usage() {
        // 異なる型で同じパラメトリックレコード型を使用
        let results = infer(
            "(type (Pair a b) (record (: fst a) (: snd b)))
             (defn int-pair [] {Pair fst 1 snd 2})
             (defn mixed-pair [] {Pair fst 1 snd true})",
        );
        assert!(results.is_ok());
        let res = results.unwrap();
        // int-pair と mixed-pair の2つの定義
        assert_eq!(res.len(), 2);
    }

    #[test]
    fn test_parametric_record_field_access() {
        // パラメトリックレコード型のフィールドアクセス
        let results = infer(
            "(type (Pair a b) (record (: fst a) (: snd b)))
             (defn get-fst [p] (Pair.fst p))",
        );
        assert!(results.is_ok());
    }

    #[test]
    fn test_parametric_record_identity() {
        // 単一型パラメータのレコード型
        let results = infer(
            "(type (Box a) (record (: value a)))
             (defn box-int [] {Box value 42})
             (defn unbox [b] (Box.value b))",
        );
        assert!(results.is_ok());
    }
}

#[cfg(test)]
mod alias_hint_tests {
    use super::*;

    fn infer(input: &str) -> Result<Vec<(String, TypeScheme)>, TypeError> {
        let program = lsharp_syntax::parse(input).unwrap();
        let mut infer_ctx = Infer::new();
        infer_ctx.infer_program(&program)
    }

    #[test]
    fn test_mismatch_with_alias_name() {
        // 型エイリアス使用時の型不一致エラーにエイリアス名が含まれる
        let result = infer(
            "(type-alias Str String)
             (defn bad [] (: 42 Str))",
        );
        assert!(result.is_err());
        let err = result.unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("Str"),
            "エラーメッセージにエイリアス名 'Str' が含まれるべき: {msg}"
        );
    }

    #[test]
    fn test_mismatch_without_alias() {
        // エイリアスを使わない場合は通常の Mismatch エラー
        let result = infer("(defn bad [] (: 42 String))");
        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            TypeError::Mismatch { .. } => {}
            _ => panic!("通常の Mismatch が期待される: {:?}", err),
        }
    }
}

#[cfg(test)]
mod fqn_tests {
    use super::*;

    #[test]
    fn test_resolve_qualified_name_with_alias() {
        let mut infer_ctx = Infer::new();
        infer_ctx.module_env.imports.push(ModuleImport {
            module: "Math".to_string(),
            alias: Some("M".to_string()),
            only: None,
            open: false,
        });
        let result = infer_ctx.resolve_qualified_name("M", "add");
        assert_eq!(result, Some("Math.add".to_string()));
    }

    #[test]
    fn test_resolve_qualified_name_direct_module() {
        let mut infer_ctx = Infer::new();
        infer_ctx.module_env.imports.push(ModuleImport {
            module: "Math".to_string(),
            alias: None,
            only: None,
            open: false,
        });
        let result = infer_ctx.resolve_qualified_name("Math", "add");
        assert_eq!(result, Some("Math.add".to_string()));
    }

    #[test]
    fn test_resolve_qualified_name_selective_import() {
        let mut infer_ctx = Infer::new();
        infer_ctx.module_env.imports.push(ModuleImport {
            module: "Math".to_string(),
            alias: Some("M".to_string()),
            only: Some(vec!["add".to_string()]),
            open: false,
        });
        // 許可されたシンボル
        assert_eq!(
            infer_ctx.resolve_qualified_name("M", "add"),
            Some("Math.add".to_string())
        );
        // 許可されていないシンボル
        assert_eq!(infer_ctx.resolve_qualified_name("M", "sub"), None);
    }

    #[test]
    fn test_resolve_qualified_name_no_match() {
        let infer_ctx = Infer::new();
        let result = infer_ctx.resolve_qualified_name("Unknown", "func");
        assert_eq!(result, None);
    }
}

#[cfg(test)]
mod kind_tests {
    use super::*;

    fn infer_with_kinds(input: &str) -> (Vec<(String, TypeScheme)>, HashMap<String, Kind>) {
        let program = lsharp_syntax::parse(input).unwrap();
        let mut infer = Infer::new();
        let results = infer.infer_program(&program).unwrap();
        (results, infer.kind_env)
    }

    #[test]
    fn test_kind_builtin_types() {
        let (_, kinds) = infer_with_kinds("(defn main [] 0)");
        assert_eq!(kinds.get("Int"), Some(&Kind::star()));
        assert_eq!(kinds.get("Float"), Some(&Kind::star()));
        assert_eq!(kinds.get("Bool"), Some(&Kind::star()));
        assert_eq!(kinds.get("String"), Some(&Kind::star()));
    }

    #[test]
    fn test_kind_adt_no_params() {
        let (_, kinds) = infer_with_kinds(
            "(type Color Red Green Blue)
             (defn main [] 0)",
        );
        assert_eq!(kinds.get("Color"), Some(&Kind::star()));
    }

    #[test]
    fn test_kind_adt_one_param() {
        let (_, kinds) = infer_with_kinds(
            "(type (Maybe a) (Just a) Nothing)
             (defn main [] 0)",
        );
        assert_eq!(kinds.get("Maybe"), Some(&Kind::unary()));
    }

    #[test]
    fn test_kind_adt_two_params() {
        let (_, kinds) = infer_with_kinds(
            "(type (Either a b) (Left a) (Right b))
             (defn main [] 0)",
        );
        assert_eq!(kinds.get("Either"), Some(&Kind::binary()));
    }

    #[test]
    fn test_kind_record() {
        let (_, kinds) = infer_with_kinds(
            "(type Point (record (: x Int) (: y Int)))
             (defn main [] 0)",
        );
        assert_eq!(kinds.get("Point"), Some(&Kind::star()));
    }

    #[test]
    fn test_kind_parametric_record() {
        let (_, kinds) = infer_with_kinds(
            "(type (Pair a b) (record (: fst a) (: snd b)))
             (defn main [] 0)",
        );
        assert_eq!(kinds.get("Pair"), Some(&Kind::binary()));
    }

    #[test]
    fn test_kind_functor_trait() {
        let (_, kinds) = infer_with_kinds("(defn main [] 0)");
        // Functor は * -> * の Kind を持つ
        assert_eq!(kinds.get("Functor"), Some(&Kind::unary()));
    }

    #[test]
    fn test_kind_monad_trait() {
        let (_, kinds) = infer_with_kinds("(defn main [] 0)");
        // Monad は * -> * の Kind を持つ
        assert_eq!(kinds.get("Monad"), Some(&Kind::unary()));
    }

    #[test]
    fn test_functor_trait_registered() {
        let program = lsharp_syntax::parse("(defn main [] 0)").unwrap();
        let mut infer = Infer::new();
        let _ = infer.infer_program(&program).unwrap();
        // Functor トレイトがレジストリに登録されている
        assert!(infer.trait_registry.contains_key("Functor"));
        let functor = &infer.trait_registry["Functor"];
        assert_eq!(functor.methods.len(), 1);
        assert_eq!(functor.methods[0].0, "fmap");
    }

    #[test]
    fn test_monad_trait_registered() {
        let program = lsharp_syntax::parse("(defn main [] 0)").unwrap();
        let mut infer = Infer::new();
        let _ = infer.infer_program(&program).unwrap();
        // Monad トレイトがレジストリに登録されている
        assert!(infer.trait_registry.contains_key("Monad"));
        let monad = &infer.trait_registry["Monad"];
        assert_eq!(monad.methods.len(), 2);
        let method_names: Vec<&str> = monad.methods.iter().map(|(n, _)| n.as_str()).collect();
        assert!(method_names.contains(&"bind"));
        assert!(method_names.contains(&"pure"));
    }

    // --- Computation Expression テスト (NC-13) ---

    #[test]
    fn test_computation_builder_registration() {
        // computation-builder が正しく登録されること
        let source = r#"
            (computation-builder maybe maybe-bind maybe-return)
            (defn main [] 0)
        "#;
        let program = lsharp_syntax::parse(source).unwrap();
        let mut infer = Infer::new();
        let _ = infer.infer_program(&program).unwrap();
        assert!(infer.computation_builders.contains_key("maybe"));
        let (bind, ret) = &infer.computation_builders["maybe"];
        assert_eq!(bind, "maybe-bind");
        assert_eq!(ret, "maybe-return");
    }

    #[test]
    fn test_computation_return_only_type_checks() {
        // return のみの computation expression が型チェックを通ること
        let source = r#"
            (computation-builder maybe maybe-bind maybe-return)
            (defn maybe-return [x] x)
            (defn maybe-bind [m f] (f m))
            (defn main [] (computation maybe (return 42)))
        "#;
        let program = lsharp_syntax::parse(source).unwrap();
        let mut infer = Infer::new();
        let result = infer.infer_program(&program);
        assert!(
            result.is_ok(),
            "computation return should type check: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_computation_let_bang_type_checks() {
        // let! を使った computation expression が型チェックを通ること
        let source = r#"
            (computation-builder maybe maybe-bind maybe-return)
            (defn maybe-return [x] x)
            (defn maybe-bind [m f] (f m))
            (defn main []
                (computation maybe
                    (let! x 10)
                    (return (+ x 1))))
        "#;
        let program = lsharp_syntax::parse(source).unwrap();
        let mut infer = Infer::new();
        let result = infer.infer_program(&program);
        assert!(
            result.is_ok(),
            "computation let! should type check: {:?}",
            result.err()
        );
    }
}

#[cfg(test)]
mod unify_property_tests {
    use super::*;
    use proptest::prelude::*;

    fn arb_type() -> impl Strategy<Value = Type> {
        let leaves = prop_oneof![
            Just(Type::int()),
            Just(Type::bool()),
            Just(Type::string()),
            (0u32..8).prop_map(Type::Var),
        ];

        leaves.prop_recursive(3, 32, 4, |inner| {
            prop_oneof![
                (prop::collection::vec(inner.clone(), 0..3), inner.clone())
                    .prop_map(|(params, ret)| Type::Fun(params, Box::new(ret))),
                prop::collection::vec(inner.clone(), 0..3)
                    .prop_map(|args| Type::App("Box".to_string(), args)),
                prop::collection::vec((Just("value".to_string()), inner.clone()), 0..3)
                    .prop_map(|fields| Type::Record("Box".to_string(), fields)),
            ]
        })
    }

    fn unifies(left: &Type, right: &Type) -> bool {
        let mut infer = Infer::new();
        infer.unify(left, right, Span::new(0, 0)).is_ok()
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        #[test]
        fn unify_success_is_symmetric(left in arb_type(), right in arb_type()) {
            prop_assert_eq!(unifies(&left, &right), unifies(&right, &left));
        }
    }
}

#[cfg(test)]
mod mutual_recursion_tests {
    use super::*;

    fn infer(input: &str) -> Result<Vec<(String, TypeScheme)>, TypeError> {
        let program = lsharp_syntax::parse(input).unwrap();
        let mut infer = Infer::new();
        infer.infer_program(&program)
    }

    // --- P8-5: 相互再帰関数の前方参照テスト ---

    #[test]
    fn test_mutual_recursion_even_odd() {
        // even?/odd? の相互再帰型推論
        let results = infer(
            "(defn even? [n] (if (= n 0) true (odd? (- n 1))))
             (defn odd? [n] (if (= n 0) false (even? (- n 1))))",
        )
        .unwrap();
        // even? : (Int) -> Bool
        let even_scheme = &results.iter().find(|(n, _)| n == "even?").unwrap().1;
        assert_eq!(even_scheme.to_string(), "(Int) -> Bool");
        // odd? : (Int) -> Bool
        let odd_scheme = &results.iter().find(|(n, _)| n == "odd?").unwrap().1;
        assert_eq!(odd_scheme.to_string(), "(Int) -> Bool");
    }

    #[test]
    fn test_mutual_recursion_three_functions() {
        // 3関数の循環再帰: 型推論がエラーにならないことを検証
        // 戻り値型は循環的なため具体的な型には解決されない（多相型変数のまま）
        let results = infer(
            "(defn f [x] (g (+ x 1)))
             (defn g [x] (h (+ x 2)))
             (defn h [x] (f (+ x 3)))",
        )
        .unwrap();
        // 全3関数が推論に成功し、関数型であること
        let f_scheme = &results.iter().find(|(n, _)| n == "f").unwrap().1;
        assert!(
            f_scheme.to_string().contains("(Int) ->"),
            "f should be a function from Int: {}",
            f_scheme
        );
        let g_scheme = &results.iter().find(|(n, _)| n == "g").unwrap().1;
        assert!(
            g_scheme.to_string().contains("(Int) ->"),
            "g should be a function from Int: {}",
            g_scheme
        );
        let h_scheme = &results.iter().find(|(n, _)| n == "h").unwrap().1;
        assert!(
            h_scheme.to_string().contains("(Int) ->"),
            "h should be a function from Int: {}",
            h_scheme
        );
    }

    #[test]
    fn test_mutual_recursion_does_not_break_non_recursive() {
        // 既存の非再帰 defn が壊れないことの回帰テスト
        let results = infer(
            "(defn add [a b] (+ a b))
             (defn double [x] (add x x))",
        )
        .unwrap();
        let add_scheme = &results.iter().find(|(n, _)| n == "add").unwrap().1;
        assert_eq!(add_scheme.to_string(), "(Int, Int) -> Int");
        let double_scheme = &results.iter().find(|(n, _)| n == "double").unwrap().1;
        assert_eq!(double_scheme.to_string(), "(Int) -> Int");
    }
}

#[cfg(test)]
mod gadt_tests {
    use super::*;

    fn infer_ok(input: &str) -> Vec<(String, TypeScheme)> {
        let program = lsharp_syntax::parse(input).unwrap();
        let mut infer = Infer::new();
        infer.infer_program(&program).unwrap()
    }

    /// 型推論がエラーになることを検証するヘルパー
    fn infer_err(input: &str) {
        let program = lsharp_syntax::parse(input).unwrap();
        let mut infer = Infer::new();
        assert!(infer.infer_program(&program).is_err());
    }

    #[test]
    fn test_gadt_return_type_registered() {
        // GADT バリアントの戻り型が記録される
        // 注: パーサーで return_type を設定するには構文拡張が必要
        // ここでは register_type_def を直接テスト
        let mut infer = Infer::new();
        let mut env = infer.builtin_env();

        let variants = vec![Variant {
            span: lsharp_syntax::span::Span::new(0, 0),
            name: "IntLit".to_string(),
            fields: vec![TypeExpr::Named(
                lsharp_syntax::span::Span::new(0, 0),
                "Int".to_string(),
            )],
            return_type: Some(TypeExpr::App(
                lsharp_syntax::span::Span::new(0, 0),
                Box::new(TypeExpr::Named(
                    lsharp_syntax::span::Span::new(0, 0),
                    "Expr".to_string(),
                )),
                vec![TypeExpr::Named(
                    lsharp_syntax::span::Span::new(0, 0),
                    "Int".to_string(),
                )],
            )),
        }];

        infer
            .register_type_def(&mut env, "Expr", &["a".to_string()], &variants)
            .unwrap();

        // IntLit が GADT 戻り型を持つ
        assert!(infer.gadt_return_types.contains_key("IntLit"));
    }

    #[test]
    fn test_gadt_basic_type_check() {
        // 基本的な GADT パターンマッチが型チェックを通る
        let _results = infer_ok(
            "(type (Maybe a) (Just a) Nothing)
             (defn unwrap [m] (match m [(Just x) x]))",
        );
    }

    // --- Kind 整合性チェックテスト (NC-12) ---

    #[test]
    fn test_kind_mismatch_functor_impl_for_star_type() {
        // Int は * なので Functor (* -> *) の impl はエラーになるべき
        let source = r#"
            (impl (Functor Int)
                (defn fmap [f x] (f x)))
            (defn main [] 0)
        "#;
        let program = lsharp_syntax::parse(source).unwrap();
        let mut infer = Infer::new();
        let result = infer.infer_program(&program);
        assert!(
            result.is_err(),
            "Int (* kind) への Functor impl はエラーになるべき"
        );
    }

    #[test]
    fn test_kind_mismatch_monad_impl_for_star_type() {
        // Bool は * なので Monad (* -> *) の impl はエラーになるべき
        let source = r#"
            (impl (Monad Bool)
                (defn bind [m f] (f m))
                (defn pure [x] x))
            (defn main [] 0)
        "#;
        let program = lsharp_syntax::parse(source).unwrap();
        let mut infer = Infer::new();
        let result = infer.infer_program(&program);
        assert!(
            result.is_err(),
            "Bool (* kind) への Monad impl はエラーになるべき"
        );
    }

    #[test]
    fn test_kind_functor_impl_for_maybe_succeeds() {
        // Maybe は * -> * なので Functor の impl は成功するべき
        let source = r#"
            (type (Maybe a) (Just a) Nothing)
            (impl (Functor Maybe)
                (defn fmap [f m]
                    (match m
                        [(Just x) (Just (f x))]
                        [Nothing Nothing])))
            (defn main [] 0)
        "#;
        let program = lsharp_syntax::parse(source).unwrap();
        let mut infer = Infer::new();
        let result = infer.infer_program(&program);
        assert!(
            result.is_ok(),
            "Maybe (* -> * kind) への Functor impl は成功すべき: {:?}",
            result.err()
        );
    }

    // --- GADT テスト追加 (G-1) ---

    #[test]
    fn test_gadt_simple_refinement() {
        // 単純な ADT パターンマッチで型が絞り込まれる
        let results = infer_ok(
            "(type (Either a b) (Left a) (Right b))
             (defn get-left [e]
               (match e
                 [(Left x) x]
                 [(Right _) 0]))",
        );
        assert!(results.iter().any(|(name, _)| name == "get-left"));
    }

    #[test]
    fn test_gadt_nested_pattern() {
        // ネストした ADT パターンマッチ
        let results = infer_ok(
            "(type (Maybe a) (Just a) Nothing)
             (defn is-just [m]
               (match m
                 [(Just _) 1]
                 [Nothing 0]))",
        );
        assert!(results.iter().any(|(name, _)| name == "is-just"));
    }

    #[test]
    fn test_gadt_multiple_type_vars() {
        // 複数の型変数を持つ ADT
        let results = infer_ok(
            "(type (Pair a b) (MkPair a b))
             (defn fst [p]
               (match p
                 [(MkPair x _) x]))",
        );
        assert!(results.iter().any(|(name, _)| name == "fst"));
    }

    #[test]
    fn test_gadt_exhaustive_match() {
        // 全コンストラクタをマッチ
        let results = infer_ok(
            "(type Color Red Green Blue)
             (defn color-to-int [c]
               (match c
                 [Red 0]
                 [Green 1]
                 [Blue 2]))",
        );
        assert!(results.iter().any(|(name, _)| name == "color-to-int"));
    }

    #[test]
    fn test_gadt_invalid_constructor_error() {
        // 未定義のコンストラクタはエラー
        infer_err(
            "(type (Maybe a) (Just a) Nothing)
             (defn bad [m]
               (match m
                 [(Foo x) x]))",
        );
    }

    // --- Where 句テスト (G-2) ---

    #[test]
    fn test_where_multi_constraint() {
        // 複数の where 制約が型チェックを通る
        let _results = infer_ok(
            "(trait (Show a)
               (defn show [self] : Int))
             (trait (Eq a)
               (defn eq [x y] : Int))
             (defn show-eq [x]
               :where [(Show a) (Eq a)]
               x)",
        );
    }

    #[test]
    fn test_where_single_constraint() {
        // 単一の where 制約
        let _results = infer_ok(
            "(trait (Num a)
               (defn add [x y] : Int))
             (defn double [x]
               :where [(Num a)]
               (+ x x))",
        );
    }
}

#[cfg(test)]
mod inference_property_tests {
    use super::*;
    use proptest::prelude::*;

    fn arb_expression_source() -> impl Strategy<Value = String> {
        let leaves = prop_oneof![
            Just("0".to_string()),
            Just("1".to_string()),
            Just("true".to_string()),
            Just("false".to_string()),
            Just("\"x\"".to_string()),
            Just("x".to_string()),
            Just("y".to_string()),
            Just("()".to_string()),
        ];

        leaves.prop_recursive(3, 32, 4, |inner| {
            prop_oneof![
                (inner.clone(), inner.clone(), inner.clone())
                    .prop_map(|(cond, then, else_)| { format!("(if {cond} {then} {else_})") }),
                (inner.clone(), inner.clone())
                    .prop_map(|(value, body)| format!("(let [x {value}] {body})")),
                inner.clone().prop_map(|body| format!("(fn [x] {body})")),
                (inner.clone(), prop::collection::vec(inner.clone(), 1..3))
                    .prop_map(|(func, args)| format!("({func} {})", args.join(" "))),
                prop::collection::vec(inner.clone(), 1..3)
                    .prop_map(|exprs| format!("(do {})", exprs.join(" "))),
                inner.clone().prop_map(|expr| format!("(: {expr} Int)")),
                inner.prop_map(|expr| format!("'{expr}")),
            ]
        })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        #[test]
        fn bounded_expression_inference_never_panics(source in arb_expression_source()) {
            let program_source = format!("(defn main [] {source})");
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let program = lsharp_syntax::parse(&program_source)
                    .expect("generated expression source must parse");
                let mut infer = Infer::new();
                let _ = infer.infer_program(&program);
            }));

            prop_assert!(result.is_ok(), "type inference panicked for source: {program_source}");
        }
    }
}
