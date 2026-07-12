# `rs-clock` 重构设计方案

## 文档信息

- **状态**：已实施
- **日期**：2026-07-11
- **兼容性**：允许破坏性变更，不保留现有 API 兼容性

## 1. 背景与定位

`rs-clock` 的核心价值不是封装系统时间，而是把代码对真实时间流逝的依赖替换成可注入、可控制的时间能力。生产环境使用真实时间实现，测试环境使用手工推进的实现，从而让 retry、timeout、延迟执行、账号锁定和过期判断等逻辑能够快速、确定性地测试。

`rs-clock` 负责以下四类能力：

1. 读取 wall time，用于创建时间、过期时间、账号锁定等业务规则。
2. 读取 monotonic time，用于耗时、deadline 和 timeout。
3. 基于 monotonic time 执行同步等待。
4. 基于 monotonic time 执行异步等待。

`rs-clock` 不负责：

- 时区、日历计算和时间格式化。
- 任务调度、任务执行或完整的虚拟运行时。
- 全局隐式时钟。
- 拦截任意 OS 阻塞调用。
- 保证操作系统具有特定的真实测量精度。

## 2. 核心设计原则

### 2.1 Wall time 与 monotonic time 分离

Wall time 表示现实世界的时间点，可以因为系统校时等原因向前或向后跳变。Monotonic time 只表示某个时间域内已经流逝的时间，不能倒退。

两者使用不同 trait 和不同实现类型，不在同一个 clock 对象上混合公开 API。

### 2.2 Sleeper 基于 monotonic clock

Sleeper 不维护第二套时间状态。每个具体 sleeper 显式持有与其后端匹配的 monotonic clock，并将 `MonotonicClock` 方法委托给该 clock。

### 2.3 共享关系必须显式

Concrete monotonic clock 不通过 `Clone` 隐式表达共享。需要多个组件观察同一个时钟对象时，统一使用 `Arc<T>` 和 `Arc::clone(&clock)`。

### 2.4 依赖最小能力

业务组件只依赖实际需要的 trait：

- 只读取业务时间：`WallClock`。
- 只计算耗时或 deadline：`MonotonicClock`。
- 需要同步等待：`BlockingSleeper`。
- 需要异步等待：`AsyncSleeper`。

## 3. 四个核心 Trait

### 3.1 `WallClock`

```rust
pub trait WallClock: Send + Sync {
    fn now(&self) -> SystemTime;
}
```

`WallClock` 返回标准库的 `SystemTime`，不依赖 `chrono`，也不提供时区和格式化能力。

### 3.2 `MonotonicClock`

```rust
pub trait MonotonicClock: Send + Sync {
    fn domain_id(&self) -> u64;
    fn elapsed_since_origin(&self) -> Duration;
    fn now(&self) -> MonotonicInstant { /* 默认实现 */ }
}
```

`MonotonicClock` 返回所属时间域内的 `MonotonicInstant`。crate 外实现者可用公开的
`allocate_clock_domain_id()` 为独立 clock 分配 domain；instant 不能转换成 wall time。

### 3.3 `BlockingSleeper`

```rust
pub trait BlockingSleeper: MonotonicClock {
    fn sleep_until(
        &self,
        deadline: MonotonicInstant,
    ) -> Result<(), TimeError>;

    fn sleep_for(&self, duration: Duration) -> Result<(), TimeError> {
        let deadline = self.now().checked_add(duration)?;
        self.sleep_until(deadline)
    }
}
```

`BlockingSleeper: MonotonicClock` 保证调用方通过同一个对象读取当前 monotonic time 和执行等待。

### 3.4 `AsyncSleeper`

```rust
pub type SleepFuture = Pin<Box<dyn Future<Output = Result<(), TimeError>> + Send + 'static>>;

pub trait AsyncSleeper: MonotonicClock {
    fn sleep_until_async(
        &self,
        deadline: MonotonicInstant,
    ) -> SleepFuture;

    fn sleep_for_async(&self, duration: Duration) -> SleepFuture;
}
```

