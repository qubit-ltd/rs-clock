# `rs-clock` 破坏性重构实施计划

> **执行约束：** 按 TDD 的 RED-GREEN-REFACTOR 循环逐项实施；不修改下游仓库；不执行 Git 提交。

**目标：** 按 `clock_refactoring_design.zh_CN.md` 重建 `qubit-clock`，提供分离的 wall、monotonic、blocking sleep 与 async sleep 抽象及真实/manual 实现。

**状态：** 已实施并完成 CI 对齐；本计划中的 checklist 保留为实施记录。

**架构：** Wall clock、monotonic clock 和 sleeper 分为三个模块。Concrete sleeper 显式持有对应的 `Arc<ConcreteMonotonicClock>`，并委托 `MonotonicClock`。Manual wall clock、blocking sleeper 和 async sleeper共享同一个 `Arc<ManualMonotonicClock>`。

**技术栈：** Rust 1.94、edition 2024、标准库同步原语、可选 Tokio time driver。

## 全局约束

- 允许破坏性变更，不保留旧 API。
- 每个公开类型单独放在与类型同名的 snake_case Rust 文件中。
- 测试只放在 `tests/`，目录结构与 `src/` 一致，文件名为源文件名加 `_tests.rs`。
- 默认 feature 不包含第三方依赖；Tokio 仅为可选依赖。
- 最终执行 `./align-ci.sh` 和 `./ci-check.sh`。
- 不修改 `rs-retry`、`rs-lock` 或其他下游仓库；下游场景通过 `rs-clock/tests/` 中的集成测试验证。

---

### Task 1：错误模型与 monotonic instant

**文件：**

- 创建：`src/error/mod.rs`
- 创建：`src/error/time_error.rs`
- 创建：`src/monotonic/mod.rs`
- 创建：`src/monotonic/clock_domain_id.rs`
- 创建：`src/monotonic/monotonic_instant.rs`
- 创建：`tests/error/mod.rs`
- 创建：`tests/error/time_error_tests.rs`
- 创建：`tests/error_tests.rs`
- 创建：`tests/monotonic/mod.rs`
- 创建：`tests/monotonic/monotonic_instant_tests.rs`
- 创建：`tests/monotonic_tests.rs`

**产出接口：**

```rust
pub enum TimeError {
    ClockDomainMismatch { expected: u64, actual: u64 },
    InstantOverflow,
    CannotMoveBackward,
}

pub struct MonotonicInstant {
    domain_id: u64,
    elapsed: Duration,
}
```

- [ ] 先写 domain 检查、checked add、duration since、跨 domain 错误和展示文本测试。
- [ ] 运行对应测试，确认因公开类型不存在而失败。
- [ ] 实现最小错误类型、AtomicU64 domain ID 生成器和 `MonotonicInstant`。
- [ ] 运行对应测试并保持通过。

### Task 2：Monotonic clock 实现

**文件：**

- 创建：`src/monotonic/monotonic_clock.rs`
- 创建：`src/monotonic/std_monotonic_clock.rs`
- 创建：`src/monotonic/tokio_monotonic_clock.rs`
- 创建：`src/monotonic/manual_monotonic_clock.rs`
- 创建：`src/monotonic/manual_monotonic_state.rs`
- 创建：`tests/monotonic/monotonic_clock_tests.rs`
- 创建：`tests/monotonic/std_monotonic_clock_tests.rs`
- 创建：`tests/monotonic/tokio_monotonic_clock_tests.rs`
- 创建：`tests/monotonic/manual_monotonic_clock_tests.rs`
- 创建：`tests/monotonic/manual_monotonic_state_tests.rs`

**产出接口：**

```rust
pub trait MonotonicClock: Send + Sync {
    fn now(&self) -> MonotonicInstant;
}

pub struct StdMonotonicClock;
pub struct TokioMonotonicClock;
pub struct ManualMonotonicClock;
```

- [ ] 先写 trait object、domain 唯一性、真实推进、Tokio paused time、manual advance/advance_to 和禁止倒退测试。
- [ ] 运行对应测试，确认 RED。
- [ ] 实现三个 clock；类型本身不实现 `Clone`，共享统一使用 `Arc<T>`。
- [ ] 为 manual clock 实现 blocking change notification、async waker registration 和 waiter introspection 所需的 crate-private API。
- [ ] 运行对应测试并保持通过。

### Task 3：Wall clock 实现

**文件：**

- 创建：`src/wall/mod.rs`
- 创建：`src/wall/wall_clock.rs`
- 创建：`src/wall/std_wall_clock.rs`
- 创建：`src/wall/fixed_wall_clock.rs`
- 创建：`src/wall/manual_wall_clock.rs`
- 创建：`tests/wall/mod.rs`
- 创建：`tests/wall/wall_clock_tests.rs`
- 创建：`tests/wall/std_wall_clock_tests.rs`
- 创建：`tests/wall/fixed_wall_clock_tests.rs`
- 创建：`tests/wall/manual_wall_clock_tests.rs`
- 创建：`tests/wall_tests.rs`

