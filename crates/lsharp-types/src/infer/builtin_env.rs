use super::*;

impl Infer {
    /// 組み込み関数の型環境
    pub(super) fn builtin_env(&mut self) -> TypeEnv {
        let mut env = TypeEnv::new();

        // 算術演算子: (Int, Int) -> Int
        let int_binop = TypeScheme::mono(Type::Fun(
            vec![Type::int(), Type::int()],
            Box::new(Type::int()),
        ));
        for op in ["+", "-", "*", "/", "%"] {
            env.insert(op.to_string(), int_binop.clone());
        }

        // 比較演算子: (Int, Int) -> Bool
        let int_cmp = TypeScheme::mono(Type::Fun(
            vec![Type::int(), Type::int()],
            Box::new(Type::bool()),
        ));
        for op in ["<", ">", "<=", ">=", "==", "!=", "="] {
            env.insert(op.to_string(), int_cmp.clone());
        }

        // 浮動小数点演算子
        let float_binop = TypeScheme::mono(Type::Fun(
            vec![Type::float(), Type::float()],
            Box::new(Type::float()),
        ));
        for op in ["+.", "-.", "*.", "/."] {
            env.insert(op.to_string(), float_binop.clone());
        }

        // print: forall a. a -> Unit
        let a = self.var_gen.fresh_id();
        env.insert(
            "print".to_string(),
            TypeScheme {
                vars: vec![a],
                constraints: Vec::new(),
                ty: Type::Fun(vec![Type::Var(a)], Box::new(Type::unit())),
            },
        );

        // __alloc: Int -> Int (メモリアロケーション: サイズ -> アドレス)
        env.insert(
            "__alloc".to_string(),
            TypeScheme::mono(Type::Fun(vec![Type::int()], Box::new(Type::int()))),
        );

        // string-length: String -> Int (文字列のバイト長を返す)
        env.insert(
            "string-length".to_string(),
            TypeScheme::mono(Type::Fun(vec![Type::string()], Box::new(Type::int()))),
        );

        // string-concat: String -> String -> String (文字列結合)
        env.insert(
            "string-concat".to_string(),
            TypeScheme::mono(Type::Fun(
                vec![Type::string(), Type::string()],
                Box::new(Type::string()),
            )),
        );

        // string-eq: String -> String -> Bool (文字列等価比較)
        env.insert(
            "string-eq".to_string(),
            TypeScheme::mono(Type::Fun(
                vec![Type::string(), Type::string()],
                Box::new(Type::bool()),
            )),
        );

        // print-string: String -> Unit (文字列を出力)
        env.insert(
            "print-string".to_string(),
            TypeScheme::mono(Type::Fun(vec![Type::string()], Box::new(Type::unit()))),
        );

        // string-char-at: String -> Int -> Int (文字列のインデックス位置のバイト値を返す)
        env.insert(
            "string-char-at".to_string(),
            TypeScheme::mono(Type::Fun(
                vec![Type::string(), Type::int()],
                Box::new(Type::int()),
            )),
        );

        // substring: String -> Int -> Int -> String (部分文字列を返す)
        env.insert(
            "substring".to_string(),
            TypeScheme::mono(Type::Fun(
                vec![Type::string(), Type::int(), Type::int()],
                Box::new(Type::string()),
            )),
        );

        // int-to-string: Int -> String (整数を文字列に変換)
        env.insert(
            "int-to-string".to_string(),
            TypeScheme::mono(Type::Fun(vec![Type::int()], Box::new(Type::string()))),
        );

        // proc-exit: Int -> Unit (プロセス終了)
        env.insert(
            "proc-exit".to_string(),
            TypeScheme::mono(Type::Fun(vec![Type::int()], Box::new(Type::unit()))),
        );

        // ref-new: forall a. a -> Ref a (Ref Cell 作成: 値 -> ヒープハンドル)
        {
            let a = self.var_gen.fresh_id();
            env.insert(
                "ref-new".to_string(),
                TypeScheme {
                    vars: vec![a],
                    constraints: Vec::new(),
                    ty: Type::Fun(vec![Type::Var(a)], Box::new(Type::ref_of(Type::Var(a)))),
                },
            );
        }

        // ref-get: forall a. Ref a -> a (Ref Cell 読み出し: ヒープハンドル -> 値)
        {
            let a = self.var_gen.fresh_id();
            env.insert(
                "ref-get".to_string(),
                TypeScheme {
                    vars: vec![a],
                    constraints: Vec::new(),
                    ty: Type::Fun(vec![Type::ref_of(Type::Var(a))], Box::new(Type::Var(a))),
                },
            );
        }

        // ref-set: forall a. (Ref a, a) -> Unit (Ref Cell 書き込み: ヒープハンドル, 値 -> Unit)
        {
            let a = self.var_gen.fresh_id();
            env.insert(
                "ref-set".to_string(),
                TypeScheme {
                    vars: vec![a],
                    constraints: Vec::new(),
                    ty: Type::Fun(
                        vec![Type::ref_of(Type::Var(a)), Type::Var(a)],
                        Box::new(Type::unit()),
                    ),
                },
            );
        }

        // === Vector (可変長配列) ビルトイン ===

        // vector-new: Int -> Vector (capacity を指定して空ベクタを作成)
        env.insert(
            "vector-new".to_string(),
            TypeScheme::mono(Type::Fun(vec![Type::int()], Box::new(Type::vector()))),
        );

        // vector-length: Vector -> Int (ベクタの現在の長さを返す)
        env.insert(
            "vector-length".to_string(),
            TypeScheme::mono(Type::Fun(vec![Type::vector()], Box::new(Type::int()))),
        );

        // vector-get: forall a. (Vector, Int) -> a (インデックス指定で要素を取得)
        {
            let a = self.var_gen.fresh_id();
            env.insert(
                "vector-get".to_string(),
                TypeScheme {
                    vars: vec![a],
                    constraints: Vec::new(),
                    ty: Type::Fun(vec![Type::vector(), Type::int()], Box::new(Type::Var(a))),
                },
            );
        }

        // vector-set: forall a. (Vector, Int, a) -> Vector (インデックス指定で要素を上書き)
        {
            let a = self.var_gen.fresh_id();
            env.insert(
                "vector-set".to_string(),
                TypeScheme {
                    vars: vec![a],
                    constraints: Vec::new(),
                    ty: Type::Fun(
                        vec![Type::vector(), Type::int(), Type::Var(a)],
                        Box::new(Type::vector()),
                    ),
                },
            );
        }

        // vector-push: forall a. (Vector, a) -> Vector (要素を末尾に追加)
        {
            let a = self.var_gen.fresh_id();
            env.insert(
                "vector-push".to_string(),
                TypeScheme {
                    vars: vec![a],
                    constraints: Vec::new(),
                    ty: Type::Fun(vec![Type::vector(), Type::Var(a)], Box::new(Type::vector())),
                },
            );
        }

        // === HashMap ビルトイン ===

        // map-new: () -> Map (デフォルト容量で空のハッシュマップを作成)
        env.insert(
            "map-new".to_string(),
            TypeScheme::mono(Type::Fun(vec![], Box::new(Type::map()))),
        );

        // map-size: Map -> Int (エントリ数を返す)
        env.insert(
            "map-size".to_string(),
            TypeScheme::mono(Type::Fun(vec![Type::map()], Box::new(Type::int()))),
        );

        // map-insert: forall k a. (Map, k, a) -> Map (キーと値を挿入)
        {
            let k = self.var_gen.fresh_id();
            let a = self.var_gen.fresh_id();
            env.insert(
                "map-insert".to_string(),
                TypeScheme {
                    vars: vec![k, a],
                    constraints: Vec::new(),
                    ty: Type::Fun(
                        vec![Type::map(), Type::Var(k), Type::Var(a)],
                        Box::new(Type::map()),
                    ),
                },
            );
        }

        // map-get: forall k a. (Map, k) -> a (キーで値を取得、未存在時は 0)
        {
            let k = self.var_gen.fresh_id();
            let a = self.var_gen.fresh_id();
            env.insert(
                "map-get".to_string(),
                TypeScheme {
                    vars: vec![k, a],
                    constraints: Vec::new(),
                    ty: Type::Fun(vec![Type::map(), Type::Var(k)], Box::new(Type::Var(a))),
                },
            );
        }

        // map-contains?: forall k. (Map, k) -> Bool (キーの存在チェック)
        {
            let k = self.var_gen.fresh_id();
            env.insert(
                "map-contains?".to_string(),
                TypeScheme {
                    vars: vec![k],
                    constraints: Vec::new(),
                    ty: Type::Fun(vec![Type::map(), Type::Var(k)], Box::new(Type::int())),
                },
            );
        }

        // map-remove: forall k. (Map, k) -> Map (キーを削除)
        {
            let k = self.var_gen.fresh_id();
            env.insert(
                "map-remove".to_string(),
                TypeScheme {
                    vars: vec![k],
                    constraints: Vec::new(),
                    ty: Type::Fun(vec![Type::map(), Type::Var(k)], Box::new(Type::map())),
                },
            );
        }

        // read-file: String -> String (ファイル内容を読み込み)
        env.insert(
            "read-file".to_string(),
            TypeScheme::mono(Type::Fun(vec![Type::string()], Box::new(Type::string()))),
        );

        // write-file: (String, String) -> Int (ファイルに書き込み、書き込みバイト数を返す)
        env.insert(
            "write-file".to_string(),
            TypeScheme::mono(Type::Fun(
                vec![Type::string(), Type::string()],
                Box::new(Type::int()),
            )),
        );

        // write-file-bytes: (String, Vector) -> Int (Vector の下位 8 bit を raw bytes として書き込み)
        env.insert(
            "write-file-bytes".to_string(),
            TypeScheme::mono(Type::Fun(
                vec![Type::string(), Type::vector()],
                Box::new(Type::int()),
            )),
        );

        // file-exists?: String -> Bool (ファイルが存在するか)
        env.insert(
            "file-exists?".to_string(),
            TypeScheme::mono(Type::Fun(vec![Type::string()], Box::new(Type::bool()))),
        );

        // command-line-args: () -> Int (引数の数を返す)
        env.insert(
            "command-line-args".to_string(),
            TypeScheme::mono(Type::Fun(vec![], Box::new(Type::int()))),
        );

        // command-line-arg: Int -> String (指定 index の argv 要素を返す)
        env.insert(
            "command-line-arg".to_string(),
            TypeScheme::mono(Type::Fun(vec![Type::int()], Box::new(Type::string()))),
        );

        // read-stdin: () -> String (stdin 全体を読む)
        env.insert(
            "read-stdin".to_string(),
            TypeScheme::mono(Type::Fun(vec![], Box::new(Type::string()))),
        );

        // root_push: forall a. a -> Int (任意値を root stack に積み、slot handle を返す)
        {
            let a = self.var_gen.fresh_id();
            env.insert(
                "root_push".to_string(),
                TypeScheme {
                    vars: vec![a],
                    constraints: Vec::new(),
                    ty: Type::Fun(vec![Type::Var(a)], Box::new(Type::int())),
                },
            );
        }

        // root_pop: forall a. () -> a (root stack から最新値を取り出す)
        {
            let a = self.var_gen.fresh_id();
            env.insert(
                "root_pop".to_string(),
                TypeScheme {
                    vars: vec![a],
                    constraints: Vec::new(),
                    ty: Type::Fun(vec![], Box::new(Type::Var(a))),
                },
            );
        }

        // root_set: forall a. (Int, a) -> Int (既存 slot を別値へ差し替える)
        {
            let a = self.var_gen.fresh_id();
            env.insert(
                "root_set".to_string(),
                TypeScheme {
                    vars: vec![a],
                    constraints: Vec::new(),
                    ty: Type::Fun(vec![Type::int(), Type::Var(a)], Box::new(Type::int())),
                },
            );
        }

        // not: Bool -> Bool
        env.insert(
            "not".to_string(),
            TypeScheme::mono(Type::Fun(vec![Type::bool()], Box::new(Type::bool()))),
        );

        // and, or: (Bool, Bool) -> Bool
        let bool_binop = TypeScheme::mono(Type::Fun(
            vec![Type::bool(), Type::bool()],
            Box::new(Type::bool()),
        ));
        env.insert("and".to_string(), bool_binop.clone());
        env.insert("or".to_string(), bool_binop);

        // 組み込み型の Kind を登録
        for name in ["Int", "Float", "String", "Bool", "Unit"] {
            self.kind_env.insert(name.to_string(), Kind::star());
        }

        // Functor トレイト: fmap : (a -> b) -> f a -> f b
        // Kind 制約: f : * -> *
        {
            let f_var = self.var_gen.fresh_id();
            let a_var = self.var_gen.fresh_id();
            let b_var = self.var_gen.fresh_id();
            let fmap_type = Type::Fun(
                vec![
                    // (a -> b)
                    Type::Fun(vec![Type::Var(a_var)], Box::new(Type::Var(b_var))),
                    // f a
                    Type::App("__f__".to_string(), vec![Type::Var(a_var)]),
                ],
                // f b
                Box::new(Type::App("__f__".to_string(), vec![Type::Var(b_var)])),
            );
            let fmap_scheme = TypeScheme {
                vars: vec![f_var, a_var, b_var],
                constraints: vec![TraitConstraint {
                    trait_name: "Functor".to_string(),
                    type_var: f_var,
                }],
                ty: fmap_type,
            };
            self.trait_registry.insert(
                "Functor".to_string(),
                TraitInfo {
                    name: "Functor".to_string(),
                    type_param: f_var,
                    methods: vec![("fmap".to_string(), fmap_scheme.clone())],
                },
            );
            self.kind_env.insert("Functor".to_string(), Kind::unary());
        }

        // Monad トレイト: bind : m a -> (a -> m b) -> m b, pure : a -> m a
        // Kind 制約: m : * -> *
        {
            let m_var = self.var_gen.fresh_id();
            let a_var = self.var_gen.fresh_id();
            let b_var = self.var_gen.fresh_id();
            let monad_constraint = TraitConstraint {
                trait_name: "Monad".to_string(),
                type_var: m_var,
            };
            // bind : m a -> (a -> m b) -> m b
            let bind_type = Type::Fun(
                vec![
                    Type::App("__m__".to_string(), vec![Type::Var(a_var)]),
                    Type::Fun(
                        vec![Type::Var(a_var)],
                        Box::new(Type::App("__m__".to_string(), vec![Type::Var(b_var)])),
                    ),
                ],
                Box::new(Type::App("__m__".to_string(), vec![Type::Var(b_var)])),
            );
            let bind_scheme = TypeScheme {
                vars: vec![m_var, a_var, b_var],
                constraints: vec![monad_constraint.clone()],
                ty: bind_type,
            };
            // pure : a -> m a
            let a2_var = self.var_gen.fresh_id();
            let pure_type = Type::Fun(
                vec![Type::Var(a2_var)],
                Box::new(Type::App("__m__".to_string(), vec![Type::Var(a2_var)])),
            );
            let pure_scheme = TypeScheme {
                vars: vec![m_var, a2_var],
                constraints: vec![monad_constraint],
                ty: pure_type,
            };
            self.trait_registry.insert(
                "Monad".to_string(),
                TraitInfo {
                    name: "Monad".to_string(),
                    type_param: m_var,
                    methods: vec![
                        ("bind".to_string(), bind_scheme),
                        ("pure".to_string(), pure_scheme),
                    ],
                },
            );
            self.kind_env.insert("Monad".to_string(), Kind::unary());
        }

        env
    }
}
