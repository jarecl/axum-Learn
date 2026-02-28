//! 额外提取器
//!
//! 这些提取器提供在 axum 中找不到的额外功能。
//!
//! 在 Spring Boot 中，这相当于：
//! - spring-boot-starter-* 系列中的额外功能
//! - 第三方库提供的额外注解（如 @CookieValue, @RequestPart 等）

// 内部模块

#[cfg(feature = "cached")]
mod cached;              /// 缓存提取器（避免重复计算）

#[cfg(feature = "with-rejection")]
mod with_rejection;     /// 自定义拒绝类型的包装器

#[cfg(feature = "form")]
mod form;                /// 表单数据提取器

#[cfg(feature = "cookie")]
pub mod cookie;           /// Cookie 提取器（@CookieCookieValue）

#[cfg(feature = "json-deserializer")]
mod json_deserializer;     /// JSON 反序列化器提取器

#[cfg(feature = "query")]
mod query;               /// 查询参数提取器（@RequestParam）

#[cfg(feature = "multipart")]
pub mod multipart;          /// Multipart 表单提取器（@RequestPart）

// 公共导出

#[cfg(feature = "cached")]
pub use self::cached::Cached; /// 缓存提取器

#[cfg(feature = "with-rejection")]
pub use self::with_rejection::WithRejection; /// 自定义拒绝包装器

#[cfg(feature = "cookie")]
pub use self::cookie::CookieJar; /// Cookie Jar（Spring Boot 的 Cookie 管理）

#[cfg(feature = "cookie-private")]
pub use self::cookie::PrivateCookieJar; /// 私有 Cookie Jar（加密）

#[cfg(feature = "cookie-signed")]
pub use self::cookie::SignedCookieJar; /// 签名 Cookie Jar（签名）

#[cfg(feature = "form")]
#[allow(deprecated)]
pub use self::form::{Form, FormRejection}; /// 表单提取器

#[cfg(feature = "query")]
pub use self::query::OptionalQuery; /// 可选查询参数提取器

#[cfg(feature = "multipart")]
pub use self::multipart::Multipart; /// Multipart 提取器

#[cfg(feature = "json-deserializer")]
pub use self::json_deserializer::{
    JsonDataError,       /// JSON 数据错误
    JsonDeserializer,      /// JSON 反序列化器
    JsonDeserializerRejection, /// JSON 反序列化器拒绝
    JsonSyntaxError,      /// JSON 语法错误
    MissingJsonContentType, /// 缺失 JSON Content-Type
};

#[cfg(feature = "json-lines")]
#[doc(no_inline)]
pub use crate::json_lines::JsonLines; /// JSON 行流

#[cfg(feature = "typed-header")]
#[doc(no_inline)]
pub use crate::typed_header::TypedHeader; /// 类型化头部提取器