**产出接口：**

```rust
pub trait WallClock: Send + Sync {
    fn now(&self) -> SystemTime;
}

pub struct StdWallClock;
pub struct FixedWallClock;
pub struct ManualWallClock;
```

- [ ] 先写真实 wall time、固定时间、manual anchor、跟随 monotonic advance 和 reanchor 不影响 monotonic deadline 的测试。
- [ ] 运行对应测试，确认 RED。
- [ ] 实现三个 wall clock，并让 `ManualWallClock::from_clock` 显式接收 `Arc<ManualMonotonicClock>`。
- [ ] 运行对应测试并保持通过。

### Task 4：Blocking sleeper

**文件：**

- 创建：`src/sleep/blocking_sleeper.rs`
- 创建：`src/sleep/std_blocking_sleeper.rs`
- 创建：`src/sleep/manual_blocking_sleeper.rs`
- 创建：`tests/sleep/blocking_sleeper_tests.rs`
- 创建：`tests/sleep/std_blocking_sleeper_tests.rs`
- 创建：`tests/sleep/manual_blocking_sleeper_tests.rs`

**产出接口：**

```rust
pub trait BlockingSleeper: MonotonicClock;
pub struct StdBlockingSleeper;
pub struct ManualBlockingSleeper;
```

- [ ] 先写显式 `Arc` 构造、clock 委托、foreign deadline、已到期 deadline、blocking waiter 注册和 manual advance 唤醒测试。
- [ ] 运行对应测试，确认 RED。
- [ ] 实现 `StdBlockingSleeper::from_clock` 和 `ManualBlockingSleeper::from_clock`。
- [ ] 实现 manual waiter 观测 API：pending count、next deadline 和带真实保护 timeout 的 waiter 等待。
- [ ] 运行对应测试并保持通过。

### Task 5：Async sleeper

**文件：**

- 创建：`src/sleep/sleep_future.rs`
- 创建：`src/sleep/async_sleeper.rs`
- 创建：`src/sleep/manual_sleep_future.rs`
- 创建：`src/sleep/tokio_async_sleeper.rs`
- 创建：`src/sleep/manual_async_sleeper.rs`
- 创建：`tests/sleep/async_sleeper_tests.rs`
- 创建：`tests/sleep/manual_sleep_future_tests.rs`
- 创建：`tests/sleep/tokio_async_sleeper_tests.rs`
- 创建：`tests/sleep/manual_async_sleeper_tests.rs`

**产出接口：**

```rust
pub type SleepFuture<'a> = Pin<Box<dyn Future<Output = Result<(), TimeError>> + Send + 'a>>;
pub trait AsyncSleeper: MonotonicClock;
pub struct TokioAsyncSleeper;
pub struct ManualAsyncSleeper;
```

- [ ] 先写 deadline-at-call-time、Tokio paused time、manual advance、foreign deadline、相同 deadline 多 waiter 和 future drop 注销测试。
- [ ] 运行对应测试，确认 RED。
- [ ] 实现 runtime-neutral manual future/waker 模型和 Tokio sleeper。
- [ ] 运行对应测试并保持通过。

### Task 6：公开 API、依赖和旧实现移除

**文件：**

- 修改：`src/lib.rs`
- 修改：`src/sleep/mod.rs`
- 修改：`Cargo.toml`
- 删除：旧 `src/clock/`、`src/meter/`、`src/mock/` 中已被新模型替代的实现
- 删除：对应旧测试入口和旧测试文件

- [ ] 先写公开 API 与默认 feature 无 chrono/时区依赖的测试。
- [ ] 运行测试，确认旧导出与新期望不一致。
- [ ] 更新导出、package 描述和 Tokio feature，移除 chrono、chrono-tz、parking_lot、serde_json。
- [ ] 删除旧实现和旧测试，确保每个新类型及测试文件命名满足目录规则。
- [ ] 运行 `cargo test` 和 `cargo test --all-features`。

### Task 7：下游场景集成测试与文档

**文件：**

- 创建：`tests/scenario/mod.rs`
- 创建：`tests/scenario/retry_backoff_tests.rs`
- 创建：`tests/scenario/lock_timeout_tests.rs`
- 创建：`tests/scenario/login_lockout_tests.rs`
- 创建：`tests/scenario_tests.rs`
- 修改：`README.md`
- 修改：`README.zh_CN.md`
- 修改：`doc/clock_design.zh_CN.md`
- 修改：`doc/mock_time_design.zh_CN.md`

