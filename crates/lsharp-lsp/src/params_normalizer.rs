/// tower-lsp の `FromParams for ()` が `params: null` や `params: {}` を拒否する問題を
/// 回避するミドルウェア。`shutdown` 等のパラメータなしメソッドで `null` や空オブジェクトが
/// 送られた場合、params を除去してから内部サービスへ転送する。
use serde_json::Value;
use std::task::{Context, Poll};
use tower_lsp::jsonrpc::{Request, Response};

/// パラメータなしの LSP メソッド一覧
const PARAMLESS_METHODS: &[&str] = &["shutdown"];

/// params が意味的に空かどうかを判定する
fn is_empty_params(v: &Value) -> bool {
    v.is_null() || (v.is_object() && v.as_object().unwrap().is_empty())
}

pub struct ParamsNormalizer<S> {
    inner: S,
}

impl<S> ParamsNormalizer<S> {
    pub fn new(inner: S) -> Self {
        Self { inner }
    }
}

impl<S> tower_service::Service<Request> for ParamsNormalizer<S>
where
    S: tower_service::Service<Request, Response = Option<Response>>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<std::result::Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let needs_strip =
            PARAMLESS_METHODS.contains(&req.method()) && req.params().is_some_and(is_empty_params);

        if needs_strip {
            // params を除去した新しい Request を構築
            let (method, id, _params) = req.into_parts();
            let mut builder = Request::build(method);
            if let Some(id) = id {
                builder = builder.id(id);
            }
            self.inner.call(builder.finish())
        } else {
            self.inner.call(req)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_is_empty_params_null() {
        assert!(is_empty_params(&Value::Null));
    }

    #[test]
    fn test_is_empty_params_empty_object() {
        assert!(is_empty_params(&json!({})));
    }

    #[test]
    fn test_is_empty_params_non_empty_object() {
        assert!(!is_empty_params(&json!({"key": "value"})));
    }

    #[test]
    fn test_is_empty_params_number() {
        assert!(!is_empty_params(&json!(42)));
    }

    #[test]
    fn test_shutdown_request_params_null_stripped() {
        // params: null の shutdown リクエストから params が除去されることを確認
        let req = Request::build("shutdown")
            .params(Value::Null)
            .id(1)
            .finish();
        assert!(req.params().is_some());

        let needs_strip =
            PARAMLESS_METHODS.contains(&req.method()) && req.params().is_some_and(is_empty_params);
        assert!(needs_strip);

        let (method, id, _) = req.into_parts();
        let mut builder = Request::build(method);
        if let Some(id) = id {
            builder = builder.id(id);
        }
        let stripped = builder.finish();
        assert!(stripped.params().is_none());
        assert_eq!(stripped.method(), "shutdown");
    }

    #[test]
    fn test_shutdown_request_params_empty_object_stripped() {
        let req = Request::build("shutdown").params(json!({})).id(2).finish();
        let needs_strip =
            PARAMLESS_METHODS.contains(&req.method()) && req.params().is_some_and(is_empty_params);
        assert!(needs_strip);
    }

    #[test]
    fn test_shutdown_request_with_non_empty_params_is_preserved() {
        let req = Request::build("shutdown")
            .params(json!({"unexpected": true}))
            .id(4)
            .finish();
        let needs_strip =
            PARAMLESS_METHODS.contains(&req.method()) && req.params().is_some_and(is_empty_params);
        assert!(!needs_strip);
        assert_eq!(req.params(), Some(&json!({"unexpected": true})));
    }

    #[test]
    fn test_non_shutdown_method_not_stripped() {
        let req = Request::build("textDocument/hover")
            .params(json!({"textDocument": {"uri": "file:///test.ls"}}))
            .id(3)
            .finish();
        let needs_strip =
            PARAMLESS_METHODS.contains(&req.method()) && req.params().is_some_and(is_empty_params);
        assert!(!needs_strip);
    }
}
