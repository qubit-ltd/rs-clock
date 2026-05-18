# Clock 时钟抽象设计文档

## 版本信息

- **文档版本**: 1.2
- **创建日期**: 2025-10-19
- **更新日期**: 2026-05-18
- **作者**: 胡海星

## 1. 设计概述

本文档描述了 `qubit-clock` crate 的架构设计。该设计提供了一套清晰、类型安全、灵活的时钟与 sleep 抽象，支持多种使用场景。

### 1.1 设计目标

1. **职责分离**：将时间获取、时区支持、高精度测量、时钟控制、relative sleep 等功能分离到不同的 trait
2. **类型安全**：通过类型系统在编译期保证功能支持（如是否支持时区、纳秒精度等）
3. **零成本抽象**：不需要的功能不付出任何性能代价
4. **易于测试**：提供共享 mock timeline 的模拟时钟和模拟 sleeper，支持单元测试和集成测试
5. **灵活组合**：通过包装器模式灵活组合不同的功能
6. **Sleep 可注入**：生产代码使用真实 sleep，测试代码通过共享 mock timeline 推进 sleep，避免真实等待

### 1.2 核心设计原则

- **接口隔离原则**：不强制实现不需要的功能
- **单一职责原则**：每个 trait 和类型只负责一个明确的功能
- **组合优于继承**：通过组合而非继承来扩展功能
- **依赖倒置原则**：依赖抽象（trait）而非具体实现

## 2. 架构设计

### 2.1 Trait 层次结构

```
Clock (基础时钟 trait)
├── NanoClock (高精度时钟 trait)
├── ZonedClock (带时区的时钟 trait)
└── ControllableClock (可控制的时钟 trait)

Sleeper (阻塞式 relative sleep trait)
└── AsyncSleeper (Tokio 异步 relative sleep trait)
```

**说明**：
- `Clock` 是基础 trait，提供 UTC 时间
- `NanoClock`、`ZonedClock`、`ControllableClock` 都继承自 `Clock`
- 这三个扩展 trait 是**正交的**，互不依赖
- `Sleeper` 系列不继承 `Clock`，因为它只表达 relative sleep，不表达 UTC 当前时间
- 异步 sleep trait 只在启用 `tokio` feature 时导出

### 2.2 实现类型

```
Clock trait 实现：
├── SystemClock (系统时钟)
├── MonotonicClock (单调时钟)
├── NanoMonotonicClock (高精度单调时钟)
└── MockClock (timeline-backed 模拟时钟)

包装器：
└── Zoned<C: Clock> (为任何 Clock 添加时区支持)

Sleep trait 实现：
├── SystemSleeper (真实 sleeper)
└── MockSleeper (timeline-backed 模拟 sleeper)

Mock time runtime：
├── MockTimeline (共享单调 mock 时间源)
├── MockInstant (timeline 上的单调 instant)
└── MockTime (组合 timeline、clock 和 sleeper 的便捷入口)
```

### 2.3 类型关系图

```mermaid
graph TD
    Clock[Clock trait<br/>基础时钟]
    NanoClock[NanoClock trait<br/>高精度]
    ZonedClock[ZonedClock trait<br/>时区支持]
    ControllableClock[ControllableClock trait<br/>可控制]

    SystemClock[SystemClock<br/>系统时钟]
    MonotonicClock[MonotonicClock<br/>单调时钟]
    NanoMonotonicClock[NanoMonotonicClock<br/>高精度单调时钟]
    MockClock[MockClock<br/>timeline-backed 模拟时钟]
    Zoned[Zoned&lt;C&gt;<br/>时区包装器]

    Clock --> NanoClock
    Clock --> ZonedClock
    Clock --> ControllableClock

    Clock -.实现.-> SystemClock
    Clock -.实现.-> MonotonicClock
    Clock -.实现.-> NanoMonotonicClock
    Clock -.实现.-> MockClock
    Clock -.实现.-> Zoned

    NanoClock -.实现.-> NanoMonotonicClock
    NanoClock -.实现.-> MockClock
    ZonedClock -.实现.-> Zoned
    ControllableClock -.实现.-> MockClock

    style Clock fill:#e1f5ff
    style NanoClock fill:#fff3e0
    style ZonedClock fill:#f3e5f5
    style ControllableClock fill:#e8f5e9
```

Sleep 模块使用独立的类型关系，不把 relative sleep 混入 UTC 时钟抽象：

