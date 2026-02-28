从请求中提取数据的类型和 trait。

# 简介

处理器函数是一个接受零个或多个"提取器"作为参数的 async 函数。提取器是实现 [`FromRequest`] 或 [`FromRequestParts`] 的类型。

例如，[`Json`] 是一个提取器，它消费请求体并将其反序列化为 JSON 到某个目标类型：

```rust,no_run
use axum::{
    extract::Json,
    routing::post,
    handler::Handler,
    Router,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct CreateUser {
    email: String,
    password: String,
}

async fn create_user(Json(payload): Json<CreateUser>) {
    // ...
}

let app = Router::new().route("/users", post(create_user));
# let _: Router = app;
```

# 常用提取器

一些常用的提取器包括：

```rust,no_run
use axum::{
    extract::{Request, Json, Path, Extension, Query},
    routing::post,
    http::header::HeaderMap,
    body::{Bytes, Body},
    Router,
};
use serde_json::Value;
use std::collections::HashMap;

// `Path` 给你路径参数并反序列化它们。查看其文档了解更多细节
async fn path(Path(user_id): Path<u32>) {}

// `Query` 给你查询参数并反序列化它们。
async fn query(Query(params): Query<HashMap<String, String>>) {}

// `HeaderMap` 给你所有的头部
async fn headers(headers: HeaderMap) {}

// `String` 消费请求体并确保它是有效的 utf-8
async fn string(body: String) {}

// `Bytes` 给你原始请求体
async fn bytes(body: Bytes) {}

// 我们已经见过用于将请求体解析为 json 的 `Json`
async fn json(Json(payload): Json<Value>) {}

// `Request` 给你整个请求以获得最大控制
async fn request(request: Request) {}

// `Extension` 从"请求扩展"中提取数据
// 这常用于与处理器共享状态
async fn extension(Extension(state): Extension<State>) {}

#[derive(Clone)]
struct State { /* ... */ }

let app = Router::new()
    .route("/path/{user_id}", post(path))
    .route("/query", post(query))
    .route("/string", post(string))
    .route("/bytes", post(bytes))
    .route("/json", post(json))
    .route("/request", post(request))
    .route("/extension", post(extension));
# let _: Router = app;
```

# 应用多个提取器

你也可以应用多个提取器：

```rust,no_run
use axum::{
    extract::{Path, Query},
    routing::get,
    Router,
};
use uuid::Uuid;
use serde::Deserialize;

let app = Router::new().route("/users/{id}/things", get(get_user_things));

#[derive(Deserialize)]
struct Pagination {
    page: usize,
    per_page: usize,
}

async fn get_user_things(
    Path(user_id): Path<Uuid>,
    Query(pagination): Query<Pagination>,
) {
    // ...
}
# let _: Router = app;
```

# 提取器的顺序

提取器总是按照函数参数的顺序运行，即从左到右。

请求体是一个只能消费一次的异步流。因此你只能有一个消费请求体的提取器。axum 通过要求此类提取器成为你的处理器接受的_最后_一个参数来强制执行这一点。

例如

```rust
use axum::{extract::State, http::{Method, HeaderMap}};
#
# #[derive(Clone)]
# struct AppState {
# }

async fn handler(
    // `Method` 和 `HeaderMap` 不消费请求体，所以它们可以
    // 放在参数列表的任何位置（但在 `body` 之前）
    method: Method,
    headers: HeaderMap,
    // `State` 也是一个提取器，所以它需要在 `body` 之前
    State(state): State<AppState>,
    // `String` 消费请求体，因此必须是最后一个提取器
    body: String,
) {
    // ...
}
#
# let _: axum::routing::MethodRouter<AppState> = axum::routing::get(handler);
```

如果 `String` 不是最后一个提取器，我们会得到编译错误：

```rust,compile_fail
use axum::http::Method;

async fn handler(
    // 这不起作用，因为 `String` 必须是最后一个参数
    body: String,
    method: Method,
) {
    // ...
}
#
# let _: axum::routing::MethodRouter = axum::routing::get(handler);
```

这也意味着你不能消费两次请求体：

```rust,compile_fail
use axum::Json;
use serde::Deserialize;

#[derive(Deserialize)]
struct Payload {}

async fn handler(
    // `String` 和 `Json` 都消费请求体
    // 所以它们不能同时使用
    string_body: String,
    json_body: Json<Payload>,
) {
    // ...
}
#
# let _: axum::routing::MethodRouter = axum::routing::get(handler);
```

axum 通过要求最后一个提取器实现 [`FromRequest`] 而其他所有提取器实现 [`FromRequestParts`] 来强制执行这一点。

