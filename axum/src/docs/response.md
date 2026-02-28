用于生成响应的类型和 trait。

# 构建响应

任何实现 [`IntoResponse`] 的东西都可以从处理器返回。axum 为常用类型提供实现：

```rust,no_run
use axum::{
    Json,
    response::{Html, IntoResponse},
    http::{StatusCode, Uri, header::{self, HeaderMap, HeaderName}},
};

// `()` 给出一个空响应
async fn empty() {}

// String 将获得一个 `text/plain; charset=utf-8` content-type
async fn plain_text(uri: Uri) -> String {
    format!("Hi from {}", uri.path())
}

// Bytes 将获得一个 `application/octet-stream` content-type
async fn bytes() -> Vec<u8> {
    vec![1, 2, 3, 4]
}

// `Json` 将获得一个 `application/json` content-type 并与任何
// 实现 `serde::Serialize` 的类型一起工作
async fn json() -> Json<Vec<String>> {
    Json(vec!["foo".to_owned(), "bar".to_owned()])
}

// `Html` 将获得一个 `text/html` content-type
async fn html() -> Html<&'static str> {
    Html("<p>Hello, World!</p>")
}

// `StatusCode` 给出一个带有该状态码的空响应
async fn status() -> StatusCode {
    StatusCode::NOT_FOUND
}

// `HeaderMap` 给出一个带有一些头部的空响应
async fn headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(header::SERVER, "axum".parse().unwrap());
    headers
}

// 一个元组数组也给出头部
async fn array_headers() -> [(HeaderName, &'static str); 2] {
    [
        (header::SERVER, "axum"),
        (header::CONTENT_TYPE, "text/plain")
    ]
}

// 使用 `impl IntoResponse` 来避免编写整个类型
async fn impl_trait() -> impl IntoResponse {
    [
        (header::SERVER, "axum"),
        (header::CONTENT_TYPE, "text/plain")
    ]
}
```

此外，你可以返回元组来从各个部分构建更复杂的响应。

```rust,no_run
use axum::{
    Json,
    response::IntoResponse,
    http::{StatusCode, HeaderMap, Uri, header},
    extract::Extension,
};

// `(StatusCode, impl IntoResponse)` 将覆盖响应的状态码
async fn with_status(uri: Uri) -> (StatusCode, String) {
    (StatusCode::NOT_FOUND, format!("Not Found: {}", uri.path()))
}

// 使用 `impl IntoResponse` 来避免编写整个类型
async fn impl_trait(uri: Uri) -> impl IntoResponse {
    (StatusCode::NOT_FOUND, format!("Not Found: {}", uri.path()))
}

// `(HeaderMap, impl IntoResponse)` 添加额外的头部
async fn with_headers() -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "text/plain".parse().unwrap());
    (headers, "foo")
}

// 或者使用元组数组来更容易地构建头部
async fn with_array_headers() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "text/plain")], "foo")
}

// 使用字符串键来定制头部
async fn with_array_headers_custom() -> impl IntoResponse {
    ([("x-custom", "custom")], "foo")
}

// `(StatusCode, headers, impl IntoResponse)` 设置状态并添加头部
// `headers` 可以是 `HeaderMap` 或元组数组
async fn with_status_and_array_headers() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        [(header::CONTENT_TYPE, "text/plain")],
        "foo",
    )
}

// `(Extension<_>, impl IntoResponse)` 设置响应扩展
async fn with_status_extensions() -> impl IntoResponse {
    (
        Extension(Foo("foo")),
        "foo",
    )
}

#[derive(Clone)]
struct Foo(&'static str);

// 或者混合匹配所有东西
async fn all_the_things(uri: Uri) -> impl IntoResponse {
    let mut header_map = HeaderMap::new();
    if uri.path() == "/" {
        header_map.insert(header::SERVER, "axum".parse().unwrap());
    }

    (
        // 设置状态码
        StatusCode::NOT_FOUND,
        // 头部使用数组
        [("x-custom", "custom")],
        // 一些扩展
        Extension(Foo("foo")),
        Extension(Foo("bar")),
        // 更多头部，动态构建
        header_map,
        // 最后是主体
        "foo",
    )
}
```

一般来说，你可以返回像这样的元组：

