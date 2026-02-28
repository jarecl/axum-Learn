//! JSON 提取器和响应包装器
//!
//! 在 Spring Boot 中，这相当于：
//! - @RequestBody + Jackson - JSON 请求体反序列化
//! - @ResponseBody + Jackson - JSON 响应体序列化
//! - MediaType.APPLICATION_JSON_VALUE - 内容类型设置

use crate::extract::Request;
use crate::extract::{rejection::*, FromRequest};
use axum_core::extract::OptionalFromRequest;
use axum_core::response::{IntoResponse, IntoResponseFailed, Response};
use bytes::{BufMut, Bytes, BytesMut};
use http::{
    header::{self, HeaderMap, HeaderValue},
    StatusCode,
};
use serde_core::{de::DeserializeOwned, Serialize};

/// JSON 提取器 / 响应包装器
///
/// ## 作为提取器使用
///
/// 当作为提取器使用时，它可以将请求体反序列化为实现 [`serde::de::DeserializeOwned`] 的某种类型。
/// 如果以下情况，请求将被拒绝（并返回 [`JsonRejection`]）：
///
//! - 请求没有 `Content-Type: application/json` (或类似）header
//! - body 不包含语法上有效的 JSON
//! - body 包含语法上有效的 JSON，但无法反序列化为目标类型
//! - 缓冲请求体失败
//!
/// ⚠️ 由于解析 JSON 需要消费请求体，`Json` 提取器必须是 *最后一个*（如果有多个提取器）。
/// 参见 ["提取器的顺序"][order-of-extractors]
///
/// 在 Spring Boot 中，这相当于：
//! ```java
//! @PostMapping("/users")
//! public ResponseEntity<User> createUser(@RequestBody CreateUserDto dto) {
//!     // dto 已经从 JSON 反序列化
//!     User user = userService.create(dto);
//!     return ResponseEntity.ok(user);
//! }
//! ```
//!
//! [order-of-extractors]: crate::extract#the-order-of-extractors
//!
//! 更多信息请参阅 [`JsonRejection`]。
//!
//! ## 提取器示例
//!
//! ```rust,no_run
//! use axum::{
//!     extract,
//!     routing::post,
//!     Router,
//! };
//! use serde::Deserialize;
//!
//! // 对应 Spring Boot 的 DTO 类
//! #[derive(Deserialize)]
//! struct CreateUser {
//!     email: String,
//!     password: String,
//! }
//!
//! async fn create_user(extract::Json(payload): extract::Json<CreateUser>) {
//!     // payload 是 `CreateUser` 类型
//! }
//!
//! let app = Router::new().route("/users", post(create_user));
//! # let _: Router = app;
//! ```
//!
/// ## 作为响应使用
///
/// 当作为响应使用时，它可以将任何实现 [`serde::Serialize`] 的类型序列化为 `JSON`，
//! 并自动设置 `Content-Type: application/json` header。
//!
//! 在 Spring Boot 中，这相当于：
//! ```java
//! @GetMapping("/users/{id}")
//! @ResponseBody
//! public User getUser(@PathVariable Long id) {
//!     User user = userService.findById(id);
//!     return user;  // Jackson 自动序列化为 JSON
//! }
//! ```
//!
/// 如果 [`Serialize`] 实现决定失败或使用了非字符串键的映射，
//! 将发出 500 响应，其 body 是 UTF-8 中的错误消息。
//!
//! ## 响应示例
//!
//! ```rust,no_run
//! use axum::{
//!     extract::Path,
//!     routing::get,
//!     Router,
//!     Json,
//! };
//! use serde::Serialize;
//! use uuid::Uuid;
//!
//! // 对应 Spring Boot 的实体类
//! #[derive(Serialize)]
//! struct User {
//!     id: Uuid,
//!     username: String,
//! }
//!
//! async fn get_user(Path(user_id) : Path<Uuid>) -> Json<User> {
//!     let user = find_user(user_id).await;
//!     Json(user)  // 自动序列化为 JSON 并设置 Content-Type
//! }
//!
//! async fn find_user(user_id: Uuid) -> User {
//!     // ...
//!     # unimplemented!()
//! }
//!
//! let app = Router::new().route("/users/{id}", get(get_user));
//! # let _: Router = app;
//! ```
#[derive(Debug, Clone, Copy, Default)]
#[cfg_attr(docsrs, doc(cfg(feature = "json"))]
#[must_use]
pub struct Json<T>(pub T);