# 处理提取器拒绝

如果你想在特定处理器内处理提取器失败的情况，可以将其包装在 `Result` 中，错误为提取器的拒绝类型：

```rust,no_run
use axum::{
    extract::{Json, rejection::JsonRejection},
    routing::post,
    Router,
};
use serde_json::Value;

async fn create_user(payload: Result<Json<Value>, JsonRejection>) {
    match payload {
        Ok(payload) => {
            // 我们获得了有效的 JSON 负载
        }
        Err(JsonRejection::MissingJsonContentType(_)) => {
            // 请求没有 `Content-Type: application/json`
            // 头部
        }
        Err(JsonRejection::JsonDataError(_)) => {
            // 无法将请求体反序列化到目标类型
        }
        Err(JsonRejection::JsonSyntaxError(_)) => {
            // 请求体中有语法错误
        }
        Err(JsonRejection::BytesRejection(_)) => {
            // 提取请求体失败
        }
        Err(_) => {
            // `JsonRejection` 被标记为 `#[non_exhaustive]`，所以 match 必须
            // 包含一个 catch-all 情况。
        }
    }
}

let app = Router::new().route("/users", post(create_user));
# let _: Router = app;
```

# 可选提取器

一些提取器除了实现 [`FromRequestParts`] 之外还实现 [`OptionalFromRequestParts`]，或除了实现 [`FromRequest`] 之外还实现 [`OptionalFromRequest`]。

这些提取器可以在 `Option` 内使用。这取决于特定的 `OptionalFromRequestParts` 或 `OptionalFromRequest` 实现。例如，对于 axum-extra 中的 `TypedHeader`，如果你尝试提取的头部不在请求中，你会得到 `None`，但如果头部存在且解析失败，请求将被拒绝。

```rust,no_run
use axum::{routing::post, Router};
use axum_extra::{headers::UserAgent, TypedHeader};
use serde_json::Value;

async fn foo(user_agent: Option<TypedHeader<UserAgent>>) {
    if let Some(TypedHeader(user_agent)) = user_agent {
        // 客户端发送了用户代理
    } else {
        // 没有用户代理头部
    }
}

let app = Router::new().route("/foo", post(foo));
# let _: Router = app;
```

# 自定义提取器响应

如果提取器失败，它将返回带有错误的响应，而不会调用你的处理器。要自定义错误响应，你有两个选择：

1. 使用 `Result<T, T::Rejection>` 作为你的提取器，如 ["处理提取器拒绝"](#handling-extractor-rejections) 所示。
   如果你在单个处理器中仅使用该提取器，这很有效。
2. 创建你自己的提取器，在其 [`FromRequest`] 实现中调用 axum 的内置提取器之一，但对于拒绝返回不同的响应。查看 [customize-extractor-error] 示例了解更多细节。

# 访问内部错误

axum 的内置提取器不直接暴露内部错误。这给了我们更多的灵活性，并允许我们在不破坏公共 API 的情况下更改内部实现。

例如，这意味着虽然 [`Json`] 是使用 [`serde_json`] 实现的，但它不直接包含在 [`JsonRejection::JsonDataError`] 中的 [`serde_json::Error`]。但是仍然可以通过 [`std::error::Error`] 的方法访问：

```rust
use std::error::Error;
use axum::{
    extract::{Json, rejection::JsonRejection},
    response::IntoResponse,
    http::StatusCode,
};
use serde_json::{json, Value};

async fn handler(
    result: Result<Json<Value>, JsonRejection>,
) -> Result<Json<Value>, (StatusCode, String)> {
    match result {
        // 如果客户端发送了有效的 JSON，我们就可以
        Ok(Json(payload)) => Ok(Json(json!({ "payload": payload }))),

        Err(err) => match err {
            JsonRejection::JsonDataError(err) => {
                Err(serde_json_error_response(err))
            }
            JsonRejection::JsonSyntaxError(err) => {
                Err(serde_json_error_response(err))
            }
            // 处理来自 `Json` 提取器的其他拒绝
            JsonRejection::MissingJsonContentType(_) => Err((
                StatusCode::BAD_REQUEST,
                "Missing `Content-Type: application/json` header".to_string(),
            )),
            JsonRejection::BytesRejection(_) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to buffer request body".to_string(),
            )),
            // 我们必须提供一个 catch-all 情况，因为 `JsonRejection` 被标记为
            // `#[non_exhaustive]`
            _ => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Unknown error".to_string(),
            )),
        },
    }
}