- [ ] 先写指数退避、锁等待超时和登录锁定十分钟的测试场景。
- [ ] 运行场景测试，确认 RED。
- [ ] 仅在 `rs-clock` 内提供测试辅助代码完成场景，不修改下游 crate。
- [ ] 更新文档，删除旧 API 示例并使用显式 `Arc` 构造关系。
- [ ] 运行全部测试与 doctest。

### Task 8：最终 CI 对齐

- [ ] 运行 `./align-ci.sh` 完成格式化与自动修正。
- [ ] 检查自动修改范围，修正不符合设计或文件组织约束的内容。
- [ ] 运行 `./ci-check.sh` 对齐 CI。
- [ ] 若失败，按失败项修复并重新运行，直到退出码为 0。
- [ ] 核对所有新类型、测试文件、默认依赖和公开导出是否覆盖设计文档。

---

## 发布前第二轮收口

### Task 9：开放 MonotonicClock 的安全实现路径

**修改：**

- `src/monotonic/clock_domain_id.rs`
- `src/monotonic/monotonic_clock.rs`
- `src/monotonic/{std,tokio,manual}_monotonic_clock.rs`
- `src/sleep/{std_blocking,tokio_async,manual_blocking,manual_async}_sleeper.rs`
- `tests/monotonic/clock_domain_id_tests.rs`
- `tests/monotonic/monotonic_clock_tests.rs`

**接口：**

- 公开 `allocate_clock_domain_id() -> u64`。
- `MonotonicClock` 的实现者提供 `domain_id()` 与
  `elapsed_since_origin()`；默认 `now()` 在 crate 内安全构造 instant。

- [x] 先增加 crate 外自定义 `MonotonicClock` 的编译与行为测试，确认旧 trait
  无法实现独立 domain。
- [x] 实现新 trait 契约并更新所有内置 clock/sleeper。
- [x] 运行 monotonic、sleep 定向测试。

### Task 10：收紧错误和 async future API

**修改：**

- `src/error/time_error.rs`
- `src/monotonic/monotonic_instant.rs`
- `src/sleep/sleep_future.rs`
- `src/sleep/async_sleeper.rs`
- `src/sleep/manual_async_sleeper.rs`
- `src/sleep/tokio_async_sleeper.rs`
- 对应 `tests/` 文件

**接口：**

- `InvalidInstantOrder` 表示 duration 计算的 instant 顺序错误；
  `CannotMoveBackward` 只表示 manual clock 向后推进。
- `SleepFuture` 固定拥有 `'static` 状态，不再携带无意义 lifetime 参数。

- [x] 先增加错误枚举和无 lifetime `SleepFuture` 编译契约测试并确认 RED。
- [x] 实现最小修改并运行对应测试确认 GREEN。

### Task 11：统一 manual waiter driver

**修改/新增：**

- `src/monotonic/manual_monotonic_clock.rs`
- `src/monotonic/manual_monotonic_state.rs`
- `src/monotonic/manual_waiter_future.rs`
- `tests/monotonic/manual_monotonic_clock_tests.rs`
- `tests/monotonic/manual_waiter_future_tests.rs`

**接口：**

- `pending_waiters()` 汇总 blocking 与 async sleeper waiter。
- `next_deadline()` 返回所有尚未到期 waiter 的最早 deadline。
- `advance_to_next_deadline()` 原子选择并推进到最早 deadline。
- `wait_for_waiters_async()` 返回 runtime-neutral、可取消的
  `ManualWaiterFuture`。

- [x] 先增加混合 blocking/async deadline 和等待注册测试并确认 API 缺失。
- [x] 实现 waiter observer 注册、waker 更新、取消注销与统一推进。
- [x] 覆盖 future 创建后未 poll、waker 替换和 drop 注销。

### Task 12：改善 advance subscription 和并发验证

**修改：**

- `src/monotonic/manual_monotonic_clock.rs`
- `tests/monotonic/manual_advance_subscription_tests.rs`
- `tests/monotonic/manual_monotonic_clock_tests.rs`
- `README.md`、`README.zh_CN.md`、设计文档

**接口：**

- `subscribe_advances` 直接接收泛型闭包，内部负责 `Arc` 包装。
- 一个 callback panic 时继续执行其他 callback，完成 fanout 后恢复首次 panic。
- 保留 `ManualWallClock::now()` 的 panic 策略并明确 `SystemTime` 溢出边界。

- [x] 先增加闭包调用、多个 panic callback 全部执行和并发 advance 测试。
- [x] 实现 panic-safe fanout，并保持 callback 在 clock mutex 外运行。
- [x] 运行 `align-ci.sh`、`ci-check.sh` 和两个下游的路径依赖测试。
