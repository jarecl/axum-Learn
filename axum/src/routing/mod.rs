//! 路由模块 - 在 [`Service`] 和处理器之间进行路由
//!
//! 在 Spring Boot 中，这相当于 @RequestMapping、@GetMapping、@PostMapping 等注解的功能
//! 以及 RouterFunction 用于构建路由配置

use self::{future::RouteFuture, not_found::NotFound, path_router::PathRouter};
#[cfg(feature = "tokio")]
use crate::extract::connect_info::IntoMakeServiceWithConnectInfo;
#[cfg(feature = "matched-path")]
use crate::extract::MatchedPath;
use crate::{
    body::{Body, HttpBody},
    boxed::BoxedIntoRoute,
    handler::Handler,
    util::try_downcast,
};
use axum_core::{
    extract::Request,
    response::{IntoResponse, Response},
};
use std::{
    convert::Infallible,
    fmt,
    marker::PhantomData,
    sync::Arc,
    task::{Context, Poll},
};
use tower::service_fn;
use tower_layer::{layer_fn, Layer};
use tower_service::Service;

// 公共模块导出
pub mod future;      /// 路由处理的 Future 类型
pub mod method_routing; /// HTTP 方法路由（GET、POST 等）

// 内部模块
mod into_make_service; /// 将 Router 转换为 MakeService 的工具
mod method_filter;     /// HTTP 方法过滤器
mod not_found;         /// 404 Not Found 处理器
pub(crate) mod path_router; /// 路径路由的核心实现
mod route;            /// Route 类型
mod strip_prefix;     /// 去除路径前缀的工具
pub(crate) mod url_params; /// URL 参数解析

#[cfg(test)]
mod tests;

// 公共类型导出
pub use self::{into_make_service::IntoMakeService, method_filter::MethodFilter, route::Route};

// 导出所有 HTTP 方法路由函数
// 在 Spring Boot 中，这些相当于 @GetMapping、@PostMapping 等注解
pub use self::method_routing::{
    any, any_service,      /// 匹配任何 HTTP 方法
    connect, connect_service, /// CONNECT 方法
    delete, delete_service,  /// DELETE 方法（@DeleteMapping）
    get, get_service,        /// GET 方法（@GetMapping）
    head, head_service,      /// HEAD 方法
    on, on_service,          /// 自定义 HTTP 方法
    options, options_service, /// OPTIONS 方法
    patch, patch_service,    /// PATCH 方法（@PatchMapping）
    post, post_service,      /// POST 方法（@PostMapping）
    put, put_service,        /// PUT 方法（@PutMapping）
    trace, trace_service,    /// TRACE 方法
    MethodRouter,            /// HTTP 方法路由器类型
};

// 内部宏：如果表达式返回 Err 则 panic
macro_rules! panic_on_err {
    ($expr:expr) => {
        match $expr {
            Ok(x) => x,
            Err(err) => panic!("{err}"),
        }
    };
}

// TakeOnceRoute 被多次调用时的错误消息
const TAKE_ONCE_ROUTE_PANIC_MSG: &str =
    "TakeOnceRoute called more than once; if this was not triggered by an intentional test, this should never happen. Please file an issue.";

// 从 Option<Route> 中取出 Route，如果没有则返回内部错误
// 这是一个优化：知道服务只会被调用一次，所以使用 Option 来避免克隆
fn take_route_or_internal_error(service: &mut Option<Route>) -> Route {
    service.take().unwrap_or_else(|| {
        if cfg!(debug_assertions) {
            panic!("{TAKE_ONCE_ROUTE_PANIC_MSG}");
        }

        Route::new(service_fn(|_req: Request| async move {
            Ok::<_, Infallible>(http::StatusCode::INTERNAL_SERVER_ERROR.into_response())
        }))
    })
}

// 路由 ID，内部用于标识路由
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct RouteId(usize);

