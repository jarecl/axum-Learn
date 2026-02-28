将此路由器转换为 [`MakeService`]，它将 `C` 的关联 `ConnectInfo` 存储在请求扩展中，以便 [`ConnectInfo`] 可以提取它。

这使提取客户端远程地址等内容成为可能。

开箱即支持提取 [`std::net::SocketAddr`]：

```rust
use axum::{
    extract::ConnectInfo,
    routing::get,
    Router,
};
use std::net::SocketAddr;

let app = Router::new().route("/", get(handler));

async fn handler(ConnectInfo(addr): ConnectInfo<SocketAddr>) -> String {
    format!("Hello {addr}")
}

# async {
let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await;
# };
```

你可以像这样实现自定义 [`Connected`]：

```rust
use axum::{
    extract::connect_info::{ConnectInfo, Connected},
    routing::get,
    serve::IncomingStream,
    Router
};
use tokio::net::TcpListener;

let app = Router::new().route("/", get(handler));

async fn handler(
    ConnectInfo(my_connect_info): ConnectInfo<MyConnectInfo>,
) -> String {
    format!("Hello {my_connect_info:?}")
}

#[derive(Clone, Debug)]
struct MyConnectInfo {
    // ...
}

impl Connected<IncomingStream<'_, TcpListener>> for MyConnectInfo {
    fn connect_info(target: IncomingStream<'_, TcpListener>) -> Self {
        MyConnectInfo {
            // ...
        }
    }
}

# async {
let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
axum::serve(listener, app.into_make_service_with_connect_info::<MyConnectInfo>()).await;
# };
```

参阅 [unix domain socket 示例][uds] 了解如何使用它收集 UDS 连接信息的示例。

[`MakeService`]: tower::make::MakeService
[`Connected`]: crate::extract::connect_info::Connected
[`ConnectInfo`]: crate::extract::connect_info::ConnectInfo
[uds]: https://github.com/tokio-rs/axum/blob/main/examples/unix-domain-socket/src/main.rs
