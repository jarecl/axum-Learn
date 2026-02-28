关闭与 0.7 路由匹配语法兼容性的检查。

这允许使用以冒号 `:` 或星号 `*` 开头的路径，否则这些是被禁止的。

# 示例

```rust
use axum::{
    routing::get,
    Router,
};

let app = Router::<()>::new()
    .without_v07_checks()
    .route("/:colon", get(|| async {}))
    .route("/*asterisk", get(|| async {}));

// 我们的应用现在接受

// - GET /:colon
// - GET /*asterisk
# let _: Router = app;
```

首先调用此方法而不添加这样的路由会导致 panic。

```rust,should_panic
use axum::{
    routing::get,
    Router,
};

// 这会 panic...
let app = Router::<()>::new()
    .route("/:colon", get(|| async {}));
```

# 合并

当两个路由器合并时，如果两个路由器也都禁用了 v0.7 检查，则在结果路由器上禁用路由注册的 v0.7 检查。

# 嵌套

每个路由器都需要显式禁用检查。嵌套一个启用了或禁用了检查的路由器对外部路由器没有影响。