/// Router - 路由器类型，用于组合处理器和服务
///
/// 在 Spring Boot 中，这相当于 RouterFunction 或使用 @RequestMapping 配置的 Controller
///
/// `Router<S>` 表示一个"缺少"类型 `S` 状态的路由器，需要提供状态才能处理请求。
/// 因此，只有 `Router<()>`（即没有缺少状态）才能传递给 [`serve`]。
/// 更多详情请参阅 [`Router::with_state`]。
///
/// 在 Java Spring Boot 中对应概念：
/// - Router 相当于 RouterFunction 或 Controller 类
/// - S 相当于依赖注入的应用状态
/// - with_state() 相当于设置应用上下文
///
/// [`serve`]: crate::serve()
#[must_use]
pub struct Router<S = ()> {
    inner: Arc<RouterInner<S>>,
}

// Router 的 Clone 实现（因为内部使用 Arc，克隆很便宜）
impl<S> Clone for Router<S> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

// Router 的内部结构（不公开）
struct RouterInner<S> {
    path_router: PathRouter<S>,         // 路径路由器
    default_fallback: bool,              // 是否使用默认的 fallback
    catch_all_fallback: Fallback<S>,    // 捕获所有请求的 fallback 处理器
}

// 为 Router 实现 Default（当 S 满足条件时）
impl<S> Default for Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

// Router 的 Debug 实现
impl<S> fmt::Debug for Router<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Router")
            .field("path_router", &self.inner.path_router)
            .field("default_fallback", &self.inner.default_fallback)
            .field("catch_all_fallback", &self.inner.catch_all_fallback)
            .finish()
    }
}

// 内部使用的常量，用于嵌套路由和 fallback 的参数名
pub(crate) const NEST_TAIL_PARAM: &str = "__private__axum_nest_tail_param";
#[cfg(feature = "matched-path")]
pub(crate) const NEST_TAIL_PARAM_CAPTURE: &str = "/{*__private__axum_nest_tail_param}";
pub(crate) const FALLBACK_PARAM: &str = "__private__axum_fallback";
pub(crate) const FALLBACK_PARAM_PATH: &str = "/{*__private__axum_fallback}";

// 内部宏：映射 inner（创建新的 Router）
macro_rules! map_inner {
    ( $self_:ident, $inner:pat_param => $expr:expr) => {
        #[allow(redundant_semicolons)]
        {
            let $inner = $self_.into_inner();
            Router {
                inner: Arc::new($expr),
            }
        }
    };
}

// 内部宏：修改 inner（创建新的 Router）
macro_rules! tap_inner {
    ( $self_:ident, mut $inner:ident => { $($stmt:stmt)* } ) => {
        #[allow(redundant_semicolons)]
        {
            let mut $inner = $self_.into_inner();
            $($stmt)*;
            Router {
                inner: Arc::new($inner),
            }
        }
    };
}

