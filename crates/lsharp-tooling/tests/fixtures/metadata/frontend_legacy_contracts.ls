(defn succ
  [x]
  :doc "DOC_MARKER_contract_inventory"
  :params [(x "PARAM_MARKER_input")]
  :returns "RETURN_MARKER_output"
  :example [(succ 0) (= (succ 1) 2)]
  :invariant (= result (+ x 1))
  (+ x 1))
