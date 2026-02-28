//! 用于生成响应的额外类型
//!
//! 这些响应类型提供了在 axum 中找不到的额外功能。
//!
//! 在 Spring Boot 中，这相当于：
//! - MediaType.APPLICATION_JSON_VALUE 等常量 - 内容类型设置
//! - ResponseEntity.headers() - 响应头设置
//! - @RequestMapping produces 属性 - 内容类型声明

// 内部模块

#[cfg(any(feature = "attachment", feature = "file-stream"))]
mod content_disposition;  /// Content-Disposition 处理

#[cfg(feature = "erased-json")]
mod erased_json;  /// 类型擦除 JSON

#[cfg(feature = "attachment")]
mod attachment;  /// 附件响应

#[cfg(feature = "multipart")]
pub mod multiple;  /// Multipart 响应

#[cfg(feature = "error-response")]
mod error_response;  /// 内部服务器错误响应

#[cfg(feature = "file-stream")]
/// 文件流处理模块
pub mod file_stream;

// 公共导出

#[cfg(feature = "file-stream")]
pub use file_stream::FileStream; /// 文件流响应

#[cfg(feature = "error-response")]
pub use error_response::InternalServerError; /// 内部服务器错误

#[cfg(feature = "erased-json")]
pub use erased_json::ErasedJson; /// 类型擦除 JSON

// 非公共 API（内部使用）
#[cfg(feature = "erased-json")]
#[doc(hidden)]
pub use erased_json::private as __private_erased_json;

#[cfg(feature = "json-lines")]
#[doc(no_inline)]
pub use crate::json_lines::JsonLines; /// JSON 行流

#[cfg(feature = "attachment")]
pub use attachment::Attachment; /// 附件响应

// 宏：为 MIME 类型生成响应包装器
// 这个宏自动设置正确的 Content-Type header
macro_rules! mime_response {
    (
        $(#[$m:meta])*
        $ident:ident,
        $mime:ident,
    ) => {
        mime_response! {
            $(#[$m])*
            $ident,
            mime::$mime.as_ref(),
        }
    };
    (
        $(#[$m:meta])*
        $ident:ident,
        $mime:expr,
    ) => {
        $(#[$m])*
        #[derive(Clone, Copy, Debug)]
        #[must_use]
        pub struct $ident<T>(pub T);

        impl<T> axum_core::response::IntoResponse for $ident<T>
        where
            T: axum_core::response::IntoResponse,
        {
            fn into_response(self) -> axum_core::response::Response {
                (
                    [
                        (
                            http::header::CONTENT_TYPE,
                            http::HeaderValue::from_static($mime),
                        )
                    ],
                    self.0,
                )
                    .into_response()
            }
        }

        impl<T> From<T> for $ident<T> {
            fn from(inner: T) -> Self {
                Self(inner)
            }
        }
    };
}

// JavaScript 响应
// 自动设置 Content-Type: application/javascript; charset=utf-8
// 类似 Spring Boot: produces = MediaType.APPLICATION_JAVASCRIPT_VALUE
mime_response! {
    /// JavaScript 响应
    ///
    /// 自动获得 `Content-Type: application/javascript; charset=utf-8`
    ///
    /// 在 Spring Boot 中，这相当于：
    /// ```java
    /// @GetMapping(produces = MediaType.APPLICATION_JAVASCRIPT_VALUE)
    /// @ResponseBody
    /// public String getScript() { return "alert('hello');"; }
    /// ```
    JavaScript,
    APPLICATION_JAVASCRIPT_UTF_8,
}

// CSS 响应
// 自动设置 Content-Type: text/css; charset=utf-8
// 类似 Spring Boot: produces = MediaType.TEXT_CSS_VALUE
mime_response! {
    /// CSS 响应
    ///
    /// 自动获得 `Content-Type: text/css; charset=utf-8`
    ///
    /// 在 Spring Boot 中，这相当于：
    /// ```java
    /// @GetMapping(produces = MediaType.TEXT_CSS_VALUE)
    /// @ResponseBody
    /// public String getStylesheet() { return "body { color: red; }"; }
    /// ```
    Css,
    TEXT_CSS_UTF_8,
}

// WebAssembly 响应
// 自动设置 Content-Type: application/wasm
mime_response! {
    /// WebAssembly 响应
    ///
    /// 自动获得 `Content-Type: application/wasm`
    ///
    /// 在 Spring Boot 中，这相当于：
    /// ```java
    /// @GetMapping(produces = MediaType.APPLICATION_OCTET_STREAM_VALUE)
    /// @ResponseBody
    /// public byte[] getWasm() { return wasmBytes; }
    /// ```
    Wasm,
    "application/wasm",
}

// 当启用 typed-header 特性时，重新导出 TypedHeader
#[cfg(feature = "typed-header")]
#[doc(no_inline)]
pub use crate::typed_header::TypedHeader;