// Router 的主要方法实现（当 S 满足条件时）
impl<S> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    /// 创建一个新的 `Router`
    ///
    /// 除非你添加其他路由，否则这将对所有请求响应 `404 Not Found`。
    ///
    /// 在 Spring Boot 中，这相当于创建一个空的 RouterFunction 或没有路径的 Controller
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RouterInner {
                path_router: Default::default(),
                default_fallback: true,
                catch_all_fallback: Fallback::Default(Route::new(NotFound)),
            }),
        }
    }

    // 将 Router 消费为内部结构
    // 如果 Arc 只有一个引用，则直接取出；否则克隆
    fn into_inner(self) -> RouterInner<S> {
        match Arc::try_unwrap(self.inner) {
            Ok(inner) => inner,
            Err(arc) => RouterInner {
                path_router: arc.path_router.clone(),
                default_fallback: arc.default_fallback,
                catch_all_fallback: arc.catch_all_fallback.clone(),
            },
        }
    }

    /// 禁用 v0.7 兼容性检查
    ///
    /// 这是用于从 axum 0.6 迁移到 0.7 的辅助方法
    #[doc = include_str!("../docs/routing/without_v07_checks.md")]
    pub fn without_v07_checks(self) -> Self {
        tap_inner!(self, mut this => {
            this.path_router.without_v07_checks();
        })
    }

    /// 添加路由
    ///
    /// 在 Spring Boot 中，这相当于：
    /// - @GetMapping(path)
    /// - @PostMapping(path)
    /// - @RequestMapping(path, method=GET)
    ///
    /// 示例：
    /// ```rust
    /// use axum::{routing::get, Router};
    ///
    /// // 相当于 @GetMapping("/users")
    /// let app = Router::new().route("/users", get(|| async { "Hello" }));
    /// ```
    #[doc = include_str!("../docs/routing/route.md")]
    #[track_caller]
    pub fn route(self, path: &str, method_router: MethodRouter<S>) -> Self {
        tap_inner!(self, mut this => {
            panic_on_err!(this.path_router.route(path, method_router));
        })
    }

    /// 为服务添加路由
    ///
    /// 这类似于 `route`，但接受任意的 `Service` 而不是 `MethodRouter`
    ///
    /// 在 Spring Boot 中，这相当于将某个 Filter 或 HandlerFunction 注册到特定路径
    #[doc = include_str!("../docs/routing/route_service.md")]
    pub fn route_service<T>(self, path: &str, service: T) -> Self
    where
        T: Service<Request, Error = Infallible> + Clone + Send + Sync + 'static,
        T::Response: IntoResponse,
        T::Future: Send + 'static,
    {
        // 检查是否错误地传递了 Router，应该使用 nest 而不是 route_service
        let Err(service) = try_downcast::<Self, _>(service) else {
            panic!(
                "Invalid route: `Router::route_service` cannot be used with `Router`s. \
                Use `Router::nest` instead"
            );
        };

        tap_inner!(self, mut this => {
            panic_on_err!(this.path_router.route_service(path, service));
        })
    }

    /// 嵌套路由器
    ///
    /// 在 Spring Boot 中，这相当于：
    /// - 使用 @RequestMapping() 的 Controller 嵌套
    /// - RouterFunction 的 nest() 方法
    ///
    /// 示例：
    /// ```rust
    /// use axum::{routing::get, Router};
    ///
    /// let api = Router::new()
    ///     .route("/users", get(users_handler))
    ///     .route("/posts", get(posts_handler));
    ///
    /// // 相当于 @RequestMapping("/api") 的 Controller
    /// let app = Router::new().nest("/api", api);
    /// ```
    #[doc = include_str!("../docs/routing/nest.md")]
    #[doc(alias = "scope")] // 其他一些库如 actix-web 使用这个术语
    #[track_caller]
    pub fn nest(self, path: &str, router: Self) -> Self {
        // 不允许在根路径嵌套
        if path.is_empty() || path == "/" {
            panic!("Nesting at the root is no longer supported. Use merge instead.");
        }

        // 取出嵌套路由器的内部结构
        let RouterInner {
            path_router,
            default_fallback: _,
            // 我们不需要继承 catch-all fallback。它仅用于具有空路径的 CONNECT 请求。
            // 如果我们要继承 catch-all fallback，它将匹配 `/{path}/*`，这不匹配空路径。
            catch_all_fallback: _,
        } = router.into_inner();

        tap_inner!(self, mut this => {
            panic_on_err!(this.path_router.nest(path, path_router));
        })
    }

    /// 类似于 [`nest`](Self::nest)，但接受任意的 `Service`
    ///
    /// 在 Spring Boot 中，这相当于将整个 Filter 链注册到某个前缀路径
    #[track_caller]
    pub fn nest_service<T>(self, path: &str, service: T) -> Self
    where
        T: Service<Request, Error = Infallible> + Clone + Send + Sync + 'static,
        T::Response: IntoResponse,
        T::Future: Send + 'static,
    {
        if path.is_empty() || path == "/" {
            panic!("Nesting at the root is no longer supported. Use fallback_service instead.");
        }

        tap_inner!(self, mut this => {
            panic_on_err!(this.path_router.nest_service(path, service));
        })
    }

    /// 合并两个路由器
    ///
    /// 在 Spring Boot 中，这相当于：
    /// - 将多个 RouterFunction 链接起来
    /// - 合并多个 Controller 的路由
    ///
    /// 示例：
    /// ```rust
    /// use axum::{routing::get, Router};
    ///
    /// let users_router = Router::new().route("/users", get(users_handler));
    /// let posts_router = Router::new().route("/posts", get(posts_handler));
    ///
    /// // 相当于将两个 RouterFunction 合并
    /// let app = Router::new().merge(users_router).merge(posts_router);
    /// ```
    #[doc = include_str!("../docs/routing/merge.md")]
    #[track_caller]
    pub fn merge<R>(self, other: R) -> Self
    where
        R: Into<Self>,
    {
        let other: Self = other.into();
        let RouterInner {
            path_router,
            default_fallback,
            catch_all_fallback,
        } = other.into_inner();

        map_inner!(self, mut this => {
            // 处理 fallback 合并逻辑
            match (this.default_fallback, default_fallback) {
                // other 有默认 fallback，使用 other 的
                (_, true) => {}
                // this 有默认 fallback，other 有自定义 fallback
                (true, false) => {
                    this.default_fallback = false;
                }
                // 两者都有自定义 fallback，不允许
                (false, false) => {
                    panic!("Cannot merge two `Router`s that both have a fallback")
                }
            };

            panic_on_err!(this.path_router.merge(path_router));

            this.catch_all_fallback = this
                .catch_all_fallback
                .merge(catch_all_fallback)
                .unwrap_or_else(|| panic!("Cannot merge two `Router`s that both have a fallback"));

            this
        })
    }

    /// 添加中间件层
    ///
    /// 在 Spring Boot 中，这相当于：
    /// - Filter
    /// - Interceptor
    /// - HandlerFilterFunction
    ///
    /// 层应用于所有路由，包括 fallback
    ///
    /// 示例：
    /// ```rust
    /// use axum::{routing::get, Router};
    /// use tower::ServiceBuilder;
    /// use tower_http::trace::TraceLayer;
    ///
    /// let app = Router::new()
    ///     .route("/", get(|| async { "Hello" }))
    ///     .layer(TraceLayer::new_for_http());
    /// ```
    #[doc = include_str!("../docs/routing/layer.md")]
    pub fn layer<L>(self, layer: L) -> Self
    where
        L: Layer<Route> + Clone + Send + Sync + 'static,
        L::Service: Service<Request> + Clone + Send + Sync + 'static,
        <L::Service as Service<Request>>::Response: IntoResponse + 'static,
        <L::Service as Service<Request>>::Error: Into<Infallible> + 'static,
        <L::Service as Service<Request>>::Future: Send + 'static,
    {
        map_inner!(self, this => RouterInner {
            path_router: this.path_router.layer(layer.clone()),
            default_fallback: this.default_fallback,
            catch_all_fallback: this.catch_all_fallback.map(|route| route.layer(layer)),
        })
    }

    /// 添加路由层
    ///
    /// 与 `layer` 不同，这仅应用于路由，不应用于 fallback
    ///
    /// 在 Spring Boot 中，这相当于只在匹配的路径上应用 Filter
    #[doc = include_str!("../docs/routing/route_layer.md")]
    #[track_caller]
    pub fn route_layer<L>(self, layer: L) -> Self
    where
        L: Layer<Route> + Clone + Send + Sync + 'static,
        L::Service: Service<Request> + Clone + Send + Sync + 'static,
        <L::Service as Service<Request>>::Response: IntoResponse + 'static,
        <L::Service as Service<Request>>::Error: Into<Infallible> + 'static,
        <L::Service as Service<Request>>::Future: Send + 'static,
    {
        map_inner!(self, this => RouterInner {
            path_router: this.path_router.route_layer(layer),
            default_fallback: this.default_fallback,
            catch_all_fallback: this.catch_all_fallback,
        })
    }

    /// 如果路由器当前至少添加了一个路由，则返回 true
    #[must_use]
    pub fn has_routes(&self) -> bool {
        self.inner.path_router.has_routes()
    }

    /// 添加 fallback 处理器
    ///
    /// 在 Spring Boot 中，这相当于：
    /// - @ControllerAdvice + @ExceptionHandler 处理所有未匹配的请求
    /// - 实现 ErrorController
    ///
    /// 示例：
    /// ```rust
    /// use axum::{routing::get, Router};
    ///
    /// let app = Router::new()
    ///     .route("/", get(|| async { "Hello" }))
    ///     .fallback(|| async { "404 Not Found" });
    /// ```
    #[track_caller]
    #[doc = include_str!("../docs/routing/fallback.md")]
    pub fn fallback<H, T>(self, handler: H) -> Self
    where
        H: Handler<T, S>,
        T: 'static,
    {
        tap_inner!(self, mut this => {
            this.catch_all_fallback =
                Fallback::BoxedHandler(BoxedIntoRoute::from_handler(handler.clone()));
        })
        .fallback_endpoint(Endpoint::MethodRouter(any(handler)))
    }

    /// 添加 fallback 服务到路由器
    ///
    /// 更多详情请参阅 [`Router::fallback`]。
    ///
    /// 在 Spring Boot 中，这相当于自定义 ErrorController
    pub fn fallback_service<T>(self, service: T) -> Self
    where
        T: Service<Request, Error = Infallible> + Clone + Send + Sync + 'static,
        T::Response: IntoResponse,
        T::Future: Send + 'static,
    {
        let route = Route::new(service);
        tap_inner!(self, mut this => {
            this.catch_all_fallback = Fallback::Service(route.clone());
        })
        .fallback_endpoint(Endpoint::Route(route))
    }

    /// 添加"方法不允许"的 fallback 处理器
    ///
    /// 当路径匹配但 HTTP 方法不匹配时调用
    ///
    /// 在 Spring Boot 中，这相当于处理 HttpRequestMethodNotSupportedException
    #[doc = include_str!("../docs/routing/method_not_allowed_fallback.md")]
    #[allow(clippy::needless_pass_by_value)]
    pub fn method_not_allowed_fallback<H, T>(self, handler: H) -> Self
    where
        H: Handler<T, S>,
        T: 'static,
    {
        tap_inner!(self, mut this => {
            this.path_router
                .method_not_allowed_fallback(&handler);
        })
    }

    /// 将 fallback 重置为默认值
    ///
    /// 用于合并两个带有 fallback 的路由器，因为 [`merge`] 不允许
    /// 两个路由器都有显式的 fallback。使用此方法在合并之前删除
    /// 你想要丢弃的那个。
    ///
    /// [`merge`]: Self::merge
    pub fn reset_fallback(self) -> Self {
        tap_inner!(self, mut this => {
            this.default_fallback = true;
            this.catch_all_fallback = Fallback::Default(Route::new(NotFound));
        })
    }

    // 内部方法：设置 fallback 端点
    fn fallback_endpoint(self, endpoint: Endpoint<S>) -> Self {
        // TODO 改进这个实现。
        // 我们需要返回的 `Service` 是 `Clone` 的，并且 `service_fn` 中的函数是 `FnMut`，
        // 所以不仅仅是使用拥有的服务，我们用 `Option` 做这个技巧。我们知道这只会被调用一次，所以没问题。
        // 我们这样做是为了避免在 `oneshot_inner` 中克隆，这样 `Router` 以及随后的 `State` 不会被过度克隆。
        tap_inner!(self, mut this => {
            _ = this.path_router.route_endpoint(
                "/",
                endpoint.clone().layer(
                    layer_fn(
                        |service: Route| {
                            let mut service = Some(service);
                            service_fn(
                                #[cfg_attr(not(feature = "matched-path"), allow(unused_mut))]
                                move |mut request: Request| {
                                    #[cfg(feature = "matched-path")]
                                    request.extensions_mut().remove::<MatchedPath>();
                                    let route = take_route_or_internal_error(&mut service);
                                    route.oneshot_inner_owned(request)
                                }
                            )
                        }
                    )
                )
            );

            _ = this.path_router.route_endpoint(
                FALLBACK_PARAM_PATH,
                endpoint.layer(
                    layer_fn(
                        |service: Route| {
                            let mut service = Some(service);
                            service_fn(
                                #[cfg_attr(not(feature = "matched-path"), allow(unused_mut))]
                                move |mut request: Request| {
                                    #[cfg(feature = "matched-path")]
                                    request.extensions_mut().remove::<MatchedPath>();
                                    let route = take_route_or_internal_error(&mut service);
                                    route.oneshot_inner_owned(request)
                                }
                            )
                        }
                    )
                )
            );

            this.default_fallback = false;
        })
    }

    /// 提供状态给路由器
    ///
    /// 这将 `Router<S>` 转换为 `Router<S2>`，将类型 S 的状态注入到路由器中。
    ///
    /// 在 Spring Boot 中，这相当于：
    /// - 依赖注入：将 Bean 注入到 Controller
    /// - 设置 ApplicationContext
    ///
    /// 示例：
    /// ```rust
    /// use axum::{extract::State, routing::get, Router};
    /// use std::sync::Arc;
    ///
    /// struct AppState { /* ... */ }
    ///
    /// // 类似 Spring Boot 的 @Component + @Autowired
    /// let state = Arc::new(AppState { /* ... */ });
    /// let app = Router::new()
    ///     .route("/", get(handler))
    ///     .with_state(state(state));
    ///
    /// async fn handler(State(state): State<Arc<AppState>>) {
    ///     // 使用状态...
    /// }
    /// ```
    #[doc = include_str!("../docs/routing/with_state.md")]
    pub fn with_state<S2>(self, state: S) -> Router<S2> {
        map_inner!(self, this => RouterInner {
            path_router: this.path_router.with_state(state.clone()),
            default_fallback: this.default_fallback,
            catch_all_fallback: this.catch_all_fallback.with_state(state),
        })
    }

    // 内部方法：使用状态调用路由器
    pub(crate) fn call_with_state(&self, req: Request, state: S) -> RouteFuture<Infallible> {
        let (req, state) = match self.inner.path_router.call_with_state(req, state) {
            Ok(future) => return future,
            Err((req, state)) => (req, state),
        };

        self.inner
            .catch_all_fallback
            .clone()
            .call_with_state(req, state)
    }

    /// 将路由器转换为具有固定请求体类型的借用 [`Service`]，以辅助类型推断
    ///
    /// 在某些情况下，当从 [`tower::ServiceExt`] 调用 [`Router`] 上的方法时，你可能会得到类型推断错误
    ///
    /// 这主要在测试中使用 `Router` 时使用。通过 [`Router::into_make_service`] 正常运行 `Router` 时不应该需要它。
    pub fn as_service<B>(&mut self) -> RouterAsService<'_, B, S> {
        RouterAsService {
            router: self,
            _marker: PhantomData,
        }
    }

    /// 将路由器转换为具有固定请求体类型的拥有 [`Service`]，以辅助类型推断
    ///
    /// 这与 [`Router::as_service`] 相同，但它返回一个拥有所有权的 [`Service`]。
    /// 更多详情请参阅该方法。
    #[must_use]
    pub fn into_service<B>(self) -> RouterIntoService<B, S> {
        RouterIntoService {
            router: self,
            _marker: PhantomData,
        }
    }
}