```mermaid
graph TD
    Sleeper[Sleeper trait<br/>阻塞 relative sleep]
    AsyncSleeper[AsyncSleeper trait<br/>Tokio 异步 relative sleep]
    SystemSleeper[SystemSleeper<br/>真实 sleeper]
    MockSleeper[MockSleeper<br/>timeline-backed 模拟 sleeper]
    MockTimeline[MockTimeline<br/>共享 mock 时间源]
    MockClock[MockClock<br/>timeline-backed clock]
    MockTime[MockTime<br/>测试 facade]

    Sleeper -.实现.-> SystemSleeper
    Sleeper -.实现.-> MockSleeper
    AsyncSleeper -.tokio feature.-> SystemSleeper
    AsyncSleeper -.tokio feature.-> MockSleeper
    MockTimeline -.驱动.-> MockSleeper
    MockTimeline -.驱动.-> MockClock
    MockTime -.组合.-> MockTimeline
    MockTime -.提供.-> MockSleeper
    MockTime -.提供.-> MockClock

    style Sleeper fill:#e1f5ff
    style AsyncSleeper fill:#f3e5f5
    style MockSleeper fill:#e8f5e9
    style MockTimeline fill:#fff3e0
```

## 3. Trait 详细设计

### 3.1 Clock - 基础时钟 Trait

**职责**：提供 UTC 时间的基础接口

**定义**：
```rust
pub trait Clock: Send + Sync {
    /// 返回当前时间的毫秒时间戳（UTC）
    fn millis(&self) -> i64;

    /// 返回当前时间（UTC）
    fn time(&self) -> DateTime<Utc> {
        let millis = self.millis();
        DateTime::from_timestamp_millis(millis)
            .unwrap_or_else(|| clamp_out_of_range_millis(millis))
    }
}
```

**设计要点**：
- 所有方法返回 **UTC 时间**，不涉及时区
- `millis()` 是必须实现的方法，返回 Unix 时间戳（毫秒）
- `time()` 有默认实现，基于 `millis()` 构造 `DateTime<Utc>`
- 当毫秒时间戳超出 chrono 可表达范围时，`time()` 会 clamp 到最近边界，而不是回退到当前系统时间
- 要求 `Send + Sync`，确保线程安全

**适用场景**：
- 日志时间戳
- 性能监控
- 任何只需要 UTC 时间的场景

**文件位置**：`src/clock/clock.rs`

---

### 3.2 NanoClock - 高精度时钟 Trait

**职责**：提供纳秒级精度的时间测量

**定义**：
```rust
pub trait NanoClock: Clock {
    /// 返回当前时间的纳秒时间戳（UTC）
    fn nanos(&self) -> i128;

    /// 返回当前时间（UTC，纳秒精度）
    fn time_precise(&self) -> DateTime<Utc> {
        let nanos = self.nanos();
        let secs = nanos.div_euclid(1_000_000_000);
        let nsecs = nanos.rem_euclid(1_000_000_000) as u32;
        let secs = match i64::try_from(secs) {
            Ok(value) => value,
            Err(_) if nanos < 0 => return DateTime::<Utc>::MIN_UTC,
            Err(_) => return DateTime::<Utc>::MAX_UTC,
        };
        DateTime::from_timestamp(secs, nsecs).unwrap_or({
            if nanos < 0 {
                DateTime::<Utc>::MIN_UTC
            } else {
                DateTime::<Utc>::MAX_UTC
            }
        })
    }
}
```

**设计要点**：
- 继承自 `Clock`，是 Clock 的特化
- 使用 `i128` 存储纳秒时间戳，避免溢出
- 提供 `time_precise()` 方法，返回纳秒精度的 `DateTime`
- 使用欧几里得除法处理负数纳秒时间戳，超出 chrono 范围时 clamp 到最近边界
- **接口隔离**：不需要纳秒精度的实现不用提供此 trait

**适用场景**：
- 高精度性能测试
- 微基准测试（microbenchmark）
- 需要纳秒级精度的时间测量

**文件位置**：`src/clock/nano_clock.rs`

---

### 3.3 ZonedClock - 带时区的时钟 Trait

**职责**：提供时区支持，能够返回本地时间

**定义**：
```rust
pub trait ZonedClock: Clock {
    /// 返回此时钟的时区
    fn timezone(&self) -> Tz;

    /// 返回当前本地时间（使用此时钟的时区）
    fn local_time(&self) -> DateTime<Tz> {
        self.time().with_timezone(&self.timezone())
    }
}
```

**设计要点**：
- 继承自 `Clock`，是 Clock 的扩展
- `timezone()` 返回时钟关联的时区
- `local_time()` 返回本地时间，使用时钟自己的时区
- 默认实现通过 `time()` + `with_timezone()` 完成转换

**适用场景**：
- 用户界面显示
- 业务逻辑（订单创建时间、记录时间等）
- 任何需要显示本地时间的场景

**文件位置**：`src/clock/zoned_clock.rs`

---

### 3.4 ControllableClock - 可控制的时钟 Trait

**职责**：提供时钟控制功能，用于测试

