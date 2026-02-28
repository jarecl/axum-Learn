//! [`axum`] 的额外工具库
//!
//! 这是 axum 框架的扩展库，提供了额外的功能和实用工具。
//!
//! 在 Spring Boot 中，这相当于：
//! - spring-boot-starter-* 系列的额外 starter
//! - 第三方库的自动配置模块
//!
//! ## 与 axum 的区别
//!
//! - `axum` - 核心框架，提供基本功能
//! - `axum-extra` - 额外功能，减少编译时依赖，按需启用
//!
//! # 特性标志
//!
//! axum-extra 使用一组 [特性标志] 来减少编译的和可选依赖的数量。
//!
//! 以下可选特性可用：
//!
//! 名称 | 描述 | 默认？
//! ---|---|---
//! `async-read-body` | 启用 [`AsyncReadBody`](crate::body::AsyncReadBody) body |
//! `attachment` | 启用 [`Attachment`](crate::response::Attachment) 响应 |
//! `cached` | 启用 [`Cached`](crate::extract::Cached) 提取器 |
//! `cookie` | 启用 [`CookieJar`](crate::extract::CookieJar) 提取器 |
//! `cookie-private` | 启用 [`PrivateCookieJar`](crate::extract::PrivateCookieJar) 提取器 |
//! `cookie-signed` | 启用 [`SignedCookieJar`](crate::extract::SignedCookieJar) 提取器 |
//! `cookie-key-expansion` | 启用 [`Key::derive_from`](crate::extract::cookie::Key::derive_from) 方法 |
//! `erased-json` | 启用 [`ErasedJson`](crate::response::ErasedJson) 响应 |
//! `error-response` | 启用 [`InternalServerError`](crate::response::InternalServerError) 响应 |
//! `form` (已弃用) | 启用 [`Form`](crate::extract::Form) 提取器 |
//! `handler` | 启用 [handler]（处理器）工具 |
//! `json-deserializer` | 启用 [`JsonDeserializer`](crate::extract::JsonDeserializer) 提取器 |
//! `json-lines` | 启用 [`JsonLines`](crate::extract::JsonLines) 提取器和响应 |
//! `middleware` | 启用 [middleware]（中间件）工具 |
//! `multipart` | 启用 [`Multipart`](crate::extract::Multipart) 提取器 |
//! `protobuf` | 启用 [`Protobuf`](crate::protobuf::Protobuf) 提取器和响应 |
//! `query` (已弃用) | 启用 [`Query`](crate::extract::Query) 提取器 |
//! `routing` | 启用 [routing]（路由）工具 |
//! `tracing` | 记录内置提取器的拒绝 | <span role="img" aria-label="Default feature">✔</span>
//! `typed-routing` | 启用 [`TypedPath`](crate::routing::TypedPath) 路由工具和 `routing` 特性。 |
//! `typed-header` | 启用 [`TypedHeader`] 提取器和响应 |
//! `file-stream` | 启用 [`FileStream`](crate::response::FileStream) 响应 |
//! `with-rejection` | 启用 [`WithRejection`](crate::extract::WithRejection) 提取器 |
//!
//! [`axum`]: https://crates.io/crates/axum

#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(test, allow(clippy::float_cmp))]
#![cfg_attr(not(test), warn(clippy::print_stdout, clippy::dbg_macro))]

// 允许未使用的外部 crate
#[allow(unused_extern_crates)]
extern crate self as axum_extra;

// 公共模块
pub mod body;          /// HTTP 消息体处理
pub mod either;        /// Either 类型（类似 Either from either crate）
pub mod extract;        /// 额外提取器
pub mod response;       /// 额外响应类型

// 路由工具模块（当启用 routing 特性时）
#[cfg(feature = "routing")]
pub mod routing;

// 中间件工具模块（当启用 middleware 特性时）
#[cfg(feature = "middleware")]
pub mod middleware;

// 处理器工具模块（当启用 handler 特性时）
#[cfg(feature = "handler")]
pub mod handler;

// JSON 行处理模块（当启用 json-lines 特性时）
#[cfg(feature = "json-lines")]
pub mod json_lines;

// 类型化头部模块（当启用 typed-header 特性时）
#[cfg(feature = "typed-header")]
pub mod typed_header;

// 当启用 typed-header 特性时，重新导出 headers crate
#[cfg(feature = "typed-header")]
#[doc(no_inline)]
pub use headers;

// 当启用 typed-header 特性时，重新导出 TypedHeader
#[cfg(feature = "typed-header")]
#[doc(inline)]
pub use typed_header::TypedHeader;

// Protobuf 模块（当启用 protobuf 特性时）
#[cfg(feature = "protobuf")]
pub mod protobuf;

// 非公共 API（内部使用）
#[cfg(feature = "typed-routing")]
#[doc(hidden)]
pub mod __private {
    use percent_encoding::{AsciiSet, CONTROLS};

    pub use percent_encoding::utf8_percent_encode;

    // 来自 https://github.com/servo/rust-url/blob/master/url/src/parser.rs
    // 用于 URL 编码的字符集
    const FRAGMENT: &AsciiSet = &CONTROLS.add(b' ').add(b'"').add(b'<').add(b'>').add(b'`');
    const PATH: &AsciiSet = &FRAGMENT.add(b'#').add(b'?').add(b'{').add(b'}');
    pub const PATH_SEGMENT: &AsciiSet = &PATH.add(b'/').add(b'%');
}

#[cfg(test)]
pub(crate) use axum::test_helpers;
