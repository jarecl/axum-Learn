//! 中间件模块 - 用于编写中间件的实用工具
//!
//! 在 Spring Boot 中，这相当于：
//! - Filter - Web 过滤器（用于请求/响应处理）
//! - HandlerInterceptor - 处理器拦截器（用于切面编程）
//! - OncePerRequestFilter - 每个请求只执行一次的过滤器
//!
//! ## 中间件类型
//!
//! axum 提供了几种不同类型的中间件：
//!
//! 1. **from_fn** - 从函数创建中间件
//! 2. **from_extractor** - 从提取器创建中间件
//! 3. **map_request** - 映射/修改请求
//! 4. **map_response** - 映射/修改响应
//!
//! 更多详情请参阅：https://docs.rs/axum/latest/axum/middleware/index.html

#![doc = include_str!("../docs/middleware.md")]

// 内部模块
mod from_extractor; /// 从提取器创建中间件
mod from_fn;        /// 从函数创建中间件
mod map_request;     /// 映射请求中间件
mod map_response;    /// 映射响应中间件
mod response_axum_body; /// 响应 body 处理

// 公共导出

// from_extractor 相关类型
// 类似 Spring Boot 的基于注解的过滤器
pub use self::from_extractor::{
    from_extractor,               /// 从提取器创建中间件
    from_extractor_with_state,    /// 从提取器和状态创建中间件
    FromExtractor,                /// 从提取器中间件类型
    FromExtractorLayer,            /// 从提取器层类型
};

// from_fn 相关类型
// 类似 Spring Boot 的 HandlerInterceptor 或 Filter
pub use self::from_fn::{
    from_fn,                /// 从函数创建中间件
    from_fn_with_state,       /// 从函数和状态创建中间件
    FromFn,                 /// 从函数中间件类型
    FromFnLayer,             /// 从函数层类型
    Next,                   /// 下一个处理器（类似 FilterChain.doFilter()）
};

// map_request 相关类型
// 类似 Spring Boot 的在 Filter.doFilter() 中修改 request
pub use self::map_request::{
    map_request,              /// 映射请求
    map_request_with_state,   /// 映射请求（带状态）
    IntoMapRequestResult,      /// 映射请求结果特质
    MapRequest,               /// 映射请求中间件
    MapRequestLayer,          /// 映射请求层
};

// map_response 相关类型
// 类似 Spring Boot 的在 postHandle 中修改 response
pub use self::map_response::{
    map_response,              /// 映射响应
    map_response_with_state,   /// 映射响应（带状态）
    MapResponse,               /// 映射响应中间件
    MapResponseLayer,          /// 映射响应层
};

// response_axum_body 相关类型
pub use self::response_axum_body::{
    ResponseAxumBody,         /// 响应 body 类型
    ResponseAxumBodyFuture,    /// 响应 body Future
    ResponseAxumBodyLayer,     /// 响应 body 层
};

// AddExtension 用于向请求添加扩展（类似设置 request attribute）
pub use crate::extension::AddExtension;

// Future 类型模块
pub mod future {
    //! Future 类型

    pub use super::from_extractor::ResponseFuture as FromExtractorResponseFuture;
    pub use super::from_fn::ResponseFuture as FromFnResponseFuture;
    pub use super::map_request::ResponseFuture as MapRequestResponseFuture;
    pub use super::map_response::ResponseFuture as MapResponseResponseFuture;
}
