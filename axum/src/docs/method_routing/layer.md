将 [`tower::Layer`] 应用于路由器中的所有路由。

这可以用于为一组路由添加对请求的额外处理。

请注意，中间件仅应用于现有的路由。所以你必须先添加路由（和/或 fallback），然后调用 `layer`。在 `layer` 调用后添加的其他路由将不会添加中间件。

与 [`Router::layer`](super::Router::layer) 类似工作。查看该方法了解更多细节。

# 示例

```rust
use axum::{routing::get, Router};
use tower::limit::ConcurrencyLimitLayer;

async fn handler() {}

let app = Router::new().route(
    "/",
    // 对 `GET /` 的所有请求都将通过 `ConcurrencyLimitLayer`
    get(handler).layer(ConcurrencyLimitLayer::new(64)),
);
# let _: Router = app;
```
