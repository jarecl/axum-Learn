在 axum 中，"处理器"是一个 async 函数，它接受零个或多个["提取器"](crate::extract) 作为参数，并返回可以[转换为响应](crate::response)的东西。

处理器是你的应用程序逻辑所在的地方，axum 应用程序通过在处理器之间路由构建。

[`debug_handler`]: https://docs.rs/axum-macros/latest/axum_macros/attr.debug_handler.html
