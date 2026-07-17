# rs-clock Timer 与单调时间域重构设计

## 背景

`rs-clock` 的首要目标不是为 `std::time` 增加一层同名包装，而是为真实应用
提供可注入的时间能力：生产环境使用真实时间，集成测试替换时间驱动器后仍然
运行同一份业务代码，并且不必真实等待退避、超时或定时任务。

当前 `BlockingSleeper` 与 `AsyncSleeper` 把“如何等待”按调用方式拆成两套
trait 和多套实现。`rs-lock` 的同步 Monitor 又依赖条件变量自己的超时能力，
异步 Monitor 则直接依赖 Tokio 定时器。结果是测试只能换成行为不同的
`MockMonitor`，无法用同一个生产 Monitor 配合手动时间完成集成测试。

本设计允许任意破坏性变更，并以一个可被同步和异步消费者共同使用的
`Timer` 作为统一等待驱动器。

## 设计目标

- 生产代码与集成测试使用同一具体组件，只替换注入的时间驱动器。
- 一个 `Timer` 既能唤醒异步任务，也能通过适配器唤醒阻塞线程。
- Clock、Timer 和 `MonotonicInstant` 始终属于同一个可验证的时间域。
- 手动时间能够确定性驱动 retry、Monitor、线程池和调度器，不等待真实时间。
- API 明确区分“注册一个异步定时事件”和“阻塞当前执行上下文”。
- 不引入全局 `TimeServices`/IOC 容器；消费者只依赖自身真正需要的能力。

## 非目标

- 不保留 `AsyncSleeper`、各后端 Sleeper 或旧名称的兼容层。
- 不让 `BlockingSleeper` 继承或实现 `Timer`。
- 不在 `TimerFuture` 完成阶段承载运行期错误。
- 不保证不同 Timer 实例之间的公平性或相同截止时间的完成顺序。
- 不把 wall clock 与 monotonic timer 合并为一个通用时间对象。

## 核心公共 API

### Timer

```rust
pub type TimerFuture =
    Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

pub trait Timer: Send + Sync {
    fn clock(&self) -> &dyn MonotonicClock;

    fn at(
        &self,
        deadline: MonotonicInstant,
    ) -> Result<TimerFuture, TimeError>;

    fn after(
        &self,
        duration: Duration,
    ) -> Result<TimerFuture, TimeError> {
        let deadline = self.clock().now().checked_add(duration)?;
        self.at(deadline)
    }
}
```

`at` 和 `after` 表示创建定时事件，而不是阻塞线程，因此不使用 `wait_*`
或 `sleep_*`。真正会阻塞的 Monitor 和 `BlockingSleeper` 继续使用
`wait_*`/`sleep_*`。

`TimerFuture` 的输出固定为 `()`。时钟域不匹配、截止时间溢出、后端定时驱动
不可用或注册失败都必须由 `at`/`after` 在返回 Future 之前通过
`Result` 报告。`TimeError` 增加 `TimerUnavailable`，统一表示后端定时驱动
不可用或无法建立注册；后端不得把这些可预期失败转换为 panic。

### MonotonicClock 创建 Timer

```rust
pub trait MonotonicClock: Send + Sync {
    fn now(&self) -> MonotonicInstant;

    fn new_timer(&self) -> Arc<dyn Timer>;
}
```

`new_timer` 是单调时间域的核心接口。它不消费 Clock，因此对
`Arc<dyn MonotonicClock>` 可以直接调用：

```rust
let clock: Arc<dyn MonotonicClock> =
    Arc::new(StdMonotonicClock::new());
let timer = clock.new_timer();

// clock 仍然可用。
let now = clock.now();
```

返回的 Timer 必须满足以下约束：

- `timer.clock()` 返回的 Clock 与创建者属于同一个 `ClockDomain`；
- Timer 不借用调用 `new_timer` 的对象，返回值具有独立的 `'static` 生命周期；
- 原 Clock 被移动或释放后，Timer 仍然保持其时间域和驱动状态有效；
- 不要求 `timer.clock()` 与原 Clock 是同一对象分配，只要求时间域和时间线
  完全一致；
- 重复调用 `new_timer` 可以创建独立 Timer，但它们必须观察同一时间线。

为了实现借用式工厂，具体 Clock 必须让 Timer 持有同一时间域的内部 handle。
`ManualMonotonicClock` 将可变时间线和 waiter registry 移入私有共享状态；
`StdMonotonicClock` 与 `TokioMonotonicClock` 则复制其 domain/origin handle。
这些内部复制不要求公开 Clock 类型实现 `Clone`。

