//! axum - 基于 tower 生态系统的 Rust 异步 Web 框架
//!
//! # 核心特性
//!
//! - 使用无宏 API 进行路由配置（类似 Spring Boot 的 @RequestMapping）
//! - 声明式请求参数提取（类似 Spring Boot 的 @RequestParam, @PathVariable, @RequestBody）
//! - 简单可预测的错误处理模型
//! - 最小化样板代码生成响应
//! - 完全利用 [`tower`] 和 [`tower-http`] 生态系统的中间件、服务和工具
//!
//! 特别是最后一点，这是 `axum` 与其他框架的主要区别。
//! `axum` 没有自己的中间件系统，而是使用 [`tower::Service`]。
//! 这意味着 axum 免费获得超时、追踪、压缩、授权等功能。
//! 它也使你能够与使用 [`hyper`] 或 [`tonic`] 编写的应用程序共享中间件。
//!
//! # 兼容性
//!
//! axum 专为与 [tokio] 和 [hyper] 协同工作而设计。
//! 运行时和传输层独立性至少在目前不是目标。
//!
//! # 示例
//!
//! axum 的 "Hello, World!" 如下：
//!
//! ```rust,no_run
//! use axum::{
//!     routing::get,  // 相当于 Spring Boot 的 @GetMapping
//!     Router,        // 相当于 Spring Boot 的 RouterFunction
//! };
//!
//! #[tokio::main]
//! async fn main() {
//!     // 构建只有一个路由的应用
//!     // 类似 Spring Boot: @GetMapping("/") public String hello() { return "Hello, World!"; }
//!     let app = Router::new().route("/", get(|| async { "Hello, World!" }));
//!
//!     // 使用 hyper 运行应用，监听全局端口 3000
//!     // 类似 Spring Boot 的 SpringApplication.run(port)
//!     let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
//!     axum::serve(listener, app).await;
//! }
//! ```
//!
//! 注意：使用 `#[tokio::main]` 需要启用 tokio 的 `macros` 和 `rt-multi-thread` 特性，
//! 或者直接使用 `full` 来启用所有特性（`cargo add tokio --features macros,rt-multi-thread`）。
//!
//! # 路由
//!
//! [`Router`] 用于设置哪些路径映射到哪些服务：
//!
//! ```rust
//! use axum::{Router, routing::get};
//!
//! // 构建路由器
//! // 类似 Spring Boot:
//! // @GetMapping("/")
//! // @PostMapping("/foo")
//! // @GetMapping("/foo")
//! // @GetMapping("/foo/bar")
//! let app = Router::new()
//!     .route("/", get(root))
//!     .route("/foo", get(get_foo).post(post_foo))
//!     .route("/foo/bar", get(foo_bar));
//!
//! // 调用以下处理器之一
//! // 这些处理器相当于 Spring Boot 的 Controller 方法
//! async fn root() {}
//! async fn get_foo() {}
//! async fn post_foo() {}
//! async fn foo_bar() {}
//! # let _: Router = app;
//! ```
//!
//! 更多路由详情请参阅 [`Router`]。
//!
//! # 处理器
//!
#![doc = include_str!("docs/handlers_intro.md")]
//!
//! 更多处理器详情请参阅 [`handler`](crate::handler)。
//!
//! # 提取器
//!
//! 提取器是实现了 [`FromRequest`] 或 [`FromRequestParts`] 的类型。
//! 提取器用于分解传入请求以获取处理器所需的各个部分。
//! 在 Spring Boot 中，这相当于各种注解：@RequestParam, @PathVariable, @RequestBody, @RequestHeader 等。
//!
//! ```rust
//! use axum::extract::{Path, Query, Json};
//! use std::collections::HashMap;
//!
//! // `Path` 获取路径参数并反序列化它们
//! // 类似 Spring Boot 的 @PathVariable
//! async fn path(Path(user_id): Path<u32>) {}
//!
//! // `Query` 获取查询参数并反序列化它们
//! // 类似 Spring Boot 的 @RequestParam
//! async fn query(Query(params): Query<HashMap<String, String>>) {}
//!
//! // 缓冲请求体并作为 JSON 反序列化到 `serde_json::Value`
//! // `Json` 支持任何实现 `serde::Deserialize` 的类型
//! // 类似 Spring Boot 的 @RequestBody
//! async fn json(Json(payload): Json<serde_json::Value>) {}
//! ```
//!
//! 更多提取器详情请参阅 [`extract`](crate::extract)。
//!
//! # 响应
//!
//! 任何实现了 [`IntoResponse`] 的类型都可以从处理器返回。
//! 类似 Spring Boot 的 @ResponseBody 或返回 ResponseEntity。
//!
//! ```rust,no_run
//! use axum::{
//!     body::Body,
//!     routing::get,
//!     response::Json,
//!     Router,
//! };
//! use serde_json::{Value, json};
//!
//! // `&'static str` 变为 `200 OK`，带有 `content-type: text/plain; charset=utf-8`
//! // 类似 Spring Boot: @GetMapping public String plainText() { return "foo"; }
//! async fn plain_text() -> &'static str {
//!     "foo"
//! }
//!
//! // `Json` 提供 `application/json` 内容类型，适用于任何实现 `serde::Serialize` 的类型
//! // 类似 Spring Boot: @GetMapping @ResponseBody public Map<String, Object> json() { ... }
//! async fn json() -> Json<Value> {
//!     Json(json!({ "data": 42 }))
//! }
//!
//! let app = Router::new()
//!     .route("/plain_text", get(plain_text))
//!     .route("/json", get(json));
//! # let _: Router = app;
//! ```
//!
//! 更多构建响应的详情请参阅 [`response`](crate::response)。
//!
//! # 错误处理
//!
//! axum 旨在有一个简单可预测的错误处理模型。
//! 这意味着将错误转换为响应很简单，并且保证所有错误都被处理。
//!
//! 更多 axum 错误处理模型以及如何优雅处理错误的详情，请参阅 [`error_handling`]。
//!
//! # 中间件
//!
//! 有几种不同的方式可以为 axum 编写中间件。
//! 更多详情请参阅 [`middleware`]。
//!
//! # 与处理器共享状态
//!
//! 在处理器之间共享一些状态是很常见的。
//! 例如，数据库连接池或其他服务的客户端可能需要被共享。
//!
//! 最常用的四种方式：
//!
//! - 使用 [`State`] 提取器（推荐，类似 Spring 的依赖注入）
//! - 使用请求扩展
//! - 使用闭包捕获
//! - 使用任务本地变量
//!
//! ## 使用 [`State`] 提取器
//!
//! ```rust,no_run
//! use axum::{
//!     extract::State,  // 类似 Spring 的 @Autowired
//!     routing::get,
//!     Router,
//! };
//! use std::sync::Arc;
//!
//! struct AppState {
//!     // ...
//! }
//!
//! let shared_state = Arc::new(AppState { /* ... */ });
//!
//! // 类似 Spring Boot: @Component ApplicationContext
//! let app = Router::new()
//!     .route("/", get(handler))
//!     .with_state(shared_state);  // 依赖注入
//!
//! async fn handler(
//!     State(state): State<Arc<AppState>>,
//! ) {
//!     // ...
//! }
//! # let _: Router = app;
//! ```
//!
//! 如果可能，你应该优先使用 [`State`]，因为它更类型安全。
//! 缺点是它比任务本地变量和请求扩展更不动态。
//!
//! 更多访问状态的详情请参阅 [`State`]。
//!
//! ## 使用请求扩展
//!
//! 与处理器共享状态的另一种方式是使用 [`Extension`] 作为层和提取器：
//!
//! ```rust,no_run
//! use axum::{
//!     extract::Extension,
//!     routing::get,
//!     Router,
//! };
//! use std::sync::Arc;
//!
//! struct AppState {
//!     // ...
//! }
//!
//! let shared_state = Arc::new(AppState { /* ... */ });
//!
//! let app = Router::new()
//!     .route("/", get(handler))
//!     .layer(Extension(shared_state));
//!
//! async fn handler(
//!     Extension(state): Extension<Arc<AppState>>,
//! ) {
//!     // ...
//! }
//! # let _: Router = app;
//! ```
//!
//! 这种方法的缺点是如果你尝试提取一个不存在的扩展
//! （也许是因为你忘记添加中间件或提取了错误的类型），
//! 你将得到运行时错误（特别是 `500 Internal Server Error` 响应）。
//!
//! ## 使用闭包捕获
//!
//! 状态也可以使用闭包捕获直接传递给处理器：
//!
//! ```rust,no_run
//! use axum::{
//!     Json,
//!     extract::{Extension, Path},
//!     routing::{get, post},
//!     Router,
//! };
//! use std::sync::Arc;
//! use serde::Deserialize;
//!
//! struct AppState {
//!     // ...
//! }
//!
//! let shared_state = Arc::new(AppState { /* ... */ });
//!
//! let app = Router::new()
//!     .route(
//!         "/users",
//!         post({
//!             let shared_state = Arc::clone(&shared_state);
//!             move |body| create_user(body, shared_state)
//!         }),
//!     )
//!     .route(
//!         "/users/{id}",
//!         get({
//!             let shared_state = Arc::clone(&shared_state);
//!             move |path| get_user(path, shared_state)
//!         }),
//!     );
//!
//! async fn get_user(Path(user_id): Path<String>, state: Arc<AppState>) {
//!     // ...
//! }
//!
//! async fn create_user(Json(payload): Json<CreateUserPayload>, state: Arc<AppState>) {
//!     // ...
//! }
//!
//! #[derive(Deserialize)]
//! struct CreateUserPayload {
//!     // ...
//! }
//! # let _: Router = app;
//! ```
//!
//! 这种方法的缺点是它是最冗长的。
//!
//! ## 使用任务本地变量
//!
//! 这也允许与 `IntoResponse` 实现共享状态：
//!
//! ```rust,no_run
//! use axum::{
//!     extract::Request,
//!     http::{header, StatusCode},
//!     middleware::{self, Next},
//!     response::{IntoResponse, Response},
//!     routing::get,
//!     Router,
//! };
//! use tokio::task_local;
//!
//! #[derive(Clone)]
//! struct CurrentUser {
//!     name: String,
//! }
//! task_local! {
//!     pub static USER: CurrentUser;
//! }
//!
//! async fn auth(req: Request, next: Next) -> Result<Response, StatusCode> {
//!     let auth_header = req
//!         .headers()
//!         .get(header::AUTHORIZATION)
//!         .and_then(|header| header.to_str().ok())
//!         .ok_or(StatusCode::UNAUTHORIZED)?;
//!     if let Some(current_user) = authorize_current_user(auth_header).await {
//!         // 状态在这里的中间件中设置
//!         Ok(USER.scope(current_user, next.run(req)).await)
//!     } else {
//!         Err(StatusCode::UNAUTHORIZED)
//!     }
//! }
//! async fn authorize_current_user(auth_token: &str) -> Option<CurrentUser> {
//!     Some(CurrentUser {
//!         name: auth_token.to_string(),
//!     })
//! }
//!
//! struct UserResponse;
//!
//! impl IntoResponse for UserResponse {
//!     fn into_response(self) -> Response {
//!         // 状态在这里的 IntoResponse 实现中访问
//!         let current_user = USER.with(|u| u.clone());
//!         (StatusCode::OK, current_user.name).into_response()
//!     }
//! }
//!
//! async fn handler() -> UserResponse {
//!     UserResponse
//! }
//!
//! let app: Router = Router::new()
//!     .route("/", get(handler))
//!     .route_layer(middleware::from_fn(auth));
//! ```
//!
//! 这种方法的主要缺点是它仅在使用的异步执行器具有任务局部变量概念时才有效。
//! 上面的示例使用了 [tokio 的 `task_local` 宏](https://docs.rs/tokio/1/tokio/macro.task_local.html)。
//! smol 目前尚未提供等效功能（参见 [this GitHub issue](https://github.com/smol-rs/async-executor/issues/139)）。
//!
//! # 为 axum 构建集成
//!
//! 库作者如果想要提供 [`FromRequest`]、[`FromRequestParts`] 或
//! [`IntoResponse`] 实现，应该依赖 [`axum-core`] crate 而不是 `axum`（如果可能）。
//! [`axum-core`] 包含核心类型和特质，不太可能接收破坏性更改。
//!
//! # 必需依赖
//!
//! 要使用 axum，你必须引入一些依赖：
//!
//! ```toml
//! [dependencies]
//! axum = "<latest-version>"
//! tokio = { version = "<latest-version>", features = ["full"] }
//! tower = "<latest-version>"
//! ```
//!
//! tokio 的 `"full"` 特性不是必需的，但是开始的最简单方式。
//!
//! Tower 也不是严格必需的，但对测试很有帮助。
//! 参见仓库中的测试示例以了解更多关于测试 axum 应用程序的信息。
//!
//! # 示例
//!
//! axum 仓库包含 [许多示例][examples]，展示如何将所有部分组合在一起。
//!
//! # 特性标志
//!
//! axum 使用一组 [特性标志] 来减少编译的和可选依赖的数量。
//!
//! 以下可选特性可用：
//!
//! 名称 | 描述 | 默认？
//! ---|---|---
//! `http1` | 启用 hyper 的 `http1` 特性 | <span role="img" aria-label="Default feature">✔</span>
//! `http2` | 启用 hyper 的 `http2` 特性 |
//! `json` | 启用 [`Json`] 类型和一些类似的便利功能 | <span role="img" aria-label="Default feature">✔</span>
//! `macros` | 启用可选的实用工具宏 |
//! `matched-path` | 启用捕获每个请求的路由器和 [`MatchedPath`] 提取器 | <span role="img" aria-label="Default feature">✔</span>
//! `multipart` | 启用使用 [`Multipart`] 解析 `multipart/form-data` 请求 |
//! `original-uri` | 启用捕获每个请求的原始 URI 和 [`OriginalUri`] 提取器 | <span role="img" aria-label="Default feature">✔</span>
//! `tokio` | 启用 `tokio` 作为依赖以及 `axum::serve`、`SSE` 和 `extract::connect_info` 类型。 | <span role="img" aria-label="Default feature">✔</span>
//! `tower-log` | 启用 `tower` 的 `log` 特性 | <span role="img" aria-label="Default feature">✔</span>
//! `tracing` | 记录内置提取器的拒绝 | <span role="img" aria-label="Default feature">✔</span>
//! `ws` | 通过 [`extract::ws`] 启用 WebSocket 支持 |
//! `form` | 启用 `Form` 提取器 | <span role="img" aria-label="Default feature">✔</span>
//! `query` | 启用 `Query` 提取器 | <span role="img" aria-label="Default feature">✔</span>
//!
//! [`MatchedPath`]: crate::extract::MatchedPath
//! [`Multipart`]: crate::extract::Multipart
//! [`OriginalUri`]: crate::extract::OriginalUri
//! [`tower`]: https://crates.io/crates/tower
//! [`tower-http`]: https://crates.io/crates/tower-http
//! [`tokio`]: http://crates.io/crates/tokio
//! [`hyper`]: http://crates.io/crates/hyper
//! [`tonic`]: http://crates.io/crates/tonic
//! [feature flags]: https://doc.rust-lang.org/cargo/reference/features.html#the-features-section
//! [`IntoResponse`]: crate::response::IntoResponse
//! [`Timeout`]: tower::timeout::Timeout
//! [examples]: https://github.com/tokio-rs/axum/tree/main/examples
//! [`Router::merge`]: crate::routing::Router::merge
//! [`Service`]: tower::Service
//! [`Service::poll_ready`]: tower::Service::poll_ready
//! [`Service`'s]: tower::Service
//! [`tower::Service`]: tower::Service
//! [tower-guides]: https://github.com/tower-rs/tower/tree/master/guides
//! [`Uuid`]: https://docs.rs/uuid/latest/uuid/
//! [`FromRequest`]: crate::extract::FromRequest
//! [`FromRequestParts`]: crate::extract::FromRequestParts
//! [`HeaderMap`]: http::header::HeaderMap
//! [`Request`]: http::Request
//! [customize-extractor-error]: https://github.com/tokio-rs/axum/blob/main/examples/customize-extractor-error/src/main.rs
//! [axum-macros]: https://docs.rs/axum-macros
//! [`debug_handler`]: https://docs.rs/axum-macros/latest/axum_macros/attr.debug_handler.html
//! [`Handler`]: crate::handler::Handler
//! [`Infallible`]: std::convert::Infallible
//! [load shed]: tower::load_shed
//! [`axum-core`]: http://crates.io/crates/axum-core
//! [`State`]: crate::extract::State

