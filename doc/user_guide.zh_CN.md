# Qubit Clock 用户手册

Qubit Clock 提供可注入的 wall clock、monotonic clock、阻塞 sleeper 和异步
sleeper。其 manual 实现可以让测试等待已注册的 deadline，并在不等待真实时间的
情况下推进逻辑时间。

英文版参见 [English User Guide](user_guide.en.md)。

## 1. 按能力选择，而不是按实现选择

与时间有关的代码通常只需要下列四种能力之一：

| 需求 | 注入类型 | 主要操作 |
|---|---|---|
| 民用时间戳、日期、审计字段 | `WallClock` | `now() -> SystemTime` |
| 耗时、timeout 计算、deadline | `MonotonicClock` | `now() -> MonotonicInstant` |
| 阻塞当前线程 | `BlockingSleeper` | `sleep_for`、`sleep_until` |
| 挂起异步任务 | `AsyncSleeper` | `sleep_for_async`、`sleep_until_async` |

本库刻意分离 wall time 与 monotonic time。系统校时后，wall clock 可能向前或向后
跳变，因此它适合表示现实世界的时间戳，不适合测量耗时。一个 clock domain 内的
monotonic clock 永不倒退，因此它才是 duration、deadline、retry 和 timeout 的
正确时间源。

每个 sleeper 都持有与自己配对的 monotonic clock，并通过 `clock()` 暴露它。既要
计算 deadline 又要 sleep 的生产组件，通常只需要注入 sleeper：

```rust
use qubit_clock::BlockingSleeper;
use std::time::Duration;

fn wait_for_batch(sleeper: &dyn BlockingSleeper) -> Result<(), qubit_clock::TimeError> {
    let started = sleeper.clock().now();
    sleeper.sleep_for(Duration::from_millis(20))?;
    let elapsed = sleeper.clock().now().duration_since(started)?;
    assert!(elapsed >= Duration::from_millis(20));
    Ok(())
}
```

不要在 sleeper 旁边再注入一个无关的 monotonic clock。如果组件确实需要两个
handle，就必须用同一个 clock 构造 sleeper，确保两者属于同一个 `ClockDomain`。

## 2. 安装与 feature

标准实现和 manual 实现不需要可选 feature：

```toml
[dependencies]
qubit-clock = "0.9"
```

使用 `TokioMonotonicClock` 和 `TokioAsyncSleeper` 时启用 `tokio`：

```toml
[dependencies]
qubit-clock = { version = "0.9", features = ["tokio"] }
```

`ManualAsyncSleeper` 以及 manual 协调 future 与 executor 无关，不要求启用
`tokio` feature；只需有某个 executor 负责 poll 它们。

## 3. 实现一览

| 能力 | 真实时间实现 | 确定性实现 |
|---|---|---|
| Wall time | `StdWallClock` | `FixedWallClock`、`ManualWallClock` |
| Monotonic time | `StdMonotonicClock`、`TokioMonotonicClock` | `ManualMonotonicClock` |
| 阻塞 sleep | `StdBlockingSleeper` | `ManualBlockingSleeper` |
| 异步 sleep | `TokioAsyncSleeper` | `ManualAsyncSleeper` |

只要 `T` 实现了相应能力 trait，`Arc<T>` 和 `Box<T>` 就会自动实现该 trait。因此
共享的具体类型无需 adapter，即可直接转换为 `Arc<dyn WallClock>`、
`Arc<dyn MonotonicClock>`、`Arc<dyn BlockingSleeper>` 或
`Arc<dyn AsyncSleeper>`。

## 4. Wall clock

### 4.1 系统 wall time

生产环境的民用时间使用 `StdWallClock`：

```rust
use qubit_clock::{StdWallClock, WallClock};

let clock = StdWallClock::new();
let created_at = clock.now();
println!("created at {created_at:?}");
```

`StdWallClock` 是零大小、可 `Copy` 的类型，底层使用 `SystemTime::now()`。

### 4.2 固定 wall time

如果每次读取都应返回同一个不可变时间戳，使用 `FixedWallClock`：

```rust
use qubit_clock::{FixedWallClock, WallClock};
use std::time::{Duration, UNIX_EPOCH};

let expected = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
let clock = FixedWallClock::new(expected);
assert_eq!(expected, clock.now());
assert_eq!(expected, clock.fixed_time());
```

对于只给记录添加时间戳的代码，这是最轻量的测试替身。

### 4.3 由 manual monotonic time 驱动的 wall time

如果 wall time 需要与 manual deadline 一起推进，使用 `ManualWallClock`：

