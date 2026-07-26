
/// NATIVE-REAL-08q: x86_64 で 11 引数 direct call bundle が 5 stack arg を持つこと
#[test]
#[ignore]
fn test_native_codegen_emits_x86_direct_call_eleven_arg_bundle_bytes() {
    let output = run_native_codegen_harness(
        r#"(module Main)
(import NativeTarget)
(import NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn main []
  (let [caller-ir (vector-push
                    (vector-push
                      (vector-push
                        (vector-push
                          (vector-push
                            (vector-push
                              (vector-push
                                (vector-push
                                  (vector-push
                                    (vector-push
                                      (vector-push
                                        (vector-push (vector-new 12) (make-instr 3 40))
                                        (make-instr 3 2))
                                      (make-instr 3 5))
                                    (make-instr 3 7))
                                  (make-instr 3 11))
                                (make-instr 3 14))
                              (make-instr 3 17))
                            (make-instr 3 19))
                          (make-instr 3 23))
                        (make-instr 3 29))
                      (make-instr 3 31))
                    (make-call 1))
        callee-ir-head (vector-push
                         (vector-push
                           (vector-push
                             (vector-push
                               (vector-push
                                 (vector-push
                                   (vector-push
                                     (vector-push
                                       (vector-push
                                         (vector-push
                                           (vector-push
                                             (vector-push
                                               (vector-push (vector-new 21) (make-local-get 0))
                                               (make-local-get 1))
                                             (make-instr 24 0))
                                           (make-local-get 2))
                                         (make-instr 24 0))
                                       (make-local-get 3))
                                     (make-instr 24 0))
                                   (make-local-get 4))
                                 (make-instr 24 0))
                               (make-local-get 5))
                             (make-instr 24 0))
                           (make-local-get 6))
                         (make-instr 24 0))
        callee-ir-mid (vector-push
                        (vector-push callee-ir-head (make-local-get 7))
                        (make-instr 24 0))
        callee-ir-tail (vector-push
                         (vector-push callee-ir-mid (make-local-get 8))
                         (make-instr 24 0))
        callee-ir-more (vector-push
                         (vector-push callee-ir-tail (make-local-get 9))
                         (make-instr 24 0))
        callee-ir (vector-push
                    (vector-push callee-ir-more (make-local-get 10))
                    (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 11 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (make-target 1)
        native (emit-native-function-meta-bundle functions target)]
    (do
      (print (vector-length native))
      (print (vector-get native 666))
      (print (vector-get native 667))
      (print (vector-get native 668))
      (print (vector-get native 669))
      (print (vector-get native 670))
      (print (vector-get native 671))
      (print (vector-get native 672))
      (print (vector-get native 673))
      (print (vector-get native 674))
      (print (vector-get native 675))
      (print (vector-get native 676))
      (print (vector-get native 677))
      (print (vector-get native 678))
      (print (vector-get native 679))
      (print (vector-get native 680))
      (print (vector-get native 681))
      (print (vector-get native 682))
      (print (vector-get native 683))
      (print (vector-get native 684))
      (print (vector-get native 685))
      (print (vector-get native 686))
      (print (vector-get native 687))
      (print (vector-get native 688))
      (print (vector-get native 689))
      (print (vector-get native 690))
      (print (vector-get native 691))
      (print (vector-get native 692))
      (print (vector-get native 693))
      (print (vector-get native 694))
      (print (vector-get native 695))
      (print (vector-get native 696))
      (print (vector-get native 697))
      (print (vector-get native 698))
      (print (vector-get native 699))
      (print (vector-get native 700))
      (print (vector-get native 701))
      (print (vector-get native 702))
      (print (vector-get native 703))
      (print (vector-get native 704))
      (print (vector-get native 705))
      (print (vector-get native 706))
      (print (vector-get native 707))
      (print (vector-get native 708))
      (print (vector-get native 709))
      (print (vector-get native 710))
      (print (vector-get native 711))
      (print (vector-get native 712))
      (print (vector-get native 713))
      (print (vector-get native 714))
      (print (vector-get native 715))
      (print (vector-get native 716))
      (print (vector-get native 717))
      (print (vector-get native 718))
      (print (vector-get native 719))
      (print (vector-get native 720))
      (print (vector-get native 721))
      (print (vector-get native 722))
      (print (vector-get native 723))
      (print (vector-get native 724))
      (print (vector-get native 725))
      (print (vector-get native 726))
      (print (vector-get native 727))
      (print (vector-get native 728))
      (print (vector-get native 729))
      (print (vector-get native 730))
      (print (vector-get native 731))
      (print (vector-get native 732))
      (print (vector-get native 733))
      (print (vector-get native 734))
      (print (vector-get native 735))
      (print (vector-get native 736))
      (print (vector-get native 737))
      (print (vector-get native 738))
      (print (vector-get native 739))
      (print (vector-get native 740))
      (print (vector-get native 741))
      (print (vector-get native 742))
      (print (vector-get native 743))
      (print (vector-get native 744))
      (print (vector-get native 745))
      (print (vector-get native 746))
      (print (vector-get native 747))
      (print (vector-get native 748))
      (print (vector-get native 749))
      (print (vector-get native 750))
      (print (vector-get native 751))
      (print (vector-get native 752))
      (print (vector-get native 753))
      (print (vector-get native 754))
      (print (vector-get native 755))
      (print (vector-get native 756))
      (print (vector-get native 757))
      (print (vector-get native 758))
      (print (vector-get native 759))
      (print (vector-get native 760))
      (print (vector-get native 761))
      (print (vector-get native 762))
      (print (vector-get native 763))
      (print (vector-get native 764))
      (print (vector-get native 765))
      (print (vector-get native 766))
      (print (vector-get native 767))
      (print (vector-get native 768))
      (print (vector-get native 769))
      (print (vector-get native 770))
      (print (vector-get native 771))
      (print (vector-get native 792))
      (print (vector-get native 793))
      (print (vector-get native 794))
      (print (vector-get native 834))
      (print (vector-get native 835))
      (print (vector-get native 836))
      (print (vector-get native 837))
      (print (vector-get native 838))
      (print (vector-get native 839))
      (print (vector-get native 840))
      (print (vector-get native 841))
      (print (vector-get native 842))
      (print (vector-get native 843))
      (print (vector-get native 844))
      (print (vector-get native 845))
      (print (vector-get native 846))
      (print (vector-get native 847))
      (print (vector-get native 848))
      (print (vector-get native 849))
      (print (vector-get native 850))
      (print (vector-get native 851))
      (print (vector-get native 852))
      (print (vector-get native 853))
      (print (vector-get native 854))
      (print (vector-get native 855))
      (print (vector-get native 856))
      (print (vector-get native 857))
      (print (vector-get native 858))
      (print (vector-get native 859))
      (print (vector-get native 860))
      (print (vector-get native 861))
      (print (vector-get native 862))
      (print (vector-get native 863))
      (print (vector-get native 864))
      (print (vector-get native 865))
      (print (vector-get native 866))
      (print (vector-get native 867))
      (print (vector-get native 868))
      (print (vector-get native 869))
      (print (vector-get native 870))
      (print (vector-get native 871))
      (print (vector-get native 872))
      (print (vector-get native 873))
      (print (vector-get native 874))
      (print (vector-get native 875))
      (print (vector-get native 876))
      (print (vector-get native 877))
      (print (vector-get native 878))
      (print (vector-get native 879))
      (print (vector-get native 880))
      (print (vector-get native 881))
      (print (vector-get native 882))
      (print (vector-get native 883))
      (print (vector-get native 884))
      (print (vector-get native 885))
      (print (vector-get native 886))
      (print (vector-get native 887))
      (print (vector-get native 888))
      (print (vector-get native 1026))
      (print (vector-get native 1027))
      0)))"#,
    );

    let lines: Vec<&str> = output.trim().lines().collect();
    let expected = [
        "1028", "72", "129", "236", "48", "0", "0", "0", "72", "137", "68", "36", "32", "72",
        "137", "76", "36", "24", "72", "139", "141", "248", "255", "255", "255", "72", "137", "76",
        "36", "16", "72", "139", "141", "240", "255", "255", "255", "72", "137", "76", "36", "8",
        "76", "139", "141", "232", "255", "255", "255", "76", "137", "12", "36", "76", "139",
        "141", "224", "255", "255", "255", "76", "139", "133", "216", "255", "255", "255", "72",
        "139", "141", "208", "255", "255", "255", "72", "139", "149", "200", "255", "255", "255",
        "72", "139", "181", "192", "255", "255", "255", "72", "139", "189", "184", "255", "255",
        "255", "232", "16", "0", "0", "0", "72", "129", "196", "48", "0", "0", "0", "72", "137",
        "189", "72", "139", "69", "16", "72", "137", "133", "200", "255", "255", "255", "72",
        "139", "69", "24", "72", "137", "133", "192", "255", "255", "255", "72", "139", "69", "32",
        "72", "137", "133", "184", "255", "255", "255", "72", "139", "69", "40", "72", "137",
        "133", "176", "255", "255", "255", "72", "139", "69", "48", "72", "137", "133", "168",
        "255", "255", "255", "93", "195",
    ];

    assert!(
        lines.len() >= expected.len(),
        "x86 direct call eleven-arg bundle bytes 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        &lines[..expected.len()],
        expected,
        "x86_64 direct call eleven-arg bundle payload/call-layout exact bytes が一致しない"
    );
}

/// NATIVE-REAL-08r: x86_64 で 12 引数 direct call bundle が 6 stack arg を持つこと
#[test]
#[ignore]
fn test_native_codegen_emits_x86_direct_call_twelve_arg_bundle_bytes() {
    let output = run_native_codegen_harness(
        r#"(module Main)
(import NativeTarget)
(import NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn main []
  (let [caller-ir (vector-push
                    (vector-push
                      (vector-push
                        (vector-push
                          (vector-push
                            (vector-push
                              (vector-push
                                (vector-push
                                  (vector-push
                                    (vector-push
                                      (vector-push
                                        (vector-push
                                          (vector-push (vector-new 13) (make-instr 3 40))
                                          (make-instr 3 2))
                                        (make-instr 3 5))
                                      (make-instr 3 7))
                                    (make-instr 3 11))
                                  (make-instr 3 14))
                                (make-instr 3 17))
                              (make-instr 3 19))
                            (make-instr 3 23))
                          (make-instr 3 29))
                        (make-instr 3 31))
                      (make-instr 3 37))
                    (make-call 1))
        callee-ir-head (vector-push
                         (vector-push
                           (vector-push
                             (vector-push
                               (vector-push
                                 (vector-push
                                   (vector-push
                                     (vector-push
                                       (vector-push
                                         (vector-push
                                           (vector-push
                                             (vector-push
                                               (vector-push (vector-new 23) (make-local-get 0))
                                               (make-local-get 1))
                                             (make-instr 24 0))
                                           (make-local-get 2))
                                         (make-instr 24 0))
                                       (make-local-get 3))
                                     (make-instr 24 0))
                                   (make-local-get 4))
                                 (make-instr 24 0))
                               (make-local-get 5))
                             (make-instr 24 0))
                           (make-local-get 6))
                         (make-instr 24 0))
        callee-ir-mid (vector-push
                        (vector-push callee-ir-head (make-local-get 7))
                        (make-instr 24 0))
        callee-ir-tail (vector-push
                         (vector-push callee-ir-mid (make-local-get 8))
                         (make-instr 24 0))
        callee-ir-more (vector-push
                         (vector-push callee-ir-tail (make-local-get 9))
                         (make-instr 24 0))
        callee-ir-last (vector-push
                         (vector-push callee-ir-more (make-local-get 10))
                         (make-instr 24 0))
        callee-ir (vector-push
                    (vector-push callee-ir-last (make-local-get 11))
                    (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 12 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (make-target 1)
        native (emit-native-function-meta-bundle functions target)]
    (do
      (print (vector-length native))
      (print (vector-get native 807))
      (print (vector-get native 808))
      (print (vector-get native 809))
      (print (vector-get native 810))
      (print (vector-get native 811))
      (print (vector-get native 812))
      (print (vector-get native 813))
      (print (vector-get native 814))
      (print (vector-get native 815))
      (print (vector-get native 816))
      (print (vector-get native 817))
      (print (vector-get native 818))
      (print (vector-get native 819))
      (print (vector-get native 820))
      (print (vector-get native 821))
      (print (vector-get native 822))
      (print (vector-get native 823))
      (print (vector-get native 824))
      (print (vector-get native 825))
      (print (vector-get native 826))
      (print (vector-get native 827))
      (print (vector-get native 828))
      (print (vector-get native 829))
      (print (vector-get native 830))
      (print (vector-get native 831))
      (print (vector-get native 832))
      (print (vector-get native 833))
      (print (vector-get native 834))
      (print (vector-get native 835))
      (print (vector-get native 836))
      (print (vector-get native 837))
      (print (vector-get native 838))
      (print (vector-get native 839))
      (print (vector-get native 840))
      (print (vector-get native 841))
      (print (vector-get native 842))
      (print (vector-get native 843))
      (print (vector-get native 844))
      (print (vector-get native 845))
      (print (vector-get native 846))
      (print (vector-get native 847))
      (print (vector-get native 848))
      (print (vector-get native 849))
      (print (vector-get native 850))
      (print (vector-get native 851))
      (print (vector-get native 852))
      (print (vector-get native 853))
      (print (vector-get native 854))
      (print (vector-get native 855))
      (print (vector-get native 856))
      (print (vector-get native 857))
      (print (vector-get native 858))
      (print (vector-get native 859))
      (print (vector-get native 860))
      (print (vector-get native 861))
      (print (vector-get native 862))
      (print (vector-get native 863))
      (print (vector-get native 864))
      (print (vector-get native 865))
      (print (vector-get native 866))
      (print (vector-get native 867))
      (print (vector-get native 868))
      (print (vector-get native 869))
      (print (vector-get native 870))
      (print (vector-get native 871))
      (print (vector-get native 872))
      (print (vector-get native 873))
      (print (vector-get native 874))
      (print (vector-get native 875))
      (print (vector-get native 876))
      (print (vector-get native 877))
      (print (vector-get native 878))
      (print (vector-get native 879))
      (print (vector-get native 880))
      (print (vector-get native 881))
      (print (vector-get native 882))
      (print (vector-get native 883))
      (print (vector-get native 884))
      (print (vector-get native 885))
      (print (vector-get native 886))
      (print (vector-get native 887))
      (print (vector-get native 888))
      (print (vector-get native 889))
      (print (vector-get native 890))
      (print (vector-get native 891))
      (print (vector-get native 892))
      (print (vector-get native 893))
      (print (vector-get native 894))
      (print (vector-get native 895))
      (print (vector-get native 896))
      (print (vector-get native 897))
      (print (vector-get native 898))
      (print (vector-get native 899))
      (print (vector-get native 900))
      (print (vector-get native 901))
      (print (vector-get native 902))
      (print (vector-get native 903))
      (print (vector-get native 904))
      (print (vector-get native 905))
      (print (vector-get native 906))
      (print (vector-get native 907))
      (print (vector-get native 908))
      (print (vector-get native 909))
      (print (vector-get native 910))
      (print (vector-get native 911))
      (print (vector-get native 912))
      (print (vector-get native 913))
      (print (vector-get native 914))
      (print (vector-get native 915))
      (print (vector-get native 916))
      (print (vector-get native 917))
      (print (vector-get native 918))
      (print (vector-get native 919))
      (print (vector-get native 920))
      (print (vector-get native 921))
      (print (vector-get native 922))
      (print (vector-get native 923))
      (print (vector-get native 924))
      (print (vector-get native 945))
      (print (vector-get native 946))
      (print (vector-get native 947))
      (print (vector-get native 987))
      (print (vector-get native 988))
      (print (vector-get native 989))
      (print (vector-get native 990))
      (print (vector-get native 991))
      (print (vector-get native 992))
      (print (vector-get native 993))
      (print (vector-get native 994))
      (print (vector-get native 995))
      (print (vector-get native 996))
      (print (vector-get native 997))
      (print (vector-get native 998))
      (print (vector-get native 999))
      (print (vector-get native 1000))
      (print (vector-get native 1001))
      (print (vector-get native 1002))
      (print (vector-get native 1003))
      (print (vector-get native 1004))
      (print (vector-get native 1005))
      (print (vector-get native 1006))
      (print (vector-get native 1007))
      (print (vector-get native 1008))
      (print (vector-get native 1009))
      (print (vector-get native 1010))
      (print (vector-get native 1011))
      (print (vector-get native 1012))
      (print (vector-get native 1013))
      (print (vector-get native 1014))
      (print (vector-get native 1015))
      (print (vector-get native 1016))
      (print (vector-get native 1017))
      (print (vector-get native 1018))
      (print (vector-get native 1019))
      (print (vector-get native 1020))
      (print (vector-get native 1021))
      (print (vector-get native 1022))
      (print (vector-get native 1023))
      (print (vector-get native 1024))
      (print (vector-get native 1025))
      (print (vector-get native 1026))
      (print (vector-get native 1027))
      (print (vector-get native 1028))
      (print (vector-get native 1029))
      (print (vector-get native 1030))
      (print (vector-get native 1031))
      (print (vector-get native 1032))
      (print (vector-get native 1033))
      (print (vector-get native 1034))
      (print (vector-get native 1035))
      (print (vector-get native 1036))
      (print (vector-get native 1037))
      (print (vector-get native 1038))
      (print (vector-get native 1039))
      (print (vector-get native 1040))
      (print (vector-get native 1041))
      (print (vector-get native 1042))
      (print (vector-get native 1043))
      (print (vector-get native 1044))
      (print (vector-get native 1045))
      (print (vector-get native 1046))
      (print (vector-get native 1047))
      (print (vector-get native 1048))
      (print (vector-get native 1049))
      (print (vector-get native 1050))
      (print (vector-get native 1051))
      (print (vector-get native 1052))
      (print (vector-get native 1202))
      (print (vector-get native 1203))
      0)))"#,
    );

    let lines: Vec<&str> = output.trim().lines().collect();
    let expected = [
        "1204", "72", "129", "236", "48", "0", "0", "0", "72", "137", "68", "36", "40", "72",
        "137", "76", "36", "32", "72", "139", "141", "248", "255", "255", "255", "72", "137", "76",
        "36", "24", "72", "139", "141", "240", "255", "255", "255", "72", "137", "76", "36", "16",
        "72", "139", "141", "232", "255", "255", "255", "72", "137", "76", "36", "8", "76", "139",
        "141", "224", "255", "255", "255", "76", "137", "12", "36", "76", "139", "141", "216",
        "255", "255", "255", "76", "139", "133", "208", "255", "255", "255", "72", "139", "141",
        "200", "255", "255", "255", "72", "139", "149", "192", "255", "255", "255", "72", "139",
        "181", "184", "255", "255", "255", "72", "139", "189", "176", "255", "255", "255", "232",
        "16", "0", "0", "0", "72", "129", "196", "48", "0", "0", "0", "72", "137", "189", "72",
        "139", "69", "16", "72", "137", "133", "200", "255", "255", "255", "72", "139", "69", "24",
        "72", "137", "133", "192", "255", "255", "255", "72", "139", "69", "32", "72", "137",
        "133", "184", "255", "255", "255", "72", "139", "69", "40", "72", "137", "133", "176",
        "255", "255", "255", "72", "139", "69", "48", "72", "137", "133", "168", "255", "255",
        "255", "72", "139", "69", "56", "72", "137", "133", "160", "255", "255", "255", "93",
        "195",
    ];

    assert!(
        lines.len() >= expected.len(),
        "x86 direct call twelve-arg bundle bytes 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        &lines[..expected.len()],
        expected,
        "x86_64 direct call twelve-arg bundle payload/call-layout exact bytes が一致しない"
    );
}

/// NATIVE-REAL-08s: x86_64 で 13 引数 direct call bundle が 7 stack arg を持つこと
#[test]
#[ignore]
fn test_native_codegen_emits_x86_direct_call_thirteen_arg_bundle_bytes() {
    let output = run_native_codegen_harness(
        r#"(module Main)
(import NativeTarget)
(import NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-range [bytes idx end]
  (if (>= idx end)
    0
    (do
      (print (vector-get bytes idx))
      (print-range bytes (+ idx 1) end))))

(defn main []
  (let [caller-ir (vector-push
                    (vector-push
                      (vector-push
                        (vector-push
                          (vector-push
                            (vector-push
                              (vector-push
                                (vector-push
                                  (vector-push
                                    (vector-push
                                      (vector-push
                                        (vector-push
                                          (vector-push
                                            (vector-push (vector-new 14) (make-instr 3 40))
                                            (make-instr 3 2))
                                          (make-instr 3 5))
                                        (make-instr 3 7))
                                      (make-instr 3 11))
                                    (make-instr 3 13))
                                  (make-instr 3 14))
                                (make-instr 3 17))
                              (make-instr 3 19))
                            (make-instr 3 23))
                          (make-instr 3 29))
                        (make-instr 3 31))
                      (make-instr 3 37))
                    (make-call 1))
        callee-ir-head (vector-push
                         (vector-push
                           (vector-push
                             (vector-push
                               (vector-push
                                 (vector-push
                                   (vector-push
                                     (vector-push
                                       (vector-push
                                         (vector-push
                                           (vector-push
                                             (vector-push
                                               (vector-push (vector-new 25) (make-local-get 0))
                                               (make-local-get 1))
                                             (make-instr 24 0))
                                           (make-local-get 2))
                                         (make-instr 24 0))
                                       (make-local-get 3))
                                     (make-instr 24 0))
                                   (make-local-get 4))
                                 (make-instr 24 0))
                               (make-local-get 5))
                             (make-instr 24 0))
                           (make-local-get 6))
                         (make-instr 24 0))
        callee-ir-mid (vector-push
                        (vector-push callee-ir-head (make-local-get 7))
                        (make-instr 24 0))
        callee-ir-tail (vector-push
                         (vector-push callee-ir-mid (make-local-get 8))
                         (make-instr 24 0))
        callee-ir-more (vector-push
                         (vector-push callee-ir-tail (make-local-get 9))
                         (make-instr 24 0))
        callee-ir-last (vector-push
                         (vector-push callee-ir-more (make-local-get 10))
                         (make-instr 24 0))
        callee-ir-next (vector-push
                         (vector-push callee-ir-last (make-local-get 11))
                         (make-instr 24 0))
        callee-ir (vector-push
                    (vector-push callee-ir-next (make-local-get 12))
                    (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 13 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        starts (collect-function-starts-x86 functions)
        caller-end (vector-get starts 1)
        call-start (- caller-end 139)
        spill-start (+ caller-end 11)
        target (make-target 1)
        native (emit-native-function-meta-bundle functions target)
        n (vector-length native)]
    (do
      (print n)
      (print-range native call-start (+ call-start 130))
      (print-range native spill-start (+ spill-start 119))
      (print (vector-get native (- n 2)))
      (print (vector-get native (- n 1)))
      0)))"#,
    );

    let lines: Vec<&str> = output.trim().lines().collect();
    let expected = [
        "1394", "72", "129", "236", "64", "0", "0", "0", "72", "137", "68", "36", "48", "72",
        "137", "76", "36", "40", "72", "139", "141", "248", "255", "255", "255", "72", "137", "76",
        "36", "32", "72", "139", "141", "240", "255", "255", "255", "72", "137", "76", "36", "24",
        "72", "139", "141", "232", "255", "255", "255", "72", "137", "76", "36", "16", "72", "139",
        "141", "224", "255", "255", "255", "72", "137", "76", "36", "8", "76", "139", "141", "216",
        "255", "255", "255", "76", "137", "12", "36", "76", "139", "141", "208", "255", "255",
        "255", "76", "139", "133", "200", "255", "255", "255", "72", "139", "141", "192", "255",
        "255", "255", "72", "139", "149", "184", "255", "255", "255", "72", "139", "181", "176",
        "255", "255", "255", "72", "139", "189", "168", "255", "255", "255", "232", "16", "0", "0",
        "0", "72", "129", "196", "64", "0", "0", "0", "72", "137", "189", "248", "255", "255",
        "255", "72", "137", "181", "240", "255", "255", "255", "72", "137", "149", "232", "255",
        "255", "255", "72", "137", "141", "224", "255", "255", "255", "76", "137", "133", "216",
        "255", "255", "255", "76", "137", "141", "208", "255", "255", "255", "72", "139", "69",
        "16", "72", "137", "133", "200", "255", "255", "255", "72", "139", "69", "24", "72", "137",
        "133", "192", "255", "255", "255", "72", "139", "69", "32", "72", "137", "133", "184",
        "255", "255", "255", "72", "139", "69", "40", "72", "137", "133", "176", "255", "255",
        "255", "72", "139", "69", "48", "72", "137", "133", "168", "255", "255", "255", "72",
        "139", "69", "56", "72", "137", "133", "160", "255", "255", "255", "72", "139", "69", "64",
        "72", "137", "133", "152", "255", "255", "255", "93", "195",
    ];

    assert!(
        lines.len() >= expected.len(),
        "x86 direct call thirteen-arg bundle bytes 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        &lines[..expected.len()],
        expected,
        "x86_64 direct call thirteen-arg bundle payload/call-layout exact bytes が一致しない"
    );
}
