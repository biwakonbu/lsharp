(module Tools.Test.AssuranceText)

;; structured assurance report を人間向けの deterministic な行形式へ射影する。
;; 集計は各 CLI の既存 JSON helper が担い、この module は表示契約だけを共有する。

(defn assurance-text-option [] 2)

(defn assurance-text-line [label value]
  (string-concat label (string-concat ": " value)))

(defn assurance-text-append [out line]
  (if (= (string-length out) 0)
    line
    (string-concat out (string-concat "\n" line))))

(defn assurance-text-contracts [examples invariants assertions cases properties]
  (+ (vector-length examples)
    (+ (vector-length invariants)
      (+ (vector-length assertions)
        (+ (vector-length cases) (vector-length properties))))))

(defn assurance-text-report
  [status method generator contracts cases discarded seed shrinks executed failed
   diagnostics first-code span-start span-end message runner target]
  (let [line0 (assurance-text-line "schema_version" "1")
    line1 (assurance-text-append line0 (assurance-text-line
      "implementation_conformance.status" status))
    line2 (assurance-text-append line1 (assurance-text-line
      "implementation_conformance.method" method))
    line3 (assurance-text-append line2 (assurance-text-line
      "implementation_conformance.generator" generator))
    line4 (assurance-text-append line3 (assurance-text-line
      "implementation_conformance.contracts" (int-to-string contracts)))
    line5 (assurance-text-append line4 (assurance-text-line
      "implementation_conformance.cases" (int-to-string cases)))
    line6 (assurance-text-append line5 (assurance-text-line
      "implementation_conformance.discarded_cases" discarded))
    line7 (assurance-text-append line6 (assurance-text-line
      "implementation_conformance.seed" (int-to-string seed)))
    line8 (assurance-text-append line7 (assurance-text-line
      "implementation_conformance.shrinks" shrinks))
    line9 (assurance-text-append line8 (assurance-text-line
      "implementation_conformance.coverage.executed" (int-to-string executed)))
    line10 (assurance-text-append line9 (assurance-text-line
      "implementation_conformance.coverage.failed" (int-to-string failed)))
    line11 (assurance-text-append line10 (assurance-text-line
      "implementation_conformance.diagnostics.count" (int-to-string diagnostics)))
    line12 (assurance-text-append line11 (assurance-text-line
      "implementation_conformance.diagnostics.firstErrorCode" (int-to-string first-code)))
    line13 (assurance-text-append line12 (assurance-text-line
      "implementation_conformance.diagnostics.firstErrorSpan.start"
      (int-to-string span-start)))
    line14 (assurance-text-append line13 (assurance-text-line
      "implementation_conformance.diagnostics.firstErrorSpan.end"
      (int-to-string span-end)))
    line15 (assurance-text-append line14 (assurance-text-line
      "implementation_conformance.diagnostics.message" message))
    line16 (assurance-text-append line15 (assurance-text-line
      "implementation_conformance.runner" runner))
    line17 (assurance-text-append line16 (assurance-text-line
      "implementation_conformance.target" target))
    line18 (assurance-text-append line17 (assurance-text-line
      "implementation_conformance.provenance.producer" "lsharp-selfhost"))
    line19 (assurance-text-append line18 (assurance-text-line
      "implementation_conformance.provenance.tool_version" "0.1.0"))
    line20 (assurance-text-append line19 (assurance-text-line
      "implementation_conformance.provenance.source_digest" "unknown"))
    line21 (assurance-text-append line20 (assurance-text-line
      "implementation_conformance.provenance.source_commit" "unknown"))
    line22 (assurance-text-append line21 (assurance-text-line
      "implementation_conformance.provenance.artifact_digest" "unknown"))
    line23 (assurance-text-append line22 (assurance-text-line
      "implementation_conformance.provenance.timestamp" "unknown"))
    line24 (assurance-text-append line23 (assurance-text-line
      "intent_validation.status" "unknown"))
    line25 (assurance-text-append line24 (assurance-text-line
      "intent_validation.open_questions" "unknown"))
    line26 (assurance-text-append line25 (assurance-text-line
      "intent_validation.independent_reviews" "unknown"))]
    (assurance-text-append line26 (assurance-text-line
      "intent_validation.contradicting_observations" "unknown"))))

(defn assurance-text-failed? [failed diagnostics]
  (if (or (> failed 0) (> diagnostics 0)) 1 0))
