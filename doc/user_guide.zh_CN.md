# Qubit Clock 用户手册

时间相关代码经常隐藏了最重要的依赖。组件一旦直接调用 `SystemTime::now()`、
`Instant::now()` 或休眠函数，就已经自行选择了时钟和等待机制。单元测试只能真实
等待、与调度器竞争，或者在生产代码中加入模拟测试专用分支。

`qubit-clock` 提供适合依赖注入（IoC）的时钟和定时器 trait。组件只依赖自己需要的
能力；应用组装层在生产环境中提供标准或 Tokio 实现，在测试中提供固定或可手动推进的
时间。组件本身无需改变。

## 第一个例子

假设一个会话需要在指定时长后过期。过期属于耗时逻辑，应使用单调时钟，避免受到墙上
时钟校准或跳变的影响：

```rust
use qubit_clock::{
    ManualMonotonicClock, MonotonicClock, MonotonicInstant, StdMonotonicClock,
    TimeError,
};
use std::{sync::Arc, time::Duration};

struct Session {
    clock: Arc<dyn MonotonicClock>,
    expires_at: MonotonicInstant,
}

impl Session {
    fn new(
        clock: Arc<dyn MonotonicClock>,
        ttl: Duration,
    ) -> Result<Self, TimeError> {
        let expires_at = clock.now().checked_add(ttl)?;
        Ok(Self { clock, expires_at })
    }

    fn is_expired(&self) -> bool {
        self.clock.now() >= self.expires_at
    }
}

fn main() -> Result<(), TimeError> {
    let _production = Session::new(
        Arc::new(StdMonotonicClock::new()),
        Duration::from_secs(30),
    )?;

    let clock = ManualMonotonicClock::new_shared();
    let session = Session::new(clock.clone(), Duration::from_secs(30))?;
    assert!(!session.is_expired());

    clock.advance(Duration::from_secs(30))?;
    assert!(session.is_expired());
    Ok(())
}
```

生产环境组装时注入 `StdMonotonicClock`。测试保留 `ManualMonotonicClock` 作为时间
控制面，同时把另一个引用注入会话。推进逻辑时间即可立即到达精确的过期边界，不需要
休眠、替换全局时钟、设置测试开关或复制一套过期算法。

## 注入模式

应选择能够准确表达组件行为的最小依赖：

- 组件需要生成对外有意义的时间戳时，注入 `Arc<dyn WallClock>`。
- 组件只需采样耗时或检查截止时间而不等待时，注入
  `Arc<dyn MonotonicClock>`。
- 组件需要异步等待截止时间时，注入 `Arc<dyn Timer>`；定时器同时提供同域的单调
  时钟。
- 同步代码必须阻塞等待时，用注入的定时器构造 `BlockingSleeper`。

具体实现只应出现在组装层。生产环境选择 `Std*` 或 `Tokio*`，测试选择
`FixedWallClock` 或从 `ManualMonotonicClock` 派生的能力。这是普通的依赖注入，不是
另一套测试模式。

## 组件

| 需求 | API | 生产实现 | 确定性测试实现 |
|---|---|---|---|
| 现实世界时间戳 | `WallClock` | `StdWallClock` | `FixedWallClock`、`ManualWallClock` |
| 单调时刻 | `MonotonicClock` | `StdMonotonicClock`、`TokioMonotonicClock` | `ManualMonotonicClock` |
| 异步截止时间 | `Timer` | `StdTimer`、`TokioTimer` | `ManualTimer` |
| 阻塞等待 | `BlockingSleeper` | 组合可独立推进的定时器 | 组合由测试推进的 `ManualTimer` |

墙上时钟时间可能跳变，适合表达对外有意义的时间戳。单调时刻属于私有时钟域，适合
超时、重试等待和耗时测量。

### `WallClock`

`WallClock::now()` 返回 `SystemTime`，用于需要离开当前进程或具有日历含义的值，
例如创建时间、审计记录、协议时间戳和持久化元数据。生产环境使用
`StdWallClock`；测试只需要一个常量时使用 `FixedWallClock`；需要与手动单调时间线
一起推进时使用 `ManualWallClock`。墙上时钟可能跳变，不应拿来测量耗时。

