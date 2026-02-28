# 简介

axum 的独特之处在于它没有自己的定制中间件系统，而是与 [`tower`] 集成。这意味着 [`tower`] 和 [`tower-http`] 中间件的生态系统都可以与 axum 一起工作。

虽然不需要完全理解 tower 就可以使用或编写与 axum 一起工作的中间件，但至少对 tower 的概念有一个基本的理解是推荐的。参阅 [tower 的指南][tower-guides] 获取一般介绍。阅读 [`tower::ServiceBuilder`] 的文档也是推荐的。

# 应用中间件

axum 允许你在几乎任何地方添加中间件

- 使用 [`Router::layer`] 和 [`Router::route_layer`] 将其应用到整个路由器
- 使用 [`MethodRouter::layer`] 和 [`MethodRouter::route_layer`] 将其应用到方法路由器
- 使用 [`Handler::layer`] 将其应用到单个处理器

## 应用多个中间件

推荐使用 [`tower::ServiceBuilder`] 一次应用多个中间件，而不是重复调用 `layer`（或 `route_layer`）：

```rust
use axum::{
    routing::get,
    Extension,
    Router,
};
use tower_http::{trace::TraceLayer};
use tower::ServiceBuilder;

async fn handler() {}

#[derive(Clone)]
struct State {}

let app = Router::new()
    .route("/", get(handler))
    .layer(
        ServiceBuilder::new()
            .layer(TraceLayer::new_for_http())
            .layer(Extension(State {}))
    );
# let _: Router = app;
```

# 常用中间件

一些常用的中间件包括：

- [`TraceLayer`](tower_http::trace) 用于高级追踪/日志记录
- [`CorsLayer`](tower_http::cors) 用于处理 CORS
- [`CompressionLayer`](tower_http::compression) 用于自动压缩响应
- [`RequestIdLayer`](tower_http::request_id) 和
  [`PropagateRequestIdLayer`](tower_http::request_id) 设置和传播请求 ID
- [`TimeoutLayer`](tower_http::timeout::TimeoutLayer) 用于超时

# 排序

当你使用 [`Router::layer`]（或类似方法）添加中间件时，所有之前添加的路由将被包装在中间件中。一般来说，这会导致中间件从下到上执行。

所以如果你这样做：

```rust
use axum::{routing::get, Router};

async fn handler() {}

# let layer_one = axum::Extension(());
# let layer_two = axum::Extension(());
# let layer_three = axum::Extension(());
#
let app = Router::new()
    .route("/", get(handler))
    .layer(layer_one)
    .layer(layer_two)
    .layer(layer_three);
# let _: Router = app;
```

将中间件想象成洋葱状的层，每个新层包装所有先前的层：

```not_rust
        requests
           |
           v
+----- layer_three -----+
| +---- layer_two ----+ |
| | +-- layer_one --+ | |
| | |               | | |
| | |    handler    | | |
| | |               | | |
| | +-- layer_one --+ | |
| +---- layer_two ----+ |
+----- layer_three -----+
           |
           v
        responses
```

也就是说：

- 首先 `layer_three` 接收请求
- 然后它做它的事情并将请求传递给 `layer_two`
- 后者将请求传递给 `layer_one`
- 后者将请求传递给 `handler`，在那里产生响应
- 该响应然后传递给 `layer_one`
- 然后传递给 `layer_two`
- 最后传递给 `layer_three`，它从你的应用中返回出来

实际上这稍微复杂一些，因为任何中间件都可以提前返回而不调用下一层，例如如果请求没有被授权，但它是一个有用的心智模型。

如前所述，推荐使用 `tower::ServiceBuilder` 添加多个中间件，但这会影响排序：

```rust
use tower::ServiceBuilder;
use axum::{routing::get, Router};

async fn handler() {}

# let layer_one = axum::Extension(());
# let layer_two = axum::Extension(());
# let layer_three = axum::Extension(());
#
let app = Router::new()
    .route("/", get(handler))
    .layer(
        ServiceBuilder::new()
            .layer(layer_one)
            .layer(layer_two)
            .layer(layer_three),
    );
# let _: Router = app;
```

`ServiceBuilder` 通过将所有层组合成一个来工作，使它们从上到下运行。所以对于之前的代码，`layer_one` 将首先接收请求，然后是 `layer_two`，然后是 `layer_three`，然后是 `handler`，然后响应将通过 `layer_three` 向上冒泡，然后是 `layer_two`，最后是 `layer_one`。

