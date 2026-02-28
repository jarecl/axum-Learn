将两个路由器的路径和 fallback 合并为单个 [`Router`]。

这有助于将应用程序分解为更小的部分并将它们组合成一个。

```rust
use axum::{
    routing::get,
    Router,
};
#
# async fn users_list() {}
# async fn users_show() {}
# async fn teams_list() {}

// 分别定义一些路由
let user_routes = Router::new()
    .route("/users", get(users_list))
    .route("/users/{id}", get(users_show));

let team_routes = Router::new()
    .route("/teams", get(teams_list));

// 将它们合并为一个
let app = Router::new()
    .merge(user_routes)
    .merge(team_routes);

// 也可以做 `user_routes.merge(team_routes)`

// 我们的应用现在接受
// - GET /users
// - GET /users/{id}
// - GET /teams
# let _: Router = app;
```

# 合并带有状态的路由器

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

async fn inner_handler(state:: State<InnerState>) {}

let inner_router = Router::new()
    .route("/bar", get(inner_handler))
    .with_state(InnerState {});

async fn outer_handler(state: State<OuterState>) {}

let app = Router::new()
    .route("/", get(outer_handler))
    .merge(inner_router)
    .with_state(OuterState {});
# let _: axum::Router = app;
```

# 合并带有 fallback 的路由器

使用此方法组合 [`Router`] 时，[fallback](Router::fallback) 也会被合并。然而只有其中一个路由器可以有 fallback。

# Panic

- 如果合并两个各自有 [fallback](Router::fallback) 的路由器。这是因为 `Router` 只允许单个 fallback。
