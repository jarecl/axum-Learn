//! 响应模块 - 构建 HTTP 响应的类型和方法
//!
//! 在 Spring Boot 中，这相当于：
//! - ResponseEntity<T> - 泛型响应包装类
//! - @ResponseBody - 响应体注解
//! - ResponseEntity.ok()、badRequest() 等静态工厂方法
//!
//! 任何实现了 `IntoResponse` 特质的类型都可以从处理器返回。
//! axum 为常用类型提供了实现，如 String、&str、StatusCode、JSON 等。
//!
//! 更多详情请参阅：https://docs.rs/axum/latest/axum/response/index.html

#![doc = include_str!("../docs/response.md")]

use http::{header, HeaderValue, StatusCode};

mod redirect; /// HTTP 重定向响应（30x 状态码）

pub mod sse;    /// Server-Sent Events（SSE）响应

// 重新导出常用响应类型

// JSON 响应（当启用 json 特性时）
// 类似 Spring Boot 的 @ResponseBody + Jackson 序列化
#[doc(no_inline)]
#[cfg(feature = "json")]
pub use crate::Json;

// Form 表单响应（当启用 form 特性时）
#[cfg(feature = "form")]
#[doc(no_inline)]
pub use crate::form::Form;

// Extension 响应
// 类似 Spring Boot 的在 HandlerInterceptor 中设置的响应属性
#[doc(no_inline)]
pub use crate::Extension;

// 从 axum-core 重新导出核心响应特质和类型
#[doc(inline)]
pub use axum_core::response::{
    AppendHeaders,      /// 追加响应头的特质
    ErrorResponse,      /// 错误响应类型
    IntoResponse,       /// 将类型转换为响应的特质（核心接口）
    IntoResponseFailed, /// 表示 IntoResponse 实现失败的标记类型
    IntoResponseParts,  /// 从类型创建响应部分的特质
    Response,           /// HTTP 响应类型
    ResponseParts,      /// 响应部分（不含 body）
    Result,            /// 响应结果类型
};

// 重新导出 HTTP 重定向响应
#[doc(inline)]
pub use self::redirect::Redirect;

// 重新导出 SSE 响应
#[doc(inline)]
pub use sse::Sse;

/// HTML 响应包装器
///
/// 自动设置 `Content-Type: text/html; charset=utf-8`
///
/// 在 Spring Boot 中，这相当于：
/// - @GetMapping(produces = "text/html")
/// - 返回 String 并手动设置 Content-Type
///
/// 示例：
/// ```rust
/// use axum::response::Html;
///
/// async fn handler() -> Html<&'static str> {
///     Html("<h1>Hello, World!</h1>")
/// }
/// ```
#[derive(Clone, Copy, Debug)]
#[must_use]
pub struct Html<T>(pub T);

// 为 Html 实现 IntoResponse
impl<T> IntoResponse for Html<T>
where
    T: IntoResponse,
{
    fn into_response(self) -> Response {
        // 设置 text/html Content-Type 并将内部类型转换为响应
        (
            [(
                header::CONTENT_TYPE,
                HeaderValue::from_static(mime::TEXT_HTML_UTF_8.as_ref()),
            )],
            self.0,
        )
            .into_response()
    }
}

// 从 T 转换为 Html
impl<T> From<T> for Html<T> {
    fn from(inner: T) -> Self {
        Self(inner)
    }
}

/// 空响应，带有 204 No Content 状态码
///
/// 由于历史和实现原因，`()`（单元类型）的 `IntoResponse` 实现
/// 返回一个带有 200 [`StatusCode::OK`] 状态的空响应。
/// 如果你特别想要 204 [`StatusCode::NO_CONTENT`] 状态，你可以直接使用 `StatusCode` 类型，
/// 或使用此快捷结构体以实现自文档化。
///
/// 在 Spring Boot 中，这相当于返回 ResponseEntity.noContent().build()
///
/// 示例：
/// ```rust
/// use axum::{extract::Path, response::NoContent};
///
/// async fn delete_user(Path(user_id): Path<String>) -> Result<NoContent, String> {
///     // ...访问数据库...
/// # drop(user_id);
///     Ok(NoContent)
/// }
/// ```
#[derive(Debug, Clone, Copy)]
pub struct NoContent;

// 为 NoContent 实现 IntoResponse
impl IntoResponse for NoContent {
    fn into_response(self) -> Response {
        StatusCode::NO_CONTENT.into_response()
    }
}