// 为 Json 实现 FromRequest（作为提取器）
// 这使得 Json<T> 可以用作处理器参数（消费请求体）
impl<T, S> FromRequest<S> for Json<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = JsonRejection;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        // 检查 Content-Type 是否为 JSON
        if !json_content_type(req.headers()) {
            return Err(MissingJsonContentType.into());
        }

        // 从请求体中提取字节
        let bytes = Bytes::from_request(req, state).await?;
        Self::from_bytes(&bytes)
    }
}

// 为 Json 实现 OptionalFromRequest
// 这允许 Option<Json<T>> 作为可选参数
impl<T, S> OptionalFromRequest<S> for Json<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = JsonRejection;

    async fn from_request(req: Request, state: &S) -> Result<Option<Self>, Self::Rejection> {
        let headers = req.headers();
        // 如果有 Content-Type header
        if headers.get(header::CONTENT_TYPE).is_some() {
            if json_content_type(headers) {
                let bytes = Bytes::from_request(req, state).await?;
                Ok(Some(Self::from_bytes(&bytes)?))
            } else {
                Err(MissingJsonContentType.into())
            }
        } else {
            // 没有 Content-Type，返回 None
            Ok(None)
        }
    }
}

// 检查 Content-Type 是否为 JSON
fn json_content_type(headers: &HeaderMap) -> bool {
    headers
        .get(header::CONTENT_TYPE)
        .and_then(|content_type| content_type.to_str().ok())
        .and_then(|content_type| content_type.parse::<mime::Mime>().ok())
        .is_some_and(|mime| {
            mime.type_() == "application"
                && (mime.subtype() == "json" || mime.suffix().is_some_and(|name| name == "json"))
        })
}

// 为 Json 实现 Deref
axum_core::__impl_deref!(Json);

// 为 Json 实现 From<T>
impl<T> From<T> for Json<T> {
    fn from(inner: T) -> Self {
        Self(inner)
    }
}

// 当 T 实现 DeserializeOwned 时为 Json<T> 实现相关方法
impl<T> Json<T>
where
    T: DeserializeOwned,
{
    /// 从字节切片构造 `Json<T>`。大多数用户应该更喜欢使用 `FromRequest` 实现，
    /// 但特殊情况可能需要首先将 `Request` 提取到 `Bytes`，然后可选地构造 `Json<T>`。
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, JsonRejection> {
        // 提取到单独的函数中，以便只为所有 T 编译一次
        fn make_rejection(err: serde_path_to_error::Error<serde_json::Error>) -> JsonRejection {
            match err.inner().classify() {
                serde_json::error::Category::Data => JsonDataError::from_err(err).into(),
                serde_json::error::Category::Syntax | serde_json::error::Category::Eof => {
                    JsonSyntaxError::from_err(err).into()
                }
                serde_json::error::Category::Io => {
                    if cfg!(debug_assertions) {
                        // 我们不使用 `serde_json::from_reader` 而始终首先缓冲 body，
                        // 所以我们不应该遇到任何 IO 错误
                        unreachable!()
                    } else {
                        JsonSyntaxError::from_err(err).into()
                    }
                }
            }
        }

        let mut deserializer = serde_json::Deserializer::from_slice(bytes);

        serde_path_to_error::deserialize(&mut deserializer)
            .map_err(make_rejection)
            .and_then(|value| {
                deserializer
                    .end()
                    .map(|()| Self(value))
                    .map_err(|err| JsonSyntaxError::from_err(err).into())
            })
    }
}