// Router<()> 的额外方法实现（当没有状态时）
impl Router {
    /// 将此路由器转换为 [`MakeService`]，即一个 [`Service`]，其响应是另一个服务
    ///
    /// 在 Spring Boot 中，这相当于创建一个 Tomcat/Jetty 服务器实例并传入 DispatcherServlet
    ///
    /// 示例：
    /// ```
    /// use axum::{
ra///     routing::get,
    ///     Router,
    /// };
    ///
    /// let app = Router::new().route("/", get(|| async { "Hi!" }));
    ///
    /// # async {
    /// let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    /// // 类似于 SpringApplication.run()
    /// axum::serve(listener, app).await;
    /// # };
    /// ```
    ///
    /// [`MakeService`]: tower::make::MakeService
    #[must_use]
    pub fn into_make_service(self) -> IntoMakeService<Self> {
        // 调用 `Router::with_state` 以便所有内容都被急切地转换为 `Route`
        // 而不是每个请求都这样做
        IntoMakeService::new(self.with_state(()))
    }

    /// 将路由器转换为能够访问连接信息的 MakeService
    #[doc = include_str!("../docs/routing/into_make_service_with_connect_info.md")]
    #[cfg(feature = "tokio")]
    #[must_use]
    pub fn into_make_service_with_connect_info<C>(self) -> IntoMakeServiceWithConnectInfo<Self, C> {
        // 调用 `Router::with_state` 以便所有内容都被急切地转换为 `Route`
        // 而不是每个请求都这样做
        IntoMakeServiceWithConnectInfo::new(self.with_state(()))
    }
}