**定义**：
```rust
pub trait ControllableClock: Clock {
    /// 设置时钟的当前时间
    fn set_time(&self, instant: DateTime<Utc>);

    /// 将时钟前进指定的时间
    fn add_duration(&self, duration: Duration);

    /// 重置时钟到初始状态
    fn reset(&self);
}
```

**设计要点**：
- 继承自 `Clock`，是 Clock 的扩展
- 提供三个控制方法：设置时间、增加时间、重置
- 主要用于测试场景，不应该在生产代码中使用

**适用场景**：
- 单元测试
- 集成测试
- 任何需要模拟时间的测试场景

**文件位置**：`src/clock/controllable_clock.rs`

---

### 3.5 Sleep Trait - Relative Sleep Trait

**职责**：提供可注入的 relative sleep 能力。

Sleep API 位于 `src/sleep` 模块下，不与 `Clock` 继承关系绑定。原因是
`Clock` 关注 UTC 当前时间，而 `Sleeper` 关注“暂停一段相对时长”。它不负责
deadline、notification wait 或 condition wait，这些属于同步原语或调度器。

**定义**：
```rust
pub trait Sleeper: Send + Sync {
    fn sleep_for(&self, duration: Duration);
}
```

启用 `tokio` feature 后还会导出：

```rust
pub type AsyncSleepFuture<'a> = Pin<Box<dyn Future<Output = ()> + Send + 'a>>;

pub trait AsyncSleeper: Send + Sync {
    fn sleep_for_async<'a>(&'a self, duration: Duration) -> AsyncSleepFuture<'a>;
}
```

**设计要点**：
- `Duration` 参数表示从方法调用时开始的一段相对时长
- `Sleeper::sleep_for()` 阻塞当前线程，不返回错误
- `AsyncSleeper::sleep_for_async()` 返回 boxed future，不引入 `async-trait` 依赖
- async sleep 的 duration 从方法调用时开始计算，而不是从首次 poll 开始计算
- sleep 模块不承载 notification 或 condition wait 语义

**适用场景**：
- 可测试的 retry / backoff 逻辑
- 不想在测试中等待真实时间的 relative sleep
- 需要在生产和测试中注入不同 sleep 实现的业务代码

**文件位置**：`src/sleep/*.rs`

## 4. 实现类型详细设计

### 4.1 SystemClock - 系统时钟

**职责**：提供基于系统时间的时钟实现

**定义**：
```rust
pub struct SystemClock;

impl Clock for SystemClock {
    fn millis(&self) -> i64 {
        Utc::now().timestamp_millis()
    }

    fn time(&self) -> DateTime<Utc> {
        Utc::now()
    }
}
```

**设计要点**：
- 零字段结构体（ZST），无运行时开销
- 直接调用系统 API 获取时间
- 受系统时间调整影响（NTP 同步、手动调整等）
- 只实现 `Clock` trait，不实现 `ZonedClock`

**使用示例**：
```rust
// 简单场景：只需要 UTC 时间
let clock = SystemClock::new();
let timestamp = clock.millis();

// 需要时区：使用 Zoned 包装
let clock = Zoned::new(SystemClock::new(), Shanghai);
let local = clock.local_time();
```

**线程安全性**：完全线程安全，无可变状态

**文件位置**：`src/clock/system_clock.rs`

---

### 4.2 MonotonicClock - 单调时钟

**职责**：提供单调递增的时钟，不受系统时间调整影响

**定义**：
```rust
pub struct MonotonicClock {
    instant_base: Instant,
    system_time_base_millis: i64,
}

impl Clock for MonotonicClock {
    fn millis(&self) -> i64 {
        self.system_time_base_millis
            .saturating_add(self.monotonic_millis())
    }
}
```

**设计要点**：
- 使用 `Instant` 作为时间源，保证单调性
- 在创建时记录基准点（`instant_base` 和 `system_time_base_millis`）
- 后续时间通过计算 `elapsed` 得出
- `millis()` 使用饱和加法，极端长运行或极端基准值下不会整数溢出
- 只实现 `Clock` trait，**不实现** `ZonedClock`

**为什么不实现 ZonedClock？**
- MonotonicClock 的主要用途是**测量时间间隔**，不是获取"当前时间"
- 时区对时间间隔测量没有意义
- 避免误导用户用它来获取本地时间

**适用场景**：
- 性能监控
- 超时控制
- 时间间隔测量
- 任何需要稳定、单调时间源的场景

**使用示例**：
```rust
let clock = MonotonicClock::new();
let start = clock.millis();

// 执行操作
do_something();

let elapsed = clock.millis() - start;
println!("耗时: {} ms", elapsed);
```

**线程安全性**：完全线程安全，所有字段不可变

**文件位置**：`src/clock/monotonic_clock.rs`

---

