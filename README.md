# axum - 使用AI翻译版

`axum` 是一个专注于人体工程学和模块化的 HTTP 路由和请求处理库。

[![构建状态](https://github.com/tokio-rs/axum/actions/workflows/CI.yml/badge.svg?branch=main)](https://github.com/tokio-rs/axum/actions/workflows/CI.yml)
[![Crates.io](https://img.shields.io/crates/v/axum)](https://crates.io/crates/axum)
[![文档](https://docs.rs/axum/badge.svg)][docs]

有关此 crate 的更多信息可以在 [crate 文档][docs] 中找到。

## 主要特性

- 使用无宏 API 将请求路由到处理程序。
- 使用提取器声明式地解析请求。
- 简单且可预测的错误处理模型。
- 以最少的样板代码生成响应。
- 充分利用 [`tower`] 和 [`tower-http`] 生态系统中的中间件、服务和工具。

特别是最后一点是 `axum` 与其他库/框架的区别所在。`axum` 没有自己的中间件系统，而是使用 [`tower::Service`]。这意味着 `axum` 免费获得了超时、追踪、压缩、授权等功能。它还使您能够与使用 [`hyper`] 或 [`tonic`] 编写的应用程序共享中间件。

## ⚠ 破坏性变更 ⚠

我们目前正在致力于 axum 0.9 版本，因此 `main` 分支包含破坏性变更。有关发布到 crates.io 的内容，请参阅 [`0.8.x`] 分支。

[`0.8.x`]: https://github.com/tokio-rs/axum/tree/v0.8.x

## 使用示例

```rust
use axum::{
    routing::{get, post},
    http::StatusCode,
    Json, Router,
};
use serde::{Deserialize, Serialize};

#[tokio::main]
async fn main() {
    // 初始化追踪
    tracing_subscriber::fmt::init();

    // 使用路由构建我们的应用程序
    let app = Router::new()
        // `GET /` 转到 `root`
        .route("/", get(root))
        // `POST /users` 转到 `create_user`
        .route("/users", post(create_user));

    // 使用 hyper 运行我们的应用程序，在 3000 端口全局监听
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await;
}

// 响应静态字符串的基本处理程序
async fn root() -> &'static str {
    "Hello, World!"
}

async fn create_user(
    // 此参数告诉 axum 将请求体解析为 JSON，
    // 并转换为 `CreateUser` 类型
    Json(payload): Json<CreateUser>,
) -> (StatusCode, Json<User>) {
    // 在此处插入您的应用程序逻辑
    let user = User {
        id: 1337,
        username: payload.username,
    };

    // 这将被转换为 JSON 响应
    // 状态码为 `201 Created`
    (StatusCode::CREATED, Json(user))
}

// 我们的 `create_user` 处理程序的输入
#[derive(Deserialize)]
struct CreateUser {
    username: String,
}

// 我们的 `create_user` 处理程序的输出
#[derive(Serialize)]
struct User {
    id: u64,
    username: String,
}
```

您可以在 [示例目录][examples] 中找到此 [示例][readme-example] 以及其他示例项目。

更多示例请参阅 [crate 文档][docs]。

## 性能

`axum` 是建立在 [`hyper`] 之上的相对较薄的层，增加的开销非常少。因此 `axum` 的性能与 [`hyper`] 相当。您可以在 [这里](https://github.com/programatik29/rust-web-benchmarks) 和 [这里](https://web-frameworks-benchmark.netlify.app/result?l=rust) 找到基准测试。

## 安全性

此 crate 使用 `#![forbid(unsafe_code)]` 来确保所有内容都以 100% 安全的 Rust 实现。

## 最低支持的 Rust 版本

axum 的 MSRV 是 1.80。

## 示例

[示例] 文件夹包含有关如何使用 `axum` 的各种示例。[文档] 也提供了大量代码片段和示例。对于完整的示例，请查看社区维护的 [展示项目] 或 [教程]。

## 获取帮助

在 `axum` 仓库中，我们还有[许多示例][examples]，展示如何将所有内容组合在一起。社区维护的 [展示项目] 和 [教程] 也演示了如何在实际应用程序中使用 `axum`。欢迎您在 [Discord 频道][chat] 中提问或开启 [讨论] 来提出您的问题。

## 社区项目

有关使用 `axum` 构建的社区维护的 crate 和项目列表，请参见[这里][ecosystem]。

## 贡献

🎈 感谢您帮助改进项目！我们很高兴有您的加入！我们有[贡献指南][contributing] 来帮助您参与 `axum` 项目。

## 许可证

本项目采用 [MIT 许可证][license]。

### 贡献

除非您明确声明，否则您有意提交供包含在 `axum` 中的任何贡献均应以 MIT 许可，无需任何附加条款或条件。

[readme-example]: https://github.com/tokio-rs/axum/tree/main/examples/readme
[examples]: https://github.com/tokio-rs/axum/tree/main/examples
[docs]: https://docs.rs/axum
[`tower`]: https://crates.io/crates/tower
[`hyper`]: https://crates.io/crates/hyper
[`tower-http`]: https://crates.io/crates/tower-http
[`tonic`]: https://crates.io/crates/tonic
[contributing]: https://github.com/tokio-rs/axum/blob/main/CONTRIBUTING.md
[chat]: https://discord.gg/tokio
[discussion]: https://github.com/tokio-rs/axum/discussions/new?category=q-a
[`tower::Service`]: https://docs.rs/tower/latest/tower/trait.Service.html
[ecosystem]: https://github.com/tokio-rs/axum/blob/main/ECOSYSTEM.md
[showcases]: https://github.com/tokio-rs/axum/blob/main/ECOSYSTEM.md#project-showcase
[tutorials]: https://github.com/tokio-rs/axum/blob/main/ECOSYSTEM.md#tutorials
[license]: https://github.com/tokio-rs/axum/blob/main/axum/LICENSE