`sleep_for_async()` 的 deadline 在调用方法时确定，而不是在 future 第一次被 poll 时确定。

为支持 IoC 式注入，将为 `Arc<T>`、`Box<T>` 等常用包装类型提供 trait 转发实现。

## 4. `MonotonicInstant`

`MonotonicInstant` 是统一的 monotonic 时间点：

```rust
pub struct MonotonicInstant {
    domain_id: u64,
    elapsed: Duration,
}
```

字段保持私有。`domain_id` 标识 instant 所属的 monotonic time domain，`elapsed` 表示从该 domain origin 开始已经流逝的时间。

### 4.1 Domain ID

- `domain_id` 直接使用 `u64`。
- 每个新的 concrete monotonic clock 获得一个新的 domain ID。
- Domain ID 由全局 `AtomicU64` 分配。
- 分配逻辑必须检测溢出，不能在回绕后复用已经分配过的 ID。
- 同一个 `Arc<MonotonicClock>` 产生的所有 instant 使用同一个 domain ID。

### 4.2 运算规则

`MonotonicInstant` 支持：

- Checked duration add。
- 同一 domain 内的 duration since。
- 同一 domain 内的 deadline 比较。
- 获取 `domain_id`，用于诊断和错误报告。

不同 domain 的 instant 不能混合运算。跨 domain 比较或把 foreign instant 交给 sleeper 时，返回 `TimeError::ClockDomainMismatch`。

已经到达或早于当前时刻的 deadline 立即完成，不视为错误。

### 4.3 精度

不再拆分毫秒和纳秒 monotonic clock。所有 monotonic 实现统一保留 `Duration` 的完整表达精度，调用方按需使用：

```rust
let elapsed: Duration = end.duration_since(start)?;
let millis = elapsed.as_millis();
let nanos = elapsed.as_nanos();
```

`as_nanos()` 只表示数值的表达粒度，不代表底层操作系统一定具有纳秒测量精度。

## 5. 错误模型

核心错误至少覆盖：

```rust
pub enum TimeError {
    ClockDomainMismatch {
        expected: u64,
        actual: u64,
    },
    InstantOverflow,
    CannotMoveBackward,
    InvalidInstantOrder,
}
```

- `ClockDomainMismatch`：不同 monotonic domain 的 instant 被混用。
- `InstantOverflow`：duration 或 deadline 运算溢出。
- `CannotMoveBackward`：试图将 manual monotonic time 移动到当前时刻之前。
- `InvalidInstantOrder`：计算 duration 时传入的 earlier instant 晚于当前 instant。

具体错误类型的模块位置和展示文本在实施阶段确定，但上述错误语义保持不变。

## 6. Wall Clock 实现

### 6.1 `StdWallClock`

- 基于 `SystemTime::now()`。
- 反映真实系统 wall time。
- 允许现实中的 wall-time 跳变。

### 6.2 `FixedWallClock`

- 构造时接收一个固定 `SystemTime`。
- `now()` 永远返回该时间。
- 用于只需要固定业务时间的简单测试。

### 6.3 `ManualWallClock`

`ManualWallClock` 是某个 `ManualMonotonicClock` 上的 wall-time 投影视图。它保存 wall anchor、对应的 monotonic anchor，并显式持有：

```rust
Arc<ManualMonotonicClock>
```

构造方式：

```rust
let monotonic_clock = Arc::new(ManualMonotonicClock::new());

let wall_clock = ManualWallClock::from_clock(
    initial_wall_time,
    Arc::clone(&monotonic_clock),
);
```

当前 wall time 按以下规则计算：

```text
wall_now = wall_anchor + (monotonic_now - monotonic_anchor)
```

`ManualWallClock::reanchor(new_wall_time)` 只重新设置采样到的当前 monotonic instant 对应的 wall time：

- 可以模拟 wall time 向前或向后跳变。
- 不改变 monotonic time。
- 不会让 monotonic deadline 提前到期。
- 不会唤醒 monotonic sleeper。
- `now()` 和 `reanchor()` 在 anchor mutex 内采样 monotonic time，避免旧 wall
  anchor 与新 monotonic elapsed 组成混合快照。

