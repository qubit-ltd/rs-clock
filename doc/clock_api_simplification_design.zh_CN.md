# `rs-clock` 时钟 API 简化与手动时钟拆分设计

## 文档信息

- **状态**：已实施，并于 2026-07-15 完成复核
- **日期**：2026-07-15
- **兼容性**：包含破坏性 API 变更，不保留 `0.9` API 兼容性

## 1. 目标

本次调整解决以下问题：

1. `MonotonicClock::now()` 由 `domain_id()` 和
   `elapsed_since_origin()` 两次调用拼装，公共 trait 暴露了不必要的实现细节，
   也无法从接口上保证一次时间采样的一致性。
2. `BlockingSleeper` 和 `AsyncSleeper` 继承 `MonotonicClock`，表达成了
   “sleeper 是 clock”，与实际的持有关系不符。
3. 裸 `u64` domain ID 缺少类型语义，domain 分配函数也泄露了底层表示。
4. `ManualMonotonicClock` 同时管理时间、waiter、waiter observer、advance
   subscriber 和通知分发，文件和类型职责过重。
5. manual async sleep 的创建时注册语义需要更直接的文档和回归测试。

本次调整不改变以下决策：

- domain 标识符耗尽时继续 panic，不增加可恢复错误路径。
- `WallClock::now()` 继续直接返回 `SystemTime`，不增加 `try_now()`。
- `StdMonotonicClock` 和 `TokioMonotonicClock` 保持两个独立公开类型。
- manual clock 的所有时间与 waiter 状态继续由同一把 mutex 保护。
- manual async sleep 继续在 future 创建时注册，而不是首次 poll 时注册。

## 2. `ClockDomain`

### 2.1 类型定义

新增公开类型：

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClockDomain(u64);
```

`ClockDomain` 表示一个进程内唯一的 monotonic time domain。内部的 `u64`
保持私有，不提供从任意 `u64` 构造 domain 的接口。

### 2.2 Domain 分配

```rust
impl ClockDomain {
    #[must_use]
    pub fn new() -> Self;
}
```

`ClockDomain::new()` 使用进程级 `AtomicU64` 分配。零值作为耗尽终态，不会作为
domain 返回；已经分配的标识符不复用。`u64::MAX` 是最后一个有效标识符：返回它
的分配会把原子状态切换为零，之后的所有分配均 panic。

`ClockDomain` 实现 `Display`，用于错误信息和日志诊断，但不提供裸 `u64`
getter，也不实现 `Default`。这可以避免调用方依赖底层表示，或在默认构造时
无意创建新的身份。

删除公开函数：

```rust
allocate_clock_domain_id() -> u64
```

原 `clock_domain_id.rs` 替换为 `clock_domain.rs`。

## 3. `MonotonicInstant` 与 `TimeError`

`MonotonicInstant` 改为持有强类型 domain：

```rust
pub struct MonotonicInstant {
    domain: ClockDomain,
    elapsed: Duration,
}
```

为支持 crate 外部实现 `MonotonicClock`，公开受类型约束的构造函数：

```rust
impl MonotonicInstant {
    pub const fn new(
        domain: ClockDomain,
        elapsed: Duration,
    ) -> Self;

    pub const fn domain(self) -> ClockDomain;

    pub const fn elapsed_since_origin(self) -> Duration;
}
```

公开构造函数不会接受裸 domain ID，因此调用方无法绕过 `ClockDomain` 的身份
分配规则。domain 身份不是安全边界；持有某个 `ClockDomain` 的时钟实现可以按其
时间源构造该 domain 内的 instant。

保留 `MonotonicInstant::elapsed_since_origin()`。这个值仍用于：

- 将 domain-scoped deadline 映射回 `std::time::Instant` 或
  `tokio::time::Instant`；
- manual clock 内部比较和推进；
- 测试与诊断。

`domain_id()` 重命名为 `domain()`。跨 domain 错误改为：

```rust
TimeError::ClockDomainMismatch {
    expected: ClockDomain,
    actual: ClockDomain,
}
```

错误展示继续输出两个 domain 的可读标识。

## 4. `MonotonicClock`

### 4.1 新接口

```rust
pub trait MonotonicClock: Send + Sync {
    fn now(&self) -> MonotonicInstant;
}
```

删除：

```rust
fn domain_id(&self) -> u64;
fn elapsed_since_origin(&self) -> Duration;
```

`now()` 不再提供默认实现，每个 clock 必须在一次实现调用中返回完整的
`MonotonicInstant`。这使 trait 只表达“读取当前单调时间”的能力，不再把 clock
内部 origin 的表示暴露为必需接口。

crate 外部实现按以下方式创建自己的稳定 domain：

```rust
pub struct ExternalClock {
    domain: ClockDomain,
    // 外部时间源状态
}