// 为 Router<()> 实现 Service<IncomingStream>
// 这使得 axum::serve(listener, router) 可以工作
#[cfg(all(feature = "tokio", any(feature = "http1", feature = "http2")))]
const _: () = {
    use crate::serve;

    impl<L> Service<serve::IncomingStream<'_, L>> for Router<()>
    where
        L: serve::Listener,
    {
        type Response = Self;
        type Error = Infallible;
        type Future = std::future::Ready<Result<Self::Response, Self::Error>>;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, _req: serve::IncomingStream<'_, L>) -> Self::Future {
            // 调用 `Router::with_state` 以便所有内容都被急切地转换为 `Route`
            // 而不是每个请求都这样做
            std::future::ready(Ok(self.clone().with_state(())))
        }
    }
};

// 为 Router<()> 实现 Service<Request<B>>
impl<B> Service<Request<B>> for Router<()>
where
    B: HttpBody<Data = bytes::Bytes> + Send + 'static,
    B::Error: Into<axum_core::BoxError>,
{
    type Response = Response;
    type Error = Infallible;
    type Future = RouteFuture<Infallible>;

    #[inline]
    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    #[inline]
    fn call(&mut self, req: Request<B>) -> Self::Future {
        let req = req.map(Body::new);
        self.call_with_state(req, ())
    }
}