#[cfg(test)]
mod tests {
    use crate::extract::Extension;
    use crate::test_helpers::*;
    use crate::Json;
    use crate::{routing::get, Router};
    use axum_core::response::ForceStatusCode;
    use axum_core::response::{
        IntoResponse, IntoResponseFailed, IntoResponseParts, Response, ResponseParts,
    };
    use http::HeaderMap;
    use http::{StatusCode, Uri};
    use std::collections::HashMap;

    // 只需要编译
    #[allow(dead_code)]
    fn impl_trait_result_works() {
        async fn impl_trait_ok() -> Result<impl IntoResponse, ()> {
            Ok(())
        }

        async fn impl_trait_err() -> Result<(), impl IntoResponse> {
            Err(())
        }

        async fn impl_trait_both(uri: Uri) -> Result<impl IntoResponse, impl IntoResponse> {
            if uri.path() == "/" {
                Ok(())
            } else {
                Err(())
            }
        }

        async fn impl_trait(uri: Uri) -> impl IntoResponse {
            if uri.path() == "/" {
                Ok(())
            } else {
                Err(())
            }
        }

        _ = Router::<()>::new()
            .route("/", get(impl_trait_ok))
            .route("/", get(impl_trait_err))
            .route("/", get(impl_trait_both))
            .route("/", get(impl_trait));
    }

    // 只需要编译
    #[allow(dead_code)]
    fn tuple_responses() {
        // 测试各种响应元组组合

        // 单个状态码
        async fn status() -> impl IntoResponse {
            StatusCode::OK
        }

        // 状态码 + HeaderMap
        async fn status_headermap() -> impl IntoResponse {
            (StatusCode::OK, HeaderMap::new())
        }

        // 状态码 + 头部数组
        async fn status_header_array() -> impl IntoResponse {
            (StatusCode::OK, [("content-type", "text/plain")])
        }

        // 状态码 + HeaderMap + body
        async fn status_headermap_body() -> impl IntoResponse {
            (StatusCode::OK, HeaderMap::new(), String::new())
        }

        // 状态码 + 头部数组 + body
        async fn status_header_array_body() -> impl IntoResponse {
            (
                StatusCode::OK,
                [("content-type", "text/plain")],
                String::new(),
            )
        }

        // 状态码 + HeaderMap + IntoResponse body
        async fn status_headermap_impl_into_response() -> impl IntoResponse {
            (StatusCode::OK, HeaderMap::new(), impl_into_response())
        }

        // 状态码 + 头部数组 + IntoResponse body
        async fn status_header_array_impl_into_response() -> impl IntoResponse {
            (
                StatusCode::OK,
                [("content-type", "text/plain")],
                impl_into_response(),
            )
        }

        fn impl_into_response() -> impl IntoResponse {}

        // 状态码 + 头部数组 + Extension + body
        async fn status_header_array_extension_body() -> impl IntoResponse {
            (
                StatusCode::OK,
                [("content-type", "text/plain")],
                Extension(1),
                String::new(),
            )
        }

        // 状态码 + 头部数组 + Extension + HeaderMap + body
        async fn status_header_array_extension_mixed_body() -> impl IntoResponse {
            (
                StatusCode::OK,
                [("content-type", "text/plain")],
                Extension(1),
                HeaderMap::new(),
                String::new(),
            )
        }

        //

        // 单个 HeaderMap
        async fn headermap() -> impl IntoResponse {
            HeaderMap::new()
        }

        // 头部数组
        async fn header_array() -> impl IntoResponse {
            [("content-type", "text/plain")]
        }

        // HeaderMap + body
        async fn headermap_body() -> impl IntoResponse {
            (HeaderMap::new(), String:​new())
        }

        // 头部数组 + body
        async fn header_array_body() -> impl IntoResponse {
            ([("content-type", "text/plain")], String::new())
        }

        // HeaderMap + IntoResponse body
        async fn headermap_impl_into_response() -> impl IntoResponse {
            (HeaderMap::new(), impl_into_response())
        }

        // 头部数组 + IntoResponse body
        async fn header_array_impl_into_response() -> impl IntoResponse {
            ([("content-type", "text/plain")], impl_into_response())
        }

        // 头部数组 + Extension + body
        async fn header_array_extension_body() -> impl IntoResponse {
            (
                [("content-type", "text/plain")],
                Extension(1),
                String::new(),
            )
        }

        // 头部数组 + Extension + HeaderMap + body
        async fn header_array_extension_mixed_body() -> impl IntoResponse {
            (
                [("content-type", "text/plain")],
                Extension(1),
                HeaderMap::new(),
                String::new(),
            )
        }

        _ = Router::<()>::new()
            .route("/", get(status))
            .route("/", get(status_headermap))
            .route("/", get(status_header_array))
            .route("/", get(status_headermap_body))
            .route("/", get(status_header_array_body))
            .route("/", get(status_headermap_impl_into_response))
            .route("/", get(status_header_array_impl_into_response))
            .route("/", get(status_header_array_extension_body))
            .route("/", get(status_header_array_extension_mixed_body))
            .route("/", get(headermap))
            .route("/", get(header_array))
            .route("/", get(headermap_body))
            .route("/", get(header_array_body))
            .route("/", get(headermap_impl_into_response))
            .route("/", get(header_array_impl_into_response))
            .route("/", get(header_array_extension_body))
            .route("/", get(header_array_extension_mixed_body));
    }

