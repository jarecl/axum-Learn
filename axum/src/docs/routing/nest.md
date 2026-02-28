将 [`Router`] 嵌套到某个路径。

这允许你将应用程序分解为更小的部分并将它们组合在一起。

# 示例

```rust
use axum::{
    routing::{get, post},
    Router,
};

let user_routes = Router::new().route("/{id}", get(|| async {}));

let team_routes = Router::new().route("/", post(|| async {}));

let api_routes = Router::new()
    .nest("/users", user_routes)
    .nest("/teams", team_routes);

let app = Router::new().nest("/api", api_routes);

// 我们的应用现在接受
// - GET /api/users/{id}
// - POST /api/teams
# let _: Router = app;
```

# URI 如何变化

请注意，嵌套路由将看不到原始请求 URI，而是有匹配的前缀被剥离。这对于像静态文件服务这样的服务工作是必要的。如果你需要原始请求 URI，使用 [`OriginalUri`]。

# 来自外部路由的捕获

将 `nest` 与动态路由一起使用时要小心，因为嵌套也会从外部路由捕获：

```rust
use axum::{
    extract::Path,
    routing::get,
    Router,
};
use std::collections::HashMap;

async fn users_get(Path(params): Path<HashMap<String, String>>) {
    // `version` 和 `id` 都被捕获了，虽然 `users_api` 只
    // 显式捕获 `id`。
    let version = params.get("version");
    let id = params.get("id");
}

let users_api = Router::new().route("/users/{id}", get(users_get));

let app = Router::new().nest("/{version}/api", users_api);
# let _: Router = app;
```

# 与通配符路由的差异

嵌套路由与通配符路由类似。区别在于通配符路由仍然看到整个 URI，而嵌套路由将有前缀被剥离：

```rust
use axum::{routing::get, http::Uri, Router};

let nested_router = Router::new()
    .route("/", get(|uri: Uri| async {
        // `uri` 将_不_包含 `/bar`
    }));

let app = Router::new()
    .route("/foo/{*rest}", get(|uri: Uri| async {
        // `uri` 将包含 `/foo`
    }))
    .nest("/bar", nested_router);
# let _: Router = app;
```

此外，虽然通配符路由 `/foo/*rest` 不会匹配路径 `/foo` 或 `/foo/`，嵌套在 `/foo` 的路由器将匹配路径 `/foo`（但不匹配 `/foo/`），嵌套在 `/foo/` 的路由器将匹配路径 `/foo/`（但不匹配 `/foo`）。

# Fallback

如果嵌套路由没有自己的 fallback，它将从外部路由器继承 fallback：

```rust
use axum::{routing::get, http::StatusCode, handler::Handler, Router};

async fn fallback() -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "Not Found")
}

let api_routes = Router::new().route("/users", get(|| async {}));

let app = Router::new()
    .nest("/api", api_routes)
    .fallback(fallback);
# let _: Router = app;
```

这里像 `GET /api/not-found` 这样的请求将进入 `api_routes`，但因为它们没有匹配路由且没有自己的 fallback，它将调用外部路由器的 fallback，即 `fallback` 函数。

如果嵌套路由有自己的 fallback，则外部 fallback 将不会被继承：

```rust
use axum::{
    routing::get,
    http::StatusCode,
    handler::Handler,
    Json,
    Router,
};

async fn fallback() -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "Not Found")
}

async fn api_fallback() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({ "status": "Not Found" })),
    )
}

let api_routes = Router::new()
    .route("/users", get(|| async {}))
    .fallback(api_fallback);

let app = Router::new()
    .nest("/api", api_routes)
    .fallback(fallback);
# let _: Router = app;
```

这里像 `GET /api/not-found` 这样的请求将转到 `api_fallback`。

# 嵌套带有状态的路由器

使用此方法组合 [`Router`] 时，每个 [`Router`] 必须具有相同类型的状体。如果你的路由器具有不同的类型，可以使用 [`Router::with_state`] 提供状态并使类型匹配：

```rust
use axum::{
    Router,
    routing::get,
    extract::State,
};

#[derive(Clone)]
struct InnerState {}

#[derive(Clone)]
struct OuterState {}

async fn inner_handler(state: State<InnerState>) {}

let inner_router = Router::new()
    .route("/bar", get(inner_handler))
    .with_state(InnerState {});

async fn outer_handler(state: State<OuterState>) {}

let app = Router::new()
    .route("/", get(outer_handler))
    .nest("/foo", inner_router)
    .with_state(OuterState {});
# let _: axum::Router = app;
```

请注意，内部路由器仍将从外部路由器继承 fallback。

# Panic

- 如果路由与另一个路由重叠。参阅 [`Router::route`] 了解更多细节。
- 如果路由包含通配符（`*`）。
- 如果 `path` 为空。

[`OriginalUri`]: crate::extract::OriginalUri
[fallbacks]: Router::fallback