#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(test, allow(clippy::float_cmp))]
#![cfg_attr(not(test), warn(clippy::print_stdout, clippy::dbg_macro))]

// 声明内部宏模块，宏工具
#[macro_use]
pub(crate) mod macros;

// 内部模块声明
mod boxed;
mod extension;
#[cfg(feature = "form")]
mod form;
#[cfg(feature = "json")]
mod json;
mod service_ext;
mod util;

// 公共 API 模块
// 这些模块是 axum 框架的核心组成部分，供用户使用

/// HTTP 消息体处理模块
/// 类似 Spring Boot 的 HttpMessage、HttpEntity
pub mod body;

/// 错误处理模块
/// 类似 Spring Boot 的 @ControllerAdvice、@ExceptionHandler
pub mod error_handling;

/// 请求参数提取模块
/// 类似 Spring Boot 的 @RequestParam、@PathVariable、@RequestBody、@RequestHeader
pub mod extract;

/// 处理器模块
/// 定义处理器的特性和类型，类似 Spring Boot 的 HandlerFunction
pub mod handler;

/// 中间件模块
/// 类似 Spring Boot 的 Filter、Interceptor
pub mod middleware;

/// 响应构建模块
/// 类似 Spring Boot 的 ResponseEntity
pub mod response;

/// 路由模块
/// 类似 Spring Boot 的 @RequestMapping、RouterFunction
pub mod routing;

