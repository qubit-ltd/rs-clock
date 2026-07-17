# Qubit Clock 用户手册

`qubit-clock` 将民用时间与单调调度分离，并让二者都可注入。应用代码无需区分生产与
测试：生产环境注入真实时间，集成测试注入 manual time 并主动推进逻辑时间。

## 能力模型

| 需求 | API | 生产实现 | 确定性测试实现 |
|---|---|---|---|
| 民用时间戳 | `WallClock` | `StdWallClock` | `FixedWallClock`、`ManualWallClock` |
| 单调时刻 | `MonotonicClock` | `StdMonotonicClock`、`TokioMonotonicClock` | `ManualMonotonicClock` |
| 异步 deadline | `Timer` | `StdTimer`、`TokioTimer` | `ManualTimer` |
| 阻塞等待 | `BlockingSleeper` | 组合任意 `Timer` | 组合 `ManualTimer` |

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
`MonotonicInstant`。二者都在返回前完成注册。注册错误同步返回；返回的
`TimerFuture` 输出为 `()`，只表示 deadline 完成。丢弃未完成 future 会取消注册。

## Tokio Timer

`TokioMonotonicClock` 与 `TokioTimer` 需要启用 `tokio` feature。未来 deadline 的
注册必须发生在启用 time 的 runtime 中；缺失或禁用 time driver 时，`at`/`after`
立即返回 `TimeError::TimerUnavailable`。已经到达的 deadline 无需访问 runtime，
会直接返回 ready future。暂停时间时，clock 创建、未来 deadline 注册、时间推进与
future poll 必须位于同一个 runtime time driver。

## 确定性 Manual Time

保留 manual clock 作为测试控制面，把它创建的 timer 注入被测组件：

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

let deadline = clock.wait_for_next_deadline_async().await;
assert_eq!(Duration::from_secs(8), deadline.elapsed_since_origin());
clock.advance_to_next_deadline().expect("deadline 应仍然有效");
task.await??;
# Ok(())
# }
```

Manual 注册是 eager 的：future 尚未第一次 poll，`pending_waiters`、`next_deadline`、
`wait_for_waiters` 与 `wait_for_next_deadline_async` 已能观察到它。若先推进到 deadline
再 poll，完成状态会被锁存。`advance_to_next_deadline` 会原子选择并到达当前最早、
严格位于未来的有效 deadline。

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

适配器 poll timer future，并只 park 当前调用线程。同一个适配器也能组合 manual
timer，因此阻塞式集成测试同样无需真实等待。

## IoC 组装

应用组件应按真实需求依赖 `Arc<dyn WallClock>`、`Arc<dyn MonotonicClock>` 或
`Arc<dyn Timer>`。生产组装注入 standard/Tokio 实现；集成测试注入 manual clock
创建的 timer，并显式推进该 clock。应用代码不需要测试模式或 mock 专用分支。

## 错误

- `ClockDomainMismatch`：deadline 来自另一个 monotonic domain。
- `InstantOverflow`：相对 deadline 或原生 deadline 转换溢出。
- `TimerUnavailable`：无法创建 timer driver 或调度器。
- `CannotMoveBackward`：manual time 被要求倒退。
- `InvalidInstantOrder`：instant 运算顺序无效。
