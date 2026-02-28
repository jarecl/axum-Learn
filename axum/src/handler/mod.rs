//! 处理器模块 - 用于处理请求的异步函数
//!
//! 在 Spring Boot 中，这相当于：
//! - HandlerFunction<T, R> - 函数式处理器的函数式接口
//! - @Controller 中的方法 - 处理 HTTP 请求的方法
//! - RouterFunction.route() - 路由到处理器的配置
//!
//! axum 的处理器是实现了 `Handler` 特质的异步函数。
//! 它们可以接受各种提取器作为参数，并返回实现了 `IntoResponse` 的类型。
//!
//! ## 处理器示例
//!
//! ```rust
//! use axum::{body::Bytes, http::StatusCode};
//!
//! // 立即返回空的 `200 OK` 响应的处理器
//! // 类似 Spring Boot: @GetMapping public void handler() {}
//! async fn unit_handler() {}
//!
//! // 立即返回带有纯文本体的 `200 OK` 响应的处理器
//! // 类似 Spring Boot: @GetMapping public String handler() { return "Hello, World!"; }
//! async fn string_handler() -> String {
//!     "Hello, World!".to_string()
//! }
//!
//! // 缓冲请求体并返回它的处理器
//! //
//! // 这工作是因为 `Bytes` 实现了 `FromRequest`
//! // 因此可以用作提取器
//! //
//! // `String` 和 `StatusCode` 都实现了 `IntoResponse`
//! // 因此 `Result<String, StatusCode>` 也实现了 `IntoResponse`
//! // 类似 Spring Boot: @PostMapping public ResponseEntity<String> echo(@RequestBody byte[] body) { ... }
//! async fn echo(body: Bytes) -> Result<String, StatusCode> {
//!     if let Ok(string) = String::from_utf8(body.to_vec()) {
//!         Ok(string)
//!     } else {
//!         Err(StatusCode::BAD_REQUEST)
//!     }
//! }
//! ```

#![doc = include_str!("../docs/handlers_intro.md")]
//!
//! 一些处理器示例：
//!
//! ```rust
//! use axum::{body::Bytes, http::StatusCode};
//!
//! // 立即返回空的 `200 OK` 响应的处理器
//! async fn unit_handler() {}
//!
//! // 立即返回带有纯文本体的 `200 OK` 响应的处理器
//! async fn string_handler() -> String {
//!     "Hello, World!".to_string()
//! }
//!
//! // 缓冲请求体并返回它的处理器
//! //
//! // 这工作是因为 `Bytes` 实现了 `FromRequest`
//! // 因此可以用作提取器
//! //
//! // `String` 和 `StatusCode` 都实现了 `IntoResponse`
//! // 因此 `Result<String, StatusCode>` 也实现了 `IntoResponse`
//! async fn echo(body: Bytes) -> Result<String, StatusCode> {
//!     if let Ok(string) = String::from_utf8(body.to_vec()) {
//!         Ok(string)
//!     } else {
//!         Err(StatusCode::BAD_REQUEST)
//!     }
//! }
//! ```
//!
//! 代替直接的 `StatusCode`，使用中间错误类型是有意义的，
//! 这些类型最终可以转换为 `Response`。这允许在处理器中使用 `?` 运算符。
//! 参见这些示例：
//!
//! * [`anyhow-error-response`][anyhow] 用于通用盒装错误
//! * [`error-handling`][error-handling] 用于应用特定详细错误
//!
//! [anyhow]: https://github.com/tokio-rs/axum/blob/main/examples/anyhow-error-response/src/main.rs
//! [error-handling]: https://github.com/tokio-rs/axum/blob/main/examples/error-handling/src/main.rs
//!
#![doc = include_str!("../docs/debugging_handler_type_errors.md")]

#[cfg(feature = "tokio")]
use crate::extract::connect_info::IntoMakeServiceWithConnectInfo;
use crate::{
    extract::{FromRequest, FromRequestParts, Request},
    response::{IntoResponse, Response},
    routing::IntoMakeService,
};
use std::{convert::Infallible, fmt, future::Future, marker::PhantomData, pin::Pin};
use tower::ServiceExt;
use tower_layer::Layer;
use tower_service::Service;

pub mod future;      /// 处理器 Future 类型
mod service;         /// 处理器 Service 实现

pub use self::service::HandlerService; /// 从 Handler 创建的 Service

