错误处理模型和工具

# axum 的错误处理模型

axum 基于 [`tower::Service`]，它通过其关联的 `Error` 类型来打包错误。如果你有一个产生错误的 [`Service`]，并且该错误一路传达到 hyper，连接将在不发送响应的情况下终止。这通常是不希望的，因此 axum 通过依赖类型系统确保你始终产生响应。

axum 通过要求所有服务将 [`Infallible`] 作为其错误类型来实现这一点。`Infallible` 是永远不会发生的错误的错误类型。

这意味着如果你定义一个像这样的处理器：

```rust
use axum::http::StatusCode;

async fn handler() -> Result<String, StatusCode> {
    # todo!()
    // ...
}
```

虽然它看起来可能会因为 `StatusCode` 而失败，但实际上这并不是"错误"。如果这个处理器返回 `Err(some_status_code)`，它仍然会被转换成 [`Response`] 并发送回客户端。这是通过 `StatusCode` 的 [`IntoResponse`] 实现完成的。

返回 `Err(StatusCode::NOT_FOUND)` 还是 `Err(StatusCode::INTERNAL_SERVER_ERROR)` 并不重要。在 axum 中这些都不被视为错误。

与其直接使用 `StatusCode`，使用一个最终可以转换为 `Response` 的中间错误类型更有意义。这允许在处理器中使用 `?` 运算符。查看这些示例：

* [`anyhow-error-response`][anyhow] 用于通用 boxed 错误
* [`error-handling`][error-handling] 用于应用特定的详细错误

[anyhow]: https://github.com/tokio-rs/axum/blob/main/examples/anyhow-error-response/src/main.rs
[error-handling]: https://github.com/tokio-rs/axum/blob/main/examples/error-handling/src/main.rs

这也适用于提取器。如果提取器与请求不匹配，请求将被拒绝并返回响应，而不会调用你的处理器。参见 [`extract`](crate::extract) 了解更多关于处理提取器失败的信息。

# 路由到可能失败的服务

如果你只使用 async 函数作为处理器，通常不需要考虑错误。但是，如果你正在嵌入通用的 `Service` 或应用可能产生错误的中间件，你必须告诉 axum 如何将这些错误转换为响应。

```rust
use axum::{
    Router,
    body::Body,
    http::{Request, Response, StatusCode},
    error_handling::HandleError,
};

async fn thing_that_might_fail() -> Result<(), anyhow::Error> {
    # Ok(())
    // ...
}

// 这个服务可能会失败并返回 `anyhow::Error`
let some_fallible_service = tower::service_fn(|_req| async {
    thing_that_might_fail().await?;
    Ok::<_, anyhow::Error>(Response::new(Body::empty()))
});

let app = Router::new().route_service(
    "/",
    // 我们不能直接路由到 `some_fallible_service`，因为它可能会失败。
    // 我们必须使用 `handle_error`，它将错误转换为响应
    // 并将其错误类型从 `anyhow::Error` 更改为 `Infallible`。
    HandleError::new(some_fallible_service, handle_anyhow_error),
);

// 通过将错误转换为实现 `IntoResponse` 的东西来处理错误
async fn handle_anyhow_error(err: anyhow::Error) -> (StatusCode, String) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        format!("Something went wrong: {err}"),
    )
}
# let _: Router = app;
```

# 应用可能失败的中间件

类似地，axum 要求你处理来自中间件的错误。这是通过 [`HandleErrorLayer`] 完成的：

```rust
use axum::{
    Router,
    BoxError,
    routing::get,
    http::StatusCode,
    error_handling::HandleErrorLayer,
};
use std::time::Duration;
use tower::ServiceBuilder;

let app = Router::new()
    .route("/", get(|| async {}))
    .layer(
        ServiceBuilder::new()
            // 如果处理器耗时太长，`timeout` 会产生错误，所以我们必须处理这些错误
            .layer(HandleErrorLayer::new(handle_timeout_error))
            .timeout(Duration::from_secs(30))
    );

async fn handle_timeout_error(err: BoxError) -> (StatusCode, String) {
    if err.is::<tower::timeout::error::Elapsed>() {
        (
            StatusCode::REQUEST_TIMEOUT,
            "Request took too long".to_string(),
        )
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Unhandled internal error: {err}"),
        )
    }
}
# let _: Router = app;
```

# 运行提取器进行错误处理

`HandleErrorLayer` 还支持运行提取器：

```rust
use axum::{
    Router,
    BoxError,
    routing::get,
    http::{StatusCode, Method, Uri},
    error_handling::HandleErrorLayer,
};
use std::time::Duration;
use tower::ServiceBuilder;

let app = Router::new()
    .route("/", get(|| async {}))
    .layer(
        ServiceBuilder::new()
            // 如果处理器耗时太长，`timeout` 会产生错误，所以我们必须处理这些错误
            .layer(HandleErrorLayer::new(handle_timeout_error))
            .timeout(Duration::from_secs(30))
    );

async fn handle_timeout_error(
    // `Method` 和 `Uri` 是提取器，所以可以在这里使用
    method: Method,
    uri: Uri,
    // 最后一个参数必须是错误本身
    err: BoxError,
) -> (StatusCode, String) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        format!("`{method} {uri}` failed with {err}"),
    )
}
# let _: Router = app;
```

[`tower::Service`]: `tower::Service`
[`Infallible`]: std::convert::Infallible
[`Response`]: crate::response::Response
[`IntoResponse`]: crate::response::IntoResponse
