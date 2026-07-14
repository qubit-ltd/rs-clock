# rs-clock 时钟 API 简化与手动时钟拆分实施计划

**状态：** 已实施并于 2026-07-15 完成复核；下方 checklist 保留为原始实施记录，
不再表示当前进度。

Goal: 用 ClockDomain 替换裸 domain ID，收紧 MonotonicClock，让 Sleeper 组合 clock，并拆分 ManualMonotonicClock 的内部状态职责。

Architecture: MonotonicClock 仅返回完整的 MonotonicInstant；Sleeper 用 clock() 组合其配对 clock；manual clock 仍只持有一把状态 mutex，但把 waiter 与 subscriber registry 拆为内部组件。

Tech Stack: Rust 1.94、edition 2024、标准库同步原语、可选 Tokio 1.52。

## 全局约束

- 不保留 0.9 API 兼容性；同时迁移受影响的 rs-retry 调用方。
- 不增加 WallClock::try_now()；domain 耗尽继续 panic。
- StdMonotonicClock 与 TokioMonotonicClock 保持独立公开类型。
- elapsed、waiter、observer 与 subscriber 状态必须仍受同一个 mutex 保护。
- 所有测试先于生产实现；不执行 git add、git commit 或 git push。

## Task 1: 建立 ClockDomain 与强类型 instant

Files:

- Create: src/monotonic/clock_domain.rs
- Delete: src/monotonic/clock_domain_id.rs
- Modify: src/monotonic/mod.rs、src/monotonic/monotonic_instant.rs、src/error/time_error.rs
- Rename: tests/monotonic/clock_domain_id_tests.rs to tests/monotonic/clock_domain_tests.rs
- Modify: tests/monotonic/mod.rs、tests/monotonic/monotonic_instant_tests.rs、tests/error/time_error_tests.rs

Produces:

~~~
pub struct ClockDomain(u64);

impl ClockDomain {
    pub fn new() -> Self;
}

impl MonotonicInstant {
    pub const fn new(domain: ClockDomain, elapsed: Duration) -> Self;
    pub const fn domain(self) -> ClockDomain;
}
~~~

- [ ] Step 1: 写失败测试。

~~~
#[test]
fn test_clock_domain_new_creates_distinct_domains() {
    let first = ClockDomain::new();
    let second = ClockDomain::new();
    assert_ne!(first, second);
    assert_ne!("0", first.to_string());
}

#[test]
fn test_monotonic_instant_preserves_clock_domain() {
    let domain = ClockDomain::new();
    let instant = MonotonicInstant::new(domain, Duration::from_secs(3));
    assert_eq!(domain, instant.domain());
    assert_eq!(Duration::from_secs(3), instant.elapsed_since_origin());
}
~~~

- [ ] Step 2: 运行 cargo test --test monotonic_tests clock_domain -- --nocapture。

Expected: 失败，ClockDomain 或公开 instant 构造函数不存在。

- [ ] Step 3: 最小实现 ClockDomain。

~~~
fn allocate_clock_domain_identifier(next: &AtomicU64) -> u64 {
    next.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
        match value {
            0 => None,
            u64::MAX => Some(0),
            value => Some(value + 1),
        }
    })
    .expect("monotonic clock domain identifiers exhausted")
}
~~~

`ClockDomain::new()` 包装该 helper 的返回值；实现 `Display`；将 instant 字段改为
`domain: ClockDomain`，保留 `elapsed_since_origin()`；将
`TimeError::ClockDomainMismatch` 字段改为 `ClockDomain`。不提供 `Default`、裸 ID
getter 或 `from_u64()`。`u64::MAX` 返回后，零值作为耗尽终态，后续分配 panic。

- [ ] Step 4: 运行 cargo test --test error_tests --test monotonic_tests clock_domain -- --nocapture。

Expected: PASS。

## Task 2: 收紧 MonotonicClock 并迁移内置 clock

Files:

- Modify: src/monotonic/monotonic_clock.rs、std_monotonic_clock.rs、tokio_monotonic_clock.rs、manual_monotonic_clock.rs
- Modify: tests/monotonic/monotonic_clock_tests.rs、std_monotonic_clock_tests.rs、tokio_monotonic_clock_tests.rs、manual_monotonic_clock_tests.rs

Produces:

~~~
pub trait MonotonicClock: Send + Sync {
    fn now(&self) -> MonotonicInstant;
}
~~~

- [ ] Step 1: 写失败测试。

~~~
struct ExternalMonotonicClock {
    domain: ClockDomain,
    elapsed: Duration,
}