从上到下执行中间件通常更容易理解和心理跟随，这是推荐使用 `ServiceBuilder` 的原因之一。

# 编写中间件

axum 提供了多种编写中间件的方法，在不同级别的抽象上，具有不同的优缺点。

## `axum::middleware::from_fn`

使用 [`axum::middleware::from_fn`] 编写中间件时：

- 你对于实现自己的 future 感到不自在，宁愿使用熟悉的 `async`/`await` 语法
- 你不打算将中间件作为 crate 发布给他人使用。这样编写的中间件仅与 axum 兼容

## `axum::middleware::from_extractor`

使用 [`axum::middleware::from_extractor`] 编写中间件时：

- 你有一个类型，有时你想将其用作提取器，有时想用作中间件。如果你只需要将类型用作中间件，prefer [`middleware::from_fn`]

## tower 的组合器

tower 有几个工具组合器，可用于对请求或响应执行简单的修改。最常用的是

- [`ServiceBuilder::map_request`]
- [`ServiceBuilder::map_response`]
- [`ServiceBuilder::then`]
- [`ServiceBuilder::and_then`]

你应该在以下情况使用这些：

- 你想要执行一个小的临时操作，例如添加一个头部
- 你不打算将中间件作为 crate 发布给他人使用

## `tower::Service` 和 `Pin<Box<dyn Future>>`

要获得最大的控制（和更底层的 API），你可以通过实现 [`tower::Service`] 来编写自己的中间件：

使用 [`tower::Service`] 和 `Pin<Box<dyn Future>>` 编写中间件时：

- 你的中间件需要是可配置的，例如通过你的 [`tower::Layer`] 上的构建器方法，如 [`tower_http::trace::TraceLayer`]
- 你打算将中间件作为 crate 发布给他人使用
- 你对于实现自己的 future 感到不自在

这样的中间件的一个不错的模板是：

```rust
use axum::{
    response::Response,
    body::Body,
    extract::Request,
};
use futures_core::future::BoxFuture;
use tower::{Service, Layer};
use std::task::{Context, Poll};

#[derive(Clone)]
struct MyLayer;

impl<S> Layer<S> for MyLayer {
    type Service = MyMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        MyMiddleware { inner }
    }
}

#[derive(Clone)]
struct MyMiddleware<S> {
    inner: S,
}

impl<S> Service<Request> for MyMiddleware<S>
where
    S: Service<Request, Response = Response> + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    // `BoxFuture` 是一个类型别名，代表 `Pin<Box<dyn Future + Send + 'a>>`
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        let future = self.inner.call(request);
        Box::pin(async move {
            let response: Response = future.await?;
            Ok(response)
        })
    }
}
```

请注意，将你的错误类型定义为 `S::Error` 意味着你的中间件通常_不返回错误_。作为原则，始终尝试返回响应，尝试不要使用自定义错误类型退出。例如，如果你在新中间件中使用的第三方库返回其自己的专门错误类型，尝试将其转换为合理的响应并返回带有该响应的 `Ok`。

如果你选择实现自定义错误类型，如 `type Error = BoxError`（一个 boxed 不透明错误），或任何其他不是 `Infallible` 的错误类型，你必须使用 `HandleErrorLayer`，这里是一个使用 `ServiceBuilder` 的示例：

```ignore
ServiceBuilder::new()
        .layer(HandleErrorLayer::new(|_: BoxError| async {
            // 因为 axum 使用不可变错误，你必须在这里处理来自中间件的自定义错误类型
            StatusCode::BAD_REQUEST
        }))
        .layer(
             // <你的实际确实返回错误的层>
        );
