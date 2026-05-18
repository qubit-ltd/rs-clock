# Mock Time 设计说明

## 版本信息

- **文档版本**: 1.0
- **创建日期**: 2026-05-18
- **作者**: 胡海星

## 1. 背景

`qubit-clock` 的 mock 机制用于解决测试中的时序问题。测试里经常同时出现几类时间相关行为：

1. 代码读取“当前时间”，例如 `Clock::time()`、`Clock::millis()`、`NanoClock::nanos()`。
2. 代码执行 relative sleep，例如 retry、backoff、polling interval。
3. 同步原语等待“状态变化或超时”，例如后续 `rs-lock` 中的 mock monitor。

如果这些组件各自维护一套 mock 时间，就会出现测试无法严格控制的情况：clock 已经前进，sleeper 没有前进；或者 sleeper 已经完成，但 monitor 仍按另一套时间判断 timeout。新的 mock 机制把时间推进收敛到一个共享的 `MockTimeline` 上，所有 mock 组件只观察这一个 timeline。

## 2. 设计目标

### 2.1 单一时间源

同一个测试场景中，mock clock、mock sleeper、未来的 mock monitor 都应由同一个 `MockTimeline` 驱动。测试只需要调用一次 `advance()`，所有组件都按同一套逻辑前进。

### 2.2 显式推进

mock time 不随真实时间自动流逝。只有测试显式调用 `MockTimeline::advance()` 或 `MockTime::advance()` 时，elapsed time 才会增加。

### 2.3 可观测等待状态

测试常见竞态是：测试线程先推进 timeline，工作线程还没真正进入 sleep 或 deadline wait。`MockTimeline` 跟踪 waiter 数量，并提供 `wait_for_blocked_waiters()`，让测试可以先确认等待者已经注册，再推进 mock 时间。

### 2.4 拒绝不安全 reset

当仍有 active waiter 时，reset timeline 会让等待者正在等待的 deadline 失去语义。新的设计在这种情况下返回 `MockTimeError::ActiveWaiters`，由测试先释放或取消等待者，再 reset。

### 2.5 为 monitor 留接口

`MockTimeline` 不只服务 sleep。它还提供 event epoch 和 `notify_external_change()`，用于支持“状态变化或 deadline 到达”的 monitor 类原语。这样 `rs-lock` 的 `MockMonitor` 后续可以使用同一个 timeline，而不是维护自己的 elapsed time。

### 2.6 拒绝跨 timeline instant

`MockInstant` 不只是一个 elapsed offset，它还携带创建它的 `MockTimeline` id。`MockTimeline::wait_until()` 会检查 instant 是否来自自己；如果来自另一个 timeline，则返回 `MockTimeError::MismatchedTimeline`。这样测试一旦把不同 mock runtime 的时间值混用，会立刻暴露错误。

## 3. 核心组件

### 3.1 MockTimeline

`MockTimeline` 是整个 mock runtime 的核心。它维护：

- `id`：创建时由全局 `AtomicU64` 自动分配的唯一 timeline id，clone 保持相同 id。
- `elapsed_nanos`：从 timeline origin 开始的单调 elapsed time。
- `time_epoch`：时间推进版本，用于区分时间推进事件。
- `event_epoch`：事件通知版本，时间推进和外部状态变化都会递增。
- `sleep_waiters`：通过 `MockSleeper` 进入 sleep 的等待者数量。
- `deadline_waiters`：直接通过 timeline deadline API 等待的等待者数量。

`MockTimeline` 的职责：

- 提供 `now()` 返回当前 `MockInstant`。
- 提供 `advance(duration)` 推进 elapsed time，并唤醒 blocking / async waiters。
- 提供 `wait_until(deadline)` 和 `wait_for(duration)` 支持 deadline wait；`wait_until()` 会拒绝 foreign instant。
- 提供 `wait_for_event_after(epoch)` 支持 monitor 等状态变化等待。
- 提供 `wait_for_blocked_waiters(kind, count, real_timeout)` 支持测试观测。
- 在 active waiter 存在时拒绝 `reset()`。