### 4.3 NanoMonotonicClock - 高精度单调时钟

**职责**：提供纳秒级精度的单调时钟

**定义**：
```rust
pub struct NanoMonotonicClock {
    instant_base: Instant,
    system_time_base_seconds: i64,
    system_time_base_nanos: u32,
}

impl Clock for NanoMonotonicClock { /* ... */ }
impl NanoClock for NanoMonotonicClock { /* ... */ }
```

**设计要点**：
- 同时实现 `Clock` 和 `NanoClock`
- 使用 `Instant` 保证单调性
- 分别存储秒和纳秒作为墙钟基准，后续叠加 `Instant` 流逝时间
- `nanos()` 使用 `i128` 和饱和加法，`millis()` 转换为 `i64` 时使用饱和转换
- 只实现 `Clock` 和 `NanoClock`，不实现 `ZonedClock`

**适用场景**：
- 高精度性能测试
- 微基准测试
- 需要纳秒级精度的时间测量

**使用示例**：
```rust
let clock = NanoMonotonicClock::new();
let start = clock.nanos();

// 执行操作
do_something();

let elapsed = clock.nanos() - start;
println!("耗时: {} ns", elapsed);
```

**线程安全性**：完全线程安全，所有字段不可变

**文件位置**：`src/clock/nano_monotonic_clock.rs`

---

### 4.4 MockClock - 模拟时钟

**职责**：提供由共享 `MockTimeline` 驱动的可控制 UTC 时钟和纳秒时钟，用于测试。

**定义**：
```rust
pub struct MockClock {
    timeline: MockTimeline,
    anchor: Arc<Mutex<MockClockAnchor>>,
}

struct MockClockAnchor {
    initial_wall_origin_nanos: i128,
    wall_origin_nanos: i128,
}

impl Clock for MockClock { /* ... */ }
impl NanoClock for MockClock { /* ... */ }
impl ControllableClock for MockClock { /* ... */ }
```

**设计要点**：
- 同时实现 `Clock`、`NanoClock` 和 `ControllableClock`
- 由 `MockTimeline` 提供单调 elapsed time；测试不推进 timeline 时，读数保持冻结
- `MockClock::new()` 使用当前 UTC 时间作为初始墙钟锚点，`MockClock::at()` 可指定锚点
- `MockClock::with_timeline()` 允许多个 mock 组件共享同一套时间推进逻辑
- `set_time()` 在当前 timeline instant 上重新锚定墙钟读数，不改变 timeline elapsed time
- `add_duration()` 只接受非负 `chrono::Duration`，内部转换为 `std::time::Duration` 后推进 timeline
- `reset()` / `try_reset()` 会在没有 active waiter 时将 timeline 和墙钟锚点恢复到初始状态

**核心功能**：
1. **设置时间**：`set_time(instant)` - 将当前 timeline instant 对应的 UTC 读数设为指定时间
2. **推进时间**：`advance(duration)` / `add_duration(duration)` - 推进共享 mock timeline
3. **纳秒读数**：`nanos()` - 返回 Unix 纳秒时间戳
4. **共享 timeline**：`timeline()` - 暴露底层 `MockTimeline`，用于与 sleeper 或 monitor 共享
5. **重置**：`try_reset()` / `reset()` - 在没有 active waiter 时恢复初始状态

**适用场景**：
- 单元测试
- 集成测试
- 需要同时控制墙钟时间、纳秒时间和 relative sleep 的测试场景

**使用示例**：
```rust
#[test]
fn test_with_fixed_time() {
    let mock = MockTime::at(
        DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc),
    );
    let clock = mock.clock();

    mock.advance(std::time::Duration::from_secs(60));
    assert_eq!(clock.millis(), 1_704_067_260_000);
    // 测试逻辑...
}
```

**线程安全性**：完全线程安全，使用 `Mutex` 保护内部状态

**文件位置**：`src/mock/mock_clock.rs`

---

### 4.5 MockTimeline - 共享 mock 时间源

**职责**：为 mock clock、mock sleeper 和 timeout-aware 测试工具提供同一套单调时间推进逻辑。

**定义**：
```rust
pub struct MockTimeline {
    shared: Arc<MockTimelineShared>,
    async_event_sender: watch::Sender<u64>, // tokio feature
}

struct MockTimelineState {
    elapsed_nanos: u128,
    time_epoch: u64,
    event_epoch: u64,
    sleep_waiters: usize,
    deadline_waiters: usize,
}
```