### `MonotonicClock`

`MonotonicClock::now()` 返回带时钟域的 `MonotonicInstant`，同一时钟域内的值永不
倒退。它适合过期判断、耗时预算、重试策略和超时计算。不同单调时钟域中的时刻不能
混用。每个单调时钟都能通过 `new_timer()` 创建同域的 `Timer`。

### `Timer`

`Timer` 将单调截止时间转换为 Future。`after(duration)` 在调用时固定相对截止时间，
`at(instant)` 接受来自同一时钟域的绝对截止时间。异步组件需要等待、让操作与超时
竞争或安排重试间隔时，应注入它。

### `BlockingSleeper`

`BlockingSleeper` 通过轮询 `Timer` 的 Future 并停驻调用线程，为同步代码提供适配。
它不会创建另一套时间模型：截止时间计算和时间推进仍由注入的定时器负责。

## 安装

标准实现和手动实现都包含在默认 feature 集中：

```toml
[dependencies]
qubit-clock = "0.12"
```

生产代码需要 `TokioMonotonicClock` 或 `TokioTimer` 时启用 `tokio`：

```toml
[dependencies]
qubit-clock = { version = "0.12", features = ["tokio"] }
```

手动时钟、手动定时器及其协调 Future 与执行器无关，不需要启用它。

## 使用 `Timer`

每种单调时钟都能直接创建同域定时器，而且创建定时器后仍可继续使用原时钟：

```rust
use qubit_clock::{MonotonicClock, StdMonotonicClock, Timer};
use std::time::Duration;

# async fn example() -> Result<(), qubit_clock::TimeError> {
let clock = StdMonotonicClock::new();
let timer = clock.new_timer();
timer.after(Duration::from_millis(10))?.await?;
let _still_usable = clock.now();
# Ok(())
# }
```

`Timer::after` 在调用期间采样时钟并固定绝对截止时间；`Timer::at` 接受绝对
`MonotonicInstant`，也会在返回前固定截止时间。返回的 `TimerFuture` 输出
`Result<(), TimeError>`，只负责等待这个固定时刻。具体后端可以等到 Future 首次被
轮询时，才向原生调度器登记。丢弃尚未完成的 Future 会取消对应通知。

如果 `StdTimer` 的调度工作线程意外退出，它会唤醒由该线程负责的 Future。这些 Future
在下一次轮询时返回包含 `SchedulerWorkerTerminated` 的 `TimerUnavailable`，不会永久
挂起，也不会误报截止时间已经到达。后续注册会启动新的工作线程。

## Tokio Timer

`TokioMonotonicClock` 与 `TokioTimer` 需要启用 `tokio` feature，二者都会保存 Tokio
运行时的 `Handle`。`current()` 和 `try_current()` 在构造时捕获当前 Handle；当前
线程未进入 Tokio 运行时时，`try_current()` 返回
`TokioRuntimeError::NotEntered`。在 IoC 组装边界应优先使用
`from_handle(handle)`：

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
runtime
    .block_on(deadline)
    .expect("deadline should complete");