/// 可用于处理请求的异步函数特质
///
/// 你不应该直接依赖这个特质。它会自动实现为正确类型的闭包。
///
/// 更多详情请参阅 [模块文档](crate::handler)。
///
/// ## 将 `Handler` 转换为 [`Service`]s
///
/// 要将 `Handler`s 转换为 [`Service`]s，你必须调用
/// [`HandlerWithoutStateExt::into_service`] 或 [`Handler::with_state`]：
///
/// ```
/// use tower::Service;
/// use axum::{
///     extract::{State, Request},
///     body::Body,
///     handler::{HandlerWithoutStateExt, Handler},
/// };
///
/// // 这个处理器不需要任何状态
/// async fn one() {}
/// // 所以可以用 `HandlerWithoutStateExt::into_service` 转换为 service
/// assert_service(one.into_service());
///
/// // 这个处理器需要状态
/// async fn two(_: State<String>) {}
//! // 所以我们必须提供它
//! let handler_with_state = two.with_state(String::new());
//! // 这给我们一个 `Service`
//! assert_service(handler_with_state);
//!
//! // 辅助函数：检查值是否实现了 `Service`
//! fn assert_service<S>(service: S)
//! where
//!     S: Service<Request>,
//! {}
//! ```
#[doc = include_str!("../docs/debugging_handler_type_errors.md")]
///
/// ## 不是函数的处理器
///
/// `Handler` 特质也为 `T: IntoResponse` 实现。
/// 这允许为路由轻松返回固定数据：
///
/// ```
/// use axum::{
///     Router,
///     routing::{get, post},
///     Json,
///     http::StatusCode,
/// };
/// use serde_json::json;
//!
//! let app = Router::new()
///     // 用固定字符串响应
///     .route("/", get("Hello, World!"))
///     // 或返回一些模拟数据
///     .route("/users", post((
///         StatusCode::CREATED,
///         Json(json!({ "id": 1, "username": "alice" })),
///     )));
//! # let _: Router = app;
//! ```
///
/// ## 关于类型参数 `T`
///
/// **通常你不需要担心 `T`**；当当调用诸如
//! [`post`](crate::routing::method_routing::post)等方法时，它将自动推断，
/// 这是在应用代码中提供此参数的预期方式。
///
/// 如果你正在实现自己的方法，接受 `Handler` 的实现作为参数，
//! 那么以下可能有用：
///
/// 类型参数 `T` 是特质一致性规则的变通方法，允许我们
/// 为具有不同数量参数的许多类型处理器函数编写 `Handler` 的通用实现，
/// 而编译器不会因为一个类型 `F` 理论上可以实现 `Fn(A) -> X` 和 `Fn(A, B) -> Y` 而禁止我们这样做。
/// `T` 是一个占位符，接受处理器函数参数的表示，以及其他类似的"一致性规则变通方法"区分符，
/// 允许我们选择一个函数签名来用作 `Handler`。
#[diagnostic::on_unimplemented(
    note = "Consider using `#[axum::debug_handler]` to improve error message"
)]
pub trait Handler<T, S>: Clone + Send + Sync + Sized + 'static {
    /// 调用此处理器返回的 Future 类型
    type Future: Future<Output = Response> + Send + 'static;

    /// 使用给定的请求调用处理器
    fn call(self, req: Request, state: S) -> Self::Future;

    /// 对处理器应用 [`tower::Layer`]
    ///
    /// 对处理器的所有请求将由层的相应中间件处理。
    ///
    /// 这可用于为单个处理器添加额外的请求处理。
    ///
    /// 注意这不同于 [`routing::Router::layer`](crate::routing::Router::layer)
    /// 后者为一组路由添加中间件。
    ///
    /// 如果你正在应用会产生错误的中间件，你必须处理
    /// 以便它们被转换为响应。你可以[在此处](crate::error_handling)了解更多关于这样做。
    ///
    /// ## 示例
    ///
    /// 将 [`tower::limit::ConcurrencyLimit`] 中间件添加到处理器
    /// 可以这样做：
    ///
    /// ```rust
    /// use axum::{
    ///     routing::get,
    ///     handler::Handler,
    ///     Router,
    /// };
    /// use tower::limit::{ConcurrencyLimitLayer, ConcurrencyLimit};
    ///
    /// async fn handler() { /* ... */ }
    ///
    /// let layered_handler = handler.layer(ConcurrencyLimitLayer::new(64));
    /// let app = Router::new().route("/", get(layered_handler));
    /// # let _: Router = app;
    /// ```
    fn layer<L>(self, layer: L) -> Layered<L, Self, T, S>
    where
        L: Layer<HandlerService<Self, T, S>> + Clone,
        L::Service: Service<Request>,
    {
        Layered {
            layer,
            handler: self,
            _marker: PhantomData,
        }
    }

    /// 通过提供状态将处理器转换为 [`Service`]
    fn with_state(self, state: S) -> HandlerService<Self, T, S> {
        HandlerService::new(self, state)
    }
}