- `(StatusCode, impl IntoResponse)`
- `(Parts, impl IntoResponse)`
- `(Response<()>, impl IntoResponse)`
- `(T1, .., Tn, impl IntoResponse)` 其中 `T1` 到 `Tn` 都实现 [`IntoResponseParts`]
- `(StatusCode, T1, .., Tn, impl IntoResponse)` 其中 `T1` 到 `Tn` 都实现 [`IntoResponseParts`]
- `(Parts, T1, .., Tn, impl IntoResponse)` 其中 `T1` 到 `Tn` 都实现 [`IntoResponseParts`]
- `(Response<()>, T1, .., Tn, impl IntoResponse)` 其中 `T1` 到 `Tn` 都实现 [`IntoResponseParts`]

这意味着你不能意外覆盖状态或主体，因为 [`IntoResponseParts`] 只允许设置头部和扩展。

使用 [`Response`] 获取更底层的控制：

```rust,no_run
use axum::{
    Json,
    response::{IntoResponse, Response},
    body::Body,
    http::StatusCode,
};

async fn response() -> Response {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header("x-foo", "custom header")
        .body(Body::from("not found"))
        .unwrap()
}
```

# 返回不同的响应类型

如果你需要返回多个响应类型，并且 `Result<T, E>` 不合适，可以调用 `.into_response()` 将东西转换为 `axum::response::Response`：

```rust
use axum::{
    response::{IntoResponse, Redirect, Response},
    http::StatusCode,
};

async fn handle() -> Response {
    if something() {
        "All good!".into_response()
    } else if something_else() {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Something went wrong...",
        ).into_response()
    } else {
        Redirect::to("/").into_response()
    }
}

fn something() -> bool {
    // ...
    # true
}

fn something_else() -> bool {
    // ...
    # true
}
```

# 关于 `impl IntoResponse`

可以使用 `impl IntoResponse` 作为处理器的返回类型以避免编写大型类型。例如

```rust
use axum::http::StatusCode;

async fn handler() -> (StatusCode, [(&'static str, &'static str); 1], &'static str) {
    (StatusCode::OK, [("x-foo", "bar")], "Hello, World!")
}
```

使用 `impl IntoResponse` 变得更简单：

```rust
use axum::{http::StatusCode, response::IntoResponse};

async fn impl_into_response() -> impl IntoResponse {
    (StatusCode::OK, [("x-foo", "bar")], "Hello, World!")
}
```

然而 `impl IntoResponse` 有一些限制。首先它只能用于返回单个类型：

```rust,compile_fail
use axum::{http::StatusCode, response::IntoResponse};

async fn handler() -> impl IntoResponse {
    if check_something() {
        StatusCode::NOT_FOUND
    } else {
        "Hello, World!"
    }
}

fn check_something() -> bool {
    # false
    // ...
}
```

这个函数返回 `StatusCode` 或 `&'static str`，`impl Trait` 不允许这样做。

其次，当与 `Result` 和 `?` 一起使用时，`impl IntoResponse` 可能导致类型推断问题：

```rust,compile_fail
use axum::{http::StatusCode, response::IntoResponse};

async fn handler() -> impl IntoResponse {
    create_thing()?;
    Ok(StatusCode::CREATED)
}

fn create_thing() -> Result<(), StatusCode> {
    # Ok(())
    // ...
}
```

这是因为 `?` 支持使用 [`From`] trait 转换为不同的错误类型，但它不知道要转换为哪种类型，因为我们只将返回类型指定为 `impl IntoResponse`。

`Result<impl IntoResponse, impl IntoResponse>` 也不总是有效：

```rust,compile_fail
use axum::{http::StatusCode, response::IntoResponse};

async fn handler() -> Result<impl IntoResponse, impl IntoResponse> {
    create_thing()?;
    Ok(StatusCode::CREATED)
}

fn create_thing() -> Result<(), StatusCode> {
    # Ok(())
    // ...
}
```

解决方案是使用具体的错误类型，如 `Result<impl IntoResponse, StatusCode>`：

```rust
use axum::{http::StatusCode, response::IntoResponse};

async fn handler() -> Result<impl IntoResponse, StatusCode> {
    create_thing()?;
    Ok(StatusCode::CREATED)
}

fn create_thing() -> Result<(), StatusCode> {
    # Ok(())
    // ...
}
```

因此，通常不建议使用 `impl IntoResponse`，除非你熟悉 `impl Trait` 工作方式的细节。

[`IntoResponse`]: crate::response::IntoResponse
[`IntoResponseParts`]: crate::response::IntoResponseParts
[`StatusCode`]: http::StatusCode