// 当 T 实现 Serialize 时为 Json<T> 实现 IntoResponse（作为响应）
impl<T> IntoResponse for Json<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        // 提取到单独的函数中，以便只为所有 T 编译一次
        fn make_response(buf: BytesMut, ser_result: serde_json::Result<()>) -> Response {
            match ser_result {
                Ok(()) => (
                    [
                        (
                            header::CONTENT_TYPE,
                            HeaderValue::from_static(mime::APPLICATION_JSON.as_ref()),
                        )
                    ],
                    buf.freeze(),
                )
                    .into_response(),
                Err(err) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    [
                        (
                            header::CONTENT_TYPE,
                            HeaderValue::from_static(mime::TEXT_PLAIN_UTF_8.as_ref()),
                        )
                    ],
                    IntoResponseFailed,
                    err.to_string(),
                )
                    .into_response(),
            }
        }

        // 使用 128 字节的初始容量，像 serde_json::to_vec
        // https://docs.rs/serde_json/1.0.82/src/serde_json/ser.rs.html#2189
        let mut buf = BytesMut::with_capacity(128).writer();
        let res = serde_json::to_writer(&mut buf, &self.0);
        make_response(buf.into_inner().res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{routing::post, test_helpers::*, Router};
    use serde::Deserialize;
    use serde_json::{json, Value};

    #[crate::test]
    async fn deserialize_body() {
        #[derive(Debug, Deserialize)]
        struct Input {
            foo: String,
        }

        let app = Router::new().route("/", post(|input: Json<Input>| async { input.0.foo }));
        let client = TestClient::new(app);
        let res = client.post("/").json(&json!({ "foo": "bar" })).await;
        let body = res.text().await;

        assert_eq!(body, "bar");
    }

    #[crate::test]
    async fn consume_body_to_json_requires_json_content_type() {
        #[derive(Debug, Deserialize)]
        struct Input {
            foo: String,
        }

        let app = Router::new().route("/", post(|input: Json<Input>| async { input.0.foo }));
        let client = TestClient::new(app);
        let res = client.post("/").body(r#"{ "foo": "bar" }"#).await;
        let status = res.status();

        assert_eq!(status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }

    #[crate::test]
    async fn json_content_types() {
        async fn valid_json_content_type(content_type: &str) -> bool {
            println!("testing {content_type:?}");

            let app = Router::new().route("/", post(|Json(_): Json<Value>| async {}));
            let res = TestClient::new(app)
                .post("/")
                .header("content-type", content_type)
                .body("{}")
                .await;

            res.status() == StatusCode::OK
        }

        // 测试各种 JSON 内容类型
        assert!(valid_json_content_type("application/json").await);
        assert!(valid_json_content_type("application/json; charset=utf-8").await);
        assert!(valid_json_content_type("application/json;charset=utf-8").await);
        assert!(valid_json_content_type("application/cloudevents+json").await);
        assert!(!valid_json_content_type("text/json").await);
    }

    #[crate::test]
    async fn invalid_json_syntax() {
        let app = Router::new().route("/", post(|_: Json<serde_json::Value>| async {}));
        let client = TestClient::new(app);
        let res = client
            .post("/")
            .body("{")
            .header("content-type", "application/json")
            .await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[crate::test]
    async fn extra_chars_after_valid_json_syntax() {
        #[derive(Debug, Deserialize)]
        struct Input {
            foo: String,
        }

        let app = Router::new().route("/", post(|input: Json<Input>| async { input.0.foo }));
        let client = TestClient::new(app);
        let res = client
            .post("/")
            .body(r#"{ "foo": "bar" } baz "#)
            .header("content-type", "application/json")
            .await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        let body_text = res.text().await;
        assert_eq!(
            body_text,
            "Failed to parse request body as JSON: trailing characters at line 1 column 18"
        );
    }

    #[derive(Deserialize)]
    struct Foo {
        #[allow(dead_code)]
        a: i32,
        #[allow(dead_code)]
        b: Vec<Bar>,
    }

    #[derive(Deserialize)]
    struct Bar {
        #[allow(dead_code)]
        x: i32,
        #[allow(dead_code)]
        y: i32,
    }

    #[crate::test]
    async fn invalid_json_data() {
        let app = Router::new().route("/", post(|_: Json<Foo>| async {}));
        let client = TestClient::new(app);
        let res = client
            .post("/")
            .body("{\"a\": 1, \"b\": [{\"x\": 2}]}")
            .header("content-type", "application/json")
            .await;

        assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let body_text = res.text().await;
        assert_eq!(
            body_text,
            "Failed to deserialize JSON body into target type: b[0]: missing field `y` at line 1 column 23"
        );
    }
}
