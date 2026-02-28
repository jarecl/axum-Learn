向路由器添加一个 fallback 服务。

如果没有路由匹配传入的请求，将调用此服务。

```rust
use axum::{
    Router
    routing::get,
    handler::Handler,
    response::IntoResponse,
    http::{StatusCode, Method, Uri},
};

let handler = get(|| async {}).fallback(fallback);

let app = Router::new().route("/", handler);

async fn fallback(method: Method, uri: Uri) -> (StatusCode, String) {
    (StatusCode::NOT_FOUND, format!("`{method}` not allowed for {uri}"))
}
# let _: Router = app;
```

## 与 `MethodRouter::merge` 一起使用时

两个都有 fallback 的路由器不能合并。这样做会导致 panic：

```rust,should_panic
use axum::{
    routing::{get, post},
    handler::Handler,
    response::IntoResponse,
    http::{StatusCode, Uri},
};

let one = get(|| async {}).fallback(fallback_one);

let two = post(|| async {}).fallback(fallback_two);

let method_route = one.merge(two);

async fn fallback_one() -> impl IntoResponse { /* ... */ }
async fn fallback_two() -> impl IntoResponse { /* ... */ }
# let app: axum::Router = axum::Router::new().route("/", method_route);
```

## 设置 `Allow` 头部

默认情况下，`MethodRouter` 在返回 `405 Method Not Allowed` 时设置 `Allow` 头部。当 fallback 返回 `405 Method Not Allowed` 时也会这样做，除非 fallback 生成的响应已经设置了 `Allow` 头部。

这意味着如果你使用 `fallback` 来接受额外的方法，你应该确保正确设置 `Allow` 头部。