/// 转换为具有固定体类型的借用 [`Service`] 的 [`Router`]
///
/// 更多详情请参阅 [`Router::as_service`]。
pub struct RouterAsService<'a, B, S = ()> {
    router: &'a mut Router<S>,
    _marker: PhantomData<fn(B)>,
}

// 为 RouterAsService 实现 Service
impl<B> Service<Request<B>> for RouterAsService<'_, B, ()>
where
    B: HttpBody<Data = bytes::Bytes> + Send + 'static,
    B::Error: Into<axum_core::BoxError>,
{
    type Response = Response;
    type Error = Infallible;
    type Future = RouteFuture<Infallible>;

    #[inline]
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        <Router as Service<Request<B>>>::poll_ready(self.router, cx)
    }

    #[inline]
    fn call(&mut self, req: Request<B>) -> Self::Future {
        self.router.call(req)
    }
}

// RouterAsService 的 Debug 实现
impl<B, S> fmt::Debug for RouterAsService<'_, B, S>
where
    S: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RouterAsService")
            .field("router", &self.router)
            .finish()
    }
}

/// 转换为具有固定体类型的拥有 [`Service`] 的 [`Router`]
///
/// 更多详情请参阅 [`Router::into_service`]。
pub struct RouterIntoService<B, S = ()> {
    router: Router<S>,
    _marker: PhantomData<fn(B)>,
}