## 7. Monotonic Clock 实现

### 7.1 `StdMonotonicClock`

- 基于 `std::time::Instant`。
- 随真实时间自动前进。
- 保存固定 origin 和 domain ID。
- 类型本身不通过 `Clone` 表达共享；共享时使用 `Arc<StdMonotonicClock>`。

### 7.2 `TokioMonotonicClock`

- 基于 `tokio::time::Instant`。
- 随 Tokio time driver 前进。
- 支持 Tokio 的 pause/advance 测试语义。
- 保存固定 origin 和 domain ID。
- 类型本身不通过 `Clone` 表达共享；共享时使用 `Arc<TokioMonotonicClock>`。

### 7.3 `ManualMonotonicClock`

- 从零开始维护逻辑 elapsed time。
- 不随真实时间自动前进。
- 只有测试显式调用控制 API 时才前进。
- 时间只能向前移动。
- 保存固定 domain ID。
- 类型本身不实现共享语义的 `Clone`；共享时使用 `Arc<ManualMonotonicClock>`。

主要控制 API：

```rust
monotonic_clock.advance(duration)?;
monotonic_clock.advance_to(deadline)?;
monotonic_clock.advance_to_next_deadline();
```

`advance_to()` 校验 deadline 的 domain，并拒绝向后移动；比较 current time、更新 elapsed 和收集唤醒通知在同一次 state lock 中完成。

`ManualMonotonicClock` 还提供 concrete-only 的时间推进订阅机制。该机制供 manual sleeper 和 `rs-lock::MockMonitor` 在时间推进时获得唤醒通知，但不进入 `MonotonicClock` trait：

```rust
let subscription = monotonic_clock.subscribe_advances(|| {
    // 唤醒 MockMonitor 自己的 Condvar 或 async waker。
});
```

订阅回调在 clock 内部 mutex 释放后同步调用。callback 应幂等并且只负责唤醒订阅方自己的等待原语；并发 advance 的 callback 可能并发执行且没有顺序保证。单个 callback panic 不会中断本次 fanout，所有已收集 callback 执行后在推进线程恢复第一个 panic。丢弃 `ManualAdvanceSubscription` 会阻止后续 advance 收集该 callback，但已被进行中的 advance 收集的 callback 仍可能执行一次。callback 若需要锁另一个同步对象，调用方不得在持有同一把锁时推进 clock。

`MockMonitor` 将此回调和普通 monitor notification 接入同一个等待状态机，并在每次唤醒后重新检查 predicate、notification epoch 和 deadline。

不提供 `reset()`。每个测试应创建新的 manual monotonic clock，避免 active waiter 的 deadline 在 reset 后失去语义。

## 8. Sleeper 的构造与 Clock 关系

### 8.1 `StdBlockingSleeper`

`StdBlockingSleeper` 只能与 `StdMonotonicClock` 配对：

```rust
let monotonic_clock = Arc::new(StdMonotonicClock::new());

let sleeper = StdBlockingSleeper::from_clock(
    Arc::clone(&monotonic_clock),
);
```

内部结构：

```rust
pub struct StdBlockingSleeper {
    clock: Arc<StdMonotonicClock>,
}
```

它实现 `MonotonicClock` 时直接委托给 `clock`，并使用该 clock 对应的 `std::time::Instant` domain 完成阻塞等待。

### 8.2 `TokioAsyncSleeper`

`TokioAsyncSleeper` 只能与 `TokioMonotonicClock` 配对：

```rust
let monotonic_clock = Arc::new(TokioMonotonicClock::new());

let sleeper = TokioAsyncSleeper::from_clock(
    Arc::clone(&monotonic_clock),
);
```

内部结构：

```rust
pub struct TokioAsyncSleeper {
    clock: Arc<TokioMonotonicClock>,
}
```

它实现 `MonotonicClock` 时直接委托给 `clock`，并将同一 domain 的 `MonotonicInstant` deadline 映射到 `tokio::time::Instant`。