Rust 不需要 C/C++ 式 forward declaration。`MonotonicClock` 与 `Timer` 可以
在兄弟模块中通过 crate 路径互相引用；这只是接口关联，不会形成递归的
sized value。

## Timer 的行为契约

### 注册与完成

- `at` 必须在返回前建立定时注册，而不是推迟到第一次 poll。
- 截止时间已经到达时，`at` 返回立即 `Ready` 的 Future。
- Future 尚未被 poll 时即使时间已经到达，完成状态也必须被锁存；第一次
  poll 直接返回 `Ready`。
- 第一次及后续 poll 只负责安装或替换 Waker、观察锁存状态，不改变截止时间。
- 同一个 Future 可以经历任意次通知或伪唤醒，截止时间始终固定。
- 丢弃未完成 Future 必须立即取消其注册并释放仅属于它的资源。
- 注册成功后，Future 的正常完成不再产生 `TimeError`。

### 相对时间

`after(duration)` 在方法调用时读取一次 `clock().now()`，计算并固定绝对
deadline，然后委托给 `at`。延迟 poll 不得延后截止时间。

### 时间域

`at` 只接受 `timer.clock()` 所属 domain 的 `MonotonicInstant`。外来 domain
返回 `TimeError::ClockDomainMismatch`。Timer 生命周期内 `clock()` 暴露的
domain 不得改变。

### 运行时与线程安全

`Timer` 本身为 `Send + Sync`，创建出的 Future 为 `Send + 'static`。
Tokio 后端仍然受其 runtime time driver 约束；调用 `at` 时无法建立定时器应
返回错误。阻塞消费者不得在禁止阻塞的 async runtime worker 上使用
`BlockingSleeper`。

## Timer 实现

### StdTimer

`StdMonotonicClock::new_timer()` 返回绑定该 Clock domain/origin 的
`StdTimer`。每个显式 Timer 实例拥有一个共享 scheduler 状态和 deadline
最小堆；后台 worker 在第一次成功注册未来 deadline 时惰性启动，而不是每个
Future 启动一个线程。

注册、取消和新增更早 deadline 都会唤醒 worker 重新计算等待时间。Future
完成状态在 scheduler 中锁存，Waker 在锁外调用。worker 启动失败通过
`TimeError` 返回。

默认生产 Monitor 可以在 `rs-lock` 内部共享一个进程级 StdTimer，以避免每个
Monitor 建立独立 worker。显式调用 `new_timer` 仍创建独立 Timer，不依赖该
全局默认值。

### TokioTimer

启用 `tokio` feature 时，`TokioMonotonicClock::new_timer()` 返回
`TokioTimer`。`at` 把 domain-scoped deadline 转换为对应的
`tokio::time::Instant` 并立即创建定时注册。它支持 Tokio paused time 和
`advance`，但 Clock 的采样、Timer 注册和 Future poll 必须由同一个 runtime
time driver 管理。

外来 domain、native instant 溢出以及 runtime/time driver 不可用都在
`at` 返回时报告。丢弃 Future 依靠 Tokio timer 的取消语义释放注册。

### ManualTimer

`ManualMonotonicClock::new_timer()` 返回共享该 Clock 私有时间域状态的
`ManualTimer`。`at` 在返回前加入 manual waiter registry；推进时间会锁存
所有已到期注册并在锁外唤醒它们。

Timer 注册必须被现有测试控制面统一观察：

- `pending_waiters()` 包含所有未取消、未完成的 Timer 注册；
- `next_deadline()` 返回当前最早的未来 deadline；
- `wait_for_waiters()` 与异步对应方法可以等待 Timer 注册出现；
- `wait_for_next_deadline()` 与异步对应方法观察 Timer deadline；
- `advance_to_next_deadline()` 原子选择并推进到当前最早 deadline。

已到达但尚未被 Future poll 清理的 waiter 不再算作未来 deadline。取消注册
必须立即影响上述观察结果。

## BlockingSleeper

`BlockingSleeper` 不再是 trait，也不再按后端提供多个具体类型。它是组合
`Timer` 的通用阻塞适配器：

```rust
#[derive(Clone)]
pub struct BlockingSleeper {
    timer: Arc<dyn Timer>,
}

impl BlockingSleeper {
    pub fn new(timer: Arc<dyn Timer>) -> Self;

    pub fn timer(&self) -> &dyn Timer;

    pub fn sleep_until(
        &self,
        deadline: MonotonicInstant,
    ) -> Result<(), TimeError>;

    pub fn sleep_for(
        &self,
        duration: Duration,
    ) -> Result<(), TimeError>;
}
```

