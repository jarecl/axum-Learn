//! 路径参数提取器模块
//!
//! 在 Spring Boot 中，这相当于 @PathVariable 注解
//!
//! 例如：
//! - `/users/{id}` 中的 `{id}` 被提取
//! - `/posts/{year}/{month}/{day}` 中的多个路径参数被提取

use crate::extract::request_parts::{FromRequestParts, FromRequestPartsState};
use http::request::Parts;
use std::convert::Infallible;

// 重新导出 RawPathParams
pubra use self::raw_path_params::RawPathParams;

mod raw_path_params;  /// 原始路径参数

/// 提取器，从 URL 路径提取并反序列化数据
///
/// 路径参数中的数据将反序列化为内部类型 `T`。
//!
//! 在 Spring Boot 中，这相当于：
//! - `@PathVariable Long id` - 从 URL 路径提取并转换类型
//! - `@PathVariable User user` - 从 URL 路径提取并反序列化为对象
//!
//! # 示例
//!
//! ## 提取路径参数
//!
//! ```rust
//! use axum::{extract::Path, routing::get, Router};
//! use serde::Deserialize;
//!
//! // Spring Boot: @GetMapping("/users/{id}")
//! // public ResponseEntity<User> getUser(@PathVariable Long id) { ... }
//! #[derive(Deserialize)]
//! struct Params {
//!     id: String,
//! }
//!
//! // 提取路径参数并反序列化为 Params 结构体
//! async fn users_show(Path(Params): Path<Params>) -> String {
//!     format!("users id: {}", params.id)
//! }
//!
//! let app = Router::new().route("/users/:id", get(users_show));
//! # let _: axum::Router = app;
//! ```
//!
//! ## 多个路径参数
//!
//! ```rust
//! use axum::{extract::Path, routing::get, Router};
//! use serde::Deserialize;
//!
//! // Spring Boot: @GetMapping("/users/{year}/{month}/{day}")
//! // public ResponseEntity<User> getUserByDate(
//! //     @PathVariable Integer year,
//! //     @PathVariable Integer month,
//! //     @PathVariable Integer day
//! // ) { ... }
//! #[derive(Deserialize)]
//! struct Params {
//!     year: u32,
//!     month: u32,
//!     day: u32,
//! }
//!
//! async fn user_by_date(Path(Params): Path<Params>) -> String {
//!     format!(
//!         "year: {}, month: {}, day: {}",
//!         params.year, params.month, params.day
//!     )
//! }
//!
//! let app = Router::new().route("/users/:year/:month/:day", get(user_by_date));
//! # let _: axum::Router = app;
//! ```
//!
//! ## 获取原始路径参数
//!
//! 如果你不希望反序列化参数，可以使用 `RawPathParams`：
//!
//! ```rust
//! use axum::{extract::Path, routing::get, Router};
//!
//! async fn users_show_raw(Path(params): Path<RawPathParams>) -> String {
//!     // params 是 Vec<(String, String)>
//!     // 第一个元素是键，第二个元素是值
//!     params
//!         .into_iter()
//!         .map(|(key, value)| format!("{} => {}", key, value))
//!         .collect::<Vec<_>>()
//!         .join(", ")
//! }
//!
//! let app = Router::new().route("/users/:id", get(users_show_raw));
//! # let _: axum::Router = app;
//! ```
//!
//! # 注意
//!
//! `Path` 消费请求路径，因此如果处理器有多个提取器，`Path` 必须在其他提取器之前。
//!
//! 更多信息请参阅 ["the order of extractors"][order-of-extractors]
//!
//! [order-of-extractors]: crate::extract#the-order-of-extractors
#[derive(Debug, Clone, Copy)]
pub struct Path<T>(pub T);

// 为 Path 实现 FromRequestParts
// 这使得 Path 可以用作处理器参数
impl<T, S> FromRequestParts<S> for Path<T>
where
    T: serde::de::DeserializeOwned + Send,
{
    type Rejection = PathRejection;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        // 从请求扩展中获取路径参数
        let params = PathData::from_request_parts(parts, state).await?;

        // 反序列化路径参数为目标类型 T
        T::deserialize(serde::de::value::MapDeserializer::new(
            params.params.clone(),
        ))
        .map(Self)
        .map_err(PathDeserError::into_err)
        .map_err(PathRejection::from_err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        extract::Request,
        routing::{get, post, delete},
        test_helpers::TestClient,
        Router,
    };
    use http::StatusCode;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Params {
        id: String,
    }

    #[crate::test]
    async fn test_path() {
        async fn handler(Path(params): Path<Params>) -> String {
            params.id.clone()
        }

        let app = Router::new().route("/users/:id", get(handler));
        let client = TestClient::new(app);

        let res = client.get("/users/123").await;
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res(res.text().await), "123");
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct MultiParams {
        id: String,
        sub_id: String,
    }

    #[crate::test]
    async fn test_path_nested() {
        async fn handler(Path(params): Path<MultiParams>) -> String {
            format!("{} - {}", params.id, params.sub_id)
        }

        let app = Router::new().route("/users/:id/posts/:sub_id", get(handler));
        let client = TestClient::new(app);

        let res = client.get("/users/123/posts/456").await;
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.text().await, "123 - 456");
    }

    #[crate::test]
    async fn test_path_missing_param() {
        async fn handler(Path(Params { id }): Path<Params>) -> String {
            id
        }

        let app = Router::new().route("/users/:id", get(handler));
        let client = TestClient::new(app);

        let res = client.get("/users/").await;
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct ParamsNumbers {
        year: u32,
        month: u32,
        day: u32,
    }

    #[crate::test]
    async fn test_path_numbers() {
        async fn handler(Path(params): Path<ParamsNumbers>) -> String {
            format!(
                "year: {}, month: {}, day: {}",
                params.year, params.month, params.day
            )
        }

        let app = Router::new().route("/date/:year/:month/:day", get(handler));
        let client = TestClient::new(app);

        let res = client.get("/date/2024/12/25").await;
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.text().await, "year: 2024, month: 12, day: 25");
    }

    #[crate::test]
    async fn test_path_in_post() {
        async fn handler(Path(Params { id }): Path<Params>) -> String {
            id
        }

        let app = Router::new().route("/users/:id", post(handler));
        let client = TestClient::new(app);

        let res = client.post("/users/123").await;
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.text().await, "123");
    }

    #[crate::test]
    async fn test_path_in_delete() {
        async fn handler(Path(Params { id }): Path<Params>) -> String {
            id
        }

        let app = Router::new().route("/users/:id", delete(handler));
        let client = TestClient::new(app);

        let res = client.delete("/users/123").await;
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.text().await, "123");
    }
}
