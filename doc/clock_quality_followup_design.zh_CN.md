# `rs-clock` 质量收口设计

## 目标

在不改变四类时钟能力和 manual driver 架构的前提下，收紧公开 API 的防误用能力，修正文档契约，补齐 waiter 边界测试，并让 CI 覆盖所有 feature 的公开文档。

## 方案选择

采用最小化增强方案：

- 保留 `WallClock`、`MonotonicClock`、`BlockingSleeper` 和 `AsyncSleeper` 的现有边界。
- 保留 `ManualMonotonicClock` 的单 mutex 状态模型，不为文件长度拆分并发状态。
- 保留 object-safe 的 boxed `SleepFuture`，不引入 GAT 或额外适配层。
- 仅修改已确认的 API 属性、文档、测试和 CI 命令。

未采用以下方案：

- 不重构 manual waiter 注册表；当前线性扫描适合测试规模，重构会增加取消和并发推理成本。
- 不改变 Tokio future 的惰性 timer 创建行为；该行为允许在 runtime 外创建 future、在 runtime 内首次 poll。
- 不新增依赖或测试框架。

## API 变更

1. `ManualAdvanceSubscription` 增加 `#[must_use]`，避免返回值被忽略后订阅立即注销。
2. `TimeError` 增加 `#[non_exhaustive]`，为 1.0 后新增错误保留兼容空间。
3. `ManualBlockingSleeper::sleep_until()` 删除重复 domain 校验，由 `ManualMonotonicClock::wait_until_blocking()` 统一校验。

## 文档变更

- 明确 Tokio timer 在返回 future 首次 poll 时创建，而不是在 `sleep_until_async()` 调用时创建。
- 在所有触发 advance callback 的推进方法上声明 callback panic 会被恢复。
- 明确 manual async waiter 和 waiter observer 的注册时点。
- 修正 subscription drop 的措辞和 `ManualWallClock` 注释中的重复单词。
- 增加可运行的 crate 级 manual clock doctest。

## 测试与 CI

- 用集成测试锁定 `#[must_use]` 和 `#[non_exhaustive]` 属性，并用普通
  doctest 展示 `TimeError` 的 wildcard 匹配方式。
- 增加 `wait_for_waiters_async()` 的零目标、已满足目标和 blocking waiter 唤醒测试。
- 增加 Tokio sleep future 可在 runtime 外创建的回归测试。
- CI rustdoc 使用 `--all-features`，并以 `-D missing-docs` 拒绝公开文档缺失。
- 完整运行 `rs-clock` CI，并运行 `rs-lock`、`rs-retry` 的 all-feature 测试验证下游兼容性。

## 成功标准

- 新增的属性契约测试在属性加入前按预期失败，加入后通过。
- 所有现有和新增测试通过。
- Clippy、格式、定制 style、rustdoc、package、coverage 和 audit 全部通过。
- `rs-lock` 与 `rs-retry` 使用路径依赖时测试通过。
- 代码知识图谱重新索引后能找到本轮新增 API 和测试。
