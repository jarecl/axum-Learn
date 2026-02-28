将 [`tower::Layer`] 应用于路由器中的所有路由。

这可以用于为一组路由添加对请求的额外处理。

请注意，中间件仅应用于现有的路由。所以你必须先添加路由（和/或 fallback），然后调用 `layer`。在 `layer` 调用后添加的其他路由将不会添加中间件。

如果你想将中间件添加到单个处理器，可以使用 [`MethodRouter::layer`] 或 [`Handler::layer`]。

# 示例

添加 [`tower_http::trace::TraceLayer`]：

```rust
use axum::{routing::get, Router};
use tower_http::trace::TraceLayer;

let app = Router::new()
    .route("/foo", get(|| async {}))
    .route("/bar", get(|| async {}))
    .layer(TraceLayer::new_for_http());
# let _: Router = app;
```

如果你需要编写自己的中间件，参阅 ["编写中间件"](crate::middleware#writing-middleware) 了解不同的选项。

如果你只想在某些路由上使用中间件，可以使用 [`Router::merge`]：

```rust
use axum::{routing::get, Router};
use tower_http::{trace::TraceLayer, compression::CompressionLayer};

let with_tracing = Router::new()
    .route("/foo", get(|| async {}))
    .layer(TraceLayer::new_for_http());

let with_compression = Router::new()
    .route("/bar", get(|| async {}))
    .layer(CompressionLayer::new());

// 将所有内容合并到一个 `Router`
let app = Router::new()
    .merge(with_tracing)
    .merge(with_compression);
# let _: Router = app;
```

# 多个中间件

应用多个中间件时推荐使用 [`tower::ServiceBuilder`]。参阅 [`middleware`](crate::middleware) 了解更多细节。

# 在路由后运行

使用此方法添加的中间件将在路由_之后_运行，因此不能用于重写请求 URI。参阅 ["在中间件中重写请求 URI"](crate::middleware#rewriting-request-uri-in-middleware) 了解更多细节和解决方法。

# 错误处理

参阅 [`middleware`](crate::middleware) 了解错误处理如何影响中间件的细节。