impl MonotonicClock for ExternalClock {
    fn now(&self) -> MonotonicInstant {
        let elapsed = /* 原子地采样外部时间源 */;
        MonotonicInstant::new(self.domain, elapsed)
    }
}
```

`Arc<T>` 和 `Box<T>` 的转发实现只转发 `now()`。

### 4.2 内置实现

三个 monotonic clock 的字段统一改名为 `domain`：

```rust
pub struct StdMonotonicClock {
    domain: ClockDomain,
    origin: std::time::Instant,
}

pub struct TokioMonotonicClock {
    domain: ClockDomain,
    origin: tokio::time::Instant,
}

pub struct ManualMonotonicClock {
    domain: ClockDomain,
    // 同步状态
}
```

每个 `now()` 直接读取固定 domain，并采样对应时间源。`ManualMonotonicClock`
在状态锁内读取 elapsed。

`StdMonotonicClock` 和 `TokioMonotonicClock` 不合并，也不引入公开泛型 backend：

- 两者的 `Instant` 类型不同；
- 一个跟随真实时间，一个跟随 Tokio time driver；
- Tokio clock 支持 pause/advance；
- 两者分别与 std blocking sleeper 和 Tokio async sleeper 配对；
- 独立类型可在编译期阻止错误后端组合。

两者当前只有少量结构性重复，抽取 enum 或泛型公共类型带来的复杂度高于收益。

Tokio time driver 的身份不属于 `ClockDomain`，Tokio 也没有提供适合此处校验的稳定
driver identity。使用暂停或显式推进的 Tokio 时间时，调用方必须在同一个 runtime
time driver 下创建和读取 `TokioMonotonicClock`，并 poll 配对
`TokioAsyncSleeper` 的 future。任务可以在同一 runtime 的 worker thread 之间迁移，
但 clock/sleeper 组合不得跨独立 runtime 使用。该约束通过 rustdoc 和 README 明确，
不增加新的公开 driver 抽象。

## 5. Sleeper 改为组合 Clock

### 5.1 `BlockingSleeper`

```rust
pub trait BlockingSleeper: Send + Sync {
    fn clock(&self) -> &dyn MonotonicClock;

    fn sleep_until(
        &self,
        deadline: MonotonicInstant,
    ) -> Result<(), TimeError>;

    fn sleep_for(&self, duration: Duration) -> Result<(), TimeError> {
        let deadline = self.clock().now().checked_add(duration)?;
        self.sleep_until(deadline)
    }
}
```

### 5.2 `AsyncSleeper`

```rust
pub trait AsyncSleeper: Send + Sync {
    fn clock(&self) -> &dyn MonotonicClock;

    fn sleep_until_async(
        &self,
        deadline: MonotonicInstant,
    ) -> SleepFuture;

    fn sleep_for_async(&self, duration: Duration) -> SleepFuture {
        // 在方法调用时读取 clock 并确定 deadline。
    }
}
```

Sleeper 不再实现或继承 `MonotonicClock`。四个 concrete sleeper 继续持有原有的
`Arc<ConcreteMonotonicClock>`，`clock()` 返回该对象的共享引用。

该设计表达的是：

```text
Sleeper has a MonotonicClock
```

它同时保留以下现有保证：

- relative sleep 使用 sleeper 配对的 clock 计算 deadline；
- `sleep_until` 可以拒绝 foreign deadline；
- 需要同时计时和等待的调用方可以从同一个 sleeper 获取配对 clock；
- `Arc<dyn BlockingSleeper>` 和 `Arc<dyn AsyncSleeper>` 仍然是完整、对象安全的
  注入边界。

`Arc<T>` 和 `Box<T>` 的 sleeper 转发实现同时转发 `clock()` 与 sleep 方法。

### 5.3 下游迁移

`rs-retry` 继续只注入 sleeper，不增加第二个可独立配置的 clock，避免出现 clock
和 sleeper domain 不匹配。原调用：

```rust
sleeper.now()
```

改为：

```rust
sleeper.clock().now()
```

`RetryFlowState` 保存 `sleeper.clock()` 返回的 `&dyn MonotonicClock`。builder 的
公开 sleeper 注入 API 不需要改变。

## 6. `ManualMonotonicClock` 内部拆分

### 6.1 约束

所有时间状态、deadline waiter、waiter-count observer 和 advance subscriber
继续位于同一个 `Mutex<ManualMonotonicState>` 中。此次拆分只建立代码与职责边界，
不拆分锁，也不改变并发语义。

这是为了保持以下操作的原子性：

- 校验当前 elapsed 并注册 waiter；
- 推进 elapsed 并收集到期 waker；
- 查询 earliest deadline 并推进到该 deadline；
- 统计 waiter 并 latch 已满足的 observer；
- 收集一次 advance 对应的 subscriber 快照。

### 6.2 类型职责

```text
ManualMonotonicClock
├── domain: ClockDomain
├── state: Mutex<ManualMonotonicState>
├── changed: Condvar
└── waiters_changed: Condvar