**设计要点**：
- `elapsed_nanos` 是唯一的单调时间状态，所有组件都从这里读取 elapsed time
- `time_epoch` 用于表示时间推进事件，`event_epoch` 同时覆盖时间推进和外部状态通知
- `advance(duration)` 推进 elapsed time，并唤醒阻塞和异步 waiter
- `notify_external_change()` 不推进时间，只用于 monitor/condition 这类状态变化通知
- `wait_until()` / `wait_for()` 可作为通用 deadline wait 原语
- `wait_for_blocked_waiters()` 用真实超时辅助测试确认某类 waiter 已经进入等待状态
- `reset()` 在存在 active waiter 时返回 `MockTimeError::ActiveWaiters`，避免等待者观察到不一致的 timeline 回退

**核心功能**：
1. **当前 instant**：`now()` - 返回 `MockInstant`
2. **推进时间**：`advance(duration)` - 推进 mock elapsed time
3. **外部通知**：`notify_external_change()` - 唤醒 event waiter，不改变 elapsed time
4. **deadline 等待**：`wait_until(deadline)` / `wait_for(duration)`
5. **等待者观测**：`wait_for_blocked_waiters(kind, count, real_timeout)`
6. **重置**：`reset()` - 无 active waiter 时回到 elapsed zero

**适用场景**：
- retry/backoff 与 clock 读数要共享同一套时间推进的测试
- future `qubit-lock` mock monitor 的 timeout/notification 测试
- 需要测试“时间推进”和“状态通知”交织的同步逻辑

**线程安全性**：完全线程安全，clone 共享状态，阻塞路径使用 `Condvar`，异步路径使用 `tokio::sync::watch`

**文件位置**：`src/mock/mock_timeline.rs`

---

### 4.6 MockTime - mock runtime facade

**职责**：以一个入口构造共享同一 `MockTimeline` 的 `MockClock` 和 `MockSleeper`。

**定义**：
```rust
pub struct MockTime {
    timeline: MockTimeline,
    clock: MockClock,
    sleeper: MockSleeper,
}
```

**设计要点**：
- `MockTime::at(start)` 创建指定 UTC 锚点的 runtime
- `MockTime::unix_epoch()` 创建以 Unix epoch 为墙钟锚点的 runtime
- `clock()` 返回 cloneable `MockClock`
- `sleeper()` 返回 cloneable `MockSleeper`
- `advance(duration)` 统一推进 clock 和 sleeper 共享的 timeline
- `set_time(instant)` 只重新锚定 clock，不改变已有 sleep deadline

**适用场景**：
- 同一个测试需要同时注入 clock 和 sleeper
- 需要确保 retry、timeout、meter、业务过期判断都按照同一套 mock 时间前进
- 后续与 `MockMonitor` 组合时，作为测试里的统一时间控制入口

**文件位置**：`src/mock/mock_time.rs`

---

### 4.7 Zoned<C> - 时区包装器

**职责**：为任何 `Clock` 添加时区支持

**定义**：
```rust
pub struct Zoned<C: Clock> {
    clock: C,
    timezone: Tz,
}

impl<C: Clock> Clock for Zoned<C> { /* 委托给 clock */ }
impl<C: Clock> ZonedClock for Zoned<C> { /* ... */ }
impl<C: NanoClock> NanoClock for Zoned<C> { /* 委托给 clock */ }
impl<C: ControllableClock> ControllableClock for Zoned<C> { /* 委托给 clock */ }
impl<C: Clock> Deref for Zoned<C> { /* ... */ }
```

**设计要点**：
- 泛型包装器，可以包装任何实现了 `Clock` 的类型
- 实现 `Clock` trait（委托给内部 clock）
- 实现 `ZonedClock` trait（提供时区功能）
- 如果内部 clock 实现 `NanoClock`，则 `Zoned<C>` 同样实现 `NanoClock`
- 如果内部 clock 实现 `ControllableClock`，则 `Zoned<C>` 同样实现 `ControllableClock`
- **关键特性**：实现 `Deref`，可以直接访问内部 clock 的方法

**扩展 trait 透传的作用**：
```rust
let mock = MockClock::new();
let zoned = Zoned::new(mock, Shanghai);

// ✅ 现在可以作为 trait object 使用
let controllable: &dyn ControllableClock = &zoned;
controllable.set_time(some_time);

let nano = Zoned::new(NanoMonotonicClock::new(), Shanghai);
let precise: &dyn NanoClock = &nano;
let nanos = precise.nanos();
```

**Deref 的作用**：
```rust
let mock = MockClock::new();
let zoned = Zoned::new(mock, Shanghai);

// ✅ 通过 Deref，仍然可以直接调用 MockClock 的方法
zoned.set_time(some_time);      // ControllableClock 方法
zoned.add_duration(duration);   // ControllableClock 方法
zoned.local_time();             // ZonedClock 方法
```

**为什么使用 Deref？**
- 提供便利性，无需手动访问内部 clock
- trait object 场景由显式 trait 透传解决，Deref 只承担调用便利性
- 符合 Rust 的智能指针惯例