// 尝试提取内部 `serde_path_to_error::Error<serde_json::Error>`，
// 如果成功，我们可以提供更具体的错误。
//
// `Json` 使用 `serde_path_to_error`，所以错误将被包装在 `serde_path_to_error::Error` 中。
fn serde_json_error_response<E>(err: E) -> (StatusCode, String)
where
    E: Error + 'static,
{
    if let Some(err) = find_error_source::<serde_path_to_error::Error<serde_json::Error>>(&err) {
        let
 serde_json_err = err.inner();
        (
            StatusCode::BAD_REQUEST,
            format!(
                "Invalid JSON at line {} column {}",
                serde_json_err.line(),
                serde_json_err.column()
            ),
        )
    } else {
        (StatusCode::BAD_REQUEST, "Unknown error".to_string())
    }
}

// 尝试将 `err` 向下转换为 `T`，如果失败，递归尝试向下转换
// `err` 的 source
fn find_error_source<'a, T>(err: &'a (dyn Error + 'static)) -> Option<&'a T>
where
    T: Error + 'static,
{
    if let Some(err) = err.downcast_ref::<T>() {
        Some(err)
    } else if let Some(source) = err.source() {
        find_error_source(source)
    } else {
        None
    }
}
#
# #[tokio::main]
# async fn main() {
#     use axum::extract::FromRequest;
#
#     let req = axum::http::Request::builder()
#         .header("content-type", "application/json")
#         .body(axum::body::Body::from("{"))
#         .unwrap();
#
#     let err = match Json::<serde_json::Value>::from_request(req, &()).await.unwrap_err() {
#         JsonRejection::JsonSyntaxError(err) => err,
#         _ => panic!(),
#     };
#
#     let (_, body) = serde_json_error_response(err);
#     assert_eq!(body, "Invalid JSON at line 1 column 1");
# }
```

请注意，虽然这种方法有效，但如果 axum 在未来更改其内部实现使用不同的错误类型，它可能会可能中断。这种更改可能在非主要破坏性版本中发生。

# 定义自定义提取器

你也可以通过实现 [`FromRequestParts`] 或 [`FromRequest`] 来定义自己的提取器。

## 实现 `FromRequestParts`

如果你的提取器不需要访问请求体，实现 `FromRequestParts`：

```rust,no_run
use axum::{
    extract::FromRequestParts,
    routing::get,
    Router,
    http::{
        StatusCode,
        header::{HeaderValue, USER_AGENT},
        request::Parts,
    },
};

struct ExtractUserAgent(HeaderValue);

impl<S> FromRequestParts<S> for ExtractUserAgent
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        if let Some(user_agent) = parts.headers.get(USER_AGENT) {
            Ok(ExtractUserAgent(user_agent.clone()))
        } else {
            Err((StatusCode::BAD_REQUEST, "`User-Agent` header is missing"))
        }
    }
}

async fn handler(ExtractUserAgent(user_agent): ExtractUserAgent) {
    // ...
}

let app = Router::new().route("/foo", get(handler));
# let _: Router = app;
```

## 实现 `FromRequest`

如果你的提取器需要消费请求体，你必须实现 [`FromRequest`]

```rust,no_run
use axum::{
    extract::{Request, FromRequest},
    response::{Response, IntoResponse},
    body::{Bytes, Body},
    routing::get,
    Router,
    http::{
        StatusCode,
        header::{HeaderValue, USER_AGENT},
    },
};

struct ValidatedBody(Bytes);

impl<S> FromRequest<S> for ValidatedBody
where
    Bytes: FromRequest<S>,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let body = Bytes::from_request(req, state)
            .await
            .map_err(IntoResponse::into_response)?;

        // 做验证...

        Ok(Self(body))
    }
}

async fn handler(ValidatedBody(body): ValidatedBody) {
    // ...
}

let app = Router::new().route("/foo", get(handler));
# let _: Router = app;
```

## 不能同时实现 `FromRequest` 和 `FromRequestParts`

请注意，通过为同一类型直接实现 `FromRequest` 和 `FromRequestParts` 会导致你的提取器不可用，除非它包装了另一个提取器：

```rust,compile_fail
use axum::{
    Router,
    routing::get,
    extract::{FromRequest, Request, FromRequestParts},
    http::request::Parts,
    body::Body,
};
use std::convert::Infallible;

// 不包装另一个提取器的提取器
struct MyExtractor;

// `MyExtractor` 实现了 `FromRequest`
impl<S> FromRequest<S> for MyExtractor
where
    S: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        // ...
        # todo!()
    }
}