```rust
use qubit_clock::{ManualMonotonicClock, WallClock};
use std::time::{Duration, UNIX_EPOCH};

let clock = ManualMonotonicClock::new_shared();
let wall_clock = clock.new_wall_clock(UNIX_EPOCH);

clock
    .advance(Duration::from_secs(60))
    .expect("manual time 应推进成功");
assert_eq!(UNIX_EPOCH + Duration::from_secs(60), wall_clock.now());

wall_clock.reanchor(UNIX_EPOCH + Duration::from_secs(3_600));
assert_eq!(UNIX_EPOCH + Duration::from_secs(3_600), wall_clock.now());
```

`reanchor()` 只改变 wall-time 映射。它可以让 wall time 向前或向后跳变，但不会
改变 manual monotonic time，也不会唤醒 sleeper。只有当 anchor 加上已推进的
manual time 超出平台可表示的 `SystemTime` 范围时，
`ManualWallClock::now()` 才会 panic。

## 5. Monotonic clock、domain 与 instant

### 5.1 标准 monotonic time

`StdMonotonicClock` 把 `std::time::Instant` 映射到一个私有原点：

```rust
use qubit_clock::{MonotonicClock, StdMonotonicClock};

let clock = StdMonotonicClock::new();
let start = clock.now();
let end = clock.now();
let elapsed = end
    .duration_since(start)
    .expect("两个 instant 应属于同一个 clock domain");
println!("elapsed: {elapsed:?}");
```

每个新建的 monotonic clock 都拥有不同的 `ClockDomain`。
`MonotonicInstant` 同时携带该 domain 和从 clock 私有原点起算的 duration。其主要
操作包括：

- `domain()`：取得来源 timeline 的标识；
- `elapsed_since_origin()`：查看 domain 内的相对时长；
- `checked_add(duration)`：构造同 domain 的 deadline；
- `duration_since(earlier)`：测量同 domain 内的时间间隔。

不同 domain 的 instant 不能混用。对跨 domain 值调用 `duration_since` 会返回
`TimeError::ClockDomainMismatch`，用 `PartialOrd` 比较则返回 `None`。这样可以避免
把一个 clock 产生的 deadline 静默传给另一个 sleeper。

### 5.2 Manual monotonic time

`ManualMonotonicClock` 从零时长开始，只能通过测试控制面改变：

```rust
use qubit_clock::{ManualMonotonicClock, MonotonicClock};
use std::time::Duration;

let clock = ManualMonotonicClock::new_shared();
assert_eq!(Duration::ZERO, clock.now().elapsed_since_origin());

clock
    .advance(Duration::from_secs(2))
    .expect("manual time 应推进成功");
assert_eq!(Duration::from_secs(2), clock.now().elapsed_since_origin());

let target = clock
    .now()
    .checked_add(Duration::from_secs(3))
    .expect("目标时间应可表示");
clock
    .advance_to(target)
    .expect("目标应属于同一 domain 且不早于当前时间");
assert_eq!(Duration::from_secs(5), clock.now().elapsed_since_origin());
```

`advance(duration)` 相对向前推进；`advance_to(target)` 绝对推进到同 domain 的
目标。向后移动会返回 `TimeError::CannotMoveBackward`，传入其他 domain 的目标会
返回 `TimeError::ClockDomainMismatch`。推进零时长是成功的 no-op，也不会发出
advance 通知。

## 6. 阻塞 sleeper

### 6.1 真实阻塞 sleep

`StdBlockingSleeper::new()` 会同时创建 sleeper 及其配对的标准 monotonic clock：

```rust
use qubit_clock::{BlockingSleeper, StdBlockingSleeper};
use std::time::Duration;

let sleeper = StdBlockingSleeper::new();
sleeper
    .sleep_for(Duration::from_millis(10))
    .expect("阻塞 sleep 应正常完成");
```

如果其他组件必须保留完全相同的 clock identity，就从同一个 `Arc` 构造两者：

```rust
use qubit_clock::{StdBlockingSleeper, StdMonotonicClock};
use std::sync::Arc;

let clock = Arc::new(StdMonotonicClock::new());
let sleeper = StdBlockingSleeper::from_clock(Arc::clone(&clock));
```

### 6.2 确定性阻塞 sleep

测试中保留共享 manual clock，并从它派生 sleeper：

