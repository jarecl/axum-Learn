//! 请求扩展模块
//!
//! 在 Spring Boot 中，这相当于：
//! - HttpServletRequest.setAttribute() / getAttribute() - 请求属性
//! - Filter 中设置的自定义请求属性
//! - @RequestScope - 请求作用域

use crate::{extract::rejection::*, response::IntoResponseParts};
use axum_core::extract::OptionalFromRequestParts;
use axum_core::{
    extract::FromRequestParts,
    response::{IntoResponse, Response, ResponseParts},
};
use http::{request::Parts, Extensions, Request};
use std::{
    convert::Infallible,
    task::{Context, Poll},
};
use tower_service::Service;

/// 扩展提取器和响应
///
/// ## 作为提取器
///
/// 这通常用于在处理器之间共享状态。
//!
/// 在 Spring Boot 中，这相当于：
//! - `@RequestAttribute` - 从请求作用域获取属性
//! - Filter 中通过 request.setAttribute() 设置的自定义属性
//!
/// ```rust,no_run
//! use axum::{
//!     Router,
//!     Extension,
//!     routing::get,
//! };
//! use std::sync::Arc;
//!
//! // 在整个应用程序中使用的某些共享状态
//! struct State {
//!     // ...
//! }
//!
//! async fn handler(state: Extension<Arc<State>>) {
//!     // ...
//! }
//!
//! let state = Arc::new(State { /* ... */ });
//!
//! let app = Router::new().route("/", get(handler))
//!     // 添加中间件，将状态插入到所有传入请求的扩展中
//!     // 类似于 Spring Boot Filter 中设置请求属性
//!     .layer(Extension(state));
//! # let _: Router = app;
;
//! ```
///
/// 如果扩展缺失，它将拒绝请求并返回 `500 Internal Server Error` 响应。
//! 或者，你可以使用 `Option<Extension<T>>` 使扩展提取器可选。
///
/// ## 作为响应
///
//! 响应扩展可用于与中间件共享状态。
//!
//! 在 Spring Boot 中，这相当于在 HandlerInterceptor.postHandle() 中设置响应属性
//!
//! ```rust
//! use axum::{
//!     Extension,
//!     response::IntoResponse,
//! };
//!
//! async fn handler() -> (Extension<Foo>, &'static str) {
//!     (
//!         Extension(Foo("foo")),
//!         "Hello, World!"
//!     )
//! }
//!
//! #[derive(Clone)]
//! struct Foo(&'static str);
//! ```
#[derive(Debug, Clone, Copy, Default)]
#[must_use]
pub struct Extension<T>(pub T);

impl<T> Extension<T>
where
    T: Clone + Send + Sync + 'static,
{
    // 从请求扩展中获取 Extension
    fn from_extensions(extensions: &Extensions) -> Option<Self> {
        extensions.get::<T>().cloned().map(Extension)
    }
}

// 为 Extension 实现 FromRequestParts
// 这使得 Extension 可以用作处理器参数
impl<T, S> FromRequestParts<S> for Extension<T>
where
    T: Clone + Send + Sync + 'static,
    S: Send + Sync,
{
    type Rejection = ExtensionRejection;

    async fn from_request_parts(req: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(Self::from_extensions(&req.extensions).ok_or_else(|| {
            MissingExtension::from_err(format!(
                "Extension of type `{}` was not found. Perhaps you forgot to add it? See `axum::Extension`.",
                std::any::type_name::<T>()
            ))
        })
    }
}

// 为 Extension 实现 OptionalFromRequestParts
// 这允许 Option<Extension<T>> 作为可选参数
impl<T, S> OptionalFromRequestParts<S> for Extension<T>
where
    T: Clone + Send + Sync + 'static,
    S: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request_parts(
        req: &mut Parts,
        _state: &S,
    ) -> Result<Option<Self>, Self::Rejection> {
        Ok(Self::from_extensions(&req.extensions))
    }
}

// 为 Extension 实现 Deref
axum_core::__impl_deref!(Extension);

// 为 Extension 实现 IntoResponseParts
// 这使得 Extension 可以作为响应的一部分返回
impl<T> IntoResponseParts for Extension<T>
where
    T: Clone + Send + Sync + 'static,
{
    type Error = Infallible;

    fn into_response_parts(self, mut res: ResponseParts) -> Result<ResponseParts, Self::Error> {
        res.extensions_mut().insert(self.0);
        Ok(res)
    }
}