impl MonotonicClock for ExternalMonotonicClock {
    fn now(&self) -> MonotonicInstant {
        MonotonicInstant::new(self.domain, self.elapsed)
    }
}

#[test]
fn test_monotonic_clock_can_return_atomic_snapshot() {
    let clock = ExternalMonotonicClock {
        domain: ClockDomain::new(),
        elapsed: Duration::from_secs(2),
    };
    assert_eq!(Duration::from_secs(2), clock.now().elapsed_since_origin());
}
~~~

- [ ] Step 2: 运行 cargo test --test monotonic_tests test_monotonic_clock_can_return_atomic_snapshot -- --nocapture。

Expected: 失败，旧 trait 仍要求 domain_id() 与 elapsed_since_origin()。

- [ ] Step 3: 最小实现。

~~~
impl MonotonicClock for StdMonotonicClock {
    fn now(&self) -> MonotonicInstant {
        MonotonicInstant::new(self.domain, self.origin.elapsed())
    }
}
~~~

对 Tokio 和 manual clock 实现相同接口；manual 的 elapsed 读取在 state lock 内。删除 trait 的拆分采样方法；Arc 和 Box 仅转发 now()；instant 调用改为 domain()。

- [ ] Step 4: 运行 cargo test --all-features --test monotonic_tests -- --nocapture。

Expected: PASS。

## Task 3: Sleeper 改为组合 clock

Files:

- Modify: src/sleep/blocking_sleeper.rs、async_sleeper.rs
- Modify: src/sleep/std_blocking_sleeper.rs、tokio_async_sleeper.rs、manual_blocking_sleeper.rs、manual_async_sleeper.rs
- Modify: tests/sleep/blocking_sleeper_tests.rs、async_sleeper_tests.rs、std_blocking_sleeper_tests.rs、tokio_async_sleeper_tests.rs、manual_blocking_sleeper_tests.rs、manual_async_sleeper_tests.rs

Produces:

~~~
pub trait BlockingSleeper: Send + Sync {
    fn clock(&self) -> &dyn MonotonicClock;
    fn sleep_until(&self, deadline: MonotonicInstant) -> Result<(), TimeError>;
}

pub trait AsyncSleeper: Send + Sync {
    fn clock(&self) -> &dyn MonotonicClock;
    fn sleep_until_async(&self, deadline: MonotonicInstant) -> SleepFuture;
}
~~~

- [ ] Step 1: 写失败测试。

~~~
#[test]
fn test_blocking_sleeper_exposes_paired_clock() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let sleeper: Arc<dyn BlockingSleeper> =
        Arc::new(ManualBlockingSleeper::from_clock(Arc::clone(&clock)));
    assert_eq!(clock.now().domain(), sleeper.clock().now().domain());
}
~~~

为 async sleeper 和 Arc/Box 转发增加同构测试。

- [ ] Step 2: 运行 cargo test --all-features --test sleep_tests test_blocking_sleeper_exposes_paired_clock -- --nocapture。

Expected: 失败，旧 trait 没有 clock()。

- [ ] Step 3: 最小实现。

~~~
impl BlockingSleeper for ManualBlockingSleeper {
    fn clock(&self) -> &dyn MonotonicClock {
        self.clock.as_ref()
    }

    fn sleep_until(&self, deadline: MonotonicInstant) -> Result<(), TimeError> {
        self.clock.wait_until_blocking(deadline)
    }
}
~~~

删除四个 sleeper 的 impl MonotonicClock；默认 relative sleep 通过 self.clock().now() 创建 deadline；native deadline 用持有 clock 的私有 domain 校验。

- [ ] Step 4: 简化 StdBlockingSleeper。

~~~
let now = Instant::now();
if let Some(remaining) = deadline.checked_duration_since(now) {
    thread::sleep(remaining);
}
Ok(())
~~~

更新 rustdoc：调度可能使返回晚于 deadline，不承诺精确唤醒时间。

- [ ] Step 5: 运行 cargo test --all-features --test sleep_tests -- --nocapture。

Expected: PASS。

## Task 4: 拆分 manual waiter 与 subscriber registry

Files:

- Create: src/monotonic/manual_waiter_registry.rs
- Create: src/monotonic/manual_advance_registry.rs
- Modify: src/monotonic/mod.rs、manual_monotonic_state.rs、manual_monotonic_clock.rs、manual_sleep_future.rs、manual_waiter_future.rs、manual_advance_subscription.rs
- Modify: tests/monotonic/manual_monotonic_clock_tests.rs、manual_waiter_future_tests.rs、manual_advance_subscription_tests.rs
- Modify: tests/sleep/manual_sleep_future_tests.rs、manual_async_sleeper_tests.rs、manual_blocking_sleeper_tests.rs