```rust
use qubit_clock::{BlockingSleeper, ManualMonotonicClock};
use std::time::Duration;

let clock = ManualMonotonicClock::new_shared();
let sleeper = clock.new_blocking_sleeper();
let worker = std::thread::spawn(move || {
    sleeper.sleep_for(Duration::from_secs(10))
});

let observed = clock
    .wait_for_next_deadline(Duration::from_secs(1))
    .expect("worker 应注册 deadline");
assert_eq!(Duration::from_secs(10), observed.elapsed_since_origin());

clock
    .advance_to_next_deadline()
    .expect("worker deadline 应仍然有效");
worker
    .join()
    .expect("worker 不应 panic")
    .expect("manual sleep 应正常完成");
```

传给 `wait_for_next_deadline` 的 timeout 是真实时间测试护栏，它绝不会推进逻辑时间。

## 7. Tokio 异步 sleep

`TokioAsyncSleeper` 跟随 Tokio time driver，需要启用 `tokio` feature：

```rust
use qubit_clock::{AsyncSleeper, TokioAsyncSleeper};
use std::time::Duration;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), qubit_clock::TimeError> {
let sleeper = TokioAsyncSleeper::new();
sleeper.sleep_for_async(Duration::from_millis(10)).await?;
Ok(())
}
```

创建返回的 `SleepFuture` 是惰性的，本身不要求存在 runtime；第一次 poll 必须位于
启用了 time 的 Tokio runtime 中。

当 Tokio time 被暂停或显式推进时，必须在同一个 runtime time driver 下创建、读取
配对的 `TokioMonotonicClock`，并 poll 所有 sleeper future。任务可以在这个 runtime
的 worker thread 之间迁移，但 clock/sleeper 组合不能跨独立 runtime 移动。本库
无法取得并校验 driver identity，因此这是调用方契约。

只有在其他组件必须保留同一个 clock identity 时，才需要使用
`TokioAsyncSleeper::from_clock(Arc<TokioMonotonicClock>)`；否则 `new()` 更简单。

## 8. 构造一套 manual time family

推荐的 manual 配置只有一个控制面 clock，所有消费能力都从它派生：

```rust
use qubit_clock::{AsyncSleeper, BlockingSleeper, ManualMonotonicClock, WallClock};
use std::sync::Arc;
use std::time::UNIX_EPOCH;

let clock = ManualMonotonicClock::new_shared();
let wall_clock = clock.new_wall_clock(UNIX_EPOCH);
let blocking_sleeper = clock.new_blocking_sleeper();
let async_sleeper = clock.new_async_sleeper();

let injected_wall: Arc<dyn WallClock> = wall_clock;
let injected_blocking: Arc<dyn BlockingSleeper> = blocking_sleeper;
let injected_async: Arc<dyn AsyncSleeper> = async_sleeper;
```

factory 方法返回具体的 `Arc<Manual...>`，所以测试可以保留 manual 专用 API，生产
builder 则仍然接收普通 trait object。所有派生组件都共享完全相同的 clock allocation
和 domain。重复调用 factory 只会创建基于同一 timeline 的独立轻量 wrapper，不会
创建新的 clock。

底层的 `ManualWallClock::from_clock`、`ManualBlockingSleeper::from_clock` 和
`ManualAsyncSleeper::from_clock` 构造器仍然可用。普通测试应优先使用 clock factory，
因为它能清晰表达同 domain 组装关系，并消除重复的 `Arc::new` 和 `Arc::clone` 模板
代码。

## 9. Manual sleep future 的行为

`ManualAsyncSleeper` 会在调用 `sleep_for_async()` 或 `sleep_until_async()` 时注册
waiter，而不是等到返回的 future 第一次被 poll。因此：

- 尚未 poll 的 sleep future 对 `pending_waiters`、deadline observer 和
  `advance_to_next_deadline` 已经可见；
- 相对 deadline 固定在调用 sleep 方法的时刻，而不是第一次 poll 的时刻；
- 如果第一次 poll 前已经推进到 deadline，该次 poll 会直接 ready；
- 丢弃未完成的 future 会立即注销 waiter；
- 传入其他 domain 的 deadline 会产生立即 ready 的错误 future。

这种 eager registration 是 manual sleep 特有的设计，用于实现确定性协调。
`TokioAsyncSleeper` 则会在第一次 poll 时才创建底层 timer。

`SleepFuture` 是所有 async sleeper 实现共同使用的 object-safe 返回类型：

```rust
type SleepFuture = Pin<Box<dyn Future<Output = Result<(), TimeError>> + Send + 'static>>;
```

调用方通常只通过 `AsyncSleeper` 使用它，不需要显式写出该 alias。

## 10. Count 协调与 deadline 协调

Manual clock 提供了两种语义不同的同步机制：

