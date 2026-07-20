# Qubit Clock 用户手册

`qubit-clock` 将民用时间与单调调度分离，并让二者都可注入。应用代码无需区分生产与
测试：生产环境注入真实时间，集成测试注入 manual time 并主动推进逻辑时间。

## 能力模型

| 需求 | API | 生产实现 | 确定性测试实现 |
|---|---|---|---|
| 民用时间戳 | `WallClock` | `StdWallClock` | `FixedWallClock`、`ManualWallClock` |
| 单调时刻 | `MonotonicClock` | `StdMonotonicClock`、`TokioMonotonicClock` | `ManualMonotonicClock` |
| 异步 deadline | `Timer` | `StdTimer`、`TokioTimer` | `ManualTimer` |
| 阻塞等待 | `BlockingSleeper` | 组合可独立推进的 timer | 组合由外部推进的 `ManualTimer` |

Wall clock 可能跳变，适合表达对外有意义的时间戳。Monotonic instant 属于私有 clock
domain，适合 timeout、retry delay 和耗时测量。

## 创建 Timer

每种 monotonic clock 都能直接创建同域 timer，调用不会消耗 clock：

```rust
use qubit_clock::{MonotonicClock, StdMonotonicClock, Timer};
use std::time::Duration;

# async fn example() -> Result<(), qubit_clock::TimeError> {
let clock = StdMonotonicClock::new();
let timer = clock.new_timer();
timer.after(Duration::from_millis(10))?.await;
let _still_usable = clock.now();
# Ok(())
# }
```

`Timer::after` 在调用期间采样 clock 并固定绝对 deadline；`Timer::at` 接受绝对
`MonotonicInstant`，同样在返回前固定 deadline。返回的 `TimerFuture` 输出为 `()`，
只等待这个固定 deadline；具体后端可以到 future 首次 poll 时才向原生 scheduler
登记。丢弃未完成 future 会取消尚未完成的通知。

## Tokio Timer

`TokioMonotonicClock` 与 `TokioTimer` 需要启用 `tokio` feature，二者都会保存 Tokio
runtime `Handle`。`current()` 和 `try_current()` 在构造时捕获当前 Handle；没有当前
runtime 时，`try_current()` 返回 `TokioRuntimeError::NotEntered`。IoC 组装边界应优先
使用 `from_handle(handle)`：

```rust
use qubit_clock::{Timer, TokioTimer};
use std::time::Duration;

let runtime = tokio::runtime::Builder::new_current_thread()
    .enable_time()
    .build()
    .expect("Tokio runtime should build");
let timer = TokioTimer::from_handle(runtime.handle().clone());
let deadline = timer
    .after(Duration::from_millis(1))
    .expect("deadline should register on the retained runtime");
runtime.block_on(deadline);
```

clock 采样和 `Sleep` 创建会短暂进入保存的 Handle，不依赖调用方当前所在的 runtime。
因此返回的 timer future 可以在其他线程或 runtime context 中 poll，但 deadline 的推进
仍属于目标 runtime：只要 future 尚未完成，目标 `Runtime` 的所有者就必须存活，并持续
驱动 time driver。目标 runtime 未启用 time 时，未来 deadline 返回
`TimerUnavailableError::TimeDriverDisabled`；已经到达的 deadline 会直接返回 ready
future，不需要 time driver。drop pending future 会取消本次等待。

<a id="manual-time-coordination"></a>

## 确定性 Manual Time

保留 manual clock 作为测试控制面，把它创建的 timer 注入被测组件：

Manual timer 及其协调 future 不绑定 executor，也不需要启用
`qubit-clock/tokio` feature。本节示例只选择 Tokio 来运行、派生和选择任务；在测试中
运行这些示例时，需要直接声明 Tokio 开发依赖：

```toml
[dev-dependencies]
tokio = { version = "1", features = ["macros", "rt"] }
```

确定性 manual time 控制逻辑时间、deadline 选择和 deadline 完成，但不定义相同
deadline 的 waiter 唤醒顺序，也不定义 executor poll 就绪任务的顺序。

```rust
use qubit_clock::{ManualMonotonicClock, MonotonicClock, Timer};
use std::time::Duration;

# #[tokio::main(flavor = "current_thread")]
# async fn main() -> Result<(), Box<dyn std::error::Error>> {
let clock = ManualMonotonicClock::new_shared();
let timer = clock.new_timer();
let task = tokio::spawn(async move {
    timer.after(Duration::from_secs(8))?.await;
    Ok::<_, qubit_clock::TimeError>(())
});

let reached = clock.advance_to_next_deadline_async().await;
assert_eq!(Duration::from_secs(8), reached.elapsed_since_origin());
task.await??;
# Ok(())
# }
```

Manual timer 的注册是 eager 的：timer future 尚未首次 poll，`pending_waiters`、
`next_deadline` 和各协调 API 已能观察到它。若 manual time 先到达 deadline，完成
状态仍会被锁存。

- `pending_waiters()` 与 `next_deadline()` 返回即时快照。
- `wait_for_waiters()` 与 `wait_for_waiters_async()` 是 count barrier；一旦观察到
  阈值，之后即使 waiter 取消，barrier 的完成状态仍会锁存。
- `wait_for_next_deadline()` 与 `wait_for_next_deadline_async()` 观察严格晚于当前
  manual time 的最早有效 deadline。返回值只是快照，不能直接充当原子推进目标。
