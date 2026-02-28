将 [`tower::Layer`] 应用于路由器，仅在请求匹配路由时才会运行。

请注意，中间件仅应用于现有的路由。首先添加路由，然后调用 `route_layer`。在 `route_layer` 调用后添加的其他路由将不会添加中间件。

这与 [`MethodRouter::layer`] 类似工作，除了中间件仅在请求匹配路由时运行。这对于提前返回的中间件（如授权）有用，否则可能将 `405 Method Not Allowed` 转换为 `401 Unauthorized`。

# 示例

```rust
use axum::{
    routing::get,
    Router,
};
use tower_http::validate_request::ValidateRequestHeaderLayer;

let app = Router::new().route(
    "/foo",
    get(|| async {})
        .route_layer(ValidateRequestHeaderLayer::bearer("password"))
);

// `GET /foo` 带有有效令牌将接收 `200 OK`
// `GET /foo` 带有无效令牌将接收 `401 Unauthorized`
// `POST /FOO` 带有无效令牌将接收 `405 Method Not Allowed`
# let _: Router = app;
```