| API | 语义 | 最适合的场景 |
|---|---|---|
| `pending_waiters()` | 当前注册数量；已到期 async waiter 在再次 poll 或 drop 前仍计数 | 诊断与断言 |
| `wait_for_waiters(count, real_timeout)` | 阻塞、锁存 count 阈值 | 已知 waiter 数量的启动屏障 |
| `wait_for_waiters_async(count)` | 异步、锁存 count 阈值 | 异步启动屏障 |
| `next_deadline()` | 当前最早有效未来 deadline 的快照 | 检查状态 |
| `wait_for_next_deadline(real_timeout)` | 阻塞等待当前有效未来 deadline | 阻塞多阶段驱动 |
| `wait_for_next_deadline_async()` | 当前有效未来 deadline 的异步状态观察器 | 异步多阶段驱动 |
| `advance_to_next_deadline()` | 原子选择并推进到当前最早未来 deadline | 驱动逻辑时间 |

Count observer 类似事件：一旦达到阈值，即使 waiter 在 observer 再次被 poll 前已经
消失，完成状态仍会锁存。已到期的 async waiter 在其 future 再次被 poll 或 drop 前，
仍可能计入 count。

Deadline observer 基于当前状态：已取消和已到期的 waiter 都不能满足它。retry loop
等多阶段操作尤其需要这个区别，因为上一阶段已到期的 waiter 可能尚未清理，而下一
阶段正在创建。

## 11. `wait_for_next_deadline_async` 的精确语义

`wait_for_next_deadline_async()` 返回 `ManualDeadlineFuture`，并遵循以下规则：

1. 调用方法时立即注册 observer，早于第一次 poll。这样可以消除“创建 observer”和
   “poll 生产者”之间的丢失唤醒窗口。
2. 注册动作不会预留或锁存某个 waiter。每次 poll 都会在持有 manual clock mutex
   时检查当前状态。
3. 如果存在有效 waiter，本次 poll 返回严格晚于当前 manual time 的最早 deadline，
   并删除 observer。
4. 如果不存在有效未来 deadline，本次 poll 会保存或更新 task waker，并返回
   `Pending`。
5. 注册未来 waiter 会唤醒 pending observer；但如果 waiter 在 observer 下一次 poll
   前已经取消，则它会被忽略。
6. deadline 已经到期的 waiter 同样会被忽略，即使其 future 尚未清理注册。
7. 丢弃 pending 的 `ManualDeadlineFuture` 会注销 observer。

Ready instant 是在状态锁内选出的时间点快照，而不是预留。另一个任务可能紧接着注册
更早的 deadline。因此 observer 只用于同步；随后应调用
`advance_to_next_deadline()`，原子选择并推进到届时真正最早的 deadline：

```rust
let _observed = clock.wait_for_next_deadline_async().await;
let reached = clock
    .advance_to_next_deadline()
    .expect("应仍存在有效未来 deadline");
```

除非测试已经用其他方式证明不可能再注册更早的 deadline，否则不要在并发驱动中使用
`clock.advance_to(observed)`。

## 12. 多阶段异步驱动示例

被测 operation 必须与 deadline observer 并发 poll。可以 spawn operation，也可以
使用 `select!`。下面用 `select!` 驱动两次连续 sleep：

```rust
use qubit_clock::{AsyncSleeper, ManualMonotonicClock, TimeError};
use std::time::Duration;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), TimeError> {
let clock = ManualMonotonicClock::new_shared();
let sleeper = clock.new_async_sleeper();
let mut operation = Box::pin(async move {
    sleeper.sleep_for_async(Duration::from_secs(2)).await?;
    sleeper.sleep_for_async(Duration::from_secs(3)).await?;
    Ok::<_, TimeError>("done")
});

for expected_elapsed in [Duration::from_secs(2), Duration::from_secs(5)] {
    tokio::select! {
        result = &mut operation => {
            panic!("operation 在 deadline 前完成：{result:?}");
        }
        _ = clock.wait_for_next_deadline_async() => {}
    }

    let reached = clock
        .advance_to_next_deadline()
        .expect("operation 应存在有效 deadline");
    assert_eq!(expected_elapsed, reached.elapsed_since_origin());
}

assert_eq!("done", operation.await?);
Ok(())
}
```

仅仅构造 async block 并不会执行它。如果 operation 既没有 spawn，也没有在
`select!` 中被 poll，它就无法注册 sleeper deadline，此时 observer 保持 pending
是正确行为。

## 13. 混合阻塞与异步 waiter

