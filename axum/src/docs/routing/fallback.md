向路由器添加一个 fallback [`Handler`]。

如果没有路由匹配传入的请求，将调用此服务。

```rust
use axum::{
    Router,
    routing::ver,
    handler::Handler,
    response::IntoResponse,
    http::{StatusCode, Uri},
};

let app = Router::new()
    .route("/foo", get(|| async { /* ... */ }))
    .fallback(fallback);

async fn fallback(uri: Uri) -> (StatusCode, String) {
    (StatusCode::NOT_FOUND, format!("No route for {uri}"))
}
# let _: Router = app;
```

Fallbacks 仅仅应用于路由器中没有任何东西匹配的路由。如果处理器被请求匹配但返回 404，则不会调用 fallback。请注意这也适用于 [`MethodRouter`]：如果请求命中有效路径但 [`MethodRouter`] 没有安装适当的方法处理器，则不会调用 fallback（为此目的使用 [`MethodRouter::fallback`]）。

# 处理没有其他路由的所有请求

使用 `Router::new().fallback(...)` 来接受所有请求而不管路径或方法，如果你没有其他路由，不是最优的：

```rust
use axum::Router;

async fn handler() {}

let app = Router::new().fallback(handler);

# async {
let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
axum::serve(listener, app).await;
# };
```

直接运行处理器更快，因为它避免了路由的开销：

```rust
use axum::handler::HandlerWithoutStateExt;

async fn handler() {}

# async {
let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
axum::serve(listener, handler.into_make_service()).await;
# };
```