**同时提供显式访问方法**：
```rust
impl<C: Clock> Zoned<C> {
    pub fn inner(&self) -> &C {
        &self.clock
    }

    pub fn into_inner(self) -> C {
        self.clock
    }
}
```

**适用场景**：
- 为任何 Clock 动态添加时区支持
- 特别适合测试场景：`Zoned<MockClock>`

**使用示例**：
```rust
// 包装 SystemClock
let clock = Zoned::new(SystemClock::new(), Shanghai);
let local = clock.local_time();

// 包装 MockClock（测试场景）
let mock = MockClock::new();
let clock = Zoned::new(mock, Shanghai);
clock.set_time(some_time);  // 通过 Deref 调用
let local = clock.local_time();
```

**线程安全性**：取决于内部 Clock 的线程安全性

**文件位置**：`src/clock/zoned.rs`

---

### 4.8 SystemSleeper - 真实 sleeper

**职责**：基于真实 elapsed time 提供 relative sleep。

**定义**：
```rust
pub struct SystemSleeper;
```

**设计要点**：
- `sleep_for()` 使用 `std::thread::sleep()`
- 启用 `tokio` feature 后，`sleep_for_async()` 使用 `tokio::time::sleep()`
- 类型本身不保存状态，适合作为默认生产实现
- 不响应 notification，不处理 condition wait

**适用场景**：
- 生产环境中的相对 sleep
- retry/backoff 等需要可注入 sleeper 的逻辑
- 与 `MockSleeper` 共享同一套 sleep trait 的实现代码

**线程安全性**：零状态类型，天然线程安全

**文件位置**：`src/sleep/system_sleeper.rs`

---

### 4.9 MockSleeper - 可控制模拟 sleeper

**职责**：提供由 `MockTimeline` 驱动的 relative sleep。

**定义**：
```rust
pub struct MockSleeper {
    timeline: MockTimeline,
}
```

**设计要点**：
- `MockSleeper::new()` 创建独立 timeline，`MockSleeper::with_timeline()` 共享已有 timeline
- elapsed time 来自底层 `MockTimeline`，不依赖真实时间流逝
- 测试通过 `timeline.advance()` 或 `MockTime::advance()` 推进 elapsed time
- `sleep_for(duration)` 在调用时以当前 elapsed time 加上 `duration` 得到目标 elapsed time
- 阻塞路径注册 `MockWaiterKind::Sleep`，便于测试确认线程已经进入 sleep 等待
- 异步路径同样在 future 创建时注册 waiter，避免 future 创建后到首次 poll 前的时间推进被静默丢失
- 不提供 notification 或 condition wait；这些能力由 `qubit-lock` 的 monitor API 承担

**适用场景**：
- retry、backoff 等 relative sleep 逻辑的确定性测试
- 需要让 sleeper 与 clock 共用同一套 mock elapsed time 的测试
- 同时覆盖阻塞式和 Tokio 异步 sleep 逻辑

**线程安全性**：完全线程安全，clone 共享状态，所有状态读写由 `Mutex` 保护

**文件位置**：`src/sleep/mock_sleeper.rs`

## 5. 使用场景与示例

### 5.1 场景 1：简单日志（只需要 UTC 时间）

```rust
use qubit_clock::{Clock, SystemClock};

fn log_event(clock: &dyn Clock, event: &str) {
    let timestamp = clock.millis();
    println!("[{}] {}", timestamp, event);
}

fn main() {
    let clock = SystemClock::new();
    log_event(&clock, "Application started");
}
```

**说明**：
- 只需要 UTC 时间戳
- 使用 `SystemClock` 即可
- 不需要时区支持

---

### 5.2 场景 2：性能监控（需要单调性）

```rust
use qubit_clock::{Clock, MonotonicClock};
use std::sync::Arc;

struct PerformanceMonitor {
    clock: Arc<dyn Clock>,
}

impl PerformanceMonitor {
    pub fn measure<F>(&self, name: &str, f: F)
    where F: FnOnce()
    {
        let start = self.clock.millis();
        f();
        let elapsed = self.clock.millis() - start;
        println!("{}: {} ms", name, elapsed);
    }
}

fn main() {
    let monitor = PerformanceMonitor {
        clock: Arc::new(MonotonicClock::new()),
    };

    monitor.measure("task1", || {
        // 执行任务
    });
}
```

**说明**：
- 使用 `MonotonicClock` 保证时间单调性
- 不受系统时间调整影响
- 适合性能监控场景

---

### 5.3 场景 3：业务逻辑（需要本地时间）

