(module Checkout
  (private
    (defn first []
      :intent "intent:checkout/same" "first declaration"
      true))
  (impl (Show Int)
    (defn second []
      :intent "intent:checkout/same" "second declaration"
      true)))