/// 服务提供模块（当启用 tokio 和 http1/http2 特性时）
/// 用于启动 HTTP 服务器
#[cfg(all(feature = "tokio", any(feature = "http1", feature = "http2")))]
pub mod serve;

/// 测试辅助模块
#[cfg(any(test, feature = "__private"))]
#[allow(missing_docs, missing_debug_implementations, clippy::print_stdout)]
#[doc(hidden)]
pub mod test_helpers;

// 重新导出 http crate 的内容
// http 是 Rust 的 HTTP 类型标准库
#[doc(no_inline)]
pub use http;

// 重新导出 Extension 类型（用于在请求中存储和提取扩展数据）
#[doc(inline)]
pub use self::extension::Extension;

// 重新导出 Json 类型（当启用 json 特性时）
// 类似 Spring Boot 的 @ResponseBody + Jackson 序列化
#[doc(inline)]
#[cfg(feature = "json")]
pub use self::json::Json;

// 重新导出 Router 类型
// 类似 Spring Boot 的 RouterFunction
#[doc(inline)]
pub use self::routing::Router;

// 重新导出 Form 类型（当启用 form 特性时）
// 类似 Spring Boot 的表单数据绑定
#[doc(inline)]
#[cfg(feature = "form")]
pub use self::form::Form;

// 重新导出 axum-core 中的核心类型
// 这些是 axum 框架的基础构建块
#[doc(inline)]
pub use axum_core::{BoxError, Error, RequestExt, RequestPartsExt};

// 重新导出 axum-macros 中的调试宏
// 用于调试处理器和中间件
#[cfg(feature = "macros")]
pub use axum_macros::{debug_handler, debug_middleware};

// 重新导出 serve 函数（当启用 tokio 和 http1/http2 特性时）
// 类似 Spring Boot 的 SpringApplication.run()
#[cfg(all(feature = "tokio", any(feature = "http1", feature = "http2")))]
#[doc(inline)]
pub use self::serve::serve;

// 重新导出 ServiceExt
// 为 Service 类型提供扩展方法
pub use self::service_ext::ServiceExt;

#[cfg(test)]
use axum_macros::__private_axum_test as test;
