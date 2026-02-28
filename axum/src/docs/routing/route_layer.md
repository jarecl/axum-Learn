将 [`tower::Layer`] 应用于路由器，仅在请求匹配路由时才会运行。

请注意，中间件仅应用于现有的路由。所以你必须先添加路由（和/或 fallback），然后调用 `route_layer`。在 `route_layer` 调用后添加的其他路由将不会添加中间件。

这与 [`Router::layer`] 类似工作，除了中间件仅在请求匹配路由时运行。这对于提前返回的中间件（如授权）有用，否则可能将 `404 Not Found` 转换为 `401 Unauthorized`。

如果路由器上尚未声明任何路由，此函数将 panic，因为新层将不起作用，这通常是一个 bug。在通用代码中，你可以通过调用 [`Router::has_routes`] 首先测试是否是这种情况。

# 示例

```rust
use axum::{
    routing::get,
    Router,
};
use tower_http::validate_request::ValidateRequestHeaderLayer;

let app = Router::new()
    .route("/foo", get(|| async {}))
    .route_layer(ValidateRequestHeaderLayer::bearer("password"));

// `GET /foo` 带有有效令牌将接收 `200 OK`
// `GET /foo` 带有无效令牌将接收 `401 Unauthorized`
// `GET /not-found` 带有无效令牌将接收 `404 Not Found`
# let _: Router = app;
```