`sleep_until`/`sleep_for` 创建一个 `TimerFuture`，使用私有 blocking Waker
进行 poll，并阻塞当前线程直到 Future ready。Waker 必须记忆 wake-before-park，
避免注册完成或时间推进发生在真正 park 之前时丢失唤醒。

生产和测试使用相同的 `BlockingSleeper`：生产注入 `StdTimer`，测试注入
`ManualTimer`。它不实现 `Timer`，也不暴露任何继承关系。

## 删除和替换

删除以下公共 API 及其内部实现，不提供 deprecated alias：

- `AsyncSleeper`
- `SleepFuture`
- `TokioAsyncSleeper`
- `ManualAsyncSleeper`
- `BlockingSleeper` trait
- `StdBlockingSleeper`
- `ManualBlockingSleeper`
- `ManualAdvanceSubscription`
- `ManualMonotonicClock::subscribe_advances`
- `ManualMonotonicClock::new_async_sleeper`
- `ManualMonotonicClock::new_blocking_sleeper`

`TimerFuture` 替换 `SleepFuture`；`StdTimer`、`TokioTimer`、`ManualTimer`
替换各后端 sleeper。`ManualDeadlineFuture` 与 waiter/deadline 协调 API 保留，
但观察对象改为 Timer 注册。

## IOC 使用模型

组件只注入实际需要的能力：

- 只读取时间：`Arc<dyn MonotonicClock>`；
- 建立异步 deadline：`Arc<dyn Timer>`；
- 阻塞等待：具体 `BlockingSleeper`；
- 同时需要 deadline 和采样：持有 Timer，并通过 `timer.clock()` 采样。

不增加通用 `TimeServices` 容器。composition root 负责创建 Clock、从 Clock
创建 Timer，再把 Timer 或 BlockingSleeper 分发给组件。

测试示例：

```rust
let clock = ManualMonotonicClock::new_shared();
let timer = clock.new_timer();
let sleeper = BlockingSleeper::new(Arc::clone(&timer));

// sleeper/业务任务运行同一生产路径。
// 测试线程通过 clock 等待注册并推进逻辑时间。
```

## 下游迁移方向

- `rs-lock`：所有真实 Monitor 注入 `Arc<dyn Timer>`，删除 MockMonitor。
- `rs-retry`：异步 retry 注入 Timer；同步 retry 注入 BlockingSleeper。
- `rs-executor`：把 `std::time::Instant`/`thread::sleep` 迁移为
  `MonotonicInstant`、Timer/BlockingSleeper；调度 worker 使用 Monitor 的
  deadline wait。
- `rs-thread-pool`：为 keepalive Monitor 注入 Timer。
- Rayon/Tokio executor：字段继续使用具体 Monitor 类型，构造阶段注入 Timer，
  不把 Monitor 本身改成动态 trait object。

## 测试要求

`rs-clock` 的外部测试至少覆盖：

- `clock.new_timer()` 对具体 Clock 和 `Arc<dyn MonotonicClock>` 都可直接调用，
  且调用后 Clock 仍可使用；
- 每种内建 Clock 创建的 Timer 与自身 domain 一致；
- `after` 在调用时固定 deadline；
- 未 poll Future 的到期锁存；
- 已到期 deadline 立即 ready；
- 外来 domain、overflow 和 `TimerUnavailable` 错误；
- Future drop 立即取消注册；
- Waker 替换和 wake-before-first-poll；
- ManualTimer 与全部 waiter/deadline 协调方法一致；
- 多个 Timer 共享一条 ManualClock 时间线但注册相互独立；
- BlockingSleeper 用 StdTimer 和 ManualTimer 运行同一实现；
- Tokio paused time、取消和 runtime affinity；
- StdTimer 大量 deadline、取消、提前插入 deadline 和 worker 生命周期。

测试继续位于与源码结构对应的 `tests/` 外部目录。实现顺序遵循 TDD：先建立
新 API/语义的失败测试，再替换旧实现，最后迁移下游。

## 与既有设计的关系

本设计在 Timer、Sleeper 和手动时间部分取代
`2026-07-16-manual-time-ergonomics-design.md`。既有 wall-clock 设计保持不变；
旧文档中关于 `AsyncSleeper`、后端 BlockingSleeper 和 advance subscription
的结论不再适用。