### 8.3 `ManualBlockingSleeper`

```rust
let monotonic_clock = Arc::new(ManualMonotonicClock::new());

let sleeper = ManualBlockingSleeper::from_clock(
    Arc::clone(&monotonic_clock),
);
```

内部结构：

```rust
pub struct ManualBlockingSleeper {
    clock: Arc<ManualMonotonicClock>,
}
```

它把 relative sleep 转换成同一 manual domain 上的 deadline，阻塞线程直到 clock 推进到该 deadline。

### 8.4 `ManualAsyncSleeper`

```rust
let monotonic_clock = Arc::new(ManualMonotonicClock::new());

let sleeper = ManualAsyncSleeper::from_clock(
    Arc::clone(&monotonic_clock),
);
```

内部结构：

```rust
pub struct ManualAsyncSleeper {
    clock: Arc<ManualMonotonicClock>,
}
```

它把 relative async sleep 转换成同一 manual domain 上的 deadline。Future 向 manual clock 的推进通知机制注册 waker；future 被取消或丢弃时必须注销 waiter。

### 8.5 不支持的组合

不允许 sleeper 接收任意 `Arc<dyn MonotonicClock>`，因为不同 sleeper 需要后端特有能力：

- `StdBlockingSleeper` 需要映射到 `std::time::Instant`。
- `TokioAsyncSleeper` 需要映射到 `tokio::time::Instant`。
- Manual sleeper 需要订阅 `ManualMonotonicClock` 的显式推进。

不提供：

- `StdAsyncSleeper`：标准库没有 executor-independent 的真实异步 timer。
- `TokioBlockingSleeper`：不应阻塞 Tokio runtime。
- 一个同时实现 blocking 和 async 的 `ManualSleeper`：同步和异步职责分别由 `ManualBlockingSleeper`、`ManualAsyncSleeper` 表达。

## 9. Manual Time 组合关系

完整的 manual test time domain 按以下方式组合：

```rust
let monotonic_clock = Arc::new(ManualMonotonicClock::new());

let wall_clock = Arc::new(ManualWallClock::from_clock(
    initial_wall_time,
    Arc::clone(&monotonic_clock),
));

let blocking_sleeper = Arc::new(ManualBlockingSleeper::from_clock(
    Arc::clone(&monotonic_clock),
));

let async_sleeper = Arc::new(ManualAsyncSleeper::from_clock(
    Arc::clone(&monotonic_clock),
));
```

共享关系：

```text
Arc<ManualMonotonicClock>
├── ManualWallClock
├── ManualBlockingSleeper
└── ManualAsyncSleeper
```

推进：

```rust
monotonic_clock.advance(Duration::from_secs(600))?;
```

一次推进会产生以下效果：

- `ManualMonotonicClock::now()` 前进十分钟。
- `ManualWallClock::now()` 自然前进十分钟。
- 到期的 `ManualBlockingSleeper` waiter 被唤醒。
- 到期的 `ManualAsyncSleeper` future 被唤醒。

`MonotonicInstant` 只是由 `MonotonicClock::now()` 返回的固定时间点。它只用于 elapsed 和 deadline 运算，绝不用于构造 wall clock 或 sleeper。

## 10. Manual 测试辅助能力

为了避免测试线程在 waiter 注册前提前推进时间，manual sleeper 提供 concrete-only 的测试辅助能力，例如：

```rust
manual_clock.pending_waiters();
manual_clock.next_deadline();
manual_clock.wait_for_waiters(expected_count, real_guard_timeout);
manual_clock.advance_to_next_deadline();
manual_clock.wait_for_waiters_async(expected_count).await;
```

其中 `real_guard_timeout` 只用于防止测试永久挂起，不参与业务时间语义。
clock 级 API 同时观察 blocking 与 async waiter；异步注册等待是 runtime-neutral
且在 future 被取消时自动注销。

Manual sleeper 需要满足：

