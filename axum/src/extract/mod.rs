//! 提取器模块 - 从 HTTP 请求中提取数据的类型和方法
//!
//! 在 Spring Boot 中，这相当于各种注解：
//! - @PathVariable - 路径变量提取
//! - @RequestParam - 查询参数提取
//! - @RequestBody - 请求体提取
//! - @RequestHeader - 请求头提取
//! - @CookieValue - Cookie 提取
//! - @RequestAttribute - 请求属性提取
//!
//! 提取器是实现了 `FromRequest` 或 `FromRequestParts` 特质的类型。
//! 它们用于分解传入请求以获取处理器所需的各个部分。
//!
//! 更多详情请参阅：https://docs.rs/axum/latest/axum/extract/index.html

#![doc = include_str!("../docs/extract.md")]

use http::header::{self, HeaderMap};

// 公共模块导出
#[cfg(feature = "tokio")]
pub mod connect_info; /// 连接信息提取器（获取客户端 IP、端口等）
pub mod path;          /// 路径参数提取器（@PathVariable）
pub mod rejection;      /// 提取器拒绝/错误处理

#[cfg(feature = "ws")]
pub mod ws;            /// WebSocket 升级提取器

// 内部模块
pub(crate) mod nested_path; /// 嵌套路径处理
#[cfg(feature = "original-uri")]
mod original_uri;    /// 原始 URI 提取器
mod raw_form;        /// 原始表单数据提取器
mod raw_query;       /// 原始查询字符串提取器
mod state;           /// 应用状态提取器（依赖注入）

// 从 axum-core 重新导出核心提取器特质和类型
// 这些是所有提取器的基础特质
#[doc(inline)]
pub use axum_core::extract::{
    DefaultBodyLimit,      /// 默认请求体大小限制
    FromRef,               /// 从应用状态引用创建特质（@Autowired）
    FromRequest,            /// 从请求创建提取器的特质（核心接口）
    FromRequestParts,       /// 从请求部分创建提取器的特质
    OptionalFromRequest,    /// 可选的 FromRequest请求创建提取器（返回 Option）
    OptionalFromRequestParts, /// 可选的 FromRequestParts（返回 Option）
    Request,               /// HTTP 请求类型
};

// 当启用 macros 特性时，重新导出宏派生的特质
#[cfg(feature = "macros")]
pub use axum_macros::{FromRef, FromRequest, FromRequestParts};

// 重新导出常用提取器类型
#[doc(inline)]
pub use self::{
    nested_path::NestedPath,  /// 嵌套路径信息
    path::{Path, RawPathParams}, /// 路径参数提取器（@PathVariable）
    raw_form::RawForm,        /// 原始表单数据（@RequestParam 但不解析）
    raw_query::RawQuery,       /// 原始查询字符串
    state::State,              /// 应用状态提取器（依赖注入，@Autowired）
};

// 连接信息提取器（当启用 tokio 特性时）
#[doc(inline)]
#[cfg(feature = "tokio")]
pub use self::connect_info::ConnectInfo;

// JSON 提取器（当启用 json 特性时）
// 类似 Spring Boot 的 @RequestBody + Jackson
#[doc(no_inline)]
#[cfg(feature = "json")]
pub use crate::Json;

// Extension 提取器
// 类似 Spring Boot 的 @RequestAttribute 或 Filter 中设置的属性
#[doc(no_inline)]
pub use crate::Extension;

// Form 表单提取器（当启用 form 特性时）
// 类似 Spring Boot 的 @RequestParam 用于表单数据
#[cfg(feature = "form")]
#[doc(no_inline)]
pub use crate::form::Form;

// 匹配路径提取器（当启用 matched-path 特性时）
#[cfg(feature = "matched-path")]
pub(crate) mod matched_path;

#[cfg(feature = "matched-path")]
#[doc(inline)]
pub use self::matched_path::MatchedPath;

// Multipart 表单提取器（当启用 multipart 特性时）
// 类似 Spring Boot 的 @RequestPart 处理 multipart/form-data
#[cfg(feature = "multipart")]
pub mod multipart;

#[cfg(feature = "multipart")]
#[doc(inline)]
pub use self::multipart::Multipart;

// Query 查询参数提取器（当启用 query 特性时）
// 类似 Spring Boot 的 @RequestParam
#[cfg(feature = "query")]
mod query;

#[cfg(feature = "query")]
#[doc(inline)]
pub use self::query::Query;

// 原始 URI 提取器（当启用 original-uri 特性时）
#[cfg(feature = "original-uri")]
#[doc(inline)]
pub use self::original_uri::OriginalUri;

// WebSocket 升级提取器（当启用 ws 特性时）
// 类似 Spring Boot 的 WebSocket 支持
#[cfg(feature = "ws")]
#[doc(inline)]
pub use self::ws::WebSocketUpgrade;

// 内部函数：检查请求是否具有指定的 Content-Type
// 这个函数在 `axum-extra/src/extract/form.rs` 中也有重复
pub(super) fn has_content_type(headers: &HeaderMap, expected_content_type: &mime::Mime) -> bool {
    let Some(content_type) = headers.get(header::CONTENT_TYPE) else {
        return false;
    };

    let Ok(content_type) = content_type.to_str() else {
        return false;
    };

    content_type.starts_with(expected_content_type.as_ref())
}

#[cfg(test)]
mod tests {
    use crate::{routing::get, test_helpers::*, Router};

    #[crate::test]
    async fn consume_body() {
        // 测试将请求体作为 String 提取
        let app = Router::new().route("/", get(|body: String| async { body }));

        let client = TestClient::new(app);
        let res = client.get("/").body("foo").await;
        let body = res.text().await;

        assert_eq!(body, "foo");
    }
}