真实时间只用于 `wait_for_blocked_waiters()` 的保护性 timeout，避免测试挂死。mock 业务语义不依赖真实时间。

### 3.2 MockInstant

`MockInstant` 是 timeline 上的单调 instant，包含两部分：

- `timeline_id`：创建它的 `MockTimeline` id。
- `nanos_since_origin`：距离该 timeline origin 的纳秒数。

它不是 UTC 时间，也不表示系统时间。

`MockInstant` 的主要用途是表达 deadline：

```rust
let deadline = timeline.now().saturating_add(Duration::from_millis(100));
timeline
    .wait_until(deadline)
    .expect("deadline should belong to this timeline");
```

不同 timeline 产生的 `MockInstant` 不应该混用。`MockTimeline::wait_until()` 会校验 `timeline_id`，不匹配时返回：

```rust
MockTimeError::MismatchedTimeline { expected, actual }
```

排序语义也只在同一个 timeline 内成立。两个来自不同 timeline 的 `MockInstant` 做 `partial_cmp()` 会返回 `None`。

### 3.3 MockClock

`MockClock` 把 timeline elapsed time 映射成 UTC wall-clock time。它的核心公式是：

```text
current_utc = wall_origin + timeline.elapsed()
```

其中 `wall_origin` 是一个锚点。创建 clock 时，`MockClock::at(start)` 表示当前 timeline instant 对应的 UTC 时间是 `start`。

`MockClock` 的关键语义：

- 实现 `Clock`、`NanoClock` 和 `ControllableClock`。
- timeline 不推进时，clock 读数保持冻结。
- `advance(duration)` 推进共享 timeline。
- `set_time(instant)` 只重新锚定当前 timeline instant 对应的 UTC 时间，不改变 elapsed time。
- `reset()` / `try_reset()` 会 reset timeline，并恢复初始 wall-clock anchor。

同一个 `MockClock` clone 会共享 timeline 和 wall-clock anchor。通过 `MockClock::with_timeline()` 创建的多个 clock 可以共享同一个 timeline，但拥有各自的 wall-clock anchor。

### 3.4 MockSleeper

`MockSleeper` 是 `Sleeper` 的测试实现。它不调用 `thread::sleep()`，而是把 relative sleep 转成 timeline deadline wait。

执行：

```rust
sleeper.sleep_for(Duration::from_millis(100));
```

等价于：

1. 读取当前 `timeline.now()`。
2. 计算 deadline = now + 100ms。
3. 注册 `MockWaiterKind::Sleep` waiter。
4. 阻塞到 timeline elapsed time 到达 deadline。

因此测试可以这样控制 sleep：

```rust
let mock = MockTime::unix_epoch();
let sleeper = mock.sleeper();

let worker = std::thread::spawn(move || {
    sleeper.sleep_for(Duration::from_millis(100));
});

mock.timeline().wait_for_blocked_waiters(
    MockWaiterKind::Sleep,
    1,
    Duration::from_secs(1),
);
mock.advance(Duration::from_millis(100));

worker.join().expect("worker should finish");
```

启用 `tokio` feature 后，`MockSleeper` 的 async sleep 也使用同一个 timeline 和 waiter 注册模型。

### 3.5 MockTime

`MockTime` 是推荐的测试入口。它一次性创建：

- 一个 `MockTimeline`
- 一个共享该 timeline 的 `MockClock`
- 一个共享该 timeline 的 `MockSleeper`

典型用法：

```rust
let mock = MockTime::unix_epoch();
let clock = mock.clock();
let sleeper = mock.sleeper();

assert_eq!(0, clock.millis());

mock.advance(Duration::from_millis(250));

assert_eq!(250, clock.millis());
assert_eq!(Duration::from_millis(250), mock.elapsed());
```

当一个测试既需要 clock 又需要 sleeper 时，优先使用 `MockTime`，而不是分别创建 `MockClock::new()` 和 `MockSleeper::new()`。后者会产生两套独立 timeline。