- 零时长 sleep 立即完成。
- 已到期 deadline 立即完成。
- 一次 advance 可以跨过多个 deadline。
- 相同 deadline 的 waiter 全部被唤醒。
- async future 被取消后不遗留 waiter。
- `advance()` 负责标记到期并发送唤醒，但不承诺异步任务已被 runtime 再次 poll。
- 不提供隐式 auto-advance 或 `run_until_idle()`。

## 11. Concrete 实现矩阵

| 能力 | 真实实现 | 测试实现 |
|---|---|---|
| `WallClock` | `StdWallClock` | `FixedWallClock`、`ManualWallClock` |
| `MonotonicClock` | `StdMonotonicClock`、`TokioMonotonicClock` | `ManualMonotonicClock` |
| `BlockingSleeper` | `StdBlockingSleeper` | `ManualBlockingSleeper` |
| `AsyncSleeper` | `TokioAsyncSleeper` | `ManualAsyncSleeper` |

不提供 `TokioWallClock`，因为 Tokio 没有独立的 wall-time 体系。

## 12. `rs-retry` 接入方案

时间依赖在执行边界注入，不放入纯配置对象 `RetryOptions`：

```rust
pub struct RetryRunner {
    sleeper: Arc<dyn BlockingSleeper>,
}

pub struct AsyncRetryRunner {
    sleeper: Arc<dyn AsyncSleeper>,
}
```

生产默认实现：

- 同步 runner 使用 `StdBlockingSleeper`。
- 异步 runner 使用 `TokioAsyncSleeper`。

测试分别注入 `ManualBlockingSleeper` 或 `ManualAsyncSleeper`。

需要替换的硬编码时间依赖：

- `std::time::Instant::now()` 改为 `sleeper.now()`。
- `std::thread::sleep()` 改为 `sleeper.sleep_for()`。
- `tokio::time::sleep()` 改为 `sleeper.sleep_for_async()`。
- `tokio::time::timeout()` 改为 operation future 与 `sleep_until_async()` 的竞争。

这样 operation elapsed、total elapsed、retry backoff 和 timeout 都使用同一个 monotonic domain。指数退避的 `1s -> 2s -> 4s -> 8s` 集成测试可以通过 manual time 完成，而不产生真实等待。

同步同线程执行的任意阻塞闭包不能被 clock 强制中断。Worker timeout 必须在 worker/channel 等待边界重新设计，不能继续依赖真实时间的 `recv_timeout()`。

## 13. `rs-lock` 接入方案

生产 monitor 继续使用对应的原生同步原语。真实 `Condvar::wait_timeout()` 不能被 mock clock 接管，因此 mock-aware monitor 仍然必要。

`MockMonitor` 重构为：

- 显式接收 `Arc<ManualMonotonicClock>`。
- 删除自行维护的 elapsed time。
- 使用 manual clock 计算 timeout deadline。
- 订阅 manual clock 的推进通知。
- Monitor notification 和 clock advance 都唤醒同一个等待状态机。
- 每次被唤醒后重新检查 predicate、notification epoch 和 deadline。

这样 `rs-lock` 保留 monitor 特有的通知语义，但不再复制一套虚拟时间机制。

## 14. DAO 与 Service 使用方式

长期持有并反复读取时间的组件通过构造函数或 builder 注入 trait object：

- 登录锁定、TTL、过期判断：`Arc<dyn WallClock>`。
- 耗时预算和 deadline：`Arc<dyn MonotonicClock>`。
- 同步延迟：`Arc<dyn BlockingSleeper>`。
- 异步延迟：`Arc<dyn AsyncSleeper>`。

纯函数仍优先显式接收 `now` 参数，不为一次时间读取引入长期依赖。

登录锁定测试中，service 注入 `ManualWallClock`，测试推进其底层 `ManualMonotonicClock` 十分钟，无需缩短业务配置，也无需真实等待。

## 15. 现有 API 处理