```rust
use qubit_clock::{Clock, ZonedClock, SystemClock, Zoned};
use chrono_tz::Asia::Shanghai;
use std::sync::Arc;

struct Order {
    id: String,
    created_at: DateTime<Tz>,
}

struct OrderService {
    clock: Arc<dyn ZonedClock>,
}

impl OrderService {
    pub fn create_order(&self, id: String) -> Order {
        Order {
            id,
            created_at: self.clock.local_time(),
        }
    }
}

fn main() {
    let service = OrderService {
        clock: Arc::new(Zoned::new(SystemClock::new(), Shanghai)),
    };

    let order = service.create_order("ORDER-001".to_string());
    println!("订单创建时间（上海）: {}", order.created_at);
}
```

**说明**：
- 使用 `Zoned<SystemClock>` 获取本地时间
- 订单时间显示为上海时区
- 适合业务逻辑场景

---

### 5.4 场景 4：单元测试（需要控制时间）

```rust
use qubit_clock::{Clock, ZonedClock, ControllableClock, MockClock, Zoned};
use chrono_tz::Asia::Shanghai;
use std::sync::Arc;

#[test]
fn test_order_creation() {
    // 创建模拟时钟
    let mock = MockClock::new();
    let clock = Zoned::new(mock, Shanghai);

    // 设置固定时间（UTC 2024-01-01 00:00:00）
    clock.set_time(
        DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    );

    // 创建服务
    let service = OrderService {
        clock: Arc::new(clock),
    };

    // 创建订单
    let order = service.create_order("TEST-001".to_string());

    // 验证时间是上海时间 2024-01-01 08:00:00
    assert_eq!(order.created_at.hour(), 8);
    assert_eq!(order.created_at.day(), 1);
}
```

**说明**：
- 使用 `Zoned<MockClock>` 进行测试
- 通过 `Deref`，可以直接调用 `set_time()` 方法
- 可以精确控制测试时间

---

### 5.5 场景 5：高精度性能测试

```rust
use qubit_clock::{NanoClock, NanoMonotonicClock};

fn benchmark(clock: &dyn NanoClock) {
    let start = clock.nanos();

    // 执行操作
    for _ in 0..1000 {
        // 一些快速操作
    }

    let elapsed = clock.nanos() - start;
    println!("耗时: {} ns", elapsed);
    println!("平均: {} ns/op", elapsed / 1000);
}

fn main() {
    let clock = NanoMonotonicClock::new();
    benchmark(&clock);
}
```

**说明**：
- 使用 `NanoMonotonicClock` 获取纳秒精度
- 适合微基准测试
- 可以测量非常短的时间间隔

---

### 5.6 场景 6：可测试 sleep 控制

```rust
use qubit_clock::MockTime;
use qubit_clock::sleep::Sleeper;
use std::time::Duration;

fn retry_after_delay<S>(sleeper: &S)
where
    S: Sleeper,
{
    sleeper.sleep_for(Duration::from_millis(100));
    retry_once();
}

#[test]
fn test_sleep_without_real_time() {
    let mock = MockTime::unix_epoch();
    let sleeper = mock.sleeper();
    let worker = sleeper.clone();

    let handle = std::thread::spawn(move || {
        retry_after_delay(&worker);
    });

    mock.advance(Duration::from_millis(100));
    handle.join().expect("retry should finish");
}
```

**说明**：
- `Duration` 参数表示从调用点开始的一段相对时长
- 测试中使用 `MockTime::advance()` 或 `MockTimeline::advance()` 可以瞬间推进 sleep，不需要真实等待
- notification wait、condition wait 和 timeout wait 由 `qubit-lock` 的 monitor API 承担

## 6. 文件组织结构

```
rs-clock/
├── src/
│   ├── lib.rs                    # 模块导出和文档
│   ├── clock/                    # Clock trait 和实现
│   ├── meter/                    # TimeMeter / NanoTimeMeter
│   └── sleep/                    # Sleeper / SystemSleeper / MockSleeper
├── tests/
│   ├── clock/                    # Clock trait 和实现测试
│   ├── clock_tests.rs            # Clock 测试入口
│   ├── meter/                    # 时间计量器测试
│   ├── meter_tests.rs            # Meter 测试入口
│   ├── sleep/                    # Sleep 模块测试
│   ├── sleep_tests.rs            # Sleep 测试入口
│   └── readme_api_tests.rs       # README API 回归测试
├── doc/
│   └── clock_design.zh_CN.md     # 本设计文档
├── Cargo.toml
└── README.md
```

**组织原则**：
1. 每个 trait 单独一个文件
2. 每个实现类型单独一个文件
3. `src/clock/xxx.rs` 对应 `tests/clock/xxx_tests.rs`
4. `src/sleep/xxx.rs` 对应 `tests/sleep/xxx_tests.rs`
5. 测试代码与源代码分离
6. 所有组件在同一个 crate 中

## 7. 设计优势

### 7.1 职责分离清晰