## 4. 同步模型

### 4.1 blocking wait

blocking wait 使用 `parking_lot::Mutex` 和 `parking_lot::Condvar`。选择 `parking_lot` 的原因不是性能优先，而是语义更贴合 mock runtime：

- mock runtime 不需要 std mutex poisoning 语义。
- panic 后不应让整个 mock timeline 永久 poisoned。
- `Condvar::wait()` / `wait_for()` 写法更直接，核心状态机更清楚。

### 4.2 async wait

启用 `tokio` feature 后，async wait 使用 `tokio::sync::watch` 发布 event epoch。timeline 每次 advance 或 external notify 都会发送新 epoch，future 被唤醒后重新检查 deadline。

async sleeper 的 waiter registration 在 future 创建时完成，并由 future 持有的 drop guard 负责注销。这样即使 future 被取消，active waiter 计数也能回到正确状态。

### 4.3 event epoch

`event_epoch` 是为 monitor 类原语准备的。它表示“timeline 相关事件发生过”，不要求 elapsed time 一定变化。

例如 monitor wait 的典型逻辑可以是：

1. 检查 predicate 是否已经满足。
2. 如果未满足，记录当前 `event_epoch`。
3. 计算 timeout deadline。
4. 等待 event epoch 变化或 deadline 到达。
5. 被唤醒后重新检查 predicate 和 deadline。

状态变化时调用 `notify_external_change()`，timeout 推进时调用 `advance()`。两者都唤醒等待者，但只有 `advance()` 改变 elapsed time。

## 5. 与 rs-lock MockMonitor 的关系

`rs-lock` 的 `MockMonitor` 后续应改为接收或创建 `MockTimeline`：

```rust
let mock_time = MockTime::unix_epoch();
let monitor = MockMonitor::with_timeline(mock_time.timeline());
```

这样一个测试中：

- `MockClock` 读当前时间。
- `MockSleeper` 控制 backoff / retry sleep。
- `MockMonitor` 控制 condition wait / timeout。

三者都通过同一个 `MockTimeline` 推进。测试只需要 `mock_time.advance(duration)`，不会再出现 clock、sleeper、monitor 各自按不同 mock 时间前进的问题。

## 6. 不做什么

当前 mock runtime 不试图成为完整虚拟运行时：

- 不接管 Tokio 的全局 time driver。
- 不模拟任务调度顺序。
- 不提供绝对 UTC deadline sleep。
- 不允许把一个 timeline 创建的 `MockInstant` 当成另一个 timeline 的 deadline 使用。
- 不在 active waiter 存在时强制 reset。

这些边界让 `MockTimeline` 保持为一个小而稳定的测试时间源，而不是复杂的 scheduler。

## 7. 使用建议

1. 单独测试 clock 行为时，可以直接使用 `MockClock::at()`。
2. 单独测试 sleeper 行为时，可以直接使用 `MockSleeper::new()`。
3. 同一个测试同时涉及 clock、sleep、timeout monitor 时，优先使用 `MockTime`。
4. 测试线程或 future 是否已经进入等待，应使用 `wait_for_blocked_waiters()` 确认。
5. 需要模拟“状态变化但时间没前进”时，使用 `notify_external_change()`。
6. reset 失败时，不要绕过错误；应先释放 active waiter，再 reset。
7. 不要跨 timeline 传递 `MockInstant`；如果需要共享 deadline，应共享同一个 `MockTimeline`。

## 8. 关键结论

新的 mock 机制只有一个核心原则：所有测试时序都落到同一个 `MockTimeline` 上。

`MockClock` 负责把 timeline elapsed time 映射成 UTC 时间，`MockSleeper` 负责把 relative sleep 映射成 timeline deadline wait，未来 `MockMonitor` 应负责把 condition wait / timeout 映射成 timeline event + deadline wait。测试只控制 timeline，其他组件只观察 timeline。