```

采样时钟和创建 `Sleep` 时会短暂进入保存的 Handle，不依赖调用方当前所在的运行时。
因此，返回的定时器 Future 可以在其他线程或运行时上下文中轮询，但截止时间仍由目标
运行时推进：只要 Future 尚未完成，目标 `Runtime` 的所有者就必须存活并持续驱动时间。
目标运行时未启用时间驱动时，未来的截止时间返回
`TimerUnavailableError::TimeDriverDisabled`；已经到达的截止时间会直接返回就绪
Future，不需要时间驱动。丢弃挂起的 Future 会取消本次等待。

Tokio `Sleep` 的调度粒度为毫秒级，不适合高分辨率计时。`TokioTimer` 会保留完整的
逻辑 `Duration`，但 Future 的实际完成时机仍受该粒度和平台调度延迟影响。

Tokio 当前没有公开 API 可查询 `Handle` 是否带有时间驱动；它会在创建未来的
`Sleep` 时通过 panic 报告未启用状态。允许栈展开时，`TokioTimer` 通过
`catch_unwind` 将其转换为 `TimeDriverDisabled`，但进程的 panic hook 会先运行，
仍可能记录或观察到这次 panic；使用 `panic = "abort"` 构建时则无法恢复。临时替换
进程级全局 hook 会与应用自己的 panic 处理竞争，因此本库不会这样做。在 Tokio 当前
公开 API 的限制下，应为注入 `TokioTimer` 的每个运行时启用时间驱动。

下游测试可在开发依赖中启用默认关闭的 `test-util` feature。
`FaultInjectingTimer` 可以复用且不依赖具体运行时，用于模拟登记失败和完成失败；
正常的时钟域校验及已到达截止时间的行为保持不变。

<a id="manual-time-coordination"></a>

## 确定性 Manual Time

测试应保留手动时钟作为时间控制面，并把它创建的定时器注入被测组件。

手动定时器及其协调 Future 不绑定执行器，也不需要启用 `qubit-clock/tokio`
feature。本节只使用 Tokio 运行、派生和选择任务；在测试中运行这些示例时，需要直接
声明 Tokio 开发依赖：

```toml
[dev-dependencies]
tokio = { version = "1", features = ["macros", "rt"] }
```

确定性手动时间可以控制逻辑时间、截止时间的选择和到达，但不规定同一截止时间上多个
等待者的唤醒顺序，也不规定执行器轮询就绪任务的顺序。

```rust
use qubit_clock::{ManualMonotonicClock, MonotonicClock, Timer};
use std::time::Duration;

# #[tokio::main(flavor = "current_thread")]
# async fn main() -> Result<(), Box<dyn std::error::Error>> {
let clock = ManualMonotonicClock::new_shared();
let timer = clock.new_timer();
let task = tokio::spawn(async move {
    timer.after(Duration::from_secs(8))?.await?;
    Ok::<_, qubit_clock::TimeError>(())
});

