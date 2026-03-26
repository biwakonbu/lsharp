use super::support::*;


#[test]
fn test_e2e_int_to_string_concat() {
    // int-to-string + string-concat の組み合わせ
    let result = compile_and_run(r#"
        (defn main []
          (do (print-string (string-concat "value=" (int-to-string 42))) 0))
    "#);
    assert_eq!(result, "value=42");
}

// === P3-3: 高階関数 (list-map, list-filter, list-fold) E2E テスト ===

#[test]
fn test_e2e_closure_with_adt_basic() {
    // クロージャ引数を ADT の再帰関数内で使う基本テスト
    // apply-to-list: リストの先頭要素にクロージャを適用
    let output = compile_and_run(
        "(type (List a) (Cons a (List a)) Nil)
         (defn apply-head [f xs]
           (match xs
             [(Cons h t) (f h)]
             [Nil 0]))
         (defn main [] (print (apply-head (fn [x] (* x 10)) (Cons 4 (Cons 2 Nil)))))",
    );
    assert_eq!(output, "40\n");
}

#[test]
fn test_e2e_list_map() {
    // list-map: リスト全要素にクロージャを適用して新しいリストを返す
    let output = compile_and_run(
        "(type (List a) (Cons a (List a)) Nil)
         (defn list-map [f xs]
           (match xs
             [Nil Nil]
             [(Cons h t) (Cons (f h) (list-map f t))]))
         (defn sum-list [xs]
           (match xs
             [Nil 0]
             [(Cons h t) (+ h (sum-list t))]))
         (defn main [] (print (sum-list (list-map (fn [x] (* x 2)) (Cons 1 (Cons 2 (Cons 3 Nil)))))))",
    );
    // (1*2) + (2*2) + (3*2) = 2 + 4 + 6 = 12
    assert_eq!(output, "12\n");
}

#[test]
fn test_e2e_list_filter() {
    // list-filter: 条件を満たす要素のみ残す
    let output = compile_and_run(
        "(type (List a) (Cons a (List a)) Nil)
         (defn list-filter [f xs]
           (match xs
             [Nil Nil]
             [(Cons h t) (if (f h) (Cons h (list-filter f t)) (list-filter f t))]))
         (defn sum-list [xs]
           (match xs
             [Nil 0]
             [(Cons h t) (+ h (sum-list t))]))
         (defn main [] (print (sum-list (list-filter (fn [x] (> x 2)) (Cons 1 (Cons 2 (Cons 3 (Cons 4 Nil))))))))",
    );
    // 3 + 4 = 7
    assert_eq!(output, "7\n");
}

#[test]
fn test_e2e_list_fold() {
    // list-fold: リストを畳み込み
    let output = compile_and_run(
        "(type (List a) (Cons a (List a)) Nil)
         (defn list-fold [f init xs]
           (match xs
             [Nil init]
             [(Cons h t) (list-fold f (f init h) t)]))
         (defn main [] (print (list-fold (fn [acc x] (+ acc x)) 0 (Cons 1 (Cons 2 (Cons 3 Nil))))))",
    );
    // 0 + 1 + 2 + 3 = 6
    assert_eq!(output, "6\n");
}

#[test]
fn test_e2e_list_map_identity() {
    // list-map に恒等関数を渡すとリストが変わらない
    let output = compile_and_run(
        "(type (List a) (Cons a (List a)) Nil)
         (defn list-map [f xs]
           (match xs
             [Nil Nil]
             [(Cons h t) (Cons (f h) (list-map f t))]))
         (defn sum-list [xs]
           (match xs
             [Nil 0]
             [(Cons h t) (+ h (sum-list t))]))
         (defn main [] (print (sum-list (list-map (fn [x] x) (Cons 10 (Cons 20 (Cons 30 Nil)))))))",
    );
    // 10 + 20 + 30 = 60
    assert_eq!(output, "60\n");
}

#[test]
fn test_e2e_list_fold_product() {
    // list-fold で積を計算
    let output = compile_and_run(
        "(type (List a) (Cons a (List a)) Nil)
         (defn list-fold [f init xs]
           (match xs
             [Nil init]
             [(Cons h t) (list-fold f (f init h) t)]))
         (defn main [] (print (list-fold (fn [acc x] (* acc x)) 1 (Cons 2 (Cons 3 (Cons 4 Nil))))))",
    );
    // 1 * 2 * 3 * 4 = 24
    assert_eq!(output, "24\n");
}

#[test]
fn test_e2e_list_filter_none() {
    // list-filter で全要素がフィルタアウトされる場合
    let output = compile_and_run(
        "(type (List a) (Cons a (List a)) Nil)
         (defn list-filter [f xs]
           (match xs
             [Nil Nil]
             [(Cons h t) (if (f h) (Cons h (list-filter f t)) (list-filter f t))]))
         (defn list-length [xs]
           (match xs
             [Nil 0]
             [(Cons h t) (+ 1 (list-length t))]))
         (defn main [] (print (list-length (list-filter (fn [x] (> x 100)) (Cons 1 (Cons 2 (Cons 3 Nil)))))))",
    );
    assert_eq!(output, "0\n");
}

#[test]
fn test_e2e_list_map_filter_compose() {
    // list-map と list-filter の合成: まず 2 倍してから 4 より大きいものを残す
    let output = compile_and_run(
        "(type (List a) (Cons a (List a)) Nil)
         (defn list-map [f xs]
           (match xs
             [Nil Nil]
             [(Cons h t) (Cons (f h) (list-map f t))]))
         (defn list-filter [f xs]
           (match xs
             [Nil Nil]
             [(Cons h t) (if (f h) (Cons h (list-filter f t)) (list-filter f t))]))
         (defn sum-list [xs]
           (match xs
             [Nil 0]
             [(Cons h t) (+ h (sum-list t))]))
         (defn main [] (print (sum-list (list-filter (fn [x] (> x 4)) (list-map (fn [x] (* x 2)) (Cons 1 (Cons 2 (Cons 3 (Cons 4 Nil)))))))))",
    );
    // map *2: [2, 4, 6, 8], filter >4: [6, 8], sum: 14
    assert_eq!(output, "14\n");
}

// === Vector (可変長配列) ビルトイン テスト ===

#[test]
fn test_e2e_vector_new_length() {
    // vector-new で作成したベクタの初期長さは 0
    let result = compile_and_run(r#"
        (defn main []
          (print (vector-length (vector-new 10))))
    "#);
    assert_eq!(result.trim(), "0");
}

#[test]
fn test_e2e_vector_push_length() {
    // vector-push で要素を追加すると長さが増える
    let result = compile_and_run(r#"
        (defn main []
          (let [v (vector-new 4)
                v1 (vector-push v 10)
                v2 (vector-push v1 20)
                v3 (vector-push v2 30)]
            (print (vector-length v3))))
    "#);
    assert_eq!(result.trim(), "3");
}

#[test]
fn test_e2e_vector_get() {
    // vector-get でインデックス指定の要素を取得
    let result = compile_and_run(r#"
        (defn main []
          (let [v (vector-new 4)
                v1 (vector-push v 100)
                v2 (vector-push v1 200)
                v3 (vector-push v2 300)]
            (do
              (print (vector-get v3 0))
              (print (vector-get v3 1))
              (print (vector-get v3 2)))))
    "#);
    assert_eq!(result.trim(), "100\n200\n300");
}

#[test]
fn test_e2e_vector_set() {
    // vector-set でインデックス指定の要素を上書き
    let result = compile_and_run(r#"
        (defn main []
          (let [v (vector-new 4)
                v1 (vector-push v 10)
                v2 (vector-push v1 20)
                v3 (vector-set v2 0 99)]
            (do
              (print (vector-get v3 0))
              (print (vector-get v3 1)))))
    "#);
    assert_eq!(result.trim(), "99\n20");
}

#[test]
fn test_e2e_vector_push_beyond_capacity() {
    // capacity を超えて push すると再割り当てされる
    let result = compile_and_run(r#"
        (defn main []
          (let [v (vector-new 2)
                v1 (vector-push v 1)
                v2 (vector-push v1 2)
                v3 (vector-push v2 3)]
            (do
              (print (vector-length v3))
              (print (vector-get v3 0))
              (print (vector-get v3 1))
              (print (vector-get v3 2)))))
    "#);
    assert_eq!(result.trim(), "3\n1\n2\n3");
}

// === Vector 高階関数テスト (ユーザー定義) ===

#[test]
fn test_e2e_vector_map() {
    // vector-map: ベクタの全要素に関数を適用して新しいベクタを返す
    let result = compile_and_run(r#"
        (defn vector-map-loop [f v i len acc]
          (if (>= i len)
            acc
            (vector-map-loop f v (+ i 1) len (vector-push acc (f (vector-get v i))))))
        (defn vector-map [f v]
          (vector-map-loop f v 0 (vector-length v) (vector-new (vector-length v))))
        (defn main []
          (let [v (vector-new 4)
                v1 (vector-push v 10)
                v2 (vector-push v1 20)
                v3 (vector-push v2 30)
                result (vector-map (fn [x] (* x 2)) v3)]
            (do
              (print (vector-length result))
              (print (vector-get result 0))
              (print (vector-get result 1))
              (print (vector-get result 2)))))
    "#);
    // 各要素を 2 倍: [10,20,30] -> [20,40,60]
    assert_eq!(result.trim(), "3\n20\n40\n60");
}

#[test]
fn test_e2e_vector_filter() {
    // vector-filter: 条件を満たす要素のみ残した新しいベクタを返す
    let result = compile_and_run(r#"
        (defn vector-filter-loop [f v i len acc]
          (if (>= i len)
            acc
            (if (f (vector-get v i))
              (vector-filter-loop f v (+ i 1) len (vector-push acc (vector-get v i)))
              (vector-filter-loop f v (+ i 1) len acc))))
        (defn vector-filter [f v]
          (vector-filter-loop f v 0 (vector-length v) (vector-new (vector-length v))))
        (defn main []
          (let [v (vector-new 4)
                v1 (vector-push v 10)
                v2 (vector-push v1 25)
                v3 (vector-push v2 5)
                v4 (vector-push v3 30)
                result (vector-filter (fn [x] (> x 15)) v4)]
            (do
              (print (vector-length result))
              (print (vector-get result 0))
              (print (vector-get result 1)))))
    "#);
    // 15 より大きい要素のみ: [10,25,5,30] -> [25,30]
    assert_eq!(result.trim(), "2\n25\n30");
}

// === HashMap ビルトイン テスト ===

#[test]
fn test_e2e_map_new_size() {
    // map-new で作成したマップの初期サイズは 0
    let result = compile_and_run(r#"
        (defn main []
          (print (map-size (map-new))))
    "#);
    assert_eq!(result.trim(), "0");
}

#[test]
fn test_e2e_map_insert_size() {
    // map-insert でエントリを追加するとサイズが増える
    let result = compile_and_run(r#"
        (defn main []
          (let [m (map-new)
                m1 (map-insert m 1 100)
                m2 (map-insert m1 2 200)]
            (print (map-size m2))))
    "#);
    assert_eq!(result.trim(), "2");
}

#[test]
fn test_e2e_map_insert_get() {
    // map-insert で挿入した値を map-get で取得
    let result = compile_and_run(r#"
        (defn main []
          (let [m (map-new)
                m1 (map-insert m 42 100)]
            (print (map-get m1 42))))
    "#);
    assert_eq!(result.trim(), "100");
}

#[test]
fn test_e2e_map_insert_get_multiple() {
    // 複数エントリの挿入と取得
    let result = compile_and_run(r#"
        (defn main []
          (let [m (map-new)
                m1 (map-insert m 1 10)
                m2 (map-insert m1 2 20)
                m3 (map-insert m2 3 30)]
            (do
              (print (map-get m3 1))
              (print (map-get m3 2))
              (print (map-get m3 3)))))
    "#);
    assert_eq!(result.trim(), "10\n20\n30");
}

#[test]
fn test_e2e_map_get_missing() {
    // 存在しないキーの取得は 0 を返す
    let result = compile_and_run(r#"
        (defn main []
          (let [m (map-new)]
            (print (map-get m 99))))
    "#);
    assert_eq!(result.trim(), "0");
}

#[test]
fn test_e2e_map_contains_true() {
    // 存在するキーの検索
    let result = compile_and_run(r#"
        (defn main []
          (let [m (map-new)
                m1 (map-insert m 42 100)]
            (print (map-contains? m1 42))))
    "#);
    assert_eq!(result.trim(), "1");
}

#[test]
fn test_e2e_map_contains_false() {
    // 存在しないキーの検索
    let result = compile_and_run(r#"
        (defn main []
          (let [m (map-new)]
            (print (map-contains? m 42))))
    "#);
    assert_eq!(result.trim(), "0");
}

#[test]
fn test_e2e_map_remove() {
    // map-remove でエントリを削除
    let result = compile_and_run(r#"
        (defn main []
          (let [m (map-new)
                m1 (map-insert m 1 10)
                m2 (map-insert m1 2 20)
                m3 (map-remove m2 1)]
            (do
              (print (map-size m3))
              (print (map-contains? m3 1))
              (print (map-get m3 2)))))
    "#);
    assert_eq!(result.trim(), "1\n0\n20");
}

#[test]
fn test_e2e_map_insert_overwrite() {
    // 同じキーへの再挿入で値が上書きされる
    let result = compile_and_run(r#"
        (defn main []
          (let [m (map-new)
                m1 (map-insert m 1 10)
                m2 (map-insert m1 1 99)]
            (do
              (print (map-size m2))
              (print (map-get m2 1)))))
    "#);
    assert_eq!(result.trim(), "1\n99");
}


// === HashMap 文字列キー テスト ===

#[test]
fn test_e2e_map_string_key_insert_get() {
    // 文字列キーで insert して get で値を取得
    let result = compile_and_run(r#"
        (defn main []
          (let [m (map-new)
                m1 (map-insert m "hello" 42)
                m2 (map-insert m1 "world" 99)]
            (do
              (print (map-get m2 "hello"))
              (print (map-get m2 "world")))))
    "#);
    assert_eq!(result.trim(), "42\n99");
}

#[test]
fn test_e2e_map_string_key_contains() {
    // 文字列キーで contains? の確認
    let result = compile_and_run(r#"
        (defn main []
          (let [m (map-new)
                m1 (map-insert m "key1" 10)]
            (do
              (print (map-contains? m1 "key1"))
              (print (map-contains? m1 "key2")))))
    "#);
    assert_eq!(result.trim(), "1\n0");
}

#[test]
fn test_e2e_map_string_key_remove() {
    // 文字列キーで remove の確認
    let result = compile_and_run(r#"
        (defn main []
          (let [m (map-new)
                m1 (map-insert m "alpha" 100)
                m2 (map-insert m1 "beta" 200)
                m3 (map-remove m2 "alpha")]
            (do
              (print (map-size m3))
              (print (map-contains? m3 "alpha"))
              (print (map-get m3 "beta")))))
    "#);
    assert_eq!(result.trim(), "1\n0\n200");
}

#[test]
fn test_e2e_map_string_key_overwrite() {
    // 同じ文字列キーで上書きされることの確認
    let result = compile_and_run(r#"
        (defn main []
          (let [m (map-new)
                m1 (map-insert m "x" 10)
                m2 (map-insert m1 "x" 77)]
            (do
              (print (map-size m2))
              (print (map-get m2 "x")))))
    "#);
    assert_eq!(result.trim(), "1\n77");
}
// === 標準ライブラリ E2E テスト ===

/// stdlib/Core.ls の基本数学関数のテスト (abs, max, min, clamp)
#[test]
fn test_e2e_stdlib_core_math() {
    let output = compile_and_run(r#"
        (defn abs [x] (if (< x 0) (- 0 x) x))
        (defn max [a b] (if (> a b) a b))
        (defn min [a b] (if (< a b) a b))
        (defn clamp [x lo hi] (max lo (min x hi)))
        (defn main [] (do
            (print (abs (- 0 5)))
            (print (abs 3))
            (print (max 3 7))
            (print (min 3 7))
            (print (clamp 15 0 10))
            (print (clamp (- 0 5) 0 10))
            (print (clamp 5 0 10))
            0))
    "#);
    assert_eq!(output.trim(), "5\n3\n7\n3\n10\n0\n5");
}

/// stdlib/Core.ls の xor 関数テスト
#[test]
fn test_e2e_stdlib_core_xor() {
    let output = compile_and_run(r#"
        (defn xor [a b] (if a (if b 0 1) (if b 1 0)))
        (defn main [] (do
            (print (xor true true))
            (print (xor true false))
            (print (xor false true))
            (print (xor false false))
            0))
    "#);
    assert_eq!(output.trim(), "0\n1\n1\n0");
}

/// stdlib/Core.ls の identity, const, twice 関数テスト
#[test]
fn test_e2e_stdlib_core_combinators() {
    let output = compile_and_run(r#"
        (defn identity [x] x)
        (defn twice [f x] (f (f x)))
        (defn main [] (do
            (print (identity 42))
            (print (twice (fn [x] (+ x 1)) 10))
            0))
    "#);
    assert_eq!(output.trim(), "42\n12");
}

/// stdlib/Core.ls の Option 型テスト (型チェックのみ - ADT は GC 型)
#[test]
fn test_e2e_stdlib_core_option_typecheck() {
    typecheck_only(r#"
        (type (Option a) (Some a) None)
        (defn unwrap [opt default]
          (match opt
            [(Some x) x]
            [None default]))
        (defn map-option [f opt]
          (match opt
            [(Some x) (Some (f x))]
            [None None]))
        (defn is-some [opt]
          (match opt
            [(Some _) 1]
            [None 0]))
        (defn main [] (print 0))
    "#);
}

/// stdlib/Core.ls の Result 型テスト (型チェックのみ - ADT は GC 型)
#[test]
fn test_e2e_stdlib_core_result_typecheck() {
    typecheck_only(r#"
        (type (Result a e) (Ok a) (Err e))
        (defn unwrap-ok [res default]
          (match res
            [(Ok x) x]
            [(Err _) default]))
        (defn map-result [f res]
          (match res
            [(Ok x) (Ok (f x))]
            [(Err e) (Err e)]))
        (defn is-ok [res]
          (match res
            [(Ok _) 1]
            [(Err _) 0]))
        (defn main [] (print 0))
    "#);
}

/// stdlib/List.ls のリスト型テスト (型チェックのみ - ADT は GC 型)
#[test]
fn test_e2e_stdlib_list_typecheck() {
    typecheck_only(r#"
        (type (List a) (Cons a (List a)) Nil)
        (defn length [xs]
          (match xs
            [Nil 0]
            [(Cons _ t) (+ 1 (length t))]))
        (defn map [f xs]
          (match xs
            [Nil Nil]
            [(Cons h t) (Cons (f h) (map f t))]))
        (defn filter [f xs]
          (match xs
            [Nil Nil]
            [(Cons h t) (if (f h) (Cons h (filter f t)) (filter f t))]))
        (defn fold [f init xs]
          (match xs
            [Nil init]
            [(Cons h t) (fold f (f init h) t)]))
        (defn append [xs ys]
          (match xs
            [Nil ys]
            [(Cons h t) (Cons h (append t ys))]))
        (defn reverse [xs]
          (fold (fn [acc x] (Cons x acc)) Nil xs))
        (defn nth [xs n default]
          (match xs
            [Nil default]
            [(Cons h t) (if (== n 0) h (nth t (- n 1) default))]))
        (defn take [n xs]
          (if (<= n 0) Nil
            (match xs
              [Nil Nil]
              [(Cons h t) (Cons h (take (- n 1) t))])))
        (defn drop [n xs]
          (if (<= n 0) xs
            (match xs
              [Nil Nil]
              [(Cons _ t) (drop (- n 1) t)])))
        (defn main [] (print 0))
    "#);
}

/// stdlib/String.ls の文字列操作テスト (starts-with, ends-with)
#[test]
fn test_e2e_stdlib_string_starts_ends_with() {
    let output = compile_and_run(r#"
        (defn starts-with [s prefix]
          (if (> (string-length prefix) (string-length s))
            false
            (string-eq (substring s 0 (string-length prefix)) prefix)))
        (defn ends-with [s suffix]
          (let [slen (string-length s)
                suflen (string-length suffix)]
            (if (> suflen slen)
              false
              (string-eq (substring s (- slen suflen) slen) suffix))))
        (defn main [] (do
            (print (if (starts-with "hello world" "hello") 1 0))
            (print (if (starts-with "hello" "hello world") 1 0))
            (print (if (ends-with "hello world" "world") 1 0))
            (print (if (ends-with "hi" "hello") 1 0))
            0))
    "#);
    assert_eq!(output.trim(), "1\n0\n1\n0");
}

/// stdlib/String.ls の string-repeat テスト
#[test]
fn test_e2e_stdlib_string_repeat() {
    let output = compile_and_run(r#"
        (defn string-repeat [s n]
          (if (<= n 0) ""
            (if (== n 1) s
              (string-concat s (string-repeat s (- n 1))))))
        (defn main [] (do
            (print (string-length (string-repeat "ab" 3)))
            (print (string-length (string-repeat "x" 1)))
            (print (string-length (string-repeat "y" 0)))
            (print (if (string-eq (string-repeat "ab" 3) "ababab") 1 0))
            0))
    "#);
    assert_eq!(output.trim(), "6\n1\n0\n1");
}

/// stdlib/String.ls の string-contains テスト
#[test]
fn test_e2e_stdlib_string_contains() {
    let output = compile_and_run(r#"
        (defn string-search-from [haystack needle hlen nlen i]
          (if (> (+ i nlen) hlen)
            (- 0 1)
            (if (string-eq (substring haystack i (+ i nlen)) needle)
              i
              (string-search-from haystack needle hlen nlen (+ i 1)))))
        (defn string-index-of [haystack needle]
          (let [hlen (string-length haystack)
                nlen (string-length needle)]
            (if (> nlen hlen)
              (- 0 1)
              (string-search-from haystack needle hlen nlen 0))))
        (defn string-contains [haystack needle]
          (if (>= (string-index-of haystack needle) 0) 1 0))
        (defn main [] (do
            (print (string-contains "hello world" "lo wo"))
            (print (string-contains "hello" "xyz"))
            (print (string-contains "abc" "abc"))
            (print (string-contains "abc" ""))
            0))
    "#);
    assert_eq!(output.trim(), "1\n0\n1\n1");
}

/// stdlib/String.ls の string-index-of テスト
#[test]
fn test_e2e_stdlib_string_index_of() {
    let output = compile_and_run(r#"
        (defn string-search-from [haystack needle hlen nlen i]
          (if (> (+ i nlen) hlen)
            (- 0 1)
            (if (string-eq (substring haystack i (+ i nlen)) needle)
              i
              (string-search-from haystack needle hlen nlen (+ i 1)))))
        (defn string-index-of [haystack needle]
          (let [hlen (string-length haystack)
                nlen (string-length needle)]
            (if (> nlen hlen)
              (- 0 1)
              (string-search-from haystack needle hlen nlen 0))))
        (defn main [] (do
            (print (string-index-of "hello world" "world"))
            (print (string-index-of "hello" "xyz"))
            (print (string-index-of "abcdef" "cd"))
            0))
    "#);
    assert_eq!(output.trim(), "6\n-1\n2");
}

// === stdlib コンパイル・実行テスト ===

#[test]
fn test_e2e_stdlib_char() {
    // Char.ls: 文字判定関数
    let result = compile_and_run(r#"
        (defn is-digit [c]
          (if (>= c 48) (<= c 57) false))
        (defn is-upper [c]
          (if (>= c 65) (<= c 90) false))
        (defn is-lower [c]
          (if (>= c 97) (<= c 122) false))
        (defn is-alpha [c]
          (if (is-upper c) true (is-lower c)))
        (defn is-whitespace [c]
          (if (== c 32) true
            (if (== c 9) true
              (if (== c 10) true
                (== c 13)))))
        (defn main []
          (do
            (print (is-digit 48))
            (print (is-digit 65))
            (print (is-alpha 65))
            (print (is-alpha 48))
            (print (is-whitespace 32))
            0))
    "#);
    // 48='0' is digit=1, 65='A' is not digit=0, 65='A' is alpha=1, 48='0' is not alpha=0, 32=' ' is whitespace=1
    assert_eq!(result.trim(), "1\n0\n1\n0\n1");
}

#[test]
fn test_e2e_stdlib_debug() {
    // Debug.ls: デバッグ・アサーション関数
    let result = compile_and_run(r#"
        (defn debug-print [x]
          (do (print x) x))
        (defn assert [cond]
          (if cond 0 0))
        (defn assert-eq [a b]
          (assert (== a b)))
        (defn main []
          (do
            (assert true)
            (assert-eq 42 42)
            (print (debug-print 99))
            0))
    "#);
    // debug-print prints 99, then main prints the return value 99 again
    assert_eq!(result.trim(), "99\n99");
}