// 和 `FromRequestParts`
impl<S> FromRequestParts<S> for MyExtractor
where
    S: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // ...
        # todo!()
    }
}

let app = Router::new().route(
    "/",
    // 当我们在处理器函数中实际使用 `MyExtractor` 时会失败。
    // 这是由于 Rust 类型系统的限制。
    //
    // 解决方法是为你的提取器实现 `FromRequest` 或 `FromRequestParts`
    // 但不能同时实现，如果你的提取器不包装另一个提取器。
    //
    // 查看"包装提取器"了解如何包装其他提取器。
    get(|_: MyExtractor| async {}),
);
# let _: Router = app;
```

# 在 `FromRequest` 或 `FromRequestParts` 实现中访问其他提取器

定义自定义提取器时，你经常需要在实现中访问另一个提取器。

```rust
use axum::{
    extract::{Extension, FromRequestParts},
    http::{StatusCode, HeaderMap, request::Parts},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};

#[derive(Clone)]
struct State {
    // ...
}

struct AuthenticatedUser {
    // ...
}

impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // 你可以直接调用它们...
        let headers = HeaderMap::from_request_parts(parts, state)
            .await
            .map
_err(|err| match err {})?;

        // ... 或者从 `RequestExt` / `RequestPartsExt` 使用 `extract` / `extract_with_state`
        use axum::RequestPartsExt;
        let Extension(state) = parts.extract::<Extension<State>>()
            .await
            .map_err(|err| err.into_response())?;

        unimplemented!("actually perform authorization")
    }
}

async fn handler(user: AuthenticatedUser) {
    // ...
}

let state = State { /* ... */ };

let app = Router::new().route("/", get(handler)).layer(Extension(state));
# let _: Router = app;
```

# 请求体限制

出于安全原因，[`Bytes`] 默认不接受大于 2MB 的请求体。这也适用于内部使用 [`Bytes`] 的提取器，如 `String`、[`Json`] 和 [`[Form`]。

有关更多细节，包括如何禁用此限制，请参阅 [`DefaultBodyLimit`]。

# 包装提取器

如果你想编写一个通用包装另一个提取器（可能消费也可能不消费请求体）的提取器，你应该同时实现 [`FromRequest`] 和 [`FromRequestParts`]：

```rust
use axum::{
    Router,
    body::Body,
    routing::get,
    extract::{Request, FromRequest, FromRequestParts},
    http::{HeaderMap, request::Parts},
};
use std::time::{Instant, Duration};

// 包装另一个并测量运行时间的提取器
struct Timing<E> {
    extractor: E,
    duration: Duration,
}

// 我们必须实现 `FromRequestParts`
impl<S, T> FromRequestParts<S> for Timing<T>
where
    S: Send + Sync,
    T: FromRequestParts<S>,
{
    type Rejection = T::Rejection;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let start = Instant::now();
        let extractor = T::from_request_parts(parts, state).await?;
        let duration = start.elapsed();
        Ok(Timing {
            extractor,
            duration,
        })
    }
}

// 和 `FromRequest`
impl<S, T> FromRequest<S> for Timing<T>
where
    S: Send + Sync,
    T: FromRequest<S>,
{
    type Rejection = T::Rejection;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let start = Instant::now();
        let extractor = T::from_request(req, state).await?;
        let duration = start.elapsed();
        Ok(Timing {
            extractor,
            duration,
        })
    }
}

async fn handler(
    // 这使用 `FromRequestParts` 实现
    _: Timing<HeaderMap>,
    // 这使用 `FromRequest` 实现
    _: Timing<String>,
) {}
# let _: axum::routing::MethodRouter = axum::routing::get(handler);
```

# 记录拒绝

所有内置提取器都会记录拒绝，以便更容易调试。要查看日志，为 axum 启用 `tracing` 功能（默认启用）和 `axum::rejection=trace` 追踪目标，例如使用 `RUST_LOG=info,axum::rejection=trace cargo run`。

[axum-extra]: https://docs.rs/axum-extra/latest/axum_extra/extract/index.html
[`body::Body`]: crate::body::Body
[`Bytes`]: crate::body::Bytes
[customize-extractor-error]: https://github.com/tokio-rs/axum/blob/main/examples/customize-extractor-error/src/main.rs
[`HeaderMap`]: https://docs.rs/http/latest/http/header/struct.HeaderMap.html
[`Request`]: https://docs.rs/http/latest/http/struct.Request.html
[`Json`]: crate::extract::Json
[`Form`]: crate::extract::Form
[`JsonRejection::JsonDataError`]: rejection::JsonRejection::JsonDataError