- **Clock**：只提供 UTC 时间
- **ZonedClock**：只添加时区支持
- **NanoClock**：只添加纳秒精度
- **ControllableClock**：只添加控制功能
- **Sleeper / AsyncSleeper**：只表达 relative sleep

每个 trait 职责单一，互不干扰。

### 7.2 类型安全

```rust
// 编译期就知道是否支持时区
fn need_zoned(clock: &dyn ZonedClock) {
    let local = clock.local_time();  // ✅ 编译通过
}

fn need_zoned_wrong(clock: &dyn Clock) {
    // let local = clock.local_time();  // ❌ 编译错误：Clock 没有 local_time
}
```

### 7.3 零成本抽象

```rust
// 不需要时区？不引入时区包装
let clock = MonotonicClock::new();  // 小对象，只保存基准时间

// 需要时区？只在需要时添加
let clock = Zoned::new(MonotonicClock::new(), Shanghai);  // 包装 clock 并增加一个 Tz 字段
```

### 7.4 灵活组合

```rust
// 可以为任何 Clock 添加时区
let zoned_system = Zoned::new(SystemClock::new(), Shanghai);
let zoned_mock = Zoned::new(MockClock::new(), Shanghai);
let zoned_monotonic = Zoned::new(MonotonicClock::new(), Shanghai);
```

### 7.5 测试友好

```rust
// Zoned<MockClock> 通过 Deref 同时支持：
// - Clock 接口
// - ZonedClock 接口
// - ControllableClock 接口

let mock = MockClock::new();
let clock = Zoned::new(mock, Shanghai);

clock.set_time(time);       // ControllableClock
clock.local_time();         // ZonedClock
clock.millis();             // Clock
```

`MockSleeper` 则让 sleep 相关测试不依赖真实时间：

```rust
let mock = MockTime::unix_epoch();
let sleeper = mock.sleeper();
let worker = sleeper.clone();

std::thread::spawn(move || {
    worker.sleep_for(Duration::from_secs(5));
});

mock.advance(Duration::from_secs(5));
```

## 8. 设计权衡

### 8.1 类型数量 vs 灵活性

**权衡**：设计引入了多个 trait 和类型，增加了学习成本

**收益**：
- 类型安全
- 职责清晰
- 灵活组合
- 零成本抽象

**结论**：收益大于成本，类型数量是必要的复杂度

### 8.2 Deref 与 trait 透传的语义

**争议**：Deref 通常用于智能指针，仅靠 Deref 做能力转发会导致 trait object 场景不完整

**解决方案**：
1. `Zoned<C>` 显式透传 `NanoClock` 和 `ControllableClock`
2. Deref 只作为调用便利性保留，不作为能力边界的唯一表达方式
3. 同时提供 `inner()` 和 `into_inner()` 方法供显式访问
4. 在文档中明确说明 Deref 的用途

### 8.3 MonotonicClock 与 ZonedClock

**设计决策**：MonotonicClock 不实现 ZonedClock

**理由**：
- MonotonicClock 用于测量时间间隔，不是获取"当前时间"
- 时区对时间间隔测量没有意义
- 避免误导用户

**如果用户非要添加时区？**
- 技术上可以：`Zoned::new(MonotonicClock::new(), tz)`
- 但语义上不推荐，应在文档中说明

## 9. 未来扩展

### 9.1 可能的扩展方向

1. **OffsetClock**：支持固定偏移量的时钟
2. **TickClock**：按固定间隔跳动的时钟
3. **SystemClockWithOffset**：系统时钟 + 偏移量

### 9.2 演进边界

- mock time 语义以共享 `MockTimeline` 为核心，旧的彼此独立 mock clock / mock sleeper 模型不再保留
- 新增 timeout-aware primitive 时应优先复用 `MockTimeline`，避免各组件各自维护一套 mock 时间
- 破坏性 API 变更需要同步 README、设计文档和测试示例，避免文档继续暴露旧模型

## 10. 参考资料

### 10.1 设计参考

- Java `java.time.Clock` 类设计
- Rust `std::time::Instant` 和 `SystemTime`
- Chrono crate 的时间处理

### 10.2 相关标准

- Unix 时间戳标准
- ISO 8601 时间格式
- IANA 时区数据库

## 11. 总结

本设计提供了一套清晰、类型安全、灵活的时钟抽象：

1. **职责分离**：通过多个正交的 trait 分离不同功能
2. **类型安全**：编译期保证功能支持
3. **零成本抽象**：不需要的功能不付出代价
4. **灵活组合**：通过 `Zoned<C>` 包装器灵活组合功能
5. **测试友好**：`MockClock`、`MockSleeper` 和 `MockTime` 支持共享 timeline 的确定性测试

这个设计在简洁性、灵活性、类型安全之间取得了良好的平衡，适合各种使用场景。
