向路由器添加另一个路由，该路由调用 [`Service`]。

# 示例

```rust,no_run
use axum::{
    Router,
    body::Body,
    routing::{any_service, get_service},
    extract::Request,
    http::StatusCode,
    error_handling::HandleErrorLayer,
};
use tower_http::services::ServeFile;
use http::Response;
use std::{convert::Infallible, io};
use tower::service_fn;

let app = Router::new()
    .route(
        // 对 `/` 的任何请求都转到服务
        "/",
        // 响应主体不是 `axum::body::BoxBody` 的服务
        // 可以包装在 `axum::routing::any_service` 中（或其他路由过滤器之一）
        // 以使响应主体映射
        any_service(service_fn(|_: Request| async {
            let res = Response::new(Body::from("Hi from `GET /`"));
            Ok::<_, Infallible>(res)
        }))
    )
    .route_service(
        "/foo",
        // 此服务的响应主体是 `axum::body::BoxBody`，所以
        // 可以直接路由到它。
        service_fn(|req: Request| async move {
            let body = Body::from(format!("Hi from `{} /foo`", req.method()));
            let res = Response::new(body);
            Ok::<_, Infallible>(res)
        })
    )
    .route_service(
        // GET `/static/Cargo.toml` 转到 tower-http 的服务
        "/static/Cargo.toml",
        ServeFile::new("Cargo.toml"),
    );
# let _: Router = app;
```

以这种方式路由到任意服务对于背压（[`Service::poll_ready`]）有复杂性。参阅 [路由到服务和中间件与背压] 模块了解更多细节。

# Panic

因与 [`Router::route`] 相同的原因 panic，或如果你尝试路由到 `Router`：

```rust,should_panic
use axum::{routing::get, Router};

let app = Router::new().route_service(
    "/",
    Router::new().route("/foo", get(|| async {})),
);
# let _: Router = app;
```

使用 [`Router::nest`] 代替。

[路由到服务和中间件与背压]: middleware/index.html#routing-to-servicesmiddleware-and-backpressure
