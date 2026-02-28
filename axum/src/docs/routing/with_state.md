为路由器提供状态。传递给此方法的状态是全局的，将用于此路由器接收的所有请求。这意味着它不适合持有从请求派生的状态，例如在中间件中提取的授权数据。对于此类数据，使用 [`Extension`] 代替。

```rust
use axum::{Router, routing::get, extract::State};

#[derive(Clone)]
struct AppState {}

let routes = Router::new()
    .route("/", get(|State(state): State<AppState>| async {
        // 使用状态
    }))
    .with_state(AppState {});

# async {
let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
axum::serve(listener, routes).await;
# };
```

# 从函数返回带有状态的路由器

从函数返回 `Router` 时，通常不建议直接设置状态：

```rust
use axum::{Router, routing::get, extract::State};

#[derive(Clone)]
struct AppState {}

// 不要在这里调用 `Router::with_state`
fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(|_: State<AppState>| async {}))
}

// 而是在运行服务器之前进行
let routes = routes().with_state(AppState {});

# async {
let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
axum::serve(listener, routes).await;
# };
```

如果你确实需要提供状态，并且你_没有_将路由器嵌套/合并到另一个路由器中，那么返回不带任何类型参数的 `Router`：

```rust
# use axum::{Router, routing::get, extract::State};
# #[derive(Clone)]
# struct AppState {}
#
// 不要返回 `Router<AppState>`
fn routes(state: AppState) -> Router {
    Router::new()
        .route("/", get(|_: State<AppState>| async {}))
        .with_state(state)
}

let routes = routes(AppState {});

# async {
let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
axum::serve(listener, routes).await;
# };
```

这是因为我们只能在 `Router<()>` 上调用 `Router::into_make_service`，而不能在 `Router<AppState>` 上调用。参阅下面了解更多关于为什么会这样的细节。

请注意，状态默认为 `()`，所以 `Router` 和 `Router<()>` 是相同的。

如果你正在嵌套/合并路由器，建议在结果路由器上使用通用状态类型：

```rust
# use axum::{Router, routing::get, extract::State};
# #[derive(Clone)]
# struct AppState {}
#
fn routes<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/", get(|_: State<AppState>| async {}))
        .with_state(state)
}

let routes = Router::new().nest("/api", routes(AppState {});

# async {
let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
axum::serve(listener, routes).await;
# };
```

# `Router<S>` 中的 `S` 意味着什么

`Router<S>` 意味着一个路由器需要类型为 `S` 的状态才能处理请求。它_不_意昧着一个_有_类型为 `S` 的状态的 `Router`。

例如：

```rust
# use axum::{Router, routing::get, extract::State};
# #[derive(Clone)]
# struct AppState {}
#
// 一个_需要_ `AppState` 来处理请求的路由器
let router: Router<AppState> = Router::new()
    .route("/", get(|_: State<AppState>| async {}));

// 一旦我们调用 `Router::with_state` 路由器就不再需要
// 状态了，因为我们刚刚提供了它
//
// 因此，路由器类型变成 `Router<()>`，即一个
// 不缺少任何状态的路由器
let router: Router<()> = router.with_state(AppState {});

// 只有 `Router<()>` 有 `into_make_service` 方法。
//
// 你不能在 `Router<AppState>` 上调用 `into_make_service`
// 因为它仍然缺少一个 `AppState`。
# async {
let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
axum::serve(listener, router).await;
# };
```

可能有点反直觉，`Router::with_state` 并不总是返回 `Router<()>`。相反，你可以选择新的缺失状态类型是什么：

```rust
# use axum::{Router, routing::get, extract::State};
# #[derive(Clone)]
# struct AppState {}
#
let router: Router<AppState> = Router::new()
    .route("/", get(|_: State<AppState>| async {}));

// 当我们调用 `with_state` 时，我们能够选择下一个缺失状态类型是什么。
// 这里我们选择 `String`。
let string_router: Router<String> = router.with_state(AppState {});

// 这允许我们添加使用 `String` 作为状态类型的新路由
let string_router = string_router
    .route("/needs-string", get(|_: State<String>| async {}));

// 提供 `String` 并选择 `()` 作为新的缺失状态。
let final_router: Router<()> = string_router.with_state("foo".to_owned());

// 由于我们有一个 `Router<()>`，我们可以运行它。
# async {
let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
axum::serve
(listener, final_router).await;
# };
```

这就是为什么在调用 `with_state` 后返回 `Router<AppState>` 不起作用：

```rust,compile_fail
# use axum::{Router, routing::get, extract::State};
# #[derive(Clone)]
# struct AppState {}
#
// 这不起作用，因为我们返回一个 `Router<AppState>`
// 即我们说我们仍然缺少一个 `AppState`
fn routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(|_: State<AppState>| async {}))
        .with_state(state)
}

let app = routes(AppState {});

// 我们只能在 `Router<()>` 上调用 `Router::into_make_service`
// 但 `app` 是一个 `Router<AppState>`
# async {
let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
axum::serve(listener, app).await;
# };
```

而是返回 `Router<()>`，因为我们提供了所有必需的状态：

```rust
# use axum::{Router, routing::get, extract::State};
# #[derive(Clone)]
# struct AppState {}
#
// 我们已经提供了所有必需的状态，所以返回 `Router<()>`
fn routes(state: AppState) -> Router<()> {
    Router::new()
        .route("/", get(|_: State<AppState>| async {}))
        .with_state(state)
}

let app = routes(AppState {});

// 我们现在可以调用 `Router::into_make_service`
# async {
let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
axum::serve(listener, app).await;
# };
```

# 关于性能的说明

如果你需要一个实现 `Service` 的 `Router` 但不需要任何状态（也许你正在创建内部使用 axum 的库），那么建议在开始服务请求之前调用此方法：

```rust
use axum::{Router, routing::get};

let app = Router::new()
    .route("/", get(|| async { /* ... */ }))
    // 即使我们不需要任何状态，无论如何都调用 `with_state(())`
    .with_state(());
# let _: Router = app;
```

这不是必需的，但它给 axum 了一个机会来更新路由器中的一些内部内容，这可能会影响性能并减少分配。

请注意 [`Router::into_make_service`] 和 [`Router::into_make_service_with_connect_info`] 会自动执行此操作。

[`Extension`]: crate::Extension