ManualMonotonicState
├── elapsed: Duration
├── waiters: ManualWaiterRegistry
└── advances: ManualAdvanceRegistry

ManualWaiterRegistry
├── blocking waiters
├── async waiters and wakers
└── waiter-count observers

ManualAdvanceRegistry
└── advance callbacks
```

职责如下：

- `ManualMonotonicClock`：公开 façade、mutex/Condvar、elapsed 状态转换、
  next-deadline 决策、advance effects 汇总，以及锁外的 waker 和 callback fanout。
- `ManualMonotonicState`：聚合由同一 mutex 保护的 elapsed、waiter registry 和
  advance registry，并提供总 waiter 数量。
- `ManualWaiterRegistry`：waiter ID 分配、注册、poll、注销、计数、最早 deadline、
  到期 waker 的一次性移出和 waiter observer latch。
- `ManualAdvanceRegistry`：subscriber ID 分配、注册、注销和 callback 快照。
- `ManualSleepFuture`、`ManualWaiterFuture`、`ManualAdvanceSubscription`：继续作为
  独立的 RAII/future 类型存在。

所有 registry 字段保持私有，`ManualMonotonicClock` 不再直接操作内部
`HashMap` 和 ID 计数器。

### 6.3 锁内状态转换与锁外副作用

clock 在持有 state mutex 时完成状态转换，并返回显式 advance effects：

```rust
struct AdvanceEffects {
    due_wakers: Vec<Waker>,
    advance_callbacks: Vec<AdvanceCallback>,
}
```

到期 waiter 的 `Waker` 通过 `Option::take()` 移入 `AdvanceEffects`，因此跨过
deadline 后、future 再次 poll 前的后续 advance 不会重复 wake 同一个已保存
waker。waiter 注册本身继续保留到 future poll 或 drop，因此 pending 计数和取消
语义不变。advance callback 是持久订阅，仍为每次成功推进创建快照。

waiter 注册只产生 observer waker 一种锁外结果，继续直接使用 `Vec<Waker>`，不为
单字段结果增加 `WaiterRegistrationEffects`。

处理顺序保持不变：

1. 持有 state mutex 完成状态转换并收集 effects；
2. 释放 state mutex；
3. 通知 Condvar；
4. wake task；
5. 调用 advance subscriber；
6. 如果 subscriber panic，完成本轮 fanout 后恢复第一个 panic。

此次拆分不改变公开的 manual control、waiter coordination 和 subscription API。

## 7. Manual Async Sleep 契约

`ManualAsyncSleeper` 继续在 `sleep_until_async()` 或 `sleep_for_async()` 返回前注册
waiter，而不是等到 future 第一次被 poll。

必须在以下位置明确该行为：

- `ManualAsyncSleeper` 类型文档；
- `sleep_until_async()` 实现文档；
- 英文和中文 README 的 manual-time coordination 章节。

契约包括：

- 尚未 poll 的 future 已计入 `pending_waiters()`；
- 创建后、首次 poll 前推进到 deadline，future 首次 poll 时立即完成；
- 丢弃未完成 future 会注销 waiter；
- foreign deadline 仍通过 ready error future 报告；
- relative deadline 仍在方法调用时确定。

## 8. `StdBlockingSleeper` 简化

`std::thread::sleep(remaining)` 已保证至少等待给定 duration，并在平台实现中处理
被信号中断的底层休眠。因此 `sleep_until()` 不需要在返回后循环重算 deadline。

实现改为：

1. 校验 deadline domain；
2. 转换为 native `std::time::Instant`；
3. deadline 已到达时立即返回；
4. 否则计算一次 remaining 并调用一次 `thread::sleep(remaining)`。

文档明确实际返回可能因为线程调度晚于 deadline，但不会承诺精确唤醒时间。

## 9. 测试设计

### 9.1 `ClockDomain` 与外部 Clock

- `ClockDomain::new()` 生成互不相同的 domain，不提供 `Default` 构造。
- allocator 返回 `u64::MAX` 后进入耗尽状态，下一次分配 panic。
- `ClockDomain` 支持复制、相等比较、哈希、Debug 和 Display。
- `TimeError::ClockDomainMismatch` 保存并展示强类型 domain。
- crate 外风格的自定义 `MonotonicClock` 可以通过
  `ClockDomain` 和 `MonotonicInstant::new()` 实现。
- `Arc<dyn MonotonicClock>` 和 `Box<dyn MonotonicClock>` 只转发 `now()`。

### 9.2 Sleeper 组合关系

- blocking/async sleeper trait object 的 `clock()` 返回构造时传入的 domain。
- `sleep_for` 和 `sleep_for_async` 使用 `clock()` 的调用时 instant。
- foreign deadline 继续返回准确的 expected/actual `ClockDomain`。
- `Arc<T>` 和 `Box<T>` sleeper 转发 `clock()` 及等待行为。

### 9.3 Manual Clock 回归

保留并强化以下行为测试：

- blocking 与 async waiter 按 deadline 顺序完成；
- 相同 deadline 的 waiter 全部被唤醒；
- async waker 替换与取消清理；
- async deadline 到期后只移出并 wake 已保存 waker 一次；
- waiter observer 达标后 latch；
- waiter 注册与 concurrent advance 不丢失唤醒；
- concurrent advance 不丢失 elapsed；
- subscriber 注销、并发调用和 panic fanout；
- `advance_to_next_deadline()` 忽略已到期但尚待清理的 waiter。

新增明确命名的测试：

```text
test_manual_async_sleep_completes_after_advance_before_first_poll
```

### 9.4 Native Clock 与 Sleeper

- `StdMonotonicClock` 继续跟随真实 monotonic time。
- `TokioMonotonicClock` 继续跟随 paused/advanced Tokio time，并明确 clock 创建、
  读取和配对 sleeper poll 必须属于同一个 runtime time driver。
- 两个 native sleeper 保持 domain 校验和 native instant overflow 测试。
- `StdBlockingSleeper` 测试已到期立即返回和短 deadline 等待；不增加脆弱的
  精确耗时断言。

### 9.5 下游验证

- `rs-clock`：无 feature 与 all-features 的格式化、Clippy、测试和 doctest。
- `rs-retry`：默认与 Tokio feature 测试，重点验证 elapsed accounting、backoff
  和 async timeout 使用 sleeper 配对 clock。
- `rs-lock`：启用 mock feature 验证 `ManualMonotonicClock` 行为保持兼容。

## 10. 文档与迁移说明

更新以下文档：

- `README.md` 与 `README.zh_CN.md`；
- public rustdoc；
- 已实施的 clock refactoring design 增加指向本文的后续调整说明；
- 发布说明列出破坏性 API 迁移。

迁移映射：

| 原 API | 新 API |
|---|---|
| `allocate_clock_domain_id()` | `ClockDomain::new()` |
| `instant.domain_id()` | `instant.domain()` |
| `MonotonicClock::domain_id()` | 删除；domain 由 `now()` 返回的 instant 携带 |
| `MonotonicClock::elapsed_since_origin()` | 删除 |
| sleeper 作为 `MonotonicClock` 使用 | `sleeper.clock()` |
| `sleeper.now()` | `sleeper.clock().now()` |

`MonotonicInstant::elapsed_since_origin()` 保持不变。

## 11. 非目标

本次不实施：

- domain 持久化、跨进程传输或序列化；
- domain ID 回收或耗尽恢复；
- `WallClock::try_now()`；
- 合并 std/Tokio clock；
- 拆分 manual state mutex；
- 新的 auto-advance、scheduler 或虚拟 runtime；
- 为 clock/sleeper 组合增加新的公开 driver 抽象。
