# Qubit Clock

[![Rust CI](https://github.com/qubit-ltd/rs-clock/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-clock/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-clock/coverage-badge.json)](https://qubit-ltd.github.io/rs-clock/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-clock.svg?color=blue)](https://crates.io/crates/qubit-clock)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![English Document](https://img.shields.io/badge/Document-English-blue.svg)](README.md)

为 Rust 提供可注入的 wall clock、monotonic clock，以及可确定性测试的同步和异步 sleeper。

## 设计

Qubit Clock 将时间拆成四种能力：

- `WallClock`：以 `SystemTime` 读取现实世界时间。
- `MonotonicClock`：读取属于特定 clock domain 的 `MonotonicInstant`。
- `BlockingSleeper`：按 monotonic deadline 阻塞当前线程。
- `AsyncSleeper`：返回按 monotonic deadline 完成的 future。

Wall time 可能跳变，monotonic time 永不倒退。每个 sleeper 都显式基于对应的
concrete monotonic clock，不维护第二套时间状态。通过 `clock()` 可取得配对
clock；sleeper 本身不再是 `MonotonicClock`。

## 实现

| 能力 | 真实时间实现 | 确定性测试实现 |
|---|---|---|
| Wall time | `StdWallClock` | `FixedWallClock`、`ManualWallClock` |
| Monotonic time | `StdMonotonicClock`、`TokioMonotonicClock` | `ManualMonotonicClock` |
| 同步 sleep | `StdBlockingSleeper` | `ManualBlockingSleeper` |
| 异步 sleep | `TokioAsyncSleeper` | `ManualAsyncSleeper` |

Tokio 类型需要启用可选的 `tokio` feature。Manual async sleeper 不依赖 Tokio runtime。

## 安装

```toml
[dependencies]
qubit-clock = "0.9"
```

需要 Tokio 集成时：

```toml
[dependencies]
qubit-clock = { version = "0.9", features = ["tokio"] }
```

## Wall Time

```rust
use qubit_clock::{StdWallClock, WallClock};

let clock = StdWallClock::new();
let now = clock.now();
println!("当前 wall time：{now:?}");
```

## Monotonic Time

```rust
use qubit_clock::{MonotonicClock, StdMonotonicClock};

let clock = StdMonotonicClock::new();
let start = clock.now();
let elapsed = clock
    .now()
    .duration_since(start)
    .expect("两个 instant 来自同一个 clock");
println!("耗时：{elapsed:?}");
```

## Tokio Time Driver

`TokioMonotonicClock` 及与其配对的 `TokioAsyncSleeper` 跟随使用它们时所在的
Tokio time context。使用暂停或显式推进的 Tokio 时间时，必须在同一个 Tokio
runtime time driver 下创建并读取 clock、poll sleeper future。任务可以在该
runtime 的不同 worker thread 之间迁移，但 clock/sleeper 组合不能跨独立 runtime
使用。`qubit-clock` 无法校验该关联，因此它属于调用方契约。

创建 sleep future 仍是惰性的，不要求当前已进入 runtime；首次 poll 必须位于启用
time driver 的 Tokio runtime 中。使用暂停时间时，该 driver 还必须与配对 clock
所使用的 driver 相同。

## 确定性同步 Sleep

通过 `Arc` 显式表达多个组件共享同一个 clock：

```rust
use qubit_clock::{
    BlockingSleeper, ManualBlockingSleeper, ManualMonotonicClock,
};
use std::sync::Arc;
use std::time::Duration;

let clock = Arc::new(ManualMonotonicClock::new());
let sleeper = Arc::new(ManualBlockingSleeper::from_clock(
    Arc::clone(&clock),
));
let worker_sleeper = Arc::clone(&sleeper);

let worker = std::thread::spawn(move || {
    worker_sleeper
        .sleep_for(Duration::from_secs(10))
        .expect("manual sleep 应正常完成");
});

assert!(clock.wait_for_waiters(1, Duration::from_secs(1)));
clock
    .advance(Duration::from_secs(10))
    .expect("manual time 应推进成功");
worker.join().expect("工作线程不应 panic");
```

## Manual Wall Time

`ManualWallClock` 将一个 manual monotonic timeline 映射到 wall-time anchor：

```rust
use qubit_clock::{
    ManualMonotonicClock, ManualWallClock, WallClock,
};
use std::sync::Arc;
use std::time::{Duration, UNIX_EPOCH};

let monotonic_clock = Arc::new(ManualMonotonicClock::new());
let wall_clock = ManualWallClock::from_clock(
    UNIX_EPOCH,
    Arc::clone(&monotonic_clock),
);

monotonic_clock
    .advance(Duration::from_secs(600))
    .expect("manual time 应推进成功");
assert_eq!(UNIX_EPOCH + Duration::from_secs(600), wall_clock.now());

wall_clock.reanchor(UNIX_EPOCH);
assert_eq!(UNIX_EPOCH, wall_clock.now());
```

重新设置 wall anchor 不会改变 monotonic deadline。
如果 wall anchor 加上手动推进的时长超出平台可表示的 `SystemTime` 范围，
`ManualWallClock::now()` 会 panic。

## 统一 Manual Time Driver

同一个 `ManualMonotonicClock` 可以同时驱动 blocking 和 async sleeper。
`pending_waiters()` 汇总两类 waiter，`next_deadline()` 查看最早的未来
deadline，`advance_to_next_deadline()` 原子地推进到该 deadline。阻塞测试驱动可用
`wait_for_next_deadline(real_timeout)` 等待后续重试或超时阶段，无需轮询，也不会被
上一阶段尚在清理的已到期 waiter 误满足。异步测试可用
`clock.wait_for_waiters_async(expected_count)` 等待注册，无需轮询，也不绑定特定
runtime。数量一旦达到目标便会被锁存，即使 waiter 随即注销，等待 future 仍会完成。
`ManualAsyncSleeper` 会在创建 sleep future 时注册 waiter，因此尚未 poll 的
manual sleep 也会计入这些协调方法。相对 deadline 在调用 sleep 方法时即被固定；
如果首次 poll 前 manual time 已推进到 deadline，该次 poll 会立即完成。丢弃尚未
完成的 future 会注销 waiter，foreign deadline 则通过立即 ready 的错误 future
返回。

## Manual Advance 通知

需要让自身 notification 与 manual deadline 竞争的同步测试替身，可以订阅 clock advance：

```rust
use qubit_clock::ManualMonotonicClock;
use std::sync::Arc;

let clock = Arc::new(ManualMonotonicClock::new());
let subscription = clock.subscribe_advances(|| {
    // 唤醒测试替身自己的 Condvar、watch channel 或 task waker。
});
```

callback 在 clock mutex 外同步执行。它应可重复调用，并且只负责唤醒另一个等待原语。并发 advance 可能无序、并发地执行 callback。如果 callback panic，本次 advance 已捕获的所有 callback 仍会执行，随后在推进线程恢复第一个 panic。需要通知期间必须保留 `subscription`；丢弃它会注销后续 advance 通知，但已被某次进行中的 advance 捕获的 callback 仍可能执行一次。

## 许可证

使用 Apache License 2.0 许可。
