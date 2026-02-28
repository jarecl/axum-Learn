## 调试处理器类型错误

要使函数作为处理器使用，它必须实现 [`Handler`] trait。axum 为以下类型的函数提供了 blanket 实现：

- 是 `async fn` 的函数
- 接受不超过 16 个参数，且所有参数都实现 `Send`
  - 除最后一个参数外的所有参数都实现 [`FromRequestParts`]
  - 最后一个参数实现 [`FromRequest`]
- 返回一个实现 [`IntoResponse`] 的类型
- 如果使用闭包，它必须实现 `Clone + Send` 并且是 `'static` 的
- 返回一个 `Send` 的 future。最常见的不小心让 future 变成 `!Send` 的方式是在 await 跨越持有一个 `!Send` 类型

不幸的是，如果你尝试使用一个不完全符合 [`Handler`] 要求的函数，Rust 会给出糟糕的错误消息。

你可能会遇到这样的错误：

```not_rust
error[E0277]: the trait bound `fn(bool) -> impl Future {handler}: Handler<_, _>` is not satisfied
   --> src/main.rs:13:44
    |
13  |     let app = Router::new().route("/", get(handler));
    |                                            ^^^^^^^        trait `Handler<_, _>` is not implemented for `fn(bool) -> impl Future {handler}`
    |
   ::: axum/src/handler/mod.rs:116:8
    |
116 |     H: Handler<T, B>,
    |        ------------- required by this bound in `axum::routing::get`
```

这个错误没有告诉你 _为什么_ 你的函数没有实现 [`Handler`]。可以使用 [axum-macros] crate 中的 [`debug_handler`] 过程宏来改进错误。

[axum-macros]: https://docs.rs/axum-macros
[`debug_handler`]: https://docs.rs/axum-macros/latest/axum_macros/attr.debug_handler.html