```

## `tower::Service` 和自定义 futures

如果你对于实现自己的 future 感到自在（或想学习它）并且需要尽可能多的控制，那么使用不带 boxed futures 的 `tower::Service` 是正确的方法。

使用 [`tower::Service`] 和手动 futures 编写中间件时：

- 你想要你的中间件具有尽可能低的开销
- 你的中间件需要是可配置的，例如通过你的 [`tower::Layer`] 上的构建器方法，如 [`tower_http::trace::TraceLayer`]
- 你打算将中间件作为 crate 发布给他人使用，也许是作为 tower-http 的一部分
- 你对于实现自己的 future 感到自在，或者想学习底层 async Rust 如何工作

tower 的 ["从零开始构建中间件"][tower-from-scratch-guide] 指南是学习如何做这件事的好地方。

# 中间件的错误处理

axum 的错误处理模型要求处理器始终返回响应。然而中间件是将错误引入应用程序的一种可能方式。如果 hyper 接收到错误，连接将在不发送响应的情况下关闭。因此 axum 要求优雅地处理这些错误：

```rust
use axum::{
    routing::get,
    error_handling::HandleErrorLayer,
    http::StatusCode,
    BoxError,
    Router,
};
use tower::{ServiceBuilder, timeout::TimeoutLayer};
use std::time::Duration;

async fn handler() {}

let app = Router::new()
    .route("/", get(handler))
    .layer(
        ServiceBuilder::new()
            // 这个中间件放在 `TimeoutLayer` 之上，因为它将接收
            // `TimeoutLayer` 返回的错误
            .layer(HandleErrorLayer::new(|_: BoxError| async {
                StatusCode::REQUEST_TIMEOUT
            }))
            .layer(TimeoutLayer::new(Duration::from_secs(10)))
    );
# let _: Router = app;
```

有关 axum 错误处理模型的更多细节，请参阅 [`error_handling`](crate::error_handling)

# 赯由到服务/中间件和背压

通常路由到多个服务之一和背压混合不好。理想情况下，你希望在调用服务之前确保服务已准备好接收请求。然而，为了知道要调用哪个服务，你需要请求...

一种方法是不考虑路由器服务本身准备好，直到所有目标服务都准备好。这是 [`tower::steer::Steer`] 使用的方法。

另一种方法是始终考虑所有服务都准备好（从 `Service::poll_ready` 始终返回 `Poll::Ready(Ok(()))`），然后在 `Service::call` 返回的响应 future 中实际驱动就绪。这在你的服务不关心背压并且始终准备好的情况下工作良好。

axum 期望应用程序中使用的所有服务都不关心背压，因此它使用后一种策略。然而这意味着你应该避免路由到_确实_关心背压的服务（或使用这样的中间件）。至少你应该 [负载丢弃][tower::load_shed]，以便请求被快速丢弃，而不继续堆积。

这也意味着如果 `poll_ready` 返回错误，该错误将在 `call` 返回的响应 future 中返回，而不是从 `poll_ready` 返回。在这种情况下，底层服务将不会被丢弃，并继续用于未来的请求。期望在 `poll_ready` 失败时被丢弃的服务不应与 axum 一起使用。

一种可能的方法是仅在整个应用程序周围应用对背压敏感的中间件。这是可能的，因为 axum 应用程序本身就是服务：

```rust
use axum::{
    routing::get,
    Router,
};
use tower::ServiceBuilder;
# let some_backpressure_sensitive_middleware =
#     tower::layered::util::Identity::new();

async fn handler() { /* ... */ }

let app = Router::new().route("/", get(handler));

let app = ServiceBuilder::new()
    .layer(some_backpressure_sensitive_middleware)
    .service(app);
# let _: Router = app;
```

然而，以这种方式在整个应用程序周围应用中间件时，你必须注意错误仍然被适当处理。

还要注意，从 async 函数创建的处理器不关心背压并且始终准备就绪。所以如果你不使用任何 Tower 中间件，你不必担心任何这些。

# 在中间件中访问状态

如何使状态对中间件可用取决于中间件的编写方式。

## 在 `axum::middleware::from_fn` 中访问状态

使用 [`axum::middleware::from_fn_with_state`](crate::middleware::from_fn_with_state)。

## 在自定义 `tower::Layer` 中访问状态

```rust
use axum::{
    Router,
    routing::get,
    middleware::{self, Next},
    response::Response,
    extract::{State, Request},
};
use tower::{Layer, Service};
use std::task::{Context, Poll};

#[derive(Clone)]
struct AppState {}

#[derive(Clone)]
struct MyLayer {
    state: AppState,
}

impl<S> Layer<S> for MyLayer {
    type Service = MyService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        MyService {
            inner,
            state: self.state.clone(),
        }
    }
}

#[derive(Clone)]
struct MyService<S> {
    inner: S,
    state: AppState,
}