// 为 Extension 实现 IntoResponse
impl<T> IntoResponse for Extension<T>
where
    T: Clone + Send + Sync + 'static,
{
    fn into_response(self) -> Response {
        let mut res = ().into_response();
        res.extensions_mut().insert(self.0);
        res
    }
}

// 为 Extension 实现 Layer
// 这使得 Extension 可以用作中间件层
impl<S, T> tower_layer::Layer<S> for Extension<T>
where
    T: Clone + Send + Sync + 'static,
{
    type Service = AddExtension<S, T>;

    fn layer(&self, inner: S) -> Self::Service {
        AddExtension {
            inner,
            value: self.0.clone(),
        }
    }
}

/// 用于向[请求扩展]添加某个可共享值的中间件
///
/// 在 Spring Boot 中，这相当于 Filter 中设置请求属性
//!
/// 更多信息请参阅 [从中间件向处理器传递状态](index.html#passing-state-from-middleware-to-handlers)
///
/// [request extensions]: https://docs.rs/http/latest/http/struct.Extensions.html
///
/// 如果你需要向每个请求添加扩展的层，使用 [Layer](tower::Layer) 的 [Extension] 实现。
///
/// ## 示例
///
/// ```rust
//! use axum::{
//!     Router,
//!     Extension,
//!     routing::get,
//! };
//! use std::sync::Arc;
//!
//! // 共享状态
//! #[derive(Clone)]
//! struct MyState {
//!     value: String,
//! }
//!
//! // 添加扩展的中间件
//! async fn add_state(req: Request, next: Next) -> impl IntoResponse {
//!     let state = MyState {
//!         value: "hello".to_string(),
//!     };
//!
//!     // 在请求中插入扩展
//!     req.extensions_mut().insert(state);
//!
//!     // 继续处理
//!     next.run(req).await
//! }
//!
//! let app = Router::new()
//!     .route("/", get(|state: Extension<MyState>| async {
//!         format!("Got: {}", state.value)
//!     }))
//!     .layer(axum::middleware::from_fn(add_state));
//! ```
#[derive(Clone, Copy, Debug)]
pub struct AddExtension<S, T> {
    pub(crate) inner: S,
    pub(crate) value: T,
}

// 为 AddExtension 实现 Service
impl<ResBody, S, T> Service<Request<ResBody>> for AddExtension<S, T>
where
    S: Service<Request<ResBody>>,
    T: Clone Clone + Send + Sync + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    #[inline]
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<ResBody>) -> Self::Future {
        // 在请求中插入扩展
        req.extensions_mut().insert(self.value.clone());
        self.inner.call(req)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routing::get;
    use crate::test_helpers::TestClient;
    use crate::Router;
    use http::StatusCode;

    #[derive(Clone)]
    struct Foo(String);

    #[derive(Clone)]
    struct Bar(String);

    #[crate::test]
    async fn extension_extractor() {
        // 需要 Foo 扩展的处理器
        async fn requires_foo(Extension(foo): Extension<Foo>) -> String {
            foo.0
        }

        // 可选 Foo 扩展的处理器
        async fn optional_foo(extension: Option<Extension<Foo>>) -> String {
            extension
                .map(|foo| foo.0.clone())
                .unwrap_or("none".to_owned())
        }

        // 需要 Bar 扩展的处理器
        async fn requires_bar(Extension(bar): Extension<Bar>) -> String {
            bar.0
        }

        // 可选 Bar 扩展的处理器
        async fn optional_bar(extension: Option<Extension<Bar>>) -> String {
            extension
                .map(|bar| bar.0.clone())
                .unwrap_or("none".to_owned())
        }

        let app = Router::new()
            .route("/requires_foo", get(requires_foo))
            .route("/optional_foo", get(optional_foo))
            .route("/requires_bar", get(requires_bar))
            .route("/optional_bar", get(optional_bar))
            .layer(Extension(Foo("foo".to_owned())));

        let client = TestClient::new(app);

        // 测试必需的 Foo
        let response = client.get("/requires_foo").await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.text().await, "foo");

        // 测试可选的 Foo
        let response = client.get("/optional_foo").await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.text().await, "foo");

        // 测试必需的 Bar（不存在）
        let response = client.get("/requires_bar").await;
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            response.text().await,
            "Missing request extension: Extension of type `axum::extension::tests::Bar` was not found. Perhaps you forgot to add it? See `axum::Extension`."
        );

        // 测试可选的 Bar
        let response = client.get("/optional_bar").await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.text().await, "none");
    }
}