    // 测试：状态码元组不会覆盖错误状态码
    #[test]
    fn status_code_tuple_doesnt_override_error() {
        // 只有单个状态码的健全性检查
        assert_eq!(
            StatusCode::INTERNAL_SERVER_ERROR.into_response().status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(
            (StatusCode::INTERNAL_SERVER_ERROR,)
                .into_response()
                .status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );

        // 非 5xx 状态应该被更改
        assert_eq!(
            (StatusCode::SEE_OTHER, StatusCode::NO_CONTENT)
                .into_response()
                .status(),
            StatusCode::SEE_OTHER
        );
        let res = (
            StatusCode::SEE_OTHER,
            [("location", "foo")],
            StatusCode::NO_CONTENT,
        )
            .into_response();
        assert_eq!(res.status(), StatusCode::SEE_OTHER);
        assert_eq!(res.headers()["location"], "foo");

        // 5xx 状态码也会被更改
        assert_eq!(
            (StatusCode::SEE_OTHER, StatusCode::INTERNAL_SERVER_ERROR)
                .into_response()
                .status(),
            StatusCode::SEE_OTHER
        );
        let res = (
            StatusCode::SEE_OTHER,
            [("location", "foo")],
            StatusCode::INTERNAL_SERVER_ERROR,
        )
            .into_response();
        assert_eq!(res.status(), StatusCode::SEE_OTHER);
        assert_eq!(res.headers()["location"], "foo");

        // 如果使用 `IntoResponseFailed`，状态码不会更改
        assert_eq!(
            (
                StatusCode::SEE_OTHER,
                (IntoResponseFailed, StatusCode::INTERNAL_SERVER_ERROR)
            )
                .into_response()
                .status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
        let res = (
            StatusCode::SEE_OTHER,
            [("location", "foo")],
            (IntoResponseFailed, StatusCode::INTERNAL_SERVER_ERROR),
        )
            .into_response();
        assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert!(res.headers().get("location").is_none());

        // 来自内部响应的响应部分确实会运行
        let res = (
            // 带有状态覆盖
            StatusCode::SEE_OTHER,
            [("location", "foo")],
            (
                [("x-bar", "bar")],
                IntoResponseFailed,
                [("x-foo", "foo")],
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
        )
            .into_response();
        assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert!(res.headers().get("location").is_none());
        assert_eq!(res.headers()["x-foo"], "foo");
        assert_eq!(res.headers()["x-bar"], "bar");

        let res = (
            // 没有状态覆盖
            [("location", "foo")],
            (
                [("x-bar", "bar")],
                IntoResponseFailed,
                [("x-foo", "foo")],
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
        )
            .into_response();
        assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert!(res.headers().get("location").is_none());
        assert_eq!(res.headers()["x-foo"], "foo");
        assert_eq!(res.headers()["x-bar"], "bar");

        // (Parts, ...)
        let res = (
            Response::new(()).into_parts().0,
            [("location", "foo")],
            (
                [("x-bar", "bar")],
                IntoResponseFailed,
                [("x-foo", "foo")],
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
        )
            .into_response();
        assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert!(res.headers().get("location").is_none());
        assert_eq!(res.headers()["x-foo"], "foo");
        assert_eq!(res.headers()["x-bar"], "bar");

        // (Response<()>, ...)
        let res = (
            Response::new(()),
            [("location", "foo")],
            (
                [("x-bar", "bar")],
                IntoResponseFailed,
                [("x-foo", "foo")],
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
        )
            .into_response();
        assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert!(res.headers().get("location").is_none());
        assert_eq!(res.headers()["x-foo"], "foo");
        assert_eq!(res.headers()["x-bar"], "bar");
    }

    // 测试：IntoResponseParts 失败时设置扩展
    #[test]
    fn into_response_parts_failing_sets_extension() {
        struct Fail;

        impl IntoResponseParts for Fail {
            type Error = ();

            fn into_response_parts(
                self,
                _res: ResponseParts,
            ) -> Result<ResponseParts, Self::Error> {
                Err(())
            }
        }

        impl IntoResponse for Fail {
            fn into_response(self) -> Response {
                (self, ()).into_response()
            }
        }

        assert!(Fail
            .into_response()
            .extensions()
            .get::<IntoResponseFailed>()
            .is_some());

        assert!((StatusCode::INTERNAL_SERVER_ERROR, Fail, ())
            .into_response()
            .extensions()
            .get::<IntoResponseFailed>()
            .is_some());

        assert!((Response::new(()).into_parts().0, Fail, ())
            .into_response()
            .extensions()
            .get::<IntoResponseFailed>()
            .is_some());

        assert!((Response::new(()), Fail, ())
            .into_response()
            .extensions()
            .get::<IntoResponseFailed>()
            .is_some());
    }

    // 测试：在同一级别使用 into_response_failed 时不会覆盖状态码
    #[test]
    fn doenst_override_status_code_when_using_into_response_failed_at_same_level() {
        assert_eq!(
            (StatusCode::INTERNAL_SERVER_ERROR, IntoResponseFailed, ())
                .into_response()
                .status(),
            StatusCode::INTERNAL_SERVER_ERROR,
        );

        #[derive(Clone)]
        struct Thing;

        let res = (
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("x-foo", "foo")
                .extension(Thing)
                .body(())
                .unwrap()
                .into_parts()
                .0,
            IntoResponseFailed,
            (),
        )
            .into_response();
        assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(res.headers()["x-foo"], "foo");
        assert!(res.extensions().get::<Thing>().is_some());

        // 健全性检查
        assert_eq!(
            (IntoResponseFailed, ()).into_response().status(),
            StatusCode::OK,
        );
    }

    // 测试：强制覆盖状态码
    #[test]
    fn force_overriding_status_code() {
        assert_eq!(
            ForceStatusCode(StatusCode::IM_A_TEAPOT)
                .into_response()
                .status(),
            StatusCode::IM_A_TEAPOT
        );

        assert_eq!(
            (ForceStatusCode(StatusCode::IM_A_TEAPOT),)
                .into_response()
                .status(),
            StatusCode::IM_A_TEAPOT
        );

        assert_eq!(
            (ForceStatusCode(StatusCode::IM_A_TEAPOT), ())
                .into_response()
                .status(),
            StatusCode::IM_A_TEAPOT
        );

        assert_eq!(
            (
                ForceStatusCode(StatusCode::IM_A_TEAPOT),
                IntoResponseFailed,
                StatusCode::INTERNAL_SERVER_ERROR,
            )
                .into_response()
                .status(),
            StatusCode::IM_A_TEAPOT
        );
    }

    // 测试：状态码元组不会覆盖 JSON 错误状态码
    #[crate::test]
    async fn status_code_tuple_doesnt_override_error_json() {
        let app = Router::new()
            .route(
                "/",
                get(|| async {
                    let not_json_compatible = HashMap::from([(Vec::from([1, 2, 3]), 123)]);
                    (StatusCode::IM_A_TEAPOT, Json(not_json_compatible))
                }),
            )
            .route(
                "/two",
                get(|| async {
                    let not_json_compatible = HashMap::from([(Vec::from([1, 2, 3]), 123)]);
                    (
                        ForceStatusCode(StatusCode::IM_A_TEAPOT),
                        Json(not_json_compatible),
                    )
                }),
            );

        let client = TestClient::new(app);

        let res = client.get("/").await;
        assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let res = client.get("/two").await;
        assert_eq!(res.status(), StatusCode::IM_A_TEAPOT);
    }

    // 测试：NoContent 返回 204 状态码
    #[test]
    fn no_content() {
        assert_eq!(
            super::NoContent.into_response().status(),
            StatusCode::NO_CONTENT,
        )
    }
}
