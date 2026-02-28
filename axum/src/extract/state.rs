//! 应用状态提取器模块
//!
//! 在 Spring Boot 中，这相当于：
//! - @Autowired - 依赖注入
//! - ApplicationContext - 应用上下文
//! - @Component - 注入的 Bean
//!
//! State 提取器允许在处理器中访问应用程序全局状态，如：
//! - 数据库连接池
//! - 配置对象
//! - 共享服务客户端
//! - 缓存实例

use axum_core::extract::{FromRef, FromRequestParts};
use http::request::Parts;
use std::{
    convert::Infallible,
    ops::{Deref, DerefMut},
};

/// 状态提取器
///
/// 用于访问在 Router 上通过 `with_state` 方法提供的应用状态。
///
/// 在 Spring Boot 中，这相当于：
//! - `@Autowired private MyService service;` - 自动注入依赖
//! - `@Value private String configValue;` - 注入配置值
//!
/// State 是全局的，在接收状态的路由器的每个请求中使用。
/// 对于访问从请求派生的数据（如授权数据），请参阅 [`Extension`]。
///
/// 更多信息请参阅 ["在中间件中访问状态"][state-from-middleware]
//!
/// [state-from-middleware]: crate::middleware#accessing-state-in-middleware
//! [`Extension`]: crate::Extension
///
/// ## 与 `Router` 一起使用
///
//! ```rust
//! use axum::{Router, routing::get, extract::State};
//!
//! // 应用状态
//! //
//! // 在这里你可以放入配置、数据库连接池，或任何你需要的
//! // 状态
//! #[derive(Clone)]
//! struct AppState {}
//!
//! let state = AppState {};
//!
//! // 创建一个保存我们状态的 `Router`
//! // 类似于 Spring Boot 的依赖注入容器
//! let app = Router::new()
//!     .route("/", get(handler))
//!     // 提供状态以便路由器可以访问它
//!     .with_state(state);
//!
//! async fn handler(
//!     // 通过 `State` 提取器访问状态
//!     // 提取错误类型的 state 会导致编译错误
//!     // 类似于 Spring Boot 的类型安全的依赖注入
//!     State(state): State<AppState>,
//! ) {
//!     // 使用 `state`...
//! }
//! # let _: axum::Router = app;
//! ```
///
/// 注意 `State` 是一个提取器，所以确保将它放在任何 body 提取器之前，
//! 请参阅 ["提取器的顺序"][order-of-extractors]。
///
/// [order-of-extractors]: crate::extract#the-order-of-extractors
///
/// ## 组合带状态的路由器
///
//! 多个 [`Router`] 可以使用 [`Router::nest`] 或 [`Router::merge`] 组合
//! 当使用这些方法之一组合 [`Router`] 时，[`Router`] 必须具有相同的状态类型。
//! 通常，这可以自动推断：
//!
//! ```rust
//! use axum::{Router, routing::get, extract::State};
//!
//! #[derive(Clone)]
//! struct AppState {}
//!
//! let state = AppState {};
//!
//! // 创建一个将嵌套在另一个中的 `Router`
//! let api = Router::new()
//!     .route("/posts", get(posts_handler));
//!
//! let app = Router::new()
//!     .nest("/api", api)
//!     .with_state(state);
//!
//! async fn posts_handler(State(state): State<AppState>) {
//!     // 使用 `state`...
//! }
//! # let _: axum::Router = app;
//! ```
///
/// 然而，如果你正在组合在不同作用域中定义的 [`Router`]，
//! 你可能需要显式注释 [`State`] 类型：
//!
//! ```rust
//! use axum::{Router, routing::get, extract::State};
//!
//! #[derive(Clone)]
//! struct AppState {}
//!
//! fn make_app() -> Router {
//!     let state = AppState {};
//!
//!     Router::new()
//!         .nest("/api", make_api())
//!         .with_state(state) // 外部 Router 的状态被推断
//! }
//!
//! // 内部 Router 必须指定其状态类型以与外部路由器组合
//! fn make_api() -> Router<AppState> {
//!     Router::new()
//!         .route("/posts", get(posts_handler))
//! }
//!
//! async fn posts_handler(State(state): State<AppState>) {
//!     // 使用 `state`...
//! }
//! # let _: axum::Router = make_app();
//! ```
///
/// 简而言之，[`Router`] 的泛型状态类型默认为 `()`（无状态），
//! 除非调用 [`Router::with_state`] 或显式给定泛型类型的值。
///
/// [`Router`]: crate::Router
/// [`Router::merge`]: crate::Router::merge
/// [`Router::nest`]: crate::Router::nest
/// [`Router::with_state`]: crate::Router::with_state
///
/// ## 与 `MethodRouter` 一起使用
///
//! ```rust
//! use axum::{routing::get, extract::State};
//!
//! #[derive(Clone)]
//! struct AppState {}
//!
//! let state = AppState {};
//!
//! let method_router_with_state = get(handler)
//!     // 提供状态以便处理器可以访问它
//!     .with_state(state);
//! # let _: axum::routing::MethodRouter = method_router_with_state;
//!
//! async fn handler(State(state): State<AppState>) {
//!     // 使用 `state`...
//! }
//! ```
///
/// ## 与 `Handler` 一起使用
///
//! ```rust
//! use axum::{routing::get, handler::Handler, extract::State};
//!
//! #[derive(Clone)]
//! struct AppState {}
//!
//! let state = AppState {};
//!
//! async fn handler(State(state): State<AppState>) {
//!     // 使用 `state`...
//! }
//!
//! // 提供状态以便处理器可以访问它
//! let handler_with_state = handler.with_state(state);
//!
//! # async {
//! let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
//! axum::serve(listener, handler_with_state.into_make_service()).await;
//! # };
//! ```
///
/// ## 子状态
///
//! [`State`] 只允许单个状态类型，但你可以使用 [`FromRef`] 提取"子状态"：
//!
//! ```rust
//! use axum::{Router, routing::get, extract::{State, FromRef}};
//!
//! // 应用状态
//! #[derive(Clone)]
//! struct AppState {
//!     // 持有一些 API 特定的状态
//!     api_state: ApiState,
//! }
//!
//! // API 特定状态
//! #[derive(Clone)]
//! struct ApiState {}
//!
//! // 支持将 `AppState` 转换为 `ApiState`
//! impl FromRef<AppState> for ApiState {
//!     fn from_ref(app_state: &AppState) -> ApiState {
//!         app_state.api_state.clone()
//!     }
//! }
//!
//! let state = AppState {
//!     api_state: ApiState {},
//! };
//!
//! let app = Router::new()
//!     .route("/", get(handler))
//!     .route("/api/users", get(api_users))
//!     .with_state(state);
//!
//! async fn api_users(
//!     // 访问 API 特定状态
//!     // 类似于 @Qualifier 指定的 Bean
//!     State(api_state): State<ApiState>,
//! ) {
//! }
//!
//! async fn handler(
//!     // 我们仍然可以访问顶层状态
//!     State(state): State<AppState>,
//! ) {
//! }
//! # let _: axum::Router = app;
//! ```
///
/// 为了方便，`FromRef` 也可以使用 `#[derive(FromRef)]` 派生。
///
/// ## 对于库作者
///
//! 如果你正在编写一个具有需要状态的提取器的库，
//! 这是推荐的做法：
///
//! ```rust
//! use axum_core::extract::{FromRequestParts, FromRef};
//! use http::request::Parts;
//! use std::convert::Infallible;
//!
//! // 你的库提供的提取器
//! struct MyLibraryExtractor;
//!
//! impl<S> FromRequestParts<S> for MyLibraryExtractor
//! where
//!     // 保持 `S` 泛型但要求它可以产生 `MyLibraryState`
//!     // 这意味着用户将必须实现 `FromRef<UserState> for MyLibraryState`
//!     MyLibraryState: FromRef<S>,
//!     S: Send + Sync,
//! {
//!     type Rejection = Infallible;
//!
//!     async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
//!         // 从状态的引用获取 `MyLibraryState`
//!         let state = MyLibraryState::from_ref(state);
//!
//!         // ...
//!         # todo!()
//!     }
//! }
//!
//! // 你的库需要的状态
//! struct MyLibraryState {
//!     // ...
//! }
//! ```
///
/// ## 共享可变状态
///
//! [由于状态在 `Router` 中是全局的][global]，你不能直接获得状态的可变引用。
//!
//! 最基本的解决方案是使用 `Arc<Mutex<_>>`。你需要哪种互斥锁取决于你的用例。
//! 更多细节请参阅 [tokio 文档]。
///
/// 注意：在 `.await` 点上持有锁定的 `std::sync::Mutex` 将导致 `!Send` future，
//! 这与 axum 不兼容。如果你需要在 `.await` 点上持有互斥锁，请考虑使用 `tokio::sync::Mutex` 代替。
///
//! 在 Spring Boot 中，这相当于使用 @Scope("request") 的 Bean 或线程安全的共享状态
///
/// ## 示例
///
//! ```rust
//! use axum::{Router, routing::get, extract::State};
//! use std::sync::{Arc, Mutex};
//!
//! #[derive(Clone)]
//! struct AppState {
//!     data: Arc<Mutex<String>>,
//! }
//!
//! async fn handler(State(state): State<AppState>) {
//!     {
//!         let mut data = state.data.lock().expect("mutex was poisoned");
//!         *data = "updated foo".to_owned();
//!     }
//!
//!     // ...
//! }
//!
//! let state = AppState {
//!     data: Arc::new(Mutex::new("foo".to_owned())),
//! };
//!
//! let app = Router::new()
//!     .route("/", get(handler))
//!     .with_state(state);
//! # let _: Router = app;
//! ```
//!
/// [global]: crate::Router::with_state
/// [the tokio docs]: https://docs.rs/tokio/1.25.0/tokio/sync/struct.Mutex.html#which-kind-of-mutex-should-you-use
#[derive(Debug, Default, Clone, Copy)]
pub struct State<S>(pub S);

// 为 State 实现 FromRequestParts
// 这使得 State 可以用作处理器参数
impl<OuterState, InnerState> FromRequestParts<OuterState> for State<InnerState>
where
    InnerState: FromRef<OuterState>,
    OuterState: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request_parts(
        _parts: &mut Parts,
        state: &OuterState,
    ) -> Result<Self, Self::Rejection> {
        // 使用 FromRef 特质从外部状态获取内部状态
        let inner_state = InnerState::from_ref(state);
        Ok(Self(inner_state))
    }
}

// 为 State 实现 Deref，允许像引用一样使用
impl<S> Deref for State<S> {
    type Target = S;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// 为 State 实现 DerefMut，允许像可变引用一样使用
impl<S> DerefMut for State<S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
