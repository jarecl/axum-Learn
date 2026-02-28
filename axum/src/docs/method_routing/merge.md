合并两个路由器为一个。

这有助于将路由器分解为更小的部分并将它们组合成一个。

```rust
use axum::{
    routing::{get, post},
    Router,
};

let get = get(|| async {});
let post = post(|| async {});

let merged = get.merge.merge);

let app = Router::new().route("/", merged);

// 我们的应用现在接受
// - GET /
// - POST /
# let _: Router = app;
```