// 为不接受任何参数的函数实现 Handler
#[diagnostic::do_not_recommend]
impl<F, Fut, Res, S> Handler<(), S> for F
where
    F: FnOnce() -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send,
    Res: IntoResponse,
{
    type Future = Pin<Box<dyn Future<Output = Response> + Send>>;

    fn call(self, _req: Request, _state: S) -> Self::Future {
        Box::pin(async move { self().await.into_response() })
    }
}

// 宏：为具有特定数量参数的函数实现 Handler
// 这个宏为从 0 到 16 个参数生成实现
macro_rules! impl_handler {
    (
        [$($ty:ident),*], $last:ident
    ) => {
        #[diagnostic::do_not_recommend]
        #[allow(non_snake_case, unused_mut)]
        impl<F, Fut, S, Res, M, $($ty,)* $last> Handler<(M, $($ty,)* $last,), S> for F
        where
            F: FnOnce($($ty,)* $last,) -> Fut + Clone + Send + Sync + 'static,
            Fut: Future<Output = Res> + Send,
            S: Send + Sync + 'static,
            Res: IntoResponse,
            $( $ty: FromRequestParts<S> + Send, )*
            $last: FromRequest<S, M> + Send,
        {
            type Future = Pin<Box<dyn Future<Output = Response> + Send>>;

            fn call(self, req: Request, state: S) -> Self::Future {
                let (mut parts, body) = req.into_parts();
                Box::pin(async move {
                    // 首先提取 FromRequestParts 参数（不消费 body）
                    $(
                        let $ty = match $ty::from_request_parts(&mut parts, &state).await {
                            Ok(value) => value,
                            Err(rejection) => return rejection.into_response(),
                        };
                    )*

                    // 重新组装请求用于 FromRequest 参数（消费 body）
                    let req = Request::from_parts(parts, body);

                    // 提取 FromRequest 参数（最后一个是消费 body 的）
                    let $last = match $last::from_request(req, &state).await {
                        Ok(value) => value,
                        Err(rejection) => return rejection.into_response(),
                    };

                    // 调用处理器函数并将结果转换为响应
                    self($($ty,)* $last,).await.into_response()
                })
            }
        }
    };
}

// 为所有参数数量生成实现
all_the_tuples!(impl_handler);

// 私有模块：用于区分器
mod private {
    // `impl<T: IntoResponse> Handler for T` 的标记类型
    #[allow(missing_debug_implementations)]
    pub enum IntoResponseHandler {}
}

// 为实现 IntoResponse 的类型实现 Handler
// 这允许直接返回值而不是函数
#[diagnostic::do_not_recommend]
impl<T, S> Handler<private::IntoResponseHandler, S> for T
where
    T: IntoResponse + Clone + Send + Sync + 'static,
{
    type Future = std::future::Ready<Response>;

    fn call(self) -> Self::Future {
        std::future::ready(self.into_response())
    }
}

/// 通过应用 Tower 中间件从 [`Handler`] 创建的 [`Service`]
///
/// 使用 [`Handler::layer`] 创建。更多详情请参阅该方法。
pub struct Layered<L, H, T, S> {
    layer: L,
    handler: H,
    _marker: PhantomData<fn() -> (T, S)>,
}

// Layered 的 Debug 实现
impl<L, H, T, S> fmt::Debug for Layered<L, H, T, S>
where
    L: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Layered")
            .field("layer",) &self.layer)
            .finish()
    }
}

// Layered 的 Clone 实现
impl<L, H, T, S> Clone for Layered<L, H, T, S>
where
    L: Clone,
    H: Clone,
{
    fn clone(&self) -> Self {
        Self {
            layer: self.layer.clone(),
            handler: self.handler.clone(),
            _marker: PhantomData,
        }
    }
}