Produces: 私有 registry、具名 `AdvanceEffects` 与锁外 effect 分发；公开 manual API
不变。

- [ ] Step 1: 写重构保护测试。

~~~
#[tokio::test]
async fn test_manual_async_sleep_completes_after_advance_before_first_poll() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let sleeper = ManualAsyncSleeper::from_clock(Arc::clone(&clock));
    let sleep = sleeper.sleep_for_async(Duration::from_secs(5));

    assert_eq!(1, clock.pending_waiters());
    clock.advance(Duration::from_secs(5))
        .expect("manual advance should succeed");

    sleep.await.expect("reached unpolled sleep should complete");
    assert_eq!(0, clock.pending_waiters());
}
~~~

- [ ] Step 2: 运行 cargo test --all-features --test sleep_tests test_manual_async_sleep_completes_after_advance_before_first_poll -- --nocapture。

Expected: PASS；该测试固定既有契约后再拆分内部实现。

- [ ] Step 3: 抽取 registry 与 state effects。

~~~
pub(crate) struct ManualWaiterRegistry {
    blocking_waiters: HashMap<u64, Duration>,
    async_waiters: HashMap<u64, (Duration, Option<Waker>)>,
    observers: HashMap<u64, (usize, Option<Waker>)>,
}

pub(crate) struct ManualAdvanceRegistry {
    subscribers: HashMap<u64, AdvanceCallback>,
}
~~~

`ManualMonotonicState` 持有 elapsed 和两个 registry；`ManualMonotonicClock` 在持锁
范围内完成 elapsed 状态转换与 next-deadline 决策，并汇总 `AdvanceEffects`，随后在
锁外执行 Condvar、wake 和 callback fanout。到期 waker 使用 `Option::take()` 只移出
一次，waiter 注册继续保留到 poll/drop；subscriber 继续为每次推进创建 callback
快照。不得引入第二把 state mutex。

- [ ] Step 4: 运行 cargo test --all-features --test monotonic_tests --test sleep_tests -- manual --nocapture。

Expected: PASS，覆盖 mixed deadline、waker replacement、drop cleanup、observer latch、concurrent advance 与 subscriber panic fanout。

- [ ] Step 5: 更新 ManualAsyncSleeper 类型和方法 rustdoc。

说明 future 创建即注册，未 poll future 可被 manual driver 观察，drop 会注销。

Run: cargo test --doc --all-features

Expected: PASS。

## Task 5: 迁移 rs-retry 与文档

Files:

- Modify: ../rs-retry/src/executor/retry_runner.rs、async_retry_runner.rs、worker_retry_runner.rs、retry_flow_state.rs
- Modify: README.md、README.zh_CN.md、doc/clock_refactoring_design.zh_CN.md

- [ ] Step 1: 验证迁移前的编译失败。

删除 sleeper 的 MonotonicClock 继承后运行：

Run: cargo test --manifest-path ../rs-retry/Cargo.toml --test executor_tests -- --nocapture

Expected: 失败，调用方仍使用 sleeper.now()。

- [ ] Step 2: 迁移三个 runner。

~~~
let sleeper = self.retry.blocking_sleeper();
let clock = sleeper.clock();
let mut state = RetryFlowState::new(clock);
let attempt_start = clock.now();
~~~

async runner 使用同一模式。不要向 builder 新增独立 clock 字段或 setter。

- [ ] Step 3: 更新 README 与历史设计引用。

README 说明 sleeper 通过 clock() 暴露配对时钟；旧设计文档加入指向 clock_api_simplification_design.zh_CN.md 的后续调整说明。

- [ ] Step 4: 运行 cargo test --manifest-path ../rs-retry/Cargo.toml --all-features && cargo test --manifest-path ../rs-lock/Cargo.toml --features mock。

Expected: PASS。

## Task 6: 全量验证

- [ ] Step 1: 运行 rg -n 'allocate_clock_domain_id|\.domain_id\(|sleeper\.now\(' src tests ../rs-retry/src ../rs-lock/src。

Expected: 无匹配；MonotonicInstant::elapsed_since_origin() 可以保留。

- [ ] Step 2: 运行 cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test --all-features。

Expected: PASS。

- [ ] Step 3: 运行 git --no-pager diff --check && git status --short。

Expected: 无空白错误；变更仅限 rs-clock、必要的 rs-retry 迁移和相关文档。
