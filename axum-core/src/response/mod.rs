//! 生成响应的类型和特质
//!
//! 这是 axum-core 的核心响应模块，定义了所有响应类型的基础特质。
//!
//! 在 Spring Boot 中，这相当于：
//! - ResponseEntity<T> - 泛型响应包装类
//! - @ResponseBody - 响应体注解
//! - HttpStatus - HTTP 状态码
//!
//! 更多详情请参阅 [`axum::response`]。
//!
//! [`axum::response`]: https://docs.rs/axum/0.8/axum/response/index.html

use std::convert::Infallible;

use http::StatusCode;

use crate::body::Body;

// 内部模块
mod append_headers;    /// 追加响应头
mod into_response;       /// 转换为响应的特质
mod into_response_parts; /// 转换为响应部分的特质

// 公共导出
pub use self::{
    append_headers::AppendHeaders,          /// 追加响应头特质
    into_response::IntoResponse,             /// 转换为响应的特质（核心接口）
    into_response_parts::{IntoResponseParts, /// 转换为响应部分的特质
    ResponseParts,                       /// 响应部分（不含 body）
    TryIntoHeaderError,                   /// 尝试转换为头部时的错误
};

/// [`http::Response`] 的类型别名，body 类型默认为 [`Body`]，
/// 这是与 axum 一起使用的最常见 body 类型
///
/// 在 Spring Boot 中，这相当于 ResponseEntity 或 HttpServletResponse
pub type Response<T = Body> = http::Response<T>;

/// 基于 [`IntoResponse`] 的结果类型，使用 [`ErrorResponse`] 作为错误类型
///
/// 所有实现了 [`IntoResponse`] 的类型都可以转换为 [`ErrorResponse`]。
/// 这使其作为结合多个不同错误类型的通用错误类型很有用，
/// 这些错误类型都实现了 [`IntoResponse`]。
///
/// 在 Spring Boot 中，这相当于使用全局异常处理器（@ControllerAdvice）
/// 统一处理不同类型的异常
///
/// # 示例
///
//! ```rust
//! use axum::{
//!     response::{IntoResponse, Response},
//!     http::StatusCode,
//! };
//!
//! // 两个具有不同错误类型的可失败函数
//! fn try_something() -> Result<(), ErrorA> {
//!     // ...
//!     # unimplemented!()
//! }
//!
//! fn try_something_else() -> Result<(), ErrorB> {
//!     // ...
//!     # unimplemented!()
//! }
//!
//! // 每个错误类型实现 `IntoResponse`
//! struct ErrorA;
//!
//! impl IntoResponse for ErrorA {
//!     fn into_response(self) -> Response {
//!         // ...
//!         # unimplemented!()
//!     }
//! }
//!
//! enum ErrorB {
//!     SomethingWentWrong,
//! }
//!
//! impl IntoResponse for ErrorB {
//!     fn into_response(self) -> Response {
//!         // ...
//!         # unimplemented!()
//!     }
//! }
//!
//! // 我们可以使用 `axum::response::Result` 组合它们，仍然使用 `?`
//! async fn handler() -> axum::response::Result<&'static str> {
//!     // 错误会自动转换为 `ErrorResponse`
//!     try_something()?;
//!     try_something_else()?;
//!
//!     Ok("it worked!")
//! }
//! ```
///
//! ## 作为 `std::result::Result` 的替代品
///
//! 由于 `axum::response::Result` 具有默认错误类型，你只需要指定 `Ok` 类型：
///
//! ```
//! use axum::{
//!     response::{IntoResponse, Response, Result},
//!     http::StatusCode,
//! };
//!
//! // `Result<T>` 自动使用 `ErrorResponse` 作为错误类型
//! async fn handler() -> Result<&'static str> {
//!     try_something()?;
//!
//!     Ok("it worked!")
//! }
//!
//! // 即使你已导入 `axum::response::Result`，你仍然可以指定错误
//! fn try_something() -> Result<(), StatusCode> {
//!     // ...
//!     # unimplemented!()
//! }
//! ```
pub type Result<T, E = ErrorResponse> = std::result::Result<T, E>;

// 为 Result<T> 实现 IntoResponse
impl<T> IntoResponse for Result<T>
where
    T: IntoResponse,
{
    fn into_response(self) -> Response {
        match self {
            Ok(ok) => ok.into_response(),
            Err(err) => err.0,
        }
    }
}

/// 基于 [`IntoResponse`] 的错误类型
///
//! 更多详情请参阅 [`Result`]。
///
/// 在 Spring Boot 中，这相当于被 @ExceptionHandler 捕获的异常包装器
#[derive(Debug)]
#[must_use]
pub struct ErrorResponse(Response);

// 为任何实现 IntoResponse 的类型实现 From<ErrorResponse>
impl<T> From<T> for ErrorResponse
where
    T: IntoResponse,
{
    fn from(value: T) -> Self {
        Self(value.into_response())
    }
}

///! 停止状态码覆盖的响应部分
///
/// 此类型应该由实现 [`IntoResponseParts`] 或
//! [`IntoResponse`] 的类型使用，当它们无法产生通常预期的响应时
//! 并返回某种错误响应。
///
/// 它由具有 [`StatusCode`] 作为第一个元素的 [`IntoResponse`] 元组实现检查。
//! 考虑以下示例：
///
//! ```no_run
//! # use axum::Json;
//! # use http::StatusCode;
//! # #[derive(serde::Serialize)]
//! # struct CreatedResponse { }
//! fn my_handler(/* ... */) -> (StatusCode, Json<CreatedResponse>) {
//!     // 此响应类型的序列化可能会失败
//!     let response = CreatedResponse { /* ... */ };
//!     (StatusCode::CREATED, Json(response))
//! }
//! ```
//!
//! 当 `response` 序列化成功时，服务器响应 201 Created 状态码
//! （覆盖 `Json` 的默认 200 OK 状态码）和预期的 JSON 载荷。
//!
//! 然而，当 `response` 序列化失败时，`impl IntoResponse for Json`
//! 返回 500 Internal Server Error 状态码的响应，以及 `IntoResponseFailed`
//! 作为响应扩展，而 201 Created 覆盖被忽略。
///
//! 这是 axum 0.9 引入的行为。
//! 要强制状态码覆盖，即使内部 [`IntoResponseParts`] / [`IntoResponse`] 失败，
//! 使用 [`ForceStatusCode`]。
#[derive(Copy, Clone, Debug)]
pub struct IntoResponseFailed;

// 为 IntoResponseFailed 实现 IntoResponseParts
impl IntoResponseParts for IntoResponseFailed {
    type Error = Infallible;

    fn into_response_parts(self, mut res: ResponseParts) -> Result<ResponseParts, Self::Error> {
        res.extensions_mut().insert(self);
        Ok(res)
    }
}

//! 返回 `IntoResponseFailed` 作为整个响应没有太大意义。
//! 你可能至少将它与状态码结合。
//!
//! ```compile_fail
//! fn foo()
//! where
//!     axum_core::response::IntoResponseFailed: axum_core::response::IntoResponse,
//! {}
//! ```
#[allow(dead_code)]
fn into_response_failed_doesnt_impl_into_response() {}

//! 设置状态码，无论是否使用 [`IntoResponseFailed` 或不。
///
//! 更多详情请参阅 [`IntoResponseFailed`] 的文档。
#[derive(Debug, Copy, Clone, Default)]
pub struct ForceStatusCode(pub StatusCode);

// 为 ForceStatusCode 实现 IntoResponse
impl IntoResponse for ForceStatusCode {
    fn into_response(self) -> Response {
        let mut res = ().into_response();
        *res.status_mut() = self.0;
        res
    }
}

// 为 (ForceStatusCode, R) 实现 IntoResponse
impl<R> IntoResponse for (ForceStatusCode, R)
where
    R: IntoResponse,
{
    fn into_response(self) -> Response {
        let (ForceStatusCode(status), res) = self;
        let mut res = res.into_response();
        *res.status_mut() = status;
        res
    }
}