// RouterIntoService 的 Clone 实现
impl<B, S> Clone for RouterIntoService<B, S>
where
    Router<S>: Clone,
{
    fn clone(&self) -> Self {
        Self {
            router: self.router.clone(),
            _marker: PhantomData,
        }
    }
}

// 为 RouterIntoService 实现 Service
impl<B> Service<Request<B>> for RouterIntoService<B, ()>
where
    B: HttpBody<Data = bytes::Bytes> + Send + 'static,
    B::Error: Into<axum_core::BoxError>,
{
    type Response = Response;
    type Error = Infallible;
    type Future = RouteFuture<Infallible>;

    #[inline]
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        <Router as Service<Request<B>>>::poll_ready(&mut self.router, cx)
    }

    #[inline]
    fn call(&mut self, req: Request<B>) -> Self::Future {
        self.router.call(req)
    }
}

// RouterIntoService 的 Debug 实现
impl<B, S> fmt::Debug for RouterIntoService<B, S>
where
    S: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RouterIntoService")
            .field("router", &self.router)
            .finish()
    }
}

// Fallback 枚举：表示不同类型的 fallback 处理器
enum Fallback<S, E = Infallible> {
    Default(Route<E>),              // 默认的 404 fallback
    Service(Route<E>),             // 自定义 Service fallback
    BoxedHandler(BoxedIntoRoute<S, E>), // 盒装的处理器 fallback
}

