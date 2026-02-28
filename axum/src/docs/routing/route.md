向路由器添加另一个路由。

`path` 是一个由 `/` 分隔的路径段字符串。每个段可以是静态的、捕获或通配符。

`method_router` 是应该在路径匹配 `path` 时接收请求的 [`MethodRouter`]。通常，`method_router` 将是一个包装在方法路由器（如 [`get`]）中的处理器。有关处理器的更多细节，参阅 [`handler`](crate::handler)。

# 静态路径

示例：

- `/`
- `/foo`
- `/users/123`

如果传入请求与路径完全匹配，将调用相应的服务。

# 捕获

路径可以包含像 `/{key}` 这样的段，它匹配任何单个段并将捕获的值存储在 `key` 处。捕获的值可以是零长度的，除了在无效路径 `//` 中。

示例：

- `/{key}`
- `/users/{id}`
- `/users/{id}/tweets`

可以使用 [`Path`](crate::extract::Path) 提取捕获。查看其文档了解更多细节。

无法创建仅匹配某些类型（如数字）或正则表达式的段。你必须在处理器中手动处理。

[`MatchedPath`] 可用于提取匹配的路径而不是实际路径。

# 通配符

路径可以以 `/{*key}` 结尾，它匹配所有段并将捕获的段存储在 `key` 处。

示例：

- `/{*key}`
- `/assets/{*path}`
- `/{id}/{repo}/{*tree}`

请注意 `/{*key}` 不匹配空段。因此：

- `/{*key}` 不匹配 `/` 但匹配 `/a`、`/a/` 等。
- `/x/{*key}` 不匹配 `/x` 或 `/x/` 但匹配 `/x/a`、`/x/a/` 等。

也可以使用 [`Path`](crate::extract::Path) 提取通配符捕获：

```rust
use axum::{
    Router,
    routing::get,
    extract::Path,
};

let app: Router = Router::new().route("/{*key}", get(handler));

async fn handler(Path(path): Path<String>) -> String {
    path
}
```

请注意，不包括前导斜杠，即对于路由 `/foo/{*rest}` 和路径 `/foo/bar/baz`，`rest` 的值将是 `bar/baz`。

# 接受多个方法

要为相同路由接受多个方法，你可以同时添加所有处理器：

```rust
use axum::{Router, routing::{get, delete}, extract::Path};

let app = Router::new().route(
    "/",
    get(get_root).post(post_root).delete(delete_root),
);

async fn get_root() {}

async fn post_root() {}

async fn delete_root() {}
# let _: Router = app;
```

或者你可以一个一个添加：

```rust
# use axum::Router;
# use axum::routing::{get, post, delete};
#
let app = Router::new()
    .route("/", get(get_root))
    .route("/", post(post_root))
    .route("/", delete(delete_root));
#
# let _: Router = app;
# async fn get_root() {}
# async fn post_root() {}
# async fn delete_root() {}
```

# 更多示例

```rust
use axum::{Router, routing::{get, delete}, extract::Path};

let app = Router::new()
    .route("/", get(root))
    .route("/users", get(list_users).post(create_user))
    .route("/users/{id}", get(show_user))
    .route("/api/{version}/users/{id}/action", delete(do_users_action))
    .route("/assets/{*path}", get(serve_asset));

async fn root() {}

async fn list_users() {}

async fn create_user() {}

async fn show_user(Path(id): Path<u64>) {}

async fn do_users_action(Path((version, id)): Path<(String, u64>>) {}

async fn serve_asset(Path(path): Path<String>) {}
# let _: Router = app;
```

# Panic

如果路由与另一个路由重叠，则会 panic：

```rust,should_panic
use axum::{routing::get, Router};

let app = Router::new()
    .route("/", get(|| async {}))
    .route("/", get(|| async {}));
# let _: Router = app;
```

静态路由 `/foo` 和动态路由 `/{key}` 不被视为重叠，`/foo` 将具有优先权。

如果 `path` 为空也会 panic。
