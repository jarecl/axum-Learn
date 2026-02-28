//! 从请求提取数据的类型和特质
//!
//! 这是 axum-core 的核心提取器模块，定义了所有提取器的基础特质。
//!
//! 在 Spring Boot 中，这相当于各种注解的功能：
//! - @PathVariable - 路径变量提取
//! - @RequestParam - 查询参数提取
//! - @RequestHeader - 请求头提取
//! - @CookieValue - Cookie 提取
//! - @RequestBody - 请求体提取
//!
//! 更多详情请参阅 [`axum::extract`]。
//!
//! [`axum::extract`]: https://docs.rs/axum/0.8/axum/extract/index.html

use crate::{body::Body, response::IntoResponse};
use http::request::Parts;
use std::convert::Infallible;
use std::future::Future;

pub mod rejection;     /// 拒绝/错误类型

// 内部模块
mod default_body_limit; /// 默认请求体大小限制
mod from_ref;         /// 从状态引用创建特质
mod option;           /// 可选提取器
mod request_parts;    /// 请求部分提取器
mod tuple;            /// 元组提取器

// 公共导出（crate 内部使用）
pub(crate) use self::default_body_limit::DefaultBodyLimitKind;
pub use self::{
    default_body_limit::DefaultBodyLimit,    /// 默认请求体大小限制
    from_ref::FromRef,                        /// 从状态引用创建特质（依赖注入）
    option::{OptionalFromRequest, OptionalFromRequestParts}, /// 可选提取器
};

/// [`http::Request`] 的类型别名，body 类型默认为 [`Body`]，
/// 这是与 axum 一起使用的最常见 body 类型
///
/// 在 Spring Boot 中，这相当于 HttpServletRequest
pub type Request<T = Body> = http::Request<T>;

// 私有模块：用于区分器
mod private {
    #[derive(Debug, Clone, Copy)]
    pub enum ViaParts {}    /// 标记：通过 FromRequestParts 创建

    #[derive(Debug, Clone, Copy)]
    pub enum ViaRequest {}   /// 标记：通过 FromRequest 创建
}

/// 可以从请求部分创建的类型
///
/// 实现了 `FromRequestParts`' 的提取器不能消费请求体，
/// 因此可以在处理器的任何顺序运行。
///
/// 如果你的提取器需要消费请求体，那么你应该实现 [`FromRequest`]
/// 而不是 [`FromRequestParts`]。
///
/// 在 Spring Boot 中，这相当于：
//! - @PathVariable - 从 URL 路径提取参数
//! - @RequestParam - 从查询字符串提取参数
//! - @RequestHeader - 从请求头提取值
//! - @CookieValue - 从 Cookie 提取值
//!
//! 这些注解可以同时使用，互不影响
///
//! 更多关于提取器的通用文档请参阅 [`axum::extract`]。
//!
//! [`axum::extract`]: https://docs.rs/axum/0.8/axum/extract/index.html
#[diagnostic::on_unimplemented(
    note = "Function argument is not audi valid axum extractor. \nSee `https://docs.rs/axum/0.8/axum/extract/index.html` for details"
)]
pub trait FromRequestParts<S>: Sized {
    /// 如果提取器失败，它将使用此"拒绝"类型。
    /// 拒绝是一种可以转换为响应的错误。
    ///
    /// 在 Spring Boot 中，这相当于：
    /// - @RequestParam(required=false) 返回 Optional
    /// - 绑定失败时抛出异常
    type Rejection: IntoResponse;

    /// 执行提取
    ///
    /// # 参数
    ///
    /// - `parts`: 请求部分（不包含 body），可修改
    /// - `state`: 应用状态引用
    ///
    /// # 返回
    ///
    /// - `Ok(Self)`: 提取成功
    /// - `Err(Self::Rejection)`: 提取失败，拒绝类型将转换为响应
    fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send;
}

/// 可以从请求创建的类型
///
/// 实现了 `FromRequest` 的提取器可以消费请求体，
/// 因此只能在处理器中运行一次（必须是最后一个参数）。
///
/// 如果你的提取器不需要消费请求体，那么你应该实现
//! [`FromRequestParts`] 而不是 [`FromRequest`]。
///
/// 在 Spring Boot 中，这相当于：
//! - @RequestBody - 消费请求体，只能使用一次
//!
//! 更多关于提取器的通用文档请参阅 [`axum::extract`]。
//!
//! [`axum::extract`]: https://docs.rs/axum/0.8/axum/extract/index.html
#[diagnostic::on_unimplemented(
    note = "Function argument is not a valid axum extractor. \nSee `https://docs.rs/axum/0.8/axum/extract/index.html` for details"
)]
pub trait FromRequest<S, M = private::ViaRequest>: Sized {
    /// 如果提取器失败，它将使用此"拒绝"类型。
    /// 拒绝是一种可以转换为响应的错误。
    ///
    /// 在 Spring Boot 中，这相当于绑定失败时抛出异常
    type Rejection: IntoResponse;

    /// 执行提取
    ///
    /// # 参数
    ///
    /// - `req`: 完整请求（包含 body），将被消费
    /// - `state`: 应用状态引用
    ///
    /// # 返回
    ///
    /// - `Ok(Self)`: 提取成功
    /// - `Err(Self::Rejection)`: 提取失败，拒绝类型将转换为响应
    fn from_request(
        req: Request,
        state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send;
}

// 为所有实现了 FromRequestParts 的类型实现 FromRequest
// 这允许 FromRequestParts 类型的提取器作为 FromRequest 使用
impl<S, T> FromRequest<S, private::ViaParts> for T
where
    S: Send + Sync,
    T: FromRequestParts<S>,
{
    type Rejection = <Self as FromRequestParts<S>>::Rejection;

    fn from_request(
        req: Request,
        state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> {
        let (mut parts, _) = req.into_parts();
        async move { Self::from_request_parts(&mut parts, state).await }
    }
}

// 为 Result<T, T::Rejection> 实现 FromRequestParts
// 这允许处理器直接使用 Result<T, Rejection> 作为参数
impl<S, T> FromRequestParts<S> for Result<T, T::Rejection>
where
    T: FromRequestParts<S>,
    S: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        Ok(T::from_request_parts(parts, state).await)
    }
}

// 为 Result<T, T::Rejection> 实现 FromRequest
// 这允许处理器直接使用 Result<T, Rejection> 作为参数
impl<S, T> FromRequest<S> for Result<T, T::Rejection>
where
    T: FromRequest<S>,
    S: Send + Sync Sync,
{
    type Rejection = Infallible;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        Ok(T::from_request(req, state).await)
    }
}