impl<S, B> Service<Request<B>> for MyService<S>
where
    S: Service<Request<B>>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        // 使用 `self.state` 做一些事情
        //
        // 参阅 `axum::RequestExt` 了解如何直接从
        // `Request` 运行提取器

        self.inner.call(req)
    }
}

async fn handler(_: State<AppState>) {}

let state = AppState {};

let app = Router::new()
    .route("/", get(handler))
    .layer(MyLayer { state: state.clone() })
    .with_state(state);
# let _: axum::Router = app;
```

# 从中间件传递状态到处理器

可以使用 [请求扩展]将状态从中间件传递到处理器：

```rust
use axum::{
    Router,
    http::StatusCode,
    routing::get,
    response::{IntoResponse, Response},
    middleware::{self, Next},
    extract::{Request, Extension},
};

#[derive(Clone)]
struct CurrentUser { /* ... */ }

async fn auth(mut req: Request, next: Next) -> Result<Response, StatusCode> {
    let auth_header = req.headers()
        .get(http::header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok());

    let auth_header = if let Some(auth_header) = auth_header {
        auth_header
    } else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    if let Some(current_user) = authorize_current_user(auth_header).await {
        // 将当前用户插入请求扩展中，以便处理器可以
        // 提取它
        req.extensions_mut().insert(current_user);
        Ok(next.run(req).await)
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

async fn authorize_current_user(auth_token: &str) -> Option<CurrentUser> {
    // ...
    # unimplemented!()
}

async fn handler(
    // 提取当前用户，由中间件设置
    Extension(current_user): Extension<CurrentUser>,
) {
    // ...
}

let app = Router::new()
    .route("/", get(handler))
    .route_layer(middleware::from_fn(auth));
# let _: Router = app;
```

也可以使用 [响应扩展]，但请注意请求扩展不会自动移动到响应扩展。你需要为你需要的扩展手动执行此操作。

# 在中间件中重写请求 URI

使用 [`Router::layer`] 添加的中间件将在路由后运行。这意味着它不能用于运行重写请求 URI 的中间件。到中间件运行时，路由已经完成。

解决方法是将中间件包装在整个 `Router` 周围（这有效，因为 `Router` 实现了 [`Service`]）：

```rust
use tower::Layer;
use axum::{
    Router,
    ServiceExt, // for `into_make_service`
    response::Response,
    middleware::Next,
    extract::Request,
};

fn rewrite_request_uri<B>(req: Request<B>) -> Request<B> {
    // ...
    # req
}

// 这可以是任何 `tower::Layer`
let middleware = tower::util::MapRequestLayer::new(rewrite_request_uri);

let app = Router::new();

//将层应用在整个 `Router` 周围
//这样中间件将在 `Router` 接收请求之前运行
let app_with_middleware = middleware.layer(app);

# async {
let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
axum::serve(listener, app_with_middleware.into_make_service()).await;
# };
```

[`tower`]: https://crates.io/crates/tower
[`tower-http`]: https://crates.io/crates/tower-http
[tower-guides]: https://github.com/tower-rs/tower/tree/master/guides
[`axum::middleware::from_fn`]: fn@crate::middleware::from_fn
[`middleware::from_fn`]: fn@crate::middleware::from_fn
[tower-from-scratch-guide]: https://github.com/tower-rs/tower/blob/master/guides/building-a-middleware-from-scratch.md
[`ServiceBuilder::map_request`]: tower::ServiceBuilder::map_request
[`ServiceBuilder::map_response`]: tower::ServiceBuilder::map_response
[`ServiceBuilder::then`]: tower::ServiceBuilder::then
[`ServiceBuilder::and_then`]: tower::ServiceBuilder::and_then
[`axum::middleware::from_extractor`]: fn@crate::middleware::from_extractor
[`Handler::layer`]: crate::handler::Handler::layer
[`Router::layer`]: crate::routing::Router::layer
[`MethodRouter::layer`]: crate::routing::MethodRouter::layer
[`Router::route_layer`]: crate::routing::Router::route_layer
[`MethodRouter::route_layer`]: crate::routing::MethodRouter::route_layer
[request extensions]: https://docs.rs/http/latest/http/request/struct.Request.html#method.extensions
[响应扩展]: https://docs.rs/http/latest/http/response/struct.Response.html#method.extensions
[`State`]: crate::extract::State
[`Service`]: tower::Service
