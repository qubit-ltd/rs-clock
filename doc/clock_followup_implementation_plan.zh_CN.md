# `rs-clock` 第三轮收口实施计划

> **执行要求：** 逐项执行 RED → GREEN → REFACTOR；不进行 Git 提交。

**目标：** 修复 manual waiter observer 竞态，统一 manual driver API，并让
`rs-retry`、`rs-lock` 的测试协调不再依赖轮询或真实 sleep 猜测。

**架构：** `ManualMonotonicClock` 是唯一 manual sleeper driver；blocking 和
async sleeper 只提供等待能力。`rs-lock::MockMonitor` 维护自己的 active timeout
waiter 注册屏障，不把 `rs-clock` 扩张成通用调度器。

**技术栈：** Rust 2024、标准库 `Mutex`/`Condvar`/`Future`/`Waker`、可选 Tokio。

## 全局约束

- 允许破坏性 API 变更，不保留旧接口兼容性。
- 生产源码一个核心类型一个 snake_case 文件；测试镜像源码目录并使用
  `_tests.rs` 后缀。
- 所有行为修改先增加失败测试并确认 RED。
- 每个修改过的 crate 最后运行 `align-ci.sh` 和 `ci-check.sh`。

---

### Task 1：锁存 waiter observer

**文件：**

- 修改：`src/monotonic/manual_monotonic_state.rs`
- 修改：`src/monotonic/manual_monotonic_clock.rs`
- 测试：`tests/monotonic/manual_waiter_future_tests.rs`

**接口：** `wait_for_waiters_async()` 一旦观察到 waiter count 达标便永久 ready。

- [x] 增加“waiter 注册后在 observer 再次 poll 前注销”的失败测试。
- [x] 运行 `cargo test --all-features --test monotonic_tests
  test_manual_waiter_future_latches_reached_count_before_waiter_drops`，确认返回
  `Pending` 的预期失败。
- [x] 在 waiter 注册的同一 state lock 内移除已满足 observer，并在锁外唤醒其
  waker；observer 后续 poll 看到注册缺失时返回 `Ready(())`。
- [x] 运行全部 monotonic 测试确认 GREEN。

### Task 2：统一 manual driver API

**文件：**

- 修改：`src/monotonic/manual_monotonic_clock.rs`
- 修改：`src/sleep/manual_async_sleeper.rs`
- 修改：`src/sleep/manual_blocking_sleeper.rs`
- 测试：`tests/monotonic/manual_monotonic_clock_tests.rs`
- 测试：`tests/monotonic/manual_advance_subscription_tests.rs`
- 修改所有调用旧 sleeper helper 的测试和下游代码。

**接口：**

```rust
clock.subscribe_advances(callback);
clock.wait_for_waiters(expected_count, real_timeout);
clock.wait_for_waiters_async(expected_count);
clock.advance_to_next_deadline() -> Option<MonotonicInstant>;
```

- [x] 先把测试改成上述 API 并确认编译失败。
- [x] 将 blocking/async waiter 的公共统计和协调入口移到 clock。
- [x] 删除两个 manual sleeper 上的 `pending_waiters()`、`next_deadline()` 和
  `wait_for_waiters()`。
- [x] 让所有 next-deadline 查询忽略已到期、尚未清理的 registration。
- [x] 运行 rs-clock monotonic、sleep 和 scenario 测试确认 GREEN。

### Task 3：收紧 ManualWallClock 并发快照

**文件：**

- 修改：`src/wall/manual_wall_clock.rs`
- 测试：`tests/wall/manual_wall_clock_tests.rs`

**接口：** `now()` 和 `reanchor()` 在 anchor mutex 内采样同一 manual monotonic
clock，避免旧 anchor 与新 elapsed 混合。

- [x] 增加并发 `reanchor/advance/now` 回归测试，验证返回值只来自有效映射。
- [x] 确认测试在旧实现上能暴露混合快照。
- [x] 调整锁范围并运行 wall 测试确认 GREEN。

### Task 4：为 rs-lock 增加 timeout waiter 注册屏障

**文件：**

- 新增：`rs-lock/src/monitor/mock_monitor_waiter_guard.rs`
- 修改：`rs-lock/src/monitor/mod.rs`
- 修改：`rs-lock/src/monitor/mock_monitor.rs`
- 修改：`rs-lock/src/monitor/arc_mock_monitor.rs`
- 测试：`rs-lock/tests/monitor/mock_monitor_tests.rs`
- 测试：`rs-lock/tests/monitor/arc_mock_monitor_tests.rs`

**接口：**

```rust
monitor.pending_timeout_waiters() -> usize;
monitor.wait_for_timeout_waiters(expected_count, real_timeout) -> bool;
```

- [x] 先改测试，用注册屏障替换真实 `thread::sleep()`，确认 API 缺失导致 RED。
- [x] 增加 RAII waiter guard，使 blocking/async timeout wait 在完成、取消或 panic
  时都注销 active count。
- [x] 在 `MockMonitor` 和 `ArcMockMonitor` 暴露查询与带真实 guard timeout 的等待。
- [x] 运行 rs-lock monitor 测试确认 GREEN。

### Task 5：迁移下游和补齐契约

**文件：**

- 修改：`rs-retry/tests/executor/retry_async_tests.rs`
- 修改：`rs-retry/tests/executor/retry_run_and_listener_tests.rs`
- 修改：`rs-clock/tests/scenario/*.rs`
- 修改：`README.md`、`README.zh_CN.md`、相关设计文档和 API 注释。

- [x] 用 clock 级 waiter barrier 替换 rs-retry 的 `yield_now()` 轮询。
- [x] 用 clock 级 blocking barrier 替换 sleeper helper。
- [x] 增加 waiter 注册/注销、subscription drop/advance 和混合 waiter 并发测试。
- [x] 修正文档中的 callback panic、`InvalidInstantOrder`、Tokio runtime panic 和
  manual driver 示例。

### Task 6：完整验证

- [x] 在 `rs-clock` 运行 `./align-ci.sh` 和 `./ci-check.sh`。
- [x] 在 `rs-lock` 运行 `./align-ci.sh` 和 `./ci-check.sh`；若仅因未发布的路径
  依赖无法 package，记录该外部发布阻塞。
- [x] 在 `rs-retry` 运行 `./align-ci.sh` 和 `./ci-check.sh`；按同样规则记录
  未发布依赖造成的 package 阻塞。
- [x] 确认所有回归测试、Clippy、rustdoc 和 feature matrix 通过。
