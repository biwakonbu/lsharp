(defn source-review-attestation []
  :review "review:checkout/reviewer-001" "sha256:review-001" "redacted"
  :review-attestation
    :review-id "review:checkout/reviewer-001"
    :subject-digest "sha256:subject-001"
    :source-commit "0123456789abcdef"
    :provenance-digest "sha256:review-001"
    :provider "github"
    :key-id "org/reviews-2026"
    :algorithm "ed25519"
    :signature "AAECAw"
    :issued-at "2026-08-01T00:00:00Z"
    :expires-at "2026-09-01T00:00:00Z"
    :sequence 3
  true)