let reached = clock.advance_to_next_deadline_async().await;
assert_eq!(Duration::from_secs(8), reached.elapsed_since_origin());
task.await??;
# Ok(())
# }
```

手动定时器会立即登记等待：即使定时器 Future 尚未首次被轮询，
`pending_waiters`、`next_deadline` 和各协调 API 也已经能够观察到它。如果手动时间
先到达截止点，完成状态仍会保留。

- `pending_waiters()` 与 `next_deadline()` 返回即时快照。
- `wait_for_waiters()` 与 `wait_for_waiters_async()` 是等待者数量屏障；一旦观察到
  指定数量，之后即使等待者取消，屏障的完成状态仍会保留。
- `wait_for_next_deadline()` 与 `wait_for_next_deadline_async()` 观察严格晚于当前
  手动时间的最早有效截止点。返回值只是快照，不能直接作为原子推进的目标。
- `advance_to_next_deadline()` 以原子方式选择并到达当前最早的未来截止点；不存在时返回
  `None`。
- `advance_to_next_deadline_after_waiters()` 会阻塞到当前等待者数量达到阈值且存在
  未来截止点，并在同一个时钟状态锁内检查两个条件和推进，从而消除观察数量与推进
  时间之间的取消窗口。
- `advance_to_next_deadline_async()` 先等待有效的未来截止点，再以原子方式推进；
  如果取消操作先发生，它会重新等待。取消驱动 Future 不会推进手动时间。

### 驱动多个阶段

如果任务会依次登记多个截止时间，可以让“任务完成”与“手动时间推进”两个 Future
竞争。这样无需猜测任务会执行多少阶段，也不会在任务完成后继续等待不存在的截止点：

```rust
# use qubit_clock::{ManualMonotonicClock, MonotonicClock, Timer};
# use std::time::Duration;
# #[tokio::main(flavor = "current_thread")]
# async fn main() -> Result<(), Box<dyn std::error::Error>> {
let clock = ManualMonotonicClock::new_shared();
let timer = clock.new_timer();
let mut task = tokio::spawn(async move {
    timer.after(Duration::from_secs(1))?.await?;
    timer.after(Duration::from_secs(2))?.await?;
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

### 运行时所有权与取消

手动协调 Future 与运行时无关：它们是普通的 Rust Future，可由任意执行器轮询。取消
观察或驱动 Future 只会移除本次观察，不会取消定时器等待者。
`TokioMonotonicClock` 与 `TokioTimer` 则保存明确的运行时能力：采样和定时器登记使用
该 Handle，返回的 Future 可以在其他执行上下文中轮询。取消 Future 会移除 Tokio
`Sleep`；移动 Future 不会把截止时间的所有权转交给执行轮询的运行时。Handle 指向的
目标运行时必须保持存活并持续推进。

## `WallClock` 投影与重新锚定

`ManualWallClock` 从共享的单调时间线投影现实世界时间：

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

`reanchor` 只改变 `WallClock` 映射，不会移动单调时间线，也不会改变截止时间或
定时器登记。

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

适配器轮询定时器 Future，并且只停驻当前调用线程；线程停驻后，定时器后端仍须能够
推进。`StdTimer` 自带调度工作线程；`ManualTimer` 必须由其他线程或测试控制方推进；
`TokioTimer` 必须由它保存的运行时独立驱动。不要在单线程 Tokio 运行时的唯一驱动
线程上，阻塞等待绑定到该运行时的定时器。

## 在相关库中的应用

`rs-lock` 向支持超时的 monitor 实现中注入定时器。生产环境使用标准或 Tokio
定时器；测试注入 `ManualMonotonicClock` 创建的定时器并直接推进到超时点，无需真实
等待八秒。

`rs-retry` 使用注入定时器的单调时钟计算耗时预算，并使用同一个定时器处理重试间隔和
单次尝试超时。因此，它的测试可以立即覆盖指数退避、精确超时边界和取消，不需要真实
等待。

这些库只是把 `Session` 示例中的模式用于更大的场景：组件依赖 trait，组装层选择具体
时钟，测试保留手动时间控制面。

## 基准测试

使用以下命令运行标准定时器调度器及其他后端的基准测试：

```bash
cargo bench --bench std_timer_scheduler
cargo bench --bench manual_timer
cargo bench --bench tokio_timer --features tokio
```

标准定时器基准测试分别报告 1、2、4、8、16 个并发调用线程下的登记、取消和截止时间
完成吞吐。手动定时器基准测试测量 1、8、32、128、1,024 个等待者下的登记、取消、
批量完成和顺序完成。Tokio 基准测试在 1,024 和 10,240 个挂起截止时间下，比较原生
`Sleep`、保留的旧版逐截止时间哨兵和共享哨兵实现。

`BlockingSleeper`、`StdTimer` 与手动定时器登记表的同步状态机还提供直接覆盖生产
算法的 Loom 模型。使用以下命令运行全部模型：

```bash
RUSTFLAGS="--cfg loom" cargo test --release --all-features loom
```

Loom 模型测试名称都包含 `loom`。共享 CI 脚本会先发现模型再执行；如果一个模型都未
发现，检查会直接失败，避免空过滤条件被误判为通过。

## 错误

- `ClockDomainMismatch`：截止时间来自另一个单调时钟域。
- `InstantOverflow`：相对截止时间或原生截止时间转换发生溢出。
- `TimerUnavailable { source }`：调度工作线程、时间驱动或自定义后端不可用，导致
  截止时间登记或完成失败；`TimerUnavailableError` 标识具体后端，并尽可能保留原始
  错误。
- `TokioRuntimeError`：`try_current()` 无法捕获当前 Tokio 运行时；显式使用
  `from_handle` 可避开这个失败边界。
- `CannotMoveBackward`：调用方试图让手动时间倒退。
- `InvalidInstantOrder`：单调时刻的运算顺序无效。

公开错误枚举都使用 `#[non_exhaustive]`。调用方应只匹配自己能够处理的变体，并保留
兜底分支，以便未来增加后端错误时仍保持源码兼容。
