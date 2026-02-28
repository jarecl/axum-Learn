//! axum-core - [`axum`] 的核心类型和特质
//!
//! 这是一个低级库，包含 axum 框架的核心构建块。
//!
//! ## 为什么需要 axum-core？
//!
//! 库作者如果想要提供 [`FromRequest`] 或 [`IntoResponse`] 实现
//! 应该依赖 [`axum-core`] crate 而不是 `axum`（如果可能）。
//!
//! `axum-core` 包含核心类型和特质，不太可能接收破坏性更改。
//! 它与 `axum` 主 crate 分离，以便：
//! 1. 减少依赖 - 用户实现自己的提取器/响应不需要依赖整个 axum
//! 2. 稳定性 - 核心 API 更稳定，不易发生破坏性更改
//! 3. 可组合性 - 第三方库可以提供与 axum 兼容的类型，而不需要依赖完整的框架
//!
//! ## 核心概念
//!
//! - [`FromRequest`]: 从请求中提取数据的特质（类似 Spring Boot 的参数绑定）
//! - [`FromRequestParts`]: 从请求部分提取数据的特质
//! - [`IntoResponse`]: 将类型转换为 HTTP 响应的特质
//! - [`Request`]: HTTP 请求类型
//! - [`Response`]: HTTP 响应类型
//!
//! 在 Java/Spring Boot 中的对应概念：
//! - `FromRequest` ≈ @RequestParam, @PathVariable, @RequestBody 等注解的功能
//! - `IntoResponse` ≈ @ResponseBody + ResponseEntity
//! - `Request` ≈ HttpServletRequest
//! - `Response` ≈ HttpServletResponse / ResponseEntity
//!
//! ## 示例：为自定义类型实现 FromRequest
//!
//! ```rust
//! use axum_core::extract::{FromRequest, Request};
//! use async_trait::async_trait;
//!
//! struct MyCustomType {
//!     // 自定义字段
//! }
//!
//! #[async_trait]
//! impl<S> FromRequest<S> for MyCustomType
//! where
//!     S: Send + Sync,
//! {
//!     type Rejection = http::StatusCode;
//!
//!     async fn from_request(req: Request, _state: &S) -> Result<Self, Self::Rejection> {
//!         // 从请求中提取数据
//!         Ok(MyCustomType {})
//!     }
//! }
//! ```
//!
//! [`FromRequest`]: crate::extract::FromRequest
//! [`IntoResponse`]: crate::response::IntoResponse
//! [`axum`]: https://crates.io/crates/axum
//! [`axum-core`]: http://crates.io/crates/axum-core

// 测试时允许浮点数比较
#![cfg_attr(test, allow(clippy::float_cmp))]
// 在非测试代码中警告使用 print_stdout 和 dbg_macro
#![cfg_attr(not(test), warn(clippy::print_stdout, clippy::dbg_macro))]

// 声明宏模块（内部使用）
#[macro_use]
pub(crate) mod macros;

// 文档中标记为隐藏的模块（宏辅助工具）
#[doc(hidden)] // macro helpers
pub mod __private {
    #[cfg(feature = "tracing")]
    pub use tracing;
}

// 内部模块
mod error;          /// 错误类型定义
mod ext_traits;      /// 扩展特质（RequestExt, RequestPartsExt）

// 公共导出
pub use self::error::Error; /// 错误类型

// 公共模块
pub mod body;          /// HTTP 消息体处理
pub mod extract;        /// 提取器特质和类型
pub mod response;       /// 响应构建特质和类型

/// 类型擦除的错误类型
///
/// 类似 Java 中的 `Throwable` 或 Rust 中的 `Box<dyn Error + Send + Sync>`
///
/// 用于在异步上下文中传递错误，需要满足 Send + Sync 约束
pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

// 重新导出扩展特质
pub use self::ext_traits::{request::RequestExt, request_parts::RequestPartsExt};

// 在测试中使用 axum_macros 提供的测试辅助
#[cfg(test)]
use axum_macros::__private_axum_test as test;
