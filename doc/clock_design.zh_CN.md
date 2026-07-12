# Clock 架构设计

当前架构以四个相互隔离的能力为核心：`WallClock`、`MonotonicClock`、`BlockingSleeper` 和 `AsyncSleeper`。

完整设计、类型关系、错误模型、构造方式和下游接入方案见：

- [重构设计方案](clock_refactoring_design.zh_CN.md)
- [破坏性重构实施计划](clock_refactoring_implementation_plan.zh_CN.md)

## 核心约束

- Wall time 和 monotonic time 使用不同 trait 与 concrete 类型。
- Sleeper 显式持有 `Arc<ConcreteMonotonicClock>`，不维护独立时间状态。
- `MonotonicInstant` 携带 `u64 domain_id`，不同 domain 的 instant 不能混用。
- Concrete monotonic clock 不通过 `Clone` 隐式共享；共享必须使用 `Arc::clone`。
- Manual wall clock、blocking sleeper 和 async sleeper 可以共享同一个 `Arc<ManualMonotonicClock>`。
- 默认构建不依赖 Tokio、chrono 或时区库。
