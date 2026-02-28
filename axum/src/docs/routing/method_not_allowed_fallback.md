为路由存在但请求方法不受支持的情况添加一个 fallback [`Handler`]。

在所有先前注册的 [`MethodRouter`] 上设置一个 fallback，在没有设置匹配的方法处理器时调用。

```rust,no_run
use axum::{response::IntoResponse, routing::get, Router};

async fn hello_world() -> impl IntoResponse {
    "Hello, world!\n"
}

async fn default_fallback() -> impl IntoResponse {
    "Default fallback\n"
}

async fn handle_405() -> impl IntoResponse {
    "Method not allowed fallback"
}

#[tokio::main]
async fn main() {
    let router = Router::new()
        .route("/", get(hello_world))
        .fallback(default_fallback)
        .method_not_allowed_fallback(handle_405);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    axum::serve(listener, router).await;
}
```

Fallback 仅仅当存在给定路径的 `MethodRouter` 但请求中使用的方法未指定时应用。在示例中，`http://localhost:3000` 上的 `GET` 导致 `hello_world` 处理器响应，而发出 `POST` 触发 `handle_405`。调用完全不同的路由，如 `http://localhost:3000/hello` 导致 `default_fallback` 运行。