- `advance_to_next_deadline()` 原子选择并到达当前最早未来 deadline；不存在时返回
  `None`。
- `advance_to_next_deadline_after_waiters()` 会阻塞至当前 waiter 数达到阈值且存在未来
  deadline，并在同一个时钟状态锁内检查两个条件和推进，从而消除 count 观察与推进
  之间的取消窗口。
- `advance_to_next_deadline_async()` 先等待有效未来 deadline，再执行原子推进；若
  取消操作赢得竞争，它会重新等待。取消 driver future 不会推进 manual time。

### 驱动多个阶段

如果任务会依次注册多个 deadline，可让任务完成与 manual-time driver 竞争。这样
无需猜测任务会执行多少阶段，也不会在任务完成后继续等待一个不存在的 deadline：

```rust
# use qubit_clock::{ManualMonotonicClock, MonotonicClock, Timer};
# use std::time::Duration;
# #[tokio::main(flavor = "current_thread")]
# async fn main() -> Result<(), Box<dyn std::error::Error>> {
let clock = ManualMonotonicClock::new_shared();
let timer = clock.new_timer();
let mut task = tokio::spawn(async move {
    timer.after(Duration::from_secs(1))?.await;
    timer.after(Duration::from_secs(2))?.await;
    Ok::<_, qubit_clock::TimeError>(())
});

loop {
    tokio::select! {
        result = &mut task => {
            result??;
            break;
        }
        _ = clock.advance_to_next_deadline_async() => {}
    }
}
# Ok(())
# }
```

### Runtime 所有权与取消

Manual 协调 future 与 runtime 无关：它们是普通 Rust future，可由任意 executor
poll。取消 observer 或 driver future 只会移除本次观察，不会取消 timer waiter。
`TokioMonotonicClock` 与 `TokioTimer` 则保存明确的 runtime capability：采样和 timer
注册使用该 Handle，返回的 future 可以在其他执行 context 中 poll。取消 future 会
移除 Tokio sleep；移动 future 不会把 deadline 的所有权转交给 polling runtime。
保存 Handle 所指向的目标 runtime 必须保持存活并持续推进。

## Wall Clock 投影与重新锚定

Manual wall clock 从共享 monotonic timeline 投影民用时间：

```rust
use qubit_clock::{ManualMonotonicClock, WallClock};
use std::time::{Duration, UNIX_EPOCH};

let clock = ManualMonotonicClock::new_shared();
let wall_clock = clock.new_wall_clock(UNIX_EPOCH);
clock.advance(Duration::from_secs(5))?;
assert_eq!(UNIX_EPOCH + Duration::from_secs(5), wall_clock.now());

wall_clock.reanchor(UNIX_EPOCH + Duration::from_secs(100));
assert_eq!(UNIX_EPOCH + Duration::from_secs(100), wall_clock.now());
# Ok::<_, qubit_clock::TimeError>(())
```

`reanchor` 只改变 wall-time mapping，不会移动 monotonic timeline，也不会改变
deadline 或 timer 注册。

## 阻塞适配

`BlockingSleeper` 是具体适配器，不是另一套调度抽象：

```rust
use qubit_clock::{BlockingSleeper, MonotonicClock, StdMonotonicClock};
use std::time::Duration;

let clock = StdMonotonicClock::new();
let sleeper = BlockingSleeper::new(clock.new_timer());
sleeper.sleep_for(Duration::from_millis(10))?;
# Ok::<_, qubit_clock::TimeError>(())
```

适配器 poll timer future，并只 park 当前调用线程；线程 park 后，timer backend 仍须
能够推进。`StdTimer` 自带 scheduler worker；`ManualTimer` 必须由其他线程或测试控制方
advance；`TokioTimer` 必须由其保存的 runtime 独立驱动。不要在 current-thread runtime
的唯一驱动线程上阻塞等待绑定该 runtime 的 timer。

## IoC 组装

应用组件应按真实需求依赖 `Arc<dyn WallClock>`、`Arc<dyn MonotonicClock>` 或
`Arc<dyn Timer>`。生产组装注入 standard/Tokio 实现；集成测试注入 manual clock
创建的 timer，并显式推进该 clock。应用代码不需要测试模式或 mock 专用分支。

## 基准测试

进程级 standard timer scheduler 可通过以下命令运行基准测试：

```bash
cargo bench --bench std_timer_scheduler
```

基准测试分别报告 1、2、4、8、16 个并发调用线程下的注册/取消与 deadline 完成吞吐。

## 错误

- `ClockDomainMismatch`：deadline 来自另一个 monotonic domain。
- `InstantOverflow`：相对 deadline 或原生 deadline 转换溢出。
- `TimerUnavailable { source }`：scheduler worker、time driver 或自定义
  backend 不可用，导致 deadline 注册失败；`TimerUnavailableError` 标识 backend
  并保留可用的原始错误。
- `TokioRuntimeError`：`try_current()` 无法捕获当前 Tokio runtime；显式使用
  `from_handle` 可避开这个失败边界。
- `CannotMoveBackward`：manual time 被要求倒退。
- `InvalidInstantOrder`：instant 运算顺序无效。

公开 error enum 都使用 `#[non_exhaustive]`。调用方应只匹配自己能处理的 variant，
并保留 fallback 分支，使未来新增 backend 错误时仍保持源码兼容。