// Fallback 的方法实现
impl<S, E> Fallback<S, E>
where
    S: Clone,
{
    // 合并两个 fallback
    fn merge(self, other: Self) -> Option<Self> {
        match (self, other) {
            // 如果任一个是 `Default`，返回另一个
            (Self::Default(_), pick) | (pick, Self::Default(_)) => Some(pick),
            // 否则，返回 None（两个自定义 fallback 不能合并）
            _ => None,
        }
    }

    // 对 fallback 应用映射函数
    fn map<F, E2>(self, f: F) -> Fallback<S, E2>
    where
        S: 'static,
        E: 'static,
        F: FnOnce(Route<E>) -> Route<E2> + Clone + Send + Sync + 'static,
        E2: 'static,
    {
        match self {
            Self::Default(route) => Fallback::Default(f(route)),
            Self::Service(route) => Fallback::Service(f(route)),
            Self::BoxedHandler(handler) => Fallback::BoxedHandler(handler.map(f)),
        }
    }

    // 使用状态调用 fallback
    fn with_state<S2>(self, state: S) -> Fallback<S2, E> {
        match self {
            Self::Default(route) => Fallback::Default(route),
            Self::Service(route) => Fallback::Service(route),
            Self::BoxedHandler(handler) => Fallback::Service(handler.into_route(state)),
        }
    }

    // 使用状态调用 fallback 处理请求
    fn call_with_state(self, req: Request, state: S) -> RouteFuture<E> {
        match self {
            Self::Default(route) | Self::Service(route) => route.oneshot_inner_owned(req),
            Self::BoxedHandler(handler) => {
                let route = handler.into_route(state);
                route.oneshot_inner_owned(req)
            }
        }
    }

    // 检查是否是默认 fallback
    fn is_default(&self) -> bool {
        matches!(self, Self::Default(..))
    }
}

// Fallback 的 Clone 实现
impl<S, E> Clone for Fallback<S, E> {
    fn clone(&self) -> Self {
        match self {
            Self::Default(inner) => Self::Default(inner.clone()),
            Self::Service(inner) => Self::Service(inner.clone()),
            Self::BoxedHandler(inner) => Self::BoxedHandler(inner.clone()),
        }
    }
}

// Fallback 的 Debug 实现
impl<S, E> fmt::Debug for Fallback<S, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Default(inner) => f.debug_tuple("Default").field(inner).finish(),
            Self::Service(inner) => f.debug_tuple("Service").field(inner).finish(),
            Self::BoxedHandler(_) => f.debug_tuple("BoxedHandler").finish(),
        }
    }
}

// Endpoint 枚举：表示路由端点的类型
#[allow(clippy::large_enum_variant)]
enum Endpoint<S> {
    MethodRouter(MethodRouter<S>),  // HTTP 方法路由器
    Route(Route),                  // 路由服务
}

// Endpoint 的方法实现
impl<S> Endpoint<S>
where
    S: Clone + Send + Sync + 'static,
{
    // 对 endpoint 应用层
    fn layer<L>(self, layer: L) -> Self
    where
        L: Layer<Route> + Clone + Send + Sync + 'static,
        L::Service: Service<Request> + Clone + Send + Sync + 'static,
        <L::Service as Service<Request>>::Response: IntoResponse + 'static,
        <L::Service as Service<Request>>::Error: Into<Infallible> + 'static,
        <L::Service as Service<Request>>::Future: Send + 'static,
    {
        match self {
            Self::MethodRouter(method_router) => Self::MethodRouter(method_router.layer(layer)),
            Self::Route(route) => Self::Route(route.layer(layer)),
        }
    }
}

// Endpoint 的 Clone 实现
impl<S> Clone for Endpoint<S> {
    fn clone(&self) -> Self {
        match self {
            Self::MethodRouter(inner) => Self::MethodRouter(inner.clone()),
            Self::Route(inner) => Self::Route(inner.clone()),
        }
    }
}

// Endpoint 的 Debug 实现
impl<S> fmt::Debug for Endpoint<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MethodRouter(method_router) => {
                f.debug_tuple("MethodRouter").field(method_router).finish()
            }
            Self::Route(route) => f.debug_tuple("Route").field(route).finish(),
        }
    }
}

#[test]
fn traits() {
    use crate::test_helpers::*;
    assert_send::<Router<()>>();
    assert_sync::<Router<()>>();
    assert_send::<RouterAsService<'static, Body, ()>>();
    assert_sync::<RouterAsService<'static, Body, ()>>();
    assert_send::<RouterIntoService<Body, ()>>();
    assert_sync::<RouterIntoService<Body, ()>>();
}