| 现有 API | 重构结果 |
|---|---|
| `Clock` | 替换为 `WallClock` |
| `SystemClock` | 替换为 `StdWallClock` |
| concrete `MonotonicClock` | 替换为 trait 和 `StdMonotonicClock` |
| `NanoClock` | 删除 |
| `NanoMonotonicClock` | 删除 |
| `ControllableClock` | 删除，控制方法只存在于 concrete manual 类型 |
| `MockClock` | 替换为 `ManualWallClock` |
| `MockInstant` | 替换为 `MonotonicInstant` |
| `MockTimeline` | 替换为 `ManualMonotonicClock` 及其推进通知机制 |
| `MockSleeper` | 拆分为 `ManualBlockingSleeper`、`ManualAsyncSleeper` |
| `SystemSleeper` | 替换为 `StdBlockingSleeper` |
| `Sleeper` | 替换为 `BlockingSleeper` |
| 现有 `AsyncSleeper` | 按新的 monotonic domain 模型重写 |
| `ZonedClock` / `Zoned` | 删除 |
| `TimeMeter` / `NanoTimeMeter` | 删除；如有真实需求，后续基于 `MonotonicClock` 统一设计 stopwatch |

## 16. 依赖与 Feature

目标依赖结构：

- 默认构建只依赖标准库。
- `tokio` 为可选 feature，仅用于 `TokioMonotonicClock` 和 `TokioAsyncSleeper`。
- `ManualAsyncSleeper` 使用标准库 `Future`/`Waker` 实现，不依赖 Tokio。
- 核心不依赖 `chrono`、时区库和格式化库。
- Wall time 与 `chrono`、`time` 等类型的转换由下游负责。
- 不使用 `async-trait`。

## 17. 验收标准

重构完成必须同时满足 crate 自身契约和真实下游场景：

1. `rs-retry` 同步指数退避不产生真实等待。
2. `rs-retry` 异步 backoff 和 attempt timeout 可由 manual time 驱动。
3. `rs-lock` 同步和异步 timeout 可由 manual time 驱动。
4. 登录失败五次并锁定十分钟的示例可以通过推进 manual time 测试。
5. Wall reanchor 不影响 monotonic deadline。
6. 不同 domain 的 `MonotonicInstant` 不能混用。
7. Async sleep 取消不遗留 waiter。
8. 多线程推进与等待不存在丢失唤醒。
9. `TokioMonotonicClock` 与 Tokio paused time 保持一致。
10. 默认 feature 下不引入 Tokio、chrono 或时区依赖。

## 18. 实施顺序

1. 重写 `rs-clock` 的四个核心 trait、`MonotonicInstant` 和错误模型。
2. 实现 wall clock 与 monotonic clock concrete 类型。
3. 实现四个 concrete sleeper 及 manual waiter 模型。
4. 完成 `rs-clock` 的契约测试、并发测试和取消安全测试。
5. 接入 `rs-retry`，验证 blocking/async backoff 与 timeout。
6. 接入 `rs-lock`，消除 `MockMonitor` 自行维护的 elapsed time。
7. 删除旧 API、旧依赖和已失效文档内容。
8. 更新 crate README、公开文档和下游使用示例。

## 19. 发布前第三轮收口

第三轮评估保持四 trait 和 concrete clock/sleeper 架构不变，只收紧 manual
测试驱动语义：

- `wait_for_waiters_async()` 使用锁存语义；waiter 数量一旦达到目标，后续注销
  不会让 observer 重新进入 pending。
- Manual driver 控制 API 统一放在 `ManualMonotonicClock`。Sleeper 只负责等待，
  不再公开含义不同的同名 waiter 查询方法。
- `advance_to_next_deadline()` 只可能返回“有下一个 deadline”或“没有”，因此
  返回 `Option<MonotonicInstant>`，不保留无实际错误路径的 `Result`。
- `subscribe_advances()` 和 `wait_for_waiters_async()` 使用 `self: &Arc<Self>`，
  在保持共享身份显式的同时提供方法调用语法。
- `ManualWallClock::now()` 与 `reanchor()` 在同一个 anchor mutex 保护下采样
  monotonic time，使两者之间具有明确的一致快照语义。
- `rs-lock::MockMonitor` 自己维护 active timeout waiter 数量和注册屏障；
  `rs-clock` 不扩张为通用任务调度器或任意下游 deadline registry。