同一个 manual clock 可以同时驱动两种 sleeper。`pending_waiters`、
`next_deadline` 和 `advance_to_next_deadline` 会统一考虑两类注册。这适合测试 worker
thread 与 async task 共享一条逻辑 timeline 的系统：

```rust
let clock = ManualMonotonicClock::new_shared();
let blocking = clock.new_blocking_sleeper();
let asynchronous = clock.new_async_sleeper();
```

在测试 harness 中保留具体的 `clock`，只向每个生产组件注入它需要的派生能力。

## 14. Advance subscription

`subscribe_advances` 面向维护自身 condition variable、channel 或 task-waker 集合的
同步测试替身：

```rust
use qubit_clock::ManualMonotonicClock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

let clock = ManualMonotonicClock::new_shared();
let notified = Arc::new(AtomicBool::new(false));
let callback_flag = Arc::clone(&notified);
let subscription = clock.subscribe_advances(move || {
    callback_flag.store(true, Ordering::SeqCst);
});

clock
    .advance(Duration::from_secs(1))
    .expect("manual time 应推进成功");
assert!(notified.load(Ordering::SeqCst));
drop(subscription); // 注销后续 callback
```

Callback 在 clock mutex 释放后同步执行。它应当可重复调用、执行迅速，并且只负责
通知另一个等待原语。并发 advance 可能并发、无序地调用 callback。需要通知期间必须
保留返回的 `ManualAdvanceSubscription`；drop 后不会再注册未来 callback，但某次
进行中的 advance 已捕获的 callback 仍可能再执行一次。

## 15. Trait object 注入

应用代码应依赖最窄的能力 trait：

```rust
use qubit_clock::{AsyncSleeper, WallClock};
use std::sync::Arc;

struct SessionService {
    wall_clock: Arc<dyn WallClock>,
    sleeper: Arc<dyn AsyncSleeper>,
}
```

生产组装可以注入 `StdWallClock` 和 `TokioAsyncSleeper`；测试则注入从同一个 manual
clock 派生的 `ManualWallClock` 与 `ManualAsyncSleeper`。业务组件无需增加“测试模式”。

如果要实现自定义 `MonotonicClock`，应在构造 clock 时只分配一次 `ClockDomain`，并
在每次 `now()` 时重复使用；绝不能每次采样都创建新 domain。自定义 sleeper 必须
校验传入 deadline，并通过 `clock()` 暴露同一个稳定 domain。

## 16. 错误处理

`TimeError` 标记为 `#[non_exhaustive]`，匹配时必须保留 wildcard 分支：

```rust
use qubit_clock::TimeError;

fn classify(error: TimeError) -> &'static str {
    match error {
        TimeError::ClockDomainMismatch { .. } => "wrong clock",
        TimeError::InstantOverflow => "deadline overflow",
        TimeError::CannotMoveBackward => "backward manual advance",
        TimeError::InvalidInstantOrder => "reversed interval",
        _ => "other time error",
    }
}
```

各 variant 的含义是：

- `ClockDomainMismatch`：deadline 或 instant 来自另一个 monotonic clock；
- `InstantOverflow`：duration 运算或底层 deadline 转换无法表示；
- `CannotMoveBackward`：`ManualMonotonicClock::advance_to` 收到了更早的同 domain
  目标；
- `InvalidInstantOrder`：传给 `duration_since` 的 instant 晚于 receiver。

## 17. 推荐模式与常见错误

- 对外部有意义的时间戳使用 `WallClock`，测量耗时则使用 `MonotonicClock` 或
  sleeper。
- Manual 测试从 `ManualMonotonicClock::new_shared()` 开始，并通过
  `new_wall_clock`、`new_blocking_sleeper`、`new_async_sleeper` factory 派生组件。
- 只在测试 harness 中保留具体 manual clock；向生产组件注入窄 trait object。
- 已知 waiter 数量的启动屏障使用 count 协调。
- Retry、重复 timeout 和多阶段工作流使用 deadline 协调。
- 观察到 deadline 后，优先使用 `advance_to_next_deadline()`，不要用
  `advance_to(observed)`，以保证选择与推进是原子的。
- 确保被测 async operation 确实在被 poll。
- 不要比较、排序或相减不同 clock domain 的 instant。
- 不要让使用暂停时间的 Tokio clock/sleeper 组合跨独立 runtime 移动。
- 必须保留 advance subscription handle；drop 它就会注销订阅。
- 阻塞协调方法的真实时间 timeout 只是测试护栏，绝不是逻辑时间。

遵循这些模式，可以让生产 API 保持精简，同时让测试用一套显式、确定性的控制面管理
所有依赖时间的组件。
