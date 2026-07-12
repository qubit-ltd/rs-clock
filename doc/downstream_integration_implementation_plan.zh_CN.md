# rs-clock 下游接入实施计划

> 本计划在已确认的 `clock_refactoring_design.zh_CN.md` 基础上执行。按用户要求不提交 Git commit；每个 crate 独立完成格式化与 CI 验证。

**目标：** 完善 `qubit-clock` 的异步与订阅契约，并通过相对路径依赖把它接入 `qubit-lock` 和 `qubit-retry`。

**架构：** `qubit-lock::MockMonitor` 共享一个 `Arc<ManualMonotonicClock>`，不再维护第二份 elapsed time；clock advance 通过订阅唤醒 blocking Condvar 和 async watch。`qubit-retry::Retry` 持有可注入的 blocking/async sleeper，三个 runner 使用 sleeper 的 monotonic domain 计算 elapsed，sync/async retry delay 由 sleeper 驱动，async attempt timeout 通过 operation future 与 sleep future 竞争实现。

**技术栈：** Rust 1.94、edition 2024、标准库同步原语、可选 Tokio、相对路径 Cargo dependency。

## 全局约束

- 不修改 rs-clock 之外的时间语义，生产实现仍分别使用 std 和 Tokio timer。
- `rs-lock` 和 `rs-retry` 通过 `path = "../rs-clock"` 引用尚未发布的 `qubit-clock 0.9`。
- 每个行为修改遵循 RED-GREEN-REFACTOR；测试统一放在各 crate 的 `tests/` 目录。
- 不执行 `git add`、`git commit` 或 `git push`。
- 每个 crate 完成后依次运行 `./align-ci.sh` 和 `./ci-check.sh`。

---

### Task 1：完善 rs-clock 契约

**文件：**

- 修改：`src/sleep/async_sleeper.rs`
- 修改：`src/sleep/manual_async_sleeper.rs`
- 修改：`src/sleep/tokio_async_sleeper.rs`
- 修改：`src/monotonic/manual_monotonic_clock.rs`
- 修改：`src/monotonic/manual_monotonic_state.rs`
- 修改：`tests/sleep/async_sleeper_tests.rs`
- 修改：`tests/monotonic/manual_advance_subscription_tests.rs`
- 修改：`README.md`
- 修改：`README.zh_CN.md`
- 修改：`doc/clock_refactoring_design.zh_CN.md`

**接口：**

- `AsyncSleeper::sleep_until_async` 和 `sleep_for_async` 返回 `SleepFuture<'static>`。
- `subscribe_advances` 保持现有签名，但明确 synchronous callback、并发、panic、in-flight drop 和 lock-order 契约。
- 删除没有消费者的内部 `revision` 状态。

- [x] 先增加 `'static` future 编译契约测试并确认旧签名编译失败。
- [x] 修改 async sleeper trait 及两个实现并确认测试通过。
- [x] 增加 subscription callback 可重入读取 clock 的测试，确认 callback 在 clock mutex 外执行。
- [x] 删除未使用 revision，更新 README 和设计文档。
- [x] 运行 `./align-ci.sh` 和 `./ci-check.sh`。

### Task 2：将 rs-lock MockMonitor 接入 ManualMonotonicClock

**文件：**

- 修改：`Cargo.toml`
- 修改：`src/monitor/mock_monitor.rs`
- 修改：`src/monitor/arc_mock_monitor.rs`
- 修改：`tests/monitor/mock_monitor_tests.rs`
- 修改：`tests/monitor/arc_mock_monitor_tests.rs`
- 修改：`README.md`
- 修改：`README.zh_CN.md`

**接口：**

- `MockMonitor::from_clock(state, Arc<ManualMonotonicClock>) -> Self`。
- `ArcMockMonitor::from_clock(state, Arc<ManualMonotonicClock>) -> Self`。
- `monotonic_clock(&self) -> &ManualMonotonicClock` 只暴露控制引用，不隐式 clone 共享对象。
- `new`、`From`、`Default` 创建独立 manual clock，作为便利构造。
- 删除 monitor 自己的 elapsed 字段以及 `set_elapsed`、`reset_elapsed`、`advance` 转发 API；测试通过显式 clock 调用 `advance`。

- [x] 先改测试为共享 clock 驱动 blocking/async timeout，确认缺少新构造 API 时失败。
- [x] 添加相对路径依赖并实现 clock subscription。
- [x] 把所有 timeout deadline 和到期判断切换到 monotonic clock。
- [x] 覆盖 blocking、async、notification 与 timeout 竞争、共享 clock 驱动多个 monitor。
- [x] 更新 README，运行 `./align-ci.sh` 和 `./ci-check.sh`。

### Task 3：将 rs-retry elapsed 与 delay 接入 sleeper

**文件：**

- 修改：`Cargo.toml`
- 修改：`src/executor/retry.rs`
- 修改：`src/executor/retry_builder.rs`
- 修改：`src/executor/retry_flow_state.rs`
- 修改：`src/executor/retry_failure_handler.rs`
- 修改：`src/executor/retry_runner.rs`
- 修改：`src/executor/async_retry_runner.rs`
- 修改：`src/executor/worker_retry_runner.rs`
- 修改：相关 `tests/executor/*_tests.rs`
- 修改：`README.md`
- 修改：`README.zh_CN.md`

**接口：**

- `RetryBuilder::blocking_sleeper(Arc<dyn BlockingSleeper>) -> Self`。
- Tokio feature 下提供 `RetryBuilder::async_sleeper(Arc<dyn AsyncSleeper>) -> Self`。
- 默认分别构造 `StdBlockingSleeper` 和 `TokioAsyncSleeper`。
- `RetryFlowState` 保存 domain-scoped start instant 和相应 monotonic clock 引用。
- async attempt timeout 使用 `tokio::select!` 竞争 operation 与 injected async sleeper。
- worker-thread 的原生 `recv_timeout`/cancellation-grace 仍由 OS 同步原语管理；其 elapsed accounting 和 retry backoff 使用 injected blocking sleeper，并在文档中明确边界。

- [x] 先增加 manual blocking exponential backoff 测试，确认现实现实等待或缺少 builder API。
- [x] 增加 manual async backoff 和 attempt-timeout 测试，确认缺少 async sleeper 注入时失败。
- [x] 实现 builder/default driver、domain-scoped RetryFlowState 和 runner 注入。
- [x] 更新现有测试构造及文档，明确 worker native timeout 边界。
- [x] 运行 `./align-ci.sh` 和 `./ci-check.sh`。

### Task 4：跨 crate 最终核对

- [x] 核对三个仓库的 `git status`，确认没有修改其他 crate。
- [x] 重新运行三个 crate 的完整 `ci-check.sh`，记录退出码。
- [x] 对照本计划核对公开 API、测试场景、相对路径依赖和文档。

## 验证结果

- `rs-clock`：`align-ci.sh` 与 `ci-check.sh` 全部通过（11/11）。
- `rs-lock`：格式、Clippy、构建、349 个集成测试、75 个 doctest、feature
  matrix、覆盖率和安全审计通过。`cargo package` 因 crates.io 尚无
  `qubit-clock 0.9` 而按 Cargo 规则停止。
- `rs-retry`：格式、Clippy、构建、157 个集成测试、文档、feature matrix、
  覆盖率和安全审计通过。`cargo package` 同样等待 `qubit-clock 0.9` 发布。
- 两个下游 manifest 同时保留 `version = "0.9"` 与 `path = "../rs-clock"`：
  本地构建使用相对路径，发布包在上游发布后自动使用 crates.io 版本。