// 为 Layered 实现 Handler
#[diagnostic::do_not_recommend]
impl<H, S, T, L> Handler<T, S> for Layered<L, H, T, S>
where
    L: Layer<HandlerService<H, T, S>> + Clone + Send + Sync + 'static,
    H: Handler<T, S>,
    L::Service: Service<Request, Error = Infallible> + Clone + Send + 'static,
    <L::Service as Service<Request>>::Response: IntoResponse,
    <L::Service as Service<Request>>::Future: Send,
    T: 'static,
    S: 'static,
{
    type Future = future::LayeredFuture<L::Service>;

    fn call(self, req: Request, state: S) -> Self::Future {
        use futures_util::future::{FutureExt, Map};

        // 创建带状态的处理器服务
        let svc = self.handler.with_state(state);
        // 应用层（中间件）
        let svc = self.layer.layer(svc);

        // 创建映射 future，处理中间件的结果
        let future: Map<
            _,
            fn(
                Result<
                    <L::Service as Service<Request>>::Response,
                    <L::Service as Service<Request>>::Error,
                >,
            ) -> _,
        > = svc.oneshot(req).map(|result| match result {
            Ok(res) => res.into_response(),
            Err(err) => match err {},
        });

        future::LayeredFuture::new(future)
    }
}

/// 没有状态的 [`Handler`] 的扩展特质
///
/// 这提供了便捷方法将 [`Handler`] 转换为 [`Service`] 或 [`MakeService`]。
///
/// 在 Spring Boot 中，这类似于创建一个 HandlerFunction 并立即应用它
///
/// [`MakeService`]: tower::make::MakeService
pub trait HandlerWithoutStateExt<T>: Handler<T, ()> {
    /// 将处理器转换为没有状态的 [`Service`]
    fn into_service(self) -> HandlerService<Self, T, ()>;

    /// 将处理器转换为没有状态的 [`MakeService`]
    ///
    /// 更多详情请参阅 [`HandlerService::into_make_service`]。
    ///
    /// [`MakeService`]: tower::make::MakeService
    fn into_make_service(self) -> IntoMakeService<HandlerService<Self, T, ()>>;

    /// 将处理器转换为 [`MakeService`]，它存储关于传入连接的信息并且没有状态
    ///
    /// 更多详情请参阅 [`HandlerService::into_make_service_with_connect_info`]。
    ///
    /// [`MakeService`]: tower::make::MakeService
    #[cfg(feature = "tokio")]
    fn into_make_service_with_connect_info<C>(
        self,
    ) -> IntoMakeServiceWithConnectInfo<HandlerService<Self, T, ()>, C>;
}

// 为所有满足条件的 H, T 实现 HandlerWithoutStateExt
impl<H, T> HandlerWithoutStateExt<T> for H
where
    H: Handler<T, ()>,
{
    fn into_service(self) -> HandlerService<Self, T, ()> {
        self.with_state(())
    }

    fn into_make_service(self) -> IntoMakeService<HandlerService<Self, T, ()>> {
        self.into_service().into_make_service()
    }

    #[cfg(feature = "tokioio")]
    fn into_make_service_with_connect_info<C>(
        self,
    ) -> IntoMakeServiceWithConnectInfo<HandlerService<Self, T, ()>, C> {
        self.into_service().into_make_service_with_connect_info()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{extract::State, test_helpers::*};
    use axum_core::body::Body;
    use http::StatusCode;
    usever std::time::Duration;
    use tower_http::{
        limit::RequestBodyLimitLayer, map_request_body::MapRequestBodyLayer,
        map_response_body::MapResponseBodyLayer, timeout::TimeoutLayer,
    };

    #[crate::test]
    async fn handler_into_service() {
        async fn handle(body: String) -> impl IntoResponse {
            format!("you said: {body}")
        }

        let client = TestClient::new(handle.into_service());

        let res = client.post("/").body("hi there!").await;
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.text().await, "you said: hi there!");
    }

    #[crate::test]
    async fn with_layer_that_changes_request_body_and_state() {
        async fn handle(State(state): State<&'static str>) -> &'static str {
            state
        }

        let svc = handle
            .layer((
                RequestBodyLimitLayer::new(1024),
                TimeoutLayer::with_status_code(
                    StatusCode::REQUEST_TIMEOUT,
                    Duration::from_secs(10),
                ),
                MapResponseBodyLayer::new(Body::new),
            ))
            .layer(MapRequestBodyLayer::new(Body::new))
            .with_state("foo");

        let client = TestClient::new(svc);
        let res = client.get("/").await;
        assert_eq!(res.text().await, "foo");
    }
}
