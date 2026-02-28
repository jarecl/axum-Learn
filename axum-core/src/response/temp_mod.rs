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

